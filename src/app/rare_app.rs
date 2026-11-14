//! Rare Grass: what's on rare grass today, a card per Pokémon (rarest
//! first, with its places), and a glossary of every Pokémon seen on rare
//! grass before: how many days, when last, and where. Today's list is
//! worked out with the game's own formula for the date on the player's clock
//! (no game files read); each day the app sees is kept in the history.

use super::rare::RareSpot;
use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::App;
use crate::apps::AppId;
use crate::rare_log::{self, RareFile, Seen};
use crate::stats::now_unix;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};

pub(super) const GREEN: Color32 = Color32::from_rgb(70, 200, 120);
/// Seconds the banner shows each Pokémon.
const CYCLE_SECS: u64 = 6;
pub(super) const GOLD: Color32 = Color32::from_rgb(236, 186, 80);

/// Which Pokémon the glossary shows.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Show {
    #[default]
    Today,
    Once,
    All,
}

/// Past days added to the history from the shared lists in git (as dates:
/// 2026-11-11, 12 and 13).
const PAST_DAYS: [i64; 3] = [20_768, 20_769, 20_770];

#[derive(Default)]
pub(super) struct RareState {
    pub file: Option<RareFile>,
    pub load_error: Option<String>,
    dirty: Option<Instant>,
    saved_once: bool,
    pub save_error: Option<String>,
    last_record: Option<Instant>,
    /// The past days from the shared lists have been filled in this run.
    backfilled: bool,
    pub query: String,
    pub show: Show,
    /// Every card out today, not just the first nine.
    pub all_today: bool,
    /// Which of today's Pokémon the banner shows (0: the rarest), and when
    /// it last moved on by itself.
    pub featured: usize,
    featured_at: Option<Instant>,
    /// The pointer was over the banner last frame (it waits then).
    hero_hover: bool,
}

impl RareState {
    pub fn file(&mut self) -> &mut RareFile {
        if self.file.is_none() {
            let (mut f, note) = rare_log::load_from(&crate::config::data_dir());
            self.load_error = note;
            if f.seed() {
                self.dirty = Some(Instant::now());
            }
            self.file = Some(f);
        }
        self.file.as_mut().expect("loaded")
    }

    pub fn touch(&mut self) {
        self.dirty.get_or_insert_with(Instant::now);
    }

    pub fn save_tick(&mut self, force: bool) {
        let Some(t) = self.dirty else { return };
        if !force && t.elapsed() < Duration::from_millis(1500) {
            return;
        }
        let Some(f) = self.file.as_ref() else { return };
        match rare_log::save_to(&crate::config::data_dir(), f, !self.saved_once) {
            Ok(()) => {
                self.saved_once = true;
                self.dirty = None;
                self.save_error = None;
            }
            Err(e) => {
                self.save_error = Some(e);
                self.dirty = Some(Instant::now());
            }
        }
    }
}

/// A Pokémon out today: everywhere it is.
#[derive(Clone)]
pub(super) struct OutToday {
    pub spots: Vec<RareSpot>,
    /// Days it's been seen (today included).
    pub days: usize,
}

impl OutToday {
    pub fn name(&self) -> &'static str {
        self.spots[0].species.name
    }
    pub fn tiles(&self) -> usize {
        self.spots.iter().map(|s| s.tiles).sum()
    }
}

/// A history date as a day number (they're stored as dates already).
pub(super) fn day_of(day: i64) -> i64 {
    day
}

/// Today's server date (CET): the game's rare grass day.
pub(super) fn reset_now() -> i64 {
    crate::rare_calc::today(now_unix() as i64)
}

/// Seconds until the rare grass changes (the daily reset, midnight CET).
pub(super) fn secs_to_reset() -> i64 {
    let now = now_unix() as i64;
    (crate::rare_calc::next_change(now) - now).max(0)
}

pub(super) fn countdown(secs: i64) -> String {
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    if h > 0 {
        format!("{h}h {m:02}m")
    } else {
        format!("{m}m")
    }
}

/// "Today", "Yesterday" or "13 Nov" for a date.
pub(super) fn when(reset: i64) -> String {
    let d = day_of(reset);
    let today = day_of(reset_now());
    match today - d {
        0 => "Today".into(),
        1 => "Yesterday".into(),
        _ => crate::stats::fmt_day_short(d),
    }
}

impl App {
    fn rare_on(&self) -> bool {
        crate::apps::live(AppId::Rare)
    }

    /// Every minute: note today's list in the history; save.
    pub(super) fn rare_tick(&mut self) {
        crate::rare_calc::set_modern(self.cfg.rare_modern);
        if !self.rare_on() {
            return;
        }
        // Wait for the Pokémon list and for the map to finish loading (it
        // gives the full place names), so each day is noted the same way.
        let ready = self.wiki.is_some() && self.gamemap.is_some();
        let mut changed = false;
        if ready && self.setup.rare.last_record.map_or(true, |t| t.elapsed() > Duration::from_secs(60)) {
            self.setup.rare.last_record = Some(Instant::now());
            let spots = self.rare_spots_today();
            let list: Vec<(String, String, u32)> = spots.iter().map(|s| (s.species.slug.clone(), s.place.clone(), s.tiles as u32)).collect();
            changed |= self.setup.rare.file().set_day(reset_now(), &list);
        }
        // The days the shared rare grass lists in git covered (11–13 Nov
        // 2026), put in whole from the formula: the shared lists only had
        // part of each day, and every tile they had matches it.
        if ready && !self.setup.rare.backfilled {
            self.setup.rare.backfilled = true;
            let today = reset_now();
            for day in PAST_DAYS.iter().copied().filter(|d| *d < today) {
                let list: Vec<(String, String, u32)> = self.rare_spots_on(day).iter().map(|s| (s.species.slug.clone(), s.place.clone(), s.tiles as u32)).collect();
                changed |= self.setup.rare.file().set_day(day, &list);
            }
        }
        if changed {
            self.setup.rare.touch();
        }
        self.setup.rare.save_tick(false);
    }

    /// Today's Pokémon, rarest first (fewest days seen, then fewest tiles).
    pub(super) fn rare_out_today(&mut self) -> Vec<OutToday> {
        let spots = self.rare_spots_today();
        let gloss = self.setup.rare.file().glossary();
        let mut out: Vec<OutToday> = Vec::new();
        for s in spots {
            match out.iter_mut().find(|o| o.spots[0].species.slug == s.species.slug) {
                Some(o) => o.spots.push(s),
                None => {
                    let days = gloss.iter().find(|g| g.slug == s.species.slug).map(|g| g.days.len()).unwrap_or(1).max(1);
                    out.push(OutToday { spots: vec![s], days });
                }
            }
        }
        for o in out.iter_mut() {
            o.spots.sort_by(|a, b| b.tiles.cmp(&a.tiles).then(a.place.cmp(&b.place)));
        }
        out.sort_by(|a, b| a.days.cmp(&b.days).then(a.tiles().cmp(&b.tiles())).then(a.name().cmp(b.name())));
        out
    }

    /// The glossary: everything seen, A–Z.
    pub(super) fn rare_glossary(&mut self) -> Vec<Seen> {
        let mut g = self.setup.rare.file().glossary();
        let name = |slug: &str| self.species.iter().find(|s| s.slug == slug).map(|s| s.name).unwrap_or("");
        g.sort_by(|a, b| name(&a.slug).cmp(name(&b.slug)));
        g
    }

    fn rare_species_by_slug(&self, slug: &str) -> Option<crate::species::Species> {
        self.species.iter().find(|s| s.slug == slug).cloned()
    }

    /// The Pokédex entry for a species slug.
    fn rare_wiki_for(&self, slug: &str) -> Option<String> {
        let w = self.wiki.as_ref()?;
        w.all().find(|m| crate::species::slugify(&m.name) == slug).map(|m| m.id.clone())
    }

    /// A sprite in a soft circle (or a short name badge until it's here).
    pub(super) fn rare_sprite(&mut self, p: &egui::Painter, c: egui::Pos2, r: f32, slug: &str, dim: bool) {
        p.circle_filled(c, r, theme::blend(kit::sunken(), Color32::WHITE, 0.05));
        let sp = self.rare_species_by_slug(slug);
        let shown = match &sp {
            Some(sp) if self.sprites.get(sp, false).is_some() => self.draw_sprite_centered(p, sp, false, c, vec2(r * 2.4, r * 2.4)),
            _ => false,
        };
        if !shown {
            let short: String = sp.as_ref().map(|s| s.name).unwrap_or("???").chars().filter(|c| c.is_alphabetic()).take(3).collect::<String>().to_uppercase();
            p.circle_filled(c, r * 0.85, theme::blend(kit::C_SLATE, GREEN, 0.25));
            p.text(c, Align2::CENTER_CENTER, short, crate::fonts::sans_bold_font(r * 0.55), Color32::WHITE);
        }
        if dim {
            p.circle_filled(c, r + 1.0, Color32::from_black_alpha(90));
        }
    }

    // ───────────────────────────── the page ─────────────────────────────

    pub(super) fn page_rare(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "RARE GRASS", "What's on rare grass today, and every Pokémon seen on it before.");
        if let Some(e) = self.setup.rare.load_error.clone() {
            kit::pill(ui, &e, kit::PillKind::Warn);
            ui.add_space(8.0);
        }
        let today = self.rare_out_today();
        let gloss = self.rare_glossary();
        let once = gloss.iter().filter(|g| g.days.len() == 1).count();
        self.rare_hero(ui, &today);
        ui.add_space(16.0);
        // search and filters
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let w = (ui.available_width() - 420.0).clamp(200.0, 420.0);
            kit::search_box(ui, &mut self.setup.rare.query, "Search a Pokémon or place", w);
            for (s, label) in [(Show::Today, format!("Out today {}", today.len())), (Show::Once, format!("Seen only once {once}")), (Show::All, format!("All seen {}", gloss.len()))] {
                let on = self.setup.rare.show == s;
                let b = egui::Button::new(RichText::new(label).size(12.5).color(if on { Color32::WHITE } else { TEXT() }))
                    .fill(if on { theme::blend(kit::CARD(), GREEN, 0.35) } else { kit::btn_fill() })
                    .stroke(Stroke::new(1.0f32, if on { GREEN } else { LINE() }))
                    .rounding(999.0)
                    .min_size(vec2(0.0, 30.0));
                if ui.add(b).on_hover_text("Which Pokémon the glossary shows").clicked() {
                    self.setup.rare.show = s;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let on = self.cfg.rare_modern;
                let t = RichText::new(if on { "✔ MODERN POKéMON on" } else { "MODERN POKéMON off" }).size(12.5).color(if on { Color32::WHITE } else { TEXT() });
                let b = egui::Button::new(t)
                    .fill(if on { theme::blend(kit::CARD(), GREEN, 0.35) } else { kit::btn_fill() })
                    .stroke(Stroke::new(1.0f32, if on { GREEN } else { LINE() }))
                    .rounding(999.0)
                    .min_size(vec2(0.0, 30.0));
                if ui.add(b).on_hover_text("Match your game's MODERN POKéMON world option: with it on, newer Pokémon can be on rare grass too, and the Pokémon on each patch change").clicked() {
                    self.cfg.rare_modern = !on;
                    self.dirty = true;
                    crate::rare_calc::set_modern(!on);
                }
            });
        });
        ui.add_space(12.0);
        let q = self.setup.rare.query.trim().to_lowercase();
        let today_f: Vec<OutToday> = today
            .iter()
            .filter(|o| q.is_empty() || o.name().to_lowercase().contains(&q) || o.spots.iter().any(|s| s.place.to_lowercase().contains(&q)))
            .cloned()
            .collect();
        self.rare_today_card(ui, &today, &today_f);
        ui.add_space(4.0);
        self.rare_glossary_card(ui, &gloss, &today, &q);
    }

    /// The banner: the featured Pokémon (the rarest out today, or the
    /// next one with ›), big on the right, today in numbers, its places and
    /// buttons. Green like rare grass, shaped like the Dashboard's hunt card.
    fn rare_hero(&mut self, ui: &mut egui::Ui, today: &[OutToday]) {
        let w = ui.available_width();
        let show_sprite = w >= 560.0;
        let art_w = if show_sprite { (w * 0.3).clamp(220.0, 320.0) } else { 0.0 };
        let text_w = (w - 56.0 - art_w).max(200.0);
        // the banner moves on to the next Pokémon every few seconds (not
        // while the pointer is over it)
        let at = *self.setup.rare.featured_at.get_or_insert_with(Instant::now);
        if !today.is_empty() && !self.setup.rare.hero_hover && at.elapsed() >= Duration::from_secs(CYCLE_SECS) {
            self.setup.rare.featured = (self.setup.rare.featured + 1) % today.len();
            self.setup.rare.featured_at = Some(Instant::now());
        }
        ui.ctx().request_repaint_after(Duration::from_millis(500));
        if self.setup.rare.featured >= today.len() {
            self.setup.rare.featured = 0;
        }
        let feat = today.get(self.setup.rare.featured).cloned();
        let tiles: usize = today.iter().map(|o| o.tiles()).sum();
        let firsts = today.iter().filter(|o| o.days == 1).count();
        let (mut map, mut hunt, mut next) = (false, false, false);
        let slot = ui.painter().add(egui::Shape::Noop);
        let frame = egui::Frame::none()
            .inner_margin(egui::Margin { left: 28.0, right: 28.0, top: 22.0, bottom: 24.0 })
            .show(ui, |ui| {
                ui.set_width(w - 56.0);
                ui.set_min_height(186.0);
                ui.allocate_ui_with_layout(vec2(text_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(text_w);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let label = match &feat {
                        Some(_) if self.setup.rare.featured == 0 => "RAREST ON RARE GRASS TODAY".to_string(),
                        Some(_) => format!("ON RARE GRASS TODAY · {} OF {}", self.setup.rare.featured + 1, today.len()),
                        None => "RARE GRASS TODAY".to_string(),
                    };
                    ui.label(RichText::new(label).font(crate::fonts::sans_bold_font(11.5)).color(theme::blend(TEXT(), GREEN, 0.45)));
                    match &feat {
                        Some(o) => {
                            ui.add(egui::Label::new(RichText::new(o.name()).font(crate::fonts::sans_bold_font(28.0)).color(TEXT())).truncate());
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                                if o.days == 1 {
                                    super::home::chip_col(ui, "★ First time on rare grass", GOLD);
                                } else {
                                    super::home::chip(ui, &format!("Seen on {} days", o.days));
                                }
                                for s in o.spots.iter().take(3) {
                                    super::home::chip(ui, &format!("{} · {} tile{}", s.place, s.tiles, if s.tiles == 1 { "" } else { "s" }));
                                }
                                if o.spots.len() > 3 {
                                    super::home::chip(ui, &format!("+{} more", o.spots.len() - 3));
                                }
                            });
                        }
                        None => {
                            ui.label(RichText::new("Nothing yet today").font(crate::fonts::sans_bold_font(28.0)).color(TEXT()));
                            ui.label(RichText::new("The shared list comes in after the reset, or use Sweet Scent on a patch and it shows up here.").size(13.0).color(MUTED()));
                        }
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(RichText::new(today.len().to_string()).font(crate::fonts::sans_bold_font(34.0)).color(TEXT()));
                        ui.label(RichText::new(format!("Pokémon on {tiles} tiles · {firsts} new today · new list in {}", countdown(secs_to_reset()))).size(13.5).color(MUTED()));
                    });
                    ui.add_space(10.0);
                    if feat.is_some() {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                            let go = egui::Button::new(RichText::new("🗺  Show on Map").size(13.0).color(Color32::from_rgb(14, 36, 22))).fill(GREEN).rounding(18.0).min_size(vec2(150.0, 36.0));
                            map = ui.add(go).on_hover_text("Open the Map at its rare grass").clicked();
                            hunt = super::home::sec_button(ui, "🎯  Hunt it", "Make it your target and hunt it with Sweet Scent on rare grass");
                            if today.len() > 1 {
                                next = super::home::sec_button(ui, "Next ›", "The next Pokémon out today (the banner moves on by itself every few seconds, and waits while your pointer is on it)");
                            }
                        });
                    }
                });
            });
        let rect = frame.response.rect;
        self.setup.rare.hero_hover = ui.rect_contains_pointer(rect);
        let h = rect.height();
        let a = theme::blend(kit::CARD(), GREEN, 0.30);
        let b = theme::blend(kit::CARD(), Color32::from_rgb(184, 224, 90), 0.12);
        ui.painter().set(
            slot,
            egui::Shape::Vec(vec![
                kit::card_shadow().as_shape(rect, 18.0).into(),
                theme::gradient_rect(rect, [18.0; 4], a, b),
                egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), GREEN, 0.35))),
            ]),
        );
        if show_sprite {
            let pc = ui.painter_at(rect.shrink(1.0));
            let c = pos2(rect.max.x - art_w / 2.0 - 12.0, rect.center().y + 4.0);
            let s = h.min(art_w);
            for (rad, alpha) in [(s * 0.6, 6u8), (s * 0.45, 8), (s * 0.31, 12)] {
                pc.circle_filled(c, rad, Color32::from_rgba_premultiplied(alpha, alpha, alpha, alpha));
            }
            match feat.as_ref().map(|o| o.spots[0].species.clone()) {
                Some(sp) if self.sprites.get(&sp, false).is_some() => {
                    self.draw_sprite(&pc, &sp, false, c + vec2(0.0, s * 0.32), vec2(s * 0.78, s * 0.72));
                }
                Some(sp) => {
                    // until its sprite is here: a big name badge
                    self.rare_sprite(&pc, c, s * 0.22, &sp.slug, false);
                }
                None => {
                    pc.text(c, Align2::CENTER_CENTER, "?", kit::font_bold(56.0), MUTED());
                }
            }
            for (dx, dy, r) in [(-0.34, -0.30, 7.0), (0.30, -0.22, 5.0), (0.24, 0.30, 4.0)] {
                theme::sparkle(&pc, c + vec2(s * dx, s * dy), r, GREEN);
            }
        }
        if let Some(o) = feat {
            if map {
                if self.has_app(AppId::Map) {
                    self.go_to_rare(&o.spots[0]);
                } else {
                    self.open_app(AppId::Map);
                }
            }
            if hunt {
                let sp = o.spots[0].species.clone();
                self.hunt_on_rare_grass(&sp.slug, sp.name, &o.spots);
            }
        }
        if next {
            self.setup.rare.featured = (self.setup.rare.featured + 1) % today.len().max(1);
            self.setup.rare.featured_at = Some(Instant::now());
        }
    }

    fn rare_today_card(&mut self, ui: &mut egui::Ui, all: &[OutToday], list: &[OutToday]) {
        let tiles: usize = all.iter().map(|o| o.tiles()).sum();
        let has_map = self.game_map().is_some();
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("OUT TODAY · {} POKÉMON ON {} TILES", all.len(), tiles)).font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("Rarest first · new list at the reset in {}", countdown(secs_to_reset()))).size(12.0).color(MUTED()));
                });
            });
            ui.add_space(8.0);
            if !has_map {
                ui.label(RichText::new("Today's rare grass needs the game's map files. Set the game folder in Advanced.").size(13.0).color(MUTED()));
                return;
            }
            if all.is_empty() {
                ui.label(RichText::new("Nothing on rare grass yet today. The shared list comes in after the reset, or use Sweet Scent on a patch and it shows up here.").size(13.0).color(MUTED()));
                return;
            }
            if list.is_empty() {
                ui.label(RichText::new("Nothing out today matches your search.").size(13.0).color(MUTED()));
                return;
            }
            let w = ui.available_width();
            let cols = ((w + 10.0) / 290.0).floor().clamp(1.0, 4.0) as usize;
            let cw = ((w - 10.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            let shown = if self.setup.rare.all_today { list.len() } else { list.len().min(cols * 3) };
            for row in list[..shown].chunks(cols) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for o in row {
                        ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            ui.set_width(cw);
                            self.rare_mon_card(ui, o);
                        });
                    }
                });
                ui.add_space(10.0);
            }
            if list.len() > cols * 3 {
                let label = if self.setup.rare.all_today { "Show fewer".to_string() } else { format!("Show all {}", list.len()) };
                if ui.add(egui::Button::new(RichText::new(label).size(12.5)).fill(kit::btn_fill()).min_size(vec2(0.0, 30.0))).clicked() {
                    self.setup.rare.all_today = !self.setup.rare.all_today;
                }
            }
        });
    }

    /// One Pokémon out today: sprite, name, tiles and places, its place
    /// chips (each opens the Map there), Show on Map and Hunt it.
    fn rare_mon_card(&mut self, ui: &mut egui::Ui, o: &OutToday) {
        let first = o.days == 1;
        let sp = o.spots[0].species.clone();
        let mut go: Option<RareSpot> = None;
        let mut hunt = false;
        // the outline lights up on hover (gold for a first-timer)
        let hover_id = ui.id().with(("rare-card", &sp.slug));
        let hovered = ui.ctx().data(|d| d.get_temp::<bool>(hover_id)).unwrap_or(false);
        let ht = kit::anim(ui.ctx(), hover_id.with("a"), hovered, kit::QUICK);
        let edge = theme::blend(LINE(), if first { GOLD } else { GREEN }, 0.7 * ht);
        let framed = egui::Frame::none()
            .fill(theme::blend(kit::sunken(), if first { GOLD } else { GREEN }, 0.04 * ht))
            .stroke(Stroke::new(1.0f32, edge))
            .rounding(12.0)
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                    let p = ui.painter().clone();
                    self.rare_sprite(&p, r.center(), 18.0, &sp.slug, false);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(RichText::new(sp.name).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                        let places = o.spots.len();
                        ui.label(RichText::new(format!("{} tile{} · {} place{}", o.tiles(), if o.tiles() == 1 { "" } else { "s" }, places, if places == 1 { "" } else { "s" })).size(11.5).color(MUTED()));
                    });
                    if first {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            kit::pill(ui, "First time", kit::PillKind::Warn).on_hover_text("Not seen on rare grass on any day before this one");
                        });
                    }
                });
                ui.add_space(6.0);
                // the places on one line (as many as fit, then "+N more"), so
                // every card is the same height
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    let pad = ui.spacing().button_padding.x * 2.0 + 2.0;
                    let font = egui::FontId::proportional(12.0);
                    let mut room = ui.available_width();
                    let labels: Vec<String> = o.spots.iter().map(|s| format!("{}  {}", s.place, s.tiles)).collect();
                    let widths: Vec<f32> = labels.iter().map(|l| ui.fonts(|f| f.layout_no_wrap(l.clone(), font.clone(), TEXT()).size().x) + pad).collect();
                    let mut shown = 0;
                    for (k, w) in widths.iter().enumerate() {
                        let left_after = o.spots.len() - k - 1;
                        let need = w + if left_after > 0 { 70.0 } else { 0.0 };
                        if need > room && k > 0 {
                            break;
                        }
                        room -= w + 6.0;
                        shown += 1;
                    }
                    for (s, l) in o.spots.iter().zip(&labels).take(shown.max(1)) {
                        let b = egui::Button::new(RichText::new(l).size(12.0).color(TEXT())).fill(kit::CARD()).stroke(Stroke::new(1.0f32, LINE())).rounding(999.0);
                        if ui.add(b).on_hover_text(format!("{} rare grass tile{} with {} at {}: show it on the Map", s.tiles, if s.tiles == 1 { "" } else { "s" }, sp.name, s.place)).clicked() {
                            go = Some(s.clone());
                        }
                    }
                    let more = o.spots.len().saturating_sub(shown.max(1));
                    if more > 0 {
                        let all: Vec<String> = o.spots.iter().skip(shown.max(1)).map(|s| format!("{} · {}", s.place, s.tiles)).collect();
                        ui.label(RichText::new(format!("+{more} more")).size(11.5).color(MUTED())).on_hover_text(all.join("\n"));
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Show on Map").size(12.5)).fill(kit::btn_fill()).min_size(vec2(0.0, 28.0))).clicked() {
                        go = Some(o.spots[0].clone());
                    }
                    if ui.add(egui::Button::new(RichText::new("Hunt it").size(12.5)).fill(kit::btn_fill()).min_size(vec2(0.0, 28.0))).on_hover_text("Make it your target and hunt it with Sweet Scent on rare grass").clicked() {
                        hunt = true;
                    }
                });
            });
        let now_hovered = ui.rect_contains_pointer(framed.response.rect);
        ui.ctx().data_mut(|d| d.insert_temp(hover_id, now_hovered));
        if let Some(s) = go {
            if self.has_app(AppId::Map) {
                self.go_to_rare(&s);
            } else {
                self.open_app(AppId::Map);
            }
        }
        if hunt {
            let spots = o.spots.clone();
            self.hunt_on_rare_grass(&sp.slug, sp.name, &spots);
        }
    }

    fn rare_glossary_card(&mut self, ui: &mut egui::Ui, gloss: &[Seen], today: &[OutToday], q: &str) {
        let now_day = day_of(reset_now());
        let out_now = |slug: &str| today.iter().find(|o| o.spots[0].species.slug == slug);
        let show = self.setup.rare.show;
        let list: Vec<&Seen> = gloss
            .iter()
            .filter(|g| match show {
                Show::Today => out_now(&g.slug).is_some(),
                Show::Once => g.days.len() == 1,
                Show::All => true,
            })
            .filter(|g| {
                if q.is_empty() {
                    return true;
                }
                let name = self.species.iter().find(|s| s.slug == g.slug).map(|s| s.name.to_lowercase()).unwrap_or_default();
                name.contains(q) || out_now(&g.slug).is_some_and(|o| o.spots.iter().any(|s| s.place.to_lowercase().contains(q)))
            })
            .collect();
        let days_kept = self.setup.rare.file().days.len();
        let mut clicked: Option<String> = None;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("GLOSSARY · EVERY POKÉMON SEEN ON RARE GRASS").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let first = self.setup.rare.file().days.first().map(|d| crate::stats::fmt_day_short(day_of(d.day))).unwrap_or_default();
                    ui.label(RichText::new(format!("{} Pokémon over {days_kept} days since {first} · dots: the last 7 days", gloss.len())).size(12.0).color(MUTED()));
                });
            });
            ui.add_space(8.0);
            if list.is_empty() {
                ui.label(RichText::new("No Pokémon here yet.").size(13.0).color(MUTED()));
                return;
            }
            let w = ui.available_width();
            let tw = 104.0f32;
            let cols = ((w + 8.0) / (tw + 8.0)).floor().max(1.0) as usize;
            let tw = ((w - 8.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            let th = 112.0;
            let rows = list.len().div_ceil(cols);
            let (area, _) = ui.allocate_exact_size(vec2(w, rows as f32 * (th + 8.0) - 8.0), Sense::hover());
            for (i, g) in list.iter().enumerate() {
                let r = Rect::from_min_size(area.min + vec2((i % cols) as f32 * (tw + 8.0), (i / cols) as f32 * (th + 8.0)), vec2(tw, th));
                let resp = ui.interact(r, ui.id().with(("rare-g", &g.slug)), Sense::click());
                let p = ui.painter().clone();
                let out = out_now(&g.slug).is_some();
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                p.rect_filled(r, 10.0, theme::blend(kit::sunken(), GREEN, 0.05 * ht));
                p.rect_stroke(r, 10.0, Stroke::new(1.0f32, theme::blend(LINE(), GREEN, 0.75 * ht)));
                self.rare_sprite(&p, pos2(r.center().x, r.min.y + 26.0), 17.0, &g.slug, false);
                let name = self.species.iter().find(|s| s.slug == g.slug).map(|s| s.name).unwrap_or("?");
                p.text(pos2(r.center().x, r.min.y + 58.0), Align2::CENTER_CENTER, name, crate::fonts::sans_bold_font(12.0), TEXT());
                // the last 7 days as dots
                let dots: Vec<bool> = (0..7).rev().map(|k| g.days.iter().any(|d| day_of(*d) == now_day - k)).collect();
                let dw = 7.0 * 7.0 + 6.0 * 3.0;
                for (k, on) in dots.iter().enumerate() {
                    let c = pos2(r.center().x - dw / 2.0 + 3.5 + k as f32 * 10.0, r.min.y + 76.0);
                    p.circle_filled(c, 3.5, if *on { GREEN } else { Color32::from_rgb(47, 47, 67) });
                }
                let last = g.days.last().copied().unwrap_or(0);
                let status = if out { "Out today".to_string() } else { format!("Last: {}", when(last)) };
                p.text(pos2(r.center().x, r.max.y - 14.0), Align2::CENTER_CENTER, status, crate::fonts::sans_font(11.0), if out { GREEN } else { MUTED() });
                let tip = format!(
                    "{name}\nSeen on {} day{}: {}\n{} rare grass tiles in all\n{}",
                    g.days.len(),
                    if g.days.len() == 1 { "" } else { "s" },
                    g.days.iter().rev().take(8).map(|d| crate::stats::fmt_day_short(day_of(*d))).collect::<Vec<_>>().join(", "),
                    g.tiles,
                    if out { "Click to show it on the Map" } else { "Click to open its Pokédex entry" }
                );
                if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    clicked = Some(g.slug.clone());
                }
            }
        });
        if let Some(slug) = clicked {
            if let Some(o) = out_now(&slug) {
                let s = o.spots[0].clone();
                if self.has_app(AppId::Map) {
                    self.go_to_rare(&s);
                } else {
                    self.open_app(AppId::Map);
                }
            } else if let Some(id) = self.rare_wiki_for(&slug) {
                if self.has_app(AppId::Pokedex) {
                    self.open_dex_entry(&id);
                } else {
                    self.open_app(AppId::Pokedex);
                }
            }
        }
    }
}
