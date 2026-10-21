//! A reader never panics: every conformance input, changed in small ways, gives a result or errors.

use otd::{Files, run_value};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

struct DirFiles(PathBuf);

impl Files for DirFiles {
    fn read(&self, relative_path: &str) -> Option<String> {
        fs::read_to_string(self.0.join(relative_path)).ok()
    }
}

/// A small deterministic generator (no clock, no randomness from outside).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn paths(v: &Value, here: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match v {
        Value::Object(m) => {
            for (k, c) in m {
                here.push(k.clone());
                out.push(here.clone());
                paths(c, here, out);
                here.pop();
            }
        }
        Value::Array(a) => {
            for (i, c) in a.iter().enumerate() {
                here.push(i.to_string());
                out.push(here.clone());
                paths(c, here, out);
                here.pop();
            }
        }
        _ => {}
    }
}

fn at<'a>(v: &'a mut Value, path: &[String]) -> Option<&'a mut Value> {
    let mut cur = v;
    for key in path {
        cur = match cur {
            Value::Object(m) => m.get_mut(key)?,
            Value::Array(a) => a.get_mut(key.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}

fn mutate(doc: &mut Value, rng: &mut Lcg) {
    let mut all = Vec::new();
    paths(doc, &mut Vec::new(), &mut all);
    if all.is_empty() {
        return;
    }
    let path = all[rng.below(all.len())].clone();
    let (parent, last) = path.split_at(path.len() - 1);
    let Some(parent) = at(doc, parent) else {
        return;
    };
    let choice = rng.below(6);
    match parent {
        Value::Object(m) => {
            let key = &last[0];
            match choice {
                0 => {
                    m.remove(key);
                }
                1 => {
                    m.insert(key.clone(), Value::Null);
                }
                2 => {
                    m.insert(key.clone(), Value::from(0));
                }
                3 => {
                    m.insert(key.clone(), Value::String(String::new()));
                }
                4 => {
                    m.insert(key.clone(), Value::Array(vec![]));
                }
                _ => {
                    if let Some(x) = m.get(key).and_then(Value::as_f64) {
                        m.insert(key.clone(), serde_json::json!(x * -3.5 + 0.25));
                    } else {
                        m.insert(key.clone(), Value::Object(Map::new()));
                    }
                }
            }
        }
        Value::Array(a) => {
            if let Ok(i) = last[0].parse::<usize>()
                && i < a.len()
            {
                match choice {
                    0 => {
                        a.remove(i);
                    }
                    1 => a.push(a[i].clone()),
                    _ => a[i] = Value::Null,
                }
            }
        }
        _ => {}
    }
}

#[test]
fn changed_conformance_inputs_never_panic() {
    let suite = Path::new(env!("CARGO_MANIFEST_DIR")).join("standard/conformance/documents");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&suite)
        .expect("documents")
        .map(|e| e.expect("entry").path())
        .collect();
    dirs.sort();
    let mut rng = Lcg(7);
    let mut runs = 0;
    for dir in dirs {
        let original =
            otd::json::parse(&fs::read_to_string(dir.join("input.json")).expect("input"))
                .expect("valid JSON");
        let files = DirFiles(dir.clone());
        for _ in 0..25 {
            let mut doc = original.clone();
            for _ in 0..=rng.below(3) {
                mutate(&mut doc, &mut rng);
            }
            let _ = run_value(&doc, &files);
            runs += 1;
        }
    }
    assert!(runs > 4000);
}
