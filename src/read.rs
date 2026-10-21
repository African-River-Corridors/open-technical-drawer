//! Small readers for a document that passed the schema: the shapes are known, and a reader never panics.

use crate::rules::{Rect, Window};
use serde_json::Value;

pub(crate) fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

pub(crate) fn p(v: &Value) -> [f64; 2] {
    [f(&v[0]), f(&v[1])]
}

pub(crate) fn st(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

pub(crate) fn arr(v: &Value) -> &[Value] {
    v.as_array().map_or(&[], Vec::as_slice)
}

pub(crate) fn points(v: &Value) -> Vec<[f64; 2]> {
    arr(v).iter().map(p).collect()
}

pub(crate) fn rect_of(v: &Value) -> Rect {
    Rect {
        x: f(&v["x_mm"]),
        y: f(&v["y_mm"]),
        w: f(&v["width_mm"]),
        h: f(&v["height_mm"]),
    }
}

pub(crate) fn window_of(v: &Value) -> Window {
    Window {
        xmin: f(&v["xmin_m"]),
        ymin: f(&v["ymin_m"]),
        xmax: f(&v["xmax_m"]),
        ymax: f(&v["ymax_m"]),
    }
}

pub(crate) fn entities_of(vp: &Value) -> &[Value] {
    arr(&vp["entities"])
}

pub(crate) fn rotation_of(vp: &Value) -> f64 {
    vp.get("rotation_deg").map_or(0.0, f).rem_euclid(360.0)
}

pub(crate) fn orientation_of(sheet: &Value) -> &str {
    sheet
        .get("orientation")
        .and_then(Value::as_str)
        .unwrap_or("landscape")
}
