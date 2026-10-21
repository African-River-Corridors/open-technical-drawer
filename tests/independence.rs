//! The crate is domain-free: it depends on no crate of a tool that uses it, and its source names
//! none. A path or git dependency on a tool crate would have to be in `Cargo.toml`.

use std::fs;
use std::path::Path;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TOOL_CRATES: [&str; 2] = ["ovcd", "owd"];

/// The dependency names of every `[…dependencies]` table of the manifest.
fn dependencies(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line.trim_matches(['[', ']']).ends_with("dependencies");
            continue;
        }
        if in_deps
            && let Some((name, _)) = line.split_once('=')
            && !line.starts_with('#')
        {
            out.push(name.trim().to_string());
        }
    }
    out
}

#[test]
fn the_crate_depends_on_no_tool_crate() {
    let manifest = fs::read_to_string(Path::new(ROOT).join("Cargo.toml")).unwrap();
    let deps = dependencies(&manifest);
    let mut runtime: Vec<_> = deps.iter().map(String::as_str).collect();
    runtime.sort_unstable();
    runtime.dedup();
    assert_eq!(
        runtime,
        [
            "libm",
            "lopdf",
            "miniz_oxide",
            "pdf-writer",
            "regex-lite",
            "roxmltree",
            "serde_json",
            "sha2",
            "ttf-parser"
        ],
        "{deps:?}"
    );
    for name in deps {
        assert!(!TOOL_CRATES.iter().any(|t| name.starts_with(t)), "{name}");
    }
}

#[test]
fn no_source_file_names_a_tool_crate() {
    for dir in ["src", "tests"] {
        for path in rs_files(&Path::new(ROOT).join(dir)) {
            let text = fs::read_to_string(&path).unwrap();
            for tool in TOOL_CRATES {
                let crate_path = format!("{tool}_");
                if path.ends_with("independence.rs") {
                    continue;
                }
                assert!(
                    !text.contains(&crate_path),
                    "{} names {crate_path}",
                    path.display()
                );
            }
        }
    }
}

fn rs_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(rs_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}
