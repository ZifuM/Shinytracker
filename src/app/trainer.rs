//! Trainer progress in the app: lifetime stats, unlocking achievements,
//! level-ups, the pop-up toasts on the overlay and the Profile page.

use super::ui_kit::{self as kit, MUTED, PillKind, CARD, CARD_HOVER, LINE, TEXT};
use super::App;
use crate::config::Unlocked;
use crate::progress::{self, Achievement, Category, Lifetime, Tier, ACHIEVEMENTS};
use crate::stats::{fmt_date, fmt_duration, now_unix, odds_multiple};
use crate::theme::{self, GOLD, OUTLINE};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const TOAST_SECS: f32 = 4.8;
/// How long a pop-up takes to open and to close.
const TOAST_IN: f32 = 0.32;
const TOAST_OUT: f32 = 0.26;

/// Size (0..1 of full) and content opacity of a pop-up `t` seconds in:
/// it springs open from a thin strip, settles, then folds away at the end.
fn toast_anim(t: f32) -> (f32, f32) {
    let ease_out_back = |x: f32| {
        let c1 = 1.4;
        let c3 = c1 + 1.0;
        1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2)
    };
    let ease_in_cubic = |x: f32| x * x * x;
    if t < TOAST_IN {
        let x = (t / TOAST_IN).clamp(0.0, 1.0);
        let s = 0.15 + 0.85 * ease_out_back(x);
        (s.min(1.03), ((x - 0.35) / 0.65).clamp(0.0, 1.0))
    } else if t > TOAST_SECS - TOAST_OUT {
        let x = ((t - (TOAST_SECS - TOAST_OUT)) / TOAST_OUT).clamp(0.0, 1.0);
        (1.0 - 0.85 * ease_in_cubic(x), (1.0 - x * 2.0).clamp(0.0, 1.0))
    } else {
        (1.0, 1.0)
    }
}

/// The pop-up card at `scale`: full width always, opening and closing in
/// height around its middle (like a blind).
fn toast_rect(area: Rect, scale: f32) -> Rect {
    let full = Rect::from_min_size(area.min, egui::vec2(area.width(), 64.0));
    let h = (full.height() * scale).max(4.0);
    Rect::from_center_size(full.center(), egui::vec2(full.width(), h))
}

pub(super) enum ToastKind {
    Achievement { name: String, desc: String, tier: Tier },
    LevelUp { level: u32 },
    Welcome { count: usize, level: u32 },
    Goal { title: String, xp: u64, weekly: bool },
    /// 1,000 encounters, 1× / 2× the odds… (its own setting, not the
    /// achievement pop-ups one).
    Milestone { text: String },
}

pub(super) struct Toast {
    kind: ToastKind,
    shown_at: Option<Instant>,
}

/// The tabs on the Profile page.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum ProfileSection {
    #[default]
    Overview,
    Goals,
    Achievements,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum AchFilter {
    All,
    Unlocked,
    Locked,
}

impl App {
    /// Everything across the current run, saved runs and finished hunts.
    pub(super) fn lifetime(&self) -> Lifetime {
        let mut l = Lifetime::default();
        let odds = self.cfg.odds_one_in();
        let mut days: BTreeMap<i64, u64> = BTreeMap::new();
        let mut targets: BTreeSet<String> = BTreeSet::new();
        let mut add_stats = |st: &crate::stats::RunStats, l: &mut Lifetime| {
            l.hunt_secs += st.hunt_secs;
            for d in &st.days {
                *days.entry(d.day).or_default() += d.encounters;
            }
            for s in &st.sessions {
                l.best_session = l.best_session.max(s.encounters);
                if s.secs >= 1800 {
                    l.best_rate = l.best_rate.max((s.encounters as f64 * 3600.0 / s.secs as f64) as u32);
                }
            }
            l.best_run_odds_x = l.best_run_odds_x.max(st.best_odds_x);
        };
        // current run
        l.encounters += self.cfg.count;
        l.battles += self.cfg.battles;
        add_stats(&self.cfg.run_stats, &mut l);
        if let Some(o) = odds {
            l.best_run_odds_x = l.best_run_odds_x.max(odds_multiple(self.cfg.count, o));
        }
        if self.cfg.count > 0 {
            if let Some(t) = &self.cfg.target {
                targets.insert(t.clone());
            }
        }
        // saved runs
        for r in &self.cfg.saved_runs {
            if r.id == self.cfg.run_id {
                continue;
            }
            l.encounters += r.count;
            l.battles += r.battles;
            add_stats(&r.stats, &mut l);
            if let Some(o) = odds {
                l.best_run_odds_x = l.best_run_odds_x.max(odds_multiple(r.count, o));
            }
            if let Some(t) = &r.target {
                targets.insert(t.clone());
            }
        }
        // finished hunts
        for h in &self.cfg.history {
            l.shinies += 1;
            if h.phase {
                // found during another hunt: its encounters are that hunt's
                continue;
            }
            l.encounters += h.encounters;
            l.battles += h.battles;
            if let Some(st) = &h.stats {
                add_stats(st, &mut l);
            }
            targets.insert(h.slug.clone().unwrap_or_else(|| h.target.to_lowercase()));
            if let Some(o) = h.odds_one_in.filter(|o| *o > 0.0) {
                let m = h.encounters as f64 / o;
                l.best_run_odds_x = l.best_run_odds_x.max(odds_multiple(h.encounters, o));
                l.luckiest_catch = Some(l.luckiest_catch.map_or(m, |x: f64| x.min(m)));
                l.longest_catch = Some(l.longest_catch.map_or(m, |x: f64| x.max(m)));
            }
        }
        l.best_day = days.values().copied().max().unwrap_or(0);
        let active: BTreeSet<i64> = days.iter().filter(|(_, n)| **n > 0).map(|(d, _)| *d).collect();
        l.longest_streak = progress::longest_streak(&active);
        l.targets = targets.len() as u32;
        l.events = self.cfg.events.iter().cloned().collect();
        let (dex, regions) = self.dex_progress();
        l.dex_caught = dex;
        l.regions_done = regions;
        l.goal_xp = self.goal_xp();
        l
    }

    /// Lifetime stats including the current trainer level.
    pub(super) fn lifetime_full(&self) -> Lifetime {
        let mut l = self.lifetime();
        l.level = progress::level_for(self.total_xp(&l).total());
        l
    }

    fn unlocked_ids(&self) -> Vec<&str> {
        self.cfg.achievements.iter().map(|u| u.id.as_str()).collect()
    }

    pub(super) fn total_xp(&self, l: &Lifetime) -> progress::Xp {
        progress::xp(l, &self.unlocked_ids())
    }

    /// Unlock anything newly earned and queue the pop-ups.
    pub(super) fn check_progress(&mut self) {
        self.progress_checked = Instant::now();
        if !self.cfg.trainer_name.trim().is_empty() && !self.cfg.events.iter().any(|e| e == "named") {
            self.cfg.events.push("named".into());
        }
        let goals_new = self.check_goals();
        let now = now_unix();
        let mut newly: Vec<&'static Achievement> = Vec::new();
        // Level goals depend on XP, which achievements add to: repeat until settled.
        for _ in 0..4 {
            let l = self.lifetime_full();
            let mut any = false;
            for a in ACHIEVEMENTS {
                if !self.cfg.achievements.iter().any(|u| u.id == a.id) && progress::earned(a, &l) {
                    self.cfg.achievements.push(Unlocked { id: a.id.to_string(), unix: now });
                    newly.push(a);
                    any = true;
                }
            }
            if !any {
                break;
            }
        }
        let l = self.lifetime();
        let level = progress::level_for(self.total_xp(&l).total());
        if self.cfg.level_seen == 0 {
            // First time: credit past hunting with one welcome pop-up.
            self.cfg.level_seen = level;
            if !newly.is_empty() || level > 1 {
                self.push_toast(ToastKind::Welcome { count: newly.len(), level });
            }
            self.dirty = true;
            return;
        }
        for (title, xp) in &goals_new {
            self.push_toast(ToastKind::Goal { title: title.clone(), xp: *xp, weekly: *xp != crate::goals::DAILY_XP });
        }
        let levelled = level > self.cfg.level_seen;
        // one chime per check, the most exciting one
        if levelled {
            self.play(crate::sound::Sfx::LevelUp);
        } else if !newly.is_empty() {
            self.play(crate::sound::Sfx::Achievement);
        } else if !goals_new.is_empty() {
            self.play(crate::sound::Sfx::Goal);
        }
        if newly.len() > 4 {
            self.push_toast(ToastKind::Welcome { count: newly.len(), level });
        } else {
            for a in &newly {
                self.push_toast(ToastKind::Achievement {
                    name: a.name.to_string(),
                    desc: a.desc.to_string(),
                    tier: a.tier,
                });
            }
        }
        if level > self.cfg.level_seen {
            self.cfg.level_seen = level;
            self.push_toast(ToastKind::LevelUp { level });
        }
        if !newly.is_empty() {
            self.dirty = true;
        }
    }

    /// Check now and then (cheap enough every few seconds).
    pub(super) fn maybe_check_progress(&mut self) {
        if self.progress_checked.elapsed() > Duration::from_secs(3) {
            self.check_progress();
        }
    }

    /// Record a one-off event (for the fun achievements).
    pub(super) fn add_event(&mut self, e: &str) {
        if !self.cfg.events.iter().any(|x| x == e) {
            self.cfg.events.push(e.to_string());
            self.dirty = true;
            self.check_progress();
        }
    }

    /// Time-of-day events when an encounter is counted.
    pub(super) fn note_time_events(&mut self) {
        let secs = (now_unix() as i64 + crate::stats::utc_offset_secs()).rem_euclid(86_400);
        let hour = secs / 3600;
        if hour < 5 {
            self.add_event("night_owl");
        } else if hour < 7 {
            self.add_event("early_bird");
        }
    }

    /// A milestone pop-up, shown like the others, just under the overlay.
    pub(super) fn push_milestone(&mut self, text: String) {
        if self.toasts.len() < 8 {
            self.toasts.push_back(Toast { kind: ToastKind::Milestone { text }, shown_at: None });
        }
    }

    /// Settings > Overlay: show one of each pop-up (`which` 0..6), even with
    /// achievement pop-ups switched off. `age` starts it part-way through
    /// (the screenshot rig).
    pub(super) fn preview_toast(&mut self, which: usize, age: Option<f32>) {
        let level = self.current_level().max(2);
        let kind = match which {
            0 => ToastKind::Achievement { name: "Show Off".into(), desc: "This is what a pop-up looks like".into(), tier: Tier::Gold },
            1 => ToastKind::LevelUp { level },
            2 => ToastKind::Goal { title: "Battle 200 Pokémon today".into(), xp: 150, weekly: false },
            3 => ToastKind::Goal { title: "Hunt for 5 hours this week".into(), xp: 600, weekly: true },
            4 => ToastKind::Milestone { text: "1,000 ENCOUNTERS!".into() },
            _ => ToastKind::Welcome { count: 12, level },
        };
        let shown_at = age.map(|a| Instant::now() - Duration::from_secs_f32(a));
        if self.toasts.len() < 8 {
            self.toasts.push_back(Toast { kind, shown_at });
        }
    }

    fn push_toast(&mut self, kind: ToastKind) {
        if !self.cfg.toasts {
            return;
        }
        if self.toasts.len() < 8 {
            self.toasts.push_back(Toast { kind, shown_at: None });
        }
    }

    pub(super) fn toasts_pending(&self) -> bool {
        // (Achievement pop-ups are only queued when they're switched on.)
        !self.toasts.is_empty()
    }

    pub(super) fn current_level(&self) -> u32 {
        let l = self.lifetime();
        progress::level_for(self.total_xp(&l).total())
    }

    /// Draw the front toast across the top of the overlay (call last).
    /// Where the front pop-up's card is right now (for cutting the overlay
    /// window to it), or None when no pop-up is showing.
    pub(super) fn toast_now(&self, area: Rect) -> Option<Rect> {
        let front = self.toasts.front()?;
        let t = front.shown_at.map_or(0.0, |at| at.elapsed().as_secs_f32());
        Some(toast_rect(area, toast_anim(t).0))
    }

    pub(super) fn draw_toast(&mut self, ctx: &egui::Context, p: &egui::Painter, area: Rect, theme: crate::config::OverlayTheme) {
        let Some(front) = self.toasts.front_mut() else { return };
        let at = *front.shown_at.get_or_insert_with(Instant::now);
        let t = at.elapsed().as_secs_f32();
        if t > TOAST_SECS {
            self.toasts.pop_front();
            ctx.request_repaint();
            return;
        }
        ctx.request_repaint();
        // The card grows out of its own middle and shrinks back into it
        // (the overlay window is cut to the card, so it can't slide around:
        // anything outside would be clipped off).
        let (scale, alpha) = toast_anim(t);
        let r = toast_rect(area, scale);
        if let Some(sk) = super::ov_toast::skin(theme) {
            use super::ov_toast::{ToastView, What};
            let (what, rgb, caption, title, line, level) = match &front.kind {
                ToastKind::Achievement { name, desc, tier } => {
                    (What::Achievement, tier.rgb(), format!("{} achievement", tier.label()), name.clone(), format!("{desc}  ·  +{} XP", tier.xp()), 0)
                }
                ToastKind::LevelUp { level } => {
                    let (_, _, light) = progress::badge_metal(*level);
                    (What::LevelUp, light, "Level up!".to_string(), format!("Level {level}"), progress::title(*level).to_string(), *level)
                }
                ToastKind::Goal { title, xp, weekly } => (
                    What::Goal,
                    [56, 214, 90],
                    if *weekly { "Weekly challenge".to_string() } else { "Daily goal".to_string() },
                    title.clone(),
                    format!("Complete  ·  +{xp} XP"),
                    0,
                ),
                ToastKind::Milestone { text } => (What::Milestone, [246, 200, 66], "Milestone!".to_string(), text.clone(), "Keep going!".to_string(), 0),
                ToastKind::Welcome { count, level } => (
                    What::Welcome,
                    [246, 200, 66],
                    "Welcome, trainer!".to_string(),
                    format!("Level {level}"),
                    if *count > 0 { format!("{count} achievements from your hunting so far") } else { progress::title(*level).to_string() },
                    *level,
                ),
            };
            let v = ToastView { what, rgb, caption, title, line, level, t, alpha };
            (sk.card)(p, r, v.tint());
            let clipped = p.with_clip_rect(r.shrink(1.0));
            (sk.toast)(&clipped, toast_rect(area, 1.0), &v);
            return;
        }
        let (rgb, caption, title, line) = match &front.kind {
            ToastKind::Achievement { name, desc, tier } => (
                tier.rgb(),
                format!("{} ACHIEVEMENT!", tier.label().to_uppercase()),
                name.to_uppercase(),
                format!("{desc}  ·  +{} XP", tier.xp()),
            ),
            ToastKind::LevelUp { level } => {
                let (_, _, light) = progress::badge_metal(*level);
                (light, "LEVEL UP!".to_string(), format!("LEVEL {level}"), progress::title(*level).to_string())
            }
            ToastKind::Goal { title, xp, weekly } => (
                [56, 214, 90],
                if *weekly { "WEEKLY CHALLENGE!".to_string() } else { "DAILY GOAL!".to_string() },
                title.to_uppercase(),
                format!("Complete  ·  +{xp} XP"),
            ),
            ToastKind::Milestone { text } => ([246, 200, 66], "MILESTONE!".to_string(), text.clone(), "Keep going!".to_string()),
            ToastKind::Welcome { count, level } => (
                [246, 200, 66],
                "WELCOME, TRAINER!".to_string(),
                format!("LEVEL {level}"),
                if *count > 0 {
                    format!("{count} achievements from your hunting so far")
                } else {
                    progress::title(*level).to_string()
                },
            ),
        };
        let tint = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        let a = |c: Color32| c.gamma_multiply(alpha);
        // The card itself stays solid (see-through would show black on some
        // PCs); only what's on it fades.
        p.rect_filled(r, 10.0 * scale, Color32::from_rgb(22, 19, 36));
        p.rect_stroke(r, 10.0 * scale, Stroke::new(2.0f32, tint.gamma_multiply(0.35 + 0.65 * alpha)));
        // Everything on the card is laid out on the full-size card and
        // revealed through the opening one.
        let clipped = p.with_clip_rect(r.shrink(1.0));
        let p = &clipped;
        let r = toast_rect(area, 1.0);
        // shimmer sweeping across once
        let sweep = ((t - 0.35) / 0.9).clamp(0.0, 1.0);
        if sweep > 0.0 && sweep < 1.0 {
            let x = r.min.x - 40.0 + sweep * (r.width() + 80.0);
            let clip = p.with_clip_rect(r.shrink(2.0));
            clip.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(x, r.max.y),
                    egui::pos2(x + 22.0, r.min.y),
                    egui::pos2(x + 40.0, r.min.y),
                    egui::pos2(x + 18.0, r.max.y),
                ],
                a(Color32::from_white_alpha(28)),
                Stroke::NONE,
            ));
        }
        // icon
        let ic = egui::pos2(r.min.x + 30.0, r.center().y - 2.0);
        match &front.kind {
            ToastKind::Achievement { .. } | ToastKind::Goal { .. } => {
                let pop = 1.0 + 0.25 * (1.0 - ((t - 0.2) / 0.35).clamp(0.0, 1.0));
                theme::medal(p, ic, 15.0 * pop, rgb, false);
            }
            ToastKind::Milestone { .. } => {
                // a big twinkling star
                let pop = 1.0 + 0.25 * (1.0 - ((t - 0.2) / 0.35).clamp(0.0, 1.0));
                theme::sparkle(p, ic, (11.0 + 2.0 * (t * 5.0).sin().abs()) * pop, a(GOLD));
            }
            ToastKind::LevelUp { level } | ToastKind::Welcome { level, .. } => {
                let pop = 1.0 + 0.25 * (1.0 - ((t - 0.2) / 0.35).clamp(0.0, 1.0));
                theme::level_badge(p, ic, 18.0 * pop, *level);
            }
        }
        for (i, (dx, dy)) in [(-18.0, -16.0), (17.0, -18.0), (19.0, 14.0)].into_iter().enumerate() {
            let tw = 0.5 + 0.5 * (t * 6.0 + i as f32 * 2.1).sin();
            theme::sparkle(p, ic + egui::vec2(dx, dy), 2.5 + 2.5 * tw, a(GOLD.gamma_multiply(0.4 + 0.6 * tw)));
        }
        // text
        let tx = r.min.x + 58.0;
        theme::outlined_text(p, egui::pos2(tx, r.min.y + 13.0), Align2::LEFT_CENTER, &caption, 6.5, a(tint));
        let size = if title.chars().count() > 18 { 7.5 } else { 9.0 };
        theme::outlined_text(p, egui::pos2(tx, r.min.y + 30.0), Align2::LEFT_CENTER, &title, size, a(Color32::WHITE));
        let clip = p.with_clip_rect(Rect::from_min_max(egui::pos2(tx, r.min.y), egui::pos2(r.max.x - 6.0, r.max.y)));
        clip.text(
            egui::pos2(tx, r.min.y + 48.0),
            Align2::LEFT_CENTER,
            line,
            kit::font(10.5),
            a(Color32::from_rgb(214, 210, 232)),
        );
    }

    // ───────────────────────────── profile page ─────────────────────────────

    pub(super) fn page_profile(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "TRAINER PROFILE", "Level up by hunting: encounters, time, shinies and achievements all give XP.");
        let l = self.lifetime_full();
        let xp = self.total_xp(&l);
        let total = xp.total();
        let level = progress::level_for(total);
        let (into, need) = progress::level_progress(total);
        let (metal, _, _) = progress::badge_metal(level);

        // Profile card
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(110.0, 110.0), Sense::hover());
                let p = ui.painter_at(r.expand(4.0));
                theme::level_badge(&p, r.center(), 48.0, level);
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    if ui.add(
                        egui::TextEdit::singleline(&mut self.cfg.trainer_name)
                            .hint_text("Your trainer name")
                            .font(egui::FontId::new(13.0, crate::fonts::pixel()))
                            .char_limit(16)
                            .desired_width(240.0),
                    )
                    .on_hover_text("Your trainer name: click to change it")
                    .changed()
                    {
                        self.dirty = true;
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Level {level} · {}", progress::title(level)))
                            .size(kit::fs(15.0))
                            .family(kit::strong_family(kit::fs(15.0))).strong()
                            .color(TEXT()),
                    );
                    ui.label(RichText::new(format!("{metal} badge")).size(kit::fs(11.5)).color(MUTED()));
                    ui.add_space(6.0);
                    let frac = if level >= progress::MAX_LEVEL { 1.0 } else { into as f32 / need.max(1) as f32 };
                    let (br, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(380.0), 12.0), Sense::hover());
                    let pp = ui.painter();
                    pp.rect_filled(br, 6.0, CARD_HOVER());
                    let fill = Rect::from_min_size(br.min, egui::vec2((br.width() * frac).max(12.0), br.height()));
                    pp.add(theme::button_shape(fill, 6.0, false));
                    ui.label(
                        RichText::new(if level >= progress::MAX_LEVEL {
                            format!("{} XP · max level!", theme::thousands(total))
                        } else {
                            format!(
                                "{} / {} XP to level {}  ·  {} XP total",
                                theme::thousands(into),
                                theme::thousands(need),
                                level + 1,
                                theme::thousands(total)
                            )
                        })
                        .size(kit::fs(11.5))
                        .color(MUTED()),
                    );
                });
            });
        });

        ui.horizontal(|ui| {
            if super::hunt_pages::primary(ui, "  🖼  Make a trainer card image  ")
                .on_hover_text("A shareable picture of your level, stats, shinies and achievements")
                .clicked()
            {
                self.setup.profile_card = Some(ProfileCardView::default());
            }
        });
        ui.add_space(10.0);

        // Section tabs
        let unlocked = self.cfg.achievements.iter().filter(|u| progress::find(&u.id).is_some()).count();
        let (goals_done, goals_all) = self.goals_done_count();
        let tabs = [
            (ProfileSection::Overview, "Overview".to_string(), None),
            (ProfileSection::Goals, "Goals".to_string(), Some((goals_done, goals_all))),
            (ProfileSection::Achievements, "Achievements".to_string(), Some((unlocked, ACHIEVEMENTS.len()))),
        ];
        if let Some(s) = section_tabs(ui, self.setup.profile_section, &tabs) {
            self.setup.profile_section = s;
        }
        ui.add_space(12.0);
        let section = self.setup.profile_section;
        if section == ProfileSection::Goals {
            self.goals_section(ui);
            return;
        }
        if section == ProfileSection::Overview {
        // The stats live on their own page now: a pointer to them.
        let w = ((ui.available_width() - 12.0) / 2.0).floor();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            if kit::stat_tile(ui, w, "📊", kit::C_CYAN, "Lifetime stats", "Luck, pace, habits, records", Some(("Open Stats ›", kit::C_CYAN))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.setup.stats_tab = super::stats_life::StatsTab::Lifetime;
                self.go_to(super::Tab::Stats);
            }
            if kit::stat_tile(ui, w, "🎯", kit::C_PINK, "This hunt's stats", "Luck curve, sessions, trends", Some(("Open Stats ›", kit::C_PINK))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.setup.stats_tab = super::stats_life::StatsTab::Hunt;
                self.go_to(super::Tab::Stats);
            }
        });
        ui.add_space(12.0);
        ui.label(
            RichText::new(format!(
                "XP from encounters {} · time {} · shinies {} · achievements {} · goals {}",
                theme::thousands(xp.encounters),
                theme::thousands(xp.time),
                theme::thousands(xp.shinies),
                theme::thousands(xp.achievements),
                theme::thousands(xp.goals)
            ))
            .size(kit::fs(11.5))
            .color(MUTED()),
        );
        ui.add_space(10.0);

        // Options
        kit::card(ui, |ui| {
            self.dirty |= kit::toggle_row(
                ui,
                "Achievement pop-ups",
                "A little pop-up on the overlay when you unlock an achievement or level up.",
                &mut self.cfg.toasts,
            );
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            self.dirty |= kit::toggle_row(
                ui,
                "Show my level on the overlay",
                "A small level badge next to the overlay title.",
                &mut self.cfg.level_on_overlay,
            );
            ui.add_space(6.0);
            if ui
                .add(egui::Button::new(RichText::new("Preview a pop-up").size(kit::fs(12.0))).fill(kit::btn_fill()))
                .clicked()
            {
                let was = self.cfg.toasts;
                self.cfg.toasts = true;
                self.push_toast(ToastKind::Achievement {
                    name: "Show Off".into(),
                    desc: "This is what a pop-up looks like".into(),
                    tier: Tier::Gold,
                });
                self.cfg.toasts = was;
                self.cfg.mini = false;
                self.play(crate::sound::Sfx::Achievement);
            }
        });

        return;
        }

        // Achievements: overall progress, category tabs, tiles
        let total_n = ACHIEVEMENTS.len();
        let done_n = unlocked;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{done_n} of {total_n} unlocked"))
                    .size(kit::fs(13.0))
                    .family(kit::strong_family(kit::fs(13.0))).strong()
                    .color(TEXT()),
            );
            let (br, _) = ui.allocate_exact_size(egui::vec2(ui.available_width() - 4.0, 8.0), Sense::hover());
            let pp = ui.painter();
            pp.rect_filled(br, 4.0, CARD_HOVER());
            let frac = done_n as f32 / total_n.max(1) as f32;
            if frac > 0.0 {
                pp.add(theme::button_shape(
                    Rect::from_min_size(br.min, egui::vec2((br.width() * frac).max(8.0), br.height())),
                    4.0,
                    false,
                ));
            }
        });
        ui.add_space(10.0);

        // tab bar
        let tab = self.setup.ach_tab.min(Category::ALL.len() - 1);
        let mut clicked = None;
        egui::ScrollArea::horizontal()
            .id_salt("ach-tabs")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (i, cat) in Category::ALL.iter().enumerate() {
                        let got = ACHIEVEMENTS
                            .iter()
                            .filter(|a| a.cat == *cat && self.cfg.achievements.iter().any(|u| u.id == a.id))
                            .count();
                        let all = ACHIEVEMENTS.iter().filter(|a| a.cat == *cat).count();
                        if category_tab(ui, cat.label(), got, all, i == tab).clicked() {
                            clicked = Some(i);
                        }
                    }
                });
            });
        if let Some(i) = clicked {
            self.setup.ach_tab = i;
        }
        let (lr, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(lr, 0.0, LINE());
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            kit::segmented(
                ui,
                &mut self.setup.ach_filter,
                &[(AchFilter::All, "  All  "), (AchFilter::Unlocked, "  Unlocked  "), (AchFilter::Locked, "  Locked  ")],
            );
        });
        ui.add_space(10.0);

        let cat = Category::ALL[tab];
        let filter = self.setup.ach_filter;
        // Every achievement as its own tile (no stacks), in order: each
        // series together, its steps from easiest to hardest.
        let mut list: Vec<&'static Achievement> = Vec::new();
        for a in ACHIEVEMENTS.iter().filter(|a| a.cat == cat) {
            let name = progress::series_of(a.id);
            if !list.iter().any(|x| progress::series_of(x.id) == name) {
                list.extend(progress::series(name).into_iter().filter(|x| x.cat == cat));
            }
        }
        let is_done = |id: &str| self.cfg.achievements.iter().any(|u| u.id == id);
        list.retain(|a| match filter {
            AchFilter::All => true,
            AchFilter::Unlocked => is_done(a.id),
            AchFilter::Locked => !is_done(a.id),
        });
        if list.is_empty() {
            ui.label(
                RichText::new(match filter {
                    AchFilter::Unlocked => "None unlocked here yet. Keep hunting!",
                    _ => "All done here!",
                })
                .size(kit::fs(12.5))
                .color(MUTED()),
            );
        }
        let gap = 10.0;
        let cols = (((ui.available_width() + gap) / (340.0 + gap)).floor() as usize).clamp(1, 4);
        let tw = ((ui.available_width() - gap * (cols - 1) as f32) / cols as f32).floor();
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for a in row {
                    let when = self.cfg.achievements.iter().find(|u| u.id == a.id).map(|u| u.unix);
                    self.achievement_tile(ui, *a, when, &l, tw, 0);
                }
            });
            ui.add_space(gap);
        }
        if self.cfg.events.is_empty() && unlocked == 0 {
            kit::pill(ui, "Start hunting and your first achievements will pop up!", PillKind::Info);
        }
    }
}

/// Big full-width tabs for the Profile page sections.
fn section_tabs(
    ui: &mut egui::Ui,
    current: ProfileSection,
    tabs: &[(ProfileSection, String, Option<(usize, usize)>)],
) -> Option<ProfileSection> {
    if kit::modern() {
        return section_tabs_cirrus(ui, current, tabs);
    }
    if kit::clean() {
        return section_tabs_clean(ui, current, tabs);
    }
    let mut out = None;
    let h = 42.0;
    let (bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), Sense::hover());
    ui.painter().rect_filled(bar, kit::r(12.0), CARD());
    let n = tabs.len() as f32;
    let cw = (bar.width() - 8.0) / n;
    for (i, (sec, label, count)) in tabs.iter().enumerate() {
        let r = Rect::from_min_size(egui::pos2(bar.min.x + 4.0 + cw * i as f32, bar.min.y + 4.0), egui::vec2(cw, h - 8.0));
        let resp = ui.interact(r, ui.id().with(("profile-tab", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let sel = *sec == current;
        let p = ui.painter();
        if sel {
            p.add(theme::button_shape(r, 9.0, resp.hovered()));
        } else if resp.hovered() {
            p.rect_filled(r, 9.0, CARD_HOVER());
        }
        let ink = if sel { theme::button_ink() } else if resp.hovered() { TEXT() } else { MUTED() };
        let font = kit::font(14.0);
        let lw = p.layout_no_wrap(label.clone(), font.clone(), ink).size().x;
        let chip = count.map(|(g, a)| (format!("{g}/{a}"), g == a && a > 0));
        let chw = chip.as_ref().map_or(0.0, |(t, _)| {
            p.layout_no_wrap(t.clone(), kit::font(11.0), ink).size().x + 14.0
        });
        let total = lw + if chip.is_some() { chw + 8.0 } else { 0.0 };
        let x0 = r.center().x - total / 2.0;
        p.text(egui::pos2(x0, r.center().y), Align2::LEFT_CENTER, label, font, ink);
        if let Some((t, full)) = chip {
            let cr = Rect::from_min_size(egui::pos2(x0 + lw + 8.0, r.center().y - 9.0), egui::vec2(chw, 18.0));
            let (bg, fg) = if full {
                (GOLD, OUTLINE)
            } else if sel {
                (Color32::from_black_alpha(60), ink)
            } else {
                (CARD_HOVER(), MUTED())
            };
            p.rect_filled(cr, 9.0, bg);
            p.text(cr.center(), Align2::CENTER_CENTER, t, kit::font(11.0), fg);
        }
        if resp.clicked() && !sel {
            out = Some(*sec);
        }
    }
    out
}

/// Cirrus section tabs: a Cloud pill track; the open tab is a pill in the button colour,
/// with its count in a small chip.
fn section_tabs_cirrus(
    ui: &mut egui::Ui,
    current: ProfileSection,
    tabs: &[(ProfileSection, String, Option<(usize, usize)>)],
) -> Option<ProfileSection> {
    let mut out = None;
    let h = 44.0;
    let (bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    p.add(kit::shadow_tier(1).as_shape(bar, h / 2.0));
    p.rect_filled(bar, h / 2.0, CARD());
    p.rect_stroke(bar, h / 2.0, Stroke::new(1.0f32, LINE()));
    let n = tabs.len() as f32;
    let (pad, gap) = (5.0, 4.0);
    let cw = (bar.width() - 2.0 * pad - gap * (n - 1.0)) / n;
    for (i, (sec, label, count)) in tabs.iter().enumerate() {
        let r = Rect::from_min_size(egui::pos2(bar.min.x + pad + (cw + gap) * i as f32, bar.min.y + pad), egui::vec2(cw, h - 2.0 * pad));
        let resp = ui.interact(r, ui.id().with(("profile-tab", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let sel = *sec == current;
        let hovered = resp.hovered();
        let p = ui.painter();
        let rad = r.height() / 2.0;
        if sel {
            p.add(kit::shadow_tier(2).as_shape(r, rad));
            p.add(kit::primary_shape(r, false));
        } else if hovered {
            p.rect_filled(r, rad, kit::sunken());
        }
        let ink = if sel { kit::on_primary() } else if hovered { TEXT() } else { MUTED() };
        let font = crate::fonts::sans_bold_font(13.5);
        let lw = p.layout_no_wrap(label.clone(), font.clone(), ink).size().x;
        let chip = count.map(|(g, a)| format!("{g}/{a}"));
        let chw = chip.as_ref().map_or(0.0, |t| p.layout_no_wrap(t.clone(), kit::font(11.0), ink).size().x + 14.0);
        let total = lw + if chip.is_some() { chw + 8.0 } else { 0.0 };
        let x0 = r.center().x - total / 2.0;
        p.text(egui::pos2(x0, r.center().y), Align2::LEFT_CENTER, label, font, ink);
        if let Some(t) = chip {
            let cr = Rect::from_min_size(egui::pos2(x0 + lw + 8.0, r.center().y - 9.0), egui::vec2(chw, 18.0));
            if sel {
                p.rect_filled(cr, 9.0, kit::on_primary().gamma_multiply(0.18));
            } else {
                p.rect_filled(cr, 9.0, kit::sunken());
                p.rect_stroke(cr, 9.0, Stroke::new(1.0f32, LINE()));
            }
            p.text(cr.center(), Align2::CENTER_CENTER, t, kit::font(11.0), if sel { ink } else { MUTED() });
        }
        if resp.clicked() && !sel {
            out = Some(*sec);
        }
    }
    out
}

/// Clean-style section tabs: the same look as the other segmented groups
/// (outlined bar, grey highlight on the open tab, no accent fill).
fn section_tabs_clean(
    ui: &mut egui::Ui,
    current: ProfileSection,
    tabs: &[(ProfileSection, String, Option<(usize, usize)>)],
) -> Option<ProfileSection> {
    let mut out = None;
    let h = 36.0;
    let (bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), Sense::hover());
    ui.painter().rect_stroke(bar.shrink(0.5), 8.0, Stroke::new(1.0f32, kit::CONTROL()));
    let n = tabs.len() as f32;
    let cw = (bar.width() - 6.0 - 2.0 * (n - 1.0)) / n;
    for (i, (sec, label, count)) in tabs.iter().enumerate() {
        let r = Rect::from_min_size(
            egui::pos2(bar.min.x + 3.0 + (cw + 2.0) * i as f32, bar.min.y + 3.0),
            egui::vec2(cw, h - 6.0),
        );
        let resp = ui.interact(r, ui.id().with(("profile-tab", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let sel = *sec == current;
        let hovered = resp.hovered();
        let p = ui.painter();
        if sel {
            p.rect_filled(r, 6.0, kit::SELECTED_BG());
        } else if hovered {
            p.rect_filled(r, 6.0, CARD_HOVER());
        }
        let ink = if sel || hovered { TEXT() } else { MUTED() };
        let font = if sel { crate::fonts::sans_bold_font(13.5) } else { crate::fonts::sans_font(13.5) };
        let lw = p.layout_no_wrap(label.clone(), font.clone(), ink).size().x;
        let chip_font = crate::fonts::sans_font(12.0);
        let chip = count.map(|(g, a)| (format!("{g}/{a}"), g == a && a > 0));
        let chw = chip.as_ref().map_or(0.0, |(t, _)| p.layout_no_wrap(t.clone(), chip_font.clone(), ink).size().x + 14.0);
        let total = lw + if chip.is_some() { chw + 8.0 } else { 0.0 };
        let x0 = r.center().x - total / 2.0;
        p.text(egui::pos2(x0, r.center().y), Align2::LEFT_CENTER, label, font, ink);
        if let Some((t, full)) = chip {
            let cr = Rect::from_min_size(egui::pos2(x0 + lw + 8.0, r.center().y - 10.0), egui::vec2(chw, 20.0));
            // Grey count chip; gold only when the set is complete.
            let (bg, fg) = if full {
                (GOLD, OUTLINE)
            } else if sel {
                (kit::CONTROL(), TEXT())
            } else {
                (kit::SELECTED_BG(), MUTED())
            };
            p.rect_filled(cr, 10.0, bg);
            p.text(cr.center(), Align2::CENTER_CENTER, t, chip_font.clone(), fg);
        }
        if resp.clicked() && !sel {
            out = Some(*sec);
        }
    }
    out
}

/// A stacked series being drawn as one tile.
struct StackInfo {
    steps: Vec<&'static Achievement>,
    got: usize,
    next: Option<&'static Achievement>,
}

/// One category tab: label + "got/all", underlined when selected.
fn category_tab(ui: &mut egui::Ui, label: &str, got: usize, all: usize, selected: bool) -> egui::Response {
    let font = kit::font(13.0);
    let count = format!("{got}/{all}");
    let lw = ui.painter().layout_no_wrap(label.to_string(), font.clone(), TEXT()).size().x;
    let cw = ui.painter().layout_no_wrap(count.clone(), kit::font(10.5), TEXT()).size().x;
    let (r, resp) = ui.allocate_exact_size(egui::vec2(lw + cw + 34.0, 36.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(r, egui::Rounding { nw: 8.0, ne: 8.0, sw: 0.0, se: 0.0 }, CARD());
        let underline = Rect::from_min_max(egui::pos2(r.min.x + 6.0, r.max.y - 3.0), egui::pos2(r.max.x - 6.0, r.max.y));
        if kit::clean() {
            // Clean style: a plain text-coloured underline, like Creative Cloud's tabs.
            p.rect_filled(underline, 1.5, TEXT());
        } else {
            p.add(theme::button_shape(underline, 1.5, false));
        }
    } else if resp.hovered() {
        p.rect_filled(r, egui::Rounding { nw: 8.0, ne: 8.0, sw: 0.0, se: 0.0 }, CARD_HOVER());
    }
    let tc = if selected { TEXT() } else { MUTED() };
    p.text(egui::pos2(r.min.x + 12.0, r.center().y), Align2::LEFT_CENTER, label, font, tc);
    let chip = Rect::from_min_size(egui::pos2(r.min.x + 16.0 + lw, r.center().y - 8.0), egui::vec2(cw + 10.0, 16.0));
    let full = got == all && all > 0;
    p.rect_filled(chip, 8.0, if full { GOLD } else { CARD_HOVER() });
    p.text(
        chip.center(),
        Align2::CENTER_CENTER,
        count,
        kit::font(10.5),
        if full { OUTLINE } else { tc },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

impl App {
    /// A tile for one achievement: its Pokémon (shiny and animated when
    /// unlocked, a silhouette while locked), medal, name, details, progress.
    /// A tile for a whole series: the highest step earned (or the first
    /// step if none), pips for every step and the next goal. Returns true
    /// when clicked (to show every step).
    #[allow(dead_code)]
    fn unit_tile(&mut self, ui: &mut egui::Ui, unit: &[&'static Achievement], l: &Lifetime, w: f32) -> bool {
        let got = unit.iter().take_while(|a| self.cfg.achievements.iter().any(|u| u.id == a.id)).count();
        if unit.len() == 1 {
            let a = unit[0];
            let when = self.cfg.achievements.iter().find(|u| u.id == a.id).map(|u| u.unix);
            self.achievement_tile(ui, a, when, l, w, 0);
            return false;
        }
        // show the highest earned step; progress is towards the next one
        let shown = unit[got.saturating_sub(1)];
        let when = if got > 0 {
            self.cfg.achievements.iter().find(|u| u.id == shown.id).map(|u| u.unix)
        } else {
            None
        };
        let next = unit.get(got).copied();
        let ctx = StackInfo { steps: unit.to_vec(), got, next };
        let resp = self.achievement_tile_stack(ui, shown, when, l, w, Some(&ctx));
        resp
    }

    fn achievement_tile(&mut self, ui: &mut egui::Ui, a: &'static Achievement, when: Option<u64>, l: &Lifetime, w: f32, _depth: u32) {
        let _ = self.achievement_tile_stack(ui, a, when, l, w, None);
    }

    fn achievement_tile_stack(
        &mut self,
        ui: &mut egui::Ui,
        a: &'static Achievement,
        when: Option<u64>,
        l: &Lifetime,
        w: f32,
        stack: Option<&StackInfo>,
    ) -> bool {
        let done = when.is_some();
        let tc = a.tier.rgb();
        let tier_col = Color32::from_rgb(tc[0], tc[1], tc[2]);
        let sense = if stack.is_some() { Sense::click() } else { Sense::hover() };
        let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 104.0), sense);
        let p = ui.painter_at(r.expand(8.0));
        if let Some(st) = stack {
            // cards peeking out behind: it's a stack
            let behind = (st.steps.len() - 1).min(2);
            for k in (1..=behind).rev() {
                let off = k as f32 * 4.0;
                let br = r.translate(egui::vec2(off, off));
                p.rect_filled(br, kit::r(12.0), CARD_HOVER().gamma_multiply(0.9));
                p.rect_stroke(br, kit::r(12.0), Stroke::new(1.0f32, LINE()));
            }
        }
        // card
        if done {
            p.rect_filled(r.expand(2.0), kit::r(14.0), tier_col.gamma_multiply(if resp.hovered() { 0.35 } else { 0.18 }));
        }
        p.rect_filled(r, kit::r(12.0), if resp.hovered() { CARD_HOVER() } else { CARD() });
        p.rect_stroke(r, kit::r(12.0), Stroke::new(if done { 1.6f32 } else { 1.0f32 }, if done { tier_col } else { LINE() }));
        // sprite stage
        let stage = Rect::from_min_size(r.min + egui::vec2(8.0, 8.0), egui::vec2(88.0, 88.0));
        let bg = if done { tier_col.gamma_multiply(0.22) } else { CARD_HOVER() };
        p.rect_filled(stage, kit::r(10.0), bg);
        theme::platform(&p, stage.center_bottom() - egui::vec2(0.0, 12.0), 28.0, 6.0);
        let slug = progress::mascot(a.id);
        if let Some(sp) = self.species.iter().find(|s| s.slug == slug).cloned() {
            let tint = if done { Color32::WHITE } else { Color32::from_rgba_unmultiplied(10, 8, 18, 235) };
            self.draw_sprite_tinted(&p, &sp, true, stage.center_bottom() - egui::vec2(0.0, 10.0), egui::vec2(80.0, 74.0), tint);
        }
        if done {
            theme::sparkle(&p, stage.right_top() + egui::vec2(-10.0, 10.0), 5.0, GOLD);
        }
        // medal badge overlapping the stage corner
        theme::medal(&p, stage.left_bottom() + egui::vec2(12.0, -14.0), 9.0, tc, !done);
        // text
        let tx = stage.max.x + 12.0;
        let tw = r.max.x - tx - 10.0;
        let clip = p.with_clip_rect(Rect::from_min_max(egui::pos2(tx, r.min.y), egui::pos2(r.max.x - 8.0, r.max.y)));
        let name = if a.secret && !done { "???" } else { a.name };
        // keep the name clear of the step pips on stacked tiles
        let name_right = match stack {
            Some(st) => r.max.x - 14.0 - st.steps.len() as f32 * 13.0,
            None => r.max.x - 8.0,
        };
        let name_clip = p.with_clip_rect(Rect::from_min_max(egui::pos2(tx, r.min.y), egui::pos2(name_right, r.max.y)));
        name_clip.text(
            egui::pos2(tx, r.min.y + 18.0),
            Align2::LEFT_CENTER,
            name,
            kit::font(14.5),
            if done { TEXT() } else { MUTED() },
        );
        // tier + xp chip
        let chip_txt = format!("{} · +{} XP", a.tier.label().to_uppercase(), a.tier.xp());
        let chip_w = clip.layout_no_wrap(chip_txt.clone(), kit::font_small(9.5), TEXT()).size().x + 12.0;
        let chip = Rect::from_min_size(egui::pos2(tx, r.min.y + 30.0), egui::vec2(chip_w, 15.0));
        clip.rect_filled(chip, 7.5, tier_col.gamma_multiply(if done { 0.95 } else { 0.3 }));
        clip.text(
            chip.center(),
            Align2::CENTER_CENTER,
            chip_txt,
            kit::font_small(9.5),
            if done { OUTLINE } else { MUTED() },
        );
        // description (wrapped to two lines)
        let desc = if a.secret && !done { "A secret. Keep hunting!" } else { a.desc };
        let galley = clip.layout(desc.to_string(), kit::font(11.5), MUTED(), tw);
        clip.galley(egui::pos2(tx, r.min.y + 50.0), galley, MUTED());
        // bottom line: unlocked date or progress
        let by = r.max.y - 14.0;
        if let Some(st) = stack {
            // pips for each step, top-right
            let n = st.steps.len();
            for (i, step) in st.steps.iter().enumerate() {
                let c = egui::pos2(r.max.x - 14.0 - (n - 1 - i) as f32 * 13.0, r.min.y + 16.0);
                let s_rgb = step.tier.rgb();
                if i < st.got {
                    p.circle_filled(c, 5.0, Color32::from_rgb(s_rgb[0], s_rgb[1], s_rgb[2]));
                    p.circle_stroke(c, 5.0, Stroke::new(1.0f32, OUTLINE));
                } else {
                    p.circle_stroke(c, 4.5, Stroke::new(1.2f32, LINE()));
                }
            }
            match st.next {
                Some(nx) => {
                    let (c, t) = progress::progress(nx, l);
                    let frac = (c / t).clamp(0.0, 1.0) as f32;
                    let fmt = |v: f64| {
                        if matches!(nx.goal, progress::Goal::HuntHours(_)) {
                            format!("{v:.1}h")
                        } else {
                            theme::thousands(v as u64)
                        }
                    };
                    let label = if progress::is_binary(nx) {
                        format!("Next: {}", nx.name)
                    } else {
                        format!("Next: {}  {} / {}", nx.name, fmt(c.min(t)), fmt(t))
                    };
                    let bar = Rect::from_min_size(egui::pos2(tx, by + 2.0), egui::vec2(tw, 5.0));
                    clip.rect_filled(bar, 2.5, LINE());
                    if frac > 0.0 {
                        clip.rect_filled(Rect::from_min_size(bar.min, egui::vec2((bar.width() * frac).max(5.0), 5.0)), 2.5, GOLD);
                    }
                    clip.text(egui::pos2(tx, by - 7.0), Align2::LEFT_CENTER, label, kit::font(10.5), MUTED());
                }
                None => {
                    clip.text(
                        egui::pos2(tx, by),
                        Align2::LEFT_CENTER,
                        format!("★ All {} steps complete!", st.steps.len()),
                        kit::font(11.0),
                        GOLD,
                    );
                }
            }
        } else if let Some(t) = when {
            clip.text(
                egui::pos2(tx, by),
                Align2::LEFT_CENTER,
                format!("✔ Unlocked {}", fmt_date(t)),
                kit::font(11.0),
                theme::GREEN,
            );
        } else if !progress::is_binary(a) {
            let (c, t) = progress::progress(a, l);
            let frac = (c / t).clamp(0.0, 1.0) as f32;
            let fmt = |v: f64| {
                if matches!(a.goal, progress::Goal::HuntHours(_)) {
                    format!("{v:.1}h")
                } else {
                    theme::thousands(v as u64)
                }
            };
            let label = format!("{} / {}", fmt(c.min(t)), fmt(t));
            let lw = clip.layout_no_wrap(label.clone(), kit::font(10.5), MUTED()).size().x;
            let bar = Rect::from_min_size(egui::pos2(tx, by - 3.0), egui::vec2((tw - lw - 10.0).max(30.0), 6.0));
            clip.rect_filled(bar, 3.0, LINE());
            if frac > 0.0 {
                clip.rect_filled(Rect::from_min_size(bar.min, egui::vec2((bar.width() * frac).max(6.0), 6.0)), 3.0, GOLD);
            }
            clip.text(egui::pos2(r.max.x - 10.0, by), Align2::RIGHT_CENTER, label, kit::font(10.5), MUTED());
        } else {
            clip.text(
                egui::pos2(tx, by),
                Align2::LEFT_CENTER,
                "Not yet",
                kit::font(11.0),
                MUTED(),
            );
        }
        if let Some(st) = stack {
            let open = resp.hovered();
            if open {
                p.rect_stroke(r, kit::r(12.0), Stroke::new(1.5f32, theme::RED()));
            }
            return resp
                .on_hover_text(format!("Step {} of {} · click to see every step", st.got.max(1), st.steps.len()))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked();
        }
        if done {
            let _ = resp.on_hover_text(format!("{} · {}", a.tier.label(), a.desc));
        }
        false
    }
}


// ───────────────────────────── trainer card image ─────────────────────────────

#[derive(Default)]
pub(super) struct ProfileCardView {
    tex: Option<egui::TextureHandle>,
    image: Option<crate::canvas::Canvas>,
    /// What the preview was drawn with (options, toggles, sprites ready, level, name).
    drawn: Option<(crate::card::CardOptions, bool, bool, usize, u32, String)>,
    msg: Option<(bool, String)>,
    saved: Option<std::path::PathBuf>,
}

impl App {
    fn profile_data(&mut self) -> (crate::card::ProfileData, usize) {
        let l = self.lifetime_full();
        let xp = self.total_xp(&l).total();
        let level = progress::level_for(xp);
        let (into, need) = progress::level_progress(xp);
        let (metal, main, light) = progress::badge_metal(level);
        let unlocked = self.cfg.achievements.iter().filter(|u| progress::find(&u.id).is_some()).count();
        let per_shiny = if l.shinies > 0 { theme::thousands(l.encounters / l.shinies) } else { "-".into() };
        let stats = vec![
            ("Encounters".to_string(), theme::thousands(l.encounters)),
            ("Hunt time".to_string(), fmt_duration(l.hunt_secs)),
            ("Shinies".to_string(), l.shinies.to_string()),
            ("Achievements".to_string(), format!("{unlocked} / {}", ACHIEVEMENTS.len())),
            ("Best day".to_string(), theme::thousands(l.best_day)),
            ("Longest streak".to_string(), format!("{} days", l.longest_streak)),
            ("Pokémon hunted".to_string(), l.targets.to_string()),
            ("Avg per shiny".to_string(), per_shiny),
        ];
        let mut ready = 0;
        // shinies, newest first
        let mut shinies = Vec::new();
        let history: Vec<_> = self.cfg.history.iter().rev().take(6).cloned().collect();
        for h in history {
            let slug = h.slug.clone().or_else(|| {
                self.species.iter().find(|s| s.name.eq_ignore_ascii_case(&h.target)).map(|s| s.slug.clone())
            });
            let img = slug.as_deref().and_then(|s| {
                if let Some(sp) = self.species.iter().find(|x| x.slug == s).cloned() {
                    let _ = self.sprites.get(&sp, true);
                }
                crate::share::sprite_frame(s, true)
            });
            ready += img.is_some() as usize;
            shinies.push((img, h.target.clone(), h.encounters));
        }
        // recent achievements with their Pokémon
        let mut recent: Vec<_> = self.cfg.achievements.clone();
        recent.sort_by_key(|u| std::cmp::Reverse(u.unix));
        let mut achievements = Vec::new();
        for u in recent.iter().filter_map(|u| progress::find(&u.id)).take(6) {
            let slug = progress::mascot(u.id);
            if let Some(sp) = self.species.iter().find(|x| x.slug == slug).cloned() {
                let _ = self.sprites.get(&sp, true);
            }
            let img = crate::share::sprite_frame(slug, true);
            ready += img.is_some() as usize;
            achievements.push((u.name.to_string(), u.tier.rgb(), img));
        }
        let data = crate::card::ProfileData {
            name: self.cfg.trainer_name.clone(),
            level,
            title: progress::title(level).to_string(),
            metal: (metal.to_string(), main, light),
            xp_into: into,
            xp_need: need,
            total_xp: xp,
            max_level: level >= progress::MAX_LEVEL,
            stats,
            shinies,
            achievements,
            ach_count: (unlocked, ACHIEVEMENTS.len()),
            show_shinies: self.cfg.profile_card_shinies,
            show_achievements: self.cfg.profile_card_achievements,
            date: fmt_date(now_unix()),
            icon: Some(crate::share::icon()),
        };
        (data, ready)
    }

    pub(super) fn page_profile_card(&mut self, ui: &mut egui::Ui) {
        if super::hunt_pages::back_button(ui, "Profile") {
            self.setup.profile_card = None;
            return;
        }
        kit::page_header(ui, "TRAINER CARD", "Your level, stats, shinies and achievements in one picture.");

        let (data, ready) = self.profile_data();
        let thumb_data = data.clone();
        let opts = {
            let mut o = self.cfg.card.clone();
            o.animated = false;
            o
        };
        let key = (
            opts.clone(),
            self.cfg.profile_card_shinies,
            self.cfg.profile_card_achievements,
            ready,
            data.level,
            self.cfg.trainer_name.clone(),
        );
        let stale = self.setup.profile_card.as_ref().is_some_and(|v| v.drawn.as_ref() != Some(&key));
        if stale {
            let fonts = {
                let reload = match &self.card_text {
                    None => true,
                    Some(t) => !t.has_pixel() && crate::config::cache_dir().join("PressStart2P-Regular.ttf").exists(),
                };
                if reload {
                    self.card_text = Some(crate::share::AbText::load());
                }
                self.card_text.as_ref().expect("fonts")
            };
            let img = crate::card::render_profile(&data, &opts, fonts);
            let pw = img.w.min(1280);
            let ph = (img.h as f32 * pw as f32 / img.w as f32).round() as u32;
            let px = match image::RgbaImage::from_raw(img.w, img.h, img.px.clone()) {
                Some(big) if pw < img.w => image::imageops::resize(&big, pw, ph, image::imageops::FilterType::Triangle).into_raw(),
                _ => img.px.clone(),
            };
            let (tw, th) = if pw < img.w { (pw, ph) } else { (img.w, img.h) };
            let tex = ui.ctx().load_texture(
                "profile-card-preview",
                egui::ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], &px),
                egui::TextureOptions::LINEAR,
            );
            if let Some(v) = self.setup.profile_card.as_mut() {
                v.tex = Some(tex);
                v.image = Some(img);
                v.drawn = Some(key);
            }
        }
        // sprites still downloading? look again shortly
        let wanted = data.shinies.len() * self.cfg.profile_card_shinies as usize
            + data.achievements.len() * self.cfg.profile_card_achievements as usize;
        if ready < wanted {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }

        // preview
        if let Some(tex) = self.setup.profile_card.as_ref().and_then(|v| v.tex.clone()) {
            let size = tex.size_vec2();
            let w = ui.available_width().min(if self.cfg.card.layout == crate::card::CardLayout::Square { 440.0 } else { 640.0 });
            let h = w * size.y / size.x;
            ui.vertical_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(w, h), Sense::hover());
                ui.painter().rect_filled(r.translate(egui::vec2(3.0, 5.0)), 12.0, Color32::from_black_alpha(70));
                egui::Image::from_texture(egui::load::SizedTexture::new(tex.id(), size))
                    .rounding(kit::r(12.0))
                    .paint_at(ui, r);
            });
        }
        ui.add_space(12.0);

        // actions
        let folder = self.share_folder();
        let mut shared = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if super::hunt_pages::primary(ui, "  📋  Copy for Discord  ")
                .on_hover_text("Then paste in Discord with Ctrl+V")
                .clicked()
            {
                if let Some(v) = self.setup.profile_card.as_mut() {
                    v.msg = Some(match v.image.as_ref().map(crate::share::copy_image) {
                        Some(Ok(())) => {
                            shared = true;
                            (true, "✔ Copied! Paste it into Discord with Ctrl+V".into())
                        }
                        Some(Err(e)) => (false, format!("Couldn't copy: {e}")),
                        None => (false, "Still drawing, try again".into()),
                    });
                }
            }
            if super::hunt_pages::secondary(ui, "  💾  Save image  ")
                .on_hover_text(format!("Saves it in {}", folder.display()))
                .clicked()
            {
                let name = if self.cfg.trainer_name.trim().is_empty() {
                    "trainer-card".to_string()
                } else {
                    format!("{}-trainer-card", self.cfg.trainer_name.trim())
                };
                if let Some(v) = self.setup.profile_card.as_mut() {
                    let res = v
                        .image
                        .as_ref()
                        .and_then(crate::share::png_bytes)
                        .map(|b| crate::share::save_bytes(&b, &name, "png", &folder));
                    match res {
                        Some(Ok(path)) => {
                            shared = true;
                            v.msg = Some((true, format!("✔ Saved to {}", path.display())));
                            v.saved = Some(path);
                        }
                        Some(Err(e)) => v.msg = Some((false, format!("Couldn't save: {e}"))),
                        None => v.msg = Some((false, "Still drawing, try again".into())),
                    }
                }
            }
            if let Some(path) = self.setup.profile_card.as_ref().and_then(|v| v.saved.clone()) {
                if ui.add(egui::Button::new(RichText::new("Show in folder").color(theme::LINK())).frame(false)).clicked() {
                    crate::share::reveal(&path);
                }
            }
        });
        if shared {
            self.add_event("show_off");
        }
        if let Some((ok, msg)) = self.setup.profile_card.as_ref().and_then(|v| v.msg.clone()) {
            ui.add_space(6.0);
            kit::pill(ui, &msg, if ok { PillKind::Ok } else { PillKind::Bad });
        }
        ui.add_space(14.0);

        // options
        let before = (self.cfg.card.clone(), self.cfg.profile_card_shinies, self.cfg.profile_card_achievements, self.cfg.trainer_name.clone());
        kit::section(ui, "Show on the card");
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Trainer name").size(kit::fs(13.0)).color(TEXT()));
                ui.add(
                    egui::TextEdit::singleline(&mut self.cfg.trainer_name)
                        .hint_text("Your trainer name")
                        .char_limit(16)
                        .desired_width(220.0),
                );
            });
            ui.add_space(6.0);
            kit::checkbox(ui, &mut self.cfg.profile_card_shinies, "Shiny showcase (your latest 6 shinies)");
            kit::checkbox(ui, &mut self.cfg.profile_card_achievements, "Recent achievements (your latest 6)");
        });
        kit::section(ui, "Style");
        {
            let data = thumb_data;
            let o = self.cfg.card.clone();
            let key = format!("profile|{:?}|{:?}|{}|{}|{}|{}|{}|{}", o.theme, o.layout, data.level, data.xp_into, ready, self.cfg.trainer_name, self.cfg.profile_card_shinies, self.cfg.profile_card_achievements);
            let ctx = ui.ctx().clone();
            self.style_thumbs(&ctx, key, |_| (super::hunt_pages::ThumbOf::Profile(data), o));
        }
        self.card_style_picker(ui);
        kit::section(ui, "Theme (pick a type)");
        kit::card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
                for t in crate::card::CardTheme::ALL {
                    if super::hunt_pages::theme_swatch(ui, t, self.cfg.card.theme == t).clicked() {
                        self.cfg.card.theme = t;
                    }
                }
            });
        });
        kit::section(ui, "Size");
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                kit::segmented(
                    ui,
                    &mut self.cfg.card.layout,
                    &[(crate::card::CardLayout::Wide, "  Wide 16:9 (Discord)  "), (crate::card::CardLayout::Square, "  Square 1:1  ")],
                );
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if !crate::card::RESOLUTIONS.contains(&self.cfg.card.res) {
                    self.cfg.card.res = 1080;
                }
                kit::segmented(
                    ui,
                    &mut self.cfg.card.res,
                    &[(480, "  480p  "), (720, "  720p  "), (1080, "  1080p  "), (1440, "  1440p  ")],
                );
                let (w, h, _) = self.cfg.card.output_size();
                ui.label(RichText::new(format!("{w} × {h} px")).size(kit::fs(11.5)).color(MUTED()));
            });
        });
        let after = (self.cfg.card.clone(), self.cfg.profile_card_shinies, self.cfg.profile_card_achievements, self.cfg.trainer_name.clone());
        if after != before {
            self.dirty = true;
            if let Some(v) = self.setup.profile_card.as_mut() {
                v.msg = None;
                v.saved = None;
            }
        }
    }
}
