//! The conformance suite of the Drawing Format Standard (clause 17), run through this crate.
//!
//! Every case of `standard/conformance/suite.json` runs: each document through the runner of clause
//! 17.3, each unit file through the unit functions of clause 17.5. A case that this crate cannot pass
//! yet is named in `EXPECTED_TO_FAIL`, in this one place. The test fails when a case that is not on
//! the list fails, and also when a case on the list passes: the list can only shrink, and it is never
//! a way to skip a case without saying so.

use otd::conformance::{compare, run_unit, same};
use otd::{Files, run_text};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

mod support {
    pub mod pending;
}

use support::pending::EXPECTED_TO_FAIL;

struct DirFiles(PathBuf);

impl Files for DirFiles {
    fn read(&self, relative_path: &str) -> Option<String> {
        fs::read_to_string(self.0.join(relative_path)).ok()
    }
    fn read_bytes(&self, relative_path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(relative_path)).ok()
    }
}

fn suite_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("standard")
        .join("conformance")
}

fn read_json(path: &Path) -> Value {
    otd::json::parse(&fs::read_to_string(path).expect("read")).expect("valid JSON")
}

fn run_case(suite: &Path, case: &Value) -> Vec<String> {
    match case["kind"].as_str() {
        Some("document") => {
            let dir = suite.join(case["dir"].as_str().expect("dir"));
            let text = fs::read_to_string(dir.join("input.json")).expect("input");
            let expected = read_json(&dir.join("expected.json"));
            let result = run_text(&text, &DirFiles(dir)).to_value();
            compare(&result, &expected)
        }
        Some("unit") => {
            let unit = read_json(&suite.join(case["file"].as_str().expect("file")));
            let function = unit["function"].as_str().expect("function");
            let mut problems = Vec::new();
            for check in unit["checks"].as_array().expect("checks") {
                match run_unit(function, &check["input"]) {
                    Some(got) if same(&got, &check["expected"]) => {}
                    Some(got) => problems.push(format!(
                        "{}: got {got}, expected {}",
                        check["id"], check["expected"]
                    )),
                    None => problems.push(format!("{function}: not implemented")),
                }
            }
            problems
        }
        other => vec![format!("unknown kind {other:?}")],
    }
}

#[test]
fn whole_suite() {
    let suite = suite_dir();
    let list = read_json(&suite.join("suite.json"));
    let cases = list["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 217, "the pinned suite has 217 cases");
    let expected: BTreeSet<&str> = EXPECTED_TO_FAIL.iter().copied().collect();
    let mut failing = BTreeSet::new();
    let mut report = Vec::new();
    for case in cases {
        let id = case["id"].as_str().expect("id");
        let problems = run_case(&suite, case);
        if !problems.is_empty() {
            failing.insert(id.to_string());
            if !expected.contains(id) {
                report.push(format!("FAIL {id}: {}", problems[0]));
            }
        }
    }
    let mut passing_but_listed: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|id| !failing.contains(*id))
        .collect();
    passing_but_listed.sort_unstable();
    assert!(
        report.is_empty(),
        "cases failed that are not on the expected-to-fail list:\n{}",
        report.join("\n")
    );
    assert!(
        passing_but_listed.is_empty(),
        "cases on the expected-to-fail list now pass; remove them: {passing_but_listed:?}"
    );
    let ids: BTreeSet<&str> = cases.iter().filter_map(|c| c["id"].as_str()).collect();
    for id in &expected {
        assert!(
            ids.contains(id),
            "{id} is on the expected-to-fail list but is not in the suite"
        );
    }
}
