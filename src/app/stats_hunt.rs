//! A hunt's stats as one dashboard: the hero (count and activity rings),
//! luck (where you sit among hunters, the curve, what's ahead), pace,
//! habits (days of the week, a 24-hour clock), trends, sessions with a
//! calendar, what you're running into, and fun facts.

use super::charts::{self as ch, Bars, Line, Ring, BLUE, CYAN, GOLD, GREEN, LIME, ORANGE, PINK, PURPLE, RED, YELLOW};
use super::hunt_pages::RunInfo;
use super::ui_kit::{self as kit, CARD_HOVER, MUTED, TEXT};
use super::App;
use crate::stats::{self, fmt_day_short, fmt_duration, local_day, now_unix, RunStats, WEEKDAYS};
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, Sense};

/// Which period the pace chart shows.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum PaceView {
    #[default]
    Days,
    SixHours,
    Hours,
}

pub(super) fn pct(x: f64) -> String {
    let p = x * 100.0;
    if p > 0.0 && p < 1.0 {
        "<1%".to_string()
    } else if p < 100.0 && p > 99.0 {
        ">99%".to_string()
    } else {
        format!("{p:.0}%")
    }
}

fn time_for(n: u64, rate: Option<f64>) -> Option<String> {
    let r = rate.filter(|r| *r > 0.0)?;
    let secs = (n as f64 / r * 3600.0).round() as u64;
    Some(if secs < 60 { "under a minute".to_string() } else { fmt_duration(secs) })
}

fn uses_sweet_scent(method: &str) -> bool {
    method.contains("Sweet Scent") || method.contains("Rare grass")
}

fn mult(x: f64) -> String {
    if x < 0.1 {
        format!("{x:.2}×")
    } else {
        format!("{x:.1}×")
    }
}

/// Change from `last` to `this` as "+12%" (None when there's nothing to compare).
fn change(this: f64, last: f64) -> Option<(f64, String)> {
    if last <= 0.0 {
        return None;
    }
    let c = (this - last) / last;
    Some((c, format!("{}{:.0}%", if c >= 0.0 { "+" } else { "" }, c * 100.0)))
}

impl App {
    /// Run stats: the whole dashboard.
    pub(super) fn hunt_dashboard(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        self.stats_hero(ui, info, st);
        ui.add_space(12.0);
        if let Some(odds) = info.odds.filter(|o| *o > 1.0) {
            self.hunt_luck(ui, info, st, odds);
            ui.add_space(12.0);
        }
        if st.sessions.is_empty() && st.days.is_empty() {
            ch::card(ui, "⏱", "Time stats", None, |ui| {
                ch::empty(ui, "Once you've hunted for a few minutes, your pace, habits and sessions show up here.");
            });
        } else {
            self.hunt_pace(ui, info, st);
            ui.add_space(12.0);
            let wide = ui.available_width() >= 900.0;
            let habit = |s: &mut Self, ui: &mut egui::Ui, k: usize| match k {
                0 => s.hunt_weekdays(ui, st),
                _ => s.hunt_clock(ui, st),
            };
            if wide {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    ui.columns(2, |cols| {
                        let (a, b) = cols.split_at_mut(1);
                        habit(self, &mut a[0], 0);
                        habit(self, &mut b[0], 1);
                    });
                });
            } else {
                habit(self, ui, 0);
                ui.add_space(12.0);
                habit(self, ui, 1);
            }
            ui.add_space(12.0);
            self.hunt_trends(ui, info, st);
            ui.add_space(12.0);
            self.hunt_sessions(ui, info, st);
            ui.add_space(12.0);
        }
        if !info.phase && !st.species.is_empty() {
            self.hunt_species(ui, info, st);
            ui.add_space(12.0);
        }
        self.hunt_facts(ui, info, st);
        ui.add_space(8.0);
        if info.stats.is_none() {
            kit::pill(ui, "This hunt was logged before stats existed, so only the totals are known.", kit::PillKind::Info);
        } else if st.base_count > 0 && st.tracking_since > 0 {
            ui.label(
                egui::RichText::new(format!("Time stats started {}; the {} encounters before that only count in the totals.", stats::fmt_date(st.tracking_since), theme::thousands(st.base_count)))
                    .size(kit::fs(11.0))
                    .color(MUTED()),
            );
        }
    }

    // ───────────────────────────── hero ─────────────────────────────

    fn stats_hero(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        let now = now_unix();
        let today = local_day(now);
        let found = info.odds.map(|o| stats::found_by(info.count, o));
        let luck_c = found.map_or(PURPLE, ch::luck_col);
        // the rings: odds, today, this session
        let mut rings: Vec<(Ring, &str, String, String)> = Vec::new();
        if let Some(o) = info.odds {
            let m = info.count as f64 / o;
            rings.push((Ring { frac: m as f32, col: luck_c }, "Odds", mult(m), format!("of 1 in {}", theme::thousands(o.round() as u64))));
        }
        if info.current {
            let (n, _) = st.today(today);
            let goal = self.cfg.daily_goal.max(1);
            rings.push((Ring { frac: n as f32 / goal as f32, col: GREEN }, "Today", format!("{}/{}", theme::thousands(n), theme::thousands(goal)), "encounters".into()));
            let sess = st.current_session(now).map_or(0, |s| s.encounters);
            let sgoal = if self.cfg.session_goal > 0 { self.cfg.session_goal } else { st.sessions.iter().map(|s| s.encounters).sum::<u64>() / st.sessions.len().max(1) as u64 };
            if sgoal > 0 {
                rings.push((Ring { frac: sess as f32 / sgoal as f32, col: CYAN }, "Session", format!("{}/{}", theme::thousands(sess), theme::thousands(sgoal)), if self.cfg.session_goal > 0 { "session goal".into() } else { "your average".into() }));
            }
        } else if let Some(r) = st.per_hour() {
            let best = st.fastest_session().map_or(r, |(_, b)| b).max(1.0);
            rings.push((Ring { frac: (r / best) as f32, col: CYAN }, "Pace", format!("{}/h", theme::thousands(r.round() as u64)), "of your best session".into()));
        }

        let h = 236.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), Sense::hover());
        let p = ui.painter_at(rect.expand(12.0));
        p.add(kit::card_shadow().as_shape(rect, 20.0));
        p.rect_filled(rect, 20.0, kit::CARD());
        // a soft glow in the luck colour behind the sprite
        for (k, a) in [(1.0, 0.05), (0.75, 0.07), (0.5, 0.09)] {
            p.circle_filled(egui::pos2(rect.min.x + 120.0, rect.center().y), 104.0 * k, ch::faint(luck_c, a));
        }
        p.rect_stroke(rect, 20.0, egui::Stroke::new(1.0f32, kit::LINE()));
        let sc = egui::pos2(rect.min.x + 120.0, rect.center().y);
        if let Some((_, sp)) = self.species_of(info.slug.as_deref()) {
            self.draw_sprite_centered(&p, &sp, true, sc, egui::vec2(170.0, 170.0));
        }
        theme::sparkle(&p, sc + egui::vec2(64.0, -62.0), 9.0, GOLD);
        theme::sparkle(&p, sc + egui::vec2(-70.0, 40.0), 5.0, GOLD);

        let x = rect.min.x + 248.0;
        let rings_w = if rings.is_empty() { 0.0 } else { 380.0f32.min(rect.width() * 0.42) };
        let room = rect.max.x - rings_w - x - 16.0;
        let mut y = rect.min.y + 26.0;
        // status chip
        let (st_t, st_c) = if info.caught {
            ("★ CAUGHT", GOLD)
        } else if info.current {
            ("● HUNTING", GREEN)
        } else {
            ("SAVED RUN", MUTED())
        };
        let g = p.layout_no_wrap(st_t.to_string(), ch::bold(11.0), ch::ink(st_c));
        let cw = g.size().x + 16.0;
        p.rect_filled(Rect::from_min_size(egui::pos2(x, y), egui::vec2(cw, 22.0)), 11.0, ch::faint(st_c, 0.18));
        p.galley(egui::pos2(x + 8.0, y + 4.0), g, ch::ink(st_c));
        y += 30.0;
        let nf = ch::fit_font(ui, &info.name, ch::heavy, 34.0, 18.0, room);
        p.text(egui::pos2(x, y), Align2::LEFT_TOP, &info.name, nf, TEXT());
        y += 44.0;
        let mut sub = Vec::new();
        if !info.method.is_empty() {
            sub.push(info.method.clone());
        }
        if let Some(o) = info.odds {
            sub.push(format!("1 in {}", theme::thousands(o.round() as u64)));
        }
        let when = match info.ended {
            Some(e) => format!("{} → {}", fmt_day_short(local_day(info.started.max(1))), fmt_day_short(local_day(e))),
            None if info.started > 0 => format!("since {}", fmt_day_short(local_day(info.started))),
            None => String::new(),
        };
        if !when.is_empty() {
            sub.push(when);
        }
        p.text(egui::pos2(x, y), Align2::LEFT_TOP, ch::clip(ui, &sub.join("  ·  "), &ch::reg(13.0), room), ch::reg(13.0), MUTED());
        y += 30.0;
        let num = theme::thousands(info.count);
        let nf = ch::fit_font(ui, &num, ch::heavy, 56.0, 28.0, room - 110.0);
        let nr = p.text(egui::pos2(x - 2.0, y), Align2::LEFT_TOP, &num, nf, ch::ink(luck_c));
        p.text(egui::pos2(nr.max.x + 8.0, nr.max.y - 12.0), Align2::LEFT_BOTTOM, "encounters", ch::bold(15.0), MUTED());
        y = nr.max.y + 8.0;
        let mut facts = Vec::new();
        if st.hunt_secs >= 60 {
            facts.push(format!("⏱ {}", fmt_duration(st.hunt_secs)));
        }
        if let Some(r) = st.per_hour() {
            facts.push(format!("⚡ {}/h", theme::thousands(r.round() as u64)));
        }
        if st.days_hunted() > 0 {
            facts.push(format!("📅 {} day{}", st.days_hunted(), if st.days_hunted() == 1 { "" } else { "s" }));
        }
        if info.battles > 0 {
            facts.push(format!("⚔ {} battles", theme::thousands(info.battles)));
        }
        p.text(egui::pos2(x, y), Align2::LEFT_TOP, ch::clip(ui, &facts.join("    "), &ch::reg(13.0), room), ch::reg(13.0), TEXT());

        if !rings.is_empty() {
            let rr = Rect::from_min_max(egui::pos2(rect.max.x - rings_w, rect.min.y), rect.max);
            let size = (rr.height() - 44.0).min(180.0);
            let c = egui::pos2(rr.min.x + size / 2.0 + 6.0, rr.center().y);
            let list: Vec<Ring> = rings.iter().map(|r| Ring { frac: r.0.frac, col: r.0.col }).collect();
            ch::rings(&p, c, size / 2.0, 20.0, 4.0, &list);
            let lx = c.x + size / 2.0 + 18.0;
            let mut ly = rr.center().y - rings.len() as f32 * 26.0;
            for (rg, name, val, note) in &rings {
                p.text(egui::pos2(lx, ly), Align2::LEFT_TOP, *name, ch::bold(12.5), TEXT());
                let vr = p.text(egui::pos2(lx, ly + 16.0), Align2::LEFT_TOP, val, ch::heavy(19.0), ch::ink(rg.col));
                p.text(egui::pos2(vr.max.x + 6.0, vr.max.y - 4.0), Align2::LEFT_BOTTOM, ch::clip(ui, note, &ch::reg(11.0), rect.max.x - vr.max.x - 16.0), ch::reg(11.0), MUTED());
                ly += 52.0;
            }
        }
    }

    // ───────────────────────────── luck ─────────────────────────────

    fn hunt_luck(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats, odds: f64) {
        let found = stats::found_by(info.count, odds);
        let m = info.count as f64 / odds;
        let rate = st.per_hour();
        let band = ch::luck_band(found);
        let (word, col) = (ch::LUCK_BANDS[band].1, ch::LUCK_BANDS[band].2);
        let right = if info.caught { format!("{word} catch") } else if found < 0.45 { "Early days".to_string() } else { format!("{word} so far") };
        ch::card(ui, "🍀", "Luck", Some((&right, col)), |ui| {
            let headline = if info.caught {
                let beat = 1.0 - found;
                if beat >= 0.5 {
                    format!("Luckier than {} of hunters", pct(beat))
                } else {
                    format!("{} of hunters would have found it sooner", pct(found))
                }
            } else {
                format!("{} of hunters would have found it by now", pct(found))
            };
            ui.label(egui::RichText::new(headline).font(ch::bold(20.0)).color(TEXT()));
            let sub = if info.caught {
                format!("Found after {} encounters: {} the odds. The typical hunter needs {}.", theme::thousands(info.count), mult(m), theme::thousands(stats::encounters_for(0.5, odds)))
            } else {
                let half = stats::encounters_for(0.5, odds);
                let when = time_for(half, rate).map_or(String::new(), |t| format!(" (about {t} at your pace)"));
                format!("You're at {} the odds. Every encounter is a fresh 1 in {}, so from here it's 50/50 within the next {}{when}.", mult(m), theme::thousands(odds.round() as u64), theme::thousands(half))
            };
            ui.add(egui::Label::new(egui::RichText::new(sub).size(kit::fs(13.0)).color(MUTED())).wrap());
            ui.add_space(14.0);
            ch::luck_scale(ui, found, if info.caught { "Your catch" } else { "You" });
            ui.add_space(14.0);

            // the curve: share of hunters with it by each multiple of the odds
            ui.label(egui::RichText::new("When hunters find this shiny").font(ch::bold(14.0)).color(TEXT()));
            let x_max = (m * 1.15).max(3.0).ceil();
            let steps = (x_max * 4.0) as usize;
            let vals: Vec<f64> = (0..=steps).map(|i| stats::found_by((i as f64 / 4.0 * odds) as u64, odds) * 100.0).collect();
            let labels: Vec<String> = (0..=steps).map(|i| if i % 4 == 0 { format!("{}×", i / 4) } else { String::new() }).collect();
            let hovers: Vec<String> = (0..=steps)
                .map(|i| format!("{} encounters ({}): {} of hunters", theme::thousands((i as f64 / 4.0 * odds) as u64), mult(i as f64 / 4.0), pct(vals[i] / 100.0)))
                .collect();
            let bands: Vec<(f64, f64, Color32)> = {
                let mut lo = 0.0;
                ch::LUCK_BANDS
                    .iter()
                    .map(|(hi, _, c)| {
                        let r = (lo * 100.0, hi.min(1.0) * 100.0, *c);
                        lo = hi.min(1.0);
                        r
                    })
                    .collect()
            };
            let ticks: Vec<(f64, String)> = [0.0, 25.0, 50.0, 75.0, 100.0].iter().map(|v| (*v, format!("{v:.0}%"))).collect();
            let you = if info.caught { "Caught" } else { "You" };
            ch::line_chart(
                ui,
                &mut Line {
                    values: &vals,
                    labels: &labels,
                    bands: &bands,
                    lo: 0.0,
                    hi: 100.0,
                    ticks: &ticks,
                    col: &|_, v| ch::luck_col(v / 100.0),
                    trend: false,
                    hovers: &hovers,
                    height: 200.0,
                    area: None,
                    stars: &[],
                    you: Some(((m * 4.0) as f32, found * 100.0, you)),
                    pic: None,
                    dots: false,
                    line: None,
                },
            );
            if !info.caught {
                ui.add_space(10.0);
                let w = ch::tile_w(ui, 4);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for (q, c) in [(0.5, GREEN), (0.75, YELLOW), (0.9, ORANGE), (0.99, RED)] {
                        let n = stats::encounters_for(q, odds);
                        let v = theme::thousands(n);
                        let note = time_for(n, rate).map_or("more encounters".into(), |t| format!("more · about {t}"));
                        ch::trend_tile(ui, w, None, &format!("{} chance within", pct(q)), &v, &note, c);
                    }
                });
            }
        });
    }

    // ───────────────────────────── pace ─────────────────────────────

    fn hunt_pace(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        let now = now_unix();
        let rate = st.per_hour();
        ch::card(ui, "⚡", "Pace", None, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 34.0;
                ch::big_number(ui, "Per hour", &rate.map_or("—".into(), |r| theme::thousands(r.round() as u64)), "/h", CYAN, 34.0);
                ch::big_number(ui, "Per encounter", &st.secs_per_encounter().map_or("—".into(), |s| format!("{s:.1}")), "s", PURPLE, 34.0);
                if let Some(b) = st.best_day().filter(|d| d.encounters > 0) {
                    ch::big_number(ui, &format!("Best day · {}", fmt_day_short(b.day)), &theme::thousands(b.encounters), "", LIME, 34.0);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut v = self.setup.pace_view;
                    kit::segmented(ui, &mut v, &[(PaceView::Hours, " Hour "), (PaceView::SixHours, " 6 h "), (PaceView::Days, " Day ")]);
                    self.setup.pace_view = v;
                });
            });
            ui.add_space(12.0);
            let end_day = if info.current || st.days.is_empty() { local_day(now) } else { st.days.last().map_or(local_day(now), |d| d.day) };
            let end_hour = if info.current || st.last_unix == 0 { stats::local_hour(now) } else { stats::local_hour(st.last_unix) };
            let (vals, labels, hovers): (Vec<f64>, Vec<String>, Vec<String>) = match self.setup.pace_view {
                PaceView::Days => {
                    let n = 21i64;
                    let mut v = Vec::new();
                    let mut l = Vec::new();
                    let mut hv = Vec::new();
                    for d in end_day - n + 1..=end_day {
                        let (e, s) = st.today(d);
                        v.push(e as f64);
                        let (_, _, dd) = stats::civil_from_days(d);
                        l.push(if dd == 1 || d == end_day || (end_day - d) % 7 == 0 { dd.to_string() } else { String::new() });
                        hv.push(format!("{}: {} encounters{}", fmt_day_short(d), theme::thousands(e), if s > 0 { format!(" · {}", fmt_duration(s)) } else { String::new() }));
                    }
                    (v, l, hv)
                }
                PaceView::SixHours | PaceView::Hours => {
                    let (step, n) = if self.setup.pace_view == PaceView::SixHours { (6i64, 28i64) } else { (1, 24) };
                    let to = end_hour;
                    let from = to - step * n + 1;
                    let hourly = st.hourly(from, to);
                    let mut v = Vec::new();
                    let mut l = Vec::new();
                    let mut hv = Vec::new();
                    for k in 0..n {
                        let s: u64 = hourly.iter().skip((k * step) as usize).take(step as usize).sum();
                        let h0 = from + k * step;
                        v.push(s as f64);
                        l.push(if k % if step == 1 { 4 } else { 4 } == 0 { stats::fmt_hour(h0) } else { String::new() });
                        hv.push(format!("{} {}: {}", fmt_day_short(h0.div_euclid(24)), stats::fmt_hour(h0), theme::thousands(s)));
                    }
                    (v, l, hv)
                }
            };
            let last = vals.len().saturating_sub(1);
            let hot = |i: usize| i == last;
            let best = vals.iter().cloned().fold(0.0, f64::max);
            let best_i = vals.iter().position(|v| *v == best && best > 0.0);
            let hot2 = |i: usize| hot(i) || Some(i) == best_i;
            ch::bars(ui, &Bars { values: &vals, labels: &labels, col: CYAN, hot: Some(&hot2), hovers: &hovers, tops: false, axis: true, height: 170.0, cols: None, subs: &[] });
        });
    }

    // ───────────────────────────── habits ─────────────────────────────

    fn hunt_weekdays(&mut self, ui: &mut egui::Ui, st: &RunStats) {
        let mut sum = [0u64; 7];
        let mut days = [0u64; 7];
        for d in &st.days {
            if d.encounters > 0 {
                let w = stats::weekday(d.day);
                sum[w] += d.encounters;
                days[w] += 1;
            }
        }
        let avg: Vec<f64> = (0..7).map(|w| if days[w] > 0 { sum[w] as f64 / days[w] as f64 } else { 0.0 }).collect();
        let best = avg.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).filter(|(_, v)| **v > 0.0).map(|(i, _)| i);
        let right = best.map(|b| format!("Best: {}", WEEKDAYS[b]));
        ch::card(ui, "📅", "Average by day", right.as_deref().map(|r| (r, YELLOW)), |ui| {
            let labels: Vec<String> = WEEKDAYS.iter().map(|s| s.to_string()).collect();
            let subs: Vec<String> = days.iter().map(|n| if *n > 0 { format!("{n}×") } else { "–".into() }).collect();
            let hovers: Vec<String> = (0..7).map(|w| format!("{}: {} encounters on average over {} day{}", WEEKDAYS[w], theme::thousands(avg[w].round() as u64), days[w], if days[w] == 1 { "" } else { "s" })).collect();
            let hot = |i: usize| Some(i) == best;
            ch::bars(ui, &Bars { values: &avg, labels: &labels, col: YELLOW, hot: Some(&hot), hovers: &hovers, tops: true, axis: false, height: 210.0, cols: None, subs: &subs });
            ch::empty(ui, "Encounters on an average hunting day; the small number is how many of those days you've hunted.");
        });
    }

    fn hunt_clock(&mut self, ui: &mut egui::Ui, st: &RunStats) {
        let slots = st.by_hour_of_day();
        let mut vals = [0f64; 24];
        for (i, s) in slots.iter().enumerate() {
            vals[i] = s.encounters;
        }
        let best = vals.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).filter(|(_, v)| **v > 0.0).map(|(i, _)| i);
        let fastest = slots.iter().enumerate().filter_map(|(i, s)| s.rate().map(|r| (i, r))).max_by(|a, b| a.1.total_cmp(&b.1));
        let right = fastest.map(|(i, r)| format!("Fastest {} · {}/h", stats::fmt_hour(i as i64), theme::thousands(r.round() as u64)));
        ch::card(ui, "🕘", "Hunt clock", right.as_deref().map(|r| (r, PINK)), |ui| {
            let centre = best.map_or(("—".to_string(), "".to_string()), |b| (stats::fmt_hour(b as i64), "busiest hour".to_string()));
            ch::clock(ui, &vals, PINK, 236.0, (&centre.0, &centre.1));
            ch::empty(ui, "When your encounters happen, by hour of the day.");
        });
    }

    // ───────────────────────────── trends ─────────────────────────────

    fn hunt_trends(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        let today = local_day(now_unix());
        let end = if info.current { today } else { st.days.last().map_or(today, |d| d.day) };
        let ((te, ts), (le, ls)) = st.week_compare(end);
        let rate = |e: u64, s: u64| if s >= 300 { e as f64 * 3600.0 / s as f64 } else { 0.0 };
        let days_in = |a: i64, b: i64| st.days.iter().filter(|d| d.day > a && d.day <= b && d.encounters > 0).count() as f64;
        let (td, ld) = (days_in(end - 7, end), days_in(end - 14, end - 7));
        let sess: Vec<u64> = st.sessions.iter().map(|s| s.secs).filter(|s| *s > 0).collect();
        let half = sess.len() / 2;
        let recent_avg = if sess.len() >= 4 { sess[half..].iter().sum::<u64>() as f64 / (sess.len() - half) as f64 } else { 0.0 };
        let early_avg = if sess.len() >= 4 { sess[..half].iter().sum::<u64>() as f64 / half as f64 } else { 0.0 };
        // (label, value, change, note when up, note when down, colour)
        let mut items: Vec<(String, String, Option<(f64, String)>, String, String, Color32)> = vec![
            ("Encounters, last 7 days".into(), theme::thousands(te), change(te as f64, le as f64), format!("Up on the week before ({}).", theme::thousands(le)), format!("The week before had {}. A couple of extra sessions would close the gap.", theme::thousands(le)), GREEN),
            ("Pace".into(), if rate(te, ts) > 0.0 { format!("{}/h", theme::thousands(rate(te, ts).round() as u64)) } else { "—".into() }, change(rate(te, ts), rate(le, ls)), "Faster than the week before.".into(), "Slower than the week before. Check your loop: fewer menus and a lead with a quick escape help.".into(), CYAN),
            ("Days hunted".into(), format!("{td:.0} of 7"), change(td, ld), "More days than the week before.".into(), "Fewer days than the week before. Little and often adds up.".into(), YELLOW),
            ("Session length".into(), if recent_avg > 0.0 { fmt_duration(recent_avg as u64) } else { "—".into() }, change(recent_avg, early_avg), "Your recent sessions run longer than your early ones.".into(), "Recent sessions are shorter than your early ones.".into(), PURPLE),
        ];
        items.retain(|i| i.1 != "—" && i.1 != "0");
        if items.is_empty() {
            return;
        }
        ch::card(ui, "📈", "Trends", Some(("this week vs last", MUTED())), |ui| {
            let up: Vec<_> = items.iter().filter(|i| i.2.as_ref().is_none_or(|c| c.0 >= 0.0)).collect();
            let down: Vec<_> = items.iter().filter(|i| i.2.as_ref().is_some_and(|c| c.0 < 0.0)).collect();
            for (title, list, is_up) in [("Keep it going", &up, true), ("Worth a look", &down, false)] {
                if list.is_empty() {
                    continue;
                }
                ui.label(egui::RichText::new(title).font(ch::bold(14.0)).color(TEXT()));
                ui.add_space(6.0);
                let per = if ui.available_width() >= 760.0 { 2 } else { 1 };
                for row in list.chunks(per) {
                    let w = ch::tile_w(ui, per);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        for (label, value, c, up_note, down_note, col) in row.iter().map(|i| (&i.0, &i.1, &i.2, &i.3, &i.4, i.5)) {
                            let flat = c.as_ref().is_some_and(|c| c.0.abs() < 0.02);
                            let dir = if flat { None } else { c.as_ref().map(|c| c.0 >= 0.0) };
                            let val = match c {
                                Some((_, t)) => format!("{value}  {t}"),
                                None => value.clone(),
                            };
                            let note = if c.is_none() { "Nothing to compare with yet." } else if flat { "About the same as before." } else if is_up { up_note.as_str() } else { down_note.as_str() };
                            ch::trend_tile(ui, w, dir, label, &val, note, if is_up { col } else { ORANGE });
                        }
                    });
                    ui.add_space(10.0);
                }
            }
        });
    }

    // ───────────────────────────── sessions ─────────────────────────────

    fn hunt_sessions(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        let now = now_unix();
        let end = if info.current { local_day(now) } else { st.days.last().map_or(local_day(now), |d| d.day) };
        let first = st.days.first().map_or(end, |d| d.day);
        let span_weeks = ((end - first) / 7 + 2) as usize;
        let right = format!("{} session{}", st.sessions.len(), if st.sessions.len() == 1 { "" } else { "s" });
        ch::card(ui, "📆", "Sessions", Some((&right, MUTED())), |ui| {
            let by: std::collections::HashMap<i64, (u64, u64)> = st.days.iter().map(|d| (d.day, (d.encounters, d.secs))).collect();
            let cal = |ui: &mut egui::Ui| {
                // as many weeks as fit at a comfortable size (at least the hunt)
                let weeks = (((ui.available_width() - 26.0) / 27.0) as usize).clamp(8, 30).max(span_weeks.min(52));
                ch::heatmap(ui, end, weeks, &|d| by.get(&d).map_or(0, |v| v.0), GREEN, &|d, v| {
                    let secs = by.get(&d).map_or(0, |x| x.1);
                    if v == 0 { format!("{}: no hunting", fmt_day_short(d)) } else { format!("{}: {} encounters · {}", fmt_day_short(d), theme::thousands(v), fmt_duration(secs)) }
                });
            };
            let recs = |ui: &mut egui::Ui, w: f32, vertical: bool| {
                let tiles: [(&str, String, &str, Color32); 3] = [
                    ("⌛", st.avg_session_secs().map_or("—".into(), fmt_duration), "Average session", PURPLE),
                    ("🏃", st.longest_session().map_or("—".into(), |s| fmt_duration(s.secs)), "Longest session", ORANGE),
                    ("🚀", st.fastest_session().map_or("—".into(), |(_, r)| format!("{}/h", theme::thousands(r.round() as u64))), "Fastest session", CYAN),
                ];
                if vertical {
                    for (i, v, l, c) in &tiles {
                        ch::fact_tile(ui, w, i, v, l, *c);
                        ui.add_space(8.0);
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        for (i, v, l, c) in &tiles {
                            ch::fact_tile(ui, w, i, v, l, *c);
                        }
                    });
                }
            };
            if ui.available_width() >= 760.0 {
                ui.horizontal_top(|ui| {
                    let rw = 270.0;
                    let lw = ui.available_width() - rw - 32.0;
                    ui.allocate_ui_with_layout(egui::vec2(lw, 260.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(lw);
                        cal(ui);
                    });
                    ui.add_space(16.0);
                    ui.vertical(|ui| recs(ui, rw, true));
                });
            } else {
                cal(ui);
                ui.add_space(10.0);
                let w = ch::tile_w(ui, 3);
                recs(ui, w, false);
            }
            ui.add_space(10.0);
            // recent sessions as bars
            let recent: Vec<&stats::Session> = st.sessions.iter().rev().take(6).collect();
            let longest = recent.iter().map(|s| s.secs).max().unwrap_or(1).max(1);
            let live = info.current && st.current_session(now).is_some();
            for (k, s) in recent.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), Sense::hover());
                let p = ui.painter_at(r);
                if resp.hovered() {
                    p.rect_filled(r, 8.0, CARD_HOVER());
                }
                let is_live = k == 0 && live;
                p.text(egui::pos2(r.min.x + 8.0, r.center().y), Align2::LEFT_CENTER, format!("{}  {}", fmt_day_short(local_day(s.start)), stats::fmt_clock(s.start)), ch::reg(12.5), TEXT());
                let rate = (s.secs >= 60 && s.encounters > 1).then(|| (s.encounters - 1) as f64 * 3600.0 / s.secs as f64);
                p.text(egui::pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, rate.map_or("—".into(), |v| format!("{}/h", theme::thousands(v.round() as u64))), ch::reg(12.5), MUTED());
                p.text(egui::pos2(r.max.x - 90.0, r.center().y), Align2::RIGHT_CENTER, format!("{} · {}", theme::thousands(s.encounters), fmt_duration(s.secs)), ch::bold(12.5), TEXT());
                let x0 = r.min.x + 150.0;
                let x1 = r.max.x - 230.0;
                if x1 > x0 + 20.0 {
                    let tr = Rect::from_min_max(egui::pos2(x0, r.center().y - 5.0), egui::pos2(x1, r.center().y + 5.0));
                    p.rect_filled(tr, 5.0, CARD_HOVER());
                    let fw = (tr.width() * s.secs as f32 / longest as f32).max(6.0);
                    p.rect_filled(Rect::from_min_size(tr.min, egui::vec2(fw, tr.height())), 5.0, if is_live { GREEN } else { ch::faint(PURPLE, 0.8) });
                    if is_live {
                        p.text(egui::pos2(tr.min.x + fw + 8.0, r.center().y), Align2::LEFT_CENTER, "LIVE", ch::bold(10.5), ch::ink(GREEN));
                    }
                }
            }
        });
    }

    // ───────────────────────────── species ─────────────────────────────

    fn hunt_species(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        let total: u64 = st.species.values().sum();
        let target_dex = self.species_of(info.slug.as_deref()).map(|(i, _)| i as u32 + 1);
        let mut rows: Vec<(u32, u64)> = st.species.iter().map(|(d, n)| (*d, *n)).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let right = format!("{} species", rows.len());
        let cols = [BLUE, GREEN, YELLOW, ORANGE, PINK, PURPLE, CYAN, LIME];
        ch::card(ui, "🔎", "What you're running into", Some((&right, MUTED())), |ui| {
            if let Some(td) = target_dex {
                let seen = st.species.get(&td).copied().unwrap_or(0).max(info.target_seen);
                let w = ch::tile_w(ui, 4);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    ch::fact_tile(ui, w, "👀", &theme::thousands(seen), &format!("{} seen", info.name), GOLD);
                    let every = if seen > 0 { info.count as f64 / seen as f64 } else { 0.0 };
                    ch::fact_tile(ui, w, "🎯", &if seen == 0 { "—".into() } else if every < 10.0 { format!("1 in {every:.1}") } else { format!("1 in {}", theme::thousands(every.round() as u64)) }, "encounters is it", GREEN);
                    let since = if seen == 0 { "Not yet".to_string() } else if st.target_last > 0 && info.count > st.target_last { format!("{} ago", theme::thousands(info.count - st.target_last)) } else { "Just now".to_string() };
                    ch::fact_tile(ui, w, "🕑", &since, "Last seen", CYAN);
                    ch::fact_tile(ui, w, "🏜", &if st.target_gap_max > 0 { theme::thousands(st.target_gap_max) } else { "—".into() }, "Longest drought", ORANGE);
                });
                ui.add_space(12.0);
            }
            let segs: Vec<(String, f64, Color32)> = rows
                .iter()
                .take(5)
                .enumerate()
                .map(|(k, (d, n))| (self.species.get(*d as usize - 1).map_or(format!("#{d}"), |s| s.name.to_string()), *n as f64, if Some(*d) == target_dex { GOLD } else { cols[k % cols.len()] }))
                .chain(std::iter::once(("Others".to_string(), rows.iter().skip(5).map(|r| r.1 as f64).sum::<f64>(), MUTED())))
                .collect();
            ch::dist_bar(ui, &segs, 3);
            ui.add_space(8.0);
            let top = rows.first().map_or(1, |r| r.1).max(1);
            for (k, (d, n)) in rows.iter().take(8).enumerate() {
                let sp = self.species.get(*d as usize - 1).cloned();
                let name = sp.as_ref().map_or(format!("#{d}"), |s| s.name.to_string());
                let is_t = Some(*d) == target_dex;
                let col = if is_t { GOLD } else { cols[k % cols.len()] };
                let value = format!("{} · {:.0}%", theme::thousands(*n), *n as f64 * 100.0 / total.max(1) as f64);
                let mut pic = |p: &egui::Painter, r: Rect| {
                    if let Some(sp) = &sp {
                        self.draw_sprite_centered(p, sp, false, r.center(), r.size());
                    }
                };
                ch::hbar_row(ui, &name, &value, *n as f32 / top as f32, col, is_t, &mut pic);
            }
        });
    }

    // ───────────────────────────── fun facts ─────────────────────────────

    fn hunt_facts(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        let rate = st.per_hour();
        let mut facts: Vec<(&str, String, String, Color32)> = Vec::new();
        if let (Some(o), Some(r)) = (info.odds, rate) {
            if !info.caught {
                let next_hour = 1.0 - (1.0 - 1.0 / o).powf(r);
                facts.push(("🎲", pct(next_hour), "shiny chance, next hour".into(), GREEN));
            }
        }
        if let Some(o) = info.odds {
            facts.push(("✨", format!("{:.2}", info.count as f64 / o), "shinies you'd expect by now".into(), GOLD));
        }
        if info.battles > 0 {
            if uses_sweet_scent(&info.method) {
                facts.push(("🌸", theme::thousands(info.battles), "Sweet Scents used".into(), PINK));
            } else {
                facts.push(("⚔", theme::thousands(info.battles), "wild battles".into(), ORANGE));
            }
            facts.push(("👥", format!("{:.2}", info.count as f64 / info.battles as f64), "Pokémon per battle".into(), BLUE));
        }
        if let Some(s) = st.secs_per_encounter() {
            facts.push(("⏱", format!("{s:.1} s"), "your encounter loop".into(), CYAN));
        }
        if let Some((d, n)) = st.species.iter().max_by_key(|(_, n)| **n) {
            if let Some(sp) = self.species.get(*d as usize - 1) {
                let every = info.count as f64 / (*n).max(1) as f64;
                facts.push(("🐾", format!("{} ×{}", sp.name, theme::thousands(*n)), format!("most met: 1 in every {every:.1}"), YELLOW));
            }
        }
        if st.hunt_secs >= 1800 {
            facts.push(("📺", format!("{:.0}", st.hunt_secs as f64 / 1320.0), "anime episodes of hunting time".into(), PURPLE));
        }
        if st.days_hunted() >= 2 {
            let per_day = st.tracked_encounters() / st.days_hunted().max(1) as u64;
            facts.push(("📆", theme::thousands(per_day), "encounters per hunting day".into(), LIME));
        }
        if facts.is_empty() {
            return;
        }
        ch::card(ui, "🎉", "Fun facts", None, |ui| {
            let per = if ui.available_width() >= 900.0 { 4 } else if ui.available_width() >= 560.0 { 2 } else { 1 };
            for row in facts.chunks(per) {
                let w = ch::tile_w(ui, per);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for (i, v, l, c) in row {
                        ch::fact_tile(ui, w, i, v, l, *c);
                    }
                });
                ui.add_space(10.0);
            }
        });
    }
}
