//! The vendored copy of the standard is pinned: a commit, and one hash of the copy.
//! Updating the copy is a deliberate change made with `tools/vendor_standard.py`.

use otd::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

fn standard_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("standard")
}

fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read_dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            files_under(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The hash of clause "the copy": one line `<file sha256>  <relative path>` per file, sorted by path,
/// PIN.json left out.
fn copy_hash() -> (String, usize) {
    let root = standard_dir();
    let mut files = Vec::new();
    files_under(&root, &mut files);
    let mut lines: Vec<(String, String)> = files
        .iter()
        .filter(|p| p.file_name().is_some_and(|n| n != "PIN.json"))
        .map(|p| {
            let rel = p
                .strip_prefix(&root)
                .expect("under root")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            (rel, hex(&Sha256::digest(fs::read(p).expect("read"))))
        })
        .collect();
    lines.sort();
    let text: String = lines
        .iter()
        .map(|(rel, h)| format!("{h}  {rel}\n"))
        .collect();
    (hex(&Sha256::digest(text.as_bytes())), lines.len())
}

#[test]
fn vendored_copy_is_unchanged() {
    let pin = json::parse(&fs::read_to_string(standard_dir().join("PIN.json")).expect("PIN.json"))
        .expect("valid JSON");
    let (digest, count) = copy_hash();
    assert_eq!(
        pin["sha256"].as_str(),
        Some(digest.as_str()),
        "the vendored standard changed; update it with tools/vendor_standard.py and say why in the PR"
    );
    assert_eq!(pin["files"].as_u64(), Some(count as u64));
}

#[test]
fn the_pin_names_a_commit() {
    let pin = json::parse(&fs::read_to_string(standard_dir().join("PIN.json")).expect("PIN.json"))
        .expect("valid JSON");
    let commit = pin["commit"].as_str().expect("commit");
    assert_eq!(commit.len(), 40);
    assert!(commit.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(
        pin["repository"].as_str(),
        Some("African-River-Corridors/technical_drawings_for_agents")
    );
}

#[test]
fn the_embedded_data_is_the_vendored_data() {
    // include_str! reads the same files; this proves the font file is the one clause 11.6.1 names.
    let font = hex(&Sha256::digest(otd::data::FONT_FILE));
    assert_eq!(
        font,
        "e8c7242f612b13d026e18a94b63b0fd079e91a3faf009d1b6981dbb5f16d1153"
    );
}
