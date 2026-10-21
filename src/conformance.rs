//! The runner side of the conformance suite (clause 17): unit functions and result comparison.
//!
//! Nothing here reads a file. The test and the CLI read the suite and pass its values in.

use crate::dimensions::{self, ChainInfo};
use crate::furniture;
use crate::numbers::{cos_sin, mm, round_half_even};
use crate::rules::{self, AlignH, AlignV, Rect, Window};
use crate::symbols;
use serde_json::{Map, Value};

/// Clause 16.9: two parsed values are equal. A whole number equals the same number with a point.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// Clause 17.4: the differences between a runner result and an expected file (empty when equal).
pub fn compare(result: &Value, expected: &Value) -> Vec<String> {
    let want = expected["result"].as_str().unwrap_or("");
    let got = result["result"].as_str().unwrap_or("");
    if got != want {
        return vec![format!(
            "result is {got:?}, expected {want:?}: {}",
            result["errors"]
        )];
    }
    if want == "error" {
        let clauses = |v: &Value| -> Vec<String> {
            let mut c: Vec<String> = v["errors"].as_array().map_or(vec![], |a| {
                a.iter()
                    .filter_map(|e| e["clause"].as_str().map(str::to_string))
                    .collect()
            });
            c.sort();
            c
        };
        let (g, w) = (clauses(result), clauses(expected));
        return if g == w {
            vec![]
        } else {
            vec![format!("errors {g:?} != {w:?}")]
        };
    }
    let mut problems = Vec::new();
    let norm = &result["normalised"];
    for key in ["sheet", "viewports"] {
        if let Some(w) = expected.get(key)
            && !same(&norm[key], w)
        {
            problems.push(format!("{key} differs"));
        }
    }
    if let Some(want_entities) = expected.get("entities") {
        let all = norm["entities"].as_array().cloned().unwrap_or_default();
        let prefixes: Vec<&str> = expected
            .get("origin_prefixes")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let entities: Vec<Value> = if prefixes.is_empty() {
            all
        } else {
            all.into_iter()
                .filter(|e| {
                    prefixes
                        .iter()
                        .any(|p| e["origin"].as_str().unwrap_or("").starts_with(p))
                })
                .collect()
        };
        let got_value = Value::Array(entities.clone());
        if !same(&got_value, want_entities) {
            problems.push(first_entity_difference(
                &entities,
                want_entities.as_array().map_or(&[], Vec::as_slice),
            ));
        }
    }
    problems
}

fn first_entity_difference(got: &[Value], want: &[Value]) -> String {
    if got.len() != want.len() {
        return format!("entities: {} found, {} expected", got.len(), want.len());
    }
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        if !same(g, w) {
            return format!("entities[{i}] differs: {g} != {w}");
        }
    }
    "entities differ".to_string()
}

fn rect(v: &Value) -> Rect {
    let n = |k: &str| v[k].as_f64().unwrap_or(0.0);
    Rect {
        x: n("x_mm"),
        y: n("y_mm"),
        w: n("width_mm"),
        h: n("height_mm"),
    }
}

fn window(v: &Value) -> Window {
    let n = |k: &str| v[k].as_f64().unwrap_or(0.0);
    Window {
        xmin: n("xmin_m"),
        ymin: n("ymin_m"),
        xmax: n("xmax_m"),
        ymax: n("ymax_m"),
    }
}

fn num(x: f64) -> Value {
    crate::entities::num(x)
}

fn error(clause: &str) -> Value {
    let mut m = Map::new();
    m.insert("error".into(), clause.into());
    Value::Object(m)
}

fn nm(v: &Value, key: &str) -> f64 {
    v[key].as_f64().unwrap_or(0.0)
}

/// Clause 17.5: apply one unit function to one input. `None` when the function is not known here.
pub fn run_unit(function: &str, input: &Value) -> Option<Value> {
    let rot = || nm(input, "rotation_deg").rem_euclid(360.0);
    let scale = || input["scale"].as_u64().unwrap_or(1) as u32;
    Some(match function {
        "round_half_even" => num(round_half_even(
            nm(input, "value"),
            nm(input, "places") as u32,
        )),
        "snap_width" => {
            rules::snap_width(nm(input, "width_mm")).map_or_else(|| error("8.3.2"), num)
        }
        "drawing_space" => {
            let landscape = input["orientation"].as_str() == Some("landscape");
            let (_, _, ds) = rules::drawing_space(input["size"].as_str().unwrap_or(""), landscape)?;
            let mut m = Map::new();
            m.insert("x_mm".into(), num(ds.x));
            m.insert("y_mm".into(), num(ds.y));
            m.insert("width_mm".into(), num(ds.w));
            m.insert("height_mm".into(), num(ds.h));
            Value::Object(m)
        }
        "map_point" => {
            let m = rules::Mapper::new(
                rect(&input["frame"]),
                window(&input["window"]),
                scale(),
                rot(),
            );
            let q = m.point([input["point_m"][0].as_f64()?, input["point_m"][1].as_f64()?]);
            Value::Array(vec![num(mm(q[0])), num(mm(q[1]))])
        }
        "window_fits" => Value::Bool(rules::window_fits(
            rect(&input["frame"]),
            window(&input["window"]),
            scale(),
            rot(),
        )),
        "fit_scale" => rules::fit_scale(rect(&input["frame"]), window(&input["window"]), rot())
            .map_or(Value::Null, Value::from),
        "bar_length" => rules::derived_bar_length(nm(input, "frame_width_mm"), scale())
            .map_or_else(|| error("7.9.4"), num),
        "bar_labels" => Value::Array(
            rules::bar_labels(
                nm(input, "length_m"),
                nm(input, "divisions") as u32,
                input["unit"].as_str().unwrap_or("m"),
            )
            .into_iter()
            .map(Value::String)
            .collect(),
        ),
        "north_deg" => num(round_half_even(
            rules::north_deg(nm(input, "rotation_deg"), nm(input, "model_bearing_deg")),
            3,
        )),
        "line_pattern" => Value::Array(
            rules::line_pattern(input["line_type"].as_str()?, nm(input, "width_mm"))?
                .into_iter()
                .map(num)
                .collect(),
        ),
        "arc_sweep" => rules::arc_sweep(nm(input, "start_deg"), nm(input, "end_deg"))
            .map_or_else(|| error("9.5.1"), num),
        "text_width" => num(rules::text_width(
            input["text"].as_str()?,
            nm(input, "height_mm"),
        )?),
        "text_box" => {
            let anchor = [
                input["anchor_mm"][0].as_f64()?,
                input["anchor_mm"][1].as_f64()?,
            ];
            let corners = rules::text_box(
                anchor,
                input["text"].as_str()?,
                nm(input, "height_mm"),
                nm(input, "rotation_deg"),
                AlignH::parse(input["align_h"].as_str().unwrap_or("left")),
                AlignV::parse(input["align_v"].as_str().unwrap_or("baseline")),
            )?;
            Value::Array(
                corners
                    .iter()
                    .map(|c| Value::Array(vec![num(mm(c[0])), num(mm(c[1]))]))
                    .collect(),
            )
        }
        "dimension_text" => {
            let factor = if input["unit"].as_str() == Some("mm") {
                1000.0
            } else {
                1.0
            };
            Value::String(dimensions::format_value(
                nm(input, "value_m") * factor,
                nm(input, "decimals") as u32,
            ))
        }
        "reading_rotation" => {
            let (c, s) = cos_sin(nm(input, "theta_deg"));
            num(round_half_even(dimensions::reading_rotation([c, -s]), 3))
        }
        "chain_ok" => {
            let info = |role: &str, text: &str| ChainInfo {
                id: "c".into(),
                role: role.into(),
                text: text.into(),
                unit: "m".into(),
                decimals: 2,
            };
            let mut infos: Vec<ChainInfo> = input["parts"]
                .as_array()?
                .iter()
                .filter_map(|t| t.as_str())
                .map(|t| info("part", t))
                .collect();
            infos.push(info("overall", input["overall"].as_str()?));
            Value::Bool(dimensions::check_chains(&infos))
        }
        "symbol_point" => {
            let pt = |v: &Value| Some([v[0].as_f64()?, v[1].as_f64()?]);
            let placement = symbols::Placement::new(
                pt(&input["position_m"])?,
                nm(input, "rotation_deg"),
                nm(input, "scale"),
                pt(&input["insertion"])?,
                input["units"].as_str() == Some("paper_mm"),
                nm(input, "denominator") as u32,
            );
            let q = placement.model(pt(&input["q"])?);
            Value::Array(vec![
                num(round_half_even(q[0], 6)),
                num(round_half_even(q[1], 6)),
            ])
        }
        "revision_seals" => Value::Array(
            furniture::seals(input["entries"].as_array()?)
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
        "register_layout" => {
            let (rows, omitted) = furniture::register_layout(
                nm(input, "entries") as usize,
                nm(input, "max_rows") as usize,
            );
            let mut m = Map::new();
            m.insert("rows".into(), Value::from(rows));
            m.insert("omitted".into(), Value::from(omitted));
            Value::Object(m)
        }
        _ => return None,
    })
}
