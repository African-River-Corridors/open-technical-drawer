//! Sheets and furniture found by comparing this crate with the Python tool on random documents.

use otd::{NoFiles, RunResult, run_text};

fn doc(sheet: &str, extra: &str) -> String {
    format!(
        r#"{{"standard_version":"1.0","sheet":{sheet},"layers":[{{"name":"OUTLINE"}}],{extra}
 "viewports":[{{"id":"v","frame":{{"x_mm":30,"y_mm":30,"width_mm":300,"height_mm":300}},
 "window":{{"xmin_m":0,"ymin_m":0,"xmax_m":100,"ymax_m":50}},"scale":"fit","entities":[]}}]}}"#
    )
}

fn clauses(text: &str) -> Vec<String> {
    match run_text(text, &NoFiles) {
        RunResult::Errors(set) => set.into_iter().collect(),
        RunResult::Ok(_) => vec![],
    }
}

#[test]
fn an_a0_sheet_in_portrait_is_refused_not_a_panic() {
    // clause 6.5 gives 24 vertical zones here, and clause 6.5.2 has 16 letters
    assert_eq!(
        clauses(&doc(r#"{"size":"A0","orientation":"portrait"}"#, "")),
        ["6.5.2"]
    );
}

#[test]
fn every_other_sheet_and_orientation_is_read() {
    for size in ["A0", "A1", "A2", "A3", "A4"] {
        for orientation in ["landscape", "portrait"] {
            if size == "A0" && orientation == "portrait" {
                continue;
            }
            let text = format!(r#"{{"size":"{size}","orientation":"{orientation}"}}"#);
            let frame_ok = match run_text(&doc(&text, ""), &NoFiles) {
                RunResult::Ok(_) => true,
                RunResult::Errors(set) => !set.contains("6.5.2"),
            };
            assert!(frame_ok, "{size} {orientation}");
        }
    }
}

#[test]
fn a_whole_number_is_written_the_same_whatever_its_spelling() {
    let block = |n: &str| {
        format!(
            r#""title_block":{{"template":"iso7200","fields":{{"owner":"Example","title":"T","document_number":"D","sheet_number":{n},"sheet_count":2}}}},"#
        )
    };
    let a = run_text(&doc(r#"{"size":"A3"}"#, &block("1")), &NoFiles);
    let b = run_text(&doc(r#"{"size":"A3"}"#, &block("1.0")), &NoFiles);
    assert!(
        matches!(a, RunResult::Errors(_)),
        "the frame overlaps the title block, so both are errors"
    );
    assert_eq!(a, b);
}
