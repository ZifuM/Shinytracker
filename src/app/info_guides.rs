//! Game Info › How to Play: the guides library (a path for new players, the
//! latest update, every topic, and a search through all the guides' text)
//! and the guide reader (contents that follow you down the page, keys and
//! tables set properly, links that open in the app, read ticks).

use super::info_home::Page;
use super::info_md::{self as md, Block, Doc};
use super::profile_page::card_head;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::theme;
use crate::wiki::{self, Guide, Wiki};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::collections::HashMap;
use std::sync::Arc;

/// Guides shown as a stack of fold-out parts (they're long, one part per version).
const FOLDED: [&str; 2] = ["whats-new", "known-bugs"];

/// Words read per minute, for "x min read".
const WPM: usize = 200;

pub(super) fn topic_meta(group: &str) -> (&'static str, Color32) {
    match group {
        "Getting started" => ("🚩", kit::C_GREEN),
        "Playing together" => ("👥", kit::C_PINK),
        "Online" => ("🌐", kit::C_CYAN),
        "Game features" => ("✨", kit::C_VIOLET),
        "Challenges" => ("🏆", kit::C_ORANGE),
        "Help" => ("❓", kit::C_SLATE),
        _ => ("📚", kit::C_BLUE),
    }
}

pub(super) fn guide_icon(name: &str) -> &'static str {
    match name {
        "bag" => "🎒",
        "spark" => "✨",
        "download" => "⬇",
        "map" => "🗺",
        "flag" => "🚩",
        "pad" => "🎮",
        "people" => "👥",
        "globe" => "🌐",
        "badge" => "🏅",
        "book" => "📖",
        "heart" => "❤",
        "gear" => "⚙",
        "ball" => "🔴",
        "save" => "💾",
        "bug" => "🐛",
        _ => "❓",
    }
}

fn read_time(words: usize) -> String {
    format!("{} min read", (words / WPM).max(1))
}

fn small(t: impl Into<String>) -> RichText {
    RichText::new(t).font(crate::fonts::sans_font(12.0)).color(MUTED())
}

/// Parsed guides, per wiki copy.
#[derive(Default)]
pub(super) struct GuideCache {
    key: usize,
    docs: HashMap<String, Arc<Doc>>,
}

/// The reader's state.
#[derive(Default)]
pub(super) struct ReaderState {
    /// Library: text to look for in every guide.
    pub query: String,
    /// Reader: words to mark (from a library search).
    pub find: Option<String>,
    /// Reader: jump to this section next frame.
    pub scroll_to: Option<usize>,
    /// Folded guides: the open parts (guide id, part's section index).
    pub open: Vec<(String, usize)>,
    /// Block heights from the last frame, for the open guide and width.
    heights: (String, i32, Vec<f32>),
    /// The contents panel's height last frame.
    toc_h: f32,
    /// Narrow windows: contents shown.
    toc_open: bool,
    /// The guide on screen and when it was opened (seconds, app clock).
    since: (String, f64),
    cache: GuideCache,
}

/// A search hit inside a guide.
struct TextHit {
    guide: String,
    section: Option<usize>,
    section_title: String,
    snippet: String,
}

/// ~140 characters around the first match.
fn snippet(text: &str, q: &str) -> Option<String> {
    let lower = text.to_lowercase();
    if lower.len() != text.len() {
        return text.to_lowercase().contains(q).then(|| text.chars().take(140).collect());
    }
    let at = lower.find(q)?;
    let mut start = at.saturating_sub(60);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (at + q.len() + 80).min(text.len());
    while !text.is_char_boundary(end) {
        end += 1;
    }
    // whole words
    if start > 0 {
        if let Some(sp) = text[start..at].find(' ') {
            start += sp + 1;
        }
    }
    if end < text.len() {
        if let Some(sp) = text[at..end].rfind(' ') {
            end = at + sp;
        }
    }
    let mut s = String::new();
    if start > 0 {
        s.push('…');
    }
    s.push_str(text[start..end].trim());
    if end < text.len() {
        s.push('…');
    }
    Some(s)
}

/// Plain text of a block (for searching).
fn block_text(b: &Block) -> String {
    match b {
        Block::Heading { spans, .. } | Block::Para(spans) => md::plain(spans),
        Block::Quote(ps) => ps.iter().map(|p| md::plain(p)).collect::<Vec<_>>().join(" "),
        Block::List(items) => items.iter().map(|i| md::plain(&i.2)).collect::<Vec<_>>().join(" · "),
        Block::Table { head, rows } => std::iter::once(head)
            .chain(rows.iter())
            .map(|r| r.iter().map(|c| md::plain(c)).collect::<Vec<_>>().join(" | "))
            .collect::<Vec<_>>()
            .join(" · "),
        Block::Code(t) => t.clone(),
        Block::Rule => String::new(),
    }
}

impl App {
    /// The parsed guide (cached per wiki copy).
    pub(super) fn guide_doc(&mut self, w: &Arc<Wiki>, id: &str) -> Option<Arc<Doc>> {
        let key = Arc::as_ptr(w) as usize;
        let c = &mut self.setup.info.reader.cache;
        if c.key != key {
            c.key = key;
            c.docs.clear();
        }
        if let Some(d) = c.docs.get(id) {
            return Some(d.clone());
        }
        let g = w.guide(id)?;
        let d = Arc::new(md::parse(&g.body));
        c.docs.insert(id.to_string(), d.clone());
        Some(d)
    }

    fn all_docs(&mut self, w: &Arc<Wiki>) -> Vec<(Guide, Arc<Doc>)> {
        let guides: Vec<Guide> = w.guides_sorted().into_iter().cloned().collect();
        guides.into_iter().filter_map(|g| self.guide_doc(w, &g.id).map(|d| (g, d))).collect()
    }

    fn guide_read(&self, id: &str) -> bool {
        self.cfg.guides_read.iter().any(|g| g == id)
    }

    fn set_guide_read(&mut self, id: &str, read: bool) {
        let has = self.guide_read(id);
        if read && !has {
            self.cfg.guides_read.push(id.to_string());
            self.cfg.save();
        } else if !read && has {
            self.cfg.guides_read.retain(|g| g != id);
            self.cfg.save();
        }
    }

    /// Open a guide (at a section, with words marked).
    pub(super) fn info_read(&mut self, id: &str, section: Option<usize>, find: Option<String>) {
        self.info_push(Page::Guide(id.to_string()));
        let r = &mut self.setup.info.reader;
        r.scroll_to = section;
        r.find = find;
        r.toc_open = false;
        if let Some(s) = section {
            // a folded part opens when you jump to it
            if FOLDED.contains(&id) && !r.open.iter().any(|o| o.0 == id && o.1 == s) {
                r.open.push((id.to_string(), s));
            }
        }
    }

    /// A link in a guide: another guide, a Pokémon, an item, or a web page.
    pub(super) fn info_follow(&mut self, w: &Arc<Wiki>, url: &str, ctx: &egui::Context) {
        if let Some(id) = url.strip_prefix("#/guide/") {
            let id = id.split('#').next().unwrap_or(id).to_string();
            if w.guide(&id).is_some() {
                self.info_read(&id, None, None);
            }
        } else if let Some(slug) = url.strip_prefix("#/pokemon/") {
            if let Some(m) = w.data.species.iter().find(|m| wiki::slug(&m.name) == slug) {
                let id = m.id.clone();
                self.info_open_dex(Some(id));
            }
        } else if let Some(slug) = url.strip_prefix("#/item/") {
            self.info_open_item_slug(w, slug);
        } else if url.starts_with("http") {
            ctx.open_url(egui::OpenUrl::new_tab(url));
        } else if let Some(rest) = url.strip_prefix("#/") {
            ctx.open_url(egui::OpenUrl::new_tab(format!("https://platinumtogether.online/wiki/#/{rest}")));
        }
    }

    // ───────────────────────────── library ─────────────────────────────

    pub(super) fn page_guides(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        self.info_back_bar(ui, "How to Play");
        let docs = self.all_docs(w);
        let total_words: usize = docs.iter().map(|d| d.1.words).sum();
        let read = docs.iter().filter(|(g, _)| self.guide_read(&g.id)).count();
        let topics: Vec<String> = {
            let mut t: Vec<String> = Vec::new();
            for (g, _) in &docs {
                if !t.contains(&g.group) {
                    t.push(g.group.clone());
                }
            }
            t
        };

        // ── header: title, progress, search
        let pl = theme::pal();
        let full = ui.available_width();
        let slot = ui.painter().add(egui::Shape::Noop);
        let searching = self.setup.info.reader.query.trim().chars().count() >= 2;
        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, if searching { 16.0 } else { 22.0 })).show(ui, |ui| {
            ui.set_width(full - 56.0);
            if !searching {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("HOW TO PLAY").font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(MUTED(), kit::C_CYAN, 0.6)));
                        ui.add_space(4.0);
                        ui.label(RichText::new("Everything the game adds, explained").font(crate::fonts::sans_bold_font(24.0)).color(TEXT()));
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(format!(
                                "{} guides in {} topics, about {} hours of reading: playing with friends, going online, challenges and fixes.",
                                docs.len(),
                                topics.len(),
                                ((total_words / WPM) as f32 / 60.0).round().max(1.0)
                            ))
                            .size(13.0)
                            .color(MUTED()),
                        );
                    });
                    // read progress, right
                    if ui.available_width() > 170.0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (r, _) = ui.allocate_exact_size(egui::vec2(150.0, 64.0), Sense::hover());
                            let p = ui.painter();
                            let f = if docs.is_empty() { 0.0 } else { read as f32 / docs.len() as f32 };
                            p.text(egui::pos2(r.max.x, r.min.y + 14.0), Align2::RIGHT_CENTER, format!("{read} of {} read", docs.len()), crate::fonts::sans_bold_font(14.0), TEXT());
                            let bar = Rect::from_min_size(egui::pos2(r.min.x, r.min.y + 32.0), egui::vec2(r.width(), 8.0));
                            p.rect_filled(bar, 4.0, kit::tint(LINE(), 0.9));
                            let gf = kit::grow(ui.ctx(), egui::Id::new("guides-read-bar"), f);
                            p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bar.width() * gf, 8.0)), 4.0, kit::C_GREEN);
                            p.text(egui::pos2(r.max.x, r.min.y + 54.0), Align2::RIGHT_CENTER, "ticks when you reach the end", crate::fonts::sans_font(10.5), MUTED());
                        });
                    }
                });
                ui.add_space(14.0);
            }
            let width = ui.available_width();
            kit::search_box_keyed(ui, Some(egui::Id::new("guides-search")), &mut self.setup.info.reader.query, "Search inside every guide, e.g. trade, Ctrl+R, shiny, autosave", width);
        });
        let rect = frame.response.rect;
        let a = theme::blend(CARD(), kit::C_CYAN, if pl.light { 0.12 } else { 0.20 });
        let b = theme::blend(CARD(), pl.accent, if pl.light { 0.06 } else { 0.10 });
        ui.painter().set(
            slot,
            egui::Shape::Vec(vec![
                kit::card_shadow().as_shape(rect, 18.0).into(),
                theme::gradient_rect(rect, [18.0; 4], a, b),
                egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_CYAN, 0.35))),
            ]),
        );
        ui.add_space(18.0);

        if searching {
            self.guides_search_results(ui, w, &docs);
            return;
        }
        self.guides_path(ui, &docs);
        ui.add_space(18.0);
        self.guides_latest(ui, w);
        for t in &topics {
            if t == "Getting started" {
                continue;
            }
            ui.add_space(22.0);
            let list: Vec<&(Guide, Arc<Doc>)> = docs.iter().filter(|(g, _)| &g.group == t).collect();
            self.guides_topic(ui, t, &list);
        }
    }

    /// New here? The "Getting started" guides as numbered steps.
    fn guides_path(&mut self, ui: &mut egui::Ui, docs: &[(Guide, Arc<Doc>)]) {
        let steps: Vec<&(Guide, Arc<Doc>)> = docs.iter().filter(|(g, _)| g.group == "Getting started").collect();
        if steps.is_empty() {
            return;
        }
        let next = steps.iter().position(|(g, _)| !self.guide_read(&g.id));
        let mut open: Option<String> = None;
        kit::card(ui, |ui| {
            let (icon, col) = topic_meta("Getting started");
            let note = match next {
                Some(i) => format!("Next up: step {}", i + 1),
                None => "All done ✔".to_string(),
            };
            ui.horizontal(|ui| {
                card_head(ui, icon, col, "New here? Start with these", None);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(small(note));
                });
            });
            ui.add_space(12.0);
            let avail = ui.available_width();
            let cols = if avail >= 1100.0 { 4 } else if avail >= 640.0 { 3 } else { 2 };
            let gap = 10.0;
            let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
            for (ri, row) in steps.chunks(cols).enumerate() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (ci, (g, d)) in row.iter().enumerate() {
                        let i = ri * cols + ci;
                        let done = self.guide_read(&g.id);
                        let is_next = next == Some(i);
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 74.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter_at(r.expand(2.0));
                        let edge = if is_next { col } else { LINE() };
                        p.rect_filled(r, 14.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                        p.rect_stroke(r, 14.0, Stroke::new(if is_next { 1.5f32 } else { 1.0 }, theme::blend(edge, col, 0.4 * ht)));
                        // the step's badge: number, or a tick
                        let c = egui::pos2(r.min.x + 26.0, r.center().y);
                        if done {
                            p.circle_filled(c, 13.0, kit::C_GREEN);
                            p.text(c, Align2::CENTER_CENTER, "✔", crate::fonts::sans_bold_font(12.0), Color32::WHITE);
                        } else {
                            p.circle_filled(c, 13.0, if is_next { col } else { kit::tint(col, 0.22) });
                            p.text(c, Align2::CENTER_CENTER, (i + 1).to_string(), crate::fonts::sans_bold_font(12.5), if is_next { Color32::WHITE } else { TEXT() });
                        }
                        let x = r.min.x + 50.0;
                        let maxw = r.max.x - x - 10.0;
                        let title = if g.short.is_empty() { &g.title } else { &g.short };
                        let f = crate::fonts::sans_bold_font(13.5);
                        p.text(egui::pos2(x, r.center().y - 9.0), Align2::LEFT_CENTER, super::charts::clip(ui, title, &f, maxw), f, TEXT());
                        let sub = if is_next { format!("Up next · {}", read_time(d.words)) } else { read_time(d.words) };
                        p.text(egui::pos2(x, r.center().y + 10.0), Align2::LEFT_CENTER, sub, crate::fonts::sans_font(11.5), if is_next { col } else { MUTED() });
                        if resp.on_hover_text(&g.summary).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = Some(g.id.clone());
                        }
                    }
                });
                ui.add_space(gap);
            }
        });
        if let Some(id) = open {
            self.info_read(&id, None, None);
        }
    }

    /// The newest version's headline and highlights.
    fn guides_latest(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let Some(doc) = self.guide_doc(w, "whats-new") else { return };
        let Some(first) = doc.sections.iter().position(|s| s.level == 2) else { return };
        let sec = &doc.sections[first];
        let end = doc.sections.iter().skip(first + 1).find(|s| s.level == 2).map(|s| s.block).unwrap_or(doc.blocks.len());
        // intro line and the first list (the highlights)
        let intro = doc.blocks[sec.block + 1..end].iter().find_map(|b| match b {
            Block::Para(s) => Some(md::plain(s)),
            _ => None,
        });
        let highlights: Vec<String> = doc.blocks[sec.block + 1..end]
            .iter()
            .find_map(|b| match b {
                Block::List(items) => Some(items.iter().filter(|i| i.0 == 0).map(|i| md::plain(&i.2)).collect()),
                _ => None,
            })
            .unwrap_or_default();
        let (version, title) = match sec.title.split_once(':') {
            Some((v, t)) => (v.trim().to_string(), t.trim().to_string()),
            None => (sec.title.clone(), String::new()),
        };
        let mut open = false;
        let mut all = false;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
                kit::icon_chip(ui.painter(), r, "✨", kit::C_VIOLET, false);
                ui.add_space(4.0);
                ui.label(RichText::new("Latest update").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.add_space(4.0);
                tag(ui, &version, kit::C_VIOLET);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    all = ui.link(RichText::new("Every version ›").size(12.5)).clicked();
                });
            });
            ui.add_space(10.0);
            if !title.is_empty() {
                ui.label(RichText::new(capitalise(&title)).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.add_space(4.0);
            }
            if let Some(i) = &intro {
                ui.add(egui::Label::new(RichText::new(i).size(13.0).color(MUTED())).wrap());
                ui.add_space(8.0);
            }
            // highlights: the bold lead of each, two columns when wide
            let leads: Vec<String> = highlights
                .iter()
                .map(|h| {
                    let lead = h.split(['.', ',']).next().unwrap_or(h).trim();
                    lead.chars().take(70).collect()
                })
                .take(6)
                .collect();
            let cols = if ui.available_width() >= 700.0 { 2 } else { 1 };
            let cw = (ui.available_width() / cols as f32).floor();
            for row in leads.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for l in row {
                        ui.allocate_ui_with_layout(egui::vec2(cw, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.set_width(cw);
                            ui.label(RichText::new("•").strong().color(kit::C_VIOLET));
                            ui.label(RichText::new(l).size(12.5).color(TEXT()));
                        });
                    }
                });
            }
            ui.add_space(10.0);
            open = ui.add(egui::Button::new(RichText::new("Read the patch notes  ›").size(13.0)).fill(kit::btn_fill())).clicked();
        });
        if open {
            self.info_read("whats-new", Some(first), None);
        } else if all {
            self.info_read("whats-new", None, None);
        }
    }

    fn guides_topic(&mut self, ui: &mut egui::Ui, topic: &str, list: &[&(Guide, Arc<Doc>)]) {
        let (icon, col) = topic_meta(topic);
        let read = list.iter().filter(|(g, _)| self.guide_read(&g.id)).count();
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
            kit::icon_chip(ui.painter(), r, icon, col, true);
            ui.add_space(4.0);
            ui.label(RichText::new(topic).font(crate::fonts::sans_bold_font(17.0)).color(TEXT()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(small(format!("{read} of {} read", list.len())));
            });
        });
        ui.add_space(10.0);
        let avail = ui.available_width();
        let cols = if avail >= 1100.0 { 3 } else if avail >= 640.0 { 2 } else { 1 };
        let gap = 14.0;
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut open: Option<String> = None;
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (g, d) in row.iter().map(|x| (&x.0, &x.1)) {
                    let done = self.guide_read(&g.id);
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 112.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let lift = 2.0 * ht;
                    let r2 = r.translate(egui::vec2(0.0, -lift));
                    let p = ui.painter_at(r.expand(6.0));
                    p.add(kit::card_shadow().as_shape(r2, 14.0));
                    p.rect_filled(r2, 14.0, theme::blend(CARD(), CARD_HOVER(), ht));
                    p.rect_stroke(r2, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.5 * ht)));
                    let chip = Rect::from_min_size(r2.min + egui::vec2(14.0, 14.0), egui::vec2(30.0, 30.0));
                    kit::icon_chip(&p, chip, guide_icon(&g.icon), col, false);
                    let x = chip.max.x + 10.0;
                    let maxw = r2.max.x - x - 14.0;
                    let f = crate::fonts::sans_bold_font(14.0);
                    let mut job = egui::text::LayoutJob::simple(g.title.clone(), f, TEXT(), maxw);
                    job.wrap.max_rows = 2;
                    let tg = ui.fonts(|fo| fo.layout_job(job));
                    let th = tg.size().y;
                    p.galley(egui::pos2(x, chip.center().y - th.min(36.0) / 2.0), tg, TEXT());
                    let mut job = egui::text::LayoutJob::simple(g.summary.clone(), crate::fonts::sans_font(11.5), MUTED(), r2.width() - 28.0);
                    job.wrap.max_rows = 2;
                    let sg = ui.fonts(|fo| fo.layout_job(job));
                    p.galley(egui::pos2(r2.min.x + 14.0, chip.max.y + 10.0), sg, MUTED());
                    // footer: time, read
                    let fy = r2.max.y - 14.0;
                    p.text(egui::pos2(r2.min.x + 14.0, fy), Align2::LEFT_CENTER, read_time(d.words), crate::fonts::sans_font(11.0), MUTED());
                    if done {
                        p.text(egui::pos2(r2.max.x - 14.0, fy), Align2::RIGHT_CENTER, "✔ Read", crate::fonts::sans_bold_font(11.0), kit::C_GREEN);
                    }
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some(g.id.clone());
                    }
                }
            });
            ui.add_space(gap);
        }
        if let Some(id) = open {
            self.info_read(&id, None, None);
        }
    }

    /// Search through every guide's text: guides that match, and where.
    fn guides_search_results(&mut self, ui: &mut egui::Ui, _w: &Arc<Wiki>, docs: &[(Guide, Arc<Doc>)]) {
        let q = self.setup.info.reader.query.trim().to_lowercase();
        let mut found: Vec<(Guide, bool, Vec<TextHit>, usize)> = Vec::new();
        for (g, d) in docs {
            let title_hit = g.title.to_lowercase().contains(&q) || g.summary.to_lowercase().contains(&q);
            let mut hits = Vec::new();
            let mut count = 0;
            let mut section: Option<usize> = None;
            for (bi, b) in d.blocks.iter().enumerate() {
                if let Some(si) = d.sections.iter().position(|s| s.block == bi) {
                    section = Some(si);
                }
                let text = block_text(b);
                let lower = text.to_lowercase();
                let n = lower.matches(q.as_str()).count();
                if n == 0 {
                    continue;
                }
                count += n;
                let is_head = matches!(b, Block::Heading { .. });
                if !is_head && hits.len() < 3 && !hits.iter().any(|h: &TextHit| h.section == section) {
                    if let Some(s) = snippet(&text, &q) {
                        let title = section.map(|i| d.sections[i].title.clone()).unwrap_or_else(|| "Introduction".into());
                        hits.push(TextHit { guide: g.id.clone(), section, section_title: title, snippet: s });
                    }
                }
            }
            if title_hit || count > 0 {
                found.push((g.clone(), title_hit, hits, count));
            }
        }
        found.sort_by_key(|f| (!f.1, std::cmp::Reverse(f.3)));
        let mut open: Option<(String, Option<usize>)> = None;
        let total: usize = found.iter().map(|f| f.3).sum();
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} in {}", plural(total, "match", "matches"), plural(found.len(), "guide", "guides"))).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.link(RichText::new("Clear").size(12.5)).clicked() {
                    self.setup.info.reader.query.clear();
                }
            });
        });
        ui.add_space(10.0);
        if found.is_empty() {
            kit::card(ui, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("No guide mentions \"{q}\"")).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.add_space(2.0);
                ui.label(small("Try a shorter word, or a button name like SAVE or TRADE."));
                ui.add_space(6.0);
            });
            return;
        }
        for (g, _, hits, count) in &found {
            let (icon, col) = topic_meta(&g.group);
            kit::card(ui, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
                    kit::icon_chip(ui.painter(), r, guide_icon(&g.icon), col, false);
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(format!("{}  {}", icon, g.group.to_uppercase())).font(crate::fonts::sans_bold_font(9.5)).color(MUTED()));
                        if ui.add(egui::Label::new(RichText::new(&g.title).font(crate::fonts::sans_bold_font(15.0)).color(TEXT())).sense(Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = Some((g.id.clone(), None));
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if *count > 0 {
                            tag(ui, &plural(*count, "match", "matches"), kit::C_GOLD);
                        }
                    });
                });
                for h in hits {
                    ui.add_space(8.0);
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 0.0), Sense::hover());
                    let _ = (r, resp);
                    let inner = egui::Frame::none().fill(kit::sunken()).rounding(10.0).inner_margin(egui::Margin::symmetric(12.0, 8.0)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(format!("§  {}", h.section_title)).font(crate::fonts::sans_bold_font(11.5)).color(col));
                        ui.add_space(2.0);
                        let spans = vec![md::Span { text: h.snippet.clone(), ..Default::default() }];
                        let (gal, _) = md::layout(ui, &spans, md::Style { size: 12.5, color: MUTED(), bold: false }, ui.available_width(), Some(&q));
                        md::show_galley(ui, gal, &[]);
                    });
                    let resp = ui.interact(inner.response.rect, ui.id().with(("hit", &h.guide, h.section)), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_stroke(inner.response.rect, 10.0, Stroke::new(1.0f32, kit::tint(col, 0.6)));
                    }
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some((h.guide.clone(), h.section));
                    }
                }
            });
            ui.add_space(12.0);
        }
        if let Some((id, s)) = open {
            let q = self.setup.info.reader.query.trim().to_string();
            self.info_read(&id, s, Some(q));
        }
    }

    // ───────────────────────────── reader ─────────────────────────────

    pub(super) fn page_guide(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, id: &str) {
        let Some(g) = w.guide(id).cloned() else {
            self.info_back_bar(ui, "How to Play");
            ui.label(small("This guide isn't in the game's data any more."));
            return;
        };
        let Some(doc) = self.guide_doc(w, id) else { return };
        let (icon, col) = topic_meta(&g.group);
        self.info_back_bar(ui, &g.group);

        // ── title
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(52.0, 52.0), Sense::hover());
            kit::icon_chip(ui.painter(), r, guide_icon(&g.icon), col, true);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(format!("{icon}  {}", g.group.to_uppercase())).font(crate::fonts::sans_bold_font(10.5)).color(col));
                ui.add(egui::Label::new(RichText::new(&g.title).font(crate::fonts::sans_bold_font(26.0)).color(TEXT())).wrap());
            });
        });
        ui.add_space(10.0);
        // long summaries (the patch notes list every version) fold to three lines
        {
            let width = ui.available_width().min(860.0);
            let key = egui::Id::new(("guide-summary-open", &g.id));
            let full = ui.ctx().data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
            let mut job = egui::text::LayoutJob::simple(g.summary.clone(), crate::fonts::sans_font(14.5), MUTED(), width);
            if !full {
                job.wrap.max_rows = 3;
            }
            let gal = ui.fonts(|f| f.layout_job(job));
            let cut = gal.elided;
            let (r, _) = ui.allocate_exact_size(gal.size(), Sense::hover());
            ui.painter().galley(r.min, gal, MUTED());
            if cut || full {
                if ui.link(RichText::new(if full { "Show less" } else { "Show all" }).size(12.5)).clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(key, !full));
                }
            }
        }
        ui.add_space(12.0);
        let read = self.guide_read(id);
        let mut toggle_read = false;
        let mut wiki_open = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            tag(ui, &format!("⏱  {}", read_time(doc.words)), kit::C_SLATE);
            let parts = doc.sections.iter().filter(|s| s.level == 2).count();
            if parts > 0 {
                tag(ui, &plural(parts, "part", "parts"), kit::C_SLATE);
            }
            let (t, c) = if read { ("✔  Read", kit::C_GREEN) } else { ("Not read yet", kit::C_SLATE) };
            if tag(ui, t, c)
                .interact(Sense::click())
                .on_hover_text(if read { "Click to mark it unread" } else { "Click to mark it read" })
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                toggle_read = true;
            }
            wiki_open = ui.link(RichText::new("Open on the wiki  ↗").size(12.5)).clicked();
        });
        if toggle_read {
            self.set_guide_read(id, !read);
        }
        if wiki_open {
            ui.ctx().open_url(egui::OpenUrl::new_tab(format!("{}{}", wiki::GUIDE_BASE, g.id)));
        }
        ui.add_space(14.0);

        // ── marked words
        if let Some(f) = self.setup.info.reader.find.clone() {
            let n: usize = doc.blocks.iter().map(|b| block_text(b).to_lowercase().matches(f.to_lowercase().as_str()).count()).sum();
            let mut clear = false;
            egui::Frame::none()
                .fill(theme::blend(CARD(), theme::GOLD, 0.14))
                .stroke(Stroke::new(1.0f32, theme::blend(LINE(), theme::GOLD, 0.45)))
                .rounding(10.0)
                .inner_margin(egui::Margin::symmetric(14.0, 8.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Marking \"{f}\": {}", plural(n, "match", "matches"))).size(13.0).color(TEXT()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            clear = ui.link(RichText::new("Stop marking").size(12.5)).clicked();
                        });
                    });
                });
            if clear {
                self.setup.info.reader.find = None;
            }
            ui.add_space(12.0);
        }

        // ── body and contents
        let avail = ui.available_width();
        let wide = avail >= 1000.0;
        let toc_w = if wide { 250.0 } else { 0.0 };
        let gap = 34.0;
        let body_w = if wide { (avail - toc_w - gap).min(800.0) } else { avail };
        let folded = FOLDED.contains(&id);
        if !wide && doc.sections.len() > 1 {
            self.toc_narrow(ui, &doc, folded);
            ui.add_space(12.0);
        }
        let mut shown = md::Shown::default();
        let body_top = ui.cursor().min.y;
        // a jump asked for last frame happens now (the contents may ask again below)
        let jump = self.setup.info.reader.scroll_to.take();
        let mut body_rect = Rect::NOTHING;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            let r = ui.allocate_ui_with_layout(egui::vec2(body_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(body_w);
                shown = self.guide_body(ui, &g, &doc, folded, jump);
            });
            body_rect = r.response.rect;
            if wide {
                let h = (body_rect.max.y - body_top).max(10.0);
                ui.allocate_ui_with_layout(egui::vec2(toc_w, h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(toc_w);
                    ui.set_min_height(h);
                    // follow the page down, but stay beside the guide
                    let clip = ui.clip_rect();
                    let toc_h = self.setup.info.reader.toc_h.max(80.0);
                    let want = (clip.min.y + 12.0).max(body_top);
                    let y = want.min((body_top + h - toc_h).max(body_top));
                    ui.add_space(y - body_top);
                    let t0 = ui.cursor().min.y;
                    // how far down the guide you are
                    let span = (h - clip.height()).max(1.0);
                    let f = ((clip.min.y - body_top) / span).clamp(0.0, 1.0);
                    let (br, _) = ui.allocate_exact_size(egui::vec2(toc_w - 6.0, 18.0), Sense::hover());
                    let bar = Rect::from_min_size(egui::pos2(br.min.x, br.center().y - 2.0), egui::vec2(br.width() - 44.0, 4.0));
                    ui.painter().rect_filled(bar, 2.0, kit::tint(LINE(), 0.9));
                    ui.painter().rect_filled(Rect::from_min_size(bar.min, egui::vec2(bar.width() * f, 4.0)), 2.0, col);
                    ui.painter().text(egui::pos2(br.max.x, br.center().y), Align2::RIGHT_CENTER, format!("{:.0}%", f * 100.0), crate::fonts::sans_bold_font(10.5), MUTED());
                    ui.add_space(10.0);
                    self.toc_panel(ui, w, &g, &doc, &shown, folded);
                    self.setup.info.reader.toc_h = ui.cursor().min.y - t0;
                });
            }
        });
        if let Some(r) = shown.scroll.take() {
            ui.scroll_to_rect(r, Some(egui::Align::TOP));
        }
        if let Some(url) = shown.clicked.take() {
            let ctx = ui.ctx().clone();
            self.info_follow(w, &url, &ctx);
            return;
        }

        // ── the end: read tick, next and previous
        ui.add_space(26.0);
        let end = ui.allocate_exact_size(egui::vec2(body_w, 1.0), Sense::hover()).0;
        // ticked off once you've reached the end and spent a little while on it
        // (folded guides need a part open: their end is close by when shut)
        let now = ui.input(|i| i.time);
        if self.setup.info.reader.since.0 != id {
            self.setup.info.reader.since = (id.to_string(), now);
        }
        let stayed = now - self.setup.info.reader.since.1 >= if doc.words < 400 { 4.0 } else { 12.0 };
        let some_open = !folded || self.setup.info.reader.open.iter().any(|o| o.0 == id);
        if ui.is_rect_visible(end) && stayed && some_open && !self.guide_read(id) {
            self.set_guide_read(id, true);
        }
        if ui.is_rect_visible(end) && !stayed && !self.guide_read(id) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("✔").color(kit::C_GREEN));
            ui.label(small(if self.guide_read(id) { "You've read this one. It's ticked off in How to Play." } else { "Reach the end to tick it off." }));
        });
        ui.add_space(14.0);
        self.guide_prev_next(ui, w, &g, body_w);
    }

    /// The guide's text: straight through, or as fold-out parts.
    fn guide_body(&mut self, ui: &mut egui::Ui, g: &Guide, doc: &Doc, folded: bool, scroll_to: Option<usize>) -> md::Shown {
        let mut shown = md::Shown::default();
        let width_key = ui.available_width().round() as i32;
        let r = &mut self.setup.info.reader;
        if r.heights.0 != g.id || r.heights.1 != width_key {
            r.heights = (g.id.clone(), width_key, Vec::new());
        }
        let find = r.find.clone();
        let mut heights = std::mem::take(&mut r.heights.2);
        if !folded {
            let mut o = md::Opts { find: find.as_deref(), scroll_to, heights: &mut heights };
            md::show_blocks(ui, doc, 0..doc.blocks.len(), &mut o, &mut shown);
        } else {
            // before the first part
            let parts: Vec<usize> = doc.sections.iter().enumerate().filter(|(_, s)| s.level == 2).map(|(i, _)| i).collect();
            let first_block = parts.first().map(|&i| doc.sections[i].block).unwrap_or(doc.blocks.len());
            {
                let mut o = md::Opts { find: find.as_deref(), scroll_to, heights: &mut heights };
                md::show_blocks(ui, doc, 0..first_block, &mut o, &mut shown);
            }
            // expand / fold all
            ui.horizontal(|ui| {
                let all_open = parts.iter().all(|p| self.setup.info.reader.open.iter().any(|o| o.0 == g.id && o.1 == *p));
                ui.label(small(format!("{} · newest first", plural(parts.len(), "part", "parts"))));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.link(RichText::new(if all_open { "Fold all" } else { "Open all" }).size(12.5)).clicked() {
                        let open = &mut self.setup.info.reader.open;
                        open.retain(|o| o.0 != g.id);
                        if !all_open {
                            open.extend(parts.iter().map(|p| (g.id.clone(), *p)));
                        }
                    }
                });
            });
            ui.add_space(8.0);
            // the first part starts open
            let key = format!("folded-seen-{}", g.id);
            let seen = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new(&key))).unwrap_or(false);
            if !seen {
                if let Some(&p0) = parts.first() {
                    if !self.setup.info.reader.open.iter().any(|o| o.0 == g.id) {
                        self.setup.info.reader.open.push((g.id.clone(), p0));
                    }
                }
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(&key), true));
            }
            for (pi, &si) in parts.iter().enumerate() {
                let sec = &doc.sections[si];
                let end = parts.get(pi + 1).map(|&n| doc.sections[n].block).unwrap_or(doc.blocks.len());
                let is_open = self.setup.info.reader.open.iter().any(|o| o.0 == g.id && o.1 == si);
                let top = ui.cursor().min.y;
                shown.heads.push((si, top));
                // the part's bar
                let (title, chip) = part_title(&g.id, &sec.title);
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 50.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter_at(r.expand(2.0));
                let current = pi == 0;
                let col = if g.id == "known-bugs" && current { kit::C_ORANGE } else if current { kit::C_VIOLET } else { kit::C_SLATE };
                let round = if is_open { egui::Rounding { nw: 12.0, ne: 12.0, sw: 0.0, se: 0.0 } } else { egui::Rounding::same(12.0) };
                p.rect_filled(r, round, theme::blend(theme::blend(CARD(), col, 0.08), CARD_HOVER(), ht));
                p.rect_stroke(r, round, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.3 + 0.3 * ht)));
                let mut x = r.min.x + 16.0;
                if let Some(c) = &chip {
                    let gal = p.layout_no_wrap(c.clone(), crate::fonts::sans_bold_font(11.5), col);
                    let cr = Rect::from_min_size(egui::pos2(x, r.center().y - 11.0), egui::vec2(gal.size().x + 16.0, 22.0));
                    p.rect_filled(cr, 11.0, kit::tint(col, 0.2));
                    p.galley(egui::pos2(cr.min.x + 8.0, cr.center().y - gal.size().y / 2.0), gal, col);
                    x = cr.max.x + 10.0;
                }
                let f = crate::fonts::sans_bold_font(14.0);
                let maxw = r.max.x - x - 90.0;
                p.text(egui::pos2(x, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &title, &f, maxw), f, TEXT());
                if current {
                    let lbl = if g.id == "known-bugs" { "NOW" } else { "LATEST" };
                    p.text(egui::pos2(r.max.x - 40.0, r.center().y), Align2::RIGHT_CENTER, lbl, crate::fonts::sans_bold_font(10.0), col);
                }
                chevron(&p, egui::pos2(r.max.x - 22.0, r.center().y), is_open, if ht > 0.5 { TEXT() } else { MUTED() });
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    let open = &mut self.setup.info.reader.open;
                    if is_open {
                        open.retain(|o| !(o.0 == g.id && o.1 == si));
                    } else {
                        open.push((g.id.clone(), si));
                    }
                }
                if scroll_to == Some(si) {
                    shown.scroll = Some(r.expand2(egui::vec2(0.0, 12.0)));
                }
                if is_open {
                    let inner = egui::Frame::none()
                        .fill(CARD())
                        .stroke(Stroke::new(1.0f32, theme::blend(LINE(), col, 0.3)))
                        .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 12.0, se: 12.0 })
                        .inner_margin(egui::Margin { left: 20.0, right: 20.0, top: 4.0, bottom: 10.0 })
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let mut o = md::Opts { find: find.as_deref(), scroll_to, heights: &mut heights };
                            md::show_blocks(ui, doc, sec.block + 1..end, &mut o, &mut shown);
                        });
                    let _ = inner;
                    ui.add_space(4.0);
                }
                ui.add_space(8.0);
            }
        }
        self.setup.info.reader.heights.2 = heights;
        shown
    }

    /// Contents beside the guide; the part you're reading is lit.
    fn toc_panel(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, g: &Guide, doc: &Doc, shown: &md::Shown, folded: bool) {
        // the section whose heading has passed the top third of the window
        let clip = ui.clip_rect();
        let line = clip.min.y + (clip.height() * 0.3).max(120.0);
        let active = shown.heads.iter().filter(|(_, y)| *y <= line).map(|(s, _)| *s).last().or(shown.heads.first().map(|h| h.0));
        let many = doc.sections.len() > 24 || folded;
        // the lit row: the section you're in, or the listed one it sits under
        let shows = |s: &md::Section| s.level <= 3 && !(many && s.level > 2);
        let active = active.and_then(|a| (0..=a).rev().find(|&i| shows(&doc.sections[i])));
        let mut jump: Option<usize> = None;
        let mut open: Option<String> = None;
        if doc.sections.len() > 1 {
            ui.label(RichText::new("ON THIS PAGE").font(crate::fonts::sans_bold_font(10.5)).color(MUTED()));
            ui.add_space(6.0);
            egui::ScrollArea::vertical().id_salt(("toc", &g.id)).max_height(360.0).auto_shrink([false, true]).show(ui, |ui| {
                for (i, s) in doc.sections.iter().enumerate() {
                    if s.level > 3 || (many && s.level > 2) {
                        continue;
                    }
                    let on = active == Some(i);
                    let indent = if s.level >= 3 { 14.0 } else { 0.0 };
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width() - 6.0, 26.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let p = ui.painter_at(r);
                    let col = topic_meta(&g.group).1;
                    if on {
                        p.rect_filled(Rect::from_min_size(r.min, egui::vec2(3.0, r.height())), 1.5, col);
                    } else {
                        p.rect_filled(Rect::from_min_size(r.min, egui::vec2(1.0, r.height())), 0.0, kit::tint(LINE(), 0.9));
                    }
                    let f = if on { crate::fonts::sans_bold_font(12.5) } else { crate::fonts::sans_font(12.5) };
                    let text = super::charts::clip(ui, &s.title.replace("**", ""), &f, r.width() - 14.0 - indent);
                    p.text(egui::pos2(r.min.x + 12.0 + indent, r.center().y), Align2::LEFT_CENTER, text, f, if on || ht > 0.5 { TEXT() } else { MUTED() });
                    if on && ui.ctx().input(|i| i.pointer.is_decidedly_dragging()) {
                        // nothing: keep the list still while dragging
                    }
                    if resp.on_hover_text(&s.title).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        jump = Some(i);
                    }
                }
            });
            ui.add_space(14.0);
        }
        // the rest of this topic
        let same: Vec<Guide> = w.guides_sorted().into_iter().filter(|x| x.group == g.group && x.id != g.id).cloned().collect();
        if !same.is_empty() {
            ui.label(RichText::new(format!("MORE IN {}", g.group.to_uppercase())).font(crate::fonts::sans_bold_font(10.5)).color(MUTED()));
            ui.add_space(6.0);
            for x in same.iter().take(8) {
                let done = self.guide_read(&x.id);
                let label = if x.short.is_empty() { x.title.clone() } else { x.short.clone() };
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width() - 6.0, 26.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter_at(r);
                let c = egui::pos2(r.min.x + 6.0, r.center().y);
                if done {
                    p.text(c, Align2::CENTER_CENTER, "✔", crate::fonts::sans_bold_font(10.5), kit::C_GREEN);
                } else {
                    p.circle_stroke(c, 3.5, Stroke::new(1.2f32, MUTED()));
                }
                let f = crate::fonts::sans_font(12.5);
                p.text(egui::pos2(r.min.x + 18.0, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &label, &f, r.width() - 20.0), f, if ht > 0.5 { TEXT() } else { MUTED() });
                if resp.on_hover_text(&x.summary).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(x.id.clone());
                }
            }
            ui.add_space(10.0);
        }
        if ui.link(RichText::new("Back to the top  ↑").size(12.0)).clicked() {
            self.setup.scroll_top = true;
        }
        if let Some(i) = jump {
            self.toc_jump(&g.id, i, folded);
        }
        if let Some(id) = open {
            self.info_read(&id, None, None);
        }
    }

    fn toc_jump(&mut self, id: &str, section: usize, folded: bool) {
        let r = &mut self.setup.info.reader;
        r.scroll_to = Some(section);
        if folded && !r.open.iter().any(|o| o.0 == id && o.1 == section) {
            r.open.push((id.to_string(), section));
        }
    }

    /// Narrow windows: the contents as a fold-out list above the guide.
    fn toc_narrow(&mut self, ui: &mut egui::Ui, doc: &Doc, folded: bool) {
        let open = self.setup.info.reader.toc_open;
        let mut jump = None;
        let mut flip = false;
        kit::card(ui, |ui| {
            let resp = ui
                .horizontal(|ui| {
                    ui.label(RichText::new("On this page").font(crate::fonts::sans_bold_font(14.0)).color(TEXT()));
                    ui.label(small(format!("{}", doc.sections.iter().filter(|s| s.level <= 2 || !folded).count())));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
                        chevron(ui.painter(), r.center(), open, MUTED());
                    });
                })
                .response
                .interact(Sense::click());
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                flip = true;
            }
            if open {
                ui.add_space(6.0);
                for (i, s) in doc.sections.iter().enumerate() {
                    if s.level > 3 || (folded && s.level > 2) {
                        continue;
                    }
                    let indent = if s.level >= 3 { 16.0 } else { 0.0 };
                    ui.horizontal(|ui| {
                        ui.add_space(indent);
                        if ui.link(RichText::new(s.title.replace("**", "")).size(12.5)).clicked() {
                            jump = Some(i);
                        }
                    });
                }
            }
        });
        if flip {
            self.setup.info.reader.toc_open = !open;
        }
        if let Some(i) = jump {
            let id = match &self.setup.info.page {
                Page::Guide(id) => id.clone(),
                _ => return,
            };
            self.toc_jump(&id, i, folded);
            self.setup.info.reader.toc_open = false;
        }
    }

    fn guide_prev_next(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, g: &Guide, width: f32) {
        let all: Vec<Guide> = w.guides_sorted().into_iter().cloned().collect();
        let Some(i) = all.iter().position(|x| x.id == g.id) else { return };
        let prev = i.checked_sub(1).and_then(|j| all.get(j)).cloned();
        let next = all.get(i + 1).cloned();
        let gap = 14.0;
        let cw = ((width - gap) / 2.0).floor();
        let mut open: Option<String> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (slot, right) in [(prev, false), (next, true)] {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 70.0), if slot.is_some() { Sense::click() } else { Sense::hover() });
                let Some(x) = slot else { continue };
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let (_, col) = topic_meta(&x.group);
                let p = ui.painter_at(r.expand(2.0));
                p.rect_filled(r, 14.0, theme::blend(CARD(), CARD_HOVER(), ht));
                p.rect_stroke(r, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.5 * ht)));
                let (ax, al) = if right { (r.max.x - 16.0, Align2::RIGHT_CENTER) } else { (r.min.x + 16.0, Align2::LEFT_CENTER) };
                let cap = if right { "NEXT  ›" } else { "‹  PREVIOUS" };
                let cap = if x.group != g.group { format!("{cap}  ·  {}", x.group.to_uppercase()) } else { cap.to_string() };
                p.text(egui::pos2(ax, r.center().y - 10.0), al, cap, crate::fonts::sans_bold_font(10.0), col);
                let f = crate::fonts::sans_bold_font(14.0);
                p.text(egui::pos2(ax, r.center().y + 10.0), al, super::charts::clip(ui, &x.title, &f, cw - 32.0), f, TEXT());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(x.id.clone());
                }
            }
        });
        if let Some(id) = open {
            self.info_read(&id, None, None);
        }
    }

    /// "‹ Back" (to wherever you came from) and where you are.
    pub(super) fn info_back_bar(&mut self, ui: &mut egui::Ui, here: &str) {
        let back_to = self.setup.info.history.last().map(|p| self.page_label(p));
        let mut back = false;
        let mut home = false;
        let mut lib = false;
        let mut moves = false;
        let mut items = false;
        ui.horizontal(|ui| {
            let label = format!("‹  {}", back_to.clone().unwrap_or_else(|| "Game Info".into()));
            back = ui.add(egui::Button::new(RichText::new(label).size(13.0)).fill(kit::btn_fill())).clicked();
            ui.add_space(10.0);
            home = ui.link(RichText::new("Game Info").size(12.5)).clicked();
            ui.label(small("›"));
            if matches!(self.setup.info.page, Page::Guide(_)) {
                lib = ui.link(RichText::new("How to Play").size(12.5)).clicked();
                ui.label(small("›"));
            }
            if matches!(self.setup.info.page, Page::Move(_)) {
                moves = ui.link(RichText::new("Moves").size(12.5)).clicked();
                ui.label(small("›"));
            }
            if matches!(self.setup.info.page, Page::Item(_)) {
                items = ui.link(RichText::new("Items").size(12.5)).clicked();
                ui.label(small("›"));
            }
            ui.label(RichText::new(here).size(12.5).color(TEXT()));
        });
        ui.add_space(16.0);
        if back {
            self.info_back();
        } else if home {
            self.info_push(Page::Home);
        } else if lib {
            self.info_push(Page::Guides);
        } else if moves {
            self.info_push(Page::Moves);
        } else if items {
            self.info_push(Page::Items);
        }
    }

    fn page_label(&self, p: &Page) -> String {
        match p {
            Page::Home => "Game Info".into(),
            Page::Guides => "How to Play".into(),
            Page::Moves => "Moves".into(),
            Page::Items => "Items".into(),
            Page::Item(id) => self.wiki.as_ref().and_then(|w| w.data.items.iter().find(|i| &i.id == id).map(|i| i.name.clone())).unwrap_or_else(|| "Item".into()),
            Page::Move(k) => self.wiki.as_ref().and_then(|w| w.move_info(k).map(|m| m.n.clone())).unwrap_or_else(|| "Move".into()),
            Page::Guide(id) => self
                .wiki
                .as_ref()
                .and_then(|w| w.guide(id).map(|g| if g.short.is_empty() { g.title.clone() } else { g.short.clone() }))
                .unwrap_or_else(|| "Guide".into()),
        }
    }
}

/// "0.3.4: safer saves…" → ("Safer saves…", "0.3.4"); "Fixed in 0.3.3" → ("Fixed", "0.3.3").
fn part_title(guide: &str, title: &str) -> (String, Option<String>) {
    let t = title.replace("**", "");
    if let Some((v, rest)) = t.split_once(':') {
        if v.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            let rest = rest.trim();
            return (if rest.is_empty() { "Update".into() } else { capitalise(rest) }, Some(v.trim().to_string()));
        }
    }
    // "0.3.3 hotfix", "In 0.3.4", "Fixed in 0.3.2"
    let words: Vec<&str> = t.split_whitespace().collect();
    if let Some(vi) = words.iter().position(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()) && w.contains('.')) {
        let v = words[vi].to_string();
        let rest: Vec<&str> = words.iter().enumerate().filter(|(i, _)| *i != vi).map(|(_, w)| *w).collect();
        let mut rest = rest.join(" ");
        rest = rest.trim_end_matches(" in").trim_end_matches("In").trim().to_string();
        if rest.is_empty() {
            rest = if guide == "known-bugs" { "Still broken".into() } else { "Update".into() };
        }
        return (capitalise(&rest), Some(v));
    }
    (t, None)
}

/// A small arrow: down when open, right when shut.
fn chevron(p: &egui::Painter, c: egui::Pos2, open: bool, col: Color32) {
    let s = 4.5;
    let pts = if open {
        vec![c + egui::vec2(-s, -s * 0.5), c + egui::vec2(0.0, s * 0.5), c + egui::vec2(s, -s * 0.5)]
    } else {
        vec![c + egui::vec2(-s * 0.5, -s), c + egui::vec2(s * 0.5, 0.0), c + egui::vec2(-s * 0.5, s)]
    };
    p.add(egui::Shape::line(pts, Stroke::new(1.8f32, col)));
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn tag(ui: &mut egui::Ui, text: &str, col: Color32) -> egui::Response {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(11.0), col);
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(16.0, 8.0), Sense::hover());
    ui.painter().rect_filled(r, r.height() / 2.0, kit::tint(col, 0.16));
    ui.painter().galley(r.min + egui::vec2(8.0, 4.0), g, col);
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_titles() {
        assert_eq!(part_title("whats-new", "0.3.4: safer saves"), ("Safer saves".into(), Some("0.3.4".into())));
        assert_eq!(part_title("whats-new", "0.3.3 hotfix"), ("Hotfix".into(), Some("0.3.3".into())));
        assert_eq!(part_title("known-bugs", "Fixed in 0.3.2"), ("Fixed".into(), Some("0.3.2".into())));
        assert_eq!(part_title("known-bugs", "In 0.3.4"), ("Still broken".into(), Some("0.3.4".into())));
        assert_eq!(part_title("x", "Plain"), ("Plain".into(), None));
    }

    #[test]
    fn snippets_cut_at_words() {
        let t = "The game saves when you pick SAVE in the pause menu, and with AUTOSAVE on it also saves by itself every few minutes and when you close the game.";
        let s = snippet(t, "autosave").expect("found");
        assert!(s.to_lowercase().contains("autosave"));
        assert!(s.chars().count() < 170);
        assert!(snippet(t, "zzz").is_none());
    }
}
