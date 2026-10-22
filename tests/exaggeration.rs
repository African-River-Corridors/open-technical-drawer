//! Standard 1.2: the vertical exaggeration of a viewport (clauses 7.3.6, 7.4.5, 7.5.2, 7.9.11,
//! 15.4.13 and 16.6.5), in the rules and the outputs.

mod support {
    pub mod dxf_reader;
}

use otd::entities::{Entity, Shape};
use otd::render::{dxf::to_dxf, svg::SvgOptions, svg::to_svg};
use otd::rules::{self, Mapper, Rect, Window};
use otd::{Files, NoFiles, Normalised, RunResult, run_text, run_value};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use support::dxf_reader::records;

struct DirFiles(PathBuf);

impl Files for DirFiles {
    fn read(&self, relative_path: &str) -> Option<String> {
        fs::read_to_string(self.0.join(relative_path)).ok()
    }
    fn read_bytes(&self, relative_path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(relative_path)).ok()
    }
}

fn dir(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("standard/conformance/documents")
        .join(id)
}

fn text_of(id: &str) -> String {
    fs::read_to_string(dir(id).join("input.json")).expect("input")
}

fn case(id: &str) -> Normalised {
    match run_text(&text_of(id), &DirFiles(dir(id))) {
        RunResult::Ok(n) => *n,
        RunResult::Errors(e) => panic!("{id}: {e:?}"),
    }
}

fn texts(norm: &Normalised, origin: &str) -> Vec<String> {
    norm.entities
        .iter()
        .filter(|e| e.origin == origin)
        .filter_map(|e| match &e.shape {
            Shape::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn without_an_exaggeration_the_map_is_the_one_of_version_1_1() {
    // 18.2.2: E = 1 gives k_v = k, bit for bit, so no 1.0 or 1.1 point moves.
    let frame = Rect {
        x: 20.0,
        y: 10.0,
        w: 390.0,
        h: 277.0,
    };
    let window = Window {
        xmin: 0.0,
        ymin: 0.0,
        xmax: 100.0,
        ymax: 50.0,
    };
    for n in [1, 25, 333, 1250, 100000] {
        let m = Mapper::new(frame, window, n, 0.0);
        assert_eq!(m.kv().to_bits(), m.k.to_bits());
        assert_eq!(m.k, 1000.0 / f64::from(n));
        let e = Mapper::exaggerated(frame, window, n, 0.0, 1);
        assert_eq!(e.point([12.5, 7.25]), m.point([12.5, 7.25]));
    }
}

#[test]
fn the_worked_numbers_of_clause_7_4_5() {
    let frame = Rect {
        x: 20.0,
        y: 10.0,
        w: 390.0,
        h: 200.0,
    };
    let window = Window {
        xmin: 0.0,
        ymin: 0.0,
        xmax: 1800.0,
        ymax: 80.0,
    };
    let m = Mapper::exaggerated(frame, window, 5000, 0.0, 10);
    assert_eq!((m.k, m.kv()), (0.2, 2.0));
    for (model, paper) in [
        ([0.0, 40.0], [35.0, 110.0]),
        ([1800.0, 32.0], [395.0, 126.0]),
        ([900.0, 50.0], [215.0, 90.0]),
    ] {
        let [x, y] = m.point(model);
        assert!(
            (x - paper[0]).abs() < 1e-9 && (y - paper[1]).abs() < 1e-9,
            "{model:?}"
        );
    }
}

#[test]
fn fit_tries_only_ratios_whose_vertical_ratio_is_a_ratio() {
    // 7.5.3: frame 390 x 120 mm, window 3000 x 100 m, E = 20, scale fit.
    let frame = Rect {
        x: 20.0,
        y: 10.0,
        w: 390.0,
        h: 120.0,
    };
    let window = Window {
        xmin: 0.0,
        ymin: 0.0,
        xmax: 3000.0,
        ymax: 100.0,
    };
    let ratios = rules::ratios(2);
    assert_eq!(
        rules::fit_scale_exaggerated(&ratios, frame, window, 0.0, 20),
        Some(20000)
    );
    assert_eq!(
        rules::fit_scale_exaggerated(&ratios, frame, window, 0.0, 1),
        rules::fit_scale_in(&ratios, frame, window, 0.0)
    );
    assert_eq!(
        rules::fit_scale_in(&ratios, frame, window, 0.0),
        Some(10000)
    );
    assert_eq!(rules::vertical_denominator(5000, 10, &ratios), Some(500));
    assert_eq!(rules::vertical_denominator(5000, 3, &ratios), None);
    assert_eq!(rules::vertical_denominator(1250, 20, &ratios), None);
}

#[test]
fn the_words_are_in_full_and_the_font_has_the_multiplication_sign() {
    assert_eq!(
        rules::exaggeration_words(10),
        "Vertical exaggeration \u{00d7}10"
    );
    assert!(rules::text_width("\u{00d7}", 2.5).is_some()); // 11.6.2
}

#[test]
fn a_1_1_document_has_no_new_member_and_no_vertical_bar() {
    let norm = case("exaggeration-absent-1-1");
    assert!(
        norm.viewports
            .iter()
            .all(|v| v.vertical_exaggeration.is_none())
    );
    assert!(
        !norm
            .entities
            .iter()
            .any(|e| e.origin.ends_with(".vertical_scale_bar"))
    );
    let value = norm.to_value();
    assert!(
        value["viewports"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v.get("vertical_exaggeration").is_none())
    );
}

#[test]
fn the_worked_numbers_of_clause_7_9_13() {
    let norm = case("exaggeration-long-section");
    assert_eq!(norm.viewports[0].vertical_exaggeration, Some(10));
    assert_eq!(
        norm.to_value()["viewports"][0]["vertical_exaggeration"],
        Value::from(10)
    );
    let bar: Vec<&Entity> = norm
        .entities
        .iter()
        .filter(|e| e.origin == "viewport.main.vertical_scale_bar")
        .collect();
    let outline = bar
        .iter()
        .find_map(|e| match &e.shape {
            Shape::Polyline { points, .. } => Some(points.clone()),
            _ => None,
        })
        .expect("the outline");
    assert_eq!(
        outline,
        vec![[26.0, 144.0], [29.0, 144.0], [29.0, 184.0], [26.0, 184.0]]
    );
    let anchors: Vec<(String, [f64; 2])> = bar
        .iter()
        .filter_map(|e| match &e.shape {
            Shape::Text { text, anchor, .. } => Some((text.clone(), *anchor)),
            _ => None,
        })
        .collect();
    for (text, at) in [
        ("0", [31.5, 184.0]),
        ("20 m", [31.5, 144.0]),
        ("Vertical 1:500", [26.0, 141.0]),
        ("Vertical exaggeration \u{00d7}10", [26.0, 208.0]),
    ] {
        assert!(
            anchors.iter().any(|(t, a)| t == text && *a == at),
            "{text} at {at:?}: {anchors:?}"
        );
    }
    assert!(texts(&norm, "viewport.main.scale_bar").contains(&"1:5000".to_string()));
}

#[test]
fn the_title_block_is_as_shown() {
    let norm = case("exaggeration-title-block-as-shown");
    let all: Vec<String> = norm
        .entities
        .iter()
        .filter(|e| e.origin == "sheet.title_block")
        .filter_map(|e| match &e.shape {
            Shape::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(all.contains(&"AS SHOWN".to_string()), "{all:?}");
}

#[test]
fn svg_prints_the_words_and_both_ratios() {
    let svg = to_svg(
        &case("exaggeration-long-section"),
        &SvgOptions { embed_font: false },
    );
    assert!(svg.contains("Vertical exaggeration \u{00d7}10"));
    assert!(svg.contains("Vertical 1:500") && svg.contains("1:5000"));
    assert!(!svg.contains("V.E."));
}

#[test]
fn dxf_writes_the_exaggerated_viewport_in_paper_space() {
    // 15.4.13: nothing of the viewport in model space; its entities in paper space with their origin.
    let id = "exaggeration-long-section";
    let doc = otd::json::parse(&text_of(id)).unwrap();
    let norm = case(id);
    let text = to_dxf(&doc, &norm, &DirFiles(dir(id))).text;
    let recs = records(&text);
    assert!(!recs.iter().any(|r| r.section == "ENTITIES"
        && r.get(67) != Some("1")
        && r.xdata().get("vp").map(String::as_str) == Some("main")));
    let origins: Vec<String> = recs
        .iter()
        .filter_map(|r| r.xdata().get("origin").cloned())
        .collect();
    assert!(origins.iter().any(|o| o == "viewport.main"));
    assert!(
        origins
            .iter()
            .any(|o| o == "viewport.main.vertical_scale_bar")
    );
    let vp = recs
        .iter()
        .find(|r| r.kind == "VIEWPORT" && r.xdata().get("id").map(String::as_str) == Some("main"))
        .expect("the viewport");
    assert_eq!(vp.xdata()["vertical_exaggeration"], "10");
    assert_eq!(vp.xdata()["scale"], "5000");
}

#[test]
fn a_dimension_in_an_exaggerated_viewport_states_the_model_length() {
    let norm = case("exaggeration-dimension-leader");
    assert_eq!(
        texts(&norm, "viewport.main.dimension"),
        vec!["1200.00".to_string()]
    );
}

/// The long section with an empty viewport of the given frame height, scale and exaggeration.
fn document(height_mm: f64, scale: u32, ex: u32) -> Value {
    let mut doc = otd::json::parse(&text_of("exaggeration-long-section")).unwrap();
    let vp = &mut doc["viewports"][0];
    vp["frame"]["height_mm"] = height_mm.into();
    vp["scale"] = scale.into();
    vp["vertical_exaggeration"] = ex.into();
    vp["entities"] = Value::Array(vec![]);
    vp["window"] =
        otd::json::parse(r#"{"xmin_m": 0, "ymin_m": 0, "xmax_m": 0.01, "ymax_m": 0.01}"#).unwrap();
    doc
}

fn boxed(e: &Entity) -> [f64; 4] {
    let pts: Vec<[f64; 2]> = match &e.shape {
        Shape::Text {
            anchor,
            text,
            height,
            rotation,
            align_h,
            align_v,
        } => rules::text_box(*anchor, text, *height, *rotation, *align_h, *align_v)
            .expect("a known text")
            .to_vec(),
        Shape::Line { from, to, .. } => vec![*from, *to],
        Shape::Polyline { points, .. } => points.clone(),
        Shape::Hatch { boundary, .. } => boundary.clone(),
        other => panic!("{other:?}"),
    };
    let xs = pts.iter().map(|p| p[0]);
    let ys = pts.iter().map(|p| p[1]);
    [
        xs.clone().fold(f64::INFINITY, f64::min),
        ys.clone().fold(f64::INFINITY, f64::min),
        xs.fold(f64::NEG_INFINITY, f64::max),
        ys.fold(f64::NEG_INFINITY, f64::max),
    ]
}

fn apart(a: [f64; 4], b: [f64; 4]) -> bool {
    !(a[2].min(b[2]) > a[0].max(b[0]) && a[3].min(b[3]) > a[1].max(b[1]))
}

#[test]
fn the_vertical_bar_never_touches_the_scale_bar() {
    // 7.9.11: for every frame height of 7.9.12 and every pair of ratios, the boxes are apart.
    let ratios = rules::ratios(2);
    let ratios = ratios.as_slice();
    let pairs: Vec<(u32, u32)> = ratios
        .iter()
        .flat_map(|&n| {
            [2, 4, 5, 10, 20, 50, 100, 500, 1000]
                .into_iter()
                .filter(move |&e| n.is_multiple_of(e) && ratios.contains(&(n / e)))
                .map(move |e| (n, e))
        })
        .collect();
    assert!(pairs.len() > 20, "{}", pairs.len());
    let words = |e: &Entity| matches!(&e.shape, Shape::Text { text, .. } if text.starts_with("Vertical exag"));
    for height in [100.0, 137.0, 200.0, 277.0] {
        for &(n, ex) in &pairs {
            let RunResult::Ok(norm) = run_value(&document(height, n, ex), &NoFiles) else {
                panic!("{height} 1:{n} x{ex}")
            };
            let vertical: Vec<&Entity> = norm
                .entities
                .iter()
                .filter(|e| e.origin.ends_with(".vertical_scale_bar"))
                .collect();
            let others: Vec<[f64; 4]> = norm
                .entities
                .iter()
                .filter(|e| e.origin.ends_with(".scale_bar"))
                .map(boxed)
                .collect();
            assert!(vertical.iter().any(|e| words(e)) && !others.is_empty());
            for v in &vertical {
                let b = boxed(v);
                assert!(
                    others.iter().all(|o| apart(b, *o)),
                    "{height} 1:{n} x{ex}: {b:?}"
                );
            }
            let top = vertical
                .iter()
                .filter(|e| matches!(e.shape, Shape::Polyline { .. }))
                .map(|e| boxed(e)[1])
                .fold(f64::INFINITY, f64::min);
            // 7.9.11: the top end is at least 37 mm below the top of the frame.
            assert!(top >= 10.0 + 37.0 - 1e-9, "{height} 1:{n} x{ex}: {top}");
        }
    }
}

#[test]
fn an_exaggerated_viewport_has_no_north_arrow_at_any_bearing() {
    // 7.3.6.9: a section has no north, and an exaggerated plan distorts every bearing.
    for bearing in [0, 90, 30] {
        let mut doc = document(200.0, 5000, 10);
        doc["viewports"][0]["north"] =
            otd::json::parse(&format!(r#"{{"model_bearing_deg": {bearing}}}"#)).unwrap();
        assert_eq!(
            run_value(&doc, &NoFiles).to_value(),
            otd::json::parse(r#"{"result": "error", "errors": [{"clause": "7.3.6.9"}]}"#).unwrap()
        );
    }
}

#[test]
fn the_exaggerated_sheets_pass_the_layout_review() {
    for id in [
        "exaggeration-absent-1-1",
        "exaggeration-dimension-leader",
        "exaggeration-fit",
        "exaggeration-long-section",
        "exaggeration-title-block-as-shown",
    ] {
        let findings = otd::review::review(&case(id), &otd::review::ReviewOptions::default());
        assert!(findings.is_empty(), "{id}: {findings:?}");
    }
}
