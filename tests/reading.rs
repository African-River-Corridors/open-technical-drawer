//! Reading a document: bad JSON, repeated keys, and text that must not be read (clauses 4.1, 4.4, 18).

use otd::json::{ParseError, parse};
use otd::{NoFiles, RunResult, run_text};

fn clauses(text: &str) -> Vec<String> {
    match run_text(text, &NoFiles) {
        RunResult::Errors(set) => set.into_iter().collect(),
        RunResult::Ok(_) => vec![],
    }
}

const GOOD: &str = r#"{"standard_version":"1.0","sheet":{"size":"A3"},"layers":[{"name":"OUTLINE"}],
 "viewports":[{"id":"v","frame":{"x_mm":20,"y_mm":10,"width_mm":390,"height_mm":277},
 "window":{"xmin_m":0,"ymin_m":0,"xmax_m":100,"ymax_m":50},"scale":"fit","entities":[
 {"type":"line","layer":"OUTLINE","from_m":[0,0],"to_m":[100,50]}]}]}"#;

#[test]
fn a_good_document_is_read() {
    assert!(matches!(run_text(GOOD, &NoFiles), RunResult::Ok(_)));
}

#[test]
fn bad_json_is_4_1() {
    assert_eq!(clauses("{"), ["4.1"]);
    assert_eq!(clauses(""), ["4.1"]);
    assert_eq!(clauses("{\"a\": NaN}"), ["4.1"]);
    assert_eq!(clauses("{\"a\": 1e999}"), ["4.1"]);
    assert_eq!(clauses(&format!("{GOOD} x")), ["4.1"]);
}

#[test]
fn a_repeated_key_is_4_4() {
    assert_eq!(parse(r#"{"a":1,"a":2}"#), Err(ParseError::DuplicateKey));
    assert_eq!(
        clauses(r#"{"standard_version":"1.0","standard_version":"1.0"}"#),
        ["4.4"]
    );
    // a repeated key deep in the document
    let doc = GOOD.replace(r#""to_m":[100,50]"#, r#""to_m":[100,50],"to_m":[1,1]"#);
    assert_eq!(clauses(&doc), ["4.4"]);
}

#[test]
fn an_unknown_key_is_never_ignored() {
    let doc = GOOD.replace(
        r#""sheet":{"size":"A3"}"#,
        r#""sheet":{"size":"A3","colour":"red"}"#,
    );
    assert_eq!(clauses(&doc), ["6.1"]);
    assert_eq!(clauses(r#"{"nonsense":1}"#), ["4.2"]);
}

#[test]
fn a_root_that_is_not_an_object_is_an_error_not_a_panic() {
    for text in ["[]", "1", "\"x\"", "null", "true"] {
        assert_eq!(clauses(text), ["4.2"], "{text}");
    }
}

#[test]
fn text_with_a_line_break_is_not_read() {
    let doc = GOOD.replace(r#""entities":["#, r#""entities":[{"type":"text","layer":"OUTLINE","anchor_m":[1,1],"text":"a\nb","height_mm":3.5},"#);
    assert_eq!(clauses(&doc), ["11.3.3"]);
    // a trailing line break too: the pattern has no "end before a final newline" rule.
    let doc = GOOD.replace(r#""entities":["#, r#""entities":[{"type":"text","layer":"OUTLINE","anchor_m":[1,1],"text":"ab\n","height_mm":3.5},"#);
    assert_eq!(clauses(&doc), ["11.3.3"]);
}

#[test]
fn string_escapes_and_surrogates_are_read() {
    let v = parse(r#""a°😀\n""#).expect("valid");
    assert_eq!(v.as_str(), Some("a\u{b0}\u{1f600}\n"));
    assert_eq!(parse(r#""\ud83d""#), Err(ParseError::Syntax));
}
