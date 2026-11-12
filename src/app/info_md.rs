//! The guides' text: a small reader for the markdown the wiki writes
//! (headings, paragraphs, lists, tables, tip boxes, **bold**, `keys` and
//! links), laid out with the app's own type and colours. Paragraphs are one
//! galley each; links inside them are found under the pointer.

use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use crate::theme;
use eframe::egui::{self, text::LayoutJob, Align2, Color32, FontId, Rect, Sense, Stroke, TextFormat};
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Span {
    pub text: String,
    pub bold: bool,
    pub code: bool,
    pub link: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) enum Block {
    Heading { level: u8, spans: Vec<Span> },
    Para(Vec<Span>),
    /// A tip box: its paragraphs.
    Quote(Vec<Vec<Span>>),
    /// Items: (depth, number for ordered lists, text).
    List(Vec<(u8, Option<u32>, Vec<Span>)>),
    Table { head: Vec<Vec<Span>>, rows: Vec<Vec<Vec<Span>>> },
    Code(String),
    Rule,
}

/// A heading you can jump to.
#[derive(Clone, Debug)]
pub(super) struct Section {
    pub title: String,
    pub level: u8,
    /// Index of its heading block.
    pub block: usize,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Doc {
    pub blocks: Vec<Block>,
    pub sections: Vec<Section>,
    pub words: usize,
}

pub(super) fn plain(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

// ───────────────────────────── parsing ─────────────────────────────

/// `**bold**`, `` `code` ``, `[text](link)` and `\` escapes.
pub(super) fn inline(s: &str) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut cur = String::new();
    let mut bold = false;
    let link: Option<String> = None;
    let chars: Vec<char> = s.chars().collect();
    let flush = |out: &mut Vec<Span>, cur: &mut String, bold: bool, code: bool, link: &Option<String>| {
        if !cur.is_empty() {
            out.push(Span { text: std::mem::take(cur), bold, code, link: link.clone() });
        }
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            cur.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '*' && chars.get(i + 1) == Some(&'*') {
            flush(&mut out, &mut cur, bold, false, &link);
            bold = !bold;
            i += 2;
            continue;
        }
        if c == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|&x| x == '`') {
                flush(&mut out, &mut cur, bold, false, &link);
                let code: String = chars[i + 1..i + 1 + end].iter().collect();
                out.push(Span { text: code, bold: false, code: true, link: link.clone() });
                i += end + 2;
                continue;
            }
        }
        if c == '[' && link.is_none() {
            // [text](url): find the matching "](" and ")"
            if let Some(close) = chars[i + 1..].iter().position(|&x| x == ']') {
                let after = i + 1 + close;
                if chars.get(after + 1) == Some(&'(') {
                    if let Some(end) = chars[after + 2..].iter().position(|&x| x == ')') {
                        flush(&mut out, &mut cur, bold, false, &link);
                        let url: String = chars[after + 2..after + 2 + end].iter().collect();
                        let text: String = chars[i + 1..after].iter().collect();
                        for mut sp in inline(&text) {
                            sp.bold |= bold;
                            sp.link = Some(url.clone());
                            out.push(sp);
                        }
                        i = after + 3 + end;
                        continue;
                    }
                }
            }
        }
        cur.push(c);
        i += 1;
    }
    flush(&mut out, &mut cur, bold, false, &link);
    out
}

fn list_marker(line: &str) -> Option<(u8, Option<u32>, &str)> {
    let indent = line.len() - line.trim_start().len();
    let t = line.trim_start();
    let depth = ((indent + 1) / 2).min(3) as u8;
    if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
        return Some((depth, None, rest));
    }
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        if let Some(rest) = t[digits.len()..].strip_prefix(". ") {
            return Some((depth, digits.parse().ok(), rest));
        }
    }
    None
}

fn table_cells(line: &str) -> Vec<String> {
    let t = line.trim().trim_start_matches('|').trim_end_matches('|');
    // split on | that isn't escaped or inside `code`
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut code = false;
    let mut prev = ' ';
    for c in t.chars() {
        if c == '`' {
            code = !code;
        }
        if c == '|' && !code && prev != '\\' {
            cells.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
        prev = c;
    }
    cells.push(cur.trim().to_string());
    cells
}

fn is_rule_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

pub(super) fn parse(md: &str) -> Doc {
    let lines: Vec<&str> = md.lines().collect();
    let mut blocks = Vec::new();
    let mut para: Vec<&str> = Vec::new();
    let flush = |blocks: &mut Vec<Block>, para: &mut Vec<&str>| {
        if !para.is_empty() {
            blocks.push(Block::Para(inline(&para.join(" "))));
            para.clear();
        }
    };
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim();
        if t.is_empty() {
            flush(&mut blocks, &mut para);
            i += 1;
            continue;
        }
        if t.starts_with("```") {
            flush(&mut blocks, &mut para);
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code.push(lines[i]);
                i += 1;
            }
            blocks.push(Block::Code(code.join("\n")));
            i += 1;
            continue;
        }
        if t.starts_with('#') {
            let level = t.chars().take_while(|&c| c == '#').count();
            if level <= 6 && t[level..].starts_with(' ') {
                flush(&mut blocks, &mut para);
                blocks.push(Block::Heading { level: level as u8, spans: inline(t[level..].trim()) });
                i += 1;
                continue;
            }
        }
        if t == "---" || t == "***" {
            flush(&mut blocks, &mut para);
            blocks.push(Block::Rule);
            i += 1;
            continue;
        }
        if t.starts_with('>') {
            flush(&mut blocks, &mut para);
            let mut paras: Vec<Vec<Span>> = Vec::new();
            let mut cur: Vec<&str> = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('>') {
                let inner = lines[i].trim().trim_start_matches('>').trim();
                if inner.is_empty() {
                    if !cur.is_empty() {
                        paras.push(inline(&cur.join(" ")));
                        cur.clear();
                    }
                } else {
                    cur.push(inner);
                }
                i += 1;
            }
            if !cur.is_empty() {
                paras.push(inline(&cur.join(" ")));
            }
            blocks.push(Block::Quote(paras));
            continue;
        }
        if t.starts_with('|') && lines.get(i + 1).is_some_and(|l| is_rule_row(l)) {
            flush(&mut blocks, &mut para);
            let head = table_cells(t).iter().map(|c| inline(c)).collect();
            i += 2;
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                rows.push(table_cells(lines[i]).iter().map(|c| inline(c)).collect());
                i += 1;
            }
            blocks.push(Block::Table { head, rows });
            continue;
        }
        if list_marker(line).is_some() {
            flush(&mut blocks, &mut para);
            let mut items: Vec<(u8, Option<u32>, String)> = Vec::new();
            while i < lines.len() {
                let l = lines[i];
                if let Some((d, n, rest)) = list_marker(l) {
                    items.push((d, n, rest.to_string()));
                } else if !l.trim().is_empty() && l.starts_with(' ') && !items.is_empty() {
                    // an indented line carries on the item above
                    let last = items.last_mut().expect("item");
                    last.2.push(' ');
                    last.2.push_str(l.trim());
                } else {
                    break;
                }
                i += 1;
            }
            blocks.push(Block::List(items.into_iter().map(|(d, n, s)| (d, n, inline(&s))).collect()));
            continue;
        }
        para.push(t);
        i += 1;
    }
    flush(&mut blocks, &mut para);
    let sections = blocks
        .iter()
        .enumerate()
        .filter_map(|(i, b)| match b {
            Block::Heading { level, spans } if *level <= 3 => Some(Section { title: plain(spans), level: *level, block: i }),
            _ => None,
        })
        .collect();
    let words = md.split_whitespace().count();
    Doc { blocks, sections, words }
}

// ───────────────────────────── drawing ─────────────────────────────

/// How text is set: size, colour, bold.
#[derive(Clone, Copy)]
pub(super) struct Style {
    pub size: f32,
    pub color: Color32,
    pub bold: bool,
}

impl Style {
    pub fn body() -> Self {
        Style { size: 14.0, color: TEXT(), bold: false }
    }
}

/// Lay out spans, with the parts matching `find` marked. Returns the galley
/// and each link's character range.
pub(super) fn layout(ui: &egui::Ui, spans: &[Span], st: Style, width: f32, find: Option<&str>) -> (Arc<egui::Galley>, Vec<(Range<usize>, String)>) {
    let pl = theme::pal();
    let mut job = LayoutJob { wrap: egui::text::TextWrapping { max_width: width, ..Default::default() }, ..Default::default() };
    job.halign = egui::Align::LEFT;
    let mut links = Vec::new();
    let mut at = 0usize;
    let mark_bg = theme::blend(CARD(), theme::GOLD, if pl.light { 0.45 } else { 0.38 });
    let find = find.map(str::to_lowercase).filter(|f| f.chars().count() >= 2);
    for sp in spans {
        let font = if sp.code {
            FontId::monospace(st.size - 1.5)
        } else if sp.bold || st.bold {
            crate::fonts::sans_bold_font(st.size)
        } else {
            crate::fonts::sans_font(st.size)
        };
        let mut fmt = TextFormat { font_id: font, color: st.color, line_height: Some(st.size * 1.55), ..Default::default() };
        if sp.code {
            fmt.background = kit::sunken();
            fmt.color = TEXT();
        }
        if sp.link.is_some() {
            fmt.color = theme::blend(pl.accent, TEXT(), if pl.light { 0.1 } else { 0.25 });
            fmt.underline = Stroke::new(1.0f32, kit::tint(pl.accent, 0.6));
        }
        // keys sit in a little box: pad them with thin spaces
        let text = if sp.code { format!("\u{2009}{}\u{2009}", sp.text) } else { sp.text.clone() };
        let n = text.chars().count();
        if let Some(link) = &sp.link {
            links.push((at..at + n, link.clone()));
        }
        // marked matches
        match &find {
            Some(f) if text.to_lowercase().len() == text.len() && text.to_lowercase().contains(f.as_str()) => {
                let lower = text.to_lowercase();
                let mut pos = 0;
                // work in bytes of the lower-cased copy; same lengths for the text we have
                while let Some(off) = lower[pos..].find(f.as_str()) {
                    let s = pos + off;
                    let e = s + f.len();
                    if !text.is_char_boundary(s) || !text.is_char_boundary(e) {
                        break;
                    }
                    if s > pos {
                        job.append(&text[pos..s], 0.0, fmt.clone());
                    }
                    let mut hi = fmt.clone();
                    hi.background = mark_bg;
                    hi.color = TEXT();
                    job.append(&text[s..e], 0.0, hi);
                    pos = e;
                }
                if pos < text.len() {
                    job.append(&text[pos..], 0.0, fmt.clone());
                }
            }
            _ => job.append(&text, 0.0, fmt),
        }
        at += n;
    }
    (ui.fonts(|f| f.layout_job(job)), links)
}

/// Draw a laid-out paragraph; returns the link clicked, if any.
pub(super) fn show_galley(ui: &mut egui::Ui, galley: Arc<egui::Galley>, links: &[(Range<usize>, String)]) -> Option<String> {
    let sense = if links.is_empty() { Sense::hover() } else { Sense::click() };
    let (rect, resp) = ui.allocate_exact_size(galley.size(), sense);
    let mut clicked = None;
    if !links.is_empty() {
        if let Some(pos) = resp.hover_pos() {
            let local = pos - rect.min;
            // only over the text itself, not past the end of a line
            let on_text = galley.rows.iter().any(|r| r.rect.contains(local.to_pos2()) && local.x <= r.rect.max.x);
            if on_text {
                let c = galley.cursor_from_pos(local);
                let idx = c.ccursor.index;
                if let Some((_, url)) = links.iter().find(|(r, _)| r.contains(&idx)) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    if resp.clicked() {
                        clicked = Some(url.clone());
                    }
                    resp.clone().on_hover_text(link_hint(url));
                }
            }
        }
    }
    if ui.is_rect_visible(rect) {
        ui.painter().galley(rect.min, galley, TEXT());
    }
    clicked
}

/// What a link opens, for its tooltip.
fn link_hint(url: &str) -> String {
    if url.starts_with("#/guide/") {
        "Open this guide".into()
    } else if url.starts_with("#/pokemon/") {
        "Open in the Pokédex".into()
    } else if url.starts_with("#/item/") {
        "Show this item".into()
    } else {
        url.to_string()
    }
}

/// What came out of drawing a document.
#[derive(Default)]
pub(super) struct Shown {
    pub clicked: Option<String>,
    /// Screen y of each section heading drawn (section index, y).
    pub heads: Vec<(usize, f32)>,
    /// Where to scroll to (asked for once the whole page is drawn, so no
    /// smaller scroll area on the page takes the request).
    pub scroll: Option<Rect>,
}

/// Options for drawing.
pub(super) struct Opts<'a> {
    pub find: Option<&'a str>,
    /// Jump to this section (scrolls it into view).
    pub scroll_to: Option<usize>,
    /// Heights of each block last time (to skip drawing ones off screen).
    pub heights: &'a mut Vec<f32>,
}

/// Draw blocks `range` of a document.
pub(super) fn show_blocks(ui: &mut egui::Ui, doc: &Doc, range: Range<usize>, o: &mut Opts<'_>, out: &mut Shown) {
    if o.heights.len() != doc.blocks.len() {
        *o.heights = vec![0.0; doc.blocks.len()];
    }
    let width = ui.available_width();
    let clip = ui.clip_rect().expand(600.0);
    for bi in range {
        let b = &doc.blocks[bi];
        let top = ui.cursor().min.y;
        let section = doc.sections.iter().position(|s| s.block == bi);
        if let Some(si) = section {
            out.heads.push((si, top));
        }
        // off screen and drawn before: just keep its room
        let known = o.heights[bi];
        let wanted = section.is_some() && o.scroll_to == section;
        if known > 0.0 && !wanted && (top > clip.max.y || top + known < clip.min.y) && o.find.is_none() {
            ui.allocate_space(egui::vec2(width, known));
            continue;
        }
        match b {
            Block::Heading { level, spans } => {
                let size = match level {
                    1 => 24.0,
                    2 => 20.0,
                    3 => 16.0,
                    _ => 14.5,
                };
                ui.add_space(if *level <= 2 { 18.0 } else { 10.0 });
                let (g, links) = layout(ui, spans, Style { size, color: TEXT(), bold: true }, width, o.find);
                let head_top = ui.cursor().min.y;
                if let Some(c) = show_galley(ui, g, &links) {
                    out.clicked = Some(c);
                }
                if *level <= 2 {
                    ui.add_space(4.0);
                    let (r, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), Sense::hover());
                    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, kit::tint(LINE(), 0.8)));
                }
                ui.add_space(8.0);
                if wanted {
                    let r = Rect::from_min_max(egui::pos2(ui.min_rect().min.x, head_top - 16.0), egui::pos2(ui.min_rect().max.x, head_top + 40.0));
                    out.scroll = Some(r);
                }
            }
            Block::Para(spans) => {
                let (g, links) = layout(ui, spans, Style::body(), width, o.find);
                if let Some(c) = show_galley(ui, g, &links) {
                    out.clicked = Some(c);
                }
                ui.add_space(12.0);
            }
            Block::Quote(paras) => {
                let pl = theme::pal();
                let r0 = ui.cursor().min;
                let inner = egui::Frame::none()
                    .fill(theme::blend(CARD(), pl.accent, if pl.light { 0.08 } else { 0.12 }))
                    .rounding(12.0)
                    .inner_margin(egui::Margin { left: 18.0, right: 14.0, top: 12.0, bottom: 4.0 })
                    .show(ui, |ui| {
                        ui.set_width(width - 32.0);
                        for p in paras {
                            let (g, links) = layout(ui, p, Style::body(), width - 32.0, o.find);
                            if let Some(c) = show_galley(ui, g, &links) {
                                out.clicked = Some(c);
                            }
                            ui.add_space(8.0);
                        }
                    });
                let r = inner.response.rect;
                ui.painter().rect_filled(Rect::from_min_size(r0, egui::vec2(4.0, r.height())), egui::Rounding { nw: 12.0, sw: 12.0, ne: 0.0, se: 0.0 }, pl.accent);
                ui.add_space(14.0);
            }
            Block::List(items) => {
                for (depth, num, spans) in items {
                    let indent = 6.0 + *depth as f32 * 22.0;
                    let mark_w = if num.is_some() { 26.0 } else { 18.0 };
                    let text_w = (width - indent - mark_w).max(80.0);
                    let (g, links) = layout(ui, spans, Style::body(), text_w, o.find);
                    let first_row_h = g.rows.first().map(|r| r.rect.height()).unwrap_or(20.0);
                    let mut got = None;
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let (mr, _) = ui.allocate_exact_size(egui::vec2(indent + mark_w, first_row_h), Sense::hover());
                        let c = egui::pos2(mr.min.x + indent + 4.0, mr.center().y);
                        match num {
                            Some(n) => {
                                let cr = Rect::from_center_size(egui::pos2(c.x + 7.0, c.y), egui::vec2(20.0, 20.0));
                                ui.painter().circle_filled(cr.center(), 10.0, kit::tint(theme::pal().accent, 0.22));
                                ui.painter().text(cr.center(), Align2::CENTER_CENTER, n.to_string(), crate::fonts::sans_bold_font(11.0), TEXT());
                            }
                            None if *depth == 0 => {
                                ui.painter().circle_filled(c, 3.0, theme::pal().accent);
                            }
                            None => {
                                ui.painter().circle_stroke(c, 2.6, Stroke::new(1.3f32, MUTED()));
                            }
                        }
                        got = show_galley(ui, g, &links);
                    });
                    if got.is_some() {
                        out.clicked = got;
                    }
                    ui.add_space(5.0);
                }
                ui.add_space(8.0);
            }
            Block::Table { head, rows } => {
                if let Some(c) = show_table(ui, head, rows, width, o.find) {
                    out.clicked = Some(c);
                }
                ui.add_space(16.0);
            }
            Block::Code(text) => {
                egui::Frame::none().fill(kit::sunken()).rounding(10.0).inner_margin(12.0).show(ui, |ui| {
                    ui.set_width(width - 24.0);
                    ui.label(egui::RichText::new(text).monospace().size(12.5).color(TEXT()));
                });
                ui.add_space(12.0);
            }
            Block::Rule => {
                ui.add_space(8.0);
                let (r, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), Sense::hover());
                ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, LINE()));
                ui.add_space(12.0);
            }
        }
        o.heights[bi] = ui.cursor().min.y - top;
    }
}

/// Column widths: each column's natural width, squeezed fairly to fit.
fn column_widths(natural: &[f32], total: f32) -> Vec<f32> {
    let n = natural.len().max(1);
    let want: f32 = natural.iter().sum();
    if want <= total {
        // spare room goes to the last column
        let mut w = natural.to_vec();
        if let Some(l) = w.last_mut() {
            *l += total - want;
        }
        return w;
    }
    let mut w = vec![0.0f32; n];
    let mut left = total;
    let mut open: Vec<usize> = (0..n).collect();
    // columns narrower than a fair share keep their width; the rest share what's left
    loop {
        let share = left / open.len().max(1) as f32;
        let (small, big): (Vec<usize>, Vec<usize>) = open.iter().partition(|&&i| natural[i] <= share);
        if small.is_empty() {
            for &i in &big {
                w[i] = share;
            }
            break;
        }
        for &i in &small {
            w[i] = natural[i];
            left -= natural[i];
        }
        open = big;
        if open.is_empty() {
            break;
        }
    }
    w
}

fn show_table(ui: &mut egui::Ui, head: &[Vec<Span>], rows: &[Vec<Vec<Span>>], width: f32, find: Option<&str>) -> Option<String> {
    let cols = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0)).max(1);
    let pad = 10.0;
    let head_st = Style { size: 12.5, color: MUTED(), bold: true };
    let cell_st = Style { size: 13.5, color: TEXT(), bold: false };
    let empty: Vec<Span> = Vec::new();
    let cell = |r: &[Vec<Span>], c: usize| -> Vec<Span> { r.get(c).cloned().unwrap_or_else(|| empty.clone()) };
    let mut natural = vec![40.0f32; cols];
    for c in 0..cols {
        let (g, _) = layout(ui, &cell(head, c), head_st, f32::INFINITY, None);
        natural[c] = natural[c].max(g.size().x + pad * 2.0);
        for r in rows {
            let (g, _) = layout(ui, &cell(r, c), cell_st, f32::INFINITY, None);
            natural[c] = natural[c].max((g.size().x + pad * 2.0).min(520.0));
        }
    }
    let ws = column_widths(&natural, width);
    let mut clicked = None;
    let top = ui.cursor().min;
    let mut y = top.y;
    let mut row_rects: Vec<Rect> = Vec::new();
    let all_rows: Vec<(bool, &[Vec<Span>])> = std::iter::once((true, head)).chain(rows.iter().map(|r| (false, r.as_slice()))).collect();
    let mut drawn: Vec<(Rect, Arc<egui::Galley>, Vec<(Range<usize>, String)>)> = Vec::new();
    for (is_head, r) in &all_rows {
        let st = if *is_head { head_st } else { cell_st };
        let mut x = top.x;
        let mut cells = Vec::new();
        let mut h: f32 = 0.0;
        for (c, w) in ws.iter().enumerate() {
            let (g, links) = layout(ui, &cell(r, c), st, (w - pad * 2.0).max(20.0), find);
            h = h.max(g.size().y);
            cells.push((x, g, links));
            x += w;
        }
        let rh = h + pad * 1.4;
        row_rects.push(Rect::from_min_size(egui::pos2(top.x, y), egui::vec2(width, rh)));
        for (x, g, links) in cells {
            drawn.push((Rect::from_min_size(egui::pos2(x + pad, y + pad * 0.7), g.size()), g, links));
        }
        y += rh;
    }
    let total = Rect::from_min_max(top, egui::pos2(top.x + width, y));
    // background, header, lines
    let p = ui.painter();
    p.rect_filled(total, 12.0, theme::blend(CARD(), kit::sunken(), 0.35));
    if let Some(h) = row_rects.first() {
        p.rect_filled(*h, egui::Rounding { nw: 12.0, ne: 12.0, sw: 0.0, se: 0.0 }, kit::sunken());
    }
    for r in row_rects.iter().skip(1) {
        p.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0f32, kit::tint(LINE(), 0.7)));
    }
    p.rect_stroke(total, 12.0, Stroke::new(1.0f32, kit::tint(LINE(), 0.9)));
    // the cells, with their links
    for (rect, g, links) in drawn {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        if let Some(c) = show_galley(&mut child, g, &links) {
            clicked = Some(c);
        }
    }
    ui.allocate_rect(total, Sense::hover());
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_bits() {
        let s = inline("Press **SAVE** then `Ctrl+R`, see [the guide](#/guide/saving).");
        assert_eq!(plain(&s), "Press SAVE then Ctrl+R, see the guide.");
        assert!(s.iter().any(|x| x.bold && x.text == "SAVE"));
        assert!(s.iter().any(|x| x.code && x.text == "Ctrl+R"));
        assert!(s.iter().any(|x| x.link.as_deref() == Some("#/guide/saving") && x.text == "the guide"));
        let b = inline("[**Bold link**](#/item/x) \\*not bold\\*");
        assert!(b[0].bold && b[0].link.is_some());
        assert_eq!(plain(&b), "Bold link *not bold*");
    }

    #[test]
    fn blocks() {
        let d = parse("> **Tip.** read this\n\n## One\n\nHello\nworld\n\n- a\n  - b\n1. first\n2. second\n\n| A | B |\n|---|---|\n| 1 | `x|y` |\n\n### Two\n");
        assert!(matches!(d.blocks[0], Block::Quote(_)));
        assert!(matches!(&d.blocks[2], Block::Para(s) if plain(s) == "Hello world"));
        match &d.blocks[3] {
            Block::List(items) => {
                assert_eq!(items.len(), 4);
                assert_eq!(items[1].0, 1);
                assert_eq!(items[3].1, Some(2));
            }
            b => panic!("{b:?}"),
        }
        match &d.blocks[4] {
            Block::Table { head, rows } => {
                assert_eq!(head.len(), 2);
                assert_eq!(plain(&rows[0][1]), "x|y");
            }
            b => panic!("{b:?}"),
        }
        assert_eq!(d.sections.len(), 2);
        assert_eq!(d.sections[1].title, "Two");
    }

    #[test]
    fn squeezes_columns() {
        let w = column_widths(&[100.0, 600.0, 900.0], 800.0);
        assert!((w.iter().sum::<f32>() - 800.0).abs() < 0.5);
        assert_eq!(w[0], 100.0);
        let w = column_widths(&[100.0, 100.0], 500.0);
        assert_eq!(w, vec![100.0, 400.0]);
    }

    #[test]
    fn every_real_guide_parses() {
        let Ok(bytes) = std::fs::read("assets/wiki.json") else { return };
        let w = crate::wiki::Wiki::parse(&bytes).expect("parses");
        for g in &w.data.guides {
            let d = parse(&g.body);
            assert!(!d.blocks.is_empty(), "{}", g.id);
            // no stray markers left in plain text
            for b in &d.blocks {
                if let Block::Para(s) = b {
                    assert!(!plain(s).contains("**"), "{}: {}", g.id, plain(s));
                }
            }
        }
    }
}
