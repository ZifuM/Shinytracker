//! Shiny Bingo's page: today's card (S H I N Y, a free square in the middle)
//! on the left; the day, the rewards and past cards on the right. A new card
//! comes at the game's daily reset. Squares tick themselves from the game's
//! log and the app's own hunting time, missions and shinies; a full line
//! pays Card Collector XP and a Pokégotchi treat, the day's first line also
//! the daily reward (a booster pack or battle pass XP, your pick), and all
//! 25 a booster pack.

use super::notices::Notice;
use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::App;
use crate::apps::AppId;
use crate::bingo::{self, BingoFile, Event, Goal, Met, Won, FREE, LETTERS, SIZE, SQUARES};
use crate::stats::{local_day, now_unix};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};

pub(super) const VIOLET: Color32 = Color32::from_rgb(139, 92, 246);
pub(super) const ORANGE: Color32 = Color32::from_rgb(240, 138, 60);
pub(super) const GOLD: Color32 = Color32::from_rgb(236, 186, 80);

#[derive(Default)]
pub(super) struct BingoState {
    pub file: Option<BingoFile>,
    pub load_error: Option<String>,
    dirty: Option<Instant>,
    saved_once: bool,
    pub save_error: Option<String>,
    /// The game's logs were found and are being read.
    pub reads_logs: bool,
    last_log_read: Option<Instant>,
    last_app_check: Option<Instant>,
    /// Past cards are showing.
    pub past: bool,
    /// When each square was last ticked in this run (for a little pop).
    pub popped: Vec<(usize, Instant)>,
    /// The daily reward popup: open, or put off for now ("Later").
    pub reward_open: bool,
    pub reward_later: bool,
}

impl BingoState {
    pub fn file(&mut self) -> &mut BingoFile {
        if self.file.is_none() {
            let (f, note) = bingo::load_from(&crate::config::data_dir());
            self.load_error = note;
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
        match bingo::save_to(&crate::config::data_dir(), f, !self.saved_once) {
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

fn today() -> i64 {
    local_day(now_unix())
}

/// Today's card: its day on the game server's calendar.
pub(super) fn this_card() -> i64 {
    bingo::card_day(now_unix() as i64)
}

/// Seconds until the next card (the game's daily reset).
pub(super) fn secs_to_next_card() -> i64 {
    let now = now_unix() as i64;
    (crate::gamemap::next_reset(now) - now).max(0)
}

pub(super) fn countdown(secs: i64) -> String {
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    if h >= 24 {
        format!("{}d {}h", h / 24, h % 24)
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

/// The colour of a square's badge.
pub(super) fn goal_col(g: Goal) -> Color32 {
    match g {
        Goal::Meet(_) => kit::C_SLATE,
        Goal::Type(..) => super::pokedex::type_color(g.type_name().unwrap_or("Normal")),
        Goal::RareGrass(_) | Goal::Evolve => kit::C_GREEN,
        Goal::Double => kit::C_SLATE,
        Goal::LevelAtLeast(_) | Goal::LevelUnder(_) | Goal::Letter(_) => kit::C_INDIGO,
        Goal::Session(_) | Goal::Today(_) | Goal::Different(_) => kit::C_BLUE,
        Goal::HuntMinutes(_) | Goal::Late | Goal::Early => kit::C_CYAN,
        Goal::Shiny => GOLD,
        Goal::Dailies(_) => kit::C_PINK,
    }
}

/// A small Poké Ball stamp centred on `c`.
pub(super) fn stamp(p: &egui::Painter, c: Pos2, r: f32) {
    let dark = Color32::from_rgb(27, 27, 39);
    p.circle_filled(c, r + 1.5, Color32::from_white_alpha(150));
    p.circle_filled(c, r, Color32::from_rgb(242, 242, 246));
    // the red top half
    let n = 16;
    let mut top = vec![c + vec2(-r, 0.0)];
    for i in 0..=n {
        let a = std::f32::consts::PI + std::f32::consts::PI * i as f32 / n as f32;
        top.push(c + vec2(a.cos(), a.sin()) * r);
    }
    p.add(egui::Shape::convex_polygon(top, Color32::from_rgb(226, 66, 90), Stroke::NONE));
    p.line_segment([c + vec2(-r, 0.0), c + vec2(r, 0.0)], Stroke::new(r * 0.22, dark));
    p.circle_filled(c, r * 0.34, dark);
    p.circle_filled(c, r * 0.2, Color32::WHITE);
}

impl App {
    fn bingo_on(&self) -> bool {
        crate::apps::live(AppId::Bingo)
    }

    /// Today's card, and its saved progress made current.
    pub(super) fn bingo_card(&mut self) -> [Option<Goal>; SQUARES] {
        let d = this_card();
        if self.setup.bingo.file().roll(d) {
            self.setup.bingo.touch();
        }
        bingo::card(d)
    }

    /// Every frame: the game's log every few seconds, hunting time and
    /// missions now and then, saving.
    pub(super) fn bingo_tick(&mut self, _ctx: &egui::Context) {
        if !self.bingo_on() {
            return;
        }
        let _ = self.bingo_card();
        // the log needs the Pokédex data (names and types): wait for it
        if self.wiki.is_some() && self.setup.bingo.last_log_read.map_or(true, |t| t.elapsed() > Duration::from_secs(5)) {
            self.setup.bingo.last_log_read = Some(Instant::now());
            self.bingo_read_logs();
        }
        if self.setup.bingo.last_app_check.map_or(true, |t| t.elapsed() > Duration::from_secs(10)) {
            self.setup.bingo.last_app_check = Some(Instant::now());
            let mins = (self.today_hunt_secs() / 60) as u32;
            self.bingo_event(Event::HuntMinutes(mins));
            let d = self.bingo_dailies_today();
            self.bingo_event(Event::Dailies(d));
        }
        self.setup.bingo.save_tick(false);
    }

    /// Battle pass dailies done today.
    fn bingo_dailies_today(&self) -> u32 {
        let day = today();
        let season = crate::pass::current(day);
        if !season.active(day) {
            return 0;
        }
        let pre = format!("pass:{}:d{day}:", season.num);
        let goal = crate::goals::day_key(day);
        self.cfg.goals_done.iter().filter(|g| g.key == goal || g.key.starts_with(&pre)).count() as u32
    }

    fn bingo_read_logs(&mut self) {
        let Some(dir) = self.game_log_dir() else {
            self.setup.bingo.reads_logs = false;
            return;
        };
        let mut cursors = std::mem::take(&mut self.setup.bingo.file().logs);
        let read = crate::gamelog::read_new_events(&dir, &mut cursors);
        self.setup.bingo.file().logs = cursors;
        let Some((found, moved)) = read else {
            self.setup.bingo.reads_logs = false;
            return;
        };
        self.setup.bingo.reads_logs = true;
        if moved {
            self.setup.bingo.touch();
        }
        for (session, ev) in found {
            let ev = match ev {
                crate::gamelog::LogEvent::Evolve => Event::Evolve,
                crate::gamelog::LogEvent::Battle { mons, hour } => {
                    let mons = mons
                        .iter()
                        .map(|w| {
                            let mon = self.wiki.as_ref().and_then(|wk| wk.by_dex(w.species));
                            Met {
                                dex: w.species,
                                level: w.level,
                                rare_grass: w.method.contains("rare grass"),
                                name: mon.map(|m| m.name.clone()).or_else(|| self.species.iter().find(|s| s.dex == w.species).map(|s| s.name.to_string())).unwrap_or_default(),
                                types: mon.map(|m| m.types.clone()).unwrap_or_default(),
                            }
                        })
                        .collect();
                    Event::Battle { mons, hour, session }
                }
            };
            self.bingo_event(ev);
        }
    }

    /// A shiny caught (from marking it caught in the Shiny Hunter).
    pub(super) fn bingo_shiny(&mut self) {
        if self.bingo_on() {
            self.bingo_event(Event::Shiny);
        }
    }

    /// Feed something in, and pay out whatever it won.
    fn bingo_event(&mut self, ev: Event) {
        let _ = self.bingo_card();
        let won = self.setup.bingo.file().apply(&ev, now_unix());
        if !won.is_empty() {
            self.setup.bingo.touch();
            self.bingo_pay(&won);
        }
    }

    fn bingo_pay(&mut self, won: &Won) {
        let now = Instant::now();
        for s in &won.squares {
            self.setup.bingo.popped.push((*s, now));
        }
        self.setup.bingo.popped.retain(|(_, t)| t.elapsed() < Duration::from_secs(3));
        let card = bingo::card(this_card());
        if won.daily {
            self.setup.bingo.reward_open = true;
            self.setup.bingo.reward_later = false;
        }
        if won.lines.is_empty() && !won.blackout {
            // a square: a quiet notice
            if let Some(Some(g)) = won.squares.last().map(|i| card[*i]) {
                let n = self.setup.bingo.file().count(&card);
                self.notify(Notice {
                    id: "bingo-square",
                    icon: "🎯",
                    title: format!("Bingo square: {}", g.text()),
                    body: format!("{n} / 25 this week"),
                    progress: None,
                    action: None,
                    expires: Some(now + Duration::from_secs(6)),
                    accent: VIOLET,
                });
            }
            return;
        }
        // a line (or more): Card Collector XP and Pokégotchi treats; they wait
        // in those apps' saves if they aren't installed yet
        let n = won.lines.len() as u64;
        if n > 0 {
            self.setup.cards.file().add_xp(bingo::LINE_CARD_XP * n, "Shiny Bingo line");
            self.setup.cards.touch();
            self.setup.gotchi.file().treats += bingo::LINE_TREATS * n as u32;
            self.setup.gotchi.touch();
        }
        if won.blackout {
            self.setup.cards.file().packs += bingo::BLACKOUT_PACKS;
            self.setup.cards.touch();
        }
        let title = if won.blackout {
            "BLACKOUT! The whole card is done".to_string()
        } else if won.lines.len() == 1 {
            format!("Bingo! {} complete", bingo::line_name(won.lines[0]))
        } else {
            format!("Bingo ×{}! {} lines at once", won.lines.len(), won.lines.len())
        };
        let mut body = Vec::new();
        if won.daily {
            body.push("your daily reward is waiting in Shiny Bingo".to_string());
        }
        if n > 0 {
            body.push(format!("+{} Card Collector XP and {} Pokégotchi treat{}", bingo::LINE_CARD_XP * n, bingo::LINE_TREATS as u64 * n, if n == 1 { "" } else { "s" }));
        }
        if won.blackout {
            body.push("a booster pack".into());
        }
        self.notify(Notice {
            id: "bingo-line",
            icon: "🎉",
            title,
            body: body.join(", and "),
            progress: None,
            action: None,
            expires: Some(now + Duration::from_secs(10)),
            accent: GOLD,
        });
        self.play(crate::sound::Sfx::Goal);
    }

    // ───────────────────────────── drawing ─────────────────────────────

    /// One square in `r`. `small`: the Dashboard widget's tiny squares.
    pub(super) fn bingo_square(&mut self, ui: &egui::Ui, r: Rect, card: &[Option<Goal>; SQUARES], i: usize, in_line: bool, small: bool) -> egui::Response {
        let resp = ui.interact(r, ui.id().with(("bingo-sq", i, small)), Sense::hover());
        let p = ui.painter_at(r.expand(6.0));
        let f = self.setup.bingo.file().clone();
        let done = f.done(card, i);
        let round = if small { 5.0 } else { 12.0 };
        let pop = self.setup.bingo.popped.iter().find(|(s, _)| *s == i).map(|(_, t)| t.elapsed().as_secs_f32()).filter(|t| *t < 1.2);
        if pop.is_some() {
            ui.ctx().request_repaint();
        }
        let r = match pop {
            Some(t) => r.expand((1.0 - t / 1.2) * 4.0),
            None => r,
        };
        if in_line {
            p.add(egui::epaint::Shadow { offset: vec2(0.0, 0.0), blur: if small { 6.0 } else { 18.0 }, spread: 0.0, color: GOLD.gamma_multiply(0.35) }.as_shape(r, round));
        }
        let Some(g) = card[i] else {
            // the free square
            p.add(theme::gradient_rect(r, [round; 4], VIOLET, ORANGE));
            if in_line {
                p.rect_stroke(r, round, Stroke::new(if small { 1.5f32 } else { 2.5 }, GOLD));
            }
            if small {
                p.text(r.center(), Align2::CENTER_CENTER, "★", crate::fonts::sans_bold_font(r.height() * 0.5), Color32::WHITE);
            } else {
                p.text(r.center() - vec2(0.0, 7.0), Align2::CENTER_CENTER, "FREE", crate::fonts::sans_bold_font(20.0), Color32::WHITE);
                p.text(r.center() + vec2(0.0, 13.0), Align2::CENTER_CENTER, "★", crate::fonts::sans_bold_font(13.0), Color32::from_white_alpha(220));
            }
            return resp;
        };
        let fill = if done { theme::blend(kit::sunken(), VIOLET, 0.24) } else { kit::sunken() };
        p.rect_filled(r, round, fill);
        let edge = if in_line {
            Stroke::new(if small { 1.5f32 } else { 2.5 }, GOLD)
        } else if done {
            Stroke::new(1.5f32, GOLD.gamma_multiply(0.6))
        } else if resp.hovered() {
            Stroke::new(1.0f32, theme::blend(LINE(), Color32::WHITE, 0.2))
        } else {
            Stroke::new(1.0f32, LINE())
        };
        p.rect_stroke(r, round, edge);
        if small {
            if done {
                p.circle_filled(r.center(), r.width().min(r.height()) * 0.22, GOLD);
            }
            return resp.on_hover_text(g.text());
        }
        // badge: the Pokémon itself, or a coloured circle with a short label
        let bc = r.min + vec2(24.0, 24.0);
        match g {
            Goal::Meet(d) => {
                // its sprite once it's on this PC; until then a short name badge
                let sp = self.species.iter().find(|s| s.dex == d).cloned();
                let shown = match sp {
                    Some(sp) if self.sprites.get(&sp, false).is_some() => {
                        p.circle_filled(bc, 17.0, theme::blend(kit::sunken(), Color32::WHITE, 0.06));
                        self.draw_sprite_centered(&p, &sp, false, bc, vec2(40.0, 40.0))
                    }
                    _ => false,
                };
                if !shown {
                    p.circle_filled(bc, 15.0, theme::blend(kit::C_SLATE, VIOLET, 0.25));
                    let short: String = bingo::meet_name(d).chars().take(3).collect::<String>().to_uppercase();
                    p.text(bc, Align2::CENTER_CENTER, short, crate::fonts::sans_bold_font(10.0), Color32::WHITE);
                }
            }
            _ => {
                let col = goal_col(g);
                p.circle_filled(bc, 15.0, col);
                let label = g.badge();
                let size = if label.chars().count() > 3 { 9.0 } else { 10.5 };
                p.text(bc, Align2::CENTER_CENTER, label, crate::fonts::sans_bold_font(size), Color32::WHITE);
            }
        }
        if done {
            stamp(&p, pos2(r.max.x - 18.0, r.min.y + 18.0), 10.0);
        }
        // what to do
        let mut text = g.text();
        if g.hard() {
            text.push_str(" ★");
        }
        let tg = p.layout(text, crate::fonts::sans_bold_font(12.5), if done { Color32::WHITE } else { TEXT() }, r.width() - 18.0);
        p.galley(pos2(r.min.x + 9.0, r.min.y + 46.0), tg, TEXT());
        // progress
        let need = g.need();
        let have = if done { need } else { f.have.get(i).copied().unwrap_or(0) };
        let foot = r.max.y - 10.0;
        if need > 1 {
            let bar = Rect::from_min_size(pos2(r.min.x + 9.0, foot - 20.0), vec2(r.width() - 18.0, 5.0));
            p.rect_filled(bar, 3.0, Color32::from_rgb(38, 38, 54));
            let fr = (have as f32 / need as f32).clamp(0.0, 1.0);
            if fr > 0.0 {
                let fill = Rect::from_min_size(bar.min, vec2(bar.width() * fr, bar.height()));
                p.add(theme::gradient_rect(fill, [3.0; 4], VIOLET, ORANGE));
            }
            let txt = format!("{} / {}", thousands(have), thousands(need));
            p.text(pos2(r.min.x + 9.0, foot), Align2::LEFT_BOTTOM, txt, crate::fonts::sans_font(11.0), MUTED());
        } else {
            p.text(pos2(r.min.x + 9.0, foot), Align2::LEFT_BOTTOM, if done { "Done" } else { "Not yet" }, crate::fonts::sans_font(11.0), MUTED());
        }
        let when = f.ticked.get(i).copied().filter(|t| *t > 0);
        let tip = match when {
            Some(t) => format!("{}\n\n{}\nTicked {}", g.text(), g.how(), ago(now_unix().saturating_sub(t))),
            None => format!("{}\n\n{}", g.text(), g.how()),
        };
        resp.on_hover_text(tip)
    }

    /// The card: S H I N Y over the 5×5 grid. Small (the widget): filling its
    /// box. Otherwise squares `fit` = (width, height), centred.
    pub(super) fn bingo_grid(&mut self, ui: &mut egui::Ui, small: bool, fit: Option<(f32, f32)>) {
        let card = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let mut in_line = [false; SQUARES];
        for l in f.lines_done(&card) {
            for s in bingo::lines()[l] {
                in_line[s] = true;
            }
        }
        let w = ui.available_width();
        let gap = if small { 4.0 } else { 8.0 };
        let fill_side = ((w - gap * (SIZE as f32 - 1.0)) / SIZE as f32).floor();
        let (side, h) = match fit {
            Some((sw, sh)) if !small => (sw.min(fill_side), sh),
            _ if small => {
                let room = kit::take_card_fill().map(|h| ((h - 64.0 - gap * 4.0) / 5.0).floor()).unwrap_or(fill_side);
                (fill_side, fill_side.min(room).max(14.0))
            }
            _ => (fill_side, (fill_side * 0.7).clamp(96.0, 118.0)),
        };
        // centred in what's there
        let board_w = side * SIZE as f32 + gap * (SIZE as f32 - 1.0);
        let x0 = ((w - board_w) / 2.0).max(0.0).floor();
        if !small {
            let (lr, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
            for (c, l) in LETTERS.iter().enumerate() {
                let x = lr.min.x + x0 + c as f32 * (side + gap) + side / 2.0;
                let col = theme::blend(VIOLET, ORANGE, c as f32 / 4.0);
                ui.painter().text(pos2(x, lr.center().y), Align2::CENTER_CENTER, *l, crate::fonts::sans_bold_font(26.0), col);
            }
        }
        let (gr, _) = ui.allocate_exact_size(vec2(w, h * SIZE as f32 + gap * (SIZE as f32 - 1.0)), Sense::hover());
        for i in 0..SQUARES {
            let (row, col) = (i / SIZE, i % SIZE);
            let r = Rect::from_min_size(gr.min + vec2(x0 + col as f32 * (side + gap), row as f32 * (h + gap)), vec2(side, h));
            self.bingo_square(ui, r, &card, i, in_line[i], small);
        }
    }

    // ───────────────────────────── the page ─────────────────────────────

    pub(super) fn page_bingo(&mut self, ui: &mut egui::Ui) {
        let card = self.bingo_card();
        kit::page_header(ui, "SHINY BINGO", "Hunt as usual: squares tick themselves from the game's log. Lines win rewards.");
        if let Some(e) = self.setup.bingo.load_error.clone() {
            kit::pill(ui, &e, kit::PillKind::Warn);
            ui.add_space(8.0);
        }
        if !self.setup.bingo.reads_logs && self.setup.bingo.last_log_read.is_some() {
            kit::pill(ui, "Can't find the game's logs: check the game folder in Advanced", kit::PillKind::Warn);
            ui.add_space(8.0);
        }
        // the board fits the window's height (no scrolling to see it all),
        // squares a little wider than tall
        let full = ui.available_width();
        let side = 300.0f32.min(full * 0.3);
        let gap = 16.0;
        let screen_h = ui.ctx().screen_rect().height();
        let sq_h = ((screen_h - 330.0 - 8.0 * 4.0) / 5.0).clamp(92.0, 116.0).floor();
        let room = full - side - gap - 34.0;
        let sq_w = ((room - 8.0 * 4.0) / 5.0).min(sq_h * 1.45).floor();
        let left = (sq_w * 5.0 + 8.0 * 4.0 + 34.0).min(full - side - gap);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.allocate_ui_with_layout(vec2(left, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(left);
                kit::card(ui, |ui| self.bingo_grid(ui, false, Some((sq_w, sq_h))));
            });
            ui.allocate_ui_with_layout(vec2(side, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(side);
                self.bingo_side(ui, &card);
            });
        });
        if self.setup.bingo.past {
            ui.add_space(14.0);
            self.bingo_history(ui);
        }
        self.bingo_reward_popup(ui.ctx());
    }

    /// Take a waiting daily reward: a booster pack (it waits in the Card
    /// Collector if that isn't installed) or battle pass XP.
    fn bingo_take(&mut self, pack: bool) {
        if !self.setup.bingo.file().take_reward(pack) {
            return;
        }
        self.setup.bingo.touch();
        self.setup.bingo.save_tick(true);
        if pack {
            self.setup.cards.file().packs += bingo::DAILY_PACKS;
            self.setup.cards.pack_at = Some(Instant::now());
            self.setup.cards.touch();
            self.setup.cards.save_tick(true);
        } else {
            let now = now_unix();
            let season = crate::pass::current(local_day(now));
            self.cfg.goals_done.push(crate::config::GoalDone { key: season.bingo_key(now), xp: 0, unix: now });
            self.dirty = true;
        }
        self.play(crate::sound::Sfx::Goal);
        if self.setup.bingo.file().rewards_waiting == 0 {
            self.setup.bingo.reward_open = false;
        }
    }

    /// Can the battle pass take XP right now?
    fn bingo_pass_on(&self) -> bool {
        let d = local_day(now_unix());
        crate::pass::current(d).active(d)
    }

    /// The two choices, side by side (the popup and the side card).
    fn bingo_reward_buttons(&mut self, ui: &mut egui::Ui) {
        let w = ((ui.available_width() - 8.0) / 2.0).floor();
        let pass_on = self.bingo_pass_on();
        let mut take = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let pack = egui::Button::new(RichText::new(format!("🎁  Booster pack")).size(13.0).color(Color32::WHITE)).fill(theme::blend(kit::CARD(), VIOLET, 0.55)).min_size(vec2(w, 40.0));
            if ui.add(pack).on_hover_text("A booster pack to open in the Card Collector").clicked() {
                take = Some(true);
            }
            let xp = egui::Button::new(RichText::new(format!("⭐  {} pass XP", bingo::DAILY_PASS_XP)).size(13.0).color(Color32::WHITE)).fill(theme::blend(kit::CARD(), ORANGE, 0.55)).min_size(vec2(w, 40.0));
            let r = ui.add_enabled(pass_on, xp);
            let r = if pass_on { r.on_hover_text("Battle pass XP towards your next tier") } else { r.on_disabled_hover_text("The battle pass season has ended") };
            if r.clicked() {
                take = Some(false);
            }
        });
        if let Some(pack) = take {
            self.bingo_take(pack);
        }
    }

    /// "Bingo! Pick your daily reward", over the page when one is won (and
    /// whenever the page opens with one waiting, unless put off).
    fn bingo_reward_popup(&mut self, ctx: &egui::Context) {
        let waiting = self.setup.bingo.file().rewards_waiting;
        if waiting == 0 {
            self.setup.bingo.reward_open = false;
            return;
        }
        if !self.setup.bingo.reward_later {
            self.setup.bingo.reward_open = true;
        }
        if !self.setup.bingo.reward_open {
            return;
        }
        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("bingo-reward-dim")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
            ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(150));
            ui.allocate_rect(screen, Sense::click());
        });
        let mut later = false;
        egui::Window::new("bingo-reward")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .fixed_size(vec2(400.0, 0.0))
            .frame(egui::Frame::none().fill(kit::CARD()).stroke(Stroke::new(1.5f32, GOLD)).rounding(16.0).inner_margin(egui::Margin::same(22.0)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("BINGO!").font(crate::fonts::sans_bold_font(30.0)).color(GOLD));
                    ui.add_space(2.0);
                    ui.label(RichText::new("Your first line today wins the daily reward. Pick one:").size(13.5).color(TEXT()));
                    if waiting > 1 {
                        ui.label(RichText::new(format!("{waiting} daily rewards are waiting")).size(12.0).color(MUTED()));
                    }
                });
                ui.add_space(14.0);
                self.bingo_reward_buttons(ui);
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Later").size(12.5).color(MUTED())).frame(false)).on_hover_text("It waits for you in the side column").clicked() {
                        later = true;
                    }
                });
            });
        if later {
            self.setup.bingo.reward_later = true;
            self.setup.bingo.reward_open = false;
        }
    }

    /// The right column: the week, the rewards, past cards.
    fn bingo_side(&mut self, ui: &mut egui::Ui, card: &[Option<Goal>; SQUARES]) {
        let f = self.setup.bingo.file().clone();
        let day = this_card();
        let n = f.count(card);
        let lines = f.lines_done(card).len();
        if f.rewards_waiting > 0 {
            egui::Frame::none().fill(theme::blend(kit::CARD(), GOLD, 0.12)).stroke(Stroke::new(1.5f32, GOLD)).rounding(14.0).inner_margin(egui::Margin::same(14.0)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("🎉  Daily reward ready").font(crate::fonts::sans_bold_font(14.0)).color(GOLD));
                ui.add_space(8.0);
                self.bingo_reward_buttons(ui);
            });
            ui.add_space(12.0);
        }
        kit::card(ui, |ui| {
            ui.label(RichText::new(format!("TODAY'S CARD · {}", crate::stats::fmt_day_short(day)).to_uppercase()).font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(n.to_string()).font(crate::fonts::sans_bold_font(30.0)).color(TEXT()));
                ui.label(RichText::new("/ 25 squares").size(14.0).color(MUTED()));
            });
            ui.add_space(6.0);
            let (label, kind) = if f.blackout {
                ("Blackout! Every square done".to_string(), kit::PillKind::Ok)
            } else if lines == 0 {
                ("No lines yet · 12 to go".to_string(), kit::PillKind::Info)
            } else {
                (format!("{lines} line{} done · {} to go", if lines == 1 { "" } else { "s" }, 12 - lines), kit::PillKind::Ok)
            };
            kit::pill(ui, &label, kind);
            ui.add_space(10.0);
            // the 12 lines as segments
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(vec2(w, 8.0), Sense::hover());
            let seg = (w - 11.0 * 4.0) / 12.0;
            for k in 0..12 {
                let sr = Rect::from_min_size(pos2(r.min.x + k as f32 * (seg + 4.0), r.min.y), vec2(seg, 8.0));
                ui.painter().rect_filled(sr, 4.0, if k < lines { GOLD } else { Color32::from_rgb(38, 38, 54) });
            }
            ui.add_space(10.0);
            ui.label(RichText::new(format!("New card in {} (the daily reset)", countdown(secs_to_next_card()))).size(12.5).color(MUTED()));
        });
        ui.add_space(12.0);
        kit::card(ui, |ui| {
            ui.label(RichText::new("REWARDS").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
            ui.add_space(4.0);
            let row = |ui: &mut egui::Ui, k: &str, v: String, col: Color32| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(k).size(13.0).color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(v).size(12.5).color(col));
                    });
                });
                ui.add_space(4.0);
            };
            let daily_done = f.daily_won == Some(day);
            row(ui, "First bingo today", if daily_done { "✔ Won".to_string() } else { format!("Pack or {} pass XP", bingo::DAILY_PASS_XP) }, if daily_done { GOLD } else { MUTED() });
            row(ui, "Each line", format!("{} card XP · {} treat", bingo::LINE_CARD_XP, bingo::LINE_TREATS), MUTED());
            row(ui, "Blackout", "Booster pack".into(), MUTED());
            let won = if lines == 0 && !f.blackout {
                "Nothing yet".to_string()
            } else {
                let mut s = format!("{} XP · {} treat{}", bingo::LINE_CARD_XP * lines as u64, lines, if lines == 1 { "" } else { "s" });
                if f.blackout {
                    s.push_str(" · a pack");
                }
                s
            };
            row(ui, "Won today", won, if lines > 0 { GOLD } else { MUTED() });
            if !self.has_app(AppId::Cards) || !self.has_app(AppId::Gotchi) {
                ui.add_space(2.0);
                ui.label(RichText::new("Rewards for an app you don't have wait in it until you get it from the App Store.").size(11.5).color(MUTED()));
            }
        });
        ui.add_space(12.0);
        // the lines closest to done
        let close = f.closest(card);
        if !f.blackout && !close.is_empty() {
            kit::card(ui, |ui| {
                ui.label(RichText::new("CLOSEST TO A LINE").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
                ui.add_space(6.0);
                for (l, left) in close.iter().take(2) {
                    let what: Vec<String> = left.iter().filter_map(|s| card[*s].map(|g| g.text())).collect();
                    ui.label(RichText::new(bingo::line_name(*l)).font(crate::fonts::sans_bold_font(12.5)).color(TEXT()));
                    ui.label(RichText::new(format!("needs {}", what.join(", "))).size(12.0).color(MUTED()));
                    ui.add_space(6.0);
                }
            });
            ui.add_space(12.0);
        }
        let label = if self.setup.bingo.past { "Hide past cards" } else { "Past cards" };
        let b = egui::Button::new(RichText::new(label).size(13.0)).fill(kit::btn_fill()).min_size(vec2(ui.available_width(), 32.0));
        if ui.add(b).clicked() {
            self.setup.bingo.past = !self.setup.bingo.past;
        }
    }

    /// Past cards as little grids.
    fn bingo_history(&mut self, ui: &mut egui::Ui) {
        let f = self.setup.bingo.file().clone();
        kit::card(ui, |ui| {
            ui.label(RichText::new("PAST CARDS").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
            ui.add_space(8.0);
            if f.history.is_empty() {
                ui.label(RichText::new("Your cards show here after each daily reset.").size(12.5).color(MUTED()));
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                for past in &f.history {
                    let (r, resp) = ui.allocate_exact_size(vec2(112.0, 142.0), Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(r, 10.0, kit::sunken());
                    p.rect_stroke(r, 10.0, Stroke::new(1.0f32, LINE()));
                    let cell = 16.0;
                    let g0 = r.min + vec2(12.0, 12.0);
                    for i in 0..SQUARES {
                        let cr = Rect::from_min_size(g0 + vec2((i % SIZE) as f32 * (cell + 2.0), (i / SIZE) as f32 * (cell + 2.0)), vec2(cell, cell));
                        let on = i == FREE || past.done & (1 << i) != 0;
                        p.rect_filled(cr, 3.0, if on { VIOLET } else { Color32::from_rgb(42, 42, 59) });
                    }
                    p.text(pos2(r.min.x + 12.0, r.max.y - 30.0), Align2::LEFT_CENTER, crate::stats::fmt_day_short(past.day), crate::fonts::sans_bold_font(11.5), TEXT());
                    let squares = (past.done | 1 << FREE).count_ones();
                    let what = if squares as usize == SQUARES { "Blackout".to_string() } else { format!("{} line{} · {squares}/25", past.lines, if past.lines == 1 { "" } else { "s" }) };
                    p.text(pos2(r.min.x + 12.0, r.max.y - 14.0), Align2::LEFT_CENTER, what, crate::fonts::sans_font(11.0), MUTED());
                    resp.on_hover_text(crate::stats::fmt_day(past.day));
                }
            });
            ui.add_space(4.0);
            ui.label(RichText::new(format!("{} lines and {} blackout{} all time · daily rewards: {} pack{}, {} pass XP", f.lines_ever, f.blackouts, if f.blackouts == 1 { "" } else { "s" }, f.picked.0, if f.picked.0 == 1 { "" } else { "s" }, f.picked.1)).size(12.0).color(MUTED()));
        });
    }
}

pub(super) fn thousands(n: u32) -> String {
    if n >= 1000 {
        format!("{},{:03}", n / 1000, n % 1000)
    } else {
        n.to_string()
    }
}

fn ago(s: u64) -> String {
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", s / 60),
        3600..=86_399 => format!("{}h ago", s / 3600),
        _ => format!("{}d ago", s / 86_400),
    }
}
