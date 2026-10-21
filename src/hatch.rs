//! Hatches: validity (clause 10.3) and the marks of a pattern (clause 10.5).

use crate::data;
use crate::entities::{Entity, Shape};
use crate::numbers::cos_sin;
use crate::rules::decimal_product;
use serde_json::Value;

type P = [f64; 2];

/// The hatch unit `u = 10 x` the thin width, worked in decimal so that 0.18 gives 1.8 (clause 10.5.1).
pub fn unit_mm(thin_width_mm: f64) -> f64 {
    let factor = data::hatch_patterns()["unit_factor"]
        .as_f64()
        .unwrap_or(10.0);
    decimal_product(thin_width_mm, factor)
}

// ---- validity (clause 10.3) --------------------------------------------------------------------

fn area(loop_: &[P]) -> f64 {
    let n = loop_.len();
    (0..n)
        .map(|i| loop_[i][0] * loop_[(i + 1) % n][1] - loop_[(i + 1) % n][0] * loop_[i][1])
        .sum::<f64>()
        / 2.0
}

fn orient(a: P, b: P, c: P) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn on_segment(a: P, b: P, q: P) -> bool {
    a[0].min(b[0]) <= q[0]
        && q[0] <= a[0].max(b[0])
        && a[1].min(b[1]) <= q[1]
        && q[1] <= a[1].max(b[1])
}

/// True when the closed segments ab and cd share any point.
fn segments_touch(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2, o3, o4) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    if (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0) && ![o1, o2, o3, o4].contains(&0.0) {
        return true;
    }
    (o1 == 0.0 && on_segment(a, b, c))
        || (o2 == 0.0 && on_segment(a, b, d))
        || (o3 == 0.0 && on_segment(c, d, a))
        || (o4 == 0.0 && on_segment(c, d, b))
}

fn edges(loop_: &[P]) -> Vec<(P, P)> {
    (0..loop_.len())
        .map(|i| (loop_[i], loop_[(i + 1) % loop_.len()]))
        .collect()
}

/// No repeated point in a row, a non-zero area, and no crossing or touching of its own edges.
fn loop_ok(loop_: &[P]) -> bool {
    let n = loop_.len();
    if n < 3 || (0..n).any(|i| loop_[i] == loop_[(i + 1) % n]) {
        return false;
    }
    if area(loop_) == 0.0 {
        return false;
    }
    let es = edges(loop_);
    for i in 0..n {
        for j in (i + 1)..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                // adjacent edges share exactly their common vertex: reject only an overlap
                let (a, b) = es[i];
                let (c, d) = es[j];
                let (shared, other_i, other_j) = if j == i + 1 { (b, a, d) } else { (a, b, c) };
                if orient(other_i, shared, other_j) == 0.0
                    && (other_i[0] - shared[0]) * (other_j[0] - shared[0])
                        + (other_i[1] - shared[1]) * (other_j[1] - shared[1])
                        > 0.0
                {
                    return false;
                }
                continue;
            }
            if segments_touch(es[i].0, es[i].1, es[j].0, es[j].1) {
                return false;
            }
        }
    }
    true
}

fn inside(q: P, loop_: &[P]) -> bool {
    let (x, y) = (q[0], q[1]);
    let mut result = false;
    for (a, b) in edges(loop_) {
        if (a[1] > y) != (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0] {
            result = !result;
        }
    }
    result
}

fn loops_cross(one: &[P], two: &[P]) -> bool {
    let (e1, e2) = (edges(one), edges(two));
    e1.iter()
        .any(|(a, b)| e2.iter().any(|(c, d)| segments_touch(*a, *b, *c, *d)))
}

/// Clause 10.3: the boundary and every hole are simple loops with an area; holes lie inside the
/// boundary and do not touch it or each other.
pub fn hatch_ok(boundary: &[P], holes: &[Vec<P>]) -> bool {
    if !loop_ok(boundary) {
        return false;
    }
    for hole in holes {
        if !loop_ok(hole)
            || loops_cross(boundary, hole)
            || !hole.iter().all(|q| inside(*q, boundary))
        {
            return false;
        }
    }
    for i in 0..holes.len() {
        for j in (i + 1)..holes.len() {
            if loops_cross(&holes[i], &holes[j])
                || inside(holes[i][0], &holes[j])
                || inside(holes[j][0], &holes[i])
            {
                return false;
            }
        }
    }
    true
}

// ---- marks (clause 10.5) -----------------------------------------------------------------------

fn crosses(sa: f64, sb: f64, c: f64) -> bool {
    (sa <= c && c < sb) || (sb <= c && c < sa)
}

fn segment_distance(q: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length2 = dx * dx + dy * dy;
    if length2 == 0.0 {
        return libm::hypot(q[0] - a[0], q[1] - a[1]);
    }
    let u = (((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / length2).clamp(0.0, 1.0);
    libm::hypot(q[0] - (a[0] + u * dx), q[1] - (a[1] + u * dy))
}

/// The mark entities of one normalised hatch entity (empty for `solid`). `origin` is the origin of
/// the marks (clause 10.5.8) and `thin_width_mm` the thin width of the line group.
pub fn marks(hatch: &Entity, origin: &str, thin_width_mm: f64) -> Vec<Entity> {
    let Shape::Hatch {
        boundary,
        holes,
        pattern,
        angle,
        width,
    } = &hatch.shape
    else {
        return vec![];
    };
    let table = &data::hatch_patterns()["patterns"][pattern.as_str()];
    let kind = table["kind"].as_str().unwrap_or("solid");
    if kind == "solid" {
        return vec![];
    }
    let u = unit_mm(thin_width_mm);
    let (c, s) = cos_sin(*angle);
    let (direction, normal) = ([c, -s], [s, c]);
    let ts = |q: P| {
        [
            direction[0] * q[0] + direction[1] * q[1],
            normal[0] * q[0] + normal[1] * q[1],
        ]
    };
    let point = |t: f64, across: f64| {
        [
            t * direction[0] + across * normal[0],
            t * direction[1] + across * normal[1],
        ]
    };
    let paper_loops: Vec<&[P]> = std::iter::once(boundary.as_slice())
        .chain(holes.iter().map(Vec::as_slice))
        .collect();
    let loops: Vec<Vec<P>> = paper_loops
        .iter()
        .map(|l| l.iter().map(|q| ts(*q)).collect())
        .collect();
    let all_t: Vec<f64> = loops.iter().flatten().map(|q| q[0]).collect();
    let all_s: Vec<f64> = loops.iter().flatten().map(|q| q[1]).collect();
    let (min_t, max_t) = (
        all_t.iter().copied().fold(f64::INFINITY, f64::min),
        all_t.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    let (min_s, max_s) = (
        all_s.iter().copied().fold(f64::INFINITY, f64::min),
        all_s.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    let layer = hatch.layer.as_str();
    let mut out = Vec::new();

    if kind == "lines" {
        let step = table["step_u"].as_f64().unwrap_or(1.0) * u;
        let line_type = table["line_type"].as_str().unwrap_or("continuous");
        let (k0, k1) = ((min_s / step).ceil() as i64, (max_s / step).floor() as i64);
        for k in k0..=k1 {
            let c0 = k as f64 * step;
            let mut crossings = Vec::new();
            for loop_ in &loops {
                for i in 0..loop_.len() {
                    let (ta, sa) = (loop_[i][0], loop_[i][1]);
                    let (tb, sb) = (
                        loop_[(i + 1) % loop_.len()][0],
                        loop_[(i + 1) % loop_.len()][1],
                    );
                    if crosses(sa, sb, c0) {
                        crossings.push(ta + (c0 - sa) / (sb - sa) * (tb - ta));
                    }
                }
            }
            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for pair in crossings.as_chunks::<2>().0 {
                if pair[1] > pair[0] {
                    out.push(Entity::line(
                        origin,
                        layer,
                        point(pair[0], c0),
                        point(pair[1], c0),
                        line_type,
                        *width,
                        None,
                    ));
                }
            }
        }
        return out;
    }

    let col = table["column_u"].as_f64().unwrap_or(1.0) * u;
    let row = table["row_u"].as_f64().unwrap_or(1.0) * u;
    let es: Vec<(P, P)> = loops.iter().flat_map(|l| edges(l)).collect();
    let paper_edges: Vec<(P, P)> = paper_loops.iter().flat_map(|l| edges(l)).collect();
    let (r0, r1) = ((min_s / row).ceil() as i64, (max_s / row).floor() as i64);
    for r in r0..=r1 {
        let odd = r.rem_euclid(2) == 1;
        let offset = if odd { 0.5 } else { 0.0 };
        let mark = &table["marks"][if odd { "odd" } else { "even" }];
        let clearance = mark["clearance_u"].as_f64().unwrap_or(0.0) * u;
        let s0 = r as f64 * row;
        let (q0, q1) = (
            (min_t / col - offset).ceil() as i64,
            (max_t / col - offset).floor() as i64,
        );
        for q in q0..=q1 {
            let t0 = (q as f64 + offset) * col;
            let crossing = es
                .iter()
                .filter(|(a, b)| {
                    crosses(a[1], b[1], s0)
                        && a[0] + (s0 - a[1]) / (b[1] - a[1]) * (b[0] - a[0]) > t0
                })
                .count();
            if crossing % 2 == 0 {
                continue;
            }
            let centre = point(t0, s0);
            if paper_edges
                .iter()
                .any(|(a, b)| segment_distance(centre, *a, *b) < clearance)
            {
                continue;
            }
            if mark["shape"].as_str() == Some("dot") {
                out.push(Entity::circle(
                    origin,
                    layer,
                    centre,
                    mark["radius_u"].as_f64().unwrap_or(0.0) * u,
                    "continuous",
                    *width,
                    None,
                ));
            } else {
                let pts: Vec<P> = mark["points_u"]
                    .as_array()
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .map(|v| mark_point(v, centre, u, direction, normal))
                    .collect();
                out.push(Entity::polyline(
                    origin,
                    layer,
                    &pts,
                    true,
                    "continuous",
                    *width,
                    None,
                ));
            }
        }
    }
    out
}

fn mark_point(v: &Value, centre: P, u: f64, direction: P, normal: P) -> P {
    let (x, y) = (v[0].as_f64().unwrap_or(0.0), v[1].as_f64().unwrap_or(0.0));
    [
        centre[0] + x * u * direction[0] + y * u * normal[0],
        centre[1] + x * u * direction[1] + y * u * normal[1],
    ]
}
