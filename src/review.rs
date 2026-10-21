//! A layout review of normalised content: what a reader of the printed sheet would trip on.
//!
//! The standard checks that a document is well formed (clauses 4 to 18). A sheet can pass those checks
//! and still be hard to read: two labels on top of each other, a label on a hatch, text too small once
//! the sheet is printed reduced. The review reads only the normalised content (clause 16), so it runs on
//! any sheet, whatever made it. Each finding names its rule, its severity, the viewport (or the sheet)
//! and the entities, by their place in the list and by `id` where they have one.
//!
//! The rules and their severities:
//!
//! | Rule | Severity | Finding |
//! |---|---|---|
//! | `text-overlap` | error | The boxes of two texts (clause 11.7.2) overlap by more than [`ReviewOptions::overlap_mm`]. |
//! | `text-print-size` | error | A text is smaller than [`ReviewOptions::min_text_mm`] at the print scale. |
//! | `text-on-hatch` | warning | The box of a text lies partly on the area of a hatch. |
//! | `outside-frame` | error | A point of an entity of a viewport lies outside the frame of that viewport. |
//!
//! A text on a hatch is a warning, not an error: a label inside the area it names is common, and a
//! halo (clause 8.6.6) can keep it legible. The caller decides whether a warning fails its own build.
//! The standard's own furniture is left out of that rule: labels on the white plates of clause 8.6.7,
//! and the scale bar and north arrow against their own labels.

use crate::Normalised;
use crate::entities::{Entity, Shape};
use crate::numbers::{FIT_TOL_MM, cos_sin};
use crate::rules::{self, Rect};

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

impl Severity {
    pub fn word(self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// One finding of the review.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub severity: Severity,
    /// The rule, as named in the module table.
    pub rule: &'static str,
    /// The viewport of the entities, or `None` for the sheet itself (furniture, title block).
    pub viewport: Option<String>,
    /// The places of the entities in `Normalised::entities`.
    pub entities: Vec<usize>,
    /// The `id`s of those entities that have one.
    pub ids: Vec<String>,
    /// One line in English: what is wrong, with the texts or the numbers that show it.
    pub message: String,
}

/// The settings of a review.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewOptions {
    /// The scale the sheet is printed at: 1 for its own size, 0.5 for an A1 printed on A3.
    pub print_scale: f64,
    /// The smallest printed cap height, in millimetres. ISO 3098 lettering starts at 1.8 mm.
    pub min_text_mm: f64,
    /// How far two text boxes may run into each other before it counts, in paper millimetres.
    pub overlap_mm: f64,
}

impl Default for ReviewOptions {
    fn default() -> Self {
        ReviewOptions {
            print_scale: 1.0,
            min_text_mm: 1.8,
            overlap_mm: 0.1,
        }
    }
}

/// The viewport an origin belongs to: `viewport.<id>` or `viewport.<id>.<part>`.
fn viewport_of<'a>(origin: &'a str, norm: &'a Normalised) -> Option<&'a str> {
    let rest = origin.strip_prefix("viewport.")?;
    // an id may hold dots, so match the longest id of the sheet that the origin starts with
    norm.viewports
        .iter()
        .map(|v| v.id.as_str())
        .filter(|id| rest == *id || rest.strip_prefix(id).is_some_and(|t| t.starts_with('.')))
        .max_by_key(|id| id.len())
}

fn text_quad(e: &Entity) -> Option<[[f64; 2]; 4]> {
    match &e.shape {
        Shape::Text {
            anchor,
            text,
            height,
            rotation,
            align_h,
            align_v,
        } => rules::text_box(*anchor, text, *height, *rotation, *align_h, *align_v),
        _ => None,
    }
}

fn text_of(e: &Entity) -> &str {
    match &e.shape {
        Shape::Text { text, .. } => text,
        _ => "",
    }
}

/// How far two convex quads run into each other: the least overlap of their projections over the
/// axes of both (the separating axis test). Zero or less is apart.
fn overlap_depth(a: &[[f64; 2]; 4], b: &[[f64; 2]; 4]) -> f64 {
    let mut depth = f64::INFINITY;
    for quad in [a, b] {
        for i in 0..4 {
            let (p, q) = (quad[i], quad[(i + 1) % 4]);
            let (ex, ey) = (q[0] - p[0], q[1] - p[1]);
            let len = libm::sqrt(ex * ex + ey * ey);
            if len == 0.0 {
                continue;
            }
            let axis = [-ey / len, ex / len];
            let span = |pts: &[[f64; 2]; 4]| {
                pts.iter()
                    .map(|v| v[0] * axis[0] + v[1] * axis[1])
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), d| {
                        (lo.min(d), hi.max(d))
                    })
            };
            let ((a0, a1), (b0, b1)) = (span(a), span(b));
            depth = depth.min(a1.min(b1) - a0.max(b0));
        }
    }
    depth
}

/// Even-odd: is a point inside the loops (a boundary and its holes)?
fn inside_loops(p: [f64; 2], loops: &[&[[f64; 2]]]) -> bool {
    let mut inside = false;
    for ring in loops {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a[1] > p[1]) != (b[1] > p[1]) {
                let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
                if p[0] < x {
                    inside = !inside;
                }
            }
        }
    }
    inside
}

/// Points of a text box to test against a hatch: the corners pulled in by a little, the edge
/// middles and the centre, so a box that only touches the hatch edge does not count.
fn probe_points(q: &[[f64; 2]; 4], inset: f64) -> Vec<[f64; 2]> {
    let c = [
        (q[0][0] + q[1][0] + q[2][0] + q[3][0]) / 4.0,
        (q[0][1] + q[1][1] + q[2][1] + q[3][1]) / 4.0,
    ];
    let pull = |p: [f64; 2]| {
        let (dx, dy) = (c[0] - p[0], c[1] - p[1]);
        let d = libm::sqrt(dx * dx + dy * dy);
        if d <= inset {
            c
        } else {
            [p[0] + dx * inset / d, p[1] + dy * inset / d]
        }
    };
    let mut out: Vec<[f64; 2]> = q.iter().map(|p| pull(*p)).collect();
    for i in 0..4 {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        out.push(pull([(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]));
    }
    out.push(c);
    out
}

/// The points of an entity that must lie inside its frame.
fn frame_points(e: &Entity) -> Vec<[f64; 2]> {
    match &e.shape {
        Shape::Line { from, to, .. } => vec![*from, *to],
        Shape::Polyline { points, .. } => points.clone(),
        Shape::Hatch { boundary, .. } => boundary.clone(),
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
                    .filter(|a| (a - start).rem_euclid(360.0) <= *sweep),
            );
            angles
                .into_iter()
                .map(|a| {
                    let (c, s) = cos_sin(a);
                    [centre[0] + radius * c, centre[1] - radius * s]
                })
                .collect()
        }
        Shape::Text { .. } => text_quad(e).map_or(vec![], |q| q.to_vec()),
        Shape::Image { box_mm, .. } => vec![[box_mm[0], box_mm[1]], [box_mm[2], box_mm[3]]],
    }
}

fn inside(p: [f64; 2], r: Rect, tol: f64) -> bool {
    p[0] >= r.x - tol && p[0] <= r.x + r.w + tol && p[1] >= r.y - tol && p[1] <= r.y + r.h + tol
}

fn ids_of(norm: &Normalised, idx: &[usize]) -> Vec<String> {
    idx.iter()
        .filter_map(|&i| norm.entities[i].id.clone())
        .collect()
}

/// The findings of a sheet, errors first, then in the order of the entities.
pub fn review(norm: &Normalised, options: &ReviewOptions) -> Vec<Finding> {
    let mut out = Vec::new();
    let vp_of = |e: &Entity| viewport_of(&e.origin, norm).map(str::to_string);
    let texts: Vec<(usize, [[f64; 2]; 4])> = norm
        .entities
        .iter()
        .enumerate()
        .filter_map(|(i, e)| text_quad(e).map(|q| (i, q)))
        .collect();

    // text-print-size
    for &(i, _) in &texts {
        if let Shape::Text { height, .. } = &norm.entities[i].shape {
            let printed = height * options.print_scale;
            if printed < options.min_text_mm - FIT_TOL_MM {
                out.push(Finding {
                    severity: Severity::Error,
                    rule: "text-print-size",
                    viewport: vp_of(&norm.entities[i]),
                    entities: vec![i],
                    ids: ids_of(norm, &[i]),
                    message: format!(
                        "text {:?} is {printed} mm high at a print scale of {}, below {} mm",
                        text_of(&norm.entities[i]),
                        options.print_scale,
                        options.min_text_mm
                    ),
                });
            }
        }
    }

    // text-overlap
    for (k, (i, a)) in texts.iter().enumerate() {
        for (j, b) in &texts[k + 1..] {
            let depth = overlap_depth(a, b);
            if depth > options.overlap_mm {
                let (ei, ej) = (&norm.entities[*i], &norm.entities[*j]);
                out.push(Finding {
                    severity: Severity::Error,
                    rule: "text-overlap",
                    viewport: vp_of(ei).or_else(|| vp_of(ej)),
                    entities: vec![*i, *j],
                    ids: ids_of(norm, &[*i, *j]),
                    message: format!(
                        "texts {:?} and {:?} overlap by {:.2} mm",
                        text_of(ei),
                        text_of(ej),
                        depth
                    ),
                });
            }
        }
    }

    // text-on-hatch
    for (h, he) in norm.entities.iter().enumerate() {
        let Shape::Hatch {
            boundary, holes, ..
        } = &he.shape
        else {
            continue;
        };
        if boundary.len() < 3 {
            continue;
        }
        let loops: Vec<&[[f64; 2]]> = std::iter::once(boundary.as_slice())
            .chain(holes.iter().map(Vec::as_slice))
            .collect();
        for &(i, q) in &texts {
            let te = &norm.entities[i];
            // A plate (clause 8.6.7) is paper under the furniture, and the scale bar and north arrow
            // place their own labels by the standard's geometry (clauses 7.9 and 7.10).
            if he.origin.ends_with(".backing")
                || (te.origin == he.origin
                    && (he.origin.ends_with(".scale_bar") || he.origin.ends_with(".north_arrow")))
            {
                continue;
            }
            if probe_points(&q, options.overlap_mm)
                .into_iter()
                .any(|p| inside_loops(p, &loops))
            {
                out.push(Finding {
                    severity: Severity::Warning,
                    rule: "text-on-hatch",
                    viewport: vp_of(te),
                    entities: vec![i, h],
                    ids: ids_of(norm, &[i, h]),
                    message: format!(
                        "text {:?} lies on a {} hatch",
                        text_of(te),
                        match &he.shape {
                            Shape::Hatch { pattern, .. } => pattern.as_str(),
                            _ => "",
                        }
                    ),
                });
            }
        }
    }

    // outside-frame
    for (i, e) in norm.entities.iter().enumerate() {
        let Some(vid) = viewport_of(&e.origin, norm) else {
            continue;
        };
        let Some(vp) = norm.viewports.iter().find(|v| v.id == vid) else {
            continue;
        };
        if let Some(p) = frame_points(e)
            .into_iter()
            .find(|p| !inside(*p, vp.frame, 0.01))
        {
            out.push(Finding {
                severity: Severity::Error,
                rule: "outside-frame",
                viewport: Some(vid.to_string()),
                entities: vec![i],
                ids: ids_of(norm, &[i]),
                message: format!(
                    "a {} of viewport {vid} reaches ({:.2}, {:.2}) mm, outside its frame",
                    e.type_name(),
                    p[0],
                    p[1]
                ),
            });
        }
    }

    out.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.entities.cmp(&b.entities))
    });
    out
}
