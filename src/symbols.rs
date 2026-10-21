//! Symbols and libraries (clause 13).

use crate::data::{self, LineGroup, Role};
use crate::entities::Entity;
use crate::hatch;
use crate::numbers::cos_sin;
use crate::pipeline::Files;
use crate::read::{arr, f, p, points, st};
use crate::rules::{self, AlignH, AlignV, Mapper};
use crate::schema::Schema;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type P = [f64; 2];

/// The symbol libraries of a document, by library id. Each holds its `symbols` object.
#[derive(Debug, Default)]
pub struct Libraries {
    map: BTreeMap<String, Value>,
}

impl Libraries {
    /// A symbol of a library, if both exist.
    pub fn symbol(&self, library: &str, symbol: &str) -> Option<&Value> {
        self.map.get(library)?.get(symbol)
    }
}

/// Clause 13.2.2: the rules of clauses 9.3 to 9.6 and 10.3 for the parts, and unique port ids.
fn symbol_ok(symbol: &Value) -> bool {
    for part in arr(&symbol["geometry"]) {
        match st(&part["type"]) {
            "line" if p(&part["from"]) == p(&part["to"]) => return false,
            "polyline" => {
                let pts = points(&part["points"]);
                if (0..pts.len().saturating_sub(1)).any(|i| pts[i] == pts[i + 1])
                    || (part.get("closed").and_then(Value::as_bool) == Some(true)
                        && pts.first() == pts.last())
                {
                    return false;
                }
            }
            "arc" if rules::arc_sweep(f(&part["start_deg"]), f(&part["end_deg"])).is_none() => {
                return false;
            }
            "hatch" if !hatch::hatch_ok(&points(&part["boundary"]), &[]) => return false,
            _ => {}
        }
    }
    let ids: Vec<&str> = arr(&symbol["ports"])
        .iter()
        .map(|port| st(&port["id"]))
        .collect();
    ids.iter().collect::<BTreeSet<_>>().len() == ids.len()
}

/// Read the libraries of a document (clauses 13.1 and 13.3.1). Errors: 13.1 and 13.3.1.
pub fn load_libraries(doc: &Value, files: &dyn Files) -> (Libraries, BTreeSet<String>) {
    let mut libs = Libraries::default();
    let mut errors = BTreeSet::new();
    let schema = Schema::new(data::library_schema());
    let mut seen = BTreeSet::new();
    for item in arr(&doc["symbol_libraries"]) {
        let id = st(&item["id"]).to_string();
        if !seen.insert(id.clone()) {
            errors.insert("13.3.1".to_string());
            continue;
        }
        let symbols = if let Some(path) = item.get("path") {
            let loaded = files
                .read(st(path))
                .and_then(|text| crate::json::parse(&text).ok());
            let Some(file) = loaded else {
                errors.insert("13.1".to_string());
                continue;
            };
            let version_ok = file
                .get("standard_version")
                .is_none_or(|v| st(v).split('.').next() == Some("1"));
            if !schema.is_valid(&file) || !version_ok {
                errors.insert("13.1".to_string());
                continue;
            }
            file["symbols"].clone()
        } else {
            item["symbols"].clone()
        };
        if !symbols
            .as_object()
            .is_some_and(|m| m.values().all(symbol_ok))
        {
            errors.insert("13.1".to_string());
            continue;
        }
        libs.map.insert(id, symbols);
    }
    (libs, errors)
}

/// What `parts` needs to know about the viewport and the layer of the insert.
pub struct SymbolContext<'a> {
    pub mapper: &'a Mapper,
    /// The rotation of the viewport.
    pub rotation: f64,
    pub denominator: u32,
    pub group: &'a LineGroup,
    pub layer_role: Role,
    pub layer_line_type: &'a str,
    /// `viewport.<id>`
    pub origin: String,
}

/// The transform of one insert (clause 13.4): symbol coordinates to model coordinates.
pub struct Placement {
    pub factor: f64,
    origin: P,
    pub rotation: f64,
    cos: f64,
    sin: f64,
    position: P,
}

impl Placement {
    /// Clause 13.4. `units_paper` is true for a symbol of `paper_mm`.
    pub fn new(
        position: P,
        rotation_deg: f64,
        scale: f64,
        insertion: P,
        units_paper: bool,
        denominator: u32,
    ) -> Placement {
        let factor = scale
            * if units_paper {
                f64::from(denominator) / 1000.0
            } else {
                1.0
            };
        let (cos, sin) = cos_sin(rotation_deg);
        Placement {
            factor,
            origin: insertion,
            rotation: rotation_deg,
            cos,
            sin,
            position,
        }
    }

    /// The placement of an insert entity of a symbol.
    pub fn of(insert: &Value, symbol: &Value, denominator: u32) -> Placement {
        let insertion = symbol.get("insertion").map_or([0.0, 0.0], p);
        Placement::new(
            p(&insert["position_m"]),
            insert.get("rotation_deg").map_or(0.0, f),
            insert.get("scale").map_or(1.0, f),
            insertion,
            st(&symbol["units"]) == "paper_mm",
            denominator,
        )
    }

    /// The model point of a symbol point.
    pub fn model(&self, q: P) -> P {
        let dx = (q[0] - self.origin[0]) * self.factor;
        let dy = (q[1] - self.origin[1]) * self.factor;
        [
            self.position[0] + dx * self.cos - dy * self.sin,
            self.position[1] + dx * self.sin + dy * self.cos,
        ]
    }
}

/// The model point of a port of an insert (clause 13.6), or `None`.
pub fn port_point(insert: &Value, symbol: &Value, port_id: &str, denominator: u32) -> Option<P> {
    let port = arr(&symbol["ports"])
        .iter()
        .find(|port| st(&port["id"]) == port_id)?;
    Some(Placement::of(insert, symbol, denominator).model(p(&port["at"])))
}

/// The normalised entities of one insert (clause 13.7) and its errors (clause 13.5.1).
pub fn parts(
    insert: &Value,
    symbol: &Value,
    ctx: &SymbolContext,
) -> (Vec<Entity>, BTreeSet<String>) {
    let mut errors = BTreeSet::new();
    let mut out = Vec::new();
    let pl = Placement::of(insert, symbol, ctx.denominator);
    let k = ctx.mapper.k;
    let origin = format!("{}.symbol", ctx.origin);
    let layer = st(&insert["layer"]);
    let pm = |q: P| ctx.mapper.point(pl.model(q));
    for part in arr(&symbol["geometry"]) {
        let role = part
            .get("role")
            .or_else(|| insert.get("role"))
            .and_then(|r| Role::parse(st(r)))
            .unwrap_or(ctx.layer_role);
        let width = ctx.group.width(role);
        let line_type = part.get("line_type").map_or(ctx.layer_line_type, st);
        match st(&part["type"]) {
            "line" => out.push(Entity::line(
                &origin,
                layer,
                pm(p(&part["from"])),
                pm(p(&part["to"])),
                line_type,
                width,
                None,
            )),
            "polyline" => {
                let pts: Vec<P> = points(&part["points"]).into_iter().map(pm).collect();
                out.push(Entity::polyline(
                    &origin,
                    layer,
                    &pts,
                    part.get("closed").and_then(Value::as_bool).unwrap_or(false),
                    line_type,
                    width,
                    None,
                ));
            }
            "hatch" => {
                let pts: Vec<P> = points(&part["boundary"]).into_iter().map(pm).collect();
                out.push(Entity::hatch(
                    &origin,
                    layer,
                    &pts,
                    &[],
                    "solid",
                    0.0,
                    width,
                    None,
                ));
            }
            "circle" => out.push(Entity::circle(
                &origin,
                layer,
                pm(p(&part["centre"])),
                f(&part["radius"]) * pl.factor * k,
                line_type,
                width,
                None,
            )),
            "arc" => {
                let sweep =
                    rules::arc_sweep(f(&part["start_deg"]), f(&part["end_deg"])).unwrap_or(0.0);
                out.push(Entity::arc(
                    &origin,
                    layer,
                    pm(p(&part["centre"])),
                    f(&part["radius"]) * pl.factor * k,
                    f(&part["start_deg"]) + pl.rotation + ctx.rotation,
                    sweep,
                    line_type,
                    width,
                    None,
                ));
            }
            _ => {}
        }
    }
    if let Some(label) = insert.get("label") {
        match symbol.get("label_slot") {
            None => {
                errors.insert("13.5.1".to_string());
            }
            Some(slot) => out.push(Entity::text(
                &origin,
                layer,
                pm(p(&slot["at"])),
                st(label),
                slot.get("height_mm").map_or(3.5, f),
                0.0,
                AlignH::parse(slot.get("align_h").map_or("centre", st)),
                AlignV::parse(slot.get("align_v").map_or("middle", st)),
                None,
            )),
        }
    }
    (out, errors)
}
