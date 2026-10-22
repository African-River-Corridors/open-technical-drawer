//! The pure rules of the standard: sheet, scale, scale bar, north, widths and text.
//!
//! Each function names the clause it follows. They hold no state.

use crate::data;
use crate::numbers::{FIT_TOL_MM, cos_sin, mm, round_half_even, round_to_units};

/// Sheet sizes: short edge and long edge in millimetres (clause 6.1).
pub const SIZES_MM: [(&str, f64, f64); 5] = [
    ("A0", 841.0, 1189.0),
    ("A1", 594.0, 841.0),
    ("A2", 420.0, 594.0),
    ("A3", 297.0, 420.0),
    ("A4", 210.0, 297.0),
];

/// Border insets (left, top, right, bottom) in millimetres (clause 6.3).
pub fn insets_mm(size: &str) -> Option<(f64, f64, f64, f64)> {
    match size {
        "A0" | "A1" => Some((20.0, 20.0, 20.0, 20.0)),
        "A2" | "A3" | "A4" => Some((20.0, 10.0, 10.0, 10.0)),
        _ => None,
    }
}

/// Zones along the long side and the short side (clause 6.5).
pub fn zones(size: &str) -> Option<(usize, usize)> {
    match size {
        "A0" => Some((24, 16)),
        "A1" => Some((16, 12)),
        "A2" => Some((12, 8)),
        "A3" => Some((8, 6)),
        "A4" => Some((6, 4)),
        _ => None,
    }
}

/// The number of zones along the vertical sides (clause 6.5.1).
pub fn vertical_zones(size: &str, landscape: bool) -> Option<usize> {
    let (long, short) = zones(size)?;
    Some(if landscape { short } else { long })
}

/// The letters of the vertical zones (clause 6.5.2).
pub const ZONE_LETTERS: &str = "ABCDEFGHJKLMNPQR";

/// The preferred ratios (clause 7.3.1).
pub const ISO_RATIOS: [u32; 16] = [
    1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000,
];
/// The house ratios (clause 7.3.4).
pub const HOUSE_RATIOS: [u32; 4] = [25, 250, 1250, 2500];

/// Both lists together, smallest denominator first (clause 7.5.2).
pub fn all_ratios() -> Vec<u32> {
    let mut all: Vec<u32> = ISO_RATIOS
        .iter()
        .chain(HOUSE_RATIOS.iter())
        .copied()
        .collect();
    all.sort_unstable();
    all
}

/// The map ratios (clause 7.3.5, from version 1.1).
pub const MAP_RATIOS: [u32; 6] = [200000, 500000, 1000000, 2000000, 5000000, 10000000];

/// The ratios a document of version `1.<minor>` may use, smallest denominator first (clauses 7.3.5
/// and 18.2.1). `fit` tries them in this order.
pub fn ratios(minor: u32) -> Vec<u32> {
    let mut all = all_ratios();
    if minor >= 1 {
        all.extend(MAP_RATIOS);
    }
    all
}

pub const BAR_INSET: f64 = 6.0;
pub const BAR_HEIGHT: f64 = 3.0;
pub const TICK_LENGTH: f64 = 1.5;
pub const BAR_LABEL_BASELINE: f64 = 5.0;
pub const BAR_RATIO_BASELINE: f64 = 9.0;
pub const BAR_TEXT_HEIGHT: f64 = 2.5;
pub const NORTH_SIZE: f64 = 18.0;
pub const NORTH_INSET: f64 = 10.0;
pub const NORTH_LABEL_HEIGHT: f64 = 3.5;
pub const MIN_BAR_FRAME_HEIGHT: f64 = 46.0;
pub const MIN_NORTH_FRAME: f64 = 38.0;
/// The vertical scale bar of an exaggerated viewport (clauses 7.9.11 and 7.9.12, from version 1.2):
/// its bottom end above the bottom of the frame, its divisions, the gap from the bar to its labels,
/// the gap from its top to the ratio text, and the baseline of the words above the scale bar.
pub const VBAR_BOTTOM: f64 = 26.0;
pub const VBAR_DIVISIONS: u32 = 5;
pub const VBAR_LABEL_GAP: f64 = 2.5;
pub const VBAR_RATIO_GAP: f64 = 3.0;
pub const WORDS_BASELINE: f64 = 13.0;
/// The lowest frame of an exaggerated viewport (clause 7.9.12).
pub const MIN_EXAGGERATED_FRAME_HEIGHT: f64 = 100.0;

/// The vertical denominator `N / E` (clause 7.3.6.2), or `None` when it is not a whole number in
/// `ratios`.
pub fn vertical_denominator(denominator: u32, exaggeration: u32, ratios: &[u32]) -> Option<u32> {
    if exaggeration == 0 || !denominator.is_multiple_of(exaggeration) {
        return None;
    }
    let nv = denominator / exaggeration;
    ratios.contains(&nv).then_some(nv)
}

/// A rectangle on the paper: top-left corner, width and height, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// A window of model space, in metres (clause 7.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    pub xmin: f64,
    pub ymin: f64,
    pub xmax: f64,
    pub ymax: f64,
}

/// Sheet width, height and drawing space (clause 6.3).
pub fn drawing_space(size: &str, landscape: bool) -> Option<(f64, f64, Rect)> {
    let (_, short, long) = SIZES_MM.iter().find(|(n, _, _)| *n == size)?;
    let (w, h) = if landscape {
        (*long, *short)
    } else {
        (*short, *long)
    };
    let (l, t, r, b) = insets_mm(size)?;
    Some((
        w,
        h,
        Rect {
            x: l,
            y: t,
            w: w - l - r,
            h: h - t - b,
        },
    ))
}

/// The map from model metres to paper millimetres (clause 7.4).
#[derive(Debug, Clone, Copy)]
pub struct Mapper {
    cx: f64,
    cy: f64,
    fx: f64,
    fy: f64,
    pub k: f64,
    kv: f64,
    cos: f64,
    sin: f64,
}

impl Mapper {
    /// A map for a frame, a window, a scale denominator and a rotation in degrees.
    pub fn new(frame: Rect, window: Window, denominator: u32, rotation_deg: f64) -> Mapper {
        let (cos, sin) = cos_sin(rotation_deg);
        Mapper {
            cx: (window.xmin + window.xmax) / 2.0,
            cy: (window.ymin + window.ymax) / 2.0,
            fx: frame.x + frame.w / 2.0,
            fy: frame.y + frame.h / 2.0,
            k: 1000.0 / f64::from(denominator),
            kv: 1000.0 / f64::from(denominator),
            cos,
            sin,
        }
    }

    /// A map with a vertical exaggeration `E` (clauses 7.3.6 and 7.4): model y is drawn at
    /// `k_v = 1000 / (N / E)`. With `E = 1` it is [`Mapper::new`], bit for bit.
    pub fn exaggerated(
        frame: Rect,
        window: Window,
        denominator: u32,
        rotation_deg: f64,
        exaggeration: u32,
    ) -> Mapper {
        let mut m = Mapper::new(frame, window, denominator, rotation_deg);
        if exaggeration > 1 {
            m.kv = 1000.0 / (f64::from(denominator) / f64::from(exaggeration));
        }
        m
    }

    /// The paper millimetres per metre of model height (clause 7.3.6). It is `k` with no
    /// exaggeration.
    pub fn kv(&self) -> f64 {
        self.kv
    }

    /// The paper point of a model point. Not rounded.
    pub fn point(&self, p: [f64; 2]) -> [f64; 2] {
        let dx = p[0] - self.cx;
        let dy = p[1] - self.cy;
        let rx = dx * self.cos - dy * self.sin;
        let ry = dx * self.sin + dy * self.cos;
        [self.fx + rx * self.k, self.fy - ry * self.kv]
    }
}

/// Paper size of the bounding box of the turned window (clause 7.5.1).
pub fn required_size(window: Window, denominator: u32, rotation_deg: f64) -> (f64, f64) {
    let (c, s) = cos_sin(rotation_deg);
    let wm = window.xmax - window.xmin;
    let hm = window.ymax - window.ymin;
    let k = 1000.0 / f64::from(denominator);
    (
        ((wm * c).abs() + (hm * s).abs()) * k,
        ((wm * s).abs() + (hm * c).abs()) * k,
    )
}

/// The paper size of clause 7.5.1 with a vertical exaggeration `E`: the height is at `k_v`. With
/// `E = 1` it is [`required_size`]. A turned exaggerated window is an error of its own (clause
/// 7.3.6.3); it is sized here as the map of clause 7.4 draws it.
pub fn required_size_exaggerated(
    window: Window,
    denominator: u32,
    rotation_deg: f64,
    exaggeration: u32,
) -> (f64, f64) {
    if exaggeration <= 1 {
        return required_size(window, denominator, rotation_deg);
    }
    let (c, s) = cos_sin(rotation_deg);
    let wm = window.xmax - window.xmin;
    let hm = window.ymax - window.ymin;
    let k = 1000.0 / f64::from(denominator);
    let kv = 1000.0 / (f64::from(denominator) / f64::from(exaggeration));
    (
        (wm * c).abs() * k + (hm * s).abs() * kv,
        (wm * s).abs() * k + (hm * c).abs() * kv,
    )
}

/// Does the window fit the frame at the scale (clause 7.5.1)?
pub fn window_fits(frame: Rect, window: Window, denominator: u32, rotation_deg: f64) -> bool {
    let (rw, rh) = required_size(window, denominator, rotation_deg);
    rw <= frame.w + FIT_TOL_MM && rh <= frame.h + FIT_TOL_MM
}

/// [`window_fits`] with a vertical exaggeration `E` (clauses 7.3.6 and 7.5.1).
pub fn window_fits_exaggerated(
    frame: Rect,
    window: Window,
    denominator: u32,
    rotation_deg: f64,
    exaggeration: u32,
) -> bool {
    let (rw, rh) = required_size_exaggerated(window, denominator, rotation_deg, exaggeration);
    rw <= frame.w + FIT_TOL_MM && rh <= frame.h + FIT_TOL_MM
}

/// The first ratio, from 1:1 upward, at which the window fits (clause 7.5.2), for a document of
/// version 1.0.
pub fn fit_scale(frame: Rect, window: Window, rotation_deg: f64) -> Option<u32> {
    fit_scale_in(&all_ratios(), frame, window, rotation_deg)
}

/// The first ratio of `ratios` at which the window fits (clause 7.5.2). Give it [`ratios`] of the
/// document's minor version.
pub fn fit_scale_in(ratios: &[u32], frame: Rect, window: Window, rotation_deg: f64) -> Option<u32> {
    ratios
        .iter()
        .copied()
        .find(|&n| window_fits(frame, window, n, rotation_deg))
}

/// [`fit_scale_in`] with a vertical exaggeration `E` (clause 7.5.2): only the ratios whose vertical
/// ratio `N / E` is in `ratios` are tried (clause 7.3.6.2). With `E = 1` it is [`fit_scale_in`].
pub fn fit_scale_exaggerated(
    ratios: &[u32],
    frame: Rect,
    window: Window,
    rotation_deg: f64,
    exaggeration: u32,
) -> Option<u32> {
    if exaggeration <= 1 {
        return fit_scale_in(ratios, frame, window, rotation_deg);
    }
    ratios.iter().copied().find(|&n| {
        vertical_denominator(n, exaggeration, ratios).is_some()
            && window_fits_exaggerated(frame, window, n, rotation_deg, exaggeration)
    })
}

/// The length of the vertical scale bar (clause 7.9.11): the largest `{1, 2, 5} x 10^n` metres, n
/// from -3 to 6, that is at most a quarter of the frame height on paper at the vertical scale.
pub fn vertical_bar_length(frame_height: f64, vertical_denominator: u32) -> Option<f64> {
    let kv = 1000.0 / f64::from(vertical_denominator);
    let target = (frame_height / 4.0) / kv;
    let mut best: Option<f64> = None;
    for exponent in -3..=6 {
        for multiple in [1.0, 2.0, 5.0] {
            let value = multiple * libm::pow(10.0, f64::from(exponent));
            if value <= target + 1e-12 {
                best = Some(best.map_or(value, |b| b.max(value)));
            }
        }
    }
    best
}

/// The words of a vertical exaggeration under the scale bar (clause 7.9.11), never initials.
pub fn exaggeration_words(exaggeration: u32) -> String {
    label!(
        "draw-vertical-exaggeration",
        "Vertical exaggeration \u{00d7}{e}",
        e = exaggeration
    )
}

/// The available width of a scale bar (clause 7.9.3).
pub fn bar_available(frame_width: f64) -> f64 {
    frame_width - 2.0 * BAR_INSET
}

/// The largest `{1, 2, 5} x 10^n` metres bar that is at most a quarter of the frame and fits (clause 7.9.4).
pub fn derived_bar_length(frame_width: f64, denominator: u32) -> Option<f64> {
    let k = 1000.0 / f64::from(denominator);
    let target = (frame_width / 4.0) / k;
    let mut best: Option<f64> = None;
    for exponent in -3..=6 {
        for multiple in [1.0, 2.0, 5.0] {
            let value = multiple * libm::pow(10.0, f64::from(exponent));
            if value <= target + 1e-12 && value * k <= bar_available(frame_width) + FIT_TOL_MM {
                best = Some(best.map_or(value, |b| b.max(value)));
            }
        }
    }
    best
}

/// The tick labels of a scale bar (clause 7.9.7).
pub fn bar_labels(length_m: f64, divisions: u32, unit: &str) -> Vec<String> {
    (0..=divisions)
        .map(|i| {
            let mut value = length_m * f64::from(i) / f64::from(divisions);
            if unit == "km" {
                value /= 1000.0;
            }
            let rounded = round_half_even(value, 6);
            let mut text = if rounded.fract() == 0.0 {
                format!("{}", rounded as i64)
            } else {
                format!("{rounded:.6}")
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            };
            if i == divisions {
                text.push(' ');
                text.push_str(unit);
            }
            text
        })
        .collect()
}

/// The paper direction of north, clockwise from up, in `[0, 360)` (clause 7.10.2). Not rounded.
pub fn north_deg(rotation_deg: f64, bearing_deg: f64) -> f64 {
    (-(rotation_deg.rem_euclid(360.0)) - bearing_deg).rem_euclid(360.0)
}

/// Snap a width to the nearest CAD width, ties to the thinner (clause 8.3.2). `None` is error 8.3.2.
pub fn snap_width(width_mm: f64) -> Option<f64> {
    let um = round_to_units(width_mm, 3)?;
    if um == 0 || um > 2110 {
        return None;
    }
    let widths = &data::line_styles().cad_widths_um;
    let best = widths
        .iter()
        .copied()
        .min_by_key(|&m| ((m - um).abs(), m))?;
    Some(best as f64 / 1000.0)
}

/// Decimal digits and a power of ten of the shortest text of a finite number.
fn decimal_parts(x: f64) -> (i128, i32) {
    let text = format!("{x:e}");
    let (mantissa, exp) = text.split_once('e').unwrap_or((&text, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let frac_len = mantissa.split_once('.').map_or(0, |(_, f)| f.len()) as i32;
    let value: i128 = digits.parse().unwrap_or(0);
    (if negative { -value } else { value }, exp - frac_len)
}

/// The product of two numbers worked in decimal from their shortest texts, then to binary.
/// Python's `float(Decimal(str(a)) * Decimal(str(b)))`: it gives 1.8 for 10 times 0.18.
pub fn decimal_product(a: f64, b: f64) -> f64 {
    let (ma, ea) = decimal_parts(a);
    let (mb, eb) = decimal_parts(b);
    format!("{}e{}", ma * mb, ea + eb)
        .parse::<f64>()
        .unwrap_or(a * b)
}

/// Dashes and gaps of a line type for a line width, in millimetres (clause 8.4.1).
pub fn line_pattern(line_type: &str, width_mm: f64) -> Option<Vec<f64>> {
    let pattern = data::line_styles().pattern(line_type)?;
    Some(
        pattern
            .iter()
            .map(|&m| mm(decimal_product(m, width_mm)))
            .collect(),
    )
}

/// The counter-clockwise sweep in `[0, 360)`; `None` for a sweep of 0 (clause 9.5.1).
pub fn arc_sweep(start_deg: f64, end_deg: f64) -> Option<f64> {
    let sweep = (end_deg - start_deg).rem_euclid(360.0);
    if sweep == 0.0 { None } else { Some(sweep) }
}

/// The sum of the advances of a text, in font units; `None` if a character has no advance (clause 11.6.2).
pub fn text_advance(text: &str) -> Option<u32> {
    let table = &data::font_widths().advance;
    text.chars().map(|c| table.get(&c).copied()).sum()
}

/// The exact width of a text, in millimetres, not rounded (clause 11.7.1).
pub fn text_width_exact(text: &str, height_mm: f64) -> Option<f64> {
    let cap = data::font_widths().cap_height_units;
    Some(f64::from(text_advance(text)?) * height_mm / cap)
}

/// The width of a text, in millimetres, rounded (clause 11.7.1).
pub fn text_width(text: &str, height_mm: f64) -> Option<f64> {
    text_width_exact(text, height_mm).map(mm)
}

/// Horizontal alignment of text (clause 11.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignH {
    Left,
    Centre,
    Right,
}

impl AlignH {
    pub fn parse(word: &str) -> AlignH {
        match word {
            "centre" => AlignH::Centre,
            "right" => AlignH::Right,
            _ => AlignH::Left,
        }
    }
    pub fn word(self) -> &'static str {
        match self {
            AlignH::Left => "left",
            AlignH::Centre => "centre",
            AlignH::Right => "right",
        }
    }
}

/// Vertical alignment of text (clause 11.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignV {
    Baseline,
    Middle,
    Top,
}

impl AlignV {
    pub fn parse(word: &str) -> AlignV {
        match word {
            "middle" => AlignV::Middle,
            "top" => AlignV::Top,
            _ => AlignV::Baseline,
        }
    }
    pub fn word(self) -> &'static str {
        match self {
            AlignV::Baseline => "baseline",
            AlignV::Middle => "middle",
            AlignV::Top => "top",
        }
    }
}

/// The four corners of the text box on the paper, not rounded (clauses 11.7.2 and 11.7.3).
pub fn text_box(
    anchor: [f64; 2],
    text: &str,
    height_mm: f64,
    rotation_deg: f64,
    h: AlignH,
    v: AlignV,
) -> Option<[[f64; 2]; 4]> {
    let [ax, ay] = anchor;
    let w = text_width_exact(text, height_mm)?;
    let left = match h {
        AlignH::Left => ax,
        AlignH::Centre => ax - w / 2.0,
        AlignH::Right => ax - w,
    };
    let base = match v {
        AlignV::Baseline => ay,
        AlignV::Middle => ay + height_mm / 2.0,
        AlignV::Top => ay + height_mm,
    };
    let (c, s) = cos_sin(rotation_deg);
    let corners = [
        (left, base),
        (left + w, base),
        (left + w, base - height_mm),
        (left, base - height_mm),
    ];
    Some(corners.map(|(x, y)| {
        let dx = x - ax;
        let dy = y - ay;
        [ax + dx * c + dy * s, ay - dx * s + dy * c]
    }))
}

/// Euclidean distance of two points.
pub fn hypot(a: [f64; 2], b: [f64; 2]) -> f64 {
    libm::hypot(a[0] - b[0], a[1] - b[1])
}
