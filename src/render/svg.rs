//! SVG from normalised content (clause 15.2). One SVG unit is one millimetre of paper.
//!
//! Everything is drawn in black (clause 8.6.1). A text is placed by the width table, so where it sits
//! does not depend on the machine: it gets the font size `h / 0.7` and the width of clause 11.7.1.

use super::{dash_array, fmt, on_circle};
use crate::Normalised;
use crate::data;
use crate::entities::{Entity, Shape};
use crate::rules;

/// The font family named in the SVG.
pub const FONT_FAMILY: &str = "Barlow Semi Condensed";

/// How to write the SVG.
#[derive(Debug, Clone, Copy)]
pub struct SvgOptions {
    /// Embed the font as a data URI (a file for a drafter). The app turns this off and loads the
    /// font once in its own style sheet.
    pub embed_font: bool,
}

impl Default for SvgOptions {
    fn default() -> Self {
        SvgOptions { embed_font: true }
    }
}

fn n(value: f64) -> String {
    fmt(value, 3)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn stroke(line_type: &str, width: f64) -> String {
    let dash = dash_array(line_type, width);
    let dash = if dash.is_empty() {
        String::new()
    } else {
        format!(
            " stroke-dasharray=\"{}\"",
            dash.iter().map(|d| n(*d)).collect::<Vec<_>>().join(" ")
        )
    };
    format!(
        "fill=\"none\" stroke=\"#000\" stroke-width=\"{}\"{dash}",
        n(width)
    )
}

fn loop_path(points: &[[f64; 2]]) -> String {
    format!(
        "M{} Z",
        points
            .iter()
            .map(|p| format!("{} {}", n(p[0]), n(p[1])))
            .collect::<Vec<_>>()
            .join(" L")
    )
}

fn arc_path(centre: [f64; 2], r: f64, start: f64, sweep: f64) -> String {
    let p0 = on_circle(centre, r, start);
    let p1 = on_circle(centre, r, start + sweep);
    let large = u8::from(sweep > 180.0);
    format!(
        "M{} {} A{} {} 0 {large} 0 {} {}",
        n(p0[0]),
        n(p0[1]),
        n(r),
        n(r),
        n(p1[0]),
        n(p1[1])
    )
}

fn one(e: &Entity) -> Option<String> {
    let attrs = format!(
        "data-origin=\"{}\" data-layer=\"{}\"",
        escape(&e.origin),
        escape(&e.layer)
    );
    Some(match &e.shape {
        Shape::Line {
            from,
            to,
            line_type,
            width,
        } => {
            format!(
                "<line {attrs} x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {}/>",
                n(from[0]),
                n(from[1]),
                n(to[0]),
                n(to[1]),
                stroke(line_type, *width)
            )
        }
        Shape::Polyline {
            points,
            closed,
            line_type,
            width,
        } => {
            let d = if *closed {
                loop_path(points)
            } else {
                format!(
                    "M{}",
                    points
                        .iter()
                        .map(|p| format!("{} {}", n(p[0]), n(p[1])))
                        .collect::<Vec<_>>()
                        .join(" L")
                )
            };
            format!("<path {attrs} d=\"{d}\" {}/>", stroke(line_type, *width))
        }
        Shape::Circle {
            centre,
            radius,
            line_type,
            width,
        } => {
            format!(
                "<circle {attrs} cx=\"{}\" cy=\"{}\" r=\"{}\" {}/>",
                n(centre[0]),
                n(centre[1]),
                n(*radius),
                stroke(line_type, *width)
            )
        }
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
            line_type,
            width,
        } => {
            format!(
                "<path {attrs} d=\"{}\" {}/>",
                arc_path(*centre, *radius, *start, *sweep),
                stroke(line_type, *width)
            )
        }
        Shape::Hatch {
            boundary,
            holes,
            pattern,
            ..
        } => {
            // A pattern other than solid draws only its marks, which are entities of their own (clause 10.5.8).
            if pattern != "solid" {
                return None;
            }
            let d = std::iter::once(boundary)
                .chain(holes.iter())
                .map(|l| loop_path(l))
                .collect::<Vec<_>>()
                .join(" ");
            format!("<path {attrs} d=\"{d}\" fill=\"#000\" fill-rule=\"evenodd\" stroke=\"none\"/>")
        }
        Shape::Text {
            anchor,
            text,
            height,
            rotation,
            align_h,
            align_v,
        } => {
            let corners = rules::text_box(*anchor, text, *height, *rotation, *align_h, *align_v)?;
            let [x0, y0] = corners[0];
            let width = rules::text_width_exact(text, *height)?;
            let transform = if *rotation == 0.0 {
                String::new()
            } else {
                format!(
                    " transform=\"rotate({} {} {})\"",
                    n(-rotation),
                    n(x0),
                    n(y0)
                )
            };
            format!(
                "<text {attrs} x=\"{}\" y=\"{}\" font-size=\"{}\" textLength=\"{}\" lengthAdjust=\"spacing\" fill=\"#000\"{transform}>{}</text>",
                n(x0),
                n(y0),
                n(height * 1000.0 / data::font_widths().cap_height_units),
                n(width),
                escape(text)
            )
        }
    })
}

/// The SVG of a sheet, made only from its normalised content.
pub fn to_svg(norm: &Normalised, options: &SvgOptions) -> String {
    let (w, h) = (norm.sheet.width_mm, norm.sheet.height_mm);
    let mut parts = vec![
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>".to_string(),
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\">",
            n(w),
            n(h),
            n(w),
            n(h)
        ),
    ];
    let mut style = format!("text{{font-family:'{FONT_FAMILY}',sans-serif}}");
    if options.embed_font {
        style = format!(
            "@font-face{{font-family:'{FONT_FAMILY}';src:url(data:font/ttf;base64,{}) format('truetype')}}{style}",
            base64(data::FONT_FILE)
        );
    }
    parts.push(format!("<style>{style}</style>"));
    parts.extend(norm.entities.iter().filter_map(one));
    parts.push("</svg>".to_string());
    parts.join("\n") + "\n"
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(char::from(TABLE[(n >> 18) as usize & 63]));
        out.push(char::from(TABLE[(n >> 12) as usize & 63]));
        out.push(if chunk.len() > 1 {
            char::from(TABLE[(n >> 6) as usize & 63])
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            char::from(TABLE[n as usize & 63])
        } else {
            '='
        });
    }
    out
}
