//! Stats › Month in review: one month as a big calendar (each day's
//! encounters, its shiny stars and the hunt you worked on). Click a day to
//! see it on the right; the month's totals sit under it. ‹ › go back
//! through the months, and Share makes the month's image.

use super::charts as ch;
use super::stat_share::{month_days, month_name, ShareWhat};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::App;
use crate::stats::{self, fmt_duration, local_day, now_unix};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use std::collections::BTreeMap;

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Which month is shown, and the day picked in it.
#[derive(Default)]
pub(super) struct MonthState {
    /// (year, month 1..12); None = this month.
    pub ym: Option<(i64, u32)>,
    pub picked: Option<i64>,
}

/// One day of the month.
#[derive(Default, Clone)]
struct Day {
    enc: u64,
    secs: u64,
    sessions: u32,
    /// Hunts that day: (name, encounters), most first.
    hunts: Vec<(String, u64)>,
    /// Shinies caught that day (phases marked).
    shinies: Vec<String>,
}

fn this_month() -> (i64, u32) {
    let (y, m, _) = stats::civil_from_days(local_day(now_unix()));
    (y, m)
}

fn prev_month((y, m): (i64, u32)) -> (i64, u32) {
    if m == 1 {
        (y - 1, 12)
    } else {
        (y, m - 1)
    }
}

fn next_month((y, m): (i64, u32)) -> (i64, u32) {
    if m == 12 {
        (y + 1, 1)
    } else {
        (y, m + 1)
    }
}

impl App {
    /// The month shown (this month unless one was picked).
    pub(super) fn month_shown(&self) -> (i64, u32) {
        self.setup.stats_month.ym.unwrap_or_else(this_month)
    }

    /// The first month anything was hunted in.
    fn first_month(&self) -> (i64, u32) {
        let mut first: Option<i64> = None;
        for (_, st) in self.month_runs() {
            for d in st.days.iter().filter(|d| d.encounters > 0) {
                first = Some(first.map_or(d.day, |f: i64| f.min(d.day)));
            }
        }
        for h in &self.cfg.history {
            let d = local_day(h.when_unix);
            first = Some(first.map_or(d, |f: i64| f.min(d)));
        }
        let (y, m, _) = stats::civil_from_days(first.unwrap_or_else(|| local_day(now_unix())));
        (y, m)
    }

    /// Every hunt with its stats, named.
    fn month_runs(&self) -> Vec<(String, &stats::RunStats)> {
        let mut v: Vec<(String, &stats::RunStats)> = vec![(self.target_name(self.cfg.target.as_deref()), &self.cfg.run_stats)];
        for s in self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id) {
            v.push((self.target_name(s.target.as_deref()), &s.stats));
        }
        for h in self.cfg.history.iter().filter(|h| !h.phase) {
            if let Some(st) = h.stats.as_ref() {
                v.push((h.target.clone(), st));
            }
        }
        v
    }

    /// The month's days, from the first to the last (or today).
    fn month_data(&self, from: i64, to: i64) -> BTreeMap<i64, Day> {
        let mut out: BTreeMap<i64, Day> = BTreeMap::new();
        for (name, st) in self.month_runs() {
            for d in st.days.iter().filter(|d| (from..=to).contains(&d.day) && d.encounters > 0) {
                let e = out.entry(d.day).or_default();
                e.enc += d.encounters;
                e.secs += d.secs;
                match e.hunts.iter_mut().find(|h| h.0 == name) {
                    Some(h) => h.1 += d.encounters,
                    None => e.hunts.push((name.clone(), d.encounters)),
                }
            }
            for s in &st.sessions {
                let day = local_day(s.start);
                if (from..=to).contains(&day) && s.encounters > 0 {
                    out.entry(day).or_default().sessions += 1;
                }
            }
        }
        for h in &self.cfg.history {
            let day = local_day(h.when_unix);
            if (from..=to).contains(&day) {
                out.entry(day).or_default().shinies.push(if h.phase { format!("{} (phase)", h.target) } else { h.target.clone() });
            }
        }
        for d in out.values_mut() {
            d.hunts.sort_by(|a, b| b.1.cmp(&a.1));
        }
        out
    }

    /// The right side of the Stats header on this tab: ‹ month › and Share.
    pub(super) fn month_header(&mut self, ui: &mut egui::Ui) -> bool {
        let shown = self.month_shown();
        let (first, now) = (self.first_month(), this_month());
        let mut share = false;
        let mut go: Option<(i64, u32)> = None;
        if ui.add_enabled(shown > first, egui::Button::new("‹").min_size(vec2(30.0, 0.0))).on_hover_text("The month before").clicked() {
            go = Some(prev_month(shown));
        }
        ui.add(egui::Button::new(egui::RichText::new(month_name(shown.0, shown.1)).size(kit::fs(12.5))).min_size(vec2(130.0, 0.0)).sense(Sense::hover()));
        if ui.add_enabled(shown < now, egui::Button::new("›").min_size(vec2(30.0, 0.0))).on_hover_text("The month after").clicked() {
            go = Some(next_month(shown));
        }
        ui.add_space(4.0);
        let b = egui::Button::new(egui::RichText::new("🖼  Share").size(kit::fs(12.5)).color(kit::on_primary()));
        if kit::accent_button(ui, b, 8.0).on_hover_text("This month as an image to copy into Discord or save").clicked() {
            share = true;
        }
        if let Some(ym) = go {
            self.setup.stats_month.ym = (ym != now).then_some(ym);
            self.setup.stats_month.picked = None;
        }
        share
    }

    /// The Month in review tab.
    pub(super) fn stats_month(&mut self, ui: &mut egui::Ui) {
        let (y, m) = self.month_shown();
        let (from, to) = month_days(y, m);
        let today = local_day(now_unix());
        let days = self.month_data(from, to);
        // the picked day: the one clicked, else today (if hunted) or the best day
        let best = days.iter().max_by_key(|(_, d)| d.enc).filter(|(_, d)| d.enc > 0).map(|(k, _)| *k);
        let picked = self.setup.stats_month.picked.filter(|d| (from..=to).contains(d)).or_else(|| days.get(&today).filter(|d| d.enc > 0).map(|_| today)).or(best).unwrap_or(to);
        let wide = ui.available_width() >= 900.0;
        let gap = 14.0;
        let full = ui.available_width();
        let side = if wide { (full * 0.36).clamp(300.0, 420.0) } else { full };
        let main = if wide { full - side - gap } else { full };
        let mut clicked: Option<i64> = None;
        let mut draw_cal = |this: &mut App, ui: &mut egui::Ui| {
            let hunted = days.values().filter(|d| d.enc > 0).count();
            let right = format!("{hunted} day{} hunted", if hunted == 1 { "" } else { "s" });
            ch::card(ui, "📅", &month_name(y, m), Some((&right, MUTED())), |ui| {
                clicked = this.month_calendar(ui, y, m, from, to, today, picked, &days);
            });
        };
        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(main, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(main);
                    draw_cal(self, ui);
                });
                ui.allocate_ui_with_layout(vec2(side, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(side);
                    ui.spacing_mut().item_spacing.y = gap;
                    self.month_day_card(ui, picked, days.get(&picked), best == Some(picked), today);
                    self.month_summary(ui, y, m, from, to, &days);
                });
            });
        } else {
            draw_cal(self, ui);
            ui.add_space(gap);
            self.month_day_card(ui, picked, days.get(&picked), best == Some(picked), today);
            ui.add_space(gap);
            self.month_summary(ui, y, m, from, to, &days);
        }
        if let Some(d) = clicked {
            self.setup.stats_month.picked = Some(d);
        }
    }

    /// The calendar: weeks from Monday, each day with its encounters, the
    /// hunt, and a gold ✦ for a shiny. Returns a day clicked.
    #[allow(clippy::too_many_arguments)]
    fn month_calendar(&mut self, ui: &mut egui::Ui, y: i64, m: u32, from: i64, to: i64, today: i64, picked: i64, days: &BTreeMap<i64, Day>) -> Option<i64> {
        let first = stats::days_from_civil(y, m, 1);
        let (ny, nm) = next_month((y, m));
        let last = stats::days_from_civil(ny, nm, 1) - 1;
        let lead = stats::weekday(first) as i64;
        let max = days.values().map(|d| d.enc).max().unwrap_or(0).max(1) as f32;
        let w = ui.available_width();
        let gap = 6.0;
        let cw = ((w - gap * 6.0) / 7.0).floor();
        let chh = (cw * 0.66).clamp(58.0, 78.0);
        // weekday heads
        let (hr, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
        for (i, d) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter().enumerate() {
            let x = hr.min.x + i as f32 * (cw + gap) + cw / 2.0;
            ui.painter().text(pos2(x, hr.center().y), Align2::CENTER_CENTER, *d, ch::reg(11.0), MUTED());
        }
        ui.add_space(6.0);
        let weeks = ((lead + (last - first + 1)) as f32 / 7.0).ceil() as usize;
        let (grid, resp) = ui.allocate_exact_size(vec2(w, weeks as f32 * (chh + gap) - gap), Sense::click());
        let p = ui.painter_at(grid.expand(4.0));
        let hover = resp.hover_pos();
        let mut hit: Option<i64> = None;
        let blue = [Color32::from_rgb(29, 58, 107), Color32::from_rgb(36, 99, 196), Color32::from_rgb(61, 139, 255)];
        for day in first..=last {
            let k = lead + (day - first);
            let (row, col) = (k / 7, k % 7);
            let r = Rect::from_min_size(pos2(grid.min.x + col as f32 * (cw + gap), grid.min.y + row as f32 * (chh + gap)), vec2(cw, chh));
            let d = days.get(&day);
            let enc = d.map_or(0, |d| d.enc);
            let future = day > to;
            let bg = if future {
                kit::CARD().gamma_multiply(0.6)
            } else if enc == 0 {
                kit::sunken()
            } else {
                let f = enc as f32 / max;
                blue[if f > 0.66 { 2 } else if f > 0.33 { 1 } else { 0 }]
            };
            let hovered = !future && hover.is_some_and(|h| r.contains(h));
            p.rect_filled(r, 10.0, if hovered && enc == 0 { kit::CARD_HOVER() } else { bg });
            if day == picked {
                p.rect_stroke(r, 10.0, Stroke::new(2.0f32, kit::C_VIOLET));
            } else if day == today {
                p.rect_stroke(r, 10.0, Stroke::new(1.0f32, MUTED()));
            } else if hovered {
                p.rect_stroke(r, 10.0, Stroke::new(1.0f32, kit::LINE()));
            }
            let num_col = if future { MUTED().gamma_multiply(0.5) } else if enc > 0 { Color32::WHITE } else { MUTED() };
            p.text(r.min + vec2(8.0, 8.0), Align2::LEFT_TOP, (day - first + 1).to_string(), ch::bold(11.5), num_col);
            if let Some(d) = d.filter(|d| d.enc > 0) {
                let nf = ch::heavy(if cw < 80.0 { 12.5 } else { 14.0 });
                p.text(pos2(r.min.x + 8.0, r.max.y - 26.0), Align2::LEFT_CENTER, theme::thousands(d.enc), nf, Color32::WHITE);
                let what = if d.hunts.len() > 1 { format!("{} hunts", d.hunts.len()) } else { d.hunts.first().map(|h| h.0.clone()).unwrap_or_default() };
                let wf = ch::reg(10.0);
                p.text(pos2(r.min.x + 8.0, r.max.y - 11.0), Align2::LEFT_CENTER, ch::clip(ui, &what, &wf, cw - 14.0), wf, Color32::from_rgb(207, 224, 255));
            }
            if d.is_some_and(|d| !d.shinies.is_empty()) {
                theme::sparkle(&p, pos2(r.max.x - 12.0, r.min.y + 12.0), 6.0, ch::GOLD);
            }
            if hovered {
                hit = Some(day);
            }
        }
        if resp.clicked() {
            return hit;
        }
        if hit.is_some() {
            resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        None
    }

    /// The picked day.
    fn month_day_card(&mut self, ui: &mut egui::Ui, day: i64, d: Option<&Day>, best: bool, today: i64) {
        let (_, mo, dd) = stats::civil_from_days(day);
        let title = format!("{} {dd} {}", DAYS[stats::weekday(day)], MONTHS[(mo - 1) as usize]);
        let tag = if day == today { "today" } else { "picked" };
        ch::card(ui, "✨", &title, Some((tag, MUTED())), |ui| {
            let Some(d) = d.filter(|d| d.enc > 0 || !d.shinies.is_empty()) else {
                ch::empty(ui, if day > today { "Still to come." } else { "Nothing hunted this day. Click a blue day to see it." });
                return;
            };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(theme::thousands(d.enc)).font(ch::heavy(30.0)).color(TEXT()));
                ui.label(egui::RichText::new("encounters").font(ch::bold(13.0)).color(MUTED()));
            });
            ui.add_space(4.0);
            let mut rows: Vec<(&str, String, Color32)> = Vec::new();
            if !d.hunts.is_empty() {
                rows.push(("Hunting", d.hunts.iter().map(|h| h.0.clone()).collect::<Vec<_>>().join(" · "), TEXT()));
            }
            if d.secs > 0 {
                let s = if d.sessions > 0 { format!(" · {} session{}", d.sessions, if d.sessions == 1 { "" } else { "s" }) } else { String::new() };
                rows.push(("Time", format!("{}{s}", fmt_duration(d.secs)), TEXT()));
                if d.secs >= 600 {
                    rows.push(("Pace", format!("{}/h", theme::thousands((d.enc as f64 / (d.secs as f64 / 3600.0)).round() as u64)), TEXT()));
                }
            }
            if !d.shinies.is_empty() {
                rows.push(("Shinies", format!("★ {}", d.shinies.join(" · ")), ch::GOLD));
            }
            if best {
                rows.push(("Your best day", "of the month".into(), ch::GOLD));
            }
            month_rows(ui, &rows);
        });
    }

    /// The month's totals.
    fn month_summary(&mut self, ui: &mut egui::Ui, y: i64, m: u32, from: i64, to: i64, days: &BTreeMap<i64, Day>) {
        let now = this_month() == (y, m);
        let title = if now { format!("{} so far", MONTHS[(m - 1) as usize]) } else { month_name(y, m) };
        let o = self.share_overview(from, to, "", "", false);
        let length = (to - from + 1).max(1);
        let hunted = days.values().filter(|d| d.enc > 0).count();
        ch::card(ui, "📊", &title, None, |ui| {
            let mut rows: Vec<(&str, String, Color32)> = vec![
                ("Encounters", theme::thousands(o.encounters), TEXT()),
                ("Shinies", o.shinies.to_string(), if o.shinies > 0 { ch::GOLD } else { TEXT() }),
                ("Hunt time", if o.secs > 0 { fmt_duration(o.secs) } else { "—".into() }, TEXT()),
                ("Days hunted", format!("{hunted} of {length}"), TEXT()),
            ];
            // luck only from this month's own shinies
            if o.shinies > 0 {
                if let Some(l) = o.luck {
                    rows.push(("Luck", format!("{l:.2}×"), if l < 1.0 { ch::GREEN } else if l < 1.5 { ch::GOLD } else { ch::RED }));
                }
            }
            if let Some((n, day, _)) = &o.best_day {
                rows.push(("Best day", format!("{} · {day}", theme::thousands(*n)), TEXT()));
            }
            month_rows(ui, &rows);
        });
    }

    /// Once a month: last month's recap is ready (a pop-up, if you hunted).
    pub(super) fn month_recap_check(&mut self) {
        let now = this_month();
        match self.cfg.recap_month {
            Some(seen) if seen == now => {}
            Some(_) => {
                self.cfg.recap_month = Some(now);
                self.dirty = true;
                let last = prev_month(now);
                let (a, b) = month_days(last.0, last.1);
                if self.month_data(a, b).values().any(|d| d.enc > 0) {
                    self.push_milestone(format!("📅 Your {} in review is ready · Stats › Month in review", MONTHS[(last.1 - 1) as usize]));
                }
            }
            None => {
                // first run: start counting from this month
                self.cfg.recap_month = Some(now);
                self.dirty = true;
            }
        }
    }

    pub(super) fn share_month(&mut self, ctx: &egui::Context) {
        let (y, m) = self.month_shown();
        self.open_stat_share(ShareWhat::Month(y, m), ctx);
    }
}

/// Label left, value right, a hairline between rows.
fn month_rows(ui: &mut egui::Ui, rows: &[(&str, String, Color32)]) {
    for (i, (l, v, c)) in rows.iter().enumerate() {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
        let p = ui.painter();
        p.text(pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, *l, ch::reg(12.5), MUTED());
        let vf = ch::reg(12.5);
        let v = ch::clip(ui, v, &vf, r.width() - 120.0);
        p.text(pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, v, vf, *c);
        if i + 1 < rows.len() {
            p.hline(r.x_range(), r.max.y - 0.5, Stroke::new(1.0f32, kit::LINE()));
        }
    }
}
