//! Dimensions and leaders (clause 12): computed from the geometry that they point at.

use crate::entities::{Entity, Shape};
use crate::numbers::{cos_sin, format_fixed};
use crate::read::{f, p, st};
use crate::rules::{self, AlignH, AlignV, Mapper};
use crate::symbols::{self, Libraries};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

type P = [f64; 2];

/// What a dimension or leader needs to know about its viewport.
pub struct Context<'a> {
    pub mapper: &'a Mapper,
    pub rotation: f64,
    /// The hatch unit `u` of the line group (clause 10.5.1).
    pub unit_mm: f64,
    /// The width of the role of the entity (clause 8.7).
    pub width: f64,
    /// `viewport.<id>`
    pub origin: String,
    /// The entities of the viewport by id.
    pub index: &'a HashMap<String, &'a Value>,
    pub libs: &'a Libraries,
    pub denominator: u32,
}

/// The text and unit of a dimension that belongs to a chain (clause 12.8).
#[derive(Debug, Clone, PartialEq)]
pub struct ChainInfo {
    pub id: String,
    pub role: String,
    pub text: String,
    pub unit: String,
    pub decimals: u32,
}

/// The parts of a dimension or leader, or the clauses it breaks.
#[derive(Debug, Default)]
pub struct DimResult {
    pub entities: Vec<Entity>,
    pub errors: BTreeSet<String>,
    pub chain: Option<ChainInfo>,
}

fn left(v: P) -> P {
    [v[1], -v[0]]
}

fn add(a: P, b: P, k: f64) -> P {
    [a[0] + k * b[0], a[1] + k * b[1]]
}

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn unit(v: P) -> P {
    let n = libm::hypot(v[0], v[1]);
    [v[0] / n, v[1] / n]
}

fn neg(v: P) -> P {
    [-v[0], -v[1]]
}

fn visual_angle(v: P) -> f64 {
    libm::atan2(-v[1], v[0]).to_degrees().rem_euclid(360.0)
}

/// The rotation of dimension text so that it never reads upside down (clause 12.3.3).
pub fn reading_rotation(v: P) -> f64 {
    let theta = visual_angle(v);
    if theta <= 90.0 || theta > 270.0 {
        theta
    } else {
        (theta - 180.0).rem_euclid(360.0)
    }
}

/// The text of a length (clause 12.1.1): rounded half-even to `decimals` places, written with that many places.
pub fn format_value(value: f64, decimals: u32) -> String {
    format_fixed(value, decimals)
}

/// The model point of a point spec, or `None` when the reference is wrong (clause 12.2).
pub fn model_point(spec: &Value, ctx: &Context) -> Option<P> {
    if let Some(pt) = spec.get("point_m") {
        return Some(p(pt));
    }
    let e = ctx.index.get(st(&spec["ref"]))?;
    let at = st(&spec["at"]);
    let t = st(&e["type"]);
    if t == "symbol" {
        let symbol = ctx.libs.symbol(st(&e["library"]), st(&e["symbol"]))?;
        return symbols::port_point(e, symbol, at.strip_prefix("port:")?, ctx.denominator);
    }
    if at.starts_with("port:") {
        return None;
    }
    match t {
        "line" | "centre_line" if at == "start" => Some(p(&e["from_m"])),
        "line" | "centre_line" if at == "end" => Some(p(&e["to_m"])),
        "polyline" if at.starts_with("vertex:") => {
            let i: usize = at["vertex:".len()..].parse().ok()?;
            e["points_m"].as_array()?.get(i).map(p)
        }
        "circle" if at == "centre" => Some(p(&e["centre_m"])),
        "arc" => {
            let (c, r) = (p(&e["centre_m"]), f(&e["radius_m"]));
            match at {
                "centre" => Some(c),
                "start" | "end" => {
                    let deg = if at == "start" {
                        f(&e["start_deg"])
                    } else {
                        f(&e["end_deg"])
                    };
                    let (cs, sn) = cos_sin(deg);
                    Some([c[0] + r * cs, c[1] + r * sn])
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn arrow(origin: &str, layer: &str, tip: P, e: P, ctx: &Context) -> Entity {
    let u = ctx.unit_mm;
    let a = 2.0 * u;
    let n = left(e);
    let base = add(tip, e, -a);
    Entity::hatch(
        origin,
        layer,
        &[tip, add(base, n, u / 4.0), add(base, n, -(u / 4.0))],
        &[],
        "solid",
        0.0,
        ctx.width,
        None,
    )
}

fn dim_text(
    origin: &str,
    layer: &str,
    text: &str,
    mid: P,
    v: P,
    height: f64,
    ctx: &Context,
) -> Entity {
    let rho = reading_rotation(v);
    let (c, s) = cos_sin(rho);
    let pos = add(mid, [-s, -c], ctx.unit_mm / 2.0);
    Entity::text(
        origin,
        layer,
        pos,
        text,
        height,
        rho,
        AlignH::Centre,
        AlignV::Baseline,
        None,
    )
}

fn fail(clause: &str) -> DimResult {
    DimResult {
        errors: BTreeSet::from([clause.to_string()]),
        ..DimResult::default()
    }
}

/// Clause 12.1 to 12.5: one dimension.
pub fn dimension(e: &Value, ctx: &Context) -> DimResult {
    let mut res = DimResult::default();
    let layer = st(&e["layer"]);
    let origin = format!("{}.dimension", ctx.origin);
    let u = ctx.unit_mm;
    let a = 2.0 * u;
    let kind = st(&e["kind"]);
    let unit_word = e.get("unit").map_or("m", st);
    let decimals = e.get("decimals").and_then(Value::as_u64).map_or_else(
        || {
            if kind == "angle" {
                1
            } else if unit_word == "m" {
                2
            } else {
                0
            }
        },
        |d| d as u32,
    );
    let height = e.get("text_height_mm").map_or(3.5, f);
    let factor = if unit_word == "mm" { 1000.0 } else { 1.0 };
    let mp = ctx.mapper;
    let width = ctx.width;
    let line = |a: P, b: P| Entity::line(&origin, layer, a, b, "continuous", width, None);
    let text;

    if kind == "linear" || kind == "aligned" {
        let (Some(m1), Some(m2)) = (model_point(&e["from"], ctx), model_point(&e["to"], ctx))
        else {
            return fail("12.2");
        };
        let horizontal = st(&e["orientation"]) == "horizontal";
        let value = if kind == "linear" {
            let axis = usize::from(!horizontal);
            (m2[axis] - m1[axis]).abs()
        } else {
            libm::hypot(m2[0] - m1[0], m2[1] - m1[1])
        };
        if value == 0.0 {
            return fail("12.4.1");
        }
        let (p1, p2) = (mp.point(m1), mp.point(m2));
        let mut d = if kind == "linear" {
            let (cr, sr) = cos_sin(ctx.rotation);
            if horizontal { [cr, -sr] } else { [-sr, -cr] }
        } else {
            unit([p2[0] - p1[0], p2[1] - p1[1]])
        };
        if dot([p2[0] - p1[0], p2[1] - p1[1]], d) < 0.0 {
            d = neg(d);
        }
        let l = left(d);
        let reference = add(
            [(p1[0] + p2[0]) / 2.0, (p1[1] + p2[1]) / 2.0],
            l,
            f(&e["offset_mm"]),
        );
        let s1 = dot([reference[0] - p1[0], reference[1] - p1[1]], l);
        let s2 = dot([reference[0] - p2[0], reference[1] - p2[1]], l);
        let (q1, q2) = (add(p1, l, s1), add(p2, l, s2));
        if s1.abs() <= u / 2.0
            || s2.abs() <= u / 2.0
            || libm::hypot(q2[0] - q1[0], q2[1] - q1[1]) < 2.0 * a
        {
            return fail("12.4.3");
        }
        for (pt, q, s) in [(p1, q1, s1), (p2, q2, s2)] {
            let v = if s > 0.0 { l } else { neg(l) };
            res.entities.push(line(add(pt, v, u / 2.0), add(q, v, u)));
        }
        res.entities.push(line(q1, q2));
        res.entities.push(arrow(&origin, layer, q1, neg(d), ctx));
        res.entities.push(arrow(&origin, layer, q2, d, ctx));
        text = format_value(value * factor, decimals);
        let mid = [(q1[0] + q2[0]) / 2.0, (q1[1] + q2[1]) / 2.0];
        res.entities
            .push(dim_text(&origin, layer, &text, mid, d, height, ctx));
    } else if kind == "radius" || kind == "diameter" {
        let Some(target) = ctx.index.get(st(&e["of"])) else {
            return fail("12.2");
        };
        let target_type = st(&target["type"]);
        if target_type != "circle" && target_type != "arc" {
            return fail("12.2");
        }
        let phi_model = e.get("direction_deg").map_or(45.0, f);
        if target_type == "arc" {
            let sweep = rules::arc_sweep(f(&target["start_deg"]), f(&target["end_deg"]));
            if sweep.is_none_or(|sw| (phi_model - f(&target["start_deg"])).rem_euclid(360.0) > sw) {
                return fail("12.5.1");
            }
        }
        let c = mp.point(p(&target["centre_m"]));
        let radius = f(&target["radius_m"]) * mp.k;
        let (cphi, sphi) = cos_sin(phi_model + ctx.rotation);
        let ev = [cphi, -sphi];
        let is_radius = kind == "radius";
        let value = f(&target["radius_m"]) * if is_radius { 1.0 } else { 2.0 };
        text = format!(
            "{}{}",
            if is_radius { "R" } else { "Ø" },
            format_value(value * factor, decimals)
        );
        if is_radius {
            if radius < 2.0 * a {
                return fail("12.4.3");
            }
            let tip = add(c, ev, radius);
            res.entities.push(line(c, tip));
            res.entities.push(arrow(&origin, layer, tip, ev, ctx));
            res.entities.push(dim_text(
                &origin,
                layer,
                &text,
                add(c, ev, radius / 2.0),
                ev,
                height,
                ctx,
            ));
        } else {
            if radius < a {
                return fail("12.4.3");
            }
            let (t1, t2) = (add(c, ev, -radius), add(c, ev, radius));
            res.entities.push(line(t1, t2));
            res.entities.push(arrow(&origin, layer, t1, neg(ev), ctx));
            res.entities.push(arrow(&origin, layer, t2, ev, ctx));
            res.entities
                .push(dim_text(&origin, layer, &text, c, ev, height, ctx));
        }
    } else {
        let (Some(mv), Some(ma), Some(mb)) = (
            model_point(&e["vertex"], ctx),
            model_point(&e["from"], ctx),
            model_point(&e["to"], ctx),
        ) else {
            return fail("12.2");
        };
        if ma == mv || mb == mv {
            return fail("12.5.4");
        }
        let ta = libm::atan2(ma[1] - mv[1], ma[0] - mv[0]).to_degrees();
        let tb = libm::atan2(mb[1] - mv[1], mb[0] - mv[0]).to_degrees();
        let value = (tb - ta).rem_euclid(360.0);
        if value == 0.0 {
            return fail("12.5.4");
        }
        let rho = f(&e["arc_radius_mm"]);
        if rho < 2.0 * a {
            return fail("12.4.3");
        }
        let v = mp.point(mv);
        let (aa, ab) = (ta + ctx.rotation, tb + ctx.rotation);
        let w = |alpha: f64| {
            let (c, s) = cos_sin(alpha);
            [c, -s]
        };
        let tangent = |alpha: f64| {
            let (c, s) = cos_sin(alpha);
            [-s, -c]
        };
        for alpha in [aa, ab] {
            res.entities
                .push(line(add(v, w(alpha), u / 2.0), add(v, w(alpha), rho + u)));
        }
        res.entities.push(Entity::arc(
            &origin,
            layer,
            v,
            rho,
            aa,
            value,
            "continuous",
            width,
            None,
        ));
        res.entities.push(arrow(
            &origin,
            layer,
            add(v, w(aa), rho),
            neg(tangent(aa)),
            ctx,
        ));
        res.entities
            .push(arrow(&origin, layer, add(v, w(ab), rho), tangent(ab), ctx));
        text = format!("{}°", format_value(value, decimals));
        res.entities.push(Entity::text(
            &origin,
            layer,
            add(v, w(aa + value / 2.0), rho + height),
            &text,
            height,
            0.0,
            AlignH::Centre,
            AlignV::Middle,
            None,
        ));
    }

    if let Some(chain) = e.get("chain") {
        res.chain = Some(ChainInfo {
            id: st(chain).to_string(),
            role: st(&e["chain_role"]).to_string(),
            text,
            unit: unit_word.to_string(),
            decimals,
        });
    }
    res
}

/// Clause 12.6: one leader.
pub fn leader(e: &Value, ctx: &Context) -> DimResult {
    let mut res = DimResult::default();
    let layer = st(&e["layer"]);
    let origin = format!("{}.leader", ctx.origin);
    let u = ctx.unit_mm;
    let Some(m) = model_point(&e["from"], ctx) else {
        return fail("12.2");
    };
    let t = ctx.mapper.point(m);
    let lp = ctx.mapper.point(p(&e["label_at_m"]));
    if t == lp {
        return fail("12.6.1");
    }
    res.entities.push(Entity::line(
        &origin,
        layer,
        lp,
        t,
        "continuous",
        ctx.width,
        None,
    ));
    res.entities.push(arrow(
        &origin,
        layer,
        t,
        unit([t[0] - lp[0], t[1] - lp[1]]),
        ctx,
    ));
    let right = lp[0] >= t[0];
    let anchor = [lp[0] + if right { u / 2.0 } else { -u / 2.0 }, lp[1]];
    res.entities.push(Entity::text(
        &origin,
        layer,
        anchor,
        st(&e["text"]),
        e.get("text_height_mm").map_or(3.5, f),
        0.0,
        if right { AlignH::Left } else { AlignH::Right },
        AlignV::Middle,
        None,
    ));
    res
}

/// An exact decimal number from its text: whole digits and a count of places.
fn decimal(text: &str) -> Option<(i128, u32)> {
    let (negative, body) = text.strip_prefix('-').map_or((false, text), |b| (true, b));
    let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
    let digits = format!("{whole}{frac}");
    let value: i128 = digits.parse().ok()?;
    Some((if negative { -value } else { value }, frac.len() as u32))
}

fn same_decimal(a: (i128, u32), b: (i128, u32)) -> bool {
    let places = a.1.max(b.1);
    let scale = |x: (i128, u32)| x.0.checked_mul(10i128.checked_pow(places - x.1)?);
    matches!((scale(a), scale(b)), (Some(x), Some(y)) if x == y)
}

/// Clause 12.8: every chain has one overall and two or more parts, one unit and one precision, and
/// the printed numbers of the parts add up to the printed overall.
pub fn check_chains(infos: &[ChainInfo]) -> bool {
    let mut chains: Vec<(&str, Vec<&ChainInfo>)> = Vec::new();
    for info in infos {
        match chains.iter_mut().find(|(id, _)| *id == info.id) {
            Some((_, items)) => items.push(info),
            None => chains.push((&info.id, vec![info])),
        }
    }
    chains.iter().all(|(_, items)| {
        let overall: Vec<_> = items.iter().filter(|i| i.role == "overall").collect();
        let parts: Vec<_> = items.iter().filter(|i| i.role == "part").collect();
        if overall.len() != 1 || parts.len() < 2 {
            return false;
        }
        let first = (&items[0].unit, items[0].decimals);
        if items.iter().any(|i| (&i.unit, i.decimals) != first) {
            return false;
        }
        let mut sum = (0i128, 0u32);
        for part in &parts {
            let Some(d) = decimal(&part.text) else {
                return false;
            };
            let places = sum.1.max(d.1);
            let (Some(a), Some(b)) = (
                sum.0.checked_mul(10i128.pow(places - sum.1)),
                d.0.checked_mul(10i128.pow(places - d.1)),
            ) else {
                return false;
            };
            sum = (a + b, places);
        }
        decimal(&overall[0].text).is_some_and(|o| same_decimal(sum, o))
    })
}

/// The points of a generated part that must lie inside the frame (clause 7.7.1).
pub fn part_points(entity: &Entity) -> Vec<P> {
    match &entity.shape {
        Shape::Line { from, to, .. } => vec![*from, *to],
        Shape::Hatch { boundary, .. } => boundary.clone(),
        Shape::Polyline { points, .. } => points.clone(),
        Shape::Text {
            anchor,
            text,
            height,
            rotation,
            align_h,
            align_v,
        } => rules::text_box(*anchor, text, *height, *rotation, *align_h, *align_v)
            .map_or(vec![], |c| c.to_vec()),
        Shape::Circle { centre, radius, .. } => vec![
            [centre[0] - radius, centre[1]],
            [centre[0] + radius, centre[1]],
            [centre[0], centre[1] - radius],
            [centre[0], centre[1] + radius],
        ],
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
            ..
        } => {
            let mut angles = vec![*start, start + sweep];
            angles.extend(
                [0.0, 90.0, 180.0, 270.0]
                    .into_iter()
                    .filter(|x| (x - start).rem_euclid(360.0) <= *sweep),
            );
            angles
                .into_iter()
                .map(|a| {
                    let (c, s) = cos_sin(a);
                    [centre[0] + radius * c, centre[1] - radius * s]
                })
                .collect()
        }
    }
}
