//! The Dashboard's core widgets, laid out by the box they fill (see
//! `w_fit`): Today at a glance, Clock, This week, Current hunt, Recent
//! shinies and Rare grass today. Each has a layout for every shape: one
//! thing big in a small square, a strip when it's flat, a column when it's
//! tall, and more of everything when it's roomy.

use super::home::ago;
use super::ui_kit::{self as kit, CARD, MUTED, TEXT};
use super::w_build::{Head, Lead, Row};
use super::w_fit::{self as fit, Shape, WBox};
use super::{App, Tab};
use crate::stats::{fmt_duration, found_by};
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Rect};

use super::w_fit::wbox;

impl App {
    /// The hunt's sprite in a soft circle filling `r` (a "?" with no target).
    fn hunt_sprite(&mut self, ui: &egui::Ui, p: &egui::Painter, r: Rect, b: &WBox) {
        let side = r.width().min(r.height());
        let c = r.center();
        p.circle_filled(c, side / 2.0, theme::blend(CARD(), kit::C_VIOLET, 0.14));
        match self.shown_target_idx(ui.ctx()) {
            Some(i) => {
                let sp = self.species[i].clone();
                self.draw_sprite_centered(p, &sp, true, c, vec2(side * 0.9, side * 0.9));
            }
            None => {
                p.text(c, Align2::CENTER_CENTER, "?", kit::font_bold(side * 0.4), MUTED());
            }
        }
        if side > b.px(40.0) {
            theme::sparkle(p, c + vec2(side * 0.36, -side * 0.36), b.px(5.0), GOLD);
        }
    }

    /// Start (or stop, or show) the overlay, as a button filling `r`.
    pub(super) fn overlay_pill(&mut self, ui: &mut egui::Ui, r: Rect, id: impl std::hash::Hash) {
        if self.overlay_on && self.ov_hidden {
            if fit::button(ui, r, "▶  Show overlay", true, id) {
                self.ov_hidden = false;
            }
        } else if self.any_overlay() {
            if fit::button(ui, r, "■  Stop overlay", false, id) {
                self.stop_overlay();
            }
        } else if fit::button(ui, r, "▶  Start overlay", true, id) {
            self.start_overlay();
        }
    }

    // ── Today at a glance ────────────────────────────────────────────────

    /// Today in boxes (encounters, daily goal, streak, trainer level) that
    /// fill the widget: a strip when it's long and flat, two by two when
    /// it's squarer, one under another when it's tall; fewer boxes when
    /// four won't fit, and each shows as much as its room allows.
    pub(super) fn f_today(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "today");
        let days = self.home_days();
        let today_n = crate::stats::local_day(crate::stats::now_unix());
        let (today, _) = self.day_totals();
        let week: Vec<u64> = (0..7).map(|k| days.get(&(today_n - 6 + k)).copied().unwrap_or(0)).collect();
        let goal = self.cfg.daily_goal;
        let streak = self.daily_streak();
        let l = self.lifetime_full();
        let xp = self.total_xp(&l).total();
        let level = crate::progress::level_for(xp);
        let (into, need) = crate::progress::level_progress(xp);
        let title = self.player_title(level);
        let hunt_secs = self.today_hunt_secs();

        let area = b.inner();
        let gap = b.px(10.0);
        // how many boxes, and how: the most that fit, laid out closest to
        // a 2:1 box
        let (min_w, min_h) = (b.px(96.0), b.px(46.0));
        let mut pick: Option<(usize, usize, usize)> = None;
        for k in [4usize, 2, 1] {
            let mut best: Option<(f32, usize, usize)> = None;
            for (c, r) in [(4usize, 1usize), (2, 2), (1, 4), (2, 1), (1, 2), (1, 1)] {
                if c * r != k {
                    continue;
                }
                let bw = (area.width() - gap * (c as f32 - 1.0)) / c as f32;
                let bh = (area.height() - gap * (r as f32 - 1.0)) / r as f32;
                if bw < min_w || bh < min_h {
                    continue;
                }
                let score = ((bw / bh) / 2.0).ln().abs();
                if best.is_none_or(|x| score < x.0) {
                    best = Some((score, c, r));
                }
            }
            if let Some((_, c, r)) = best {
                pick = Some((k, c, r));
                break;
            }
        }
        let (k, c, r) = pick.unwrap_or((1, 1, 1));
        let bw = (area.width() - gap * (c as f32 - 1.0)) / c as f32;
        let bh = (area.height() - gap * (r as f32 - 1.0)) / r as f32;
        let mut open: Option<usize> = None;
        for i in 0..k {
            let rr = Rect::from_min_size(area.min + vec2((i % c) as f32 * (bw + gap), (i / c) as f32 * (bh + gap)), vec2(bw, bh));
            let resp = fit::hit(ui, rr, ("today-box", i, b.r.min.x as i32, b.r.min.y as i32), b.px(12.0));
            let p = ui.painter_at(rr);
            let (icon, col, label) = [("⚔", kit::C_VIOLET, "Today"), ("🎯", kit::C_PINK, "Daily goal"), ("🔥", kit::C_ORANGE, "Streak"), ("⭐", kit::C_GOLD, "Trainer level")][i];
            fit::tile(&p, rr, col, b.px(12.0));
            // what fits: the label, the note, the picture
            let pad = b.px(if bh > b.px(90.0) { 12.0 } else { 9.0 });
            let inner = rr.shrink(pad);
            let with_label = bh >= b.px(66.0);
            let with_note = bh >= b.px(84.0);
            let with_art = bw >= b.px(170.0) && bh >= b.px(54.0);
            let (lab_r, rest) = if with_label { fit::split_top(inner, b.px(16.0), b.px(4.0)) } else { (Rect::NOTHING, inner) };
            let (rest, note_r) = if with_note { fit::split_bottom(rest, b.px(15.0), b.px(4.0)) } else { (rest, Rect::NOTHING) };
            let (num_r, art_r) = if with_art { fit::split_right(rest, (bw * 0.42).min(b.px(190.0)), b.px(8.0)) } else { (rest, Rect::NOTHING) };
            if with_label {
                fit::label(&p, lab_r, &format!("{icon}  {label}"), b.font(11.5), MUTED(), Align2::LEFT_CENTER);
            }
            let num_max = (num_r.height() * 0.8).min(b.px(34.0));
            let (num, note, note_col) = match i {
                0 => (theme::thousands(today), format!("encounters · {}", fmt_duration(hunt_secs)), MUTED()),
                1 => {
                    if goal > 0 {
                        let done = today >= goal;
                        (format!("{} / {}", theme::thousands(today.min(goal)), theme::thousands(goal)), if done { "Goal reached ✔".to_string() } else { format!("{} to go", theme::thousands(goal - today)) }, if done { theme::GREEN } else { MUTED() })
                    } else {
                        ("Off".to_string(), "Set one on your Profile".to_string(), MUTED())
                    }
                }
                2 => {
                    let kept = week[6] > 0;
                    (
                        format!("{streak} day{}", if streak == 1 { "" } else { "s" }),
                        if streak == 0 { "Hunt today to start one".to_string() } else if kept { "Kept going today".to_string() } else { "Hunt today to keep it".to_string() },
                        if !kept && streak > 0 { kit::C_ORANGE } else { MUTED() },
                    )
                }
                _ => (level.to_string(), title.clone(), super::pass_page::title_col(&title, MUTED())),
            };
            fit::hero(&p, num_r, &num, num_max, TEXT(), Align2::LEFT_CENTER);
            if with_note {
                fit::label(&p, note_r, &note, b.font(11.0), note_col, Align2::LEFT_CENTER);
            }
            if with_art {
                match i {
                    0 => {
                        fit::bars(&p, art_r, &week, Some(6), col, None, b.font(9.0));
                    }
                    1 => {
                        let f = if goal > 0 { (today as f32 / goal as f32).min(1.0) } else { 0.0 };
                        let rad = (art_r.height().min(art_r.width()) / 2.0 - b.px(4.0)).max(8.0);
                        let cc = pos2(art_r.max.x - rad - b.px(2.0), art_r.center().y);
                        fit::ring(&p, cc, rad, (rad * 0.24).max(3.0), kit::grow(ui.ctx(), resp.id.with("ring"), f), if f >= 1.0 { theme::GREEN } else { col });
                        p.text(cc, Align2::CENTER_CENTER, format!("{:.0}%", f * 100.0), b.bold((rad / b.s() * 0.42).clamp(9.0, 15.0)), TEXT());
                    }
                    2 => {
                        let slot = art_r.width() / 7.0;
                        let rad = (slot * 0.32).min(art_r.height() * 0.22).max(2.5);
                        let with_days = art_r.height() > rad * 2.0 + b.px(16.0);
                        let cy = if with_days { art_r.center().y - b.px(6.0) } else { art_r.center().y };
                        for (k, v) in week.iter().enumerate() {
                            let cc = pos2(art_r.min.x + slot * (k as f32 + 0.5), cy);
                            if *v > 0 {
                                p.circle_filled(cc, rad, col);
                            } else {
                                p.circle_stroke(cc, rad - 0.5, egui::Stroke::new(1.5f32, theme::blend(CARD(), TEXT(), 0.18)));
                            }
                            if with_days {
                                let wd = crate::stats::WEEKDAYS[crate::stats::weekday(today_n - 6 + k as i64)];
                                p.text(cc + vec2(0.0, rad + b.px(9.0)), Align2::CENTER_CENTER, &wd[..1], b.font(9.0), if k == 6 { TEXT() } else { MUTED() });
                            }
                        }
                    }
                    _ => {
                        let f = (into as f32 / need.max(1) as f32).clamp(0.0, 1.0);
                        let bar = Rect::from_center_size(pos2(art_r.center().x, art_r.center().y + b.px(6.0)), vec2(art_r.width(), b.px(8.0)));
                        kit::paint_bar(&p, bar, kit::grow(ui.ctx(), resp.id.with("xp"), f));
                        fit::label(&p, Rect::from_min_max(pos2(art_r.min.x, bar.min.y - b.px(18.0)), pos2(art_r.max.x, bar.min.y - b.px(3.0))), &format!("{} / {} XP", theme::thousands(into), theme::thousands(need)), b.font(10.0), MUTED(), Align2::RIGHT_CENTER);
                    }
                }
            }
            let tip = ["Your current hunt's stats", "Your goals", "Your goals and streaks", "Your Trainer Profile"][i];
            if resp.on_hover_text(tip).clicked() {
                open = Some(i);
            }
        }
        if let Some(i) = open {
            self.setup.scroll_top = true;
            match i {
                0 => {
                    self.go_to(Tab::Runs);
                    self.setup.view_run = Some(super::RunRef::Current);
                    self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
                }
                1 | 2 => {
                    self.go_to(Tab::Profile);
                    self.setup.profile_section = super::trainer::ProfileSection::Goals;
                }
                _ => {
                    self.go_to(Tab::Profile);
                    self.setup.profile_section = super::trainer::ProfileSection::Overview;
                }
            }
        }
    }

    // ── Clock ────────────────────────────────────────────────────────────

    /// The time, big, with the date; with room, how long the app's been
    /// open and how long you've hunted today.
    pub(super) fn f_clock(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "clock");
        let now = crate::config::now_unix();
        let local = now as i64 + crate::stats::utc_offset_secs();
        let secs = local.rem_euclid(86_400);
        let (h, m) = (secs / 3600, (secs / 60) % 60);
        let (h12, ampm) = (if h % 12 == 0 { 12 } else { h % 12 }, if h < 12 { "am" } else { "pm" });
        let day = crate::stats::local_day(now);
        let date = format!("{}, {}", crate::stats::WEEKDAYS[crate::stats::weekday(day)], crate::stats::fmt_day(day));
        let stats = [("App open", fmt_duration(self.started.elapsed().as_secs())), ("Hunted today", fmt_duration(self.today_hunt_secs()))];
        let p = ui.painter_at(b.r);
        let inner = b.inner();
        let gap = b.px(10.0);
        // where the time goes, and where the two numbers go
        let (time_r, stat_r) = match b.shape() {
            Shape::Mini => (inner, Rect::NOTHING),
            _ if b.flat() && b.cols < 4 => (inner, Rect::NOTHING),
            Shape::Strip => {
                let (l, r) = fit::split_right(inner, inner.width() * 0.42, gap);
                (l, r)
            }
            _ => fit::split_bottom(inner, (inner.height() * 0.38).max(b.px(44.0)), gap),
        };
        // the time and the date under it, as one group, the time as big as
        // the room allows
        let fl = fit::vflow(time_r, &[(b.px(28.0), time_r.height()), (b.px(15.0), b.px(17.0))], b.px(4.0));
        let (clock_r, date_r) = (fl[0], fl[1]);
        let (num_r, _) = fit::split_right(clock_r, b.px(28.0), b.px(2.0));
        let at = fit::hero(&p, num_r, &format!("{h12}:{m:02}"), num_r.height() * 1.05, TEXT(), Align2::LEFT_CENTER);
        p.text(pos2(at.max.x + b.px(4.0), at.max.y - at.height() * 0.2), Align2::LEFT_BOTTOM, ampm, b.font((at.height() / b.s() * 0.25).clamp(11.0, 22.0)), MUTED());
        fit::label(&p, date_r, &date, b.font(12.0), MUTED(), Align2::LEFT_CENTER);
        // the two numbers, side by side when there's width, else stacked
        if stat_r.is_positive() {
            let parts = if stat_r.width() > stat_r.height() * 2.2 && b.shape() != Shape::Strip { fit::cols(stat_r, 2, gap) } else { fit::stack(stat_r, 2, b.px(6.0)) };
            for (pr, (label, v)) in parts.iter().zip(stats.iter()) {
                fit::tile(&p, *pr, kit::C_CYAN, b.px(10.0));
                let ir = pr.shrink2(vec2(b.px(10.0), b.px(4.0)));
                if ir.height() > b.px(34.0) {
                    let (t, bo) = fit::split_top(ir, ir.height() * 0.4, 0.0);
                    fit::label(&p, t, label, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
                    fit::hero(&p, bo, v, b.px(18.0), TEXT(), Align2::LEFT_CENTER);
                } else {
                    fit::label(&p, ir, label, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
                    fit::label(&p, ir, v, b.bold(12.5), TEXT(), Align2::RIGHT_CENTER);
                }
            }
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
    }

    // ── This week ────────────────────────────────────────────────────────

    /// The last seven days as bars, with the week's total; the bars take
    /// whatever room is left.
    pub(super) fn f_week(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "week");
        let days = self.home_days();
        let today = crate::stats::local_day(crate::config::now_unix());
        let week: Vec<(i64, u64)> = (0..7).map(|k| (today - 6 + k, days.get(&(today - 6 + k)).copied().unwrap_or(0))).collect();
        let vals: Vec<u64> = week.iter().map(|d| d.1).collect();
        let total: u64 = vals.iter().sum();
        let best_k = vals.iter().enumerate().max_by_key(|(_, v)| **v).map(|(k, _)| k).filter(|k| vals[*k] > 0);
        let best_ever = days.iter().max_by_key(|(_, v)| **v).map(|(d, v)| (*d, *v)).filter(|b| b.1 > 0);
        let labels: Vec<String> = week.iter().map(|(d, _)| crate::stats::WEEKDAYS[crate::stats::weekday(*d)][..1].to_string()).collect();
        let small = matches!(b.shape(), Shape::Mini) || b.flat();
        let (body, go) = if small { (b.inner(), false) } else { fit::head(ui, &b, "📊", kit::C_BLUE, "This week", Some("Stats ›")) };
        let p = ui.painter_at(b.r);
        let gap = b.px(10.0);
        let total_txt = theme::thousands(total);
        match b.shape() {
            Shape::Mini => {
                let (top, chart) = fit::split_top(body, body.height() * 0.48, b.px(4.0));
                let (num, lab) = fit::split_bottom(top, b.px(14.0), 0.0);
                fit::hero(&p, num, &total_txt, b.px(30.0), TEXT(), Align2::LEFT_BOTTOM);
                fit::label(&p, lab, "encounters this week", b.font(10.5), MUTED(), Align2::LEFT_CENTER);
                fit::bars(&p, chart, &vals, best_k, kit::C_BLUE, None, b.font(9.0));
            }
            _ if b.flat() || b.shape() == Shape::Strip => {
                let (left, chart) = fit::split_left(body, (body.width() * 0.3).clamp(b.px(90.0), b.px(220.0)), gap);
                let parts = if left.height() > b.px(70.0) { 3 } else { 2 };
                let st = fit::stack(left, parts, 0.0);
                fit::hero(&p, st[0], &total_txt, b.px(32.0), TEXT(), Align2::LEFT_BOTTOM);
                fit::label(&p, st[1], "encounters in 7 days", b.font(11.0), MUTED(), Align2::LEFT_TOP);
                if parts == 3 {
                    if let Some((d, v)) = best_ever {
                        fit::label(&p, st[2], &format!("Best day: {} on {}", theme::thousands(v), crate::stats::fmt_day_short(d)), b.font(10.5), MUTED(), Align2::LEFT_CENTER);
                    }
                }
                fit::bars(&p, chart, &vals, best_k, kit::C_BLUE, Some(&labels), b.font(10.0));
            }
            _ => {
                let (top, rest) = fit::split_top(body, (body.height() * 0.2).clamp(b.px(30.0), b.px(64.0)), b.px(8.0));
                let at = fit::hero(&p, Rect::from_min_max(top.min, pos2(top.max.x - b.px(120.0), top.max.y)), &total_txt, top.height(), TEXT(), Align2::LEFT_CENTER);
                fit::label(&p, Rect::from_min_max(pos2(at.max.x + b.px(8.0), top.min.y), pos2(top.max.x, at.max.y - at.height() * 0.12)), "encounters in 7 days", b.font(11.5), MUTED(), Align2::LEFT_BOTTOM);
                let (chart, foot) = if best_ever.is_some() && rest.height() > b.px(110.0) { fit::split_bottom(rest, b.px(15.0), b.px(6.0)) } else { (rest, Rect::NOTHING) };
                fit::bars(&p, chart, &vals, best_k, kit::C_BLUE, Some(&labels), b.font(10.5));
                if let (Some((d, v)), true) = (best_ever, foot.is_positive()) {
                    fit::label(&p, foot, &format!("Best day ever: {} on {}", theme::thousands(v), crate::stats::fmt_day_short(d)), b.font(11.0), MUTED(), Align2::LEFT_CENTER);
                }
            }
        }
        // the whole thing opens Stats when there's no heading
        let mut go = go;
        if small && ui.interact(b.r, ui.id().with(("week-open", b.r.min.x as i32, b.r.min.y as i32)), egui::Sense::click()).on_hover_text("Open Stats").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            go = true;
        }
        if go {
            self.go_to(Tab::Stats);
        }
    }

    // ── Current hunt ─────────────────────────────────────────────────────

    /// The hunt you're on: its Pokémon, count and odds, and Start overlay.
    pub(super) fn f_hunt(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "hunt_ctrl");
        let (count, _) = self.odds_count();
        let odds = self.cfg.odds_one_in();
        let method = self.cfg.odds_caption();
        let name = self.target_label();
        let secs = self.cfg.run_stats.hunt_secs;
        let enc = theme::thousands(self.cfg.count);
        let (mult, odds_txt, odds_col) = match odds {
            Some(o) => {
                let m = count as f64 / o;
                let col = super::charts::luck_col(found_by(count, o));
                (m as f32, format!("{m:.2}× the odds (1 in {})", super::fmt_one_in(o)), super::charts::ink(col))
            }
            None => (0.0, String::new(), MUTED()),
        };
        let gap = b.px(10.0);
        let mut open = false;
        let mut change = false;
        let id = (b.r.min.x as i32, b.r.min.y as i32);
        let p = ui.painter_at(b.r);
        match b.shape() {
            Shape::Mini => {
                let inner = b.inner();
                let (sprite_r, rest) = fit::split_top(inner, inner.height() * 0.58, b.px(4.0));
                self.hunt_sprite(ui, &p, sprite_r, &b);
                let (num, lab) = fit::split_bottom(rest, b.px(13.0), 0.0);
                fit::hero(&p, num, &enc, b.px(26.0), TEXT(), Align2::CENTER_CENTER);
                fit::label(&p, lab, &name, b.font(10.5), MUTED(), Align2::CENTER_CENTER);
                open = ui.interact(b.r, ui.id().with(("hunt-open", id)), egui::Sense::click()).on_hover_text("Open the Shiny Hunter").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
            }
            _ if b.flat() => {
                let inner = b.inner();
                let (sprite_r, rest) = fit::split_left(inner, inner.height(), gap);
                self.hunt_sprite(ui, &p, sprite_r, &b);
                let (rest, btn_r) = if b.cols >= 6 { fit::split_right(rest, b.px(150.0), gap) } else { (rest, Rect::NOTHING) };
                let (text_r, odds_r) = if b.cols >= 4 && odds.is_some() { fit::split_right(rest, rest.width() * 0.42, gap) } else { (rest, Rect::NOTHING) };
                let st = fit::stack(text_r, 2, 0.0);
                fit::label(&p, st[0], &name, b.bold(13.0), TEXT(), Align2::LEFT_BOTTOM);
                let at = fit::hero(&p, st[1], &enc, b.px(22.0), TEXT(), Align2::LEFT_TOP);
                p.text(pos2(at.max.x + b.px(6.0), at.center().y), Align2::LEFT_CENTER, "encounters", b.font(11.0), MUTED());
                if odds_r.is_positive() {
                    let (mr, tr) = fit::split_top(Rect::from_center_size(odds_r.center(), vec2(odds_r.width(), b.px(30.0))), b.px(7.0), b.px(6.0));
                    fit::meter(&p, mr, mult.min(1.0), odds_col);
                    fit::label(&p, tr, &odds_txt, b.font(10.5), odds_col, Align2::LEFT_TOP);
                }
                if btn_r.is_positive() {
                    self.overlay_pill(ui, Rect::from_center_size(btn_r.center(), vec2(btn_r.width(), b.px(32.0).min(btn_r.height()))), ("hunt-ov", id));
                }
                if ui.interact(sprite_r, ui.id().with(("hunt-open", id)), egui::Sense::click()).on_hover_text("Open the Shiny Hunter").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = true;
                }
            }
            shape => {
                let (body, link) = fit::head(ui, &b, "🎯", kit::C_VIOLET, "Current hunt", Some("Open ›"));
                open = link;
                let tall = shape == Shape::Column;
                // the buttons at the bottom (stacked in a column, side by side otherwise)
                let btn_h = b.px(32.0);
                let want_btns = body.height() > b.px(150.0) || (shape == Shape::Strip && b.cols >= 6);
                let (body, btns) = if want_btns && shape != Shape::Strip {
                    let h = if tall { btn_h * 2.0 + b.px(6.0) } else { btn_h };
                    let (a, bb) = fit::split_bottom(body, h, gap);
                    (a, bb)
                } else if want_btns {
                    let (a, bb) = fit::split_right(body, b.px(150.0), gap);
                    (a, bb)
                } else {
                    (body, Rect::NOTHING)
                };
                // the sprite: on top in a column, else on the left
                // (squarish room: the sprite on top too, so neither side is
                // left empty)
                let stacked = tall || body.width() < body.height() * 1.45;
                let (sprite_r, info) = if stacked {
                    fit::split_top(body, (body.width() * 0.8).min(body.height() * 0.46), gap)
                } else {
                    let side = body.height().min(body.width() * if shape == Shape::Big { 0.5 } else { 0.42 });
                    fit::split_left(body, side, gap)
                };
                self.hunt_sprite(ui, &p, sprite_r, &b);
                // name, how, count, time and the odds bar as one group,
                // centred beside (or under) the sprite; the count takes the
                // room that's left, and what doesn't fit is left out
                let line = b.px(16.0);
                let mut want: Vec<(&str, f32, f32)> = vec![("name", b.px(20.0), b.px(26.0)), ("count", b.px(26.0), b.px(72.0))];
                let room = info.height();
                let mut used: f32 = want.iter().map(|w| w.1).sum::<f32>() + b.px(4.0);
                for (k, h) in [("odds", b.px(28.0)), ("how", line), ("time", line)] {
                    if (k != "odds" || odds.is_some()) && used + h + b.px(4.0) <= room {
                        used += h + b.px(4.0);
                        want.push((k, h, h));
                    }
                }
                let order = ["name", "how", "count", "time", "odds"];
                want.sort_by_key(|w| order.iter().position(|o| *o == w.0).unwrap_or(9));
                let fl = fit::vflow(info, &want.iter().map(|w| (w.1, w.2)).collect::<Vec<_>>(), b.px(4.0));
                for (w, r) in want.iter().zip(fl.iter()) {
                    match w.0 {
                        "name" => {
                            fit::label(&p, *r, &name, b.bold(16.0), TEXT(), Align2::LEFT_CENTER);
                        }
                        "how" => {
                            fit::label(&p, *r, &method, b.font(11.5), MUTED(), Align2::LEFT_CENTER);
                        }
                        "count" => {
                            let at = fit::hero(&p, Rect::from_min_max(r.min, pos2(r.max.x - b.px(80.0), r.max.y)), &enc, r.height() * 1.05, TEXT(), Align2::LEFT_CENTER);
                            p.text(pos2(at.max.x + b.px(6.0), at.max.y - at.height() * 0.18), Align2::LEFT_BOTTOM, "encounters", b.font(11.5), MUTED());
                        }
                        "time" => {
                            fit::label(&p, *r, &format!("{} on this hunt · {} today", fmt_duration(secs), fmt_duration(self.today_hunt_secs())), b.font(11.0), MUTED(), Align2::LEFT_CENTER);
                        }
                        _ => {
                            let (mr, tr) = fit::split_top(*r, b.px(7.0), b.px(5.0));
                            fit::meter(&p, mr, mult.min(1.0), odds_col);
                            fit::label(&p, tr, &odds_txt, b.font(11.0), odds_col, Align2::LEFT_TOP);
                        }
                    }
                }
                if btns.is_positive() {
                    let parts = if tall || shape == Shape::Strip { fit::stack(Rect::from_center_size(btns.center(), vec2(btns.width(), (btn_h * 2.0 + b.px(6.0)).min(btns.height()))), 2, b.px(6.0)) } else { fit::cols(btns, 2, b.px(8.0)) };
                    self.overlay_pill(ui, parts[0], ("hunt-ov", id));
                    if fit::button(ui, parts[1], "🎯  Change target", false, ("hunt-change", id)) {
                        change = true;
                    }
                }
            }
        }
        if change {
            self.open_target_picker();
        }
        if open {
            self.go_to(Tab::Shiny);
            self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
        }
    }

    // ── Recent shinies ───────────────────────────────────────────────────

    /// Your latest shinies, as many as the box holds (one, big, in a small
    /// square); click one for its trophy.
    pub(super) fn f_shinies(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "shinies");
        let recent: Vec<(Option<usize>, String, u64, u64, Option<String>)> = self
            .cfg
            .history
            .iter()
            .rev()
            .take(32)
            .map(|h| (h.slug.as_deref().and_then(|s| self.species.iter().position(|sp| sp.slug == s)), h.target.clone(), h.encounters, h.when_unix, h.slug.clone()))
            .collect();
        let mini = b.shape() == Shape::Mini;
        let (body, dex) = if mini { (b.inner(), false) } else { fit::head(ui, &b, "✨", kit::C_GOLD, "Recent shinies", Some("Shiny Dex ›")) };
        let mut trophy: Option<Option<String>> = None;
        let id = (b.r.min.x as i32, b.r.min.y as i32);
        if recent.is_empty() {
            let p = ui.painter_at(b.r);
            fit::empty(&p, &b, body, "✨", "No shinies yet. Your first one shows up here, with how long it took.");
        } else {
            let gap = b.px(8.0);
            let cells = if mini { vec![body] } else { fit::tiles(body, recent.len(), 0.85, b.px(64.0), b.px(64.0), gap) };
            for (k, r) in cells.iter().enumerate() {
                let (idx, name, enc, when, slug) = &recent[k];
                let resp = fit::hit(ui, *r, ("shiny-tile", k, id), b.px(12.0));
                let p = ui.painter_at(*r);
                fit::tile(&p, *r, GOLD, b.px(12.0));
                let with_name = r.height() >= b.px(88.0);
                let with_enc = r.height() >= b.px(108.0);
                let text_h = if with_enc { b.px(32.0) } else if with_name { b.px(17.0) } else { 0.0 };
                let (sprite_r, text_r) = fit::split_bottom(r.shrink(b.px(6.0)), text_h, 0.0);
                if let Some(i) = idx {
                    let sp = self.species[*i].clone();
                    let side = sprite_r.width().min(sprite_r.height());
                    self.draw_sprite_centered(&p, &sp, true, sprite_r.center(), vec2(side, side));
                } else {
                    p.text(sprite_r.center(), Align2::CENTER_CENTER, "?", b.bold(18.0), MUTED());
                }
                theme::sparkle(&p, r.right_top() + vec2(-b.px(10.0), b.px(10.0)), b.px(4.0), GOLD);
                if with_name {
                    let (n_r, e_r) = if with_enc { fit::split_top(text_r, text_h * 0.55, 0.0) } else { (text_r, Rect::NOTHING) };
                    fit::label(&p, n_r, name, b.bold(12.0), TEXT(), Align2::CENTER_CENTER);
                    if with_enc {
                        fit::label(&p, e_r, &format!("{} enc.", theme::thousands(*enc)), b.font(10.5), MUTED(), Align2::CENTER_CENTER);
                    }
                }
                if resp.on_hover_text(format!("{name}: {} encounters · {}\nClick for its trophy", theme::thousands(*enc), ago(*when))).clicked() {
                    trophy = Some(slug.clone());
                }
            }
        }
        if dex || trophy.is_some() {
            self.setup.tab = Tab::Pokedex;
            self.setup.pokedex.shiny = true;
            self.setup.pokedex.open = None;
            self.setup.dex.open = trophy.flatten();
            self.setup.trophy_back = self.setup.dex.open.as_ref().map(|_| Tab::Home);
            self.setup.scroll_top = true;
        }
    }

    // ── Rare grass today ─────────────────────────────────────────────────

    /// Today's rare grass: as many Pokémon (rarest first) as the box holds,
    /// each with where it is; the rest are a click away on the Map.
    pub(super) fn f_rare(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "rare");
        let ready = self.game_map().is_some();
        let spots = self.rare_spots_today();
        let n = spots.len();
        let id = (b.r.min.x as i32, b.r.min.y as i32);
        let link = if n > 0 { format!("{n} on the Map ›") } else { "Open the Map ›".to_string() };
        let mini = b.shape() == Shape::Mini;
        let (body, mut to_map) = if mini { (b.inner(), false) } else { fit::head(ui, &b, "🌿", kit::C_GREEN, "Rare grass today", Some(&link)) };
        let mut go: Option<super::rare::RareSpot> = None;
        let mut open: Option<String> = None;
        let p = ui.painter_at(b.r);
        if !ready {
            fit::empty(&p, &b, body, "🗺", "Today's rare grass shows here once the Map has read your game's map files.");
        } else if spots.is_empty() {
            fit::empty(&p, &b, body, "🌿", "Nothing on rare grass yet today. Use Sweet Scent on a rare grass patch and it shows up here.");
        } else if mini {
            // how many, and the rarest few
            let (top, row) = fit::split_top(body, body.height() * 0.5, b.px(4.0));
            let (num, lab) = fit::split_bottom(top, b.px(13.0), 0.0);
            fit::hero(&p, num, &n.to_string(), b.px(30.0), TEXT(), Align2::LEFT_BOTTOM);
            fit::label(&p, lab, "on rare grass today", b.font(10.5), MUTED(), Align2::LEFT_CENTER);
            for (k, r) in fit::cols(row, 3.min(n), b.px(4.0)).iter().enumerate() {
                let side = r.width().min(r.height());
                self.draw_sprite(&p, &spots[k].species, false, pos2(r.center().x, r.center().y + side / 2.0), vec2(side, side));
            }
            to_map = ui.interact(b.r, ui.id().with(("rare-open", id)), egui::Sense::click()).on_hover_text("Open the Map").on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
        } else {
            let gap = b.px(8.0);
            let compact = body.height() < b.px(70.0);
            let (min_w, min_h) = if compact { (b.px(110.0), b.px(44.0)) } else { (b.px(150.0), b.px(56.0)) };
            let mut cells = fit::tiles(body, n, 2.6, min_w, min_h, gap);
            let more = n.saturating_sub(cells.len());
            // the last tile says how many more there are
            let more_r = if more > 0 { cells.pop() } else { None };
            for (k, r) in cells.iter().enumerate() {
                let s = &spots[k];
                let resp = fit::hit(ui, *r, ("rare-tile", k, id), b.px(10.0));
                let tp = ui.painter_at(*r);
                fit::tile(&tp, *r, kit::C_GREEN, b.px(10.0));
                let ir = r.shrink(b.px(6.0));
                let (sp_r, txt) = fit::split_left(ir, ir.height().min(ir.width() * 0.4), b.px(8.0));
                let side = sp_r.width().min(sp_r.height());
                tp.circle_filled(sp_r.center(), side / 2.0, egui::Color32::from_black_alpha(45));
                self.draw_sprite(&tp, &s.species, false, pos2(sp_r.center().x, sp_r.center().y + side * 0.46), vec2(side * 0.95, side * 0.95));
                let lines: Vec<(String, egui::FontId, egui::Color32)> = if txt.height() > b.px(48.0) {
                    vec![
                        (s.species.name.to_string(), b.bold(12.5), TEXT()),
                        (s.place.clone(), b.font(11.0), MUTED()),
                        (if s.tiles > 1 { format!("{} spots", s.tiles) } else { "1 spot".to_string() }, b.font(10.5), GOLD),
                    ]
                } else {
                    vec![(s.species.name.to_string(), b.bold(12.0), TEXT()), (s.place.clone(), b.font(10.5), MUTED())]
                };
                let lh = txt.height() / lines.len() as f32;
                let used = (lines.len() as f32 * b.px(16.0)).min(txt.height());
                let y0 = txt.center().y - used / 2.0;
                for (j, (t, f, c)) in lines.iter().enumerate() {
                    let row = Rect::from_min_size(pos2(txt.min.x, y0 + j as f32 * used / lines.len() as f32), vec2(txt.width(), lh.min(b.px(16.0))));
                    fit::label(&tp, row, t, f.clone(), *c, Align2::LEFT_CENTER);
                }
                let sp_resp = ui.interact(sp_r, ui.id().with(("rare-sprite", k, id)), egui::Sense::click());
                if sp_resp.on_hover_text("Open in the Pokédex").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = s.wiki_id.clone();
                } else if resp.on_hover_text(format!("Show {} on the Map", s.place)).clicked() {
                    go = Some(s.clone());
                }
            }
            if let Some(r) = more_r {
                let resp = fit::hit(ui, r, ("rare-more", id), b.px(10.0));
                let tp = ui.painter_at(r);
                fit::tile(&tp, r, kit::C_GREEN, b.px(10.0));
                fit::label(&tp, r.shrink(b.px(6.0)), &format!("+{} more on the Map ›", more + 1), b.bold(12.0), MUTED(), Align2::CENTER_CENTER);
                if resp.clicked() {
                    to_map = true;
                }
            }
        }
        if let Some(id) = open {
            self.open_dex_entry(&id);
        } else if let Some(s) = go {
            self.go_to_rare(&s);
        } else if to_map {
            self.go_to(Tab::Map);
        }
    }

    // ── Almost there ─────────────────────────────────────────────────────

    /// The achievements you're closest to, as many as fit.
    pub(super) fn f_almost(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "almost");
        let l = self.lifetime_full();
        let near = self.near_achievements(&l);
        let rows: Vec<Row> = near
            .iter()
            .map(|(a, f)| {
                let (c, t) = crate::progress::progress(a, &l);
                let fmt = |v: f64| if matches!(a.goal, crate::progress::Goal::HuntHours(_)) { format!("{v:.1}h") } else { theme::thousands(v as u64) };
                let tc = super::profile_page::tier_col(a.tier);
                Row::new(a.name).lead(Lead::Slug(crate::progress::mascot(a.id).to_string())).sub(format!("{} · {} / {}", a.desc, fmt(c.min(t)), fmt(t))).right(format!("{:.0}%", f * 100.0), Some(super::charts::ink(tc))).bar(*f as f32, tc)
            })
            .collect();
        let c = self.f_list(ui, &b, Head { icon: "🎯", col: kit::C_PINK, title: "Almost there", link: Some("All ›") }, &rows, "Start hunting and your next achievements show up here.");
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::Profile);
            self.setup.profile_section = super::trainer::ProfileSection::Achievements;
        }
    }

    // ── Jump to ──────────────────────────────────────────────────────────

    /// Every app a click away: tiles that fill the box (icon only when
    /// they're small, the name and what it's for when there's room).
    pub(super) fn f_jump(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "jump");
        let items: [(Tab, &str); 9] = [
            (Tab::Pokedex, "Every Pokémon, and your Shiny Dex"),
            (Tab::Map, "Areas, Pokémon and rare grass"),
            (Tab::Info, "Everything from the game's wiki"),
            (Tab::Team, "Build teams, check what they cover"),
            (Tab::Sim, "Battle wild Pokémon and trainers"),
            (Tab::Breed, "Plan Eggs, stats and egg moves"),
            (Tab::Calc, "Damage maths, any set against any"),
            (Tab::Stats, "Your hunting over time"),
            (Tab::Runs, "Every hunt, saved and caught"),
        ];
        let items: Vec<(Tab, &str)> = items.into_iter().filter(|(t, _)| super::shell::app_of(*t).map_or(true, |a| self.has_app(a))).collect();
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, _) = if small { (b.inner(), false) } else { fit::head(ui, &b, "📌", kit::C_CYAN, "Jump to", None) };
        let mut go: Option<Tab> = None;
        let cells = fit::tiles(body, items.len(), 3.0, b.px(44.0), b.px(40.0), b.px(8.0));
        for (k, r) in cells.iter().enumerate() {
            let (tab, about) = items[k];
            let (icon, name, color) = super::shell::tab_meta(tab);
            let resp = ui.interact(*r, ui.id().with(("jump", k, b.r.min.x as i32, b.r.min.y as i32)), egui::Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let p = ui.painter_at(*r);
            p.rect_filled(*r, b.px(12.0), theme::blend(kit::CARD_HOVER(), color, 0.06 + 0.06 * ht));
            p.rect_stroke(*r, b.px(12.0), egui::Stroke::new(1.0f32, theme::blend(kit::LINE(), color, 0.15 + 0.4 * ht)));
            let ir = r.shrink(b.px(8.0));
            let chip_side = ir.height().min(b.px(34.0)).min(ir.width());
            if r.width() < b.px(110.0) {
                // just the icon
                kit::icon_chip(&p, Rect::from_center_size(ir.center(), vec2(chip_side, chip_side)), icon, color, true);
            } else {
                let (chip_r, words) = fit::split_left(ir, chip_side, b.px(10.0));
                kit::icon_chip(&p, Rect::from_center_size(chip_r.center(), vec2(chip_side, chip_side)), icon, color, true);
                let with_about = words.height() > b.px(32.0) && words.width() > b.px(90.0);
                let fl = fit::vflow(words, &if with_about { vec![(b.px(16.0), b.px(18.0)), (b.px(13.0), b.px(15.0))] } else { vec![(b.px(16.0), b.px(18.0))] }, b.px(2.0));
                fit::label(&p, fl[0], name, b.bold(13.0), TEXT(), Align2::LEFT_CENTER);
                if with_about {
                    fit::label(&p, fl[1], about, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
                }
            }
            if resp.on_hover_text(format!("{name}: {about}")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                go = Some(tab);
            }
        }
        if let Some(t) = go {
            if t == Tab::Runs {
                self.setup.nav_collapsed.retain(|x| *x != Tab::Shiny);
            }
            self.go_to(t);
        }
    }

    // ── Pick up a hunt ───────────────────────────────────────────────────

    /// Your other hunts, most recent first (click one to switch to it), or,
    /// with none to pick up, the hunts you've finished.
    pub(super) fn f_hunts(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "hunts");
        let mut runs: Vec<(u64, u64)> = self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id).map(|r| (r.id, r.stats.last_unix.max(r.saved_unix))).collect();
        runs.sort_by(|a, b| b.1.cmp(&a.1));
        let mut done: Vec<usize> = (0..self.cfg.history.len()).collect();
        done.sort_by(|a, b| self.cfg.history[*b].when_unix.cmp(&self.cfg.history[*a].when_unix));
        let finished = runs.is_empty() && !done.is_empty();
        let mut rows: Vec<Row> = Vec::new();
        let mut refs: Vec<super::RunRef> = Vec::new();
        let list: Vec<(super::RunRef, Option<u64>)> = if finished { done.iter().map(|i| (super::RunRef::Finished(*i), None)).collect() } else { runs.iter().map(|(id, last)| (super::RunRef::Saved(*id), Some(*last))).collect() };
        for (rr, last) in list {
            let Some(info) = self.run_info(rr) else { continue };
            let odds = info.odds.filter(|o| *o > 1.0);
            let col = odds.map_or(MUTED(), |o| super::charts::luck_col(found_by(info.count, o)));
            let mult = odds.map(|o| info.count as f64 / o);
            let when = match last {
                Some(t) => ago(t),
                None => format!("found {}", info.ended.map(ago).unwrap_or_default()),
            };
            let lead = self.species_of(info.slug.as_deref()).map_or(Lead::None, |(i, _)| Lead::Species(i, finished));
            rows.push(
                Row::new(info.name.clone())
                    .lead(lead)
                    .sub(format!("{} · {when}{}", info.method, mult.map_or(String::new(), |m| format!(" · {m:.2}× the odds"))))
                    .right(theme::thousands(info.count), Some(if mult.is_some() { super::charts::ink(col) } else { TEXT() }))
                    .tip(if finished { "Open this hunt's stats" } else { "Switch to this hunt (the one you're on is saved)" }),
            );
            refs.push(rr);
        }
        let (title, icon) = if finished { ("Completed hunts", "🏆") } else { ("Pick up a hunt", "📜") };
        let link = format!("All runs ({}) ›", if finished { done.len() } else { runs.len() });
        let c = self.f_list(ui, &b, Head { icon, col: kit::C_GREEN, title, link: Some(&link) }, &rows, "No saved hunts yet. When you change target, the hunt you were on is saved here so you can pick it up again.");
        if let Some(k) = c.item {
            match refs.get(k) {
                Some(super::RunRef::Saved(id)) => self.resume_run(*id),
                Some(super::RunRef::Finished(i)) => {
                    self.setup.scroll_top = true;
                    self.go_to(Tab::Runs);
                    self.setup.view_run = Some(super::RunRef::Finished(*i));
                    self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
                }
                _ => {}
            }
        } else if c.link || c.more {
            self.go_to(Tab::Runs);
            self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
        }
    }

    // ── Breeding ─────────────────────────────────────────────────────────

    /// What's going on in the Breeding Planner: the plan and the Day Care.
    pub(super) fn f_breeding(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "breeding");
        let mut rows: Vec<Row> = Vec::new();
        if let Some(w) = self.wiki.clone() {
            let f = self.setup.breed.file().clone();
            let name = |id: &str| w.mon(id).map(|m| m.name.clone()).unwrap_or_else(|| id.to_string());
            if !f.target.species.is_empty() {
                let t = &f.target;
                let stats = t.ivs.iter().filter(|x| **x).count();
                let mut what = vec![];
                if let Some(n) = &t.nature {
                    what.push(n.clone());
                }
                if stats > 0 {
                    what.push(format!("{stats} at 31"));
                }
                if t.shiny {
                    what.push("shiny".into());
                }
                rows.push(Row::new(format!("Planning a {}", name(&t.species))).lead(Lead::Wiki(t.species.clone(), t.shiny)).sub(if what.is_empty() { "Your target".to_string() } else { what.join(", ") }));
            }
            if !f.daycare.a.species.is_empty() && !f.daycare.b.species.is_empty() {
                let fam = crate::breed::family_pair(&w, &f.daycare.a, &f.daycare.b);
                let mine: Vec<&crate::breed::Hatch> = f.log.iter().filter(|h| h.pair == fam).collect();
                let goal = if f.daycare.goal_set { f.daycare.goal.clone() } else { crate::breed::Goal::from_parents(&f.daycare.a, &f.daycare.b) };
                let hits = mine.iter().filter(|h| h.hits(&goal)).count();
                let shinies = mine.iter().filter(|h| h.shiny).count();
                let mut sub = format!("{} Egg{} logged", mine.len(), if mine.len() == 1 { "" } else { "s" });
                if hits > 0 {
                    sub.push_str(&format!(", {hits} the one you want"));
                }
                if shinies > 0 {
                    sub.push_str(&format!(", {shinies} shiny"));
                }
                rows.push(Row::new(format!("Day Care: {} × {}", name(&f.daycare.a.species), name(&f.daycare.b.species))).lead(Lead::Wiki(f.daycare.a.species.clone(), false)).sub(sub));
            }
        }
        let c = self.f_list(ui, &b, Head { icon: "🐣", col: kit::C_PINK, title: "Breeding", link: Some("Open ›") }, &rows, "Put two Pokémon in the Day Care in Breeding and it shows here.");
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::Breed);
        }
    }
}
