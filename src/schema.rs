//! A small JSON Schema checker for the schemas of the standard (clause 18).
//!
//! It handles the keywords that the three schema files use, and nothing else. A failed rule gives
//! the `x-clause` of the schema object that holds the rule (clause 18.6), or `4.2` when that object
//! has none. A `$ref` is followed, so an error inside a referenced definition carries that
//! definition's clause.

use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

/// The clause given when a failed rule has no `x-clause` (clause 4.2: the members of the root).
pub const DEFAULT_CLAUSE: &str = "4.2";

/// One schema document.
pub struct Schema<'a> {
    root: &'a Value,
    regexes: RefCell<HashMap<String, Option<Pattern>>>,
}

impl<'a> Schema<'a> {
    /// A checker for the schema document `root`.
    pub fn new(root: &'a Value) -> Schema<'a> {
        Schema {
            root,
            regexes: RefCell::new(HashMap::new()),
        }
    }

    /// The set of clauses of every rule that the instance fails (empty when it is valid).
    pub fn clauses(&self, instance: &Value) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        self.check(self.root, instance, &mut out);
        out
    }

    /// True when the instance follows the schema.
    pub fn is_valid(&self, instance: &Value) -> bool {
        self.clauses(instance).is_empty()
    }

    fn resolve(&self, reference: &str) -> Option<&'a Value> {
        let pointer = reference.strip_prefix('#')?;
        if pointer.is_empty() {
            return Some(self.root);
        }
        let mut node = self.root;
        for part in pointer.strip_prefix('/')?.split('/') {
            let part = part.replace("~1", "/").replace("~0", "~");
            node = node.get(&part)?;
        }
        Some(node)
    }

    fn matches(&self, pattern: &str, text: &str) -> bool {
        let mut cache = self.regexes.borrow_mut();
        let compiled = cache
            .entry(pattern.to_string())
            .or_insert_with(|| Pattern::new(pattern));
        // A pattern that cannot be compiled never matches: a loud failure, not a silent pass.
        compiled.as_ref().is_some_and(|re| re.is_match(text))
    }

    fn valid(&self, schema: &Value, instance: &Value) -> bool {
        let mut scratch = BTreeSet::new();
        self.check(schema, instance, &mut scratch);
        scratch.is_empty()
    }

    fn check(&self, schema: &Value, instance: &Value, out: &mut BTreeSet<String>) {
        let Some(map) = schema.as_object() else {
            if schema == &Value::Bool(false) {
                out.insert(DEFAULT_CLAUSE.to_string());
            }
            return;
        };
        let clause = map
            .get("x-clause")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_CLAUSE);
        let fail = |out: &mut BTreeSet<String>| {
            out.insert(clause.to_string());
        };
        if let Some(target) = map.get("$ref").and_then(Value::as_str) {
            match self.resolve(target) {
                Some(t) => self.check(t, instance, out),
                None => fail(out),
            }
        }
        if let Some(t) = map.get("type") {
            let ok = match t {
                Value::String(name) => type_ok(name, instance),
                Value::Array(names) => names
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|n| type_ok(n, instance)),
                _ => true,
            };
            if !ok {
                fail(out);
            }
        }
        if let Some(Value::Array(options)) = map.get("enum")
            && !options.iter().any(|o| equal(o, instance))
        {
            fail(out);
        }
        if let Some(c) = map.get("const")
            && !equal(c, instance)
        {
            fail(out);
        }
        match instance {
            Value::Object(obj) => {
                if let Some(Value::Array(required)) = map.get("required")
                    && required
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|k| !obj.contains_key(k))
                {
                    fail(out);
                }
                if let Some(Value::Object(deps)) = map.get("dependentRequired") {
                    for (key, needed) in deps {
                        if obj.contains_key(key)
                            && needed.as_array().is_some_and(|n| {
                                n.iter()
                                    .filter_map(Value::as_str)
                                    .any(|k| !obj.contains_key(k))
                            })
                        {
                            fail(out);
                        }
                    }
                }
                if let Some(n) = map.get("minProperties").and_then(Value::as_u64)
                    && (obj.len() as u64) < n
                {
                    fail(out);
                }
                let props = map.get("properties").and_then(Value::as_object);
                if let Some(props) = props {
                    for (key, sub) in props {
                        if let Some(v) = obj.get(key) {
                            self.check(sub, v, out);
                        }
                    }
                }
                match map.get("additionalProperties") {
                    Some(Value::Bool(false)) => {
                        if obj
                            .keys()
                            .any(|k| !props.is_some_and(|p| p.contains_key(k)))
                        {
                            fail(out);
                        }
                    }
                    Some(sub @ Value::Object(_)) => {
                        for (key, v) in obj {
                            if !props.is_some_and(|p| p.contains_key(key)) {
                                self.check(sub, v, out);
                            }
                        }
                    }
                    _ => {}
                }
                if let Some(names) = map.get("propertyNames") {
                    for key in obj.keys() {
                        self.check(names, &Value::String(key.clone()), out);
                    }
                }
            }
            Value::Array(items) => {
                if let Some(n) = map.get("minItems").and_then(Value::as_u64)
                    && (items.len() as u64) < n
                {
                    fail(out);
                }
                if let Some(n) = map.get("maxItems").and_then(Value::as_u64)
                    && (items.len() as u64) > n
                {
                    fail(out);
                }
                if let Some(sub) = map.get("items") {
                    for item in items {
                        self.check(sub, item, out);
                    }
                }
            }
            Value::String(text) => {
                let length = text.chars().count() as u64;
                if let Some(n) = map.get("minLength").and_then(Value::as_u64)
                    && length < n
                {
                    fail(out);
                }
                if let Some(n) = map.get("maxLength").and_then(Value::as_u64)
                    && length > n
                {
                    fail(out);
                }
                if let Some(p) = map.get("pattern").and_then(Value::as_str)
                    && !self.matches(p, text)
                {
                    fail(out);
                }
            }
            Value::Number(n) => {
                if let Some(x) = n.as_f64() {
                    if let Some(m) = map.get("minimum").and_then(Value::as_f64)
                        && x < m
                    {
                        fail(out);
                    }
                    if let Some(m) = map.get("maximum").and_then(Value::as_f64)
                        && x > m
                    {
                        fail(out);
                    }
                    if let Some(m) = map.get("exclusiveMinimum").and_then(Value::as_f64)
                        && x <= m
                    {
                        fail(out);
                    }
                    if let Some(m) = map.get("exclusiveMaximum").and_then(Value::as_f64)
                        && x >= m
                    {
                        fail(out);
                    }
                }
            }
            _ => {}
        }
        if let Some(Value::Array(all)) = map.get("allOf") {
            for sub in all {
                self.check(sub, instance, out);
            }
        }
        if let Some(sub) = map.get("not")
            && self.valid(sub, instance)
        {
            fail(out);
        }
        if let Some(cond) = map.get("if") {
            if self.valid(cond, instance) {
                if let Some(then) = map.get("then") {
                    self.check(then, instance, out);
                }
            } else if let Some(otherwise) = map.get("else") {
                self.check(otherwise, instance, out);
            }
        }
    }
}

fn type_ok(name: &str, v: &Value) -> bool {
    match name {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        "number" => v.is_number(),
        "integer" => v
            .as_f64()
            .is_some_and(|x| v.is_number() && x.fract() == 0.0),
        _ => true,
    }
}

/// JSON equality with numbers compared as numbers (1 equals 1.0), as the schema keywords need.
pub fn equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| equal(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| equal(v, w)))
        }
        _ => a == b,
    }
}

/// A schema pattern. `regex-lite` has no look-around, so a pattern of the form `^(?!A)(?!B)REST`
/// is read as: neither `^A` nor `^B` matches, and `^REST` does. The path of an image (clause 9.10)
/// is the one pattern of this form. Any other look-around does not compile.
#[derive(Debug)]
struct Pattern {
    refused: Vec<regex_lite::Regex>,
    rest: regex_lite::Regex,
}

impl Pattern {
    fn new(pattern: &str) -> Option<Pattern> {
        let mut tail = pattern.strip_prefix('^');
        let mut refused = Vec::new();
        while let Some(t) = tail.and_then(|t| t.strip_prefix("(?!")) {
            let end = group_end(t)?;
            refused.push(regex_lite::Regex::new(&format!("^(?:{})", &t[..end])).ok()?);
            tail = Some(&t[end + 1..]);
        }
        let rest = match tail {
            Some(t) if !refused.is_empty() => format!("^{t}"),
            _ => pattern.to_string(),
        };
        Some(Pattern {
            refused,
            rest: regex_lite::Regex::new(&rest).ok()?,
        })
    }

    fn is_match(&self, text: &str) -> bool {
        !self.refused.iter().any(|r| r.is_match(text)) && self.rest.is_match(text)
    }
}

/// The index of the `)` that closes a group whose `(` came just before `text`.
fn group_end(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut escaped = false;
    for (i, c) in text.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '(' => depth += 1,
            ')' if depth == 0 => return Some(i),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod pattern_tests {
    use super::Pattern;

    #[test]
    fn leading_negative_look_aheads() {
        let p = Pattern::new(r"^(?!/)(?!.*(^|/)\.\.(/|$))[A-Za-z0-9_./-]+$").expect("compiles");
        for ok in ["backdrop.jpg", "img/a.b.jpg", "a..b.jpg", "..a/x.jpg"] {
            assert!(p.is_match(ok), "{ok}");
        }
        for bad in [
            "/abs.jpg",
            "../up.jpg",
            "a/../b.jpg",
            "a/..",
            "..",
            "sp ace.jpg",
        ] {
            assert!(!p.is_match(bad), "{bad}");
        }
    }

    #[test]
    fn other_look_around_does_not_compile() {
        assert!(Pattern::new("^a(?!b)c$").is_none());
        assert!(Pattern::new("^[a-z]+$").is_some_and(|p| p.is_match("abc")));
    }
}
