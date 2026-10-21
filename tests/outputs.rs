//! SVG and PDF made from normalised content (clause 15): the font is embedded, the output is black,
//! and two builds give the same bytes. The files are read back with readers that are not the writers.

use lopdf::{Dictionary, Document, Object};
use otd::data;
use otd::entities::{Entity, Shape};
use otd::render::{pdf::to_pdf, svg::SvgOptions, svg::to_svg};
use otd::rules::{AlignH, AlignV, Rect};
use otd::{Files, Normalised, RunResult, run_text};
use std::fs;
use std::path::{Path, PathBuf};

struct DirFiles(PathBuf);

impl Files for DirFiles {
    fn read(&self, relative_path: &str) -> Option<String> {
        fs::read_to_string(self.0.join(relative_path)).ok()
    }
    fn read_bytes(&self, relative_path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(relative_path)).ok()
    }
}

fn case(id: &str) -> Normalised {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("standard/conformance/documents")
        .join(id);
    let text = fs::read_to_string(dir.join("input.json")).expect("input");
    match run_text(&text, &DirFiles(dir)) {
        RunResult::Ok(n) => *n,
        RunResult::Errors(e) => panic!("{id}: {e:?}"),
    }
}

/// A case that has every kind of entity: text, lines, polylines, arcs, circles, solid hatches and a
/// dashed line type, with a rotated text.
const CASES: &[&str] = &[
    "tb-register-two",
    "sym-ports",
    "dim-angle",
    "hatch-concrete",
    "ent-rotated-40-25",
    "bar-and-north",
    "sheet-a0-landscape",
];

fn deref<'a>(doc: &'a Document, o: &'a Object) -> &'a Object {
    match o {
        Object::Reference(r) => doc.get_object(*r).expect("object"),
        other => other,
    }
}

fn dict<'a>(doc: &'a Document, o: &'a Object) -> &'a Dictionary {
    match deref(doc, o) {
        Object::Dictionary(d) => d,
        Object::Stream(s) => &s.dict,
        other => panic!("not a dictionary: {other:?}"),
    }
}

fn font_parts(doc: &Document) -> (&Dictionary, &Dictionary, &Dictionary) {
    let page_id = *doc.get_pages().values().next().expect("one page");
    let fonts = doc.get_page_fonts(page_id).expect("fonts");
    let type0 = *fonts.values().next().expect("a font");
    let descendants = type0
        .get(b"DescendantFonts")
        .expect("descendants")
        .as_array()
        .expect("array");
    let cid = dict(doc, &descendants[0]);
    let descriptor = dict(doc, cid.get(b"FontDescriptor").expect("descriptor"));
    (type0, cid, descriptor)
}

#[test]
fn the_pdf_embeds_the_font_of_the_standard() {
    for id in CASES {
        let pdf = to_pdf(&case(id));
        let doc = Document::load_mem(&pdf).expect("a second reader reads the PDF");
        assert_eq!(doc.get_pages().len(), 1, "{id}: one page");
        let (type0, cid, descriptor) = font_parts(&doc);
        assert_eq!(
            type0.get(b"BaseFont").unwrap().as_name().unwrap(),
            b"BarlowSemiCondensed-Regular"
        );
        assert_eq!(
            type0.get(b"Encoding").unwrap().as_name().unwrap(),
            b"Identity-H"
        );
        assert_eq!(
            cid.get(b"Subtype").unwrap().as_name().unwrap(),
            b"CIDFontType2"
        );
        let file = deref(
            &doc,
            descriptor
                .get(b"FontFile2")
                .expect("the font file is embedded"),
        );
        let Object::Stream(stream) = file else {
            panic!("FontFile2 is a stream")
        };
        let embedded = stream.decompressed_content().expect("inflate");
        assert_eq!(
            embedded,
            data::FONT_FILE,
            "{id}: the embedded font is the vendored font, byte for byte"
        );
        assert_eq!(
            otd_font_sha256(&embedded),
            "e8c7242f612b13d026e18a94b63b0fd079e91a3faf009d1b6981dbb5f16d1153"
        );
    }
}

fn otd_font_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn the_page_has_the_size_of_the_trimmed_sheet() {
    let norm = case("sheet-a3-landscape");
    let doc = Document::load_mem(&to_pdf(&norm)).unwrap();
    let page = doc
        .get_dictionary(*doc.get_pages().values().next().unwrap())
        .unwrap();
    let media: Vec<f64> = page
        .get(b"MediaBox")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_float().map(f64::from).unwrap_or(0.0))
        .collect();
    let pt = 72.0 / 25.4;
    assert!(
        (media[2] - 420.0 * pt).abs() < 0.01 && (media[3] - 297.0 * pt).abs() < 0.01,
        "{media:?}"
    );
}

#[test]
fn glyph_widths_in_the_pdf_are_the_width_table_of_the_standard() {
    let doc = Document::load_mem(&to_pdf(&case("tb-minimal"))).unwrap();
    let (_, cid, _) = font_parts(&doc);
    let w = cid.get(b"W").unwrap().as_array().unwrap();
    let face = ttf_parser::Face::parse(data::FONT_FILE, 0).unwrap();
    let mut seen = 0;
    for pair in w.chunks(2) {
        let gid = pair[0].as_i64().unwrap() as u16;
        let width = pair[1].as_array().unwrap()[0].as_float().unwrap();
        let ch = data::font_widths()
            .advance
            .iter()
            .find(|(c, _)| face.glyph_index(**c).map(|g| g.0) == Some(gid))
            .map(|(c, a)| (*c, *a))
            .expect("a glyph of the table");
        assert_eq!(f64::from(width), f64::from(ch.1), "{:?}", ch.0);
        seen += 1;
    }
    assert_eq!(seen, data::font_widths().advance.len());
}

#[test]
fn the_text_codes_decode_to_the_text_through_the_unicode_map() {
    let norm = case("tb-register-two");
    let doc = Document::load_mem(&to_pdf(&norm)).unwrap();
    let (type0, _, _) = font_parts(&doc);
    let to_unicode = deref(&doc, type0.get(b"ToUnicode").unwrap());
    let Object::Stream(cmap) = to_unicode else {
        panic!("a stream")
    };
    let cmap = String::from_utf8(cmap.decompressed_content().unwrap()).unwrap();
    let mut map = std::collections::HashMap::new();
    for line in cmap.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2
            && parts[0].starts_with('<')
            && parts[1].starts_with('<')
            && parts[0].len() == 6
        {
            let gid = u16::from_str_radix(&parts[0][1..5], 16).unwrap();
            let ch =
                char::from_u32(u32::from_str_radix(&parts[1][1..parts[1].len() - 1], 16).unwrap())
                    .unwrap();
            map.insert(gid, ch);
        }
    }
    let page_id = *doc.get_pages().values().next().unwrap();
    let content = String::from_utf8(doc.get_page_content(page_id)).unwrap();
    let mut decoded: Vec<String> = content
        .lines()
        .filter(|l| l.starts_with("BT "))
        .map(|l| {
            let hex = &l[l.find('<').unwrap() + 1..l.find('>').unwrap()];
            hex.as_bytes()
                .chunks(4)
                .map(|c| map[&u16::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap()])
                .collect()
        })
        .collect();
    let mut expected: Vec<String> = norm
        .entities
        .iter()
        .filter_map(|e| {
            if let Shape::Text { text, .. } = &e.shape {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect();
    decoded.sort();
    expected.sort();
    assert_eq!(decoded, expected);
}

#[test]
fn the_pdf_is_black_only() {
    for id in CASES {
        let doc = Document::load_mem(&to_pdf(&case(id))).unwrap();
        let page_id = *doc.get_pages().values().next().unwrap();
        let content = String::from_utf8(doc.get_page_content(page_id)).unwrap();
        for line in content.lines() {
            let ops: Vec<&str> = line.split_whitespace().collect();
            for colour_op in [
                "rg", "RG", "k", "K", "sc", "SC", "scn", "SCN", "cs", "CS", "sh",
            ] {
                assert!(
                    !ops.contains(&colour_op),
                    "{id}: colour operator {colour_op} in {line}"
                );
            }
        }
        assert!(content.contains("0 G 0 g"), "{id}");
    }
}

#[test]
fn the_svg_is_well_formed_and_black_only() {
    for id in CASES {
        let norm = case(id);
        let svg = to_svg(&norm, &SvgOptions { embed_font: false });
        let doc = roxmltree::Document::parse(&svg).expect("well-formed XML");
        let root = doc.root_element();
        assert_eq!(root.tag_name().name(), "svg");
        assert!(root.attribute("width").unwrap().ends_with("mm"));
        for node in doc.descendants().filter(roxmltree::Node::is_element) {
            for attr in ["stroke", "fill"] {
                if let Some(v) = node.attribute(attr) {
                    assert!(matches!(v, "none" | "#000"), "{id}: {attr}={v}");
                }
            }
        }
        // one element for each entity that draws, in the order of the list
        let drawn = norm
            .entities
            .iter()
            .filter(|e| !matches!(&e.shape, Shape::Hatch { pattern, .. } if pattern != "solid"))
            .count();
        let elements = doc
            .descendants()
            .filter(|n| n.attribute("data-origin").is_some())
            .count();
        assert_eq!(elements, drawn, "{id}");
    }
}

#[test]
fn the_svg_embeds_the_font_when_asked() {
    let norm = case("tb-minimal");
    let with = to_svg(&norm, &SvgOptions { embed_font: true });
    let without = to_svg(&norm, &SvgOptions { embed_font: false });
    assert!(with.contains("data:font/ttf;base64,"));
    assert!(!without.contains("base64"));
    assert!(with.len() > without.len() + 100_000);
}

#[test]
fn output_is_the_same_on_repeat_runs() {
    for id in CASES {
        let (a, b) = (case(id), case(id));
        assert_eq!(to_pdf(&a), to_pdf(&b), "{id}: pdf");
        assert_eq!(
            to_svg(&a, &SvgOptions::default()),
            to_svg(&b, &SvgOptions::default()),
            "{id}: svg"
        );
    }
}

#[test]
fn the_writers_need_only_normalised_content() {
    // No document, no schema, no files: a sheet built by hand from the entities alone.
    let norm = Normalised {
        standard_version: "1.0".into(),
        sheet: otd::pipeline::SheetInfo {
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
        viewports: vec![],
        entities: vec![
            Entity::line(
                "viewport.v",
                "OUTLINE",
                [10.0, 10.0],
                [50.0, 10.0],
                "dashed",
                0.18,
                None,
            ),
            Entity::text(
                "viewport.v",
                "TEXT",
                [30.0, 30.0],
                "Ø 12 ±1 m²",
                3.5,
                0.0,
                AlignH::Left,
                AlignV::Baseline,
                None,
            ),
        ],
        images: Default::default(),
    };
    let svg = to_svg(&norm, &SvgOptions { embed_font: false });
    assert!(svg.contains("stroke-dasharray=\"2.16 0.54\""), "{svg}");
    assert!(svg.contains("Ø 12 ±1 m²"));
    let doc = Document::load_mem(&to_pdf(&norm)).unwrap();
    let page = *doc.get_pages().values().next().unwrap();
    let content = String::from_utf8(doc.get_page_content(page)).unwrap();
    assert!(content.contains("[2.16 0.54] 0 d"), "{content}");
}

#[test]
fn an_arc_and_a_circle_are_curves_of_the_right_extent() {
    let norm = case("ent-rotated-40-25");
    let doc = Document::load_mem(&to_pdf(&norm)).unwrap();
    let page = *doc.get_pages().values().next().unwrap();
    let content = String::from_utf8(doc.get_page_content(page)).unwrap();
    assert!(content.contains(" c\n"), "curves are written");
    let curves = content.lines().filter(|l| l.ends_with(" c")).count();
    let expected: usize = norm
        .entities
        .iter()
        .map(|e| match &e.shape {
            Shape::Circle { .. } => 4,
            Shape::Arc { sweep, .. } => {
                ((sweep.to_radians() / std::f64::consts::FRAC_PI_2 - 1e-9).ceil() as usize).max(1)
            }
            _ => 0,
        })
        .sum();
    assert_eq!(curves, expected);
}

/// The colour cases of standard 1.1 (clauses 8.6.4 to 8.6.7 and 9.10).
const COLOUR_CASES: &[&str] = &["img-colour-basic", "img-plates", "img-sheet-colour"];

fn backdrop() -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("standard/conformance/documents/img-colour-basic/backdrop.jpg"),
    )
    .expect("the backdrop of the suite")
}

#[test]
fn the_pdf_holds_each_image_as_its_jpeg_bytes() {
    for id in COLOUR_CASES {
        let norm = case(id);
        let pdf = to_pdf(&norm);
        let doc = Document::load_mem(&pdf).unwrap();
        let images: Vec<&lopdf::Stream> = doc
            .objects
            .values()
            .filter_map(|o| match o {
                Object::Stream(s)
                    if s.dict.get(b"Subtype").ok() == Some(&Object::Name(b"Image".to_vec())) =>
                {
                    Some(s)
                }
                _ => None,
            })
            .collect();
        assert_eq!(images.len(), 1, "{id}: one image XObject");
        let s = images[0];
        assert_eq!(
            s.dict.get(b"Filter").unwrap(),
            &Object::Name(b"DCTDecode".to_vec())
        );
        assert_eq!(s.content, backdrop(), "{id}: the JPEG bytes as they are");
        let page_id = *doc.get_pages().values().next().unwrap();
        let content = String::from_utf8(doc.get_page_content(page_id)).unwrap();
        let first_draw = content
            .lines()
            .position(|l| l.ends_with(" Do Q"))
            .expect("the image is drawn");
        let first_other = content
            .lines()
            .position(|l| l.ends_with(" S") || l == "S" || l.ends_with("f*") || l.contains(" Tj"))
            .unwrap_or(usize::MAX);
        assert!(
            first_draw < first_other,
            "{id}: the image is drawn first (9.10.7)"
        );
    }
}

#[test]
fn the_pdf_draws_the_colour_members() {
    let norm = case("img-plates");
    let doc = Document::load_mem(&to_pdf(&norm)).unwrap();
    let page_id = *doc.get_pages().values().next().unwrap();
    let content = String::from_utf8(doc.get_page_content(page_id)).unwrap();
    assert!(content.contains("1 1 1 RG 1 1 1 rg"), "the white plates");
    assert!(content.contains("/GS0 gs"), "the image opacity: {content}");
    let resources = doc.get_page_resources(page_id).unwrap().0.unwrap();
    let states = dict(&doc, resources.get(b"ExtGState").unwrap());
    let alpha = dict(&doc, states.get(b"GS0").unwrap()).get(b"ca").unwrap();
    assert!((alpha.as_float().unwrap() - 0.8).abs() < 1e-6);
}

#[test]
fn the_svg_draws_the_colour_members_and_the_image() {
    for id in COLOUR_CASES {
        let norm = case(id);
        let svg = to_svg(&norm, &SvgOptions { embed_font: false });
        let doc = roxmltree::Document::parse(&svg).expect("well-formed XML");
        let drawn: Vec<roxmltree::Node> = doc
            .descendants()
            .filter(|n| n.attribute("data-origin").is_some())
            .collect();
        assert_eq!(drawn[0].tag_name().name(), "image", "{id}: drawn first");
        assert!(
            drawn[0]
                .attribute("href")
                .unwrap()
                .starts_with("data:image/jpeg;base64,/9j/"),
            "{id}"
        );
        // every colour in the SVG is one of a layer, a plate, or black
        for e in &norm.entities {
            if let Some(c) = &e.paint.print_colour {
                assert!(svg.contains(&format!("\"{c}\"")), "{id}: {c}");
            }
        }
    }
}

#[test]
fn a_monochrome_sheet_names_no_print_profile() {
    let norm = case("tb-minimal");
    assert!(!norm.sheet.colour);
    assert!(norm.to_value()["sheet"].get("print_profile").is_none());
    assert!(norm.entities.iter().all(|e| e.paint == Default::default()));
    let colour = case("img-plates");
    assert_eq!(colour.to_value()["sheet"]["print_profile"], "colour");
}
