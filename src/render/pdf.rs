//! PDF from normalised content (clause 15.3).
//!
//! One page, the size of the trimmed sheet, drawn in black. The font of clause 11.6 is embedded whole
//! (clause 11.6.4) as a CID-keyed TrueType font with the glyph ids as codes, so the text can be
//! selected and searched. Nothing reads a clock: two builds of one document give the same bytes.

use super::{fmt, on_circle};
use crate::Normalised;
use crate::data;
use crate::entities::{Entity, Shape};
use crate::numbers::cos_sin;
use crate::rules;
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Filter, Finish, Name, Pdf, Rect, Ref, Str};
use ttf_parser::Face;

/// PDF points in one millimetre.
const PT_PER_MM: f64 = 72.0 / 25.4;
/// The name of the font in the file.
pub const FONT_NAME: &str = "BarlowSemiCondensed-Regular";

fn n(value: f64) -> String {
    fmt(value, 4)
}

fn deflate(bytes: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(bytes, 9)
}

fn loop_ops(points: &[[f64; 2]], close: bool) -> Vec<String> {
    let mut ops = vec![format!("{} {} m", n(points[0][0]), n(points[0][1]))];
    ops.extend(
        points[1..]
            .iter()
            .map(|p| format!("{} {} l", n(p[0]), n(p[1]))),
    );
    if close {
        ops.push("h".to_string());
    }
    ops
}

fn arc_ops(centre: [f64; 2], r: f64, start_deg: f64, sweep_deg: f64) -> Vec<String> {
    let start = start_deg.to_radians();
    let sweep = sweep_deg.to_radians();
    let pieces = ((sweep / std::f64::consts::FRAC_PI_2 - 1e-9).ceil() as usize).max(1);
    let step = sweep / pieces as f64;
    let k = 4.0 / 3.0 * libm::tan(step / 4.0);
    let at = |a: f64| on_circle(centre, r, a.to_degrees());
    let tangent = |a: f64| [-libm::sin(a), -libm::cos(a)];
    let p0 = at(start);
    let mut ops = vec![format!("{} {} m", n(p0[0]), n(p0[1]))];
    for i in 0..pieces {
        let (a0, a1) = (start + i as f64 * step, start + (i + 1) as f64 * step);
        let (q0, q1) = (at(a0), at(a1));
        let (t0, t1) = (tangent(a0), tangent(a1));
        ops.push(format!(
            "{} {} {} {} {} {} c",
            n(q0[0] + k * r * t0[0]),
            n(q0[1] + k * r * t0[1]),
            n(q1[0] - k * r * t1[0]),
            n(q1[1] - k * r * t1[1]),
            n(q1[0]),
            n(q1[1])
        ));
    }
    ops
}

fn circle_ops(c: [f64; 2], r: f64) -> Vec<String> {
    let k = 0.552_284_749_830_793_6 * r;
    vec![
        format!("{} {} m", n(c[0] + r), n(c[1])),
        format!(
            "{} {} {} {} {} {} c",
            n(c[0] + r),
            n(c[1] - k),
            n(c[0] + k),
            n(c[1] - r),
            n(c[0]),
            n(c[1] - r)
        ),
        format!(
            "{} {} {} {} {} {} c",
            n(c[0] - k),
            n(c[1] - r),
            n(c[0] - r),
            n(c[1] - k),
            n(c[0] - r),
            n(c[1])
        ),
        format!(
            "{} {} {} {} {} {} c",
            n(c[0] - r),
            n(c[1] + k),
            n(c[0] - k),
            n(c[1] + r),
            n(c[0]),
            n(c[1] + r)
        ),
        format!(
            "{} {} {} {} {} {} c",
            n(c[0] + k),
            n(c[1] + r),
            n(c[0] + r),
            n(c[1] + k),
            n(c[0] + r),
            n(c[1])
        ),
        "h".to_string(),
    ]
}

/// The glyph ids of a text, as the two-byte codes of an Identity-H font, in hex.
fn hex_codes(face: &Face, text: &str) -> String {
    text.chars()
        .map(|c| format!("{:04X}", face.glyph_index(c).map_or(0, |g| g.0)))
        .collect()
}

fn stroke_ops(width: f64, line_type: &str) -> Vec<String> {
    let dash = super::dash_array(line_type, width);
    vec![
        format!("{} w", n(width)),
        if dash.is_empty() {
            "[] 0 d".to_string()
        } else {
            format!(
                "[{}] 0 d",
                dash.iter().map(|d| n(*d)).collect::<Vec<_>>().join(" ")
            )
        },
    ]
}

fn content(norm: &Normalised, face: &Face) -> Vec<u8> {
    let mut out = vec![
        format!(
            "{} 0 0 {} 0 {} cm",
            n(PT_PER_MM),
            n(-PT_PER_MM),
            n(norm.sheet.height_mm * PT_PER_MM)
        ),
        "0 G 0 g".to_string(),
        "0 J 0 j 10 M".to_string(),
    ];
    for e in &norm.entities {
        out.extend(entity_ops(e, face));
    }
    out.join("\n").into_bytes()
}

fn entity_ops(e: &Entity, face: &Face) -> Vec<String> {
    match &e.shape {
        Shape::Text {
            anchor,
            text,
            height,
            rotation,
            align_h,
            align_v,
        } => {
            let Some(corners) =
                rules::text_box(*anchor, text, *height, *rotation, *align_h, *align_v)
            else {
                return vec![];
            };
            let (c, s) = cos_sin(*rotation);
            let [x0, y0] = corners[0];
            let size = height * 1000.0 / data::font_widths().cap_height_units;
            // The page is y down, so the text matrix turns the glyphs back upright.
            vec![format!(
                "BT /F1 {} Tf {} {} {} {} {} {} Tm <{}> Tj ET",
                n(size),
                n(c),
                n(-s),
                n(-s),
                n(-c),
                n(x0),
                n(y0),
                hex_codes(face, text)
            )]
        }
        Shape::Hatch {
            boundary,
            holes,
            pattern,
            ..
        } => {
            // A pattern other than solid draws only its marks (clause 10.5.8).
            if pattern != "solid" {
                return vec![];
            }
            let mut ops: Vec<String> = std::iter::once(boundary)
                .chain(holes.iter())
                .flat_map(|l| loop_ops(l, true))
                .collect();
            ops.push("f*".to_string());
            ops
        }
        Shape::Line {
            from,
            to,
            line_type,
            width,
        } => {
            let mut ops = stroke_ops(*width, line_type);
            ops.extend([
                format!("{} {} m", n(from[0]), n(from[1])),
                format!("{} {} l", n(to[0]), n(to[1])),
                "S".to_string(),
            ]);
            ops
        }
        Shape::Polyline {
            points,
            closed,
            line_type,
            width,
        } => {
            let mut ops = stroke_ops(*width, line_type);
            ops.extend(loop_ops(points, *closed));
            ops.push("S".to_string());
            ops
        }
        Shape::Circle {
            centre,
            radius,
            line_type,
            width,
        } => {
            let mut ops = stroke_ops(*width, line_type);
            ops.extend(circle_ops(*centre, *radius));
            ops.push("S".to_string());
            ops
        }
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
            line_type,
            width,
        } => {
            let mut ops = stroke_ops(*width, line_type);
            ops.extend(arc_ops(*centre, *radius, *start, *sweep));
            ops.push("S".to_string());
            ops
        }
    }
}

/// The PDF of a sheet, made only from its normalised content.
pub fn to_pdf(norm: &Normalised) -> Vec<u8> {
    let face = Face::parse(data::FONT_FILE, 0).expect("the vendored font file is a valid font");
    let units = f64::from(face.units_per_em());
    let scale = 1000.0 / units;
    let (catalog, pages, page, contents) = (Ref::new(1), Ref::new(2), Ref::new(3), Ref::new(4));
    let (type0, cid, descriptor, font_file, to_unicode) = (
        Ref::new(5),
        Ref::new(6),
        Ref::new(7),
        Ref::new(8),
        Ref::new(9),
    );
    let mut pdf = Pdf::new();
    pdf.catalog(catalog).pages(pages);
    pdf.pages(pages).kids([page]).count(1);
    let (wpt, hpt) = (
        norm.sheet.width_mm * PT_PER_MM,
        norm.sheet.height_mm * PT_PER_MM,
    );
    {
        let mut p = pdf.page(page);
        p.media_box(Rect::new(0.0, 0.0, wpt as f32, hpt as f32))
            .parent(pages)
            .contents(contents);
        p.resources().fonts().pair(Name(b"F1"), type0);
        p.finish();
    }
    let page_bytes = deflate(&content(norm, &face));
    pdf.stream(contents, &page_bytes)
        .filter(Filter::FlateDecode);

    // The characters of clause 11.6.2, with their glyphs, in glyph order.
    let mut glyphs: Vec<(u16, char)> = data::font_widths()
        .advance
        .keys()
        .filter_map(|c| face.glyph_index(*c).map(|g| (g.0, *c)))
        .collect();
    glyphs.sort_unstable();
    let name = Name(FONT_NAME.as_bytes());
    pdf.type0_font(type0)
        .base_font(name)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(cid)
        .to_unicode(to_unicode);
    {
        let mut c = pdf.cid_font(cid);
        c.subtype(CidFontType::Type2)
            .base_font(name)
            .system_info(SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            })
            .font_descriptor(descriptor);
        c.cid_to_gid_map_predefined(Name(b"Identity"));
        c.default_width(0.0);
        let mut w = c.widths();
        for (gid, _) in &glyphs {
            let advance = face
                .glyph_hor_advance(ttf_parser::GlyphId(*gid))
                .unwrap_or(0);
            w.consecutive(*gid, [(f64::from(advance) * scale) as f32]);
        }
        w.finish();
        c.finish();
    }
    let bbox = face.global_bounding_box();
    let to_1000 = |v: i16| (f64::from(v) * scale) as f32;
    pdf.font_descriptor(descriptor)
        .name(name)
        .flags(FontFlags::NON_SYMBOLIC)
        .bbox(Rect::new(
            to_1000(bbox.x_min),
            to_1000(bbox.y_min),
            to_1000(bbox.x_max),
            to_1000(bbox.y_max),
        ))
        .italic_angle(0.0)
        .ascent(to_1000(face.ascender()))
        .descent(to_1000(face.descender()))
        .cap_height((data::font_widths().cap_height_units) as f32)
        .stem_v(80.0)
        .font_file2(font_file);
    {
        let font_bytes = deflate(data::FONT_FILE);
        let mut s = pdf.stream(font_file, &font_bytes);
        s.filter(Filter::FlateDecode);
        s.pair(Name(b"Length1"), data::FONT_FILE.len() as i32);
    }
    let mut cmap = UnicodeCmap::new(
        Name(b"Custom"),
        SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"UCS"),
            supplement: 0,
        },
    );
    for (gid, ch) in &glyphs {
        cmap.pair(*gid, *ch);
    }
    pdf.cmap(to_unicode, &cmap.finish());
    pdf.finish()
}
