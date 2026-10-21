//! Drawing document, checks, normalised content (clauses 4 to 9, 16 and 17.3).

use crate::data::{self, LineGroup, Role};
use crate::dimensions::{self, ChainInfo, Context as DimContext, DimResult};
use crate::entities::{self, Entity, num};
use crate::furniture;
use crate::hatch;
use crate::numbers::{FIT_TOL_MM, cos_sin};
use crate::read::{
    arr, entities_of, f, orientation_of, p, points, rect_of, rotation_of, st, window_of,
};
use crate::rules::{self, AlignH, AlignV, Mapper, Rect};
use crate::schema::Schema;
use crate::symbols::{self, Libraries, SymbolContext};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// The furniture layer (clause 16.6.1). It is declared without the document declaring it.
pub const FURNITURE_LAYER: &str = "TITLEBLOCK";
/// The version of the standard that this crate reads (clause 18.2).
pub const SUPPORTED_VERSION: (u32, u32) = (1, 0);

/// Where a runner may read the files that a document names (clause 17.3): symbol libraries and
/// private title block templates. The path is relative to the folder of the document.
pub trait Files {
    fn read(&self, relative_path: &str) -> Option<String>;
}

/// No files: a document that names a file then has error 13.1 or 14.2.
pub struct NoFiles;

impl Files for NoFiles {
    fn read(&self, _relative_path: &str) -> Option<String> {
        None
    }
}

/// The normalised content of clause 16.
#[derive(Debug, Clone, PartialEq)]
pub struct Normalised {
    pub standard_version: String,
    pub sheet: SheetInfo,
    pub viewports: Vec<ViewportInfo>,
    pub entities: Vec<Entity>,
}

/// The `sheet` member of the normalised content (clause 16.2).
#[derive(Debug, Clone, PartialEq)]
pub struct SheetInfo {
    pub size: String,
    pub orientation: String,
    pub line_group: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub drawing_space: Rect,
}

/// A viewport as resolved (clause 16.2).
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportInfo {
    pub id: String,
    pub scale: u32,
    pub frame: Rect,
}

fn rect_value(r: Rect) -> Value {
    let mut m = Map::new();
    m.insert("x_mm".into(), num(r.x));
    m.insert("y_mm".into(), num(r.y));
    m.insert("width_mm".into(), num(r.w));
    m.insert("height_mm".into(), num(r.h));
    Value::Object(m)
}

impl Normalised {
    /// The JSON form of clause 16.2.
    pub fn to_value(&self) -> Value {
        let mut sheet = Map::new();
        sheet.insert("size".into(), self.sheet.size.clone().into());
        sheet.insert("orientation".into(), self.sheet.orientation.clone().into());
        sheet.insert("line_group".into(), self.sheet.line_group.clone().into());
        sheet.insert("width_mm".into(), num(self.sheet.width_mm));
        sheet.insert("height_mm".into(), num(self.sheet.height_mm));
        sheet.insert("drawing_space".into(), rect_value(self.sheet.drawing_space));
        let viewports = self
            .viewports
            .iter()
            .map(|v| {
                let mut m = Map::new();
                m.insert("id".into(), v.id.clone().into());
                m.insert("scale".into(), v.scale.into());
                m.insert("frame".into(), rect_value(v.frame));
                Value::Object(m)
            })
            .collect();
        let mut root = Map::new();
        root.insert(
            "standard_version".into(),
            self.standard_version.clone().into(),
        );
        root.insert("sheet".into(), Value::Object(sheet));
        root.insert("viewports".into(), Value::Array(viewports));
        root.insert(
            "entities".into(),
            Value::Array(self.entities.iter().map(Entity::to_value).collect()),
        );
        Value::Object(root)
    }
}

/// The result of one document (clause 17.3).
#[derive(Debug, Clone, PartialEq)]
pub enum RunResult {
    Ok(Box<Normalised>),
    /// One entry for each different clause number, sorted as text.
    Errors(BTreeSet<String>),
}

impl RunResult {
    /// The result JSON of clause 17.3.
    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        match self {
            RunResult::Ok(n) => {
                m.insert("result".into(), "ok".into());
                m.insert("normalised".into(), n.to_value());
            }
            RunResult::Errors(set) => {
                m.insert("result".into(), "error".into());
                m.insert(
                    "errors".into(),
                    Value::Array(
                        set.iter()
                            .map(|c| {
                                let mut e = Map::new();
                                e.insert("clause".into(), c.clone().into());
                                Value::Object(e)
                            })
                            .collect(),
                    ),
                );
            }
        }
        Value::Object(m)
    }
}

fn error_set(clause: &str) -> RunResult {
    RunResult::Errors(BTreeSet::from([clause.to_string()]))
}

/// Run a document given as text (clause 17.3).
pub fn run_text(text: &str, files: &dyn Files) -> RunResult {
    match crate::json::parse(text) {
        Ok(doc) => run_value(&doc, files),
        Err(crate::json::ParseError::DuplicateKey) => error_set("4.4"),
        Err(crate::json::ParseError::Syntax) => error_set("4.1"),
    }
}

/// Run a parsed document (clause 17.3): the schema, then the checks, then the normalised content.
pub fn run_value(doc: &Value, files: &dyn Files) -> RunResult {
    let errors = Schema::new(data::document_schema()).clauses(doc);
    if !errors.is_empty() {
        return RunResult::Errors(errors);
    }
    let errors = check_document(doc, files);
    if !errors.is_empty() {
        return RunResult::Errors(errors);
    }
    RunResult::Ok(Box::new(normalise_document(doc, files)))
}

/// The declared layers: name to role, line type and CAD colour (clause 8.5).
pub type Layers = BTreeMap<String, (Role, String, i64)>;

fn layer_table(doc: &Value, errors: &mut BTreeSet<String>) -> Layers {
    let styles = data::line_styles();
    let mut declared = Layers::new();
    let mut seen = BTreeSet::new();
    for item in doc["layers"].as_array().map_or(&[][..], Vec::as_slice) {
        let name = st(&item["name"]).to_string();
        if !seen.insert(name.clone()) {
            errors.insert("8.5.5".into());
            continue;
        }
        if let Some(std) = styles.layer(&name) {
            if item
                .get("role")
                .is_some_and(|r| Role::parse(st(r)) != Some(std.role))
            {
                errors.insert("8.5.3".into());
            }
            if item
                .get("line_type")
                .is_some_and(|l| st(l) != std.line_type)
            {
                errors.insert("8.5.3".into());
            }
            if item.get("aci").is_some_and(|a| a.as_i64() != Some(std.aci)) {
                errors.insert("8.5.3".into());
            }
            declared.insert(name, (std.role, std.line_type.clone(), std.aci));
        } else {
            let role = Role::parse(st(&item["role"])).unwrap_or(Role::Thin);
            declared.insert(
                name,
                (
                    role,
                    st(&item["line_type"]).to_string(),
                    item["aci"].as_i64().unwrap_or(0),
                ),
            );
        }
    }
    let colours: Vec<i64> = declared.values().map(|v| v.2).collect();
    let unique: BTreeSet<i64> = colours.iter().copied().collect();
    if unique.len() != colours.len() {
        errors.insert("8.6.3".into());
    }
    declared
}

pub(crate) fn line_group_of(size: &str) -> &'static LineGroup {
    data::line_styles()
        .group_for(size)
        .expect("a size accepted by the schema has a line group")
}

// ---------------------------------------------------------------------------------------------
// the checks

fn inside_frame(px: f64, py: f64, frame: Rect) -> bool {
    px >= frame.x - FIT_TOL_MM
        && px <= frame.x + frame.w + FIT_TOL_MM
        && py >= frame.y - FIT_TOL_MM
        && py <= frame.y + frame.h + FIT_TOL_MM
}

fn arc_points(centre: [f64; 2], r: f64, start: f64, sweep: f64) -> Vec<[f64; 2]> {
    let mut angles = vec![start, start + sweep];
    angles.extend(
        [0.0, 90.0, 180.0, 270.0]
            .into_iter()
            .filter(|a| (a - start).rem_euclid(360.0) <= sweep),
    );
    angles
        .into_iter()
        .map(|a| {
            let (c, s) = cos_sin(a);
            [centre[0] + r * c, centre[1] - r * s]
        })
        .collect()
}

/// The points of an entity that clause 7.7.2 tests against the frame.
fn containment_points(e: &Value, mapper: &Mapper, rotation: f64) -> Vec<[f64; 2]> {
    match st(&e["type"]) {
        "line" | "centre_line" => vec![mapper.point(p(&e["from_m"])), mapper.point(p(&e["to_m"]))],
        "polyline" => e["points_m"]
            .as_array()
            .map_or(vec![], |a| a.iter().map(|q| mapper.point(p(q))).collect()),
        "hatch" => e["boundary_m"]
            .as_array()
            .map_or(vec![], |a| a.iter().map(|q| mapper.point(p(q))).collect()),
        "circle" => {
            let [cx, cy] = mapper.point(p(&e["centre_m"]));
            let r = f(&e["radius_m"]) * mapper.k;
            vec![[cx - r, cy], [cx + r, cy], [cx, cy - r], [cx, cy + r]]
        }
        "arc" => {
            let c = mapper.point(p(&e["centre_m"]));
            let r = f(&e["radius_m"]) * mapper.k;
            let sweep = rules::arc_sweep(f(&e["start_deg"]), f(&e["end_deg"])).unwrap_or(0.0);
            arc_points(
                c,
                r,
                (f(&e["start_deg"]) + rotation).rem_euclid(360.0),
                sweep,
            )
        }
        "text" => {
            let anchor = mapper.point(p(&e["anchor_m"]));
            rules::text_box(
                anchor,
                st(&e["text"]),
                f(&e["height_mm"]),
                e.get("rotation_deg").map_or(0.0, f),
                AlignH::parse(e.get("align_h").map_or("left", st)),
                AlignV::parse(e.get("align_v").map_or("baseline", st)),
            )
            .map_or(vec![], |c| c.to_vec())
        }
        _ => vec![],
    }
}

fn same_point(a: &Value, b: &Value) -> bool {
    p(a) == p(b)
}

fn check_entity(
    e: &Value,
    declared: &Layers,
    mapper: Option<&Mapper>,
    rotation: f64,
    frame: Rect,
    errors: &mut BTreeSet<String>,
) {
    let t = st(&e["type"]);
    if !declared.contains_key(st(&e["layer"])) {
        errors.insert("8.5.1".into());
    }
    if matches!(t, "line" | "centre_line") && same_point(&e["from_m"], &e["to_m"]) {
        errors.insert("9.3".into());
    }
    if t == "polyline" {
        let pts = points(&e["points_m"]);
        let n = pts.len();
        if (0..n.saturating_sub(1)).any(|i| pts[i] == pts[i + 1])
            || (e.get("closed").and_then(Value::as_bool) == Some(true)
                && n > 0
                && pts[0] == pts[n - 1])
        {
            errors.insert("9.4.1".into());
        }
    }
    if t == "arc" && rules::arc_sweep(f(&e["start_deg"]), f(&e["end_deg"])).is_none() {
        errors.insert("9.5.1".into());
        return;
    }
    if t == "hatch" {
        let holes: Vec<Vec<[f64; 2]>> = e
            .get("holes_m")
            .map_or(vec![], |h| arr(h).iter().map(points).collect());
        if !hatch::hatch_ok(&points(&e["boundary_m"]), &holes) {
            errors.insert("10.3".into());
        }
    }
    let Some(mapper) = mapper else { return };
    for [px, py] in containment_points(e, mapper, rotation) {
        if !inside_frame(px, py, frame) {
            errors.insert("7.7.1".into());
            break;
        }
    }
}

fn resolved_scale(vp: &Value) -> Option<u32> {
    let window = window_of(&vp["window"]);
    if window.xmax <= window.xmin || window.ymax <= window.ymin {
        return None;
    }
    match &vp["scale"] {
        Value::String(_) => rules::fit_scale(rect_of(&vp["frame"]), window, rotation_of(vp)),
        n => n
            .as_u64()
            .map(|x| x as u32)
            .or_else(|| n.as_f64().map(|x| x as u32)),
    }
}

fn check_scale_bar(
    vp: &Value,
    frame: Rect,
    denominator: Option<u32>,
    errors: &mut BTreeSet<String>,
) {
    if frame.h < rules::MIN_BAR_FRAME_HEIGHT {
        errors.insert("7.9.10".into());
    }
    let Some(n) = denominator else { return };
    let k = 1000.0 / f64::from(n);
    match vp["scale_bar"].get("length_m") {
        Some(l) => {
            if f(l) * k > rules::bar_available(frame.w) + FIT_TOL_MM {
                errors.insert("7.9.3".into());
            }
        }
        None => {
            if rules::derived_bar_length(frame.w, n).is_none() {
                errors.insert("7.9.4".into());
            }
        }
    }
}

fn parse_version(text: &str) -> (u32, u32) {
    let mut parts = text.split('.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor)
}

/// Every error clause of a document that passed the schema (clause 4.6).
pub fn check_document(doc: &Value, files: &dyn Files) -> BTreeSet<String> {
    let mut errors = BTreeSet::new();
    let (major, minor) = parse_version(st(&doc["standard_version"]));
    if major != SUPPORTED_VERSION.0 || minor > SUPPORTED_VERSION.1 {
        return BTreeSet::from(["18.2".to_string()]);
    }
    let sheet = &doc["sheet"];
    let size = st(&sheet["size"]);
    let landscape = orientation_of(sheet) == "landscape";
    let group = line_group_of(size);
    if sheet.get("line_group").is_some_and(|g| st(g) != group.name) {
        errors.insert("8.2.2".into());
    }
    let Some((_, _, ds)) = rules::drawing_space(size, landscape) else {
        return errors;
    };
    if rules::vertical_zones(size, landscape).is_some_and(|n| n > rules::ZONE_LETTERS.len()) {
        // Clause 6.5.2 has 16 letters, and an A0 sheet in portrait needs 24 vertical zones: the
        // standard has no label for the others, so the sheet is refused, not guessed.
        errors.insert("6.5.2".into());
    }
    let declared = layer_table(doc, &mut errors);
    let (libs, lib_errors) = symbols::load_libraries(doc, files);
    errors.extend(lib_errors.iter().cloned());

    let mut ids: Vec<String> = Vec::new();
    let mut frames: Vec<Rect> = Vec::new();
    for vp in arr(&doc["viewports"]) {
        ids.push(st(&vp["id"]).to_string());
        let frame = rect_of(&vp["frame"]);
        frames.push(frame);
        if frame.x < ds.x - FIT_TOL_MM
            || frame.y < ds.y - FIT_TOL_MM
            || frame.x + frame.w > ds.x + ds.w + FIT_TOL_MM
            || frame.y + frame.h > ds.y + ds.h + FIT_TOL_MM
        {
            errors.insert("7.2.2".into());
        }
        let window = window_of(&vp["window"]);
        let rotation = rotation_of(vp);
        let window_ok = window.xmax > window.xmin && window.ymax > window.ymin;
        if !window_ok {
            errors.insert("7.6.1".into());
        }
        let mut denominator = None;
        if window_ok {
            if vp["scale"].is_string() {
                denominator = rules::fit_scale(frame, window, rotation);
                if denominator.is_none() {
                    errors.insert("7.5.2".into());
                }
            } else {
                let n = resolved_scale(vp).unwrap_or(1);
                denominator = Some(n);
                if !rules::window_fits(frame, window, n, rotation) {
                    errors.insert("7.5.1".into());
                }
            }
        }
        if vp.get("scale_bar").is_some() {
            check_scale_bar(vp, frame, denominator, &mut errors);
        }
        if vp.get("north").is_some()
            && (frame.w < rules::MIN_NORTH_FRAME || frame.h < rules::MIN_NORTH_FRAME)
        {
            errors.insert("7.10.7".into());
        }
        let mapper = denominator.map(|n| Mapper::new(frame, window, n, rotation));
        let index: HashMap<String, &Value> = entities_of(vp)
            .iter()
            .filter_map(|e| e.get("id").map(|id| (st(id).to_string(), e)))
            .collect();
        let mut chains: Vec<ChainInfo> = Vec::new();
        for e in entities_of(vp) {
            if let Some(id) = e.get("id") {
                ids.push(st(id).to_string());
            }
            check_entity(e, &declared, mapper.as_ref(), rotation, frame, &mut errors);
            let (Some(mapper), Some(n)) = (mapper.as_ref(), denominator) else {
                continue;
            };
            let layer_declared = declared.contains_key(st(&e["layer"]));
            match st(&e["type"]) {
                "symbol" => match libs.symbol(st(&e["library"]), st(&e["symbol"])) {
                    None => {
                        if lib_errors.is_empty() {
                            errors.insert("13.3.2".into());
                        }
                    }
                    Some(symbol) if layer_declared => {
                        let ctx =
                            symbol_context(e, &declared, group, mapper, rotation, st(&vp["id"]), n);
                        let (parts, errs) = symbols::parts(e, symbol, &ctx);
                        errors.extend(errs);
                        if parts
                            .iter()
                            .flat_map(dimensions::part_points)
                            .any(|[px, py]| !inside_frame(px, py, frame))
                        {
                            errors.insert("7.7.1".into());
                        }
                    }
                    Some(_) => {}
                },
                "dimension" | "leader" if layer_declared => {
                    let res = dimension_result(
                        e,
                        &declared,
                        group,
                        mapper,
                        rotation,
                        st(&vp["id"]),
                        &index,
                        &libs,
                        n,
                    );
                    errors.extend(res.errors.iter().cloned());
                    if res.errors.is_empty() {
                        if res
                            .entities
                            .iter()
                            .flat_map(dimensions::part_points)
                            .any(|[px, py]| !inside_frame(px, py, frame))
                        {
                            errors.insert("7.7.1".into());
                        }
                        chains.extend(res.chain);
                    }
                }
                _ => {}
            }
        }
        if !dimensions::check_chains(&chains) {
            errors.insert("12.8".into());
        }
    }
    let unique: BTreeSet<&String> = ids.iter().collect();
    if unique.len() != ids.len() {
        errors.insert("9.8".into());
    }
    let scales: Vec<Option<u32>> = arr(&doc["viewports"]).iter().map(resolved_scale).collect();
    let plan = furniture::plan(doc, size, ds, group, &scales, files);
    errors.extend(plan.errors.iter().cloned());
    for frame in &frames {
        if plan.regions.iter().any(|(_, r)| overlap(*frame, *r)) {
            errors.insert("7.2.4".into());
        }
    }
    for i in 0..frames.len() {
        for j in (i + 1)..frames.len() {
            if overlap(frames[i], frames[j]) {
                errors.insert("7.2.3".into());
            }
        }
    }
    errors
}

fn symbol_context<'a>(
    e: &Value,
    declared: &'a Layers,
    group: &'a LineGroup,
    mapper: &'a Mapper,
    rotation: f64,
    vid: &str,
    denominator: u32,
) -> SymbolContext<'a> {
    let (role, line_type, _) = &declared[st(&e["layer"])];
    SymbolContext {
        mapper,
        rotation,
        denominator,
        group,
        layer_role: *role,
        layer_line_type: line_type,
        origin: format!("viewport.{vid}"),
    }
}

#[allow(clippy::too_many_arguments)]
fn dimension_result(
    e: &Value,
    declared: &Layers,
    group: &LineGroup,
    mapper: &Mapper,
    rotation: f64,
    vid: &str,
    index: &HashMap<String, &Value>,
    libs: &Libraries,
    denominator: u32,
) -> DimResult {
    let role = e
        .get("role")
        .and_then(|r| Role::parse(st(r)))
        .unwrap_or(declared[st(&e["layer"])].0);
    let ctx = DimContext {
        mapper,
        rotation,
        unit_mm: hatch::unit_mm(group.thin),
        width: group.width(role),
        origin: format!("viewport.{vid}"),
        index,
        libs,
        denominator,
    };
    if st(&e["type"]) == "dimension" {
        dimensions::dimension(e, &ctx)
    } else {
        dimensions::leader(e, &ctx)
    }
}

/// Two rectangles overlap by more than the tolerance in both directions (clauses 7.2.3, 7.2.5).
pub(crate) fn overlap(a: Rect, b: Rect) -> bool {
    let ox = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
    let oy = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
    ox > FIT_TOL_MM && oy > FIT_TOL_MM
}

// ---------------------------------------------------------------------------------------------
// normalised content (clause 16)

fn sheet_furniture(size: &str, landscape: bool, group: &LineGroup) -> Vec<Entity> {
    let (w, h, _) = rules::drawing_space(size, landscape).expect("a known size");
    let (l, t, r, b) = rules::insets_mm(size).expect("a known size");
    let (long_n, short_n) = rules::zones(size).expect("a known size");
    let (nh, nv) = if landscape {
        (long_n, short_n)
    } else {
        (short_n, long_n)
    };
    let zw = (w - l - r) / nh as f64;
    let zh = (h - t - b) / nv as f64;
    let lay = FURNITURE_LAYER;
    let solid = "continuous";
    let mut out = vec![Entity::polyline(
        "sheet.border",
        lay,
        &[[l, t], [w - r, t], [w - r, h - b], [l, h - b]],
        true,
        solid,
        group.extra_thick,
        None,
    )];
    for (a, b2) in [
        ([w / 2.0, 0.0], [w / 2.0, t + 5.0]),
        ([w / 2.0, h], [w / 2.0, h - b - 5.0]),
        ([0.0, h / 2.0], [l + 5.0, h / 2.0]),
        ([w, h / 2.0], [w - r - 5.0, h / 2.0]),
    ] {
        out.push(Entity::line(
            "sheet.centring_mark",
            lay,
            a,
            b2,
            solid,
            0.5,
            None,
        ));
    }
    for k in 1..nh {
        let x = l + k as f64 * zw;
        out.push(Entity::line(
            "sheet.zone_mark",
            lay,
            [x, 0.0],
            [x, t],
            solid,
            0.35,
            None,
        ));
        out.push(Entity::line(
            "sheet.zone_mark",
            lay,
            [x, h],
            [x, h - b],
            solid,
            0.35,
            None,
        ));
    }
    for j in 1..nv {
        let y = t + j as f64 * zh;
        out.push(Entity::line(
            "sheet.zone_mark",
            lay,
            [0.0, y],
            [l, y],
            solid,
            0.35,
            None,
        ));
        out.push(Entity::line(
            "sheet.zone_mark",
            lay,
            [w, y],
            [w - r, y],
            solid,
            0.35,
            None,
        ));
    }
    for k in 1..=nh {
        let x = l + (k as f64 - 0.5) * zw;
        let label = k.to_string();
        out.push(Entity::text(
            "sheet.zone_label",
            lay,
            [x, t / 2.0],
            &label,
            3.5,
            0.0,
            AlignH::Centre,
            AlignV::Middle,
            None,
        ));
        out.push(Entity::text(
            "sheet.zone_label",
            lay,
            [x, h - b / 2.0],
            &label,
            3.5,
            0.0,
            AlignH::Centre,
            AlignV::Middle,
            None,
        ));
    }
    let letters: Vec<char> = rules::ZONE_LETTERS
        .chars()
        .chain(std::iter::repeat('?'))
        .take(nv)
        .collect();
    for j in 1..=nv {
        let y = t + (j as f64 - 0.5) * zh;
        let label = letters[j - 1].to_string();
        out.push(Entity::text(
            "sheet.zone_label",
            lay,
            [l / 2.0, y],
            &label,
            3.5,
            0.0,
            AlignH::Centre,
            AlignV::Middle,
            None,
        ));
        out.push(Entity::text(
            "sheet.zone_label",
            lay,
            [w - r / 2.0, y],
            &label,
            3.5,
            0.0,
            AlignH::Centre,
            AlignV::Middle,
            None,
        ));
    }
    out
}

fn scale_bar(vid: &str, frame: Rect, denominator: u32, sb: &Value, thin: f64) -> Vec<Entity> {
    let k = 1000.0 / f64::from(denominator);
    let length = sb
        .get("length_m")
        .map(f)
        .or_else(|| rules::derived_bar_length(frame.w, denominator))
        .unwrap_or(0.0);
    let div = sb.get("divisions").and_then(Value::as_u64).unwrap_or(5) as u32;
    let unit = sb.get("unit").map_or("m", st);
    let total = length * k;
    let seg = total / f64::from(div);
    let x0 = frame.x + rules::BAR_INSET;
    let y0 = frame.y + frame.h - 18.0;
    let yb = y0 + rules::BAR_HEIGHT;
    let origin = format!("viewport.{vid}.scale_bar");
    let (lay, solid) = (FURNITURE_LAYER, "continuous");
    let mut out = vec![Entity::polyline(
        &origin,
        lay,
        &[[x0, y0], [x0 + total, y0], [x0 + total, yb], [x0, yb]],
        true,
        solid,
        thin,
        None,
    )];
    for i in 1..div {
        let x = x0 + f64::from(i) * seg;
        out.push(Entity::line(
            &origin,
            lay,
            [x, y0],
            [x, yb],
            solid,
            thin,
            None,
        ));
    }
    for i in (0..div).step_by(2) {
        let a = x0 + f64::from(i) * seg;
        let b = x0 + f64::from(i + 1) * seg;
        out.push(Entity::hatch(
            &origin,
            lay,
            &[[a, y0], [b, y0], [b, yb], [a, yb]],
            &[],
            "solid",
            0.0,
            thin,
            None,
        ));
    }
    for i in 0..=div {
        let x = x0 + f64::from(i) * seg;
        out.push(Entity::line(
            &origin,
            lay,
            [x, yb],
            [x, yb + rules::TICK_LENGTH],
            solid,
            thin,
            None,
        ));
    }
    for (i, label) in rules::bar_labels(length, div, unit).iter().enumerate() {
        let x = x0 + i as f64 * seg;
        out.push(Entity::text(
            &origin,
            lay,
            [x, yb + rules::BAR_LABEL_BASELINE],
            label,
            rules::BAR_TEXT_HEIGHT,
            0.0,
            AlignH::Centre,
            AlignV::Baseline,
            None,
        ));
    }
    out.push(Entity::text(
        &origin,
        lay,
        [x0 + total / 2.0, yb + rules::BAR_RATIO_BASELINE],
        &format!("1:{denominator}"),
        rules::BAR_TEXT_HEIGHT,
        0.0,
        AlignH::Centre,
        AlignV::Baseline,
        None,
    ));
    out
}

fn north_arrow(vid: &str, frame: Rect, rotation: f64, nb: &Value, thin: f64) -> Vec<Entity> {
    let cx = frame.x + rules::NORTH_INSET + rules::NORTH_SIZE / 2.0;
    let cy = frame.y + rules::NORTH_INSET + rules::NORTH_SIZE / 2.0;
    let phi = rules::north_deg(rotation, nb.get("model_bearing_deg").map_or(0.0, f));
    let (c, s) = cos_sin(phi);
    let turn = |q: [f64; 2]| {
        let dx = q[0] - cx;
        let dy = q[1] - cy;
        [cx + dx * c - dy * s, cy + dx * s + dy * c]
    };
    let origin = format!("viewport.{vid}.north_arrow");
    let lay = FURNITURE_LAYER;
    let head = [
        turn([cx, cy - 9.0]),
        turn([cx - 4.14, cy + 5.76]),
        turn([cx, cy + 0.9]),
        turn([cx + 4.14, cy + 5.76]),
    ];
    let label = nb.get("label").map_or("N", st);
    vec![
        Entity::hatch(&origin, lay, &head, &[], "solid", 0.0, thin, None),
        Entity::line(
            &origin,
            lay,
            turn([cx, cy + 0.9]),
            turn([cx, cy + 9.0]),
            "continuous",
            thin,
            None,
        ),
        Entity::text(
            &origin,
            lay,
            turn([cx, cy - 11.5]),
            label,
            rules::NORTH_LABEL_HEIGHT,
            0.0,
            AlignH::Centre,
            AlignV::Baseline,
            None,
        ),
    ]
}

/// The line type and width of an entity of a viewport (clauses 8.7 and 9.7).
pub(crate) fn entity_style(e: &Value, declared: &Layers, group: &LineGroup) -> (String, f64) {
    let (layer_role, layer_lt, _) = &declared[st(&e["layer"])];
    let is_centre = st(&e["type"]) == "centre_line";
    let role = e
        .get("role")
        .and_then(|r| Role::parse(st(r)))
        .unwrap_or(if is_centre { Role::Thin } else { *layer_role });
    let line_type = e.get("line_type").map(st).map_or_else(
        || {
            if is_centre {
                "chain".to_string()
            } else {
                layer_lt.clone()
            }
        },
        str::to_string,
    );
    (line_type, group.width(role))
}

fn viewport_entities(
    vp: &Value,
    declared: &Layers,
    group: &LineGroup,
    mapper: &Mapper,
    rotation: f64,
) -> Vec<Entity> {
    let origin = format!("viewport.{}", st(&vp["id"]));
    let mut out = Vec::new();
    for e in entities_of(vp) {
        let t = st(&e["type"]);
        if matches!(t, "dimension" | "leader" | "symbol") {
            continue; // their parts are made by `dimensions` and `symbols` (clauses 12.7 and 13.7)
        }
        let (line_type, width) = entity_style(e, declared, group);
        let layer = st(&e["layer"]);
        let id = e.get("id").map(st);
        match t {
            "line" | "centre_line" => out.push(Entity::line(
                &origin,
                layer,
                mapper.point(p(&e["from_m"])),
                mapper.point(p(&e["to_m"])),
                &line_type,
                width,
                id,
            )),
            "polyline" => {
                let pts: Vec<[f64; 2]> = points(&e["points_m"])
                    .into_iter()
                    .map(|q| mapper.point(q))
                    .collect();
                out.push(Entity::polyline(
                    &origin,
                    layer,
                    &pts,
                    e.get("closed").and_then(Value::as_bool).unwrap_or(false),
                    &line_type,
                    width,
                    id,
                ));
            }
            "circle" => out.push(Entity::circle(
                &origin,
                layer,
                mapper.point(p(&e["centre_m"])),
                f(&e["radius_m"]) * mapper.k,
                &line_type,
                width,
                id,
            )),
            "arc" => {
                let sweep = rules::arc_sweep(f(&e["start_deg"]), f(&e["end_deg"])).unwrap_or(0.0);
                out.push(Entity::arc(
                    &origin,
                    layer,
                    mapper.point(p(&e["centre_m"])),
                    f(&e["radius_m"]) * mapper.k,
                    f(&e["start_deg"]) + rotation,
                    sweep,
                    &line_type,
                    width,
                    id,
                ));
            }
            "hatch" => {
                let boundary: Vec<[f64; 2]> = points(&e["boundary_m"])
                    .into_iter()
                    .map(|q| mapper.point(q))
                    .collect();
                let holes: Vec<Vec<[f64; 2]>> = e.get("holes_m").map_or(vec![], |h| {
                    arr(h)
                        .iter()
                        .map(|l| points(l).into_iter().map(|q| mapper.point(q)).collect())
                        .collect()
                });
                let item = Entity::hatch(
                    &origin,
                    layer,
                    &boundary,
                    &holes,
                    st(&e["pattern"]),
                    e.get("angle_deg").map_or(0.0, f) + rotation,
                    width,
                    id,
                );
                let marks = hatch::marks(&item, &format!("{origin}.hatch"), group.thin);
                out.push(item);
                out.extend(marks);
            }
            "text" => out.push(Entity::text(
                &origin,
                layer,
                mapper.point(p(&e["anchor_m"])),
                st(&e["text"]),
                f(&e["height_mm"]),
                e.get("rotation_deg").map_or(0.0, f),
                AlignH::parse(e.get("align_h").map_or("left", st)),
                AlignV::parse(e.get("align_v").map_or("baseline", st)),
                id,
            )),
            _ => {}
        }
    }
    out
}

/// The declared layers of a document (clause 8.5).
pub(crate) fn declared_layers(doc: &Value) -> Layers {
    layer_table(doc, &mut BTreeSet::new())
}

/// The normalised content of a document that has no errors (clause 16).
pub fn normalise_document(doc: &Value, files: &dyn Files) -> Normalised {
    let sheet = &doc["sheet"];
    let size = st(&sheet["size"]);
    let landscape = orientation_of(sheet) == "landscape";
    let (w, h, ds) = rules::drawing_space(size, landscape).expect("a known size");
    let group = line_group_of(size);
    let mut scratch = BTreeSet::new();
    let declared = layer_table(doc, &mut scratch);
    let (libs, _) = symbols::load_libraries(doc, files);
    let mut entities = sheet_furniture(size, landscape, group);
    let scales: Vec<Option<u32>> = arr(&doc["viewports"]).iter().map(resolved_scale).collect();
    entities.extend(furniture::plan(doc, size, ds, group, &scales, files).entities);
    let mut viewports = Vec::new();
    for vp in doc["viewports"].as_array().map_or(&[][..], Vec::as_slice) {
        let frame = rect_of(&vp["frame"]);
        let rotation = rotation_of(vp);
        let denominator = resolved_scale(vp).unwrap_or(1);
        let mapper = Mapper::new(frame, window_of(&vp["window"]), denominator, rotation);
        viewports.push(ViewportInfo {
            id: st(&vp["id"]).to_string(),
            scale: denominator,
            frame,
        });
        entities.extend(viewport_entities(vp, &declared, group, &mapper, rotation));
        let index: HashMap<String, &Value> = entities_of(vp)
            .iter()
            .filter_map(|e| e.get("id").map(|id| (st(id).to_string(), e)))
            .collect();
        for e in entities_of(vp) {
            match st(&e["type"]) {
                "dimension" | "leader" => entities.extend(
                    dimension_result(
                        e,
                        &declared,
                        group,
                        &mapper,
                        rotation,
                        st(&vp["id"]),
                        &index,
                        &libs,
                        denominator,
                    )
                    .entities,
                ),
                "symbol" => {
                    if let Some(symbol) = libs.symbol(st(&e["library"]), st(&e["symbol"])) {
                        let ctx = symbol_context(
                            e,
                            &declared,
                            group,
                            &mapper,
                            rotation,
                            st(&vp["id"]),
                            denominator,
                        );
                        entities.extend(symbols::parts(e, symbol, &ctx).0);
                    }
                }
                _ => {}
            }
        }
        if let Some(sb) = vp.get("scale_bar") {
            entities.extend(scale_bar(st(&vp["id"]), frame, denominator, sb, group.thin));
        }
        if let Some(nb) = vp.get("north") {
            entities.extend(north_arrow(st(&vp["id"]), frame, rotation, nb, group.thin));
        }
    }
    entities.sort_by(entities::compare);
    viewports.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    Normalised {
        standard_version: st(&doc["standard_version"]).to_string(),
        sheet: SheetInfo {
            size: size.to_string(),
            orientation: if landscape { "landscape" } else { "portrait" }.to_string(),
            line_group: group.name.clone(),
            width_mm: w,
            height_mm: h,
            drawing_space: ds,
        },
        viewports,
        entities,
    }
}
