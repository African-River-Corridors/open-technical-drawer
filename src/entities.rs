//! The entities of the normalised content (clause 16.4) and their order (clause 16.8).

use crate::numbers::{angle, mm};
use crate::rules::{AlignH, AlignV};
use serde_json::{Map, Number, Value};
use std::cmp::Ordering;

/// One entity, in paper millimetres, already rounded (clause 16.7).
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub origin: String,
    pub layer: String,
    pub id: Option<String>,
    pub shape: Shape,
}

/// What an entity draws (clause 16.4).
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Line {
        from: [f64; 2],
        to: [f64; 2],
        line_type: String,
        width: f64,
    },
    Polyline {
        points: Vec<[f64; 2]>,
        closed: bool,
        line_type: String,
        width: f64,
    },
    Arc {
        centre: [f64; 2],
        radius: f64,
        start: f64,
        sweep: f64,
        line_type: String,
        width: f64,
    },
    Circle {
        centre: [f64; 2],
        radius: f64,
        line_type: String,
        width: f64,
    },
    Hatch {
        boundary: Vec<[f64; 2]>,
        holes: Vec<Vec<[f64; 2]>>,
        pattern: String,
        angle: f64,
        width: f64,
    },
    Text {
        anchor: [f64; 2],
        text: String,
        height: f64,
        rotation: f64,
        align_h: AlignH,
        align_v: AlignV,
    },
}

/// Round a paper point to 3 places.
pub fn pt(p: [f64; 2]) -> [f64; 2] {
    [mm(p[0]), mm(p[1])]
}

impl Entity {
    fn make(origin: &str, layer: &str, id: Option<&str>, shape: Shape) -> Entity {
        Entity {
            origin: origin.to_string(),
            layer: layer.to_string(),
            id: id.filter(|s| !s.is_empty()).map(str::to_string),
            shape,
        }
    }

    pub fn line(
        origin: &str,
        layer: &str,
        a: [f64; 2],
        b: [f64; 2],
        line_type: &str,
        width: f64,
        id: Option<&str>,
    ) -> Entity {
        Entity::make(
            origin,
            layer,
            id,
            Shape::Line {
                from: pt(a),
                to: pt(b),
                line_type: line_type.to_string(),
                width,
            },
        )
    }

    pub fn polyline(
        origin: &str,
        layer: &str,
        points: &[[f64; 2]],
        closed: bool,
        line_type: &str,
        width: f64,
        id: Option<&str>,
    ) -> Entity {
        Entity::make(
            origin,
            layer,
            id,
            Shape::Polyline {
                points: points.iter().map(|p| pt(*p)).collect(),
                closed,
                line_type: line_type.to_string(),
                width,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn arc(
        origin: &str,
        layer: &str,
        centre: [f64; 2],
        radius: f64,
        start_deg: f64,
        sweep_deg: f64,
        line_type: &str,
        width: f64,
        id: Option<&str>,
    ) -> Entity {
        Entity::make(
            origin,
            layer,
            id,
            Shape::Arc {
                centre: pt(centre),
                radius: mm(radius),
                start: angle(start_deg),
                sweep: mm(sweep_deg),
                line_type: line_type.to_string(),
                width,
            },
        )
    }

    pub fn circle(
        origin: &str,
        layer: &str,
        centre: [f64; 2],
        radius: f64,
        line_type: &str,
        width: f64,
        id: Option<&str>,
    ) -> Entity {
        Entity::make(
            origin,
            layer,
            id,
            Shape::Circle {
                centre: pt(centre),
                radius: mm(radius),
                line_type: line_type.to_string(),
                width,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn hatch(
        origin: &str,
        layer: &str,
        boundary: &[[f64; 2]],
        holes: &[Vec<[f64; 2]>],
        pattern: &str,
        angle_deg: f64,
        width: f64,
        id: Option<&str>,
    ) -> Entity {
        Entity::make(
            origin,
            layer,
            id,
            Shape::Hatch {
                boundary: boundary.iter().map(|p| pt(*p)).collect(),
                holes: holes
                    .iter()
                    .map(|h| h.iter().map(|p| pt(*p)).collect())
                    .collect(),
                pattern: pattern.to_string(),
                angle: angle(angle_deg),
                width,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn text(
        origin: &str,
        layer: &str,
        anchor: [f64; 2],
        text: &str,
        height: f64,
        rotation: f64,
        align_h: AlignH,
        align_v: AlignV,
        id: Option<&str>,
    ) -> Entity {
        Entity::make(
            origin,
            layer,
            id,
            Shape::Text {
                anchor: pt(anchor),
                text: text.to_string(),
                height,
                rotation: angle(rotation),
                align_h,
                align_v,
            },
        )
    }

    /// The type word of clause 16.3.
    pub fn type_name(&self) -> &'static str {
        match self.shape {
            Shape::Line { .. } => "line",
            Shape::Polyline { .. } => "polyline",
            Shape::Arc { .. } => "arc",
            Shape::Circle { .. } => "circle",
            Shape::Hatch { .. } => "hatch",
            Shape::Text { .. } => "text",
        }
    }

    /// The line type, or the empty text for a hatch or a text (clause 16.8, part 3).
    pub fn line_type(&self) -> &str {
        match &self.shape {
            Shape::Line { line_type, .. }
            | Shape::Polyline { line_type, .. }
            | Shape::Arc { line_type, .. }
            | Shape::Circle { line_type, .. } => line_type,
            _ => "",
        }
    }

    /// The width, or 0 for a text (clause 16.8, part 4).
    pub fn width(&self) -> f64 {
        match &self.shape {
            Shape::Line { width, .. }
            | Shape::Polyline { width, .. }
            | Shape::Arc { width, .. }
            | Shape::Circle { width, .. }
            | Shape::Hatch { width, .. } => *width,
            Shape::Text { .. } => 0.0,
        }
    }

    /// The JSON form of clause 16.4.
    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        m.insert("type".into(), self.type_name().into());
        m.insert("origin".into(), self.origin.clone().into());
        m.insert("layer".into(), self.layer.clone().into());
        if let Some(id) = &self.id {
            m.insert("id".into(), id.clone().into());
        }
        match &self.shape {
            Shape::Line {
                from,
                to,
                line_type,
                width,
            } => {
                m.insert("from_mm".into(), point_value(*from));
                m.insert("to_mm".into(), point_value(*to));
                m.insert("line_type".into(), line_type.clone().into());
                m.insert("width_mm".into(), num(*width));
            }
            Shape::Polyline {
                points,
                closed,
                line_type,
                width,
            } => {
                m.insert(
                    "points_mm".into(),
                    Value::Array(points.iter().map(|p| point_value(*p)).collect()),
                );
                m.insert("closed".into(), (*closed).into());
                m.insert("line_type".into(), line_type.clone().into());
                m.insert("width_mm".into(), num(*width));
            }
            Shape::Arc {
                centre,
                radius,
                start,
                sweep,
                line_type,
                width,
            } => {
                m.insert("centre_mm".into(), point_value(*centre));
                m.insert("radius_mm".into(), num(*radius));
                m.insert("start_deg".into(), num(*start));
                m.insert("sweep_deg".into(), num(*sweep));
                m.insert("line_type".into(), line_type.clone().into());
                m.insert("width_mm".into(), num(*width));
            }
            Shape::Circle {
                centre,
                radius,
                line_type,
                width,
            } => {
                m.insert("centre_mm".into(), point_value(*centre));
                m.insert("radius_mm".into(), num(*radius));
                m.insert("line_type".into(), line_type.clone().into());
                m.insert("width_mm".into(), num(*width));
            }
            Shape::Hatch {
                boundary,
                holes,
                pattern,
                angle,
                width,
            } => {
                m.insert(
                    "boundary_mm".into(),
                    Value::Array(boundary.iter().map(|p| point_value(*p)).collect()),
                );
                m.insert(
                    "holes_mm".into(),
                    Value::Array(
                        holes
                            .iter()
                            .map(|h| Value::Array(h.iter().map(|p| point_value(*p)).collect()))
                            .collect(),
                    ),
                );
                m.insert("pattern".into(), pattern.clone().into());
                m.insert("angle_deg".into(), num(*angle));
                m.insert("width_mm".into(), num(*width));
            }
            Shape::Text {
                anchor,
                text,
                height,
                rotation,
                align_h,
                align_v,
            } => {
                m.insert("anchor_mm".into(), point_value(*anchor));
                m.insert("text".into(), text.clone().into());
                m.insert("height_mm".into(), num(*height));
                m.insert("rotation_deg".into(), num(*rotation));
                m.insert("align_h".into(), align_h.word().into());
                m.insert("align_v".into(), align_v.word().into());
            }
        }
        Value::Object(m)
    }
}

/// A JSON number from a finite f64.
pub fn num(x: f64) -> Value {
    Number::from_f64(x).map_or(Value::Null, Value::Number)
}

/// A JSON point.
pub fn point_value(p: [f64; 2]) -> Value {
    Value::Array(vec![num(p[0]), num(p[1])])
}

fn cmp_f(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

fn geometry(e: &Entity) -> Vec<f64> {
    match &e.shape {
        Shape::Line { from, to, .. } => vec![from[0], from[1], to[0], to[1]],
        Shape::Polyline { points, closed, .. } => points
            .iter()
            .flat_map(|p| [p[0], p[1]])
            .chain([if *closed { 1.0 } else { 0.0 }])
            .collect(),
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
            ..
        } => vec![centre[0], centre[1], *radius, *start, *sweep],
        Shape::Circle { centre, radius, .. } => vec![centre[0], centre[1], *radius],
        Shape::Hatch {
            boundary,
            holes,
            angle,
            ..
        } => boundary
            .iter()
            .flat_map(|p| [p[0], p[1]])
            .chain([holes.len() as f64])
            .chain(
                holes
                    .iter()
                    .flat_map(|h| h.iter().flat_map(|p| [p[0], p[1]])),
            )
            .chain([*angle])
            .collect(),
        Shape::Text {
            anchor,
            height,
            rotation,
            ..
        } => vec![anchor[0], anchor[1], *height, *rotation],
    }
}

fn last_text(e: &Entity) -> &str {
    match &e.shape {
        Shape::Text { text, .. } => text,
        Shape::Hatch { pattern, .. } => pattern,
        _ => "",
    }
}

fn aligns(e: &Entity) -> (&'static str, &'static str) {
    match &e.shape {
        Shape::Text {
            align_h, align_v, ..
        } => (align_h.word(), align_v.word()),
        _ => ("", ""),
    }
}

/// The order of clause 16.8: strings byte by byte, numbers as numbers, lists number by number.
pub fn compare(a: &Entity, b: &Entity) -> Ordering {
    a.origin
        .as_bytes()
        .cmp(b.origin.as_bytes())
        .then_with(|| a.type_name().cmp(b.type_name()))
        .then_with(|| a.layer.as_bytes().cmp(b.layer.as_bytes()))
        .then_with(|| {
            a.id.as_deref()
                .unwrap_or("")
                .as_bytes()
                .cmp(b.id.as_deref().unwrap_or("").as_bytes())
        })
        .then_with(|| a.line_type().as_bytes().cmp(b.line_type().as_bytes()))
        .then_with(|| cmp_f(a.width(), b.width()))
        .then_with(|| {
            let (ga, gb) = (geometry(a), geometry(b));
            for (x, y) in ga.iter().zip(&gb) {
                let o = cmp_f(*x, *y);
                if o != Ordering::Equal {
                    return o;
                }
            }
            ga.len().cmp(&gb.len())
        })
        .then_with(|| last_text(a).as_bytes().cmp(last_text(b).as_bytes()))
        .then_with(|| aligns(a).0.cmp(aligns(b).0))
        .then_with(|| aligns(a).1.cmp(aligns(b).1))
}
