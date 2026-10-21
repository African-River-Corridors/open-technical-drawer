//! The layout review of normalised content: each rule passes and fails, and every sheet of the
//! conformance suite passes with no finding.
//!
//! Label: property

use otd::entities::Entity;
use otd::pipeline::{SheetInfo, ViewportInfo};
use otd::review::{ReviewOptions, Severity, review};
use otd::rules::{AlignH, AlignV, Rect};
use otd::{Files, NoFiles, Normalised, RunResult, run_text, run_value};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

struct DirFiles(PathBuf);

impl Files for DirFiles {
    fn read(&self, relative_path: &str) -> Option<String> {
        fs::read_to_string(self.0.join(relative_path)).ok()
    }
    fn read_bytes(&self, relative_path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(relative_path)).ok()
    }
}

/// A one-viewport A3 document over a 100 m x 70 m window at 1:500, with these entities.
fn sheet(entities: Value) -> Normalised {
    let doc = json!({
        "standard_version": "1.0",
        "sheet": {"size": "A3", "orientation": "landscape"},
        "layers": [
            {"name": "NOTES", "role": "thin", "line_type": "continuous", "aci": 30},
            {"name": "AREA", "role": "thin", "line_type": "continuous", "aci": 31}
        ],
        "viewports": [{
            "id": "plan",
            "frame": {"x_mm": 20, "y_mm": 10, "width_mm": 300, "height_mm": 200},
            "window": {"xmin_m": 0, "ymin_m": 0, "xmax_m": 100, "ymax_m": 70},
            "scale": 500,
            "entities": entities
        }]
    });
    match run_value(&doc, &NoFiles) {
        RunResult::Ok(n) => *n,
        RunResult::Errors(e) => panic!("{e:?}"),
    }
}

fn text(id: &str, at: [f64; 2], words: &str) -> Value {
    json!({"type": "text", "layer": "NOTES", "id": id, "anchor_m": at, "text": words, "height_mm": 3.5})
}

#[test]
fn every_sheet_of_the_suite_passes_with_no_finding() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("standard/conformance/documents");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    let mut sheets = 0;
    for dir in dirs {
        let input = fs::read_to_string(dir.join("input.json")).unwrap();
        if let RunResult::Ok(norm) = run_text(&input, &DirFiles(dir.clone())) {
            sheets += 1;
            let findings = review(&norm, &ReviewOptions::default());
            assert!(findings.is_empty(), "{}: {findings:?}", dir.display());
        }
    }
    assert!(sheets > 90, "{sheets} sheets");
}

#[test]
fn overlapping_texts_are_an_error_and_apart_they_pass() {
    let apart = sheet(json!([
        text("a", [10.0, 40.0], "PUMP HOUSE"),
        text("b", [10.0, 30.0], "INTAKE")
    ]));
    assert!(review(&apart, &ReviewOptions::default()).is_empty());
    let on_top = sheet(json!([
        text("a", [10.0, 40.0], "PUMP HOUSE"),
        text("b", [12.0, 40.5], "INTAKE")
    ]));
    let findings = review(&on_top, &ReviewOptions::default());
    assert_eq!(findings.len(), 1, "{findings:?}");
    let f = &findings[0];
    assert_eq!((f.rule, f.severity), ("text-overlap", Severity::Error));
    assert_eq!(f.viewport.as_deref(), Some("plan"));
    assert_eq!(f.ids, ["a", "b"]);
    assert!(f.message.contains("PUMP HOUSE") && f.message.contains("INTAKE"));
    // texts that only touch do not overlap
    let touching = sheet(json!([
        text("a", [10.0, 40.0], "AB"),
        text("b", [10.0, 38.25], "AB")
    ]));
    assert!(review(&touching, &ReviewOptions::default()).is_empty());
}

#[test]
fn text_too_small_at_the_print_scale_is_an_error() {
    let s = sheet(json!([text("a", [10.0, 40.0], "NOTE")]));
    assert!(review(&s, &ReviewOptions::default()).is_empty());
    let on_a = |options: &ReviewOptions| {
        review(&s, options)
            .into_iter()
            .filter(|f| f.ids == ["a"])
            .map(|f| (f.rule, f.severity))
            .collect::<Vec<_>>()
    };
    // printed at half size, the 3.5 mm note is 1.75 mm (and the 2.5 mm zone labels 1.25 mm)
    let half = ReviewOptions {
        print_scale: 0.5,
        ..ReviewOptions::default()
    };
    assert_eq!(on_a(&half), [("text-print-size", Severity::Error)]);
    assert!(
        review(&s, &half).len() > 1,
        "the furniture text is too small too"
    );
    let at_limit = ReviewOptions {
        print_scale: 1.8 / 3.5,
        ..ReviewOptions::default()
    };
    assert!(on_a(&at_limit).is_empty(), "exactly 1.8 mm passes");
}

#[test]
fn text_on_a_hatch_is_a_warning_and_beside_it_passes() {
    let area = json!({"type": "hatch", "layer": "AREA", "id": "pond", "pattern": "solid",
        "boundary_m": [[40, 10], [90, 10], [90, 60], [40, 60]]});
    let beside = sheet(json!([area.clone(), text("a", [5.0, 40.0], "POND")]));
    assert!(review(&beside, &ReviewOptions::default()).is_empty());
    let on = sheet(json!([area, text("a", [60.0, 40.0], "POND")]));
    let findings = review(&on, &ReviewOptions::default());
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(
        (findings[0].rule, findings[0].severity),
        ("text-on-hatch", Severity::Warning)
    );
    assert_eq!(findings[0].ids, ["a", "pond"]);
}

#[test]
fn an_entity_outside_its_frame_is_an_error() {
    // The pipeline refuses such a document (clause 7.7.1), so the sheet is built by hand: the review
    // reads any normalised content, whatever made it.
    let frame = Rect {
        x: 20.0,
        y: 10.0,
        w: 100.0,
        h: 80.0,
    };
    let make = |to: [f64; 2]| Normalised {
        standard_version: "1.1".into(),
        sheet: SheetInfo {
            size: "A4".into(),
            orientation: "landscape".into(),
            line_group: "0.35".into(),
            width_mm: 297.0,
            height_mm: 210.0,
            drawing_space: Rect {
                x: 20.0,
                y: 10.0,
                w: 267.0,
                h: 190.0,
            },
            colour: false,
        },
        viewports: vec![ViewportInfo {
            id: "v.1".into(),
            scale: 100,
            frame,
        }],
        entities: vec![
            Entity::line(
                "viewport.v.1",
                "OUTLINE",
                [30.0, 20.0],
                to,
                "continuous",
                0.35,
                Some("edge"),
            ),
            Entity::text(
                "sheet.notes",
                "TITLEBLOCK",
                [200.0, 150.0],
                "OUTSIDE ANY FRAME",
                3.5,
                0.0,
                AlignH::Left,
                AlignV::Baseline,
                None,
            ),
        ],
        images: Default::default(),
    };
    assert!(review(&make([110.0, 80.0]), &ReviewOptions::default()).is_empty());
    let findings = review(&make([130.0, 80.0]), &ReviewOptions::default());
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(
        (findings[0].rule, findings[0].severity),
        ("outside-frame", Severity::Error)
    );
    assert_eq!(
        findings[0].viewport.as_deref(),
        Some("v.1"),
        "an id with a dot"
    );
    assert_eq!(findings[0].ids, ["edge"]);
}

#[test]
fn errors_come_before_warnings() {
    let area = json!({"type": "hatch", "layer": "AREA", "pattern": "solid",
        "boundary_m": [[40, 10], [90, 10], [90, 60], [40, 60]]});
    let s = sheet(json!([
        area,
        text("a", [60.0, 40.0], "POND"),
        text("b", [10.0, 40.0], "PUMP HOUSE"),
        text("c", [12.0, 40.5], "INTAKE")
    ]));
    let severities: Vec<Severity> = review(&s, &ReviewOptions::default())
        .iter()
        .map(|f| f.severity)
        .collect();
    assert_eq!(severities, [Severity::Error, Severity::Warning]);
}
