//! A small reader of ASCII DXF, written for the tests. It shares no code with the writer
//! (`otd::render::dxf`): it works on the group codes of the file itself, and it rebuilds the
//! normalised content of the standard (clause 16) from the DXF as clause 15.4.11 says: paper-space
//! entities as they are, model-space entities through the viewports (clause 7.4). The entities of
//! an exaggerated viewport are paper-space entities (clause 15.4.13), so they need no map.

// Two test crates include this file, and each uses a part of it.
#![allow(dead_code)]

use otd::entities::{Entity, Paint, Shape};
use otd::numbers::{angle, cos_sin, mm};
use otd::rules::{AlignH, AlignV, Mapper, Rect, Window};
use std::collections::BTreeMap;

pub struct Record {
    pub kind: String,
    pub items: Vec<(i32, String)>,
    pub block: Option<String>,
    pub section: String,
}

impl Record {
    pub fn get(&self, code: i32) -> Option<&str> {
        self.items
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, v)| v.as_str())
    }
    pub fn nth(&self, code: i32, n: usize) -> Option<&str> {
        self.items
            .iter()
            .filter(|(c, _)| *c == code)
            .nth(n)
            .map(|(_, v)| v.as_str())
    }
    pub fn all(&self, code: i32) -> Vec<&str> {
        self.items
            .iter()
            .filter(|(c, _)| *c == code)
            .map(|(_, v)| v.as_str())
            .collect()
    }
    pub fn f(&self, code: i32) -> f64 {
        self.get(code)
            .unwrap_or_else(|| panic!("{} has no group {code}", self.kind))
            .trim()
            .parse()
            .expect("a number")
    }
    pub fn f_or(&self, code: i32, default: f64) -> f64 {
        self.get(code)
            .map_or(default, |v| v.trim().parse().expect("a number"))
    }
    pub fn i_or(&self, code: i32, default: i64) -> i64 {
        self.get(code)
            .map_or(default, |v| v.trim().parse().expect("an integer"))
    }
    /// The `key=value` strings after the 1001 TDFA tag.
    pub fn xdata(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let mut on = false;
        for (c, v) in &self.items {
            if *c == 1001 {
                on = v == "TDFA";
            } else if on
                && *c == 1000
                && let Some((k, val)) = v.split_once('=')
            {
                out.insert(k.to_string(), val.to_string());
            }
        }
        out
    }
}

pub fn tags(text: &str) -> Vec<(i32, String)> {
    let lines: Vec<&str> = text.split('\n').collect();
    (0..lines.len().saturating_sub(1))
        .step_by(2)
        .map(|i| {
            (
                lines[i].trim().parse().expect("a group code"),
                lines[i + 1].trim_end_matches('\r').to_string(),
            )
        })
        .collect()
}

pub fn records(text: &str) -> Vec<Record> {
    let items = tags(text);
    let mut out = Vec::new();
    let mut section = String::new();
    let mut block: Option<String> = None;
    let mut i = 0;
    while i < items.len() {
        let (code, value) = &items[i];
        if *code == 0 && value != "SECTION" && value != "ENDSEC" && value != "EOF" {
            let mut j = i + 1;
            while j < items.len() && items[j].0 != 0 {
                j += 1;
            }
            let rec = Record {
                kind: value.clone(),
                items: items[i + 1..j].to_vec(),
                block: block.clone(),
                section: section.clone(),
            };
            if value == "BLOCK" {
                block = rec.get(2).map(str::to_string);
            } else if value == "ENDBLK" {
                block = None;
            }
            out.push(rec);
            i = j;
        } else if *code == 0 && value == "SECTION" {
            section = items[i + 1].1.clone();
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

fn linetype_id(name: &str) -> String {
    if name.eq_ignore_ascii_case("CONTINUOUS") {
        return "continuous".into();
    }
    let rest = name.strip_prefix("TDFA_").expect("a TDFA line type");
    let (id, digits) = rest.rsplit_once('_').expect("a width");
    assert!(
        digits.len() == 3 && digits.bytes().all(|b| b.is_ascii_digit()),
        "{name}"
    );
    id.to_lowercase()
}

pub fn sheet_size(recs: &[Record]) -> (f64, f64) {
    let layout = recs
        .iter()
        .find(|r| r.kind == "LAYOUT" && r.nth(1, 1) == Some("SHEET"))
        .expect("a layout SHEET");
    (layout.f(44), layout.f(45))
}

fn loops(r: &Record) -> Vec<Vec<[f64; 2]>> {
    let items = &r.items;
    let start = items.iter().position(|(c, _)| *c == 91).expect("91");
    let n_paths: usize = items[start].1.trim().parse().unwrap();
    let mut k = start + 1;
    let mut out = Vec::new();
    for _ in 0..n_paths {
        assert_eq!(items[k].0, 92);
        assert!(
            items[k].1.trim().parse::<i64>().unwrap() & 2 != 0,
            "polyline paths only"
        );
        k += 1;
        assert_eq!(items[k].0, 72);
        k += 1;
        assert_eq!(items[k].0, 73);
        k += 1;
        assert_eq!(items[k].0, 93);
        let count: usize = items[k].1.trim().parse().unwrap();
        k += 1;
        let mut lp = Vec::new();
        for _ in 0..count {
            assert!(items[k].0 == 10 && items[k + 1].0 == 20);
            lp.push([
                items[k].1.trim().parse().unwrap(),
                items[k + 1].1.trim().parse().unwrap(),
            ]);
            k += 2;
        }
        assert!(items[k].0 == 97 && items[k].1.trim() == "0");
        k += 1;
        out.push(lp);
    }
    out
}

fn text_point(r: &Record) -> [f64; 2] {
    if r.get(11).is_some() && (r.i_or(72, 0) != 0 || r.i_or(73, 0) != 0) {
        [r.f(11), r.f(21)]
    } else {
        [r.f(10), r.f(20)]
    }
}

fn halign(v: i64) -> AlignH {
    match v {
        1 => AlignH::Centre,
        2 => AlignH::Right,
        _ => AlignH::Left,
    }
}

fn valign(v: i64) -> AlignV {
    match v {
        2 => AlignV::Middle,
        3 => AlignV::Top,
        _ => AlignV::Baseline,
    }
}

fn style(r: &Record) -> (String, f64) {
    (
        linetype_id(r.get(6).expect("a line type")),
        r.get(370)
            .expect("a line weight")
            .trim()
            .parse::<f64>()
            .unwrap()
            / 100.0,
    )
}

fn pts(r: &Record) -> Vec<[f64; 2]> {
    r.all(10)
        .iter()
        .zip(r.all(20))
        .map(|(x, y)| [x.trim().parse().unwrap(), y.trim().parse().unwrap()])
        .collect()
}

pub struct ViewportBack {
    pub n: u32,
    pub frame: Rect,
    pub centre: [f64; 2],
    pub twist: f64,
    pub view_height: f64,
    pub height: f64,
    /// The vertical exaggeration in the extended data (clause 15.4.8), from version 1.2. The
    /// entities of such a viewport are in paper space (clause 15.4.13).
    pub vertical_exaggeration: Option<u32>,
}

pub struct Back {
    pub width: f64,
    pub height: f64,
    pub viewports: BTreeMap<String, ViewportBack>,
    pub entities: Vec<Entity>,
}

pub fn read_normalised(text: &str) -> Back {
    let recs = records(text);
    let (width, height) = sheet_size(&recs);
    let in_paper = |r: &Record| {
        (r.section == "BLOCKS"
            && r.block
                .as_deref()
                .is_some_and(|b| b.starts_with("*Paper_Space")))
            || (r.section == "ENTITIES" && r.get(67) == Some("1"))
    };
    let paper: Vec<&Record> = recs.iter().filter(|r| in_paper(r)).collect();
    let model: Vec<&Record> = recs
        .iter()
        .filter(|r| r.section == "ENTITIES" && r.get(67) != Some("1"))
        .collect();
    let mut viewports = BTreeMap::new();
    for r in &paper {
        let x = r.xdata();
        if r.kind == "VIEWPORT" && x.contains_key("id") {
            let (cx, cy, w, h) = (r.f(10), r.f(20), r.f(40), r.f(41));
            let n: u32 = x["scale"].parse().unwrap();
            assert_eq!(
                (r.f(45) * 1000.0 / h).round() as u32,
                n,
                "the viewport plots at 1:N"
            );
            viewports.insert(
                x["id"].clone(),
                ViewportBack {
                    n,
                    frame: Rect {
                        x: cx - w / 2.0,
                        y: height - (cy + h / 2.0),
                        w,
                        h,
                    },
                    centre: [r.f(12), r.f(22)],
                    twist: r.f_or(51, 0.0),
                    view_height: r.f(45),
                    height: h,
                    vertical_exaggeration: x
                        .get("vertical_exaggeration")
                        .map(|v| v.parse().expect("a whole number")),
                },
            );
        }
    }
    let mut entities = Vec::new();
    for r in &paper {
        let x = r.xdata();
        if x.contains_key("origin") {
            let mut e = paper_entity(r, &x, height);
            e.paint.print_colour = x.get("print_colour").cloned(); // a plate (8.6.7)
            entities.push(e);
        }
    }
    for r in &model {
        let x = r.xdata();
        let Some(vp) = x.get("vp").and_then(|id| viewports.get(id)) else {
            continue;
        };
        if r.kind == "INSERT" {
            entities.extend(expand_insert(r, &x, vp, &recs));
        } else if r.kind == "IMAGE" {
            entities.push(image_entity(r, &x, vp));
        } else {
            entities.push(model_entity(r, &x, vp, &Identity, 1.0, 0.0, ""));
        }
    }
    apply_layer_colours(&mut entities, &recs);
    Back {
        width,
        height,
        viewports,
        entities,
    }
}

/// Clause 8.6.6 from the DXF: the colour members are extended data of the LAYER records (15.4.12).
fn apply_layer_colours(entities: &mut [Entity], recs: &[Record]) {
    let table: BTreeMap<String, BTreeMap<String, String>> = recs
        .iter()
        .filter(|r| r.kind == "LAYER" && !r.xdata().is_empty())
        .map(|r| (r.get(2).unwrap().to_string(), r.xdata()))
        .collect();
    for e in entities {
        let o = e.origin.as_str();
        let Some(c) = table.get(&e.layer) else {
            continue;
        };
        if matches!(e.shape, Shape::Image { .. })
            || !o.starts_with("viewport.")
            || o.ends_with(".scale_bar")
            || o.ends_with(".vertical_scale_bar")
            || o.ends_with(".north_arrow")
            || o.ends_with(".backing")
        {
            continue;
        }
        let solid = matches!(&e.shape, Shape::Hatch { pattern, .. } if pattern == "solid");
        let text = matches!(e.shape, Shape::Text { .. });
        e.paint = Paint {
            print_colour: c.get("print_colour").cloned(),
            fill_opacity: c
                .get("fill_opacity")
                .filter(|_| solid)
                .map(|v| v.parse().expect("a number")),
            halo_colour: c.get("halo_colour").filter(|_| text).cloned(),
        };
    }
}

/// An IMAGE: the insert point (10, 20), the u and v vectors of one pixel (11, 21 and 12, 22), the
/// size in pixels (13, 23).
fn image_entity(r: &Record, x: &BTreeMap<String, String>, vp: &ViewportBack) -> Entity {
    let window = Window {
        xmin: vp.centre[0],
        xmax: vp.centre[0],
        ymin: vp.centre[1],
        ymax: vp.centre[1],
    };
    let mapper = Mapper::new(vp.frame, window, vp.n, vp.twist);
    let (w, h) = (r.f(13).round() as u32, r.f(23).round() as u32);
    let (x0, y0) = (r.f(10), r.f(20));
    let (x1, y1) = (x0 + r.f(11) * f64::from(w), y0 + r.f(22) * f64::from(h));
    let [px0, py0] = mapper.point([x0, y1]);
    let [px1, py1] = mapper.point([x1, y0]);
    Entity::image(
        &format!("viewport.{}", x["vp"]),
        r.get(8).unwrap(),
        [px0, py0, px1, py1],
        &x["sha256"],
        [w, h],
        x["components"].parse().expect("1 or 3"),
        x["opacity"].parse().expect("a number"),
        x.get("id").filter(|v| *v != "None").map(String::as_str),
    )
}

fn common(
    origin: &str,
    layer: &str,
    x: &BTreeMap<String, String>,
) -> (String, String, Option<String>) {
    (
        origin.to_string(),
        layer.to_string(),
        x.get("id").filter(|v| *v != "None").cloned(),
    )
}

fn paper_entity(r: &Record, x: &BTreeMap<String, String>, height: f64) -> Entity {
    let (origin, layer, id) = common(&x["origin"], r.get(8).unwrap(), x);
    let flip = |q: [f64; 2]| [q[0], height - q[1]];
    let id = id.as_deref();
    match x["type"].as_str() {
        "line" => {
            let (lt, w) = style(r);
            Entity::line(
                &origin,
                &layer,
                flip([r.f(10), r.f(20)]),
                flip([r.f(11), r.f(21)]),
                &lt,
                w,
                id,
            )
        }
        "polyline" => {
            let (lt, w) = style(r);
            let p: Vec<[f64; 2]> = pts(r).into_iter().map(flip).collect();
            Entity::polyline(&origin, &layer, &p, r.i_or(70, 0) & 1 == 1, &lt, w, id)
        }
        "circle" => {
            let (lt, w) = style(r);
            Entity::circle(
                &origin,
                &layer,
                flip([r.f(10), r.f(20)]),
                r.f(40),
                &lt,
                w,
                id,
            )
        }
        "arc" => {
            let (lt, w) = style(r);
            Entity::arc(
                &origin,
                &layer,
                flip([r.f(10), r.f(20)]),
                r.f(40),
                r.f(50),
                (r.f(51) - r.f(50)).rem_euclid(360.0),
                &lt,
                w,
                id,
            )
        }
        "hatch" => {
            let l: Vec<Vec<[f64; 2]>> = loops(r)
                .into_iter()
                .map(|lp| lp.into_iter().map(flip).collect())
                .collect();
            let w = r.get(370).unwrap().trim().parse::<f64>().unwrap() / 100.0;
            Entity::hatch(
                &origin,
                &layer,
                &l[0],
                &l[1..],
                &x["pattern"],
                x["angle"].parse().unwrap(),
                w,
                id,
            )
        }
        _ => {
            let p = text_point(r);
            Entity::text(
                &origin,
                &layer,
                flip(p),
                r.get(1).unwrap(),
                r.f(40),
                r.f_or(50, 0.0),
                halign(r.i_or(72, 0)),
                valign(r.i_or(73, 0)),
                id,
            )
        }
    }
}

/// A block point to a model point.
trait Transform {
    fn apply(&self, q: [f64; 2]) -> [f64; 2];
}

struct Identity;

impl Transform for Identity {
    fn apply(&self, q: [f64; 2]) -> [f64; 2] {
        q
    }
}

struct Insert {
    base: [f64; 2],
    factor: f64,
    cos: f64,
    sin: f64,
    at: [f64; 2],
}

impl Transform for Insert {
    fn apply(&self, q: [f64; 2]) -> [f64; 2] {
        let (dx, dy) = (
            (q[0] - self.base[0]) * self.factor,
            (q[1] - self.base[1]) * self.factor,
        );
        [
            self.at[0] + dx * self.cos - dy * self.sin,
            self.at[1] + dx * self.sin + dy * self.cos,
        ]
    }
}

fn model_entity(
    r: &Record,
    x: &BTreeMap<String, String>,
    vp: &ViewportBack,
    xf: &dyn Transform,
    scale: f64,
    rot_off: f64,
    suffix: &str,
) -> Entity {
    let window = Window {
        xmin: vp.centre[0],
        xmax: vp.centre[0],
        ymin: vp.centre[1],
        ymax: vp.centre[1],
    };
    let mapper = Mapper::new(vp.frame, window, vp.n, vp.twist);
    let k = 1000.0 / f64::from(vp.n);
    let point = |q: [f64; 2]| mapper.point(xf.apply(q));
    let layer = r.get(8).unwrap().to_string();
    let origin = format!("viewport.{}{suffix}", x["vp"]);
    let id = x.get("id").filter(|v| *v != "None").map(String::as_str);
    let kind = x.get("type").map_or_else(
        || r.kind.to_lowercase().replace("lwpolyline", "polyline"),
        String::clone,
    );
    match kind.as_str() {
        "line" | "centre_line" => {
            let (lt, w) = style(r);
            Entity::line(
                &origin,
                &layer,
                point([r.f(10), r.f(20)]),
                point([r.f(11), r.f(21)]),
                &lt,
                w,
                id,
            )
        }
        "polyline" => {
            let (lt, w) = style(r);
            let p: Vec<[f64; 2]> = pts(r).into_iter().map(point).collect();
            Entity::polyline(&origin, &layer, &p, r.i_or(70, 0) & 1 == 1, &lt, w, id)
        }
        "circle" => {
            let (lt, w) = style(r);
            Entity::circle(
                &origin,
                &layer,
                point([r.f(10), r.f(20)]),
                r.f(40) * scale * k,
                &lt,
                w,
                id,
            )
        }
        "arc" => {
            let (lt, w) = style(r);
            Entity::arc(
                &origin,
                &layer,
                point([r.f(10), r.f(20)]),
                r.f(40) * scale * k,
                r.f(50) + rot_off + vp.twist,
                (r.f(51) - r.f(50)).rem_euclid(360.0),
                &lt,
                w,
                id,
            )
        }
        "hatch" => {
            let l: Vec<Vec<[f64; 2]>> = loops(r)
                .into_iter()
                .map(|lp| lp.into_iter().map(point).collect())
                .collect();
            let w = r.get(370).unwrap().trim().parse::<f64>().unwrap() / 100.0;
            let a = if suffix == ".symbol" {
                0.0
            } else {
                x["angle"].parse::<f64>().unwrap() + rot_off + vp.twist
            };
            Entity::hatch(&origin, &layer, &l[0], &l[1..], &x["pattern"], a, w, id)
        }
        _ => {
            let origin = if x.get("part").map(String::as_str) == Some("symbol") {
                format!("{origin}.symbol")
            } else {
                origin
            };
            Entity::text(
                &origin,
                &layer,
                point(text_point(r)),
                r.get(1).unwrap(),
                mm(r.f(40) * 1000.0 / f64::from(vp.n)),
                angle(r.f_or(50, 0.0) + vp.twist),
                halign(r.i_or(72, 0)),
                valign(r.i_or(73, 0)),
                id,
            )
        }
    }
}

fn expand_insert(
    r: &Record,
    x: &BTreeMap<String, String>,
    vp: &ViewportBack,
    recs: &[Record],
) -> Vec<Entity> {
    let name = r.get(2).unwrap();
    let base = recs
        .iter()
        .find(|b| b.kind == "BLOCK" && b.get(2) == Some(name))
        .expect("the block of the insert");
    let (c, s) = cos_sin(r.f_or(50, 0.0));
    let xf = Insert {
        base: [base.f_or(10, 0.0), base.f_or(20, 0.0)],
        factor: r.f_or(41, 1.0),
        cos: c,
        sin: s,
        at: [r.f(10), r.f(20)],
    };
    recs.iter()
        .filter(|e| {
            e.block.as_deref() == Some(name)
                && e.section == "BLOCKS"
                && e.kind != "BLOCK"
                && e.kind != "ENDBLK"
        })
        .map(|e| {
            let mut ex = e.xdata();
            ex.insert("vp".into(), x["vp"].clone());
            model_entity(e, &ex, vp, &xf, xf.factor, r.f_or(50, 0.0), ".symbol")
        })
        .collect()
}

pub struct Tables {
    pub layers: BTreeMap<String, (i64, String, i64, i64)>,
    pub ltypes: BTreeMap<String, Vec<f64>>,
    pub styles: BTreeMap<String, String>,
    pub header: BTreeMap<String, i64>,
}

/// Layers (colour, line type, line weight, plot), line types, text styles and the header variables that
/// the standard fixes.
pub fn style_tables(text: &str) -> Tables {
    let recs = records(text);
    let layers = recs
        .iter()
        .filter(|r| r.kind == "LAYER")
        .map(|r| {
            (
                r.get(2).unwrap().to_string(),
                (
                    r.i_or(62, 0),
                    r.get(6).unwrap().to_string(),
                    r.i_or(370, 0),
                    r.i_or(290, 1),
                ),
            )
        })
        .collect();
    let ltypes = recs
        .iter()
        .filter(|r| r.kind == "LTYPE")
        .map(|r| {
            (
                r.get(2).unwrap().to_string(),
                r.all(49)
                    .iter()
                    .map(|v| v.trim().parse().unwrap())
                    .collect(),
            )
        })
        .collect();
    let styles = recs
        .iter()
        .filter(|r| r.kind == "STYLE")
        .map(|r| (r.get(2).unwrap().to_string(), r.get(3).unwrap().to_string()))
        .collect();
    let items = tags(text);
    let mut header = BTreeMap::new();
    for (i, (c, v)) in items.iter().enumerate() {
        if *c == 9 && ["$INSUNITS", "$PSLTSCALE", "$MEASUREMENT"].contains(&v.as_str()) {
            header.insert(v.clone(), items[i + 1].1.trim().parse().unwrap());
        }
    }
    Tables {
        layers,
        ltypes,
        styles,
        header,
    }
}
