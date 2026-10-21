//! Title block, revision register, status line, notes and references (clause 14).

use crate::data::{self, LineGroup, Role};
use crate::entities::Entity;
use crate::numbers::FIT_TOL_MM;
use crate::pipeline::{FURNITURE_LAYER, Files, overlap};
use crate::read::{arr, f, p, st};
use crate::rules::{self, AlignH, AlignV, Rect};
use crate::schema::Schema;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const STATUS_HEIGHT: f64 = 2.5;
const STATUS_PAD: f64 = 2.0;
const STATUS_REGION_HEIGHT: f64 = 6.5;
const STRIP_PAD: f64 = 2.0;
const STRIP_HEADING: f64 = 3.5;
const STRIP_ITEM: f64 = 2.5;
const STRIP_PITCH: f64 = 4.5;
const GIVEN: [&str; 7] = [
    "owner",
    "title",
    "supplementary_title",
    "document_number",
    "document_type",
    "language",
    "department",
];

/// All the furniture of clause 14: entities, reserved regions (clause 6.6) and errors.
#[derive(Debug, Default)]
pub struct Plan {
    pub entities: Vec<Entity>,
    pub errors: BTreeSet<String>,
    pub regions: Vec<(String, Rect)>,
}

/// Clause 14.9: the text of the status line. `issued` has none.
fn status_text(status: &str) -> Option<&'static str> {
    match status {
        "draft" => Some(label!(
            "draw-status-draft",
            "DRAFT \u{2013} NOT FOR CONSTRUCTION"
        )),
        "concept" => Some(label!(
            "draw-status-concept",
            "CONCEPT \u{2013} NOT FOR CONSTRUCTION"
        )),
        _ => None,
    }
}

fn fits(text: &str, height: f64, width: f64) -> bool {
    rules::text_width(text, height).is_some_and(|w| w <= width + FIT_TOL_MM)
}

fn num(v: &Value, key: &str) -> f64 {
    f(&v[key])
}

// ---- template (clause 14.2) -------------------------------------------------------------------

/// The template of a title block (clause 14.2), or error 14.2.
fn load_template(block: &Value, files: &dyn Files) -> Result<Value, ()> {
    let data = if let Some(name) = block.get("template") {
        if st(name) != "iso7200" {
            return Err(());
        }
        data::title_block_iso7200().clone()
    } else {
        let text = files.read(st(&block["template_path"])).ok_or(())?;
        crate::json::parse(&text).map_err(|_| ())?
    };
    if !Schema::new(data::template_schema()).is_valid(&data)
        || st(&data["standard_version"]).split('.').next() != Some("1")
    {
        return Err(());
    }
    Ok(data)
}

/// Clause 14.2.1.
fn template_ok(t: &Value) -> bool {
    let (w, h) = (num(t, "width_mm"), num(t, "height_mm"));
    let pad = t.get("pad_mm").map_or(1.0, f);
    for ln in arr(&t["lines"]) {
        for key in ["from_mm", "to_mm"] {
            let q = p(&ln[key]);
            if !(-FIT_TOL_MM <= q[0]
                && q[0] <= w + FIT_TOL_MM
                && -FIT_TOL_MM <= q[1]
                && q[1] <= h + FIT_TOL_MM)
            {
                return false;
            }
        }
    }
    let cells = arr(&t["cells"]);
    let fields: Vec<&str> = cells.iter().map(|c| st(&c["field"])).collect();
    if fields.iter().collect::<BTreeSet<_>>().len() != fields.len() {
        return false;
    }
    for c in cells {
        let (x, y, cw, ch) = (
            num(c, "x_mm"),
            num(c, "y_mm"),
            num(c, "width_mm"),
            num(c, "height_mm"),
        );
        if x < 0.0 || y < 0.0 || x + cw > w + FIT_TOL_MM || y + ch > h + FIT_TOL_MM {
            return false;
        }
        let label_h = c.get("label_height_mm").map_or(2.5, f);
        if label_h + num(c, "text_height_mm") + 3.0 * pad > ch + FIT_TOL_MM {
            return false;
        }
        let label = st(&c["label"]);
        if !label.is_empty() && !fits(label, label_h, cw - 2.0 * pad) {
            return false;
        }
    }
    let rect = |c: &Value| Rect {
        x: num(c, "x_mm"),
        y: num(c, "y_mm"),
        w: num(c, "width_mm"),
        h: num(c, "height_mm"),
    };
    for i in 0..cells.len() {
        for j in (i + 1)..cells.len() {
            if overlap(rect(&cells[i]), rect(&cells[j])) {
                return false;
            }
        }
    }
    let reg = &t["register"];
    let columns = arr(&reg["columns"]);
    if (columns.iter().map(|c| num(c, "width_mm")).sum::<f64>() - w).abs() > FIT_TOL_MM {
        return false;
    }
    if num(reg, "row_height_mm") < num(reg, "text_height_mm") + 2.0 * pad - FIT_TOL_MM {
        return false;
    }
    if columns
        .iter()
        .map(|c| st(&c["field"]))
        .collect::<BTreeSet<_>>()
        .len()
        != 6
    {
        return false;
    }
    columns.iter().all(|c| {
        st(&c["label"]).is_empty()
            || fits(
                st(&c["label"]),
                num(reg, "text_height_mm"),
                num(c, "width_mm") - 2.0 * pad,
            )
    })
}

// ---- revisions (clauses 14.7 and 14.8) ----------------------------------------------------------

/// A token's place in its scheme: shorter first, then by text. For `alpha` this is the order
/// A < B < ... < Z < AA; for `numeric` the number without leading zeros.
fn token_key(scheme: &str, token: &str) -> Option<(usize, String)> {
    if scheme == "alpha" {
        (!token.is_empty() && token.bytes().all(|b| b.is_ascii_uppercase()))
            .then(|| (token.len(), token.to_string()))
    } else {
        if token.is_empty() || !token.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let trimmed = token.trim_start_matches('0');
        Some((trimmed.len(), trimmed.to_string()))
    }
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn opt_string(v: Option<&Value>) -> String {
    v.and_then(Value::as_str)
        .map_or_else(|| "null".to_string(), json_string)
}

/// The canonical text of entry `index` (clause 14.8).
fn canonical(entry: &Value, index: usize, prior: &str) -> String {
    format!(
        "{{\"app\":{},\"by\":{},\"chk\":{},\"date\":{},\"description\":{},\"index\":{},\"prior_digest\":{},\"rev\":{}}}",
        opt_string(entry.get("app")),
        json_string(st(&entry["by"])),
        opt_string(entry.get("chk")),
        json_string(st(&entry["date"])),
        json_string(st(&entry["description"])),
        index,
        json_string(prior),
        json_string(st(&entry["rev"])),
    )
}

/// The seal that each entry should carry (clause 14.8).
pub fn seals(entries: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    let mut prior = String::new();
    for (i, entry) in entries.iter().enumerate() {
        let digest = Sha256::digest(canonical(entry, i, &prior).as_bytes());
        prior = format!(
            "sha256:{}",
            digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        out.push(prior.clone());
    }
    out
}

/// A date `YYYY-MM-DD` that exists, as (year, month, day).
fn valid_date(text: &str) -> Option<(u32, u32, u32)> {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |s: &str| s.parse::<u32>().ok();
    let (y, m, d) = (num(&text[0..4])?, num(&text[5..7])?, num(&text[8..10])?);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 28 + u32::from(leap),
        _ => return None,
    };
    (y >= 1 && d >= 1 && d <= days).then_some((y, m, d))
}

fn revision_errors(revisions: &Value) -> BTreeSet<String> {
    let mut errors = BTreeSet::new();
    let scheme = revisions.get("scheme").map_or("alpha", st);
    let entries = arr(&revisions["entries"]);
    let mut prior_key: Option<(usize, String)> = None;
    let mut prior_date = None;
    for entry in entries {
        match token_key(scheme, st(&entry["rev"])) {
            Some(key) if prior_key.as_ref().is_none_or(|pk| key > *pk) => prior_key = Some(key),
            _ => {
                errors.insert("14.7.4".to_string());
            }
        }
        match valid_date(st(&entry["date"])) {
            None => {
                errors.insert("14.7.5".to_string());
            }
            Some(date) => {
                if prior_date.is_some_and(|pd| date < pd) {
                    errors.insert("14.7.5".to_string());
                }
                prior_date = Some(date);
            }
        }
    }
    for (entry, seal) in entries.iter().zip(seals(entries)) {
        if entry.get("seal").is_some_and(|s| st(s) != seal) {
            errors.insert("14.7.6".to_string());
        }
    }
    errors
}

/// Clause 14.7.2: how many rows show, and how many earlier entries are not shown.
pub fn register_layout(entries: usize, max_rows: usize) -> (usize, usize) {
    (
        entries.min(max_rows),
        if entries <= max_rows {
            0
        } else {
            entries - (max_rows - 1)
        },
    )
}

/// One row of the register: the cell texts by column field.
type RegisterRow = Vec<(&'static str, String)>;

fn register_rows(entries: &[Value], max_rows: usize) -> Vec<RegisterRow> {
    let cell = |e: &Value, k: &str| e.get(k).map_or(String::new(), |v| st(v).to_string());
    let (shown, mut rows): (&[Value], Vec<RegisterRow>) = if entries.len() <= max_rows {
        (entries, vec![])
    } else {
        let start = entries.len() - (max_rows - 1);
        (
            &entries[start..],
            vec![vec![(
                "description",
                label!(
                    "draw-earlier-revisions",
                    "{count} EARLIER REVISIONS NOT SHOWN",
                    count = entries.len() - (max_rows - 1)
                ),
            )]],
        )
    };
    for e in shown {
        rows.push(vec![
            ("rev", cell(e, "rev")),
            ("date", cell(e, "date")),
            ("description", cell(e, "description")),
            ("by", cell(e, "by")),
            ("chk", cell(e, "chk")),
            ("app", cell(e, "app")),
        ]);
    }
    rows
}

// ---- the plan -------------------------------------------------------------------------------

fn inside(r: Rect, ds: Rect) -> bool {
    r.x >= ds.x - FIT_TOL_MM
        && r.y >= ds.y - FIT_TOL_MM
        && r.x + r.w <= ds.x + ds.w + FIT_TOL_MM
        && r.y + r.h <= ds.y + ds.h + FIT_TOL_MM
}

/// All the furniture of clause 14, or its errors.
pub fn plan(
    doc: &Value,
    size: &str,
    ds: Rect,
    group: &LineGroup,
    scales: &[Option<u32>],
    files: &dyn Files,
) -> Plan {
    let mut out = Plan::default();
    if let Some(revisions) = doc.get("revisions") {
        out.errors.extend(revision_errors(revisions));
    }
    let entries = doc.get("revisions").map_or(&[][..], |r| arr(&r["entries"]));
    let block = doc.get("title_block");
    let mut template = None;
    if let Some(block) = block {
        match load_template(block, files) {
            Err(()) => {
                out.errors.insert("14.2".to_string());
            }
            Ok(t) if !template_ok(&t) => {
                out.errors.insert("14.2.1".to_string());
            }
            Ok(t) => template = Some(t),
        }
    }
    if let (Some(t), Some(b)) = (&template, block) {
        title_block(doc, b, t, entries, size, ds, group, scales, &mut out);
    }
    let band_top = out
        .regions
        .iter()
        .map(|(_, r)| r.y)
        .fold(ds.y + ds.h, f64::min);
    let strip_errors = strips(
        doc,
        Rect {
            x: ds.x,
            y: ds.y,
            w: ds.w,
            h: band_top - ds.y,
        },
        group,
        &mut out,
    );
    out.errors.extend(strip_errors);
    let status = doc.get("status").map(st);
    if status == Some("issued") {
        let latest = entries.last();
        let ok = block.is_some()
            && latest.is_some_and(|l| {
                let app_ok = l.get("app").is_some_and(|a| !st(a).trim().is_empty());
                app_ok
                    && l.get("seal").is_some_and(|s| {
                        seals(entries)
                            .last()
                            .is_some_and(|expected| st(s) == expected)
                    })
            });
        if !ok {
            out.errors.insert("14.9".to_string());
        }
    }
    if let Some(text) = status.and_then(status_text) {
        status_line(doc, text, ds, &mut out);
    }
    let regions = &out.regions;
    let mut bad = false;
    for i in 0..regions.len() {
        if !inside(regions[i].1, ds) {
            bad = true;
        }
        for j in (i + 1)..regions.len() {
            if overlap(regions[i].1, regions[j].1) {
                bad = true;
            }
        }
    }
    if bad {
        out.errors.insert("14.5".to_string());
    }
    out
}

/// Clause 14.9: the status line, in the notes strip when there is one, else in a region of its own.
fn status_line(doc: &Value, text: &str, ds: Rect, out: &mut Plan) {
    let notes = doc
        .get("strips")
        .map_or(&[][..], |s| arr(s))
        .iter()
        .find(|s| st(&s["content"]) == "notes")
        .map(|s| st(&s["id"]).to_string());
    let anchor = if let Some(id) = notes {
        let Some(r) = out.regions.iter().find(|(n, _)| *n == id).map(|(_, r)| *r) else {
            return; // the strip has an error of its own
        };
        if !fits(text, STATUS_HEIGHT, r.w - 2.0 * STATUS_PAD) {
            out.errors.insert("14.6".to_string());
        }
        [r.x + STATUS_PAD, r.y + r.h - STATUS_PAD]
    } else {
        let width = rules::text_width(text, STATUS_HEIGHT).unwrap_or(0.0);
        out.regions.push((
            "status".to_string(),
            Rect {
                x: ds.x,
                y: ds.y + ds.h - STATUS_REGION_HEIGHT,
                w: width + 2.0 * STATUS_PAD,
                h: STATUS_REGION_HEIGHT,
            },
        ));
        [ds.x + STATUS_PAD, ds.y + ds.h - STATUS_PAD]
    };
    out.entities.push(Entity::text(
        "sheet.status_line",
        FURNITURE_LAYER,
        anchor,
        text,
        STATUS_HEIGHT,
        0.0,
        AlignH::Left,
        AlignV::Baseline,
        None,
    ));
}

/// The values of the derived fields (clause 14.4.2) and the given fields (clause 14.4.1).
fn field_value(
    doc: &Value,
    entries: &[Value],
    size: &str,
    scales: &[Option<u32>],
    field: &str,
) -> String {
    let latest = entries.last();
    let from_latest = |k: &str| {
        latest
            .and_then(|l| l.get(k))
            .map_or(String::new(), |v| st(v).to_string())
    };
    let fields = doc.get("title_block").map(|b| &b["fields"]);
    match field {
        "revision" => from_latest("rev"),
        "date_of_issue" => from_latest("date"),
        "created_by" => from_latest("by"),
        "checked_by" => from_latest("chk"),
        "approved_by" => from_latest("app"),
        "status" => doc
            .get("status")
            .map_or(String::new(), |s| st(s).to_uppercase()),
        "scale" => {
            if scales.is_empty() || scales.iter().any(Option::is_none) {
                String::new()
            } else if scales.iter().all(|s| *s == scales[0]) {
                format!("1:{}", scales[0].unwrap_or(1))
            } else {
                label!("draw-scale-as-shown", "AS SHOWN").to_string()
            }
        }
        "sheet" => {
            match fields.and_then(|fl| Some((fl.get("sheet_number")?, fl.get("sheet_count")?))) {
                Some((n, m)) => format!("{}/{}", f(n) as i64, f(m) as i64),
                None => String::new(),
            }
        }
        "sheet_size" => size.to_string(),
        given => fields
            .and_then(|fl| fl.get(given))
            .map_or(String::new(), |v| st(v).to_string()),
    }
}

#[allow(clippy::too_many_arguments)]
fn title_block(
    doc: &Value,
    block: &Value,
    t: &Value,
    entries: &[Value],
    size: &str,
    ds: Rect,
    group: &LineGroup,
    scales: &[Option<u32>],
    out: &mut Plan,
) {
    let (w, h) = (num(t, "width_mm"), num(t, "height_mm"));
    let pad = t.get("pad_mm").map_or(1.0, f);
    let (bx, by) = (ds.x + ds.w - w, ds.y + ds.h - h);
    out.regions
        .push(("title_block".to_string(), Rect { x: bx, y: by, w, h }));
    let cells = arr(&t["cells"]);
    let cell_fields: BTreeSet<&str> = cells.iter().map(|c| st(&c["field"])).collect();
    if let Some(fields) = block["fields"].as_object() {
        for key in fields.keys() {
            let given_without_cell =
                GIVEN.contains(&key.as_str()) && !cell_fields.contains(key.as_str());
            let sheet_without_cell =
                (key == "sheet_number" || key == "sheet_count") && !cell_fields.contains("sheet");
            if given_without_cell || sheet_without_cell {
                out.errors.insert("14.4.3".to_string());
            }
        }
    }
    let (origin, lay) = ("sheet.title_block", FURNITURE_LAYER);
    for ln in arr(&t["lines"]) {
        let (a, b) = (p(&ln["from_mm"]), p(&ln["to_mm"]));
        let role = Role::parse(st(&ln["role"])).unwrap_or(Role::Thin);
        out.entities.push(Entity::line(
            origin,
            lay,
            [bx + a[0], by + a[1]],
            [bx + b[0], by + b[1]],
            "continuous",
            group.width(role),
            None,
        ));
    }
    for c in cells {
        let value = field_value(doc, entries, size, scales, st(&c["field"]));
        if c.get("required").and_then(Value::as_bool) == Some(true) && value.is_empty() {
            out.errors.insert("14.4.3".to_string());
        }
        let (cx, cy, cw, ch) = (
            bx + num(c, "x_mm"),
            by + num(c, "y_mm"),
            num(c, "width_mm"),
            num(c, "height_mm"),
        );
        let label_h = c.get("label_height_mm").map_or(2.5, f);
        let label = st(&c["label"]);
        if !label.is_empty() {
            out.entities.push(Entity::text(
                origin,
                lay,
                [cx + pad, cy + pad + label_h],
                label,
                label_h,
                0.0,
                AlignH::Left,
                AlignV::Baseline,
                None,
            ));
        }
        if !value.is_empty() {
            let text_h = num(c, "text_height_mm");
            if !fits(&value, text_h, cw - 2.0 * pad) {
                out.errors.insert("14.6".to_string());
            }
            let align = AlignH::parse(c.get("align_h").map_or("left", st));
            let x = match align {
                AlignH::Left => cx + pad,
                AlignH::Centre => cx + cw / 2.0,
                AlignH::Right => cx + cw - pad,
            };
            out.entities.push(Entity::text(
                origin,
                lay,
                [x, cy + ch - pad],
                &value,
                text_h,
                0.0,
                align,
                AlignV::Baseline,
                None,
            ));
        }
    }
    if !entries.is_empty() {
        register(entries, t, bx, by, w, pad, group, out);
    }
}

#[allow(clippy::too_many_arguments)]
fn register(
    entries: &[Value],
    t: &Value,
    bx: f64,
    by: f64,
    w: f64,
    pad: f64,
    group: &LineGroup,
    out: &mut Plan,
) {
    let reg = &t["register"];
    let rows = register_rows(entries, num(reg, "max_rows") as usize);
    let (rh, th) = (num(reg, "row_height_mm"), num(reg, "text_height_mm"));
    let height = (1 + rows.len()) as f64 * rh;
    let ry = by - height;
    out.regions.push((
        "register".to_string(),
        Rect {
            x: bx,
            y: ry,
            w,
            h: height,
        },
    ));
    let (origin, lay) = ("sheet.revision_register", FURNITURE_LAYER);
    out.entities.push(Entity::polyline(
        origin,
        lay,
        &[[bx, ry], [bx + w, ry], [bx + w, by], [bx, by]],
        true,
        "continuous",
        group.thick,
        None,
    ));
    for i in 1..=rows.len() {
        let y = ry + i as f64 * rh;
        out.entities.push(Entity::line(
            origin,
            lay,
            [bx, y],
            [bx + w, y],
            "continuous",
            group.thin,
            None,
        ));
    }
    let columns = arr(&reg["columns"]);
    let mut x = bx;
    for col in &columns[..columns.len().saturating_sub(1)] {
        x += num(col, "width_mm");
        out.entities.push(Entity::line(
            origin,
            lay,
            [x, ry],
            [x, by],
            "continuous",
            group.thin,
            None,
        ));
    }
    let header: Vec<(&'static str, String)> = columns
        .iter()
        .map(|c| (field_name(st(&c["field"])), st(&c["label"]).to_string()))
        .collect();
    let table: Vec<&Vec<(&'static str, String)>> =
        std::iter::once(&header).chain(rows.iter()).collect();
    for (r, row) in table.iter().enumerate() {
        let mut x = bx;
        for col in columns {
            let name = st(&col["field"]);
            let text = row
                .iter()
                .find(|(k, _)| *k == name)
                .map_or("", |(_, v)| v.as_str());
            if !text.is_empty() {
                if !fits(text, th, num(col, "width_mm") - 2.0 * pad) {
                    out.errors.insert("14.6".to_string());
                }
                out.entities.push(Entity::text(
                    origin,
                    lay,
                    [x + pad, ry + (r + 1) as f64 * rh - pad],
                    text,
                    th,
                    0.0,
                    AlignH::Left,
                    AlignV::Baseline,
                    None,
                ));
            }
            x += num(col, "width_mm");
        }
    }
}

fn field_name(name: &str) -> &'static str {
    match name {
        "rev" => "rev",
        "date" => "date",
        "description" => "description",
        "by" => "by",
        "chk" => "chk",
        _ => "app",
    }
}

fn strips(doc: &Value, area: Rect, group: &LineGroup, out: &mut Plan) -> BTreeSet<String> {
    let mut errors = BTreeSet::new();
    let list = doc.get("strips").map_or(&[][..], |s| arr(s));
    let ids: Vec<&str> = list.iter().map(|s| st(&s["id"])).collect();
    let contents: Vec<&str> = list.iter().map(|s| st(&s["content"])).collect();
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
        || contents.iter().collect::<BTreeSet<_>>().len() != contents.len()
    {
        errors.insert("14.10".to_string());
    }
    for member in ["notes", "references"] {
        if doc.get(member).is_some() && !contents.contains(&member) {
            errors.insert("14.10".to_string());
        }
    }
    let mut free = area;
    for s in list {
        let size = num(s, "size_mm");
        let edge = st(&s["edge"]);
        let rect;
        if edge == "left" || edge == "right" {
            if size >= free.w {
                errors.insert("14.10".to_string());
                continue;
            }
            if edge == "left" {
                rect = Rect {
                    x: free.x,
                    y: free.y,
                    w: size,
                    h: free.h,
                };
                free = Rect {
                    x: free.x + size,
                    y: free.y,
                    w: free.w - size,
                    h: free.h,
                };
            } else {
                rect = Rect {
                    x: free.x + free.w - size,
                    y: free.y,
                    w: size,
                    h: free.h,
                };
                free = Rect {
                    x: free.x,
                    y: free.y,
                    w: free.w - size,
                    h: free.h,
                };
            }
        } else {
            if size >= free.h {
                errors.insert("14.10".to_string());
                continue;
            }
            if edge == "top" {
                rect = Rect {
                    x: free.x,
                    y: free.y,
                    w: free.w,
                    h: size,
                };
                free = Rect {
                    x: free.x,
                    y: free.y + size,
                    w: free.w,
                    h: free.h - size,
                };
            } else {
                rect = Rect {
                    x: free.x,
                    y: free.y + free.h - size,
                    w: free.w,
                    h: size,
                };
                free = Rect {
                    x: free.x,
                    y: free.y,
                    w: free.w,
                    h: free.h - size,
                };
            }
        }
        out.regions.push((st(&s["id"]).to_string(), rect));
        strip_entities(doc, s, rect, group, out);
    }
    errors
}

fn strip_entities(doc: &Value, strip: &Value, r: Rect, group: &LineGroup, out: &mut Plan) {
    let kind = st(&strip["content"]);
    let origin = format!("sheet.{kind}");
    let lay = FURNITURE_LAYER;
    out.entities.push(Entity::polyline(
        &origin,
        lay,
        &[
            [r.x, r.y],
            [r.x + r.w, r.y],
            [r.x + r.w, r.y + r.h],
            [r.x, r.y + r.h],
        ],
        true,
        "continuous",
        group.thin,
        None,
    ));
    out.entities.push(Entity::text(
        &origin,
        lay,
        [r.x + STRIP_PAD, r.y + STRIP_PAD + STRIP_HEADING],
        if kind == "notes" {
            label!("draw-heading-notes", "NOTES")
        } else {
            label!("draw-heading-references", "REFERENCES")
        },
        STRIP_HEADING,
        0.0,
        AlignH::Left,
        AlignV::Baseline,
        None,
    ));
    // Clause 14.10.1: the status line stands on the last line of a notes strip.
    let reserve = if kind == "notes" && doc.get("status").and_then(|s| status_text(st(s))).is_some()
    {
        STRIP_PITCH
    } else {
        0.0
    };
    if reserve > 0.0
        && r.y + STRIP_PAD + STRIP_HEADING > r.y + r.h - STRIP_PAD - reserve + FIT_TOL_MM
    {
        out.errors.insert("14.6".to_string());
    }
    let items: Vec<String> = if kind == "notes" {
        doc.get("notes")
            .map_or(&[][..], |n| arr(n))
            .iter()
            .enumerate()
            .map(|(i, n)| format!("{}. {}", i + 1, st(n)))
            .collect()
    } else {
        doc.get("references")
            .map_or(&[][..], |n| arr(n))
            .iter()
            .map(|r| {
                ["document_number", "revision", "title"]
                    .iter()
                    .filter_map(|k| r.get(*k).map(st))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("  ")
            })
            .collect()
    };
    for (i, text) in items.iter().enumerate() {
        let baseline = r.y + STRIP_PAD + STRIP_HEADING + STRIP_PITCH * (i + 1) as f64;
        if baseline > r.y + r.h - STRIP_PAD - reserve + FIT_TOL_MM
            || !fits(text, STRIP_ITEM, r.w - 2.0 * STRIP_PAD)
        {
            out.errors.insert("14.6".to_string());
        }
        out.entities.push(Entity::text(
            &origin,
            lay,
            [r.x + STRIP_PAD, baseline],
            text,
            STRIP_ITEM,
            0.0,
            AlignH::Left,
            AlignV::Baseline,
            None,
        ));
    }
}
