//! The DXF output (standard clause 15.4), read back with a reader that shares no code with the writer
//! (`support/dxf_reader.rs`, which works on the group codes of the file).

mod support {
    pub mod dxf_reader;
    pub mod pending;
}

use otd::entities::{self, Entity};
use otd::render::dxf::{FONT_FILE_NAME, to_dxf};
use otd::{Files, Normalised, RunResult, run_text};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use support::dxf_reader::{read_normalised, records, style_tables};

struct DirFiles(PathBuf);

impl Files for DirFiles {
    fn read(&self, relative_path: &str) -> Option<String> {
        fs::read_to_string(self.0.join(relative_path)).ok()
    }
}

fn suite() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("standard/conformance")
}

/// Every conformance document that has a result (not an error), and the schema examples that are valid.
fn ok_inputs() -> Vec<PathBuf> {
    let list = otd::json::parse(&fs::read_to_string(suite().join("suite.json")).unwrap()).unwrap();
    let mut out = Vec::new();
    for case in list["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if case["kind"] == "document" && !support::pending::EXPECTED_TO_FAIL.contains(&id) {
            let dir = suite().join(case["dir"].as_str().unwrap());
            let expected =
                otd::json::parse(&fs::read_to_string(dir.join("expected.json")).unwrap()).unwrap();
            if expected["result"] == "ok" {
                out.push(dir.join("input.json"));
            }
        }
    }
    out
}

fn build(path: &Path) -> (Value, Normalised, String, Vec<String>) {
    let text = fs::read_to_string(path).unwrap();
    let files = DirFiles(path.parent().unwrap().to_path_buf());
    let RunResult::Ok(norm) = run_text(&text, &files) else {
        panic!("{} has errors", path.display())
    };
    let doc = otd::json::parse(&text).unwrap();
    let dxf = to_dxf(&doc, &norm, &files);
    (doc, *norm, dxf.text, dxf.warnings)
}

fn named(id: &str) -> PathBuf {
    suite().join("documents").join(id).join("input.json")
}

#[test]
fn dxf_read_back_gives_the_same_normalised_content() {
    let inputs = ok_inputs();
    assert!(inputs.len() > 90, "{} documents", inputs.len());
    for path in inputs {
        let (_, norm, text, _) = build(&path);
        let back = read_normalised(&text);
        let mut want: Vec<Entity> = norm.entities.clone();
        let mut got: Vec<Entity> = back.entities;
        want.sort_by(entities::compare);
        got.sort_by(entities::compare);
        let id = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(got.len(), want.len(), "{id}");
        for (g, w) in got.iter().zip(&want) {
            assert_eq!(g, w, "{id}");
        }
        assert_eq!(
            (back.width, back.height),
            (norm.sheet.width_mm, norm.sheet.height_mm),
            "{id}"
        );
        let ids: Vec<&String> = back.viewports.keys().collect();
        let want_ids: Vec<&String> = norm.viewports.iter().map(|v| &v.id).collect();
        assert_eq!(ids, want_ids, "{id}");
        for vp in &norm.viewports {
            let b = &back.viewports[&vp.id];
            assert_eq!(b.n, vp.scale, "{id}: the viewport holds the stated scale");
            let close = |a: f64, c: f64| (a - c).abs() < 1e-6;
            assert!(
                close(b.frame.x, vp.frame.x)
                    && close(b.frame.y, vp.frame.y)
                    && close(b.frame.w, vp.frame.w)
                    && close(b.frame.h, vp.frame.h),
                "{id}: the frame"
            );
        }
    }
}

#[test]
fn viewports_plot_at_the_stated_scale_and_turn_by_the_rotation() {
    for id in [
        "two-viewports",
        "ent-rotated-90",
        "fit-barge-250",
        "scale-house-1250",
        "ent-rotated-40-25",
    ] {
        let (_, norm, text, _) = build(&named(id));
        let back = read_normalised(&text);
        for vp in &norm.viewports {
            let b = &back.viewports[&vp.id];
            // paper height over model height is 1000 / N, exactly as clause 15.4.6 says
            assert!(
                (b.height / b.view_height - 1000.0 / f64::from(vp.scale)).abs() < 1e-9,
                "{id}"
            );
        }
    }
    let (_, _, text, _) = build(&named("ent-rotated-90"));
    assert_eq!(read_normalised(&text).viewports["main"].twist, 90.0);
}

#[test]
fn model_space_is_in_metres_and_paper_space_in_millimetres() {
    let (doc, _, text, _) = build(&named("ent-line"));
    let tables = style_tables(&text);
    assert_eq!(tables.header["$INSUNITS"], 6);
    assert_eq!(tables.header["$PSLTSCALE"], 1);
    assert_eq!(tables.header["$MEASUREMENT"], 1);
    let recs = records(&text);
    let model_line = recs
        .iter()
        .find(|r| r.section == "ENTITIES" && r.kind == "LINE" && r.get(67).is_none())
        .unwrap();
    let src = doc["viewports"][0]["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["type"] == "line")
        .unwrap();
    assert_eq!(
        (
            model_line.f(10),
            model_line.f(20),
            model_line.f(11),
            model_line.f(21)
        ),
        (
            src["from_m"][0].as_f64().unwrap(),
            src["from_m"][1].as_f64().unwrap(),
            src["to_m"][0].as_f64().unwrap(),
            src["to_m"][1].as_f64().unwrap()
        )
    );
    let border = recs
        .iter()
        .find(|r| {
            r.kind == "LWPOLYLINE"
                && r.xdata().get("origin").map(String::as_str) == Some("sheet.border")
        })
        .unwrap();
    let xs: Vec<f64> = border.all(10).iter().map(|v| v.parse().unwrap()).collect();
    assert_eq!(xs, [20.0, 410.0, 410.0, 20.0], "paper millimetres");
    let ys: Vec<f64> = border.all(20).iter().map(|v| v.parse().unwrap()).collect();
    assert_eq!(
        ys,
        [287.0, 287.0, 10.0, 10.0],
        "paper space is y up: (x, H - y)"
    );
}

#[test]
fn layers_carry_colour_line_type_and_snapped_width_and_no_two_share_a_colour() {
    let (_, _, text, _) = build(&named("ent-a3-widths"));
    let t = style_tables(&text);
    let styles = otd::data::line_styles();
    let group = styles.group_for("A3").unwrap();
    let standard: Vec<&String> = t
        .layers
        .keys()
        .filter(|n| styles.layer(n).is_some())
        .collect();
    assert!(!standard.is_empty());
    for name in &standard {
        let row = styles.layer(name).unwrap();
        let (aci, _, weight, plot) = &t.layers[*name];
        assert_eq!(*aci, row.aci, "{name}");
        assert_eq!(
            *weight,
            (group.width(row.role) * 100.0).round() as i64,
            "{name}"
        );
        assert!(
            styles.cad_widths_um.contains(&(weight * 10)),
            "{name}: a width a CAD file can hold"
        );
        assert_eq!(*plot, 1);
    }
    let colours: std::collections::BTreeSet<i64> =
        standard.iter().map(|n| t.layers[*n].0).collect();
    assert_eq!(colours.len(), standard.len());
    assert_eq!(t.layers["VIEWPORT"].3, 0, "the viewport frames do not plot");
}

#[test]
fn line_types_hold_the_patterns_in_paper_millimetres() {
    let (_, _, text, _) = build(&named("ent-line"));
    let t = style_tables(&text);
    assert_eq!(t.ltypes["TDFA_DASHED_018"], [2.16, -0.54]); // clause 8.4.4, d = 0.18
    assert_eq!(t.ltypes["TDFA_CHAIN_018"], [4.32, -0.54, 0.09, -0.54]);
}

#[test]
fn the_font_is_named_in_the_dxf() {
    let (_, _, text, _) = build(&named("tb-minimal"));
    assert_eq!(style_tables(&text).styles["TDFA"], FONT_FILE_NAME);
    assert_eq!(FONT_FILE_NAME, "BarlowSemiCondensed-Regular.ttf");
}

#[test]
fn symbols_are_blocks_and_inserts() {
    let (_, _, text, _) = build(&named("sym-ports"));
    let recs = records(&text);
    let blocks: Vec<&str> = recs
        .iter()
        .filter(|r| r.kind == "BLOCK")
        .filter_map(|r| r.get(2))
        .filter(|n| !n.starts_with('*'))
        .collect();
    assert_eq!(
        blocks.len(),
        1,
        "one block for a symbol used twice on one layer: {blocks:?}"
    );
    assert!(
        blocks[0].starts_with("TDFA_") && blocks[0].contains('@'),
        "{blocks:?}"
    );
    let inserts = recs.iter().filter(|r| r.kind == "INSERT").count();
    assert_eq!(inserts, 2);
}

#[test]
fn overlapping_model_windows_give_a_warning() {
    let (_, _, _, warnings) = build(&named("two-viewports"));
    assert!(
        warnings.iter().any(|w| w.contains("15.4.9")),
        "{warnings:?}"
    );
    let (_, _, _, none) = build(&named("sheet-a3-landscape"));
    assert!(none.is_empty());
}

#[test]
fn two_builds_of_one_document_are_identical() {
    for id in ["sym-ports", "tb-register-two", "hatch-concrete"] {
        assert_eq!(build(&named(id)).2, build(&named(id)).2, "{id}");
    }
}

#[test]
fn the_layout_is_named_sheet_and_has_the_size_of_the_trimmed_sheet() {
    let (_, norm, text, _) = build(&named("sheet-a2-landscape"));
    let recs = records(&text);
    let layouts: Vec<&str> = recs
        .iter()
        .filter(|r| r.kind == "LAYOUT")
        .filter_map(|r| r.nth(1, 1))
        .collect();
    assert!(
        layouts.contains(&"SHEET") && layouts.contains(&"Model"),
        "{layouts:?}"
    );
    assert_eq!(
        support::dxf_reader::sheet_size(&recs),
        (norm.sheet.width_mm, norm.sheet.height_mm)
    );
}

#[test]
fn every_handle_is_unique_and_every_owner_exists() {
    for id in ["sym-ports", "tb-register-two", "dim-chain-ok"] {
        let (_, _, text, _) = build(&named(id));
        let recs = records(&text);
        let mut handles = std::collections::BTreeSet::new();
        for r in &recs {
            for code in [5, 105] {
                if let Some(h) = r.get(code) {
                    assert!(
                        handles.insert(h.to_string()),
                        "{id}: handle {h} is used twice"
                    );
                }
            }
        }
        for r in &recs {
            if let Some(owner) = r.get(330) {
                assert!(
                    owner == "0" || handles.contains(owner),
                    "{id}: {} has the owner {owner}, which does not exist",
                    r.kind
                );
            }
        }
    }
}
