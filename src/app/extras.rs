//! Shiny Dex collection log, daily goals and weekly challenges, the hunt
//! journal, sound effects, today's timer and undo.

use super::ui_kit::{self as kit, MUTED, CARD, CARD_HOVER, LINE, TEXT};
use super::{App, RunRef, Tab};
use crate::config::{GoalDone, HuntRecord};
use crate::goals::{self, WeekStats};
use crate::moddex::{self, Region};
use crate::sound::Sfx;
use crate::stats::{fmt_clock, fmt_date, fmt_day_short, local_day, now_unix, Note, RunStats};
use crate::theme::{self, GOLD};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum DexShow {
    All,
    Caught,
    Missing,
}

pub(super) struct DexView {
    /// Caught species being looked at (its catches / trophy cards).
    pub(super) open: Option<String>,
    search: String,
    region: Option<Region>,
    show: DexShow,
    unobtainable: bool,
}

impl Default for DexView {
    fn default() -> Self {
        Self { open: None, search: String::new(), region: None, show: DexShow::All, unobtainable: true }
    }
}

/// Target sightings in a run.
pub(super) struct TargetWatch {
    pub seen: u64,
    /// Encounters since the target was last seen (None = unknown).
    pub since: Option<u64>,
    /// Hasn't turned up for much longer than usual: maybe the wrong spot.
    pub dry: bool,
}

impl TargetWatch {
    pub(super) fn new(count: u64, seen: u64, last: u64) -> Self {
        let since = if last > 0 && count >= last {
            Some(count - last)
        } else if seen == 0 {
            Some(count)
        } else {
            None
        };
        let dry = match since {
            Some(n) if seen == 0 => n >= 150,
            Some(n) if seen >= 3 => n > (5 * count / seen).max(60),
            _ => false,
        };
        Self { seen, since, dry }
    }

    pub(super) fn tip(&self, name: &str) -> String {
        let mut t = format!("{name} seen {} times this run.", theme::thousands(self.seen));
        if let Some(n) = self.since {
            if self.seen > 0 {
                t.push_str(&format!("\nLast one {} encounters ago.", theme::thousands(n)));
            }
        }
        if self.dry {
            t.push_str(&format!(
                "\n{name} hasn't turned up for a while. Is it on this route, at this time of day, with this method?"
            ));
        }
        t
    }
}

/// Where a shiny in the dex came from.
#[derive(Clone)]
pub(super) enum DexSource {
    /// Caught on a hunt: date and encounters.
    Hunt(u64, u64),
    Manual,
}

/// Tile grid shared by the Shiny Dex and the target picker.
pub(super) const TILE_H: f32 = 98.0;
pub(super) const TILE_GAP: f32 = 6.0;
const TILE_MIN_W: f32 = 86.0;

/// Columns and tile width that exactly fill `avail`.
pub(super) fn grid_cols(avail: f32) -> (usize, f32) {
    let cols = (((avail + TILE_GAP) / (TILE_MIN_W + TILE_GAP)).floor() as usize).max(1);
    let w = ((avail - TILE_GAP * (cols - 1) as f32) / cols as f32).floor();
    (cols, w)
}

/// How a grid tile is coloured.
pub(super) struct TileLook {
    pub fill: Color32,
    pub stroke: Stroke,
    pub name: Color32,
    pub num: Color32,
    /// Multiplies the sprite (black = silhouette).
    pub tint: Color32,
    pub shiny: bool,
}

impl App {
    /// One grid tile: sprite on top, number and name bottom-left.
    pub(super) fn grid_tile(&mut self, p: &egui::Painter, r: Rect, sp: &crate::species::Species, look: &TileLook) {
        p.rect_filled(r, kit::r(10.0), look.fill);
        p.rect_stroke(r, kit::r(10.0), look.stroke);
        let stage = Rect::from_min_size(r.min + egui::vec2(5.0, 5.0), egui::vec2(r.width() - 10.0, 62.0));
        self.draw_sprite_tinted(p, sp, look.shiny, stage.center_bottom() - egui::vec2(0.0, 2.0), egui::vec2(stage.width().min(72.0), 58.0), look.tint);
        p.text(
            egui::pos2(r.min.x + 8.0, r.max.y - 24.0),
            Align2::LEFT_CENTER,
            format!("#{:04}", sp.dex),
            kit::font_small(10.0),
            look.num,
        );
        let clip = p.with_clip_rect(Rect::from_min_max(r.min, egui::pos2(r.max.x - 4.0, r.max.y)));
        clip.text(egui::pos2(r.min.x + 8.0, r.max.y - 11.0), Align2::LEFT_CENTER, sp.name, kit::font(11.5), look.name);
    }

    // ───────────────────────────── sounds / timer / undo ─────────────────────────────

    pub(super) fn play(&self, s: Sfx) {
        if self.cfg.sounds.on(s) {
            self.play_now(s);
        }
    }

    /// Play a sound effect regardless of its switch (previews): the user's
    /// own file if they picked one and it plays, otherwise the built-in one.
    pub(super) fn play_now(&self, s: Sfx) {
        let snd = &self.cfg.sounds;
        if let Some(p) = snd.custom_for(s) {
            if crate::sound::play_custom(s, p, snd.volume) {
                return;
            }
        }
        crate::sound::play(s, snd.volume, &snd.device);
    }

    /// A shiny was spotted: the alert song if it's on and the hunt is long
    /// enough, otherwise the "Shiny spotted!" sound.
    pub(super) fn shiny_fanfare(&mut self) {
        let s = &self.cfg.sounds;
        if s.enabled
            && s.alert_song
            && self.cfg.count >= s.alert_song_after
            && crate::sound::play_song(&s.alert_song_path, s.volume)
        {
            return;
        }
        self.play(Sfx::Shiny);
    }

    /// Hunting time today across every run, ticking live between encounters.
    pub(super) fn today_hunt_secs(&self) -> u64 {
        let now = now_unix();
        let today = local_day(now);
        let mut secs = self.cfg.run_stats.today(today).1;
        for r in &self.cfg.saved_runs {
            if r.id != self.cfg.run_id {
                secs += r.stats.today(today).1;
            }
        }
        for h in &self.cfg.history {
            if let Some(st) = &h.stats {
                secs += st.today(today).1;
            }
        }
        let last = self.cfg.run_stats.last_unix;
        if last > 0 && now >= last && now - last <= self.cfg.idle_gap_secs() && local_day(last) == today && !self.cfg.paused && !self.game_closed() {
            secs += now - last;
        }
        secs
    }

    /// The timer has stopped because no encounter came for a while (it
    /// carries on with the next one).
    pub(super) fn timer_idle(&self) -> bool {
        let last = self.cfg.run_stats.last_unix;
        let now = now_unix();
        last > 0 && now.saturating_sub(last) > self.cfg.idle_gap_secs() && local_day(last) == local_day(now)
    }

    /// Undo the last battle's encounters (or 1 if there's nothing to undo).
    pub(super) fn undo_last(&mut self) {
        let n = self.last_added.take().unwrap_or(1).max(1);
        self.add(-(n as i64));
    }

    // ───────────────────────────── target sightings ─────────────────────────────

    /// How often the target turns up in the current run (wild hunts only).
    pub(super) fn target_watch(&self) -> Option<TargetWatch> {
        self.cfg.target.as_ref()?;
        if !self.cfg.hunt_method.is_wild_battles() {
            return None;
        }
        Some(TargetWatch::new(self.cfg.count, self.cfg.target_seen, self.cfg.run_stats.target_last))
    }

    /// What the odds bar counts: (number, "encounters" / "Riolu seen").
    pub(super) fn odds_count(&self) -> (u64, String) {
        if self.cfg.odds_target_only && self.target_watch().is_some() {
            let name = self.target_name(self.cfg.target.as_deref());
            (self.cfg.target_seen, format!("{name} encounters"))
        } else {
            (self.cfg.count, "encounters".to_string())
        }
    }

    // ───────────────────────────── phases ─────────────────────────────

    /// History indices of the phases found during run `run_id`.
    pub(super) fn phases_of(&self, run_id: u64) -> Vec<usize> {
        if run_id == 0 {
            return Vec::new();
        }
        (0..self.cfg.history.len())
            .filter(|&i| self.cfg.history[i].phase && self.cfg.history[i].run_id == run_id)
            .collect()
    }

    /// A shiny of another Pokémon: logged with its own trophy; the hunt goes on.
    pub(super) fn log_phase(&mut self, slug: String) {
        let Some(sp) = self.species.iter().find(|s| s.slug == slug).cloned() else { return };
        let hunting = self.target_name(self.cfg.target.as_deref());
        let now = now_unix();
        // Undo is offered on the overlay for a few seconds.
        self.undo_caught = Some((std::time::Instant::now(), Box::new(self.cfg.clone())));
        self.cfg.history.push(HuntRecord {
            target: sp.name.to_string(),
            encounters: self.cfg.count,
            battles: self.cfg.battles,
            when_unix: now,
            slug: Some(slug),
            started_unix: self.cfg.run_started_unix,
            target_seen: 0,
            method: format!("Phase during the {hunting} hunt"),
            odds_one_in: None,
            stats: None,
            phase: true,
            run_id: self.cfg.run_id,
        });
        self.cfg.run_stats.notes.push(Note { unix: now, count: self.cfg.count, text: format!("Phase: shiny {}!", sp.name) });
        self.dirty = true;
        self.play(Sfx::Caught);
        self.check_progress();
    }

    /// Settings page: which shiny was the phase?
    pub(super) fn page_phase(&mut self, ui: &mut egui::Ui) {
        if super::hunt_pages::back_button(ui, "Cancel") {
            self.setup.phase_pick = false;
            return;
        }
        let hunting = self.target_name(self.cfg.target.as_deref());
        kit::page_header(
            ui,
            "LOG A PHASE",
            &format!(
                "Which shiny did you find? It gets its own trophy and Shiny Dex entry, and your {hunting} hunt keeps counting from {}.",
                theme::thousands(self.cfg.count)
            ),
        );
        if let Some(slug) = self.target_picker(ui, 480.0) {
            self.log_phase(slug.clone());
            self.setup.phase_pick = false;
            self.setup.tab = Tab::Dex;
            self.setup.dex.open = Some(slug);
            self.setup.view_run = None;
            self.setup.share = None;
        }
    }

    /// Run stats: the phases found during this run.
    pub(super) fn phases_section(&mut self, ui: &mut egui::Ui, run_id: u64) {
        let list = self.phases_of(run_id);
        if list.is_empty() {
            return;
        }
        kit::section(ui, if list.len() == 1 { "Phase" } else { "Phases" });
        let mut share = None;
        kit::card(ui, |ui| {
            for (k, &i) in list.iter().enumerate() {
                if k > 0 {
                    ui.add_space(6.0);
                }
                let h = self.cfg.history[i].clone();
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(40.0, 36.0), Sense::hover());
                    if let Some((_, sp)) = self.species_of(h.slug.as_deref()) {
                        let p = ui.painter_at(r);
                        self.draw_sprite(&p, &sp, true, r.center_bottom(), egui::vec2(40.0, 36.0));
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("✨ Shiny {}", h.target)).size(kit::fs(13.5)).family(kit::strong_family(kit::fs(13.5))).strong().color(GOLD));
                        ui.label(
                            RichText::new(format!("at {} encounters · {}", theme::thousands(h.encounters), fmt_date(h.when_unix)))
                                .size(kit::fs(11.5))
                                .color(MUTED()),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("🖼 Share").fill(kit::btn_fill())).clicked() {
                            share = Some(i);
                        }
                    });
                });
            }
        });
        if let Some(i) = share {
            self.open_share(RunRef::Finished(i), None);
        }
    }

    // ───────────────────────────── encounter breakdown ─────────────────────────────


    // ───────────────────────────── shiny dex ─────────────────────────────

    /// Every shiny species collected: slug -> where it came from.
    pub(super) fn dex_caught(&self) -> BTreeMap<String, DexSource> {
        let mut m = BTreeMap::new();
        for s in &self.cfg.dex_manual {
            m.insert(s.clone(), DexSource::Manual);
        }
        for h in &self.cfg.history {
            let slug = h.slug.clone().or_else(|| {
                self.species.iter().find(|s| s.name.eq_ignore_ascii_case(&h.target)).map(|s| s.slug.clone())
            });
            if let Some(slug) = slug {
                let keep_earlier = matches!(m.get(&slug), Some(DexSource::Hunt(t, _)) if *t <= h.when_unix);
                if !keep_earlier {
                    m.insert(slug, DexSource::Hunt(h.when_unix, h.encounters));
                }
            }
        }
        m
    }

    /// (species in the dex, regions complete) for achievements.
    pub(super) fn dex_progress(&self) -> (u32, [bool; 5]) {
        let caught = self.dex_caught();
        let mut done = [true; 5];
        for e in moddex::entries() {
            if e.unobtainable {
                continue;
            }
            let slug = &self.species[(e.dex - 1) as usize].slug;
            if !caught.contains_key(slug) {
                let i = Region::ALL.iter().position(|r| *r == e.region).unwrap_or(0);
                done[i] = false;
            }
        }
        (caught.len() as u32, done)
    }

    pub(super) fn page_dex(&mut self, ui: &mut egui::Ui) {
        if let Some(slug) = self.setup.dex.open.clone() {
            self.page_dex_entry(ui, &slug);
            return;
        }
        kit::page_header(
            ui,
            "SHINY DEX",
            &format!(
                "Every Pokémon in Platinum Together (wiki data v{}). Click a caught shiny for its trophy; right-click to mark one you caught before.",
                moddex::WIKI_VERSION
            ),
        );
        let caught = self.dex_caught();
        let entries = moddex::entries();
        let slug_of = |dex: u16| self.species[(dex - 1) as usize].slug.clone();

        // overall + per region progress
        let obtainable: Vec<_> = entries.iter().filter(|e| !e.unobtainable).collect();
        let got = obtainable.iter().filter(|e| caught.contains_key(&slug_of(e.dex))).count();
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{got} / {}", obtainable.len()))
                        .font(kit::display_font(18.0))
                        .color(if kit::clean() { theme::LINK() } else { GOLD }),
                );
                ui.label(RichText::new("obtainable shinies collected").size(kit::fs(13.0)).color(MUTED()));
                let hunts: Vec<&HuntRecord> = self.cfg.history.iter().filter(|h| !h.phase).collect();
                let n = hunts.len();
                let phases = self.cfg.history.len() - n;
                if n > 0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let total: u64 = hunts.iter().map(|h| h.encounters).sum();
                        let mut txt = format!(
                            "{n} caught on hunts  ·  avg {} encounters",
                            theme::thousands(total / n as u64)
                        );
                        if phases > 0 {
                            txt.push_str(&format!("  ·  {phases} phase{}", if phases == 1 { "" } else { "s" }));
                        }
                        if let Some(h) = hunts.iter().min_by_key(|h| h.encounters) {
                            txt.push_str(&format!("  ·  luckiest {} at {}", h.target, theme::thousands(h.encounters)));
                        }
                        ui.label(RichText::new(txt).size(kit::fs(11.5)).color(MUTED()));
                    });
                }
            });
            ui.add_space(6.0);
            let (br, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 10.0), Sense::hover());
            let pp = ui.painter();
            pp.rect_filled(br, 5.0, CARD_HOVER());
            let frac = got as f32 / obtainable.len().max(1) as f32;
            if frac > 0.0 {
                pp.add(theme::button_shape(
                    Rect::from_min_size(br.min, egui::vec2((br.width() * frac).max(10.0), br.height())),
                    5.0,
                    false,
                ));
            }
            ui.add_space(10.0);
            let w5 = ((ui.available_width() - 32.0) / 5.0).floor();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for r in Region::ALL {
                    let all: Vec<_> = obtainable.iter().filter(|e| e.region == r).collect();
                    let g = all.iter().filter(|e| caught.contains_key(&slug_of(e.dex))).count();
                    let (rr, _) = ui.allocate_exact_size(egui::vec2(w5, 42.0), Sense::hover());
                    let p = ui.painter();
                    let complete = g == all.len() && !all.is_empty();
                    p.rect_filled(rr, 8.0, if complete { GOLD.gamma_multiply(0.25) } else { CARD_HOVER() });
                    p.text(rr.min + egui::vec2(10.0, 13.0), Align2::LEFT_CENTER, r.label(), kit::font(12.5), TEXT());
                    p.text(
                        egui::pos2(rr.max.x - 10.0, rr.min.y + 13.0),
                        Align2::RIGHT_CENTER,
                        format!("{g}/{}", all.len()),
                        kit::font(11.5),
                        if complete { GOLD } else { MUTED() },
                    );
                    let bar = Rect::from_min_size(egui::pos2(rr.min.x + 10.0, rr.max.y - 12.0), egui::vec2(rr.width() - 20.0, 5.0));
                    p.rect_filled(bar, 2.5, LINE());
                    let f = g as f32 / all.len().max(1) as f32;
                    if f > 0.0 {
                        p.rect_filled(Rect::from_min_size(bar.min, egui::vec2((bar.width() * f).max(5.0), 5.0)), 2.5, GOLD);
                    }
                }
            });
        });

        // filters
        ui.horizontal(|ui| {
            // Search box, filter group and checkbox all share the 30px
            // control height so the row lines up.
            kit::search_box(ui, &mut self.setup.dex.search, "Search name or number", 200.0);
            kit::segmented(
                ui,
                &mut self.setup.dex.show,
                &[(DexShow::All, "  All  "), (DexShow::Caught, "  Caught  "), (DexShow::Missing, "  Missing  ")],
            );
            kit::checkbox(ui, &mut self.setup.dex.unobtainable, "Unobtainable")
                .on_hover_text("Show Pokémon that can't be obtained in the game yet");
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            let mut sel = self.setup.dex.region;
            let labels: Vec<String> = Region::ALL.iter().map(|r| format!("  {}  ", r.label())).collect();
            let mut refs: Vec<(Option<Region>, &str)> = vec![(None, "  All regions  ")];
            for (r, l) in Region::ALL.iter().zip(&labels) {
                refs.push((Some(*r), l.as_str()));
            }
            kit::segmented(ui, &mut sel, &refs);
            self.setup.dex.region = sel;
        });
        ui.add_space(10.0);

        // grid
        let q = self.setup.dex.search.trim().to_lowercase();
        let show = self.setup.dex.show;
        let region = self.setup.dex.region;
        let unob = self.setup.dex.unobtainable;
        let list: Vec<_> = entries
            .iter()
            .filter(|e| region.map_or(true, |r| e.region == r))
            .filter(|e| unob || !e.unobtainable)
            .filter(|e| {
                let has = caught.contains_key(&slug_of(e.dex));
                match show {
                    DexShow::All => true,
                    DexShow::Caught => has,
                    DexShow::Missing => !has,
                }
            })
            .filter(|e| {
                q.is_empty()
                    || self.species[(e.dex - 1) as usize].name.to_lowercase().contains(&q)
                    || e.dex.to_string() == q.trim_start_matches('#').trim_start_matches('0')
            })
            .copied()
            .collect();
        if list.is_empty() {
            ui.label(RichText::new("Nothing matches.").size(kit::fs(12.5)).color(MUTED()));
            return;
        }
        let gap = TILE_GAP;
        let (cols, tw) = grid_cols(ui.available_width());
        let tile = egui::vec2(tw, TILE_H);
        let mut toggle: Option<(String, bool)> = None;
        let mut open: Option<String> = None;
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for e in row {
                    let sp = self.species[(e.dex - 1) as usize].clone();
                    let src = caught.get(&sp.slug).cloned();
                    let (r, resp) = ui.allocate_exact_size(tile, Sense::click());
                    if !ui.is_rect_visible(r) {
                        continue;
                    }
                    let p = ui.painter_at(r.expand(2.0));
                    let has = src.is_some();
                    let look = TileLook {
                        fill: if has && !kit::clean() {
                            GOLD.gamma_multiply(0.16)
                        } else if resp.hovered() {
                            CARD_HOVER()
                        } else {
                            CARD()
                        },
                        stroke: Stroke::new(if has { 1.5f32 } else { 1.0f32 }, if has { GOLD } else { LINE() }),
                        name: if has { TEXT() } else { MUTED() },
                        num: if has { if kit::clean() { theme::LINK() } else { GOLD } } else { MUTED() },
                        tint: if has {
                            Color32::WHITE
                        } else if e.unobtainable {
                            Color32::from_rgba_unmultiplied(10, 8, 18, 110)
                        } else {
                            Color32::from_rgba_unmultiplied(10, 8, 18, 200)
                        },
                        shiny: has,
                    };
                    self.grid_tile(&p, r, &sp, &look);
                    if has {
                        theme::sparkle(&p, r.right_top() + egui::vec2(-9.0, 9.0), 4.5, GOLD);
                    }
                    if e.unobtainable {
                        // padlock
                        let c = r.right_top() + egui::vec2(-11.0, 12.0);
                        p.rect_filled(Rect::from_center_size(c + egui::vec2(0.0, 2.0), egui::vec2(9.0, 7.0)), 1.5, MUTED());
                        p.circle_stroke(c - egui::vec2(0.0, 1.5), 3.0, Stroke::new(1.3f32, MUTED()));
                    }
                    let types = {
                        let mut t = moddex::TYPES[e.types.0 as usize].to_string();
                        if let Some(t2) = moddex::TYPES.get(e.types.1 as usize) {
                            t.push('/');
                            t.push_str(t2);
                        }
                        t
                    };
                    let info = match &src {
                        Some(DexSource::Hunt(t, n)) => format!("Shiny caught {} after {} encounters", fmt_date(*t), theme::thousands(*n)),
                        Some(DexSource::Manual) => "Shiny marked by hand".to_string(),
                        None if e.unobtainable => "Can't be obtained in the game yet".to_string(),
                        None => "Not caught yet".to_string(),
                    };
                    let resp = resp.on_hover_text(format!(
                        "#{:04} {}  ·  {}  ·  {}\n{}{}",
                        e.dex,
                        sp.name,
                        types,
                        e.region.label(),
                        info,
                        if has { "\nClick for details" } else { "" }
                    ));
                    if has && resp.clicked() {
                        open = Some(sp.slug.clone());
                    }
                    let resp = if has { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
                    let manual = matches!(src, Some(DexSource::Manual));
                    let slug = sp.slug.clone();
                    resp.context_menu(|ui| {
                        if !has {
                            if ui.button("✨ Mark as caught (shiny)").clicked() {
                                toggle = Some((slug.clone(), true));
                                ui.close_menu();
                            }
                        } else if manual {
                            if ui.button("Remove the mark").clicked() {
                                toggle = Some((slug.clone(), false));
                                ui.close_menu();
                            }
                        } else {
                            ui.label("Caught on a hunt: click it to see the trophy");
                        }
                    });
                }
            });
            ui.add_space(gap);
        }
        if open.is_some() {
            self.setup.dex.open = open;
            self.setup.note_draft.clear();
        }
        if let Some((slug, on)) = toggle {
            if on {
                if !self.cfg.dex_manual.contains(&slug) {
                    self.cfg.dex_manual.push(slug);
                }
            } else {
                self.cfg.dex_manual.retain(|s| *s != slug);
            }
            self.dirty = true;
            self.check_progress();
        }
    }

    /// One caught species: its trophy cards (every catch), or the hand mark.
    fn page_dex_entry(&mut self, ui: &mut egui::Ui, slug: &str) {
        if super::hunt_pages::back_button(ui, "Shiny Dex") {
            self.setup.dex.open = None;
            return;
        }
        let Some(idx) = self.species.iter().position(|s| s.slug == slug) else {
            self.setup.dex.open = None;
            return;
        };
        let sp = self.species[idx].clone();
        self.add_event("trophy_room");
        let catches: Vec<usize> = (0..self.cfg.history.len())
            .rev()
            .filter(|&i| {
                let h = &self.cfg.history[i];
                h.slug.as_deref() == Some(slug) || (h.slug.is_none() && h.target.eq_ignore_ascii_case(sp.name))
            })
            .collect();
        let manual = self.cfg.dex_manual.iter().any(|s| s == slug);
        let entry = moddex::entries().into_iter().find(|e| e.dex as u32 == sp.dex);
        ui.add_space(6.0);
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(150.0, 130.0), Sense::hover());
                let p = ui.painter_at(r.expand(4.0));
                p.rect_filled(r, kit::r(12.0), CARD_HOVER());
                theme::platform(&p, r.center_bottom() - egui::vec2(0.0, 16.0), r.width() * 0.32, 9.0);
                self.draw_sprite(&p, &sp, true, r.center_bottom() - egui::vec2(0.0, 14.0), egui::vec2(130.0, 112.0));
                theme::sparkle(&p, r.right_top() + egui::vec2(-14.0, 14.0), 6.0, GOLD);
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(if kit::clean() { format!("Shiny {}", sp.name) } else { format!("SHINY {}", sp.name.to_uppercase()) })
                            .font(kit::display_font(15.0))
                            .color(if kit::clean() { theme::LINK() } else { GOLD }),
                    );
                    ui.add_space(4.0);
                    let mut sub = vec![format!("#{:04}", sp.dex)];
                    if let Some(e) = &entry {
                        let mut t = moddex::TYPES[e.types.0 as usize].to_string();
                        if let Some(t2) = moddex::TYPES.get(e.types.1 as usize) {
                            t.push('/');
                            t.push_str(t2);
                        }
                        sub.push(t);
                        sub.push(e.region.label().to_string());
                    }
                    ui.label(RichText::new(sub.join("  ·  ")).size(kit::fs(13.0)).color(MUTED()));
                    ui.add_space(8.0);
                    let line = if catches.is_empty() {
                        "Marked as caught by hand".to_string()
                    } else {
                        let hs: Vec<_> = catches.iter().map(|&i| &self.cfg.history[i]).collect();
                        let first = hs.iter().map(|h| h.when_unix).min().unwrap_or(0);
                        let best = hs.iter().map(|h| h.encounters).min().unwrap_or(0);
                        if hs.len() == 1 {
                            format!("Caught {} after {} encounters", fmt_date(first), theme::thousands(best))
                        } else {
                            format!(
                                "Caught {} times  ·  first {}  ·  best {} encounters",
                                hs.len(),
                                fmt_date(first),
                                theme::thousands(best)
                            )
                        }
                    };
                    ui.label(RichText::new(line).size(kit::fs(14.0)).family(kit::strong_family(kit::fs(14.0))).strong().color(TEXT()));
                });
            });
        });
        if !catches.is_empty() {
            kit::section(ui, if catches.len() == 1 { "Trophy" } else { "Trophies" });
            self.trophy_grid(ui, &catches);
        }
        if manual {
            kit::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if catches.is_empty() {
                            "You marked this one by hand, so there are no hunt details. Press Caught! on a run next time and it gets a full trophy."
                        } else {
                            "Also marked by hand."
                        })
                        .size(kit::fs(12.0))
                        .color(MUTED()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("Remove the mark").fill(kit::btn_fill())).clicked() {
                            self.cfg.dex_manual.retain(|s| s != slug);
                            self.dirty = true;
                            if catches.is_empty() {
                                self.setup.dex.open = None;
                            }
                        }
                    });
                });
            });
        }
    }

    // ───────────────────────────── goals ─────────────────────────────

    /// What happened in a week, across every run.
    pub(super) fn week_stats(&self, week: i64) -> WeekStats {
        let (a, b) = goals::week_days(week);
        let mut w = WeekStats::default();
        let mut add = |st: &RunStats, target: Option<&String>| {
            let mut active = false;
            for d in st.days.iter().filter(|d| d.day >= a && d.day <= b) {
                let e = w.days.entry(d.day).or_insert((0, 0));
                e.0 += d.encounters;
                e.1 += d.secs;
                active |= d.encounters > 0;
            }
            for s in st.sessions.iter().filter(|s| {
                let d = local_day(s.start);
                d >= a && d <= b
            }) {
                w.sessions.push((s.encounters, s.secs));
            }
            if active {
                if let Some(t) = target {
                    w.targets.insert(t.clone());
                }
            }
        };
        add(&self.cfg.run_stats, self.cfg.target.as_ref());
        for r in &self.cfg.saved_runs {
            if r.id != self.cfg.run_id {
                add(&r.stats, r.target.as_ref());
            }
        }
        for h in &self.cfg.history {
            if let Some(st) = &h.stats {
                let t = h.slug.clone().unwrap_or_else(|| h.target.to_lowercase());
                add(st, Some(&t));
            }
        }
        w
    }

    fn goal_done(&self, key: &str) -> bool {
        self.cfg.goals_done.iter().any(|g| g.key == key)
    }

    /// Record today's goal and this week's challenges when they're met.
    /// Returns (title, xp) for each one just completed.
    pub(super) fn check_goals(&mut self) -> Vec<(String, u64)> {
        let now = now_unix();
        let today = local_day(now);
        let week = goals::week_of(today);
        let ws = self.week_stats(week);
        let mut out = Vec::new();
        let today_enc = ws.days.get(&today).map_or(0, |d| d.0);
        let dk = goals::day_key(today);
        if self.cfg.daily_goal > 0 && today_enc >= self.cfg.daily_goal && !self.goal_done(&dk) {
            self.cfg.goals_done.push(GoalDone { key: dk, xp: goals::DAILY_XP, unix: now });
            out.push(("Daily goal reached!".to_string(), goals::DAILY_XP));
        }
        for c in goals::challenges(week) {
            let k = goals::week_key(week, &c);
            if goals::done(&c, &ws) && !self.goal_done(&k) {
                self.cfg.goals_done.push(GoalDone { key: k, xp: goals::WEEKLY_XP, unix: now });
                out.push((c.title.to_string(), goals::WEEKLY_XP));
            }
        }
        if !out.is_empty() {
            self.dirty = true;
        }
        out
    }

    pub(super) fn goal_xp(&self) -> u64 {
        self.cfg.goals_done.iter().map(|g| g.xp).sum()
    }

    pub(super) fn daily_streak(&self) -> u32 {
        let days: BTreeSet<i64> = self
            .cfg
            .goals_done
            .iter()
            .filter_map(|g| g.key.strip_prefix("day:").and_then(|d| d.parse().ok()))
            .collect();
        goals::streak(&days, local_day(now_unix()))
    }

    /// (done, total) of today's goal + this week's challenges, for the tab chip.
    pub(super) fn goals_done_count(&self) -> (usize, usize) {
        let today = local_day(now_unix());
        let week = goals::week_of(today);
        let cs = goals::challenges(week);
        let mut n = self.goal_done(&goals::day_key(today)) as usize;
        for c in &cs {
            n += self.goal_done(&goals::week_key(week, c)) as usize;
        }
        (n, cs.len() + 1)
    }

    /// The Goals tab on the Profile page: side by side when there's room.
    pub(super) fn goals_section(&mut self, ui: &mut egui::Ui) {
        let today = local_day(now_unix());
        let ws = self.week_stats(goals::week_of(today));
        if ui.available_width() >= 820.0 {
            ui.columns(2, |cols| {
                cols[0].label(RichText::new("Today").size(kit::fs(12.0)).color(MUTED()));
                cols[0].add_space(4.0);
                self.daily_goal_card(&mut cols[0], &ws);
                self.weekly_card(&mut cols[1], &ws);
            });
        } else {
            ui.label(RichText::new("Today").size(kit::fs(12.0)).color(MUTED()));
            ui.add_space(4.0);
            self.daily_goal_card(ui, &ws);
            ui.add_space(6.0);
            self.weekly_card(ui, &ws);
        }
    }

    fn daily_goal_card(&mut self, ui: &mut egui::Ui, ws: &WeekStats) {
        let today = local_day(now_unix());
        let today_enc = ws.days.get(&today).map_or(0, |d| d.0);
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                // progress ring
                let (r, _) = ui.allocate_exact_size(egui::vec2(84.0, 84.0), Sense::hover());
                let p = ui.painter();
                let c = r.center();
                let goal = self.cfg.daily_goal.max(1);
                let frac = (today_enc as f32 / goal as f32).min(1.0);
                p.circle_stroke(c, 34.0, Stroke::new(8.0f32, CARD_HOVER()));
                if frac > 0.0 {
                    let n = ((48.0 * frac) as usize).max(2);
                    let pts: Vec<egui::Pos2> = (0..=n)
                        .map(|i| {
                            let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * frac * i as f32 / n as f32;
                            c + egui::vec2(a.cos(), a.sin()) * 34.0
                        })
                        .collect();
                    p.add(egui::Shape::line(pts, Stroke::new(8.0f32, if frac >= 1.0 { GOLD } else { theme::BUTTON() })));
                }
                p.text(c - egui::vec2(0.0, 6.0), Align2::CENTER_CENTER, theme::thousands(today_enc), kit::font(16.0), TEXT());
                p.text(c + egui::vec2(0.0, 12.0), Align2::CENTER_CENTER, format!("of {}", theme::thousands(goal)), kit::font(10.5), MUTED());
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    let done = today_enc >= goal;
                    ui.label(
                        RichText::new(if done { "Daily goal reached! ✔" } else { "Today's goal" })
                            .size(kit::fs(15.0))
                            .family(kit::strong_family(kit::fs(15.0))).strong()
                            .color(if done { theme::GREEN } else { TEXT() }),
                    );
                    ui.label(
                        RichText::new(format!("Encounter {} Pokémon today for +{} XP.", theme::thousands(goal), goals::DAILY_XP))
                            .size(kit::fs(11.5))
                            .color(MUTED()),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Goal").size(kit::fs(12.0)).color(MUTED()));
                        let mut g = (self.cfg.daily_goal / 100) as u32;
                        if kit::stepper(ui, &mut g, 1, 100) {
                            self.cfg.daily_goal = g as u64 * 100;
                            self.dirty = true;
                        }
                        // the stepper counts in hundreds: say what that means
                        ui.label(RichText::new(format!("{} encounters a day", theme::thousands(self.cfg.daily_goal))).size(kit::fs(12.0)).color(MUTED()));
                    });
                    let streak = self.daily_streak();
                    ui.label(
                        RichText::new(if streak > 0 {
                            format!("Streak: {streak} day{} in a row", if streak == 1 { "" } else { "s" })
                        } else {
                            "Streak: start one today!".to_string()
                        })
                        .size(kit::fs(12.5))
                        .family(kit::strong_family(kit::fs(12.5))).strong()
                        .color(GOLD),
                    );
                });
            });
        });
    }

    fn weekly_card(&mut self, ui: &mut egui::Ui, ws: &WeekStats) {
        let today = local_day(now_unix());
        let week = goals::week_of(today);
        let (_, end) = goals::week_days(week);
        let left = end - today + 1;
        ui.label(
            RichText::new(format!(
                "This week's challenges  ·  +{} XP each  ·  new ones in {left} day{}",
                goals::WEEKLY_XP,
                if left == 1 { "" } else { "s" }
            ))
            .size(kit::fs(12.0))
            .color(MUTED()),
        );
        ui.add_space(4.0);
        kit::card(ui, |ui| {
            for (i, c) in goals::challenges(week).iter().enumerate() {
                if i > 0 {
                    ui.add_space(6.0);
                }
                let (cur, target) = goals::progress(c, ws);
                let done = self.goal_done(&goals::week_key(week, c));
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::hover());
                    theme::medal(ui.painter(), r.center() - egui::vec2(0.0, 3.0), 9.0, [246, 200, 66], !done);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(c.title).size(kit::fs(13.5)).family(kit::strong_family(kit::fs(13.5))).strong().color(if done { TEXT() } else { MUTED() }));
                        let (br, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(420.0), 6.0), Sense::hover());
                        let pp = ui.painter();
                        pp.rect_filled(br, 3.0, LINE());
                        let f = (cur / target).clamp(0.0, 1.0) as f32;
                        if f > 0.0 {
                            pp.rect_filled(Rect::from_min_size(br.min, egui::vec2((br.width() * f).max(6.0), 6.0)), 3.0, if done { theme::GREEN } else { GOLD });
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let txt = if done {
                            "✔ Done".to_string()
                        } else if matches!(c.kind, goals::Kind::Hours(_)) {
                            format!("{:.1} / {:.0} h", cur, target)
                        } else {
                            format!("{} / {}", theme::thousands(cur as u64), theme::thousands(target as u64))
                        };
                        ui.label(RichText::new(txt).size(kit::fs(11.5)).color(if done { theme::GREEN } else { MUTED() }));
                    });
                });
            }
        });
    }

    // ───────────────────────────── hunt journal ─────────────────────────────

    fn notes_mut(&mut self, run: RunRef) -> Option<&mut Vec<Note>> {
        match run {
            RunRef::Current => Some(&mut self.cfg.run_stats.notes),
            RunRef::Saved(id) => self.cfg.saved_runs.iter_mut().find(|r| r.id == id).map(|r| &mut r.stats.notes),
            RunRef::Finished(i) => self.cfg.history.get_mut(i).map(|h| &mut h.stats.get_or_insert_with(Default::default).notes),
        }
    }

    /// Journal section on a run's stats page.
    pub(super) fn journal_section(&mut self, ui: &mut egui::Ui, run: RunRef, count: u64) {
        kit::section(ui, "Hunt journal");
        let notes: Vec<Note> = self.notes_mut(run).map(|n| n.clone()).unwrap_or_default();
        let mut add = false;
        let mut remove = None;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.setup.note_draft)
                        .hint_text("Add a note, e.g. \"switched to Route 214 here\"")
                        .char_limit(120)
                        .desired_width(ui.available_width() - 90.0),
                );
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if super::hunt_pages::primary(ui, "  Add  ").clicked() || enter {
                    add = true;
                }
            });
            if notes.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new("No notes yet. They're saved with this run and can go on share images.").size(kit::fs(11.5)).color(MUTED()));
            }
            for (i, n) in notes.iter().enumerate().rev() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(8.0, 30.0), Sense::hover());
                    ui.painter().rect_filled(Rect::from_min_size(r.min + egui::vec2(2.0, 2.0), egui::vec2(3.0, 26.0)), 1.5, GOLD);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&n.text).size(kit::fs(13.0)).color(TEXT()));
                        ui.label(
                            RichText::new(format!(
                                "{} · {} · at {} encounters",
                                fmt_day_short(local_day(n.unix)),
                                fmt_clock(n.unix),
                                theme::thousands(n.count)
                            ))
                            .size(kit::fs(11.0))
                            .color(MUTED()),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("✖").color(MUTED())).frame(false)).on_hover_text("Delete this note").clicked() {
                            remove = Some(i);
                        }
                    });
                });
            }
        });
        if add && !self.setup.note_draft.trim().is_empty() {
            let text = self.setup.note_draft.trim().to_string();
            if let Some(n) = self.notes_mut(run) {
                n.push(Note { unix: now_unix(), count, text });
            }
            self.setup.note_draft.clear();
            self.dirty = true;
        }
        if let Some(i) = remove {
            if let Some(n) = self.notes_mut(run) {
                if i < n.len() {
                    n.remove(i);
                }
            }
            self.dirty = true;
        }
    }

    // ───────────────────────────── sound settings ─────────────────────────────

    pub(super) fn sounds_card(&mut self, ui: &mut egui::Ui) {
        kit::section(ui, "Sounds");
        kit::card(ui, |ui| {
            self.dirty |= kit::toggle_row(
                ui,
                "Sound effects",
                "Little chimes for shinies, achievements, level-ups, milestones, goals and Caught! You can swap any of them for your own sound file.",
                &mut self.cfg.sounds.enabled,
            );
            if !self.cfg.sounds.enabled {
                return;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Volume").size(kit::fs(13.0)).color(TEXT()));
                ui.spacing_mut().slider_width = (ui.available_width() - kit::SLIDER_VALUE_W - 16.0).max(80.0);
                let text = format!("{:.0}%", self.cfg.sounds.volume * 100.0);
                self.dirty |= kit::slider_f32(ui, &mut self.cfg.sounds.volume, 0.0..=1.0, &text).changed();
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Play on").size(kit::fs(13.0)).color(TEXT()));
                let devices = crate::sound::devices(false);
                let cur = self.cfg.sounds.device.clone();
                let shown = if cur.is_empty() {
                    "Default (Windows)".to_string()
                } else if devices.contains(&cur) {
                    cur.clone()
                } else {
                    format!("{cur} (not connected)")
                };
                let mut pick = cur.clone();
                egui::ComboBox::from_id_salt("sfx-device")
                    .selected_text(shown)
                    .width((ui.available_width() - 40.0).clamp(160.0, 360.0))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut pick, String::new(), "Default (Windows)");
                        for d in &devices {
                            ui.selectable_value(&mut pick, d.clone(), d.as_str());
                        }
                    });
                if ui
                    .add(egui::Button::new("⟳").fill(kit::btn_fill()))
                    .on_hover_text("Look for headphones/speakers plugged in since")
                    .clicked()
                {
                    crate::sound::devices(true);
                }
                if pick != cur {
                    self.cfg.sounds.device = pick;
                    self.dirty = true;
                    crate::sound::play(Sfx::Milestone, self.cfg.sounds.volume, &self.cfg.sounds.device);
                }
            });
            ui.add_space(6.0);
            // A picked file arrives here (for an effect, or the alert song).
            if let Some((target, rx)) = &self.setup.song_picker {
                if let Ok(res) = rx.try_recv() {
                    let target = *target;
                    if let Some(p) = res {
                        let p = p.display().to_string();
                        match target {
                            Some(s) => {
                                self.cfg.sounds.custom.insert(s.key().to_string(), p);
                            }
                            None => self.cfg.sounds.alert_song_path = p,
                        }
                        self.dirty = true;
                    }
                    self.setup.song_picker = None;
                }
            }
            let picking = self.setup.song_picker.is_some();
            ui.label(
                RichText::new("Each sound can use your own file (MP3, WAV, WMA, M4A). Leave it on Default for the built-in one.")
                    .size(kit::fs(11.5))
                    .color(MUTED()),
            );
            ui.add_space(4.0);
            for s in Sfx::ALL {
                ui.horizontal(|ui| {
                    let mut on = *self.cfg.sounds.flag(s);
                    if kit::checkbox(ui, &mut on, s.label()).changed() {
                        *self.cfg.sounds.flag(s) = on;
                        self.dirty = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("▶ Play").size(kit::fs(11.5))).fill(kit::btn_fill()))
                            .clicked()
                        {
                            self.play_now(s);
                        }
                        let custom = self.cfg.sounds.custom_for(s).map(|p| p.to_string());
                        if custom.is_some()
                            && ui
                                .add(egui::Button::new(RichText::new("Default").size(kit::fs(11.5))).fill(kit::btn_fill()))
                                .on_hover_text("Go back to the built-in sound")
                                .clicked()
                        {
                            self.cfg.sounds.custom.remove(s.key());
                            self.dirty = true;
                        }
                        let start = custom
                            .as_deref()
                            .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()))
                            .or_else(dirs::audio_dir)
                            .unwrap_or_default();
                        if ui
                            .add_enabled(
                                !picking,
                                egui::Button::new(RichText::new(if custom.is_some() { "Change…" } else { "Custom…" }).size(kit::fs(11.5)))
                                    .fill(kit::btn_fill()),
                            )
                            .on_hover_text("Use your own sound file for this")
                            .clicked()
                        {
                            self.setup.song_picker = Some((Some(s), crate::share::pick_audio(start, ui.ctx().clone())));
                        }
                        // Which file is in use (or Default), in the space left.
                        let (text, col) = match &custom {
                            None => ("Default".to_string(), MUTED()),
                            Some(p) => {
                                let name = std::path::Path::new(p)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                if crate::sound::usable_file(p) {
                                    (format!("♪ {name}"), TEXT())
                                } else {
                                    (format!("{name} (not found, using default)"), theme::HP_RED)
                                }
                            }
                        };
                        ui.add(egui::Label::new(RichText::new(text).size(kit::fs(12.0)).color(col)).truncate())
                            .on_hover_text(custom.clone().unwrap_or_else(|| "Built-in sound".into()));
                    });
                });
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            self.dirty |= kit::toggle_row(
                ui,
                "Alert song",
                "Plays a song of your choice instead of the Shiny spotted! sound when a shiny shows up after a long hunt. Uses the volume above and your Windows default speakers.",
                &mut self.cfg.sounds.alert_song,
            );
            if self.cfg.sounds.alert_song {
                ui.add_space(6.0);
                let path = self.cfg.sounds.alert_song_path.clone();
                let name = std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let missing = !path.is_empty() && !std::path::Path::new(&path).is_file();
                ui.horizontal(|ui| {
                    let (text, col) = if path.is_empty() {
                        ("No song chosen yet".to_string(), MUTED())
                    } else if missing {
                        (format!("{name} (file not found)"), theme::HP_RED)
                    } else {
                        (format!("♪ {name}"), TEXT())
                    };
                    ui.add(egui::Label::new(RichText::new(text).size(kit::fs(12.5)).color(col)).truncate())
                        .on_hover_text(path.as_str());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("■ Stop").size(kit::fs(11.5))).fill(kit::btn_fill()))
                            .clicked()
                        {
                            crate::sound::stop_song();
                        }
                        if ui
                            .add_enabled(
                                !path.is_empty() && !missing,
                                egui::Button::new(RichText::new("▶ Test").size(kit::fs(11.5))).fill(kit::btn_fill()),
                            )
                            .clicked()
                        {
                            crate::sound::play_song(&path, self.cfg.sounds.volume);
                        }
                        let start = std::path::Path::new(&path)
                            .parent()
                            .map(|p| p.to_path_buf())
                            .or_else(dirs::audio_dir)
                            .unwrap_or_default();
                        if ui
                            .add_enabled(
                                !picking,
                                egui::Button::new(RichText::new("Choose song…").size(kit::fs(11.5))).fill(kit::btn_fill()),
                            )
                            .clicked()
                        {
                            self.setup.song_picker = Some((None, crate::share::pick_audio(start, ui.ctx().clone())));
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Only for shinies after").size(kit::fs(12.5)).color(TEXT()));
                    self.dirty |= ui
                        .add(
                            egui::DragValue::new(&mut self.cfg.sounds.alert_song_after)
                                .range(0..=1_000_000)
                                .speed(50.0),
                        )
                        .changed();
                    ui.label(RichText::new("encounters").size(kit::fs(12.5)).color(TEXT()));
                });
                ui.label(
                    RichText::new("Earlier shinies still get the normal Shiny spotted! sound. Dismissing the alert stops the song.")
                        .size(kit::fs(11.5))
                        .color(MUTED()),
                );
            }
        });
    }
}
