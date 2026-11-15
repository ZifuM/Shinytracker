//! How the Journal shows Markdown: Live preview (the note as typed, styled
//! as it reads, with the `**`, `#` and `[[ ]]` marks faded) and Reading
//! (fully laid out: headings, lists, ticks, quotes, callouts, tables and
//! code, with links, tags and ticks you can click).

use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use crate::theme;
use eframe::egui::{self, pos2, text::LayoutJob, vec2, Align2, Color32, FontId, Rect, Sense, Stroke, TextFormat};

/// Sizes and colours for one note.
#[derive(Clone, Copy)]
pub(super) struct MdStyle {
    pub size: f32,
    pub accent: Color32,
}

pub(super) const LINK: Color32 = Color32::from_rgb(167, 139, 250);
pub(super) const TAG: Color32 = Color32::from_rgb(167, 139, 250);
const MARK_BG: Color32 = Color32::from_rgb(92, 76, 20);
const MARK_FG: Color32 = Color32::from_rgb(255, 233, 168);

fn heading_scale(l: u8) -> f32 {
    match l {
        1 => 1.9,
        2 => 1.5,
        3 => 1.28,
        4 => 1.14,
        5 => 1.05,
        _ => 1.0,
    }
}

fn callout_color(kind: &str) -> Color32 {
    match kind.to_lowercase().as_str() {
        "warning" | "caution" | "attention" => kit::C_ORANGE,
        "danger" | "error" | "bug" | "fail" | "failure" => kit::C_RED,
        "success" | "check" | "done" => kit::C_GREEN,
        "question" | "help" | "faq" => kit::C_GOLD,
        "example" | "quote" => kit::C_VIOLET,
        _ => kit::C_CYAN,
    }
}

/// What a piece of a line is.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(super) struct Fx {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub mark: bool,
    pub code: bool,
    pub link: bool,
    pub tag: bool,
    /// Markdown's own marks (`**`, `[[`…): faded in Live preview, left out in Reading.
    pub marker: bool,
}

/// One run of a line, and where a link or tag in it goes.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Seg {
    pub text: String,
    pub fx: Fx,
    /// A link's target ("Route 214", or a web address), a tag's name.
    pub target: Option<String>,
}

/// A line's runs: bold, italic, strike, highlight, code, links and tags,
/// with their marks as runs of their own.
pub(super) fn inline(line: &str) -> Vec<Seg> {
    let c: Vec<char> = line.chars().collect();
    let mut out: Vec<Seg> = Vec::new();
    let mut fx = Fx::default();
    let mut buf = String::new();
    let flush = |buf: &mut String, out: &mut Vec<Seg>, fx: Fx| {
        if !buf.is_empty() {
            out.push(Seg { text: std::mem::take(buf), fx, target: None });
        }
    };
    let mark = |out: &mut Vec<Seg>, s: &str, fx: Fx| out.push(Seg { text: s.to_string(), fx: Fx { marker: true, ..fx }, target: None });
    let at = |i: usize, s: &str| -> bool { s.chars().enumerate().all(|(k, ch)| c.get(i + k) == Some(&ch)) };
    let find = |from: usize, s: &str| -> Option<usize> { (from..c.len()).find(|&j| at(j, s)) };
    let mut i = 0;
    while i < c.len() {
        // `code`
        if c[i] == '`' {
            if let Some(j) = find(i + 1, "`") {
                flush(&mut buf, &mut out, fx);
                mark(&mut out, "`", fx);
                out.push(Seg { text: c[i + 1..j].iter().collect(), fx: Fx { code: true, ..Fx::default() }, target: None });
                mark(&mut out, "`", fx);
                i = j + 1;
                continue;
            }
        }
        // [[link]] / [[link|shown]]
        if at(i, "[[") {
            if let Some(j) = find(i + 2, "]]") {
                let inner: String = c[i + 2..j].iter().collect();
                if !inner.is_empty() {
                    flush(&mut buf, &mut out, fx);
                    let (target, shown) = match inner.split_once('|') {
                        Some((t, s)) => (t.to_string(), Some(s.to_string())),
                        None => (inner.clone(), None),
                    };
                    match shown {
                        Some(s) => {
                            mark(&mut out, &format!("[[{target}|"), fx);
                            out.push(Seg { text: s, fx: Fx { link: true, ..fx }, target: Some(target) });
                        }
                        None => {
                            mark(&mut out, "[[", fx);
                            out.push(Seg { text: inner.clone(), fx: Fx { link: true, ..fx }, target: Some(target) });
                        }
                    }
                    mark(&mut out, "]]", fx);
                    i = j + 2;
                    continue;
                }
            }
        }
        // [text](url)
        if c[i] == '[' && !at(i, "[[") {
            if let Some(j) = find(i + 1, "](") {
                if let Some(k) = find(j + 2, ")") {
                    flush(&mut buf, &mut out, fx);
                    mark(&mut out, "[", fx);
                    let url: String = c[j + 2..k].iter().collect();
                    out.push(Seg { text: c[i + 1..j].iter().collect(), fx: Fx { link: true, ..fx }, target: Some(url.clone()) });
                    mark(&mut out, &format!("]({url})"), fx);
                    i = k + 1;
                    continue;
                }
            }
        }
        // #tag at the start of a word
        if c[i] == '#' && (i == 0 || c[i - 1].is_whitespace() || c[i - 1] == '(') {
            let mut j = i + 1;
            while j < c.len() && (c[j].is_alphanumeric() || matches!(c[j], '_' | '-' | '/')) {
                j += 1;
            }
            let tag: String = c[i + 1..j].iter().collect();
            if !tag.is_empty() && !tag.chars().all(|ch| ch.is_ascii_digit()) {
                flush(&mut buf, &mut out, fx);
                out.push(Seg { text: c[i..j].iter().collect(), fx: Fx { tag: true, ..fx }, target: Some(tag) });
                i = j;
                continue;
            }
        }
        // two-character toggles
        let two: Option<(&str, fn(&mut Fx) -> &mut bool)> = if at(i, "**") || at(i, "__") {
            Some((if at(i, "**") { "**" } else { "__" }, |f| &mut f.bold))
        } else if at(i, "~~") {
            Some(("~~", |f| &mut f.strike))
        } else if at(i, "==") {
            Some(("==", |f| &mut f.mark))
        } else {
            None
        };
        if let Some((m, field)) = two {
            let on = *field(&mut fx.clone());
            // open only if it closes later on the line
            if on || find(i + 2, m).is_some() {
                flush(&mut buf, &mut out, fx);
                let mut nfx = fx;
                *field(&mut nfx) = !on;
                mark(&mut out, m, if on { fx } else { nfx });
                fx = nfx;
                i += 2;
                continue;
            }
        }
        // *italic* / _italic_ (a lone `_` inside a word stays a `_`)
        if c[i] == '*' || (c[i] == '_' && (i == 0 || !c[i - 1].is_alphanumeric() || fx.italic)) {
            let ch = c[i].to_string();
            let closes = fx.italic || (c.get(i + 1).is_some_and(|n| !n.is_whitespace()) && find(i + 1, &ch).is_some());
            if closes {
                flush(&mut buf, &mut out, fx);
                let nfx = Fx { italic: !fx.italic, ..fx };
                mark(&mut out, &ch, if fx.italic { fx } else { nfx });
                fx = nfx;
                i += 1;
                continue;
            }
        }
        buf.push(c[i]);
        i += 1;
    }
    flush(&mut buf, &mut out, fx);
    out
}

fn fmt_for(fx: Fx, size: f32, base: Color32, accent: Color32, bold_all: bool) -> TextFormat {
    let bold = fx.bold || bold_all;
    let font = if fx.code {
        FontId::monospace(size * 0.92)
    } else if bold {
        crate::fonts::sans_bold_font(size)
    } else {
        crate::fonts::sans_font(size)
    };
    let mut f = TextFormat { font_id: font, color: base, ..Default::default() };
    if fx.italic {
        f.italics = true;
    }
    if fx.strike {
        f.strikethrough = Stroke::new(1.2f32, base);
    }
    if fx.mark {
        f.background = MARK_BG;
        f.color = MARK_FG;
    }
    if fx.code {
        f.background = theme::blend(CARD(), TEXT(), 0.08);
        f.color = theme::blend(base, accent, 0.35);
    }
    if fx.link {
        f.color = LINK;
        f.underline = Stroke::new(1.0f32, LINK.gamma_multiply(0.45));
    }
    if fx.tag {
        f.color = TAG;
        f.background = TAG.gamma_multiply(0.14);
    }
    if fx.marker {
        f.color = MUTED().gamma_multiply(0.75);
        f.font_id = if fx.code { FontId::monospace(size * 0.85) } else { crate::fonts::sans_font(size * 0.86) };
        f.background = Color32::TRANSPARENT;
        f.underline = Stroke::NONE;
        f.strikethrough = Stroke::NONE;
        f.italics = false;
    }
    f
}

/// What a line is, for both views.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum LineKind {
    Blank,
    Heading(u8),
    /// indent, marker ("- ", "1. "), task (Some(done))
    Item(usize, String, Option<bool>),
    Quote,
    Callout(String),
    Fence,
    Rule,
    Table,
    /// A line that's only `![[picture.png]]` or `![](picture.png)`: what it shows.
    Embed(String),
    Text,
}

pub(super) fn line_kind(line: &str) -> LineKind {
    let t = line.trim_start();
    let indent = (line.len() - t.len()) / 2;
    if t.is_empty() {
        return LineKind::Blank;
    }
    if t.starts_with("```") {
        return LineKind::Fence;
    }
    if let Some((l, _)) = crate::journal::heading(line) {
        return LineKind::Heading(l);
    }
    if matches!(t, "---" | "***" | "___") {
        return LineKind::Rule;
    }
    if let Some(rest) = t.strip_prefix("> ").or(t.strip_prefix(">")) {
        let r = rest.trim_start();
        if let Some(k) = r.strip_prefix("[!") {
            if let Some(end) = k.find(']') {
                return LineKind::Callout(k[..end].to_string());
            }
        }
        return LineKind::Quote;
    }
    if t.starts_with('|') && t.trim_end().ends_with('|') {
        return LineKind::Table;
    }
    let te = t.trim_end();
    if let Some(inner) = te.strip_prefix("![[").and_then(|r| r.strip_suffix("]]")) {
        if !inner.is_empty() && !inner.contains("]]") {
            return LineKind::Embed(inner.split('|').next().unwrap_or(inner).trim().to_string());
        }
    }
    if let Some(r) = te.strip_prefix("![") {
        if let Some((_, url)) = r.split_once("](") {
            if let Some(u) = url.strip_suffix(')') {
                if !u.is_empty() && !u.contains(')') {
                    return LineKind::Embed(u.trim().to_string());
                }
            }
        }
    }
    for m in ["- ", "* ", "+ "] {
        if let Some(rest) = t.strip_prefix(m) {
            let task = if rest.starts_with("[ ] ") || rest == "[ ]" {
                Some(false)
            } else if rest.starts_with("[x] ") || rest.starts_with("[X] ") || rest == "[x]" {
                Some(true)
            } else {
                None
            };
            return LineKind::Item(indent, m.to_string(), task);
        }
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && t[digits..].starts_with(". ") {
        return LineKind::Item(indent, t[..digits + 2].to_string(), None);
    }
    LineKind::Text
}

/// Live preview: the note's own text, every character kept, laid out as it
/// will read (headings big, bold bold, links purple), the marks faded.
pub(super) fn live_job(text: &str, st: MdStyle) -> LayoutJob {
    let mut job = LayoutJob::default();
    let (_, body_at) = crate::journal::front_matter(text);
    // properties: keys faded
    if body_at > 0 {
        for line in text[..body_at].split_inclusive('\n') {
            let f = TextFormat { font_id: FontId::monospace(st.size * 0.85), color: MUTED(), ..Default::default() };
            job.append(line, 0.0, f);
        }
    }
    let mut in_code = false;
    for raw in text[body_at..].split_inclusive('\n') {
        let line = raw.trim_end_matches(['\n', '\r']);
        let nl = &raw[line.len()..];
        let base = TEXT();
        let kind = line_kind(line);
        if kind == LineKind::Fence || in_code {
            if kind == LineKind::Fence {
                in_code = !in_code;
            }
            let f = TextFormat { font_id: FontId::monospace(st.size * 0.9), color: if kind == LineKind::Fence { MUTED() } else { theme::blend(base, st.accent, 0.3) }, background: theme::blend(CARD(), TEXT(), 0.05), ..Default::default() };
            job.append(raw, 0.0, f);
            continue;
        }
        let mut size = st.size;
        let mut bold_all = false;
        let mut color = base;
        let mut rest = line;
        let mut italic_all = false;
        let mut strike_all = false;
        match &kind {
            LineKind::Heading(l) => {
                size = st.size * heading_scale(*l);
                bold_all = true;
                let n = *l as usize + 1;
                job.append(&line[..n], 0.0, fmt_for(Fx { marker: true, ..Fx::default() }, size * 0.7, base, st.accent, false));
                rest = &line[n..];
            }
            LineKind::Item(_, m, task) => {
                let lead = line.len() - line.trim_start().len();
                let mut n = lead + m.len();
                if task.is_some() {
                    n += 4.min(line.len() - n);
                }
                let n = n.min(line.len());
                let mut f = fmt_for(Fx::default(), st.size, st.accent, st.accent, true);
                f.color = if *task == Some(true) { kit::C_GREEN } else { st.accent };
                job.append(&line[..n], 0.0, f);
                rest = &line[n..];
                if *task == Some(true) {
                    color = MUTED();
                    strike_all = true;
                }
            }
            LineKind::Quote | LineKind::Callout(_) => {
                let n = line.find('>').map_or(0, |i| i + 1);
                let n = if line[n..].starts_with(' ') { n + 1 } else { n };
                let c = match &kind {
                    LineKind::Callout(k) => callout_color(k),
                    _ => MUTED(),
                };
                job.append(&line[..n], 0.0, TextFormat { font_id: crate::fonts::sans_bold_font(st.size), color: c, ..Default::default() });
                rest = &line[n..];
                if let LineKind::Callout(k) = &kind {
                    color = callout_color(k);
                    bold_all = true;
                } else {
                    italic_all = true;
                    color = theme::blend(base, MUTED(), 0.4);
                }
            }
            LineKind::Rule => {
                job.append(raw, 0.0, TextFormat { font_id: crate::fonts::sans_font(st.size), color: LINE(), ..Default::default() });
                continue;
            }
            LineKind::Table => {
                for s in inline(line) {
                    let mut f = fmt_for(s.fx, st.size * 0.95, base, st.accent, false);
                    if !s.fx.marker && s.text.contains('|') {
                        // the pipes, faded
                        for (k, part) in s.text.split('|').enumerate() {
                            if k > 0 {
                                job.append("|", 0.0, TextFormat { font_id: f.font_id.clone(), color: LINE(), ..Default::default() });
                            }
                            job.append(part, 0.0, f.clone());
                        }
                        continue;
                    }
                    f.font_id.size = st.size * 0.95;
                    job.append(&s.text, 0.0, f);
                }
                job.append(nl, 0.0, fmt_for(Fx::default(), st.size, base, st.accent, false));
                continue;
            }
            _ => {}
        }
        for s in inline(rest) {
            let mut fx = s.fx;
            fx.italic |= italic_all && !fx.marker;
            fx.strike |= strike_all && !fx.marker;
            let f = fmt_for(fx, size, color, st.accent, bold_all);
            job.append(&s.text, 0.0, f);
        }
        job.append(nl, 0.0, fmt_for(Fx::default(), size, base, st.accent, false));
    }
    job
}

/// Something clicked in the Reading view.
pub(super) enum Clicked {
    /// A `[[link]]` (a note's name) or a web address.
    Link(String),
    Tag(String),
    /// A tick on this line (counted from the top of the whole note).
    Toggle(usize),
}

/// One paragraph-ish line's galley, its links and tags by char range.
fn line_galley(ui: &egui::Ui, segs: &[Seg], size: f32, color: Color32, accent: Color32, bold_all: bool, italic_all: bool, strike_all: bool, width: f32) -> (std::sync::Arc<egui::Galley>, Vec<(std::ops::Range<usize>, Clicked)>) {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let mut hits = Vec::new();
    let mut at = 0usize;
    for s in segs.iter().filter(|s| !s.fx.marker) {
        let mut fx = s.fx;
        fx.italic |= italic_all;
        fx.strike |= strike_all;
        let mut f = fmt_for(fx, size, color, accent, bold_all);
        f.line_height = Some(size * 1.55);
        job.append(&s.text, 0.0, f);
        let n = s.text.chars().count();
        if let Some(t) = &s.target {
            if s.fx.link {
                hits.push((at..at + n, Clicked::Link(t.clone())));
            } else if s.fx.tag {
                hits.push((at..at + n, Clicked::Tag(t.clone())));
            }
        }
        at += n;
    }
    (ui.fonts(|f| f.layout_job(job)), hits)
}

/// Draw a galley and report a click on one of its links or tags.
fn show_galley(ui: &mut egui::Ui, g: std::sync::Arc<egui::Galley>, hits: Vec<(std::ops::Range<usize>, Clicked)>, indent: f32) -> Option<Clicked> {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), g.size().y), Sense::click());
    let origin = r.min + vec2(indent, 0.0);
    let over = resp.hover_pos().and_then(|p| {
        let cur = g.cursor_from_pos(p - origin);
        let i = cur.ccursor.index;
        hits.iter().position(|(rg, _)| rg.contains(&i) || (i > 0 && rg.contains(&(i - 1)) && rg.end == i))
    });
    ui.painter().galley(origin, g, TEXT());
    if over.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.clicked() {
        if let Some(k) = over {
            return hits.into_iter().nth(k).map(|(_, c)| c);
        }
    }
    None
}

/// Draws an embedded picture (or note) in the Reading view: (ui, what, width).
pub(super) type Embed<'a> = &'a mut dyn FnMut(&mut egui::Ui, &str, f32) -> Option<Clicked>;

/// The Reading view of a note's body (properties are shown by the page).
pub(super) fn reading(ui: &mut egui::Ui, text: &str, st: MdStyle, embed: Embed) -> Option<Clicked> {
    let (_, body_at) = crate::journal::front_matter(text);
    let first_line = text[..body_at].lines().count();
    let lines: Vec<&str> = text[body_at..].lines().collect();
    let width = ui.available_width();
    ui.spacing_mut().item_spacing.y = 4.0;
    let mut out: Option<Clicked> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        match line_kind(line) {
            LineKind::Blank => {
                ui.add_space(st.size * 0.55);
                i += 1;
            }
            LineKind::Heading(l) => {
                ui.add_space(if l <= 2 { st.size * 0.9 } else { st.size * 0.5 });
                let (_, t) = crate::journal::heading(line).unwrap_or((l, line));
                let (g, hits) = line_galley(ui, &inline(t), st.size * heading_scale(l), TEXT(), st.accent, true, false, false, width);
                out = out.or(show_galley(ui, g, hits, 0.0));
                if l <= 2 {
                    let (r, _) = ui.allocate_exact_size(vec2(width, 6.0), Sense::hover());
                    if l == 1 {
                        ui.painter().hline(r.x_range(), r.center().y, Stroke::new(1.0f32, LINE()));
                    }
                }
                i += 1;
            }
            LineKind::Rule => {
                let (r, _) = ui.allocate_exact_size(vec2(width, st.size * 1.4), Sense::hover());
                ui.painter().hline(r.x_range(), r.center().y, Stroke::new(1.0f32, LINE()));
                i += 1;
            }
            LineKind::Fence => {
                let mut code = Vec::new();
                i += 1;
                while i < lines.len() && line_kind(lines[i]) != LineKind::Fence {
                    code.push(lines[i]);
                    i += 1;
                }
                i += 1;
                egui::Frame::none().fill(theme::blend(CARD(), TEXT(), 0.05)).rounding(10.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
                    ui.set_width(width - 24.0);
                    ui.label(egui::RichText::new(code.join("\n")).font(FontId::monospace(st.size * 0.9)).color(theme::blend(TEXT(), st.accent, 0.25)));
                });
                ui.add_space(6.0);
            }
            LineKind::Quote | LineKind::Callout(_) => {
                // the whole block of `>` lines
                let kind = line_kind(line);
                let start = i;
                while i < lines.len() && matches!(line_kind(lines[i]), LineKind::Quote | LineKind::Callout(_)) {
                    i += 1;
                }
                let strip = |l: &str| -> String {
                    let t = l.trim_start();
                    let t = t.strip_prefix('>').unwrap_or(t);
                    t.strip_prefix(' ').unwrap_or(t).to_string()
                };
                let col = match &kind {
                    LineKind::Callout(k) => callout_color(k),
                    _ => MUTED(),
                };
                let fill = match &kind {
                    LineKind::Callout(_) => col.gamma_multiply(0.10),
                    _ => Color32::TRANSPARENT,
                };
                let resp = egui::Frame::none().fill(fill).rounding(egui::Rounding { nw: 0.0, sw: 0.0, ne: 10.0, se: 10.0 }).inner_margin(egui::Margin { left: 16.0, right: 12.0, top: 8.0, bottom: 8.0 }).show(ui, |ui| {
                    ui.set_width(width - 28.0);
                    let mut got = None;
                    for (k, l) in lines[start..i].iter().enumerate() {
                        let mut t = strip(l);
                        let mut bold = false;
                        let mut c = theme::blend(TEXT(), MUTED(), 0.3);
                        if k == 0 {
                            if let LineKind::Callout(kd) = &kind {
                                let after = t.find(']').map_or(String::new(), |e| t[e + 1..].trim().to_string());
                                let title = if after.is_empty() {
                                    let mut s = kd.clone();
                                    if let Some(f) = s.get_mut(0..1) {
                                        f.make_ascii_uppercase();
                                    }
                                    s
                                } else {
                                    after
                                };
                                let icon = match kd.to_lowercase().as_str() {
                                    "tip" | "hint" | "idea" => "💡",
                                    "warning" | "caution" | "danger" | "bug" => "⚠",
                                    "success" | "done" | "check" => "✔",
                                    "question" | "faq" | "help" => "❓",
                                    "shiny" | "hunt" => "✨",
                                    "pokedex" => "📕",
                                    "move" => "💥",
                                    "item" => "🎒",
                                    "place" => "🗺",
                                    "guide" => "📚",
                                    _ => "📝",
                                };
                                t = format!("{icon}  {title}");
                                bold = true;
                                c = col;
                            }
                        }
                        let italic = matches!(kind, LineKind::Quote);
                        let (g, hits) = line_galley(ui, &inline(&t), st.size, c, st.accent, bold, italic, false, width - 28.0);
                        got = got.or(show_galley(ui, g, hits, 0.0));
                    }
                    got
                });
                let r = resp.response.rect;
                ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 1.5, col);
                out = out.or(resp.inner);
                ui.add_space(6.0);
            }
            LineKind::Table => {
                let start = i;
                while i < lines.len() && line_kind(lines[i]) == LineKind::Table {
                    i += 1;
                }
                let rows: Vec<Vec<String>> = lines[start..i]
                    .iter()
                    .filter(|l| !l.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')))
                    .map(|l| l.trim().trim_matches('|').split('|').map(|c| c.trim().to_string()).collect())
                    .collect();
                let cols = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                let got_t = egui::Frame::none().stroke(Stroke::new(1.0f32, LINE())).rounding(10.0).inner_margin(egui::Margin::same(2.0)).show(ui, |ui| {
                    let mut got_t: Option<Clicked> = None;
                    egui::Grid::new(("md-table", start)).num_columns(cols).spacing([18.0, 8.0]).min_col_width((width - 40.0) / cols as f32 - 18.0).striped(true).show(ui, |ui| {
                        for (k, r) in rows.iter().enumerate() {
                            for c in 0..cols {
                                let cell = r.get(c).cloned().unwrap_or_default();
                                let (g, hits) = line_galley(ui, &inline(&cell), st.size * 0.95, if k == 0 { MUTED() } else { TEXT() }, st.accent, k == 0, false, false, (width - 40.0) / cols as f32);
                                let got = show_galley(ui, g, hits, 6.0);
                                if got_t.is_none() {
                                    got_t = got;
                                }
                            }
                            ui.end_row();
                        }
                    });
                    got_t
                }).inner;
                out = out.or(got_t);
                ui.add_space(6.0);
            }
            LineKind::Item(indent, marker, task) => {
                let lead = line.len() - line.trim_start().len();
                let mut body = &line[lead + marker.len()..];
                if task.is_some() {
                    body = body.get(3..).unwrap_or("").trim_start();
                }
                let x0 = 6.0 + indent as f32 * 22.0;
                let line_no = first_line + i;
                let (g, hits) = line_galley(ui, &inline(body), st.size, if task == Some(true) { MUTED() } else { TEXT() }, st.accent, false, false, task == Some(true), width - x0 - 26.0);
                let h = g.size().y.max(st.size * 1.5);
                let (r, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
                let c = pos2(r.min.x + x0 + 8.0, r.min.y + st.size * 0.78);
                match task {
                    Some(done) => {
                        let br = Rect::from_center_size(c, vec2(st.size, st.size));
                        let resp = ui.interact(br.expand(3.0), ui.id().with(("md-tick", line_no)), Sense::click());
                        if done {
                            ui.painter().rect_filled(br, 4.0, st.accent);
                            ui.painter().text(br.center(), Align2::CENTER_CENTER, "✔", crate::fonts::sans_bold_font(st.size * 0.7), Color32::WHITE);
                        } else {
                            ui.painter().rect_stroke(br, 4.0, Stroke::new(1.5f32, if resp.hovered() { st.accent } else { MUTED() }));
                        }
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            out = out.or(Some(Clicked::Toggle(line_no)));
                        }
                    }
                    None => {
                        if marker.trim_end().ends_with('.') {
                            ui.painter().text(c, Align2::CENTER_CENTER, marker.trim(), crate::fonts::sans_font(st.size * 0.9), MUTED());
                        } else {
                            ui.painter().circle_filled(c, st.size * 0.17, st.accent);
                        }
                    }
                }
                // the text beside it
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(r.min.x + x0 + 24.0, r.min.y), vec2(width - x0 - 24.0, h))));
                out = out.or(show_galley(&mut child, g, hits, 0.0));
                i += 1;
            }
            LineKind::Embed(what) => {
                let got = embed(ui, &what, width);
                if out.is_none() {
                    out = got;
                }
                i += 1;
            }
            LineKind::Text => {
                let (g, hits) = line_galley(ui, &inline(line), st.size, TEXT(), st.accent, false, false, false, width);
                out = out.or(show_galley(ui, g, hits, 0.0));
                i += 1;
            }
        }
    }
    out
}

/// Tick or untick the task on line `n` of `text`.
pub(super) fn toggle_task(text: &str, n: usize) -> String {
    let mut out = String::with_capacity(text.len());
    for (k, l) in text.split_inclusive('\n').enumerate() {
        if k == n {
            let swapped = if l.contains("[ ]") {
                l.replacen("[ ]", "[x]", 1)
            } else if l.contains("[x]") {
                l.replacen("[x]", "[ ]", 1)
            } else {
                l.replacen("[X]", "[ ]", 1)
            };
            out.push_str(&swapped);
        } else {
            out.push_str(l);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(segs: &[Seg]) -> String {
        segs.iter().filter(|s| !s.fx.marker).map(|s| s.text.as_str()).collect()
    }
    fn all(segs: &[Seg]) -> String {
        segs.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn inline_keeps_every_character_and_reads_marks() {
        for line in [
            "Sweet Scent on **rare grass**, see [[Route 214|the route]] and #gible.",
            "a *b* _c_ ~~d~~ ==e== `f` [g](https://x.y) snake_case_name 2*3",
            "unclosed ** bold and [[ link",
            "#1 isn't a tag but #tag is",
        ] {
            assert_eq!(all(&inline(line)), line, "nothing dropped from {line}");
        }
        let s = inline("Sweet Scent on **rare grass**, see [[Route 214|the route]] and #gible.");
        assert_eq!(plain(&s), "Sweet Scent on rare grass, see the route and #gible.");
        assert!(s.iter().any(|x| x.fx.bold && x.text == "rare grass"));
        assert!(s.iter().any(|x| x.fx.link && x.target.as_deref() == Some("Route 214")));
        assert!(s.iter().any(|x| x.fx.tag && x.target.as_deref() == Some("gible")));
        let s = inline("snake_case_name");
        assert!(s.iter().all(|x| !x.fx.italic));
        let s = inline("#1 isn't a tag but #tag is");
        assert_eq!(s.iter().filter(|x| x.fx.tag).count(), 1);
    }

    #[test]
    fn line_kinds() {
        assert_eq!(line_kind("## Tonight"), LineKind::Heading(2));
        assert_eq!(line_kind("- [ ] Try it"), LineKind::Item(0, "- ".into(), Some(false)));
        assert_eq!(line_kind("  - [x] done"), LineKind::Item(1, "- ".into(), Some(true)));
        assert_eq!(line_kind("12. twelfth"), LineKind::Item(0, "12. ".into(), None));
        assert_eq!(line_kind("> [!tip] Night only"), LineKind::Callout("tip".into()));
        assert_eq!(line_kind("> said"), LineKind::Quote);
        assert_eq!(line_kind("| a | b |"), LineKind::Table);
        assert_eq!(line_kind("---"), LineKind::Rule);
        assert_eq!(line_kind("#tag at start"), LineKind::Text);
        assert_eq!(line_kind("![[Attachments/Gible 2412.png]]"), LineKind::Embed("Attachments/Gible 2412.png".into()));
        assert_eq!(line_kind("![[Gible.png|300]]"), LineKind::Embed("Gible.png".into()));
        assert_eq!(line_kind("![shiny](Attachments/x.png)"), LineKind::Embed("Attachments/x.png".into()));
        assert_eq!(line_kind("![[a]] and [[b]]"), LineKind::Text);
    }

    #[test]
    fn ticking_a_task_changes_only_its_line() {
        let t = "# A\n- [ ] one\n- [x] two\n";
        assert_eq!(toggle_task(t, 1), "# A\n- [x] one\n- [x] two\n");
        assert_eq!(toggle_task(t, 2), "# A\n- [ ] one\n- [ ] two\n");
    }
}
