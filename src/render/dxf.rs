//! DXF from a drawing document and its normalised content (clause 15.4).
//!
//! The file is written directly, as group-code text, because no maintained Rust crate writes paper-space
//! layouts with viewports and hatches (`dxf` 0.6 has neither a HATCH nor a VIEWPORT entity). The format is
//! DXF R2013 (AC1027), which is UTF-8, so the characters of clause 11.6.2 need no escape.
//!
//! Model space holds the entities of the viewports in metres. Paper space, the layout `SHEET`, holds the
//! sheet furniture, the furniture of each viewport and the marks of the hatches, in millimetres with +y up
//! (a paper point `(x, y)` of the normalised content is written `(x, H - y)`), and one VIEWPORT for each
//! viewport of the document at its scale. Every entity carries extended data of the application `TDFA`, so
//! that the normalised content can be made again from the file (clause 15.4.11).
//!
//! Two builds of one document give the same bytes: nothing here reads a clock or a random source.

use crate::data::{self, LineGroup, Role};
use crate::entities::{Entity, Shape};
use crate::numbers::round_half_even;
use crate::pipeline::{
    FURNITURE_LAYER, Files, Layers, Normalised, declared_layers, entity_style, line_group_of,
};
use crate::read::{arr, entities_of, f, p, points, rotation_of, st, window_of};
use crate::rules::{self, AlignH, AlignV};
use crate::symbols::{self, Libraries, Placement};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The application name of the extended data (clause 15.4.8).
pub const APPID: &str = "TDFA";
/// The name of the paper-space layout (clause 15.4.5).
pub const LAYOUT: &str = "SHEET";
/// The text style, which names the font file (clause 15.4.7).
pub const TEXT_STYLE: &str = "TDFA";
/// The layer of the viewport frames; it does not plot (clause 15.4.2).
pub const VIEWPORT_LAYER: &str = "VIEWPORT";
/// The font file named in the text style (clause 11.6.1).
pub const FONT_FILE_NAME: &str = "BarlowSemiCondensed-Regular.ttf";

/// The DXF text and the warnings (clause 15.4.9).
#[derive(Debug, Clone, PartialEq)]
pub struct DxfOutput {
    pub text: String,
    pub warnings: Vec<String>,
}

/// The name of the DXF line type for a standard line type and a width (clause 15.4.3).
pub fn linetype_name(line_type: &str, width_mm: f64) -> String {
    if line_type == "continuous" {
        "CONTINUOUS".to_string()
    } else {
        format!(
            "TDFA_{}_{:03}",
            line_type.to_uppercase(),
            hundredths(width_mm)
        )
    }
}

/// A width in hundredths of a millimetre, as a DXF line weight (clause 8.3.4).
pub fn hundredths(width_mm: f64) -> i32 {
    round_half_even(width_mm * 100.0, 0) as i32
}

// ---- group-code text ---------------------------------------------------------------------------

#[derive(Default)]
struct Out(String);

impl Out {
    fn s(&mut self, code: i32, value: &str) {
        let _ = write!(self.0, "{code}\n{value}\n");
    }
    fn i(&mut self, code: i32, value: i64) {
        let _ = write!(self.0, "{code}\n{value}\n");
    }
    fn f(&mut self, code: i32, value: f64) {
        let value = if value == 0.0 { 0.0 } else { value };
        let text = format!("{value}");
        let text = if text.contains('.')
            || text.contains('e')
            || text.contains("inf")
            || text.contains("NaN")
        {
            text
        } else {
            format!("{text}.0")
        };
        let _ = write!(self.0, "{code}\n{text}\n");
    }
    fn pt(&mut self, code: i32, x: f64, y: f64) {
        self.f(code, x);
        self.f(code + 10, y);
    }
    fn pt3(&mut self, code: i32, x: f64, y: f64) {
        self.pt(code, x, y);
        self.f(code + 20, 0.0);
    }
}

struct Handles(u64);

impl Handles {
    fn next(&mut self) -> String {
        let h = format!("{:X}", self.0);
        self.0 += 1;
        h
    }
}

/// Extended data tags: `key=value`, without the empty values.
fn xdata(out: &mut Out, tags: &[(&str, Option<String>)]) {
    out.s(1001, APPID);
    for (key, value) in tags {
        if let Some(v) = value {
            out.s(1000, &format!("{key}={v}"));
        }
    }
}

fn text_align(h: AlignH, v: AlignV) -> (i64, i64) {
    (
        match h {
            AlignH::Left => 0,
            AlignH::Centre => 1,
            AlignH::Right => 2,
        },
        match v {
            AlignV::Baseline => 0,
            AlignV::Middle => 2,
            AlignV::Top => 3,
        },
    )
}

/// Everything the writer collects while it makes the entities: the line types in use, the symbol blocks.
struct Builder<'a> {
    handles: Handles,
    ltypes: BTreeMap<String, (String, f64)>,
    group: &'static LineGroup,
    declared: &'a Layers,
    height: f64,
    rec_model: String,
    rec_paper: String,
}

impl Builder<'_> {
    fn linetype(&mut self, line_type: &str, width: f64) -> String {
        let name = linetype_name(line_type, width);
        if line_type != "continuous" {
            self.ltypes
                .entry(name.clone())
                .or_insert_with(|| (line_type.to_string(), width));
        }
        name
    }

    /// The start of an entity: type, handle, owner, layer, and the space it lives in.
    fn head(&mut self, out: &mut Out, kind: &str, owner: &str, paper: bool, layer: &str) {
        out.s(0, kind);
        let h = self.handles.next();
        out.s(5, &h);
        out.s(330, owner);
        out.s(100, "AcDbEntity");
        if paper {
            out.i(67, 1);
        }
        out.s(8, layer);
    }

    fn style(&mut self, out: &mut Out, line_type: &str, width: f64) {
        let name = self.linetype(line_type, width);
        out.s(6, &name);
        out.i(370, i64::from(hundredths(width)));
    }
}

/// Where an entity is written.
#[derive(Clone, Copy)]
struct Place<'a> {
    owner: &'a str,
    paper: bool,
}

#[allow(clippy::too_many_arguments)]
fn write_line(
    b: &mut Builder,
    out: &mut Out,
    at: Place,
    layer: &str,
    line_type: &str,
    width: f64,
    a: [f64; 2],
    c: [f64; 2],
    tags: &[(&str, Option<String>)],
) {
    b.head(out, "LINE", at.owner, at.paper, layer);
    b.style(out, line_type, width);
    out.s(100, "AcDbLine");
    out.pt3(10, a[0], a[1]);
    out.pt3(11, c[0], c[1]);
    xdata(out, tags);
}

#[allow(clippy::too_many_arguments)]
fn write_polyline(
    b: &mut Builder,
    out: &mut Out,
    at: Place,
    layer: &str,
    line_type: &str,
    width: f64,
    pts: &[[f64; 2]],
    closed: bool,
    tags: &[(&str, Option<String>)],
) {
    b.head(out, "LWPOLYLINE", at.owner, at.paper, layer);
    b.style(out, line_type, width);
    out.s(100, "AcDbPolyline");
    out.i(90, pts.len() as i64);
    out.i(70, i64::from(closed));
    out.f(43, 0.0);
    for q in pts {
        out.pt(10, q[0], q[1]);
    }
    xdata(out, tags);
}

#[allow(clippy::too_many_arguments)]
fn write_circle(
    b: &mut Builder,
    out: &mut Out,
    at: Place,
    layer: &str,
    line_type: &str,
    width: f64,
    c: [f64; 2],
    r: f64,
    tags: &[(&str, Option<String>)],
) {
    b.head(out, "CIRCLE", at.owner, at.paper, layer);
    b.style(out, line_type, width);
    out.s(100, "AcDbCircle");
    out.pt3(10, c[0], c[1]);
    out.f(40, r);
    xdata(out, tags);
}

#[allow(clippy::too_many_arguments)]
fn write_arc(
    b: &mut Builder,
    out: &mut Out,
    at: Place,
    layer: &str,
    line_type: &str,
    width: f64,
    c: [f64; 2],
    r: f64,
    start: f64,
    end: f64,
    tags: &[(&str, Option<String>)],
) {
    b.head(out, "ARC", at.owner, at.paper, layer);
    b.style(out, line_type, width);
    out.s(100, "AcDbCircle");
    out.pt3(10, c[0], c[1]);
    out.f(40, r);
    out.s(100, "AcDbArc");
    out.f(50, start);
    out.f(51, end);
    xdata(out, tags);
}

/// A solid hatch: the boundary, then the holes. A hatch with another pattern is written invisible: its
/// marks are entities of their own (clause 10.5.8).
#[allow(clippy::too_many_arguments)]
fn write_hatch(
    b: &mut Builder,
    out: &mut Out,
    at: Place,
    layer: &str,
    width: f64,
    boundary: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    solid: bool,
    tags: &[(&str, Option<String>)],
) {
    b.head(out, "HATCH", at.owner, at.paper, layer);
    out.i(370, i64::from(hundredths(width)));
    if !solid {
        out.i(60, 1);
    }
    out.s(100, "AcDbHatch");
    out.pt3(10, 0.0, 0.0);
    out.f(210, 0.0);
    out.f(220, 0.0);
    out.f(230, 1.0);
    out.s(2, "SOLID");
    out.i(70, 1);
    out.i(71, 0);
    out.i(91, 1 + holes.len() as i64);
    for loop_ in std::iter::once(boundary).chain(holes.iter().map(Vec::as_slice)) {
        out.i(92, 3);
        out.i(72, 0);
        out.i(73, 1);
        out.i(93, loop_.len() as i64);
        for q in loop_ {
            out.pt(10, q[0], q[1]);
        }
        out.i(97, 0);
    }
    out.i(75, 0);
    out.i(76, 1);
    out.i(98, 0);
    xdata(out, tags);
}

#[allow(clippy::too_many_arguments)]
fn write_text(
    b: &mut Builder,
    out: &mut Out,
    at: Place,
    layer: &str,
    text: &str,
    anchor: [f64; 2],
    height: f64,
    rotation: f64,
    h: AlignH,
    v: AlignV,
    tags: &[(&str, Option<String>)],
) {
    b.head(out, "TEXT", at.owner, at.paper, layer);
    out.s(100, "AcDbText");
    out.pt3(10, anchor[0], anchor[1]);
    out.f(40, height);
    out.s(1, text);
    out.f(50, rotation);
    out.s(7, TEXT_STYLE);
    let (hh, vv) = text_align(h, v);
    out.i(72, hh);
    out.pt3(11, anchor[0], anchor[1]);
    out.s(100, "AcDbText");
    out.i(73, vv);
    xdata(out, tags);
}

// ---- the whole file ------------------------------------------------------------------------------

/// One symbol block (clause 15.4.10).
struct SymbolBlock {
    name: String,
    record: String,
    body: String,
}

/// The DXF of a document, made from the document and its normalised content together (clause 15.1).
pub fn to_dxf(document: &Value, norm: &Normalised, files: &dyn Files) -> DxfOutput {
    let height = norm.sheet.height_mm;
    let width = norm.sheet.width_mm;
    let group = line_group_of(&norm.sheet.size);
    let styles = data::line_styles();
    let declared = declared_layers(document);
    let (libs, _) = symbols::load_libraries(document, files);

    // Layers: those of the document and the furniture layer.
    let mut layers: Layers = declared.clone();
    for name in std::iter::once(FURNITURE_LAYER) {
        if let Some(s) = styles.layer(name) {
            layers
                .entry(name.to_string())
                .or_insert((s.role, s.line_type.clone(), s.aci));
        }
    }

    let mut b = Builder {
        handles: Handles(1),
        ltypes: BTreeMap::new(),
        group,
        declared: &declared,
        height,
        rec_model: String::new(),
        rec_paper: String::new(),
    };
    // Fixed handles first.
    let t_layer = b.handles.next();
    let t_ltype = b.handles.next();
    let t_appid = b.handles.next();
    let t_dimstyle = b.handles.next();
    let t_style = b.handles.next();
    let t_ucs = b.handles.next();
    let t_view = b.handles.next();
    let t_vport = b.handles.next();
    let t_block_record = b.handles.next();
    let root_dict = b.handles.next();
    let layout_dict = b.handles.next();
    let group_dict = b.handles.next();
    let layout_model = b.handles.next();
    let layout_sheet = b.handles.next();
    b.rec_model = b.handles.next();
    b.rec_paper = b.handles.next();
    let (rec_model, rec_paper) = (b.rec_model.clone(), b.rec_paper.clone());

    // The layers' line types come first, so that the tables list them.
    for (role, line_type, _) in layers.values() {
        let w = group.width(*role);
        b.linetype(line_type, w);
    }

    let mut model = Out::default();
    let mut blocks: Vec<SymbolBlock> = Vec::new();
    let scale_of = |id: &str| {
        norm.viewports
            .iter()
            .find(|v| v.id == id)
            .map_or(1, |v| v.scale)
    };
    for vp in arr(&document["viewports"]) {
        let vid = st(&vp["id"]);
        let n = scale_of(vid);
        let rotation = rotation_of(vp);
        for e in entities_of(vp) {
            match st(&e["type"]) {
                "dimension" | "leader" => {} // their parts are paper-space entities (clauses 12.7 and 15.4.5)
                "symbol" => {
                    symbol_insert(&mut b, &mut model, &mut blocks, e, vid, n, rotation, &libs)
                }
                _ => model_entity(&mut b, &mut model, e, vid, n, rotation),
            }
        }
    }

    let mut paper = Out::default();
    let paper_at = Place {
        owner: &rec_paper,
        paper: true,
    };
    for e in &norm.entities {
        let origin = e.origin.as_str();
        if origin.starts_with("viewport.")
            && !origin.ends_with(".scale_bar")
            && !origin.ends_with(".north_arrow")
            && !origin.ends_with(".hatch")
            && !origin.ends_with(".dimension")
            && !origin.ends_with(".leader")
        {
            continue; // made in model space
        }
        paper_entity(&mut b, &mut paper, paper_at, e);
    }
    // The viewports. The first, of id 1, is the sheet's own, as CAD programs write it.
    let main = b.handles.next();
    write_viewport(
        &mut paper,
        &main,
        &rec_paper,
        (width / 2.0, height / 2.0),
        (width * 1.1, height * 1.1),
        1,
        1,
        (width / 2.0, height / 2.0),
        height * 1.1,
        0.0,
        &[],
    );
    let mut viewport_ids = Vec::new();
    for (i, v) in norm.viewports.iter().enumerate() {
        let src = arr(&document["viewports"])
            .iter()
            .find(|d| st(&d["id"]) == v.id)
            .expect("a viewport of the document");
        let w = window_of(&src["window"]);
        let handle = b.handles.next();
        viewport_ids.push(handle.clone());
        let tags = [
            ("id", Some(v.id.clone())),
            ("scale", Some(v.scale.to_string())),
        ];
        write_viewport(
            &mut paper,
            &handle,
            &rec_paper,
            (
                v.frame.x + v.frame.w / 2.0,
                height - (v.frame.y + v.frame.h / 2.0),
            ),
            (v.frame.w, v.frame.h),
            2,
            i as i64 + 2,
            ((w.xmin + w.xmax) / 2.0, (w.ymin + w.ymax) / 2.0),
            v.frame.h * f64::from(v.scale) / 1000.0,
            rotation_of(src),
            &tags,
        );
    }

    // ---- sections -------------------------------------------------------------------------------
    let mut out = Out::default();
    // CLASSES
    out.s(0, "SECTION");
    out.s(2, "CLASSES");
    out.s(0, "CLASS");
    out.s(1, "LAYOUT");
    out.s(2, "AcDbLayout");
    out.s(3, "ObjectDBX Classes");
    out.i(90, 0);
    out.i(91, 0);
    out.i(280, 0);
    out.i(281, 0);
    out.s(0, "ENDSEC");
    // TABLES
    out.s(0, "SECTION");
    out.s(2, "TABLES");
    table_head(&mut out, "VPORT", &t_vport, 1, false);
    {
        let h = b.handles.next();
        out.s(0, "VPORT");
        out.s(5, &h);
        out.s(330, &t_vport);
        out.s(100, "AcDbSymbolTableRecord");
        out.s(100, "AcDbViewportTableRecord");
        out.s(2, "*Active");
        out.i(70, 0);
        out.pt(10, 0.0, 0.0);
        out.pt(11, 1.0, 1.0);
        out.pt(12, 0.0, 0.0);
        out.pt(13, 0.0, 0.0);
        out.pt(14, 0.5, 0.5);
        out.pt(15, 0.5, 0.5);
        out.pt(16, 0.0, 0.0);
        out.f(36, 1.0);
        out.pt3(17, 0.0, 0.0);
        out.f(40, 1000.0);
        out.f(41, 1.34);
        out.f(42, 50.0);
        out.f(43, 0.0);
        out.f(44, 0.0);
        out.f(50, 0.0);
        out.f(51, 0.0);
        out.i(71, 0);
        out.i(72, 1000);
        out.i(73, 1);
        out.i(74, 3);
        out.i(75, 0);
        out.i(76, 0);
        out.i(77, 0);
        out.i(78, 0);
        out.i(281, 0);
        out.i(65, 0);
        out.f(146, 0.0);
    }
    out.s(0, "ENDTAB");
    // LTYPE: the three of every file, then one for each line type and width in use
    let ltype_names: Vec<(String, (String, f64))> = b
        .ltypes
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    table_head(&mut out, "LTYPE", &t_ltype, 3 + ltype_names.len(), false);
    for (name, description) in [
        ("ByBlock", ""),
        ("ByLayer", ""),
        ("CONTINUOUS", "Solid line"),
    ] {
        let h = b.handles.next();
        ltype_record(&mut out, &h, &t_ltype, name, description, &[]);
    }
    for (name, (line_type, w)) in &ltype_names {
        let h = b.handles.next();
        let pattern = rules::line_pattern(line_type, *w).unwrap_or_default();
        ltype_record(
            &mut out,
            &h,
            &t_ltype,
            name,
            &format!("{line_type}, line width {w} mm"),
            &pattern,
        );
    }
    out.s(0, "ENDTAB");
    // LAYER
    let mut layer_rows: Vec<(String, i64, String, i32, bool)> =
        vec![("0".to_string(), 7, "CONTINUOUS".to_string(), 25, true)];
    for (name, (role, line_type, aci)) in &layers {
        let w = group.width(*role);
        layer_rows.push((
            name.clone(),
            *aci,
            linetype_name(line_type, w),
            hundredths(w),
            true,
        ));
    }
    layer_rows.push((
        VIEWPORT_LAYER.to_string(),
        7,
        "CONTINUOUS".to_string(),
        25,
        false,
    ));
    table_head(&mut out, "LAYER", &t_layer, layer_rows.len(), false);
    for (name, aci, lt, lw, plot) in &layer_rows {
        let h = b.handles.next();
        out.s(0, "LAYER");
        out.s(5, &h);
        out.s(330, &t_layer);
        out.s(100, "AcDbSymbolTableRecord");
        out.s(100, "AcDbLayerTableRecord");
        out.s(2, name);
        out.i(70, 0);
        out.i(62, *aci);
        out.s(6, lt);
        if !plot {
            out.i(290, 0);
        }
        out.i(370, i64::from(*lw));
    }
    out.s(0, "ENDTAB");
    // STYLE
    table_head(&mut out, "STYLE", &t_style, 2, false);
    for (name, file) in [("Standard", "txt"), (TEXT_STYLE, FONT_FILE_NAME)] {
        let h = b.handles.next();
        out.s(0, "STYLE");
        out.s(5, &h);
        out.s(330, &t_style);
        out.s(100, "AcDbSymbolTableRecord");
        out.s(100, "AcDbTextStyleTableRecord");
        out.s(2, name);
        out.i(70, 0);
        out.f(40, 0.0);
        out.f(41, 1.0);
        out.f(50, 0.0);
        out.i(71, 0);
        out.f(42, 2.5);
        out.s(3, file);
        out.s(4, "");
    }
    out.s(0, "ENDTAB");
    table_head(&mut out, "VIEW", &t_view, 0, false);
    out.s(0, "ENDTAB");
    table_head(&mut out, "UCS", &t_ucs, 0, false);
    out.s(0, "ENDTAB");
    // APPID
    table_head(&mut out, "APPID", &t_appid, 2, false);
    for name in ["ACAD", APPID] {
        let h = b.handles.next();
        out.s(0, "APPID");
        out.s(5, &h);
        out.s(330, &t_appid);
        out.s(100, "AcDbSymbolTableRecord");
        out.s(100, "AcDbRegAppTableRecord");
        out.s(2, name);
        out.i(70, 0);
    }
    out.s(0, "ENDTAB");
    // DIMSTYLE: the one every file has
    table_head(&mut out, "DIMSTYLE", &t_dimstyle, 1, true);
    dimstyle_standard(&mut out, &b.handles.next(), &t_dimstyle);
    out.s(0, "ENDTAB");
    // BLOCK_RECORD
    table_head(
        &mut out,
        "BLOCK_RECORD",
        &t_block_record,
        2 + blocks.len(),
        false,
    );
    block_record(
        &mut out,
        &rec_model,
        &t_block_record,
        "*Model_Space",
        &layout_model,
    );
    block_record(
        &mut out,
        &rec_paper,
        &t_block_record,
        "*Paper_Space",
        &layout_sheet,
    );
    for sb in &blocks {
        block_record(&mut out, &sb.record, &t_block_record, &sb.name, "0");
    }
    out.s(0, "ENDTAB");
    out.s(0, "ENDSEC");
    // BLOCKS
    out.s(0, "SECTION");
    out.s(2, "BLOCKS");
    for (name, record) in [("*Model_Space", &rec_model), ("*Paper_Space", &rec_paper)] {
        let (begin, end) = (b.handles.next(), b.handles.next());
        block_begin(&mut out, &begin, record, name, [0.0, 0.0]);
        block_end(&mut out, &end, record);
    }
    for sb in &blocks {
        out.0.push_str(&sb.body);
    }
    out.s(0, "ENDSEC");
    // ENTITIES: model space, then paper space (code 67 = 1)
    out.s(0, "SECTION");
    out.s(2, "ENTITIES");
    out.0.push_str(&model.0);
    out.0.push_str(&paper.0);
    out.s(0, "ENDSEC");
    // OBJECTS
    out.s(0, "SECTION");
    out.s(2, "OBJECTS");
    dictionary(
        &mut out,
        &root_dict,
        "0",
        &[("ACAD_GROUP", &group_dict), ("ACAD_LAYOUT", &layout_dict)],
    );
    dictionary(
        &mut out,
        &layout_dict,
        &root_dict,
        &[("Model", &layout_model), (LAYOUT, &layout_sheet)],
    );
    dictionary(&mut out, &group_dict, &root_dict, &[]);
    layout_object(
        &mut out,
        &layout_model,
        &layout_dict,
        "Model",
        0,
        &rec_model,
        None,
        (width, height),
        false,
    );
    layout_object(
        &mut out,
        &layout_sheet,
        &layout_dict,
        LAYOUT,
        1,
        &rec_paper,
        Some(&main),
        (width, height),
        true,
    );
    out.s(0, "ENDSEC");
    out.s(0, "EOF");

    // HEADER, written last: it records the next free handle
    let mut head = Out::default();
    head.s(0, "SECTION");
    head.s(2, "HEADER");
    head.s(9, "$ACADVER");
    head.s(1, "AC1027");
    head.s(9, "$HANDSEED");
    head.s(5, &format!("{:X}", b.handles.0));
    head.s(9, "$INSUNITS");
    head.i(70, 6);
    head.s(9, "$MEASUREMENT");
    head.i(70, 1);
    head.s(9, "$PSLTSCALE");
    head.i(70, 1);
    head.s(9, "$LWDISPLAY");
    head.i(290, 1);
    head.s(9, "$TEXTSTYLE");
    head.s(7, TEXT_STYLE);
    head.s(0, "ENDSEC");
    head.0.push_str(&out.0);
    DxfOutput {
        text: head.0,
        warnings: window_overlap_warnings(document),
    }
}

fn table_head(out: &mut Out, name: &str, handle: &str, count: usize, dim: bool) {
    out.s(0, "TABLE");
    out.s(2, name);
    out.s(5, handle);
    out.s(330, "0");
    out.s(100, "AcDbSymbolTable");
    out.i(70, count as i64);
    if dim {
        out.s(100, "AcDbDimStyleTable");
    }
}

fn ltype_record(
    out: &mut Out,
    handle: &str,
    table: &str,
    name: &str,
    description: &str,
    pattern: &[f64],
) {
    out.s(0, "LTYPE");
    out.s(5, handle);
    out.s(330, table);
    out.s(100, "AcDbSymbolTableRecord");
    out.s(100, "AcDbLinetypeTableRecord");
    out.s(2, name);
    out.i(70, 0);
    out.s(3, description);
    out.i(72, 65);
    out.i(73, pattern.len() as i64);
    out.f(40, pattern.iter().map(|v| v.abs()).sum());
    for v in pattern {
        out.f(49, *v);
        out.i(74, 0);
    }
}

fn block_record(out: &mut Out, handle: &str, table: &str, name: &str, layout: &str) {
    out.s(0, "BLOCK_RECORD");
    out.s(5, handle);
    out.s(330, table);
    out.s(100, "AcDbSymbolTableRecord");
    out.s(100, "AcDbBlockTableRecord");
    out.s(2, name);
    out.s(340, layout);
    out.i(70, 0);
    out.i(280, 1);
    out.i(281, 0);
}

fn block_begin(out: &mut Out, handle: &str, record: &str, name: &str, base: [f64; 2]) {
    out.s(0, "BLOCK");
    out.s(5, handle);
    out.s(330, record);
    out.s(100, "AcDbEntity");
    out.s(8, "0");
    out.s(100, "AcDbBlockBegin");
    out.s(2, name);
    out.i(70, 0);
    out.pt3(10, base[0], base[1]);
    out.s(3, name);
    out.s(1, "");
}

fn block_end(out: &mut Out, handle: &str, record: &str) {
    out.s(0, "ENDBLK");
    out.s(5, handle);
    out.s(330, record);
    out.s(100, "AcDbEntity");
    out.s(8, "0");
    out.s(100, "AcDbBlockEnd");
}

fn dictionary(out: &mut Out, handle: &str, owner: &str, entries: &[(&str, &str)]) {
    out.s(0, "DICTIONARY");
    out.s(5, handle);
    out.s(330, owner);
    out.s(100, "AcDbDictionary");
    out.i(281, 1);
    for (name, target) in entries {
        out.s(3, name);
        out.s(350, target);
    }
}

/// A layout (clause 15.4.5): the plot settings with the paper size in millimetres, then the layout itself.
#[allow(clippy::too_many_arguments)]
fn layout_object(
    out: &mut Out,
    handle: &str,
    owner: &str,
    name: &str,
    tab: i64,
    record: &str,
    viewport: Option<&str>,
    (w, h): (f64, f64),
    sheet: bool,
) {
    out.s(0, "LAYOUT");
    out.s(5, handle);
    out.s(330, owner);
    out.s(100, "AcDbPlotSettings");
    out.s(1, "");
    if sheet {
        out.s(2, "DWG to PDF.pc3");
    }
    out.s(4, &format!("TDFA_({w:.2}_x_{h:.2}_MM)"));
    out.s(6, "");
    for code in [40, 41, 42, 43] {
        out.f(code, 0.0);
    }
    out.f(44, w);
    out.f(45, h);
    for code in [46, 47, 48, 49, 140, 141] {
        out.f(code, 0.0);
    }
    out.f(142, 1.0);
    out.f(143, 1.0);
    out.i(70, if sheet { 672 } else { 1024 });
    out.i(72, 1);
    out.i(73, 0);
    out.i(74, 5);
    out.s(7, "");
    out.i(75, 16);
    out.i(76, 0);
    out.i(77, 2);
    out.i(78, 300);
    out.f(147, 1.0);
    out.f(148, 0.0);
    out.f(149, 0.0);
    out.s(100, "AcDbLayout");
    out.s(1, name);
    out.i(70, 1);
    out.i(71, tab);
    out.pt(10, 0.0, 0.0);
    out.pt(11, w, h);
    out.pt3(12, 0.0, 0.0);
    out.pt3(14, 1e20, 1e20);
    out.pt3(15, -1e20, -1e20);
    out.f(146, 0.0);
    out.pt3(13, 0.0, 0.0);
    out.pt3(16, 1.0, 0.0);
    out.pt3(17, 0.0, 1.0);
    out.i(76, 1);
    out.s(330, record);
    if let Some(v) = viewport {
        out.s(331, v);
    }
}

fn dimstyle_standard(out: &mut Out, handle: &str, table: &str) {
    out.s(0, "DIMSTYLE");
    out.s(105, handle);
    out.s(330, table);
    out.s(100, "AcDbSymbolTableRecord");
    out.s(100, "AcDbDimStyleTableRecord");
    out.s(2, "Standard");
    out.i(70, 0);
    for (code, v) in [
        (40, 1.0),
        (41, 2.5),
        (42, 0.625),
        (43, 3.75),
        (44, 1.25),
        (45, 0.0),
        (46, 0.0),
        (47, 0.0),
        (48, 0.0),
        (49, 2.5),
        (140, 2.5),
        (141, 2.5),
        (142, 0.0),
        (143, 0.03937007874),
        (144, 1.0),
        (145, 0.0),
        (146, 1.0),
        (147, 0.625),
        (148, 0.0),
    ] {
        out.f(code, v);
    }
    for (code, v) in [
        (69, 0),
        (70, 0),
        (71, 0),
        (72, 0),
        (73, 0),
        (74, 0),
        (75, 0),
        (76, 0),
        (77, 1),
        (78, 8),
        (79, 3),
        (170, 0),
        (171, 3),
        (172, 1),
        (173, 0),
        (174, 0),
        (175, 0),
        (176, 0),
        (177, 0),
        (178, 0),
        (179, 2),
        (271, 2),
        (272, 2),
        (273, 2),
        (274, 3),
        (275, 0),
        (276, 0),
        (277, 2),
        (278, 44),
        (279, 0),
        (280, 0),
        (281, 0),
        (282, 0),
        (283, 0),
        (284, 8),
        (285, 0),
        (286, 0),
        (288, 0),
        (289, 3),
        (290, 0),
        (371, -2),
        (372, -2),
    ] {
        out.i(code, v);
    }
}

#[allow(clippy::too_many_arguments)]
fn write_viewport(
    out: &mut Out,
    handle: &str,
    owner: &str,
    centre: (f64, f64),
    size: (f64, f64),
    status: i64,
    id: i64,
    view_centre: (f64, f64),
    view_height: f64,
    twist: f64,
    tags: &[(&str, Option<String>)],
) {
    out.s(0, "VIEWPORT");
    out.s(5, handle);
    out.s(330, owner);
    out.s(100, "AcDbEntity");
    out.i(67, 1);
    out.s(8, VIEWPORT_LAYER);
    out.s(100, "AcDbViewport");
    out.pt3(10, centre.0, centre.1);
    out.f(40, size.0);
    out.f(41, size.1);
    out.i(68, status);
    out.i(69, id);
    out.pt(12, view_centre.0, view_centre.1);
    out.pt(13, 0.0, 0.0);
    out.pt(14, 10.0, 10.0);
    out.pt(15, 10.0, 10.0);
    out.pt(16, 0.0, 0.0);
    out.f(36, 1.0);
    out.pt3(17, 0.0, 0.0);
    out.f(42, 50.0);
    out.f(43, 0.0);
    out.f(44, 0.0);
    out.f(45, view_height);
    out.f(50, 0.0);
    out.f(51, twist);
    out.i(72, 100);
    out.i(90, if id == 1 { 557088 } else { 0 });
    out.s(1, "");
    out.i(281, 0);
    out.i(71, 0);
    out.i(74, 0);
    out.pt3(110, 0.0, 0.0);
    out.pt3(111, 1.0, 0.0);
    out.pt3(112, 0.0, 1.0);
    out.i(79, 0);
    out.f(146, 0.0);
    out.i(282, 0);
    if !tags.is_empty() {
        xdata(out, tags);
    }
}

/// Clause 15.4.9: two viewports share one model space, so overlapping windows show each other's content.
fn window_overlap_warnings(document: &Value) -> Vec<String> {
    let vps = arr(&document["viewports"]);
    let mut out = Vec::new();
    for i in 0..vps.len() {
        for j in (i + 1)..vps.len() {
            let (a, c) = (window_of(&vps[i]["window"]), window_of(&vps[j]["window"]));
            let ox = a.xmax.min(c.xmax) - a.xmin.max(c.xmin);
            let oy = a.ymax.min(c.ymax) - a.ymin.max(c.ymin);
            if ox > 0.0 && oy > 0.0 {
                out.push(format!(
                    "15.4.9: the model windows of viewports {} and {} overlap",
                    st(&vps[i]["id"]),
                    st(&vps[j]["id"])
                ));
            }
        }
    }
    out
}

// ---- entities ------------------------------------------------------------------------------------

fn id_tag(e: &Value) -> Option<String> {
    e.get("id").map(|v| st(v).to_string())
}

fn model_entity(b: &mut Builder, out: &mut Out, e: &Value, vid: &str, n: u32, rotation: f64) {
    let t = st(&e["type"]);
    let (line_type, width) = entity_style(e, b.declared, b.group);
    let layer = st(&e["layer"]).to_string();
    let owner = b.rec_model.clone();
    let at = Place {
        owner: &owner,
        paper: false,
    };
    let tags = [
        ("vp", Some(vid.to_string())),
        ("id", id_tag(e)),
        ("type", Some(t.to_string())),
    ];
    match t {
        "line" | "centre_line" => write_line(
            b,
            out,
            at,
            &layer,
            &line_type,
            width,
            p(&e["from_m"]),
            p(&e["to_m"]),
            &tags,
        ),
        "polyline" => write_polyline(
            b,
            out,
            at,
            &layer,
            &line_type,
            width,
            &points(&e["points_m"]),
            e.get("closed").and_then(Value::as_bool).unwrap_or(false),
            &tags,
        ),
        "circle" => write_circle(
            b,
            out,
            at,
            &layer,
            &line_type,
            width,
            p(&e["centre_m"]),
            f(&e["radius_m"]),
            &tags,
        ),
        "arc" => write_arc(
            b,
            out,
            at,
            &layer,
            &line_type,
            width,
            p(&e["centre_m"]),
            f(&e["radius_m"]),
            f(&e["start_deg"]).rem_euclid(360.0),
            f(&e["end_deg"]).rem_euclid(360.0),
            &tags,
        ),
        "hatch" => {
            let holes: Vec<Vec<[f64; 2]>> = e
                .get("holes_m")
                .map_or(vec![], |h| arr(h).iter().map(points).collect());
            let tags = [
                ("vp", Some(vid.to_string())),
                ("id", id_tag(e)),
                ("type", Some("hatch".to_string())),
                ("pattern", Some(st(&e["pattern"]).to_string())),
                ("angle", Some(angle_text(e.get("angle_deg").map_or(0.0, f)))),
            ];
            write_hatch(
                b,
                out,
                at,
                &layer,
                width,
                &points(&e["boundary_m"]),
                &holes,
                st(&e["pattern"]) == "solid",
                &tags,
            );
        }
        "text" => {
            let height = f(&e["height_mm"]) * f64::from(n) / 1000.0;
            let model_rotation =
                (e.get("rotation_deg").map_or(0.0, f) - rotation).rem_euclid(360.0);
            let tags = [
                ("vp", Some(vid.to_string())),
                ("id", id_tag(e)),
                ("type", Some("text".to_string())),
                ("paper_height", Some(angle_text(f(&e["height_mm"])))),
            ];
            write_text(
                b,
                out,
                at,
                &layer,
                st(&e["text"]),
                p(&e["anchor_m"]),
                height,
                model_rotation,
                AlignH::parse(e.get("align_h").map_or("left", st)),
                AlignV::parse(e.get("align_v").map_or("baseline", st)),
                &tags,
            );
        }
        _ => {}
    }
}

/// A number as the tags write it: the shortest text that reads back, with a point for whole numbers.
fn angle_text(x: f64) -> String {
    let text = format!("{x}");
    if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    }
}

#[allow(clippy::too_many_arguments)]
fn symbol_insert(
    b: &mut Builder,
    out: &mut Out,
    blocks: &mut Vec<SymbolBlock>,
    e: &Value,
    vid: &str,
    n: u32,
    rotation: f64,
    libs: &Libraries,
) {
    let Some(symbol) = libs.symbol(st(&e["library"]), st(&e["symbol"])) else {
        return;
    };
    let layer = st(&e["layer"]);
    let role_override = e.get("role").map(st);
    let name = format!(
        "TDFA_{}_{}@{}{}",
        st(&e["library"]),
        st(&e["symbol"]),
        layer,
        role_override.map_or(String::new(), |r| format!("~{r}"))
    );
    let (layer_role, layer_lt, _) = b.declared[layer].clone();
    if !blocks.iter().any(|sb| sb.name == name) {
        let (record, block, end) = (b.handles.next(), b.handles.next(), b.handles.next());
        let mut body = Out::default();
        let insertion = symbol.get("insertion").map_or([0.0, 0.0], p);
        block_begin(&mut body, &block, &record, &name, insertion);
        let at = Place {
            owner: &record,
            paper: false,
        };
        for part in arr(&symbol["geometry"]) {
            let role = part
                .get("role")
                .or(e.get("role"))
                .and_then(|r| Role::parse(st(r)))
                .unwrap_or(layer_role);
            let width = b.group.width(role);
            let line_type = part
                .get("line_type")
                .map_or(layer_lt.as_str(), st)
                .to_string();
            match st(&part["type"]) {
                "line" => write_line(
                    b,
                    &mut body,
                    at,
                    layer,
                    &line_type,
                    width,
                    p(&part["from"]),
                    p(&part["to"]),
                    &[],
                ),
                "polyline" => write_polyline(
                    b,
                    &mut body,
                    at,
                    layer,
                    &line_type,
                    width,
                    &points(&part["points"]),
                    part.get("closed").and_then(Value::as_bool).unwrap_or(false),
                    &[],
                ),
                "circle" => write_circle(
                    b,
                    &mut body,
                    at,
                    layer,
                    &line_type,
                    width,
                    p(&part["centre"]),
                    f(&part["radius"]),
                    &[],
                ),
                "arc" => write_arc(
                    b,
                    &mut body,
                    at,
                    layer,
                    &line_type,
                    width,
                    p(&part["centre"]),
                    f(&part["radius"]),
                    f(&part["start_deg"]).rem_euclid(360.0),
                    f(&part["end_deg"]).rem_euclid(360.0),
                    &[],
                ),
                _ => {
                    let tags = [
                        ("type", Some("hatch".to_string())),
                        ("pattern", Some("solid".to_string())),
                        ("angle", Some("0.0".to_string())),
                    ];
                    write_hatch(
                        b,
                        &mut body,
                        at,
                        layer,
                        width,
                        &points(&part["boundary"]),
                        &[],
                        true,
                        &tags,
                    );
                }
            }
        }
        block_end(&mut body, &end, &record);
        blocks.push(SymbolBlock {
            name: name.clone(),
            record,
            body: body.0,
        });
    }
    let placement = Placement::of(e, symbol, n);
    let owner = b.rec_model.clone();
    b.head(out, "INSERT", &owner, false, layer);
    out.s(100, "AcDbBlockReference");
    out.s(2, &name);
    out.pt3(10, f(&e["position_m"][0]), f(&e["position_m"][1]));
    out.f(41, placement.factor);
    out.f(42, placement.factor);
    out.f(43, placement.factor);
    out.f(50, placement.rotation);
    xdata(
        out,
        &[
            ("vp", Some(vid.to_string())),
            ("id", id_tag(e)),
            ("type", Some("symbol".to_string())),
        ],
    );
    if let Some(label) = e.get("label") {
        let slot = &symbol["label_slot"];
        let at = placement.model(p(&slot["at"]));
        let height_mm = slot.get("height_mm").map_or(3.5, f);
        let at_place = Place {
            owner: &owner,
            paper: false,
        };
        write_text(
            b,
            out,
            at_place,
            layer,
            st(label),
            at,
            height_mm * f64::from(n) / 1000.0,
            (0.0 - rotation).rem_euclid(360.0),
            AlignH::parse(slot.get("align_h").map_or("centre", st)),
            AlignV::parse(slot.get("align_v").map_or("middle", st)),
            &[
                ("vp", Some(vid.to_string())),
                ("type", Some("text".to_string())),
                ("part", Some("symbol".to_string())),
                ("paper_height", Some(angle_text(height_mm))),
            ],
        );
    }
}

fn paper_entity(b: &mut Builder, out: &mut Out, at: Place, e: &Entity) {
    let h = b.height;
    let flip = |q: [f64; 2]| [q[0], h - q[1]];
    let id = e.id.clone();
    let layer = e.layer.as_str();
    let tags = |kind: &str| {
        [
            ("origin", Some(e.origin.clone())),
            ("id", id.clone()),
            ("type", Some(kind.to_string())),
        ]
    };
    match &e.shape {
        Shape::Text {
            anchor,
            text,
            height,
            rotation,
            align_h,
            align_v,
        } => write_text(
            b,
            out,
            at,
            layer,
            text,
            flip(*anchor),
            *height,
            *rotation,
            *align_h,
            *align_v,
            &tags("text"),
        ),
        Shape::Hatch {
            boundary,
            holes,
            pattern,
            angle: a,
            width,
        } => {
            let holes: Vec<Vec<[f64; 2]>> = holes
                .iter()
                .map(|l| l.iter().map(|q| flip(*q)).collect())
                .collect();
            let b2: Vec<[f64; 2]> = boundary.iter().map(|q| flip(*q)).collect();
            let t = [
                ("origin", Some(e.origin.clone())),
                ("id", id.clone()),
                ("type", Some("hatch".to_string())),
                ("pattern", Some(pattern.clone())),
                ("angle", Some(angle_text(*a))),
            ];
            write_hatch(
                b,
                out,
                at,
                layer,
                *width,
                &b2,
                &holes,
                pattern == "solid",
                &t,
            );
        }
        Shape::Line {
            from,
            to,
            line_type,
            width,
        } => write_line(
            b,
            out,
            at,
            layer,
            line_type,
            *width,
            flip(*from),
            flip(*to),
            &tags("line"),
        ),
        Shape::Polyline {
            points: pts,
            closed,
            line_type,
            width,
        } => {
            let pts: Vec<[f64; 2]> = pts.iter().map(|q| flip(*q)).collect();
            write_polyline(
                b,
                out,
                at,
                layer,
                line_type,
                *width,
                &pts,
                *closed,
                &tags("polyline"),
            );
        }
        Shape::Circle {
            centre,
            radius,
            line_type,
            width,
        } => write_circle(
            b,
            out,
            at,
            layer,
            line_type,
            *width,
            flip(*centre),
            *radius,
            &tags("circle"),
        ),
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
            line_type,
            width,
        } => {
            // The angle is a visual counter-clockwise angle, which the flip of y keeps.
            write_arc(
                b,
                out,
                at,
                layer,
                line_type,
                *width,
                flip(*centre),
                *radius,
                *start,
                (start + sweep).rem_euclid(360.0),
                &tags("arc"),
            );
        }
    }
}
