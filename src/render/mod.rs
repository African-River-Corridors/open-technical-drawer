//! Outputs made from normalised content only (clause 15): SVG and PDF here, DXF in `dxf`.
//!
//! Every writer takes a [`crate::Normalised`] and nothing else, so what a sheet looks like is decided
//! by the standard (clause 16), not by the writer. Output is the same on every run: nothing reads a
//! clock, a random source or a file.

pub mod dxf;
pub mod pdf;
pub mod svg;

use crate::numbers::cos_sin;

/// A number for a path or a matrix: up to `places` decimals, no trailing zeros, no "-0".
pub(crate) fn fmt(value: f64, places: usize) -> String {
    let text = format!("{value:.places$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    };
    if text == "-0" || text.is_empty() {
        "0".to_string()
    } else {
        text
    }
}

/// The dashes of a line type for a width, in millimetres, as drawn: gaps are positive lengths.
pub(crate) fn dash_array(line_type: &str, width_mm: f64) -> Vec<f64> {
    if line_type == "continuous" {
        return vec![];
    }
    crate::rules::line_pattern(line_type, width_mm)
        .unwrap_or_default()
        .into_iter()
        .map(f64::abs)
        .collect()
}

/// The point at paper angle `deg` (counter-clockwise as seen) on a circle, in y-down coordinates.
pub(crate) fn on_circle(centre: [f64; 2], r: f64, deg: f64) -> [f64; 2] {
    let (c, s) = cos_sin(deg);
    [centre[0] + r * c, centre[1] - r * s]
}
