//! The data files of the standard, embedded in the crate (clause 17.3 lists the files a runner may read).

use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

const LINE_STYLES: &str = include_str!("../standard/data/line-styles.json");
const FONT_WIDTHS: &str = include_str!("../standard/data/font-widths.json");
const HATCH_PATTERNS: &str = include_str!("../standard/data/hatch-patterns.json");
const TITLE_BLOCK_ISO7200: &str = include_str!("../standard/data/title-block-iso7200.json");
const DOCUMENT_SCHEMA: &str = include_str!("../standard/schema/drawing-document-1.0.schema.json");
const LIBRARY_SCHEMA: &str = include_str!("../standard/schema/symbol-library-1.0.schema.json");
const TEMPLATE_SCHEMA: &str =
    include_str!("../standard/schema/title-block-template-1.0.schema.json");

/// The font file of clause 11.6.1 (SIL Open Font License 1.1; the licence is shipped beside it).
pub const FONT_FILE: &[u8] = include_bytes!("../standard/fonts/BarlowSemiCondensed-Regular.ttf");

/// A line role (clause 8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    ExtraThick,
    Thick,
    Thin,
}

impl Role {
    /// The role named by its schema word.
    pub fn parse(word: &str) -> Option<Role> {
        match word {
            "extra_thick" => Some(Role::ExtraThick),
            "thick" => Some(Role::Thick),
            "thin" => Some(Role::Thin),
            _ => None,
        }
    }
}

/// A line group: three widths in paper millimetres (clause 8.2).
#[derive(Debug, Clone, PartialEq)]
pub struct LineGroup {
    pub name: String,
    pub sizes: Vec<String>,
    pub thin: f64,
    pub thick: f64,
    pub extra_thick: f64,
}

impl LineGroup {
    /// The width of a role in this group.
    pub fn width(&self, role: Role) -> f64 {
        match role {
            Role::ExtraThick => self.extra_thick,
            Role::Thick => self.thick,
            Role::Thin => self.thin,
        }
    }
}

/// A standard layer (clause 8.5.2).
#[derive(Debug, Clone, PartialEq)]
pub struct LayerStyle {
    pub name: String,
    pub role: Role,
    pub line_type: String,
    pub aci: i64,
}

/// `line-styles.json`: clause 8.
#[derive(Debug)]
pub struct LineStyles {
    pub groups: Vec<LineGroup>,
    pub cad_widths_um: Vec<i64>,
    /// Line type identifier to its pattern, in units of the line width (clause 8.4).
    pub line_types: Vec<(String, Vec<f64>)>,
    pub layers: Vec<LayerStyle>,
}

impl LineStyles {
    /// The group of a sheet size (clause 8.2).
    pub fn group_for(&self, size: &str) -> Option<&LineGroup> {
        self.groups
            .iter()
            .find(|g| g.sizes.iter().any(|s| s == size))
    }

    /// A standard layer by name.
    pub fn layer(&self, name: &str) -> Option<&LayerStyle> {
        self.layers.iter().find(|l| l.name == name)
    }

    /// The pattern of a line type, in units of the line width.
    pub fn pattern(&self, line_type: &str) -> Option<&[f64]> {
        self.line_types
            .iter()
            .find(|(n, _)| n == line_type)
            .map(|(_, p)| p.as_slice())
    }
}

fn load(text: &str) -> Value {
    crate::json::parse(text).expect("an embedded data file of the standard is valid JSON")
}

fn f(v: &Value, key: &str) -> f64 {
    v[key].as_f64().expect("a number in an embedded data file")
}

fn s(v: &Value, key: &str) -> String {
    v[key]
        .as_str()
        .expect("a string in an embedded data file")
        .to_string()
}

/// The line styles of clause 8.
pub fn line_styles() -> &'static LineStyles {
    static CELL: OnceLock<LineStyles> = OnceLock::new();
    CELL.get_or_init(|| {
        let v = load(LINE_STYLES);
        let groups = v["line_groups"]
            .as_object()
            .expect("line_groups")
            .iter()
            .map(|(name, g)| LineGroup {
                name: name.clone(),
                sizes: g["sizes"]
                    .as_array()
                    .expect("sizes")
                    .iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect(),
                thin: f(g, "thin"),
                thick: f(g, "thick"),
                extra_thick: f(g, "extra_thick"),
            })
            .collect();
        let cad_widths_um = v["cad_line_widths_um"]
            .as_array()
            .expect("widths")
            .iter()
            .filter_map(Value::as_i64)
            .collect();
        let line_types = v["line_types"]
            .as_object()
            .expect("line_types")
            .iter()
            .map(|(n, t)| {
                (
                    n.clone(),
                    t["pattern_d"]
                        .as_array()
                        .expect("pattern_d")
                        .iter()
                        .filter_map(Value::as_f64)
                        .collect(),
                )
            })
            .collect();
        let layers = v["layers"]
            .as_array()
            .expect("layers")
            .iter()
            .map(|l| LayerStyle {
                name: s(l, "name"),
                role: Role::parse(l["role"].as_str().unwrap_or("")).expect("role"),
                line_type: s(l, "line_type"),
                aci: l["aci"].as_i64().expect("aci"),
            })
            .collect();
        LineStyles {
            groups,
            cad_widths_um,
            line_types,
            layers,
        }
    })
}

/// `font-widths.json`: the advance of each character, in font units (clause 11.7.1).
#[derive(Debug)]
pub struct FontWidths {
    pub cap_height_units: f64,
    pub units_per_em: f64,
    pub advance: HashMap<char, u32>,
}

/// The width table of clause 11.7.1.
pub fn font_widths() -> &'static FontWidths {
    static CELL: OnceLock<FontWidths> = OnceLock::new();
    CELL.get_or_init(|| {
        let v = load(FONT_WIDTHS);
        let advance = v["advance"]
            .as_object()
            .expect("advance")
            .iter()
            .filter_map(|(k, a)| {
                let mut chars = k.chars();
                let c = chars.next()?;
                if chars.next().is_some() {
                    None
                } else {
                    Some((c, u32::try_from(a.as_u64()?).ok()?))
                }
            })
            .collect();
        FontWidths {
            cap_height_units: f(&v, "cap_height_units"),
            units_per_em: f(&v, "units_per_em"),
            advance,
        }
    })
}

/// The data of clause 10.5 (hatch patterns), as parsed JSON.
pub fn hatch_patterns() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| load(HATCH_PATTERNS))
}

/// The shipped title block template (clause 14.3), as parsed JSON.
pub fn title_block_iso7200() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| load(TITLE_BLOCK_ISO7200))
}

/// The JSON Schema of the drawing document (clause 18.3).
pub fn document_schema() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| load(DOCUMENT_SCHEMA))
}

/// The JSON Schema of a symbol library file (clause 13.1).
pub fn library_schema() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| load(LIBRARY_SCHEMA))
}

/// The JSON Schema of a title block template (clause 14.2).
pub fn template_schema() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| load(TEMPLATE_SCHEMA))
}
