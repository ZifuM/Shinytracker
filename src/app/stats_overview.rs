//! The Stats page's Overview: everything you've hunted, over a time range
//! you pick (7 days, 30 days, 12 months or all time), each number set
//! against the same stretch before it. Activity with the shinies you
//! caught on it, your luck, highlights, when you hunt, the Pokémon you
//! meet, a year of hunting days, your shiny shelf and the milestones ahead.

use super::charts::{self as ch, BLUE, CYAN, GOLD, GREEN, ORANGE, PINK, PURPLE, RED, YELLOW};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::App;
use crate::stats::{self, fmt_day_short, fmt_duration, local_day, now_unix, Session, WEEKDAYS};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use std::collections::BTreeMap;

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// The time range the Overview shows.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum StatsPeriod {
    Week,
    #[default]
    Month,
    Year,
    All,
}

impl StatsPeriod {
    fn days(self) -> Option<i64> {
        match self {
            StatsPeriod::Week => Some(7),
            StatsPeriod::Month => Some(30),
            StatsPeriod::Year => Some(365),
            StatsPeriod::All => None,
        }
    }
    fn phrase(self) -> &'static str {
        match self {
            StatsPeriod::Week => "these 7 days",
            StatsPeriod::Month => "these 30 days",
            StatsPeriod::Year => "these 12 months",
            StatsPeriod::All => "all time",
        }
    }
    fn before(self) -> &'static str {
        match self {
            StatsPeriod::Week => "vs the 7 days before",
            StatsPeriod::Month => "vs the 30 days before",
            StatsPeriod::Year => "vs the year before",
            StatsPeriod::All => "",
        }
    }
}

/// How the Shiny shelf is ordered.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum ShelfSort {
    #[default]
    Recent,
    Luckiest,
    Toughest,
}

/// A caught shiny (phases included, marked).
#[derive(Clone)]
struct Shiny {
    name: String,
    slug: Option<String>,
    encounters: u64,
    odds: Option<f64>,
    when: u64,
    method: String,
    phase: bool,
}

impl Shiny {
    fn mult(&self) -> Option<f64> {
        self.odds.filter(|o| *o > 1.0).map(|o| self.encounters as f64 / o)
    }
    fn found(&self) -> Option<f64> {
        self.odds.filter(|o| *o > 1.0).map(|o| stats::found_by(self.encounters, o))
    }
}

/// Totals for a stretch of days.
#[derive(Default, Clone, Copy)]
struct Sums {
    enc: u64,
    secs: u64,
    days: u64,
    shinies: u64,
}

impl Sums {
    fn pace(&self) -> Option<f64> {
        (self.secs >= 600).then(|| self.enc as f64 / (self.secs as f64 / 3600.0))
    }
}

/// Everything the Overview draws, worked out once per frame.
struct Data {
    today: i64,
    /// First day of the range (inclusive).
    from: i64,
    /// Encounters and seconds per day, every run together.
    days: BTreeMap<i64, (u64, u64)>,
    sessions: Vec<Session>,
    /// Encounters per local hour of the day, over the range.
    clock: [f64; 24],
    /// Encounters per species, every run (national dex → count).
    species: BTreeMap<u32, u64>,
    shinies: Vec<Shiny>,
    now: Sums,
    prev: Option<Sums>,
    /// Lifetime numbers (include encounters from before tracking began).
    life: crate::progress::Lifetime,
}

fn sums(days: &BTreeMap<i64, (u64, u64)>, shinies: &[Shiny], a: i64, b: i64) -> Sums {
    let mut s = Sums::default();
    for (_, (e, t)) in days.range(a..=b) {
        s.enc += e;
        s.secs += t;
        if *e > 0 {
            s.days += 1;
        }
    }
    s.shinies = shinies.iter().filter(|c| (a..=b).contains(&local_day(c.when))).count() as u64;
    s
}

fn month_key(day: i64) -> (i64, u32) {
    let (y, m, _) = stats::civil_from_days(day);
    (y, m)
}

/// Days in a row with encounters, ending at `end` (or the day before, so a
/// streak isn't "broken" just because today hasn't started yet).
fn streak_ending(days: &BTreeMap<i64, (u64, u64)>, end: i64) -> u32 {
    let hunted = |d: i64| days.get(&d).is_some_and(|v| v.0 > 0);
    let mut d = if hunted(end) { end } else { end - 1 };
    let mut n = 0;
    while hunted(d) {
        n += 1;
        d -= 1;
    }
    n
}

fn longest_streak(days: &BTreeMap<i64, (u64, u64)>, a: i64, b: i64) -> u32 {
    let (mut best, mut run, mut last) = (0u32, 0u32, i64::MIN);
    for (d, (e, _)) in days.range(a..=b) {
        if *e == 0 {
            continue;
        }
        run = if *d == last + 1 { run + 1 } else { 1 };
        last = *d;
        best = best.max(run);
    }
    best
}

/// A "▲ 24%" chip: green when up, red when down (for numbers where more is
/// better), muted when flat or there's nothing to compare with.
fn delta(now: f64, before: f64) -> Option<(String, Color32)> {
    if before <= 0.0 {
        return (now > 0.0).then(|| ("▲ new".to_string(), GREEN));
    }
    let d = (now - before) / before;
    if d.abs() < 0.005 {
        return Some(("same".to_string(), MUTED()));
    }
    let pct = (d.abs() * 100.0).round();
    let txt = if pct >= 1000.0 { format!("{}×", ((now / before) * 10.0).round() / 10.0) } else { format!("{pct:.0}%") };
    Some((format!("{} {txt}", if d > 0.0 { "▲" } else { "▼" }), if d > 0.0 { GREEN } else { RED }))
}

/// "vs the 30 days before", or "none the 30 days before" when there's
/// nothing to compare with.
fn before_note(p: StatsPeriod, prev: Option<u64>) -> String {
    match prev {
        Some(0) => p.before().replacen("vs", "none", 1),
        _ => p.before().to_string(),
    }
}

/// Round numbers worth celebrating, the next one above `v`.
fn next_mark(v: f64, marks: &[f64]) -> Option<f64> {
    marks.iter().copied().find(|m| *m > v)
}

impl App {
    /// The Overview tab: pick a range, see everything for it.
    pub(super) fn stats_overview(&mut self, ui: &mut egui::Ui) {
        let d = self.overview_data();
        let wide = ui.available_width() >= 1000.0;
        let gap = 14.0;
        self.so_summary(ui, &d);
        ui.add_space(gap);
        self.so_activity(ui, &d);
        ui.add_space(gap);
        two(ui, wide, gap, |this: &mut App, ui: &mut egui::Ui, col| if col == 0 { this.so_luck(ui, &d) } else { this.so_highlights(ui, &d) }, self);
        ui.add_space(gap);
        two(ui, wide, gap, |this: &mut App, ui: &mut egui::Ui, col| if col == 0 { this.so_when(ui, &d) } else { this.so_species(ui, &d) }, self);
        ui.add_space(gap);
        self.so_year(ui, &d);
        ui.add_space(gap);
        self.so_shelf(ui, &d);
        ui.add_space(gap);
        self.so_milestones(ui, &d);
    }

    fn overview_data(&self) -> Data {
        let today = local_day(now_unix());
        let mut all: Vec<&stats::RunStats> = vec![&self.cfg.run_stats];
        all.extend(self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id).map(|r| &r.stats));
        all.extend(self.cfg.history.iter().filter(|h| !h.phase).filter_map(|h| h.stats.as_ref()));
        let mut days: BTreeMap<i64, (u64, u64)> = BTreeMap::new();
        let mut species: BTreeMap<u32, u64> = BTreeMap::new();
        let mut sessions = Vec::new();
        for st in &all {
            for dd in &st.days {
                let e = days.entry(dd.day).or_insert((0, 0));
                e.0 += dd.encounters;
                e.1 += dd.secs;
            }
            for (k, n) in &st.species {
                *species.entry(*k).or_insert(0) += n;
            }
            sessions.extend(st.sessions.iter().cloned());
        }
        let shinies: Vec<Shiny> = self
            .cfg
            .history
            .iter()
            .map(|h| Shiny { name: h.target.clone(), slug: h.slug.clone(), encounters: h.encounters, odds: h.odds_one_in, when: h.when_unix, method: h.method.clone(), phase: h.phase })
            .collect();
        let period = self.setup.stats_period;
        let first = days.keys().next().copied().unwrap_or(today);
        let from = match period.days() {
            Some(n) => today - n + 1,
            None => first.min(today),
        };
        let now = sums(&days, &shinies, from, today);
        let prev = period.days().map(|n| sums(&days, &shinies, from - n, from - 1));
        // the clock over the range, from each run's hourly counts
        let mut clock = [0f64; 24];
        let (h0, h1) = (from * 24, today * 24 + 23);
        for st in &all {
            for (i, n) in st.hourly(h0, h1).iter().enumerate() {
                clock[((h0 + i as i64).rem_euclid(24)) as usize] += *n as f64;
            }
        }
        Data { today, from, days, sessions, clock, species, shinies, now, prev, life: self.lifetime_full() }
    }

    // ───────────────────────────── summary ─────────────────────────────

    fn so_summary(&mut self, ui: &mut egui::Ui, d: &Data) {
        let period = self.setup.stats_period;
        let all = period == StatsPeriod::All;
        // all time includes encounters from before tracking began
        let enc = if all { d.life.encounters.max(d.now.enc) } else { d.now.enc };
        let secs = if all { d.life.hunt_secs.max(d.now.secs) } else { d.now.secs };
        let shinies = d.shinies.iter().filter(|c| (d.from..=d.today).contains(&local_day(c.when))).count() as u64;
        let shinies = if all { d.shinies.len() as u64 } else { shinies };
        let p = d.prev;
        let metrics: [(&str, String, Option<(String, Color32)>, Color32, String); 5] = [
            ("Encounters", theme::thousands(enc), p.and_then(|p| delta(enc as f64, p.enc as f64)), BLUE, if all { "since you started".into() } else { before_note(period, p.map(|p| p.enc)) }),
            ("Hunt time", fmt_duration(secs), p.and_then(|p| delta(secs as f64, p.secs as f64)), PURPLE, if all { "time spent hunting".into() } else { before_note(period, p.map(|p| p.secs)) }),
            ("Shinies", shinies.to_string(), p.and_then(|p| delta(shinies as f64, p.shinies as f64)), GOLD, {
                let phases = d.shinies.iter().filter(|c| c.phase && (all || (d.from..=d.today).contains(&local_day(c.when)))).count();
                if phases > 0 { format!("{phases} of them phases") } else if all { "caught on your hunts".into() } else { before_note(period, p.map(|p| p.shinies)) }
            }),
            (
                "Pace",
                d.now.pace().map(|v| format!("{}/h", theme::thousands(v.round() as u64))).unwrap_or_else(|| "—".into()),
                p.and_then(|p| Some((d.now.pace()?, p.pace()?))).and_then(|(a, b)| delta(a, b)),
                CYAN,
                "encounters per hour".into(),
            ),
            ("Days hunted", d.now.days.to_string(), p.and_then(|p| delta(d.now.days as f64, p.days as f64)), GREEN, {
                let s = streak_ending(&d.days, d.today);
                if s >= 2 { format!("{s}-day streak going") } else if all { "days with encounters".into() } else { format!("of {}", period.days().unwrap_or(0)) }
            }),
        ];
        let w = ui.available_width();
        let per = if w >= 1000.0 { 5 } else if w >= 640.0 { 3 } else { 2 };
        let gap = 12.0;
        let tw = ((w - gap * (per as f32 - 1.0)) / per as f32).floor();
        for row in metrics.chunks(per) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (label, value, dl, col, sub) in row {
                    let (r, resp) = ui.allocate_exact_size(vec2(tw, 108.0), Sense::hover());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let r = r.translate(vec2(0.0, -2.0 * ht));
                    let p = ui.painter_at(r.expand(14.0));
                    p.add(kit::shadow_tier(if ht > 0.0 { 2 } else { 1 }).as_shape(r, 18.0));
                    p.rect_filled(r, 18.0, kit::CARD());
                    p.rect_stroke(r, 18.0, Stroke::new(1.0f32, kit::LINE()));
                    // a slim stripe of the metric's colour
                    p.rect_filled(Rect::from_min_size(r.min + vec2(16.0, 16.0), vec2(18.0, 4.0)), 2.0, *col);
                    p.text(r.min + vec2(40.0, 18.0), Align2::LEFT_CENTER, label.to_uppercase(), ch::bold(11.0), MUTED());
                    let f = ch::fit_font(ui, value, ch::heavy, 30.0, 16.0, tw - 32.0);
                    p.text(r.min + vec2(16.0, 34.0), Align2::LEFT_TOP, value, f, TEXT());
                    let mut x = r.min.x + 16.0;
                    let y = r.max.y - 20.0;
                    if let Some((t, c)) = dl {
                        let g = p.layout_no_wrap(t.clone(), ch::bold(11.0), ch::ink(*c));
                        let chip = Rect::from_min_size(pos2(x, y - 10.0), vec2(g.size().x + 14.0, 20.0));
                        p.rect_filled(chip, 10.0, ch::faint(*c, 0.16));
                        p.galley(pos2(chip.min.x + 7.0, chip.center().y - g.size().y / 2.0), g, ch::ink(*c));
                        x = chip.max.x + 8.0;
                    }
                    let s = ch::clip(ui, sub, &ch::reg(11.5), r.max.x - x - 12.0);
                    p.text(pos2(x, y), Align2::LEFT_CENTER, s, ch::reg(11.5), MUTED());
                }
            });
            ui.add_space(gap);
        }
    }

    // ───────────────────────────── activity ─────────────────────────────

    fn so_activity(&mut self, ui: &mut egui::Ui, d: &Data) {
        let period = self.setup.stats_period;
        // buckets: days for 7 / 30 days, months for 12 months / all time
        struct B {
            label: String,
            tip: String,
            enc: u64,
            secs: u64,
            shinies: Vec<usize>,
        }
        let mut bs: Vec<B> = Vec::new();
        // all time: days while the history is short, then weeks, then months
        let span = d.today - d.from + 1;
        let gran = match period {
            StatsPeriod::Week | StatsPeriod::Month => 0,
            StatsPeriod::Year => 2,
            StatsPeriod::All if span <= 45 => 0,
            StatsPeriod::All if span <= 26 * 7 => 1,
            StatsPeriod::All => 2,
        };
        let monthly = gran == 2;
        let weekly = gran == 1;
        if weekly {
            // weeks ending on today's weekday
            let n = (span + 6) / 7;
            for k in (0..n).rev() {
                let b = d.today - k * 7;
                let a = (b - 6).max(d.from);
                let s = sums(&d.days, &[], a, b);
                let label = if k % 4 == 0 { fmt_day_short(a) } else { String::new() };
                bs.push(B { label, tip: format!("{} – {}", fmt_day_short(a), fmt_day_short(b)), enc: s.enc, secs: s.secs, shinies: Vec::new() });
            }
        } else if !monthly {
            for day in d.from..=d.today {
                let (e, s) = d.days.get(&day).copied().unwrap_or((0, 0));
                let wd = WEEKDAYS[stats::weekday(day)];
                let label = if span <= 7 {
                    wd[..3.min(wd.len())].to_string()
                } else if (d.today - day) % (if span <= 14 { 2 } else { 5 }) == 0 {
                    fmt_day_short(day)
                } else {
                    String::new()
                };
                bs.push(B { label, tip: format!("{wd} {}", fmt_day_short(day)), enc: e, secs: s, shinies: Vec::new() });
            }
        } else {
            let (mut y, mut m) = month_key(d.from);
            let (ty, tm) = month_key(d.today);
            while (y, m) <= (ty, tm) {
                let a = stats::days_from_civil(y, m, 1);
                let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
                let b = stats::days_from_civil(ny, nm, 1) - 1;
                let s = sums(&d.days, &[], a.max(d.from), b.min(d.today));
                let name = MONTHS[(m - 1) as usize];
                bs.push(B { label: if m == 1 || bs.is_empty() { format!("{name} '{}", y % 100) } else { name.to_string() }, tip: format!("{name} {y}"), enc: s.enc, secs: s.secs, shinies: Vec::new() });
                (y, m) = (ny, nm);
            }
        }
        for (i, c) in d.shinies.iter().enumerate() {
            let day = local_day(c.when);
            if day < d.from || day > d.today {
                continue;
            }
            let k = if monthly {
                let (y, m) = month_key(day);
                let (fy, fm) = month_key(d.from);
                ((y - fy) * 12 + (m as i64 - fm as i64)) as usize
            } else if weekly {
                let n = ((span + 6) / 7) as usize;
                n - 1 - ((d.today - day) / 7) as usize
            } else {
                (day - d.from) as usize
            };
            if let Some(b) = bs.get_mut(k) {
                b.shinies.push(i);
            }
        }
        let total: u64 = bs.iter().map(|b| b.enc).sum();
        let active = bs.iter().filter(|b| b.enc > 0).count().max(1);
        let avg = total as f64 / active as f64;
        let right = if total > 0 { format!("{} encounters", theme::thousands(total)) } else { String::new() };
        let mut open: Option<String> = None;
        ch::card(ui, "📈", "Activity", (!right.is_empty()).then(|| (right.as_str(), BLUE)), |ui| {
            let each = if monthly { "month" } else if weekly { "week" } else { "day" };
            let mut sub = format!("Encounters each {each}. Gold stars mark the shinies you caught.");
            if period == StatsPeriod::All && d.life.encounters > total {
                sub.push_str(&format!(" ({} counted before day-by-day tracking began have no date, so they're not here.)", theme::thousands(d.life.encounters - total)));
            }
            ui.label(egui::RichText::new(sub).size(12.0).color(MUTED()));
            ui.add_space(10.0);
            if total == 0 {
                empty_note(ui, &format!("No encounters in {}. Start the overlay and they'll show up here.", period.phrase()));
                return;
            }
            let h = 210.0;
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
            let p = ui.painter_at(rect.expand(4.0));
            let plot = Rect::from_min_max(rect.min + vec2(44.0, 22.0), rect.max - vec2(6.0, 24.0));
            let max = bs.iter().map(|b| b.enc).max().unwrap_or(1).max(1) as f64;
            // grid: 0, half, max
            for (f, v) in [(0.0, 0.0), (0.5, max / 2.0), (1.0, max)] {
                let y = plot.max.y - plot.height() * f;
                p.line_segment([pos2(plot.min.x, y), pos2(plot.max.x, y)], Stroke::new(1.0f32, ch::faint(kit::LINE(), 0.7)));
                p.text(pos2(plot.min.x - 8.0, y), Align2::RIGHT_CENTER, ch::short_num(v), ch::reg(10.5), MUTED());
            }
            let n = bs.len().max(1);
            let slot = plot.width() / n as f32;
            let bw = (slot * 0.62).clamp(3.0, 46.0);
            let hover = resp.hover_pos().filter(|h| plot.expand2(vec2(0.0, 20.0)).contains(*h)).map(|h| (((h.x - plot.min.x) / slot).floor() as usize).min(n - 1));
            let grow = kit::grow(ui.ctx(), resp.id, 1.0);
            for (i, b) in bs.iter().enumerate() {
                let cx = plot.min.x + slot * (i as f32 + 0.5);
                let hh = (plot.height() * (b.enc as f64 / max) as f32 * grow).max(if b.enc > 0 { 3.0 } else { 0.0 });
                let bar = Rect::from_min_max(pos2(cx - bw / 2.0, plot.max.y - hh), pos2(cx + bw / 2.0, plot.max.y));
                let on = hover == Some(i);
                if on {
                    p.rect_filled(Rect::from_min_max(pos2(cx - slot / 2.0, plot.min.y - 18.0), pos2(cx + slot / 2.0, plot.max.y)), 8.0, ch::faint(TEXT(), 0.05));
                }
                if hh > 0.0 {
                    let c = if on { theme::blend(BLUE, Color32::WHITE, 0.18) } else { BLUE };
                    p.rect_filled(bar, (bw / 2.0).min(6.0), c);
                }
                if !b.shinies.is_empty() {
                    let sy = (plot.max.y - hh - 12.0).max(plot.min.y - 8.0);
                    theme::sparkle(&p, pos2(cx, sy), if bw > 10.0 { 7.0 } else { 5.0 }, GOLD);
                    if b.shinies.len() > 1 {
                        p.text(pos2(cx + 9.0, sy - 6.0), Align2::LEFT_CENTER, format!("{}", b.shinies.len()), ch::bold(10.0), ch::ink(GOLD));
                    }
                }
                if !b.label.is_empty() {
                    p.text(pos2(cx, plot.max.y + 13.0), Align2::CENTER_CENTER, &b.label, ch::reg(10.5), if on { TEXT() } else { MUTED() });
                }
            }
            // average line
            if avg > 0.0 {
                let y = plot.max.y - plot.height() * (avg / max) as f32;
                p.add(egui::Shape::dashed_line(&[pos2(plot.min.x, y), pos2(plot.max.x, y)], Stroke::new(1.2f32, ch::faint(ORANGE, 0.8)), 5.0, 4.0));
                let t = format!("avg {} a {}", theme::thousands(avg.round() as u64), if monthly { "month" } else if weekly { "week" } else { "day" });
                let g = p.layout_no_wrap(t, ch::bold(10.5), ch::ink(ORANGE));
                let chip = Rect::from_min_size(pos2(plot.min.x + 6.0, y - 20.0), vec2(g.size().x + 12.0, 17.0));
                p.rect_filled(chip, 8.5, kit::CARD());
                p.rect_stroke(chip, 8.5, Stroke::new(1.0f32, ch::faint(ORANGE, 0.6)));
                p.galley(pos2(chip.min.x + 6.0, chip.center().y - g.size().y / 2.0), g, ch::ink(ORANGE));
            }
            if let Some(i) = hover {
                let b = &bs[i];
                let mut t = format!("{}\n{} encounters", b.tip, theme::thousands(b.enc));
                if b.secs > 0 {
                    t.push_str(&format!(" · {}", fmt_duration(b.secs)));
                }
                for k in &b.shinies {
                    let c = &d.shinies[*k];
                    t.push_str(&format!("\n✨ {}{}", c.name, if c.phase { " (phase)" } else { "" }));
                }
                if !b.shinies.is_empty() {
                    t.push_str("\nClick to see it");
                }
                resp.clone().on_hover_text_at_pointer(t);
                if resp.clicked() {
                    open = b.shinies.first().and_then(|k| d.shinies[*k].slug.clone());
                }
            }
        });
        if let Some(slug) = open {
            self.open_trophy(slug);
        }
    }

    /// Show one shiny's trophy (in the Pokédex's Shiny Dex view).
    fn open_trophy(&mut self, slug: String) {
        self.setup.tab = super::Tab::Pokedex;
        self.setup.pokedex.shiny = true;
        self.setup.pokedex.open = None;
        self.setup.dex.open = Some(slug);
        self.setup.note_draft.clear();
        self.setup.trophy_back = Some(super::Tab::Stats);
    }

    // ───────────────────────────── luck ─────────────────────────────

    fn so_luck(&mut self, ui: &mut egui::Ui, d: &Data) {
        let period = self.setup.stats_period;
        let in_range: Vec<&Shiny> = d.shinies.iter().filter(|c| !c.phase && c.mult().is_some() && (d.from..=d.today).contains(&local_day(c.when))).collect();
        let all: Vec<&Shiny> = d.shinies.iter().filter(|c| !c.phase && c.mult().is_some()).collect();
        let (list, note) = if !in_range.is_empty() || period == StatsPeriod::All {
            (in_range, None)
        } else {
            (all, Some(format!("No shinies in {}, so this is all time.", period.phrase())))
        };
        let mut open: Option<String> = None;
        let avg_found = (!list.is_empty()).then(|| list.iter().filter_map(|c| c.found()).sum::<f64>() / list.len() as f64);
        let right = avg_found.map(|f| (ch::LUCK_BANDS[ch::luck_band(f)].1.to_string(), ch::luck_col(f)));
        ch::card(ui, "🍀", "Your luck", right.as_ref().map(|(t, c)| (t.as_str(), *c)), |ui| {
            if list.is_empty() {
                empty_note(ui, "Catch a shiny on a hunt and you'll see how lucky you were against its odds.");
                return;
            }
            if let Some(n) = &note {
                ui.label(egui::RichText::new(n).size(12.0).color(MUTED()));
                ui.add_space(4.0);
            }
            let avg_mult = list.iter().filter_map(|c| c.mult()).sum::<f64>() / list.len() as f64;
            let f = avg_found.unwrap_or(0.5);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{avg_mult:.2}×")).font(ch::heavy(34.0)).color(ch::ink(ch::luck_col(f))));
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("the odds, on average").font(ch::bold(13.0)).color(TEXT()));
                    let beat = ((1.0 - f) * 100.0).round();
                    ui.label(egui::RichText::new(format!("Luckier than {beat:.0}% of hunters, on average")).size(12.0).color(MUTED()));
                });
            });
            ui.add_space(10.0);
            // every shiny on one strip from 0× to 3× (or more), on the luck zones
            let hi = list.iter().filter_map(|c| c.mult()).fold(3.0f64, f64::max).ceil();
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::click());
            let p = ui.painter_at(rect.expand(6.0));
            // inset so the pins at either end stay inside the card
            let band = Rect::from_min_max(pos2(rect.min.x + 22.0, rect.min.y + 44.0), pos2(rect.max.x - 22.0, rect.min.y + 58.0));
            let x_of = |m: f64| band.min.x + band.width() * (m / hi).clamp(0.0, 1.0) as f32;
            for (a, b, c) in ch::luck_zones_mult(hi) {
                let r = Rect::from_min_max(pos2(x_of(a), band.min.y), pos2(x_of(b), band.max.y));
                p.rect_filled(r, 0.0, ch::faint(c, 0.75));
            }
            p.rect_stroke(band, 7.0, Stroke::new(1.0f32, kit::LINE()));
            for k in 0..=hi as i32 {
                let x = x_of(k as f64);
                p.text(pos2(x, band.max.y + 12.0), Align2::CENTER_CENTER, format!("{k}×"), ch::reg(10.5), MUTED());
            }
            let hover = resp.hover_pos();
            let mut tip: Option<String> = None;
            for c in &list {
                let m = c.mult().unwrap_or(0.0);
                let x = x_of(m);
                let pin = pos2(x, rect.min.y + 22.0);
                let col = ch::luck_col(c.found().unwrap_or(0.5));
                p.line_segment([pin + vec2(0.0, 14.0), pos2(x, band.min.y)], Stroke::new(1.5f32, col));
                p.circle_filled(pin, 17.0, kit::CARD());
                p.circle_stroke(pin, 17.0, Stroke::new(2.0f32, col));
                if let Some((_, sp)) = self.species_of(c.slug.as_deref()) {
                    self.draw_sprite_centered(&p, &sp, true, pin, vec2(30.0, 30.0));
                }
                if hover.is_some_and(|h| (h - pin).length() < 18.0) {
                    tip = Some(format!("✨ {}\n{} encounters · {:.2}× the odds\n{} · {}\nClick to see it", c.name, theme::thousands(c.encounters), m, c.method, fmt_day_short(local_day(c.when))));
                    if resp.clicked() {
                        open = c.slug.clone();
                    }
                }
            }
            if let Some(t) = tip {
                resp.on_hover_text_at_pointer(t);
            }
            ui.add_space(8.0);
            // the two ends: luckiest and toughest catch
            let by = |a: &&&Shiny, b: &&&Shiny| a.mult().unwrap_or(0.0).total_cmp(&b.mult().unwrap_or(0.0));
            let ends = [("Luckiest", list.iter().min_by(by)), ("Toughest", list.iter().max_by(by))];
            for (label, c) in ends {
                let Some(c) = c else { continue };
                if list.len() < 2 && label == "Toughest" {
                    continue;
                }
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
                let p = ui.painter_at(r.expand(2.0));
                if resp.hovered() {
                    p.rect_filled(r, 12.0, kit::CARD_HOVER());
                }
                let col = ch::luck_col(c.found().unwrap_or(0.5));
                let pc = pos2(r.min.x + 22.0, r.center().y);
                p.circle_filled(pc, 17.0, ch::faint(col, 0.18));
                if let Some((_, sp)) = self.species_of(c.slug.as_deref()) {
                    self.draw_sprite_centered(&p, &sp, true, pc, vec2(30.0, 30.0));
                }
                p.text(pos2(pc.x + 26.0, r.center().y - 7.0), Align2::LEFT_CENTER, format!("{label}: {}", c.name), ch::bold(12.5), TEXT());
                p.text(pos2(pc.x + 26.0, r.center().y + 9.0), Align2::LEFT_CENTER, format!("{} encounters · {}", theme::thousands(c.encounters), fmt_day_short(local_day(c.when))), ch::reg(11.5), MUTED());
                p.text(pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, format!("{:.2}×", c.mult().unwrap_or(0.0)), ch::heavy(15.0), ch::ink(col));
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = c.slug.clone();
                }
            }
            ui.add_space(8.0);
            // how many landed in each band
            let mut counts = [0usize; 5];
            for c in &list {
                counts[ch::luck_band(c.found().unwrap_or(0.5))] += 1;
            }
            ui.horizontal_wrapped(|ui| {
                for (i, (_, name, col)) in ch::LUCK_BANDS.iter().enumerate() {
                    let t = format!("{} {}", counts[i], name.to_lowercase());
                    let g = ui.painter().layout_no_wrap(t.clone(), ch::bold(11.5), ch::ink(*col));
                    let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 20.0, 24.0), Sense::hover());
                    ui.painter().rect_filled(r, 12.0, ch::faint(*col, if counts[i] > 0 { 0.18 } else { 0.06 }));
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, t, ch::bold(11.5), if counts[i] > 0 { ch::ink(*col) } else { MUTED() });
                }
            });
        });
        if let Some(slug) = open {
            self.open_trophy(slug);
        }
    }

    // ───────────────────────────── highlights ─────────────────────────────

    fn so_highlights(&mut self, ui: &mut egui::Ui, d: &Data) {
        let period = self.setup.stats_period;
        let (a, b) = (d.from, d.today);
        let start_unix = |day: i64| (day * 86_400 - stats::utc_offset_secs()).max(0) as u64;
        let sess: Vec<&Session> = d.sessions.iter().filter(|s| s.start >= start_unix(a)).collect();
        let best_day = d.days.range(a..=b).max_by_key(|(_, v)| v.0).filter(|(_, v)| v.0 > 0);
        let longest = sess.iter().max_by_key(|s| s.secs).filter(|s| s.secs > 0);
        let fastest = sess.iter().filter(|s| s.secs >= 1800).map(|s| (s, s.encounters as f64 / (s.secs as f64 / 3600.0))).max_by(|x, y| x.1.total_cmp(&y.1));
        let streak_now = streak_ending(&d.days, d.today);
        let streak_best = longest_streak(&d.days, a, b);
        let busiest = d.clock.iter().enumerate().max_by(|x, y| x.1.total_cmp(y.1)).filter(|(_, v)| **v > 0.0).map(|(h, _)| h);
        let mut wd = [0u64; 7];
        for (day, (e, _)) in d.days.range(a..=b) {
            wd[stats::weekday(*day)] += e;
        }
        let fav = wd.iter().enumerate().max_by_key(|(_, v)| **v).filter(|(_, v)| **v > 0).map(|(i, _)| i);
        let rows: Vec<(&str, Color32, String, String)> = vec![
            ("🏆", YELLOW, "Best day".into(), best_day.map(|(day, v)| format!("{} encounters · {}", theme::thousands(v.0), fmt_day_short(*day))).unwrap_or_else(|| "—".into())),
            ("⏳", PURPLE, "Longest session".into(), longest.map(|s| format!("{} · {} encounters", fmt_duration(s.secs), theme::thousands(s.encounters))).unwrap_or_else(|| "—".into())),
            ("⚡", CYAN, "Fastest session".into(), fastest.map(|(s, r)| format!("{}/h over {}", theme::thousands(r.round() as u64), fmt_duration(s.secs))).unwrap_or_else(|| "— (30 min or more)".into())),
            ("🔥", ORANGE, "Streak".into(), if streak_now >= 2 { format!("{streak_now} days in a row now · best {streak_best}") } else if streak_best == 0 { "—".into() } else { format!("best {streak_best} day{} in a row", if streak_best == 1 { "" } else { "s" }) }),
            ("🕘", PINK, "Busiest hour".into(), busiest.map(|h| format!("{} – {}", stats::fmt_hour(h as i64), stats::fmt_hour(h as i64 + 1))).unwrap_or_else(|| "—".into())),
            ("📅", GREEN, "Favourite day".into(), fav.map(|i| WEEKDAYS[i].to_string()).unwrap_or_else(|| "—".into())),
        ];
        ch::card(ui, "✨", "Highlights", Some((period_word(period), MUTED())), |ui| {
            for (icon, col, title, value) in rows {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
                let p = ui.painter_at(r.expand(2.0));
                if resp.hovered() {
                    p.rect_filled(r, 12.0, kit::CARD_HOVER());
                }
                let chip = Rect::from_center_size(pos2(r.min.x + 22.0, r.center().y), vec2(30.0, 30.0));
                p.rect_filled(chip, 10.0, ch::faint(col, 0.18));
                p.text(chip.center(), Align2::CENTER_CENTER, icon, egui::FontId::proportional(14.0), TEXT());
                p.text(pos2(chip.max.x + 12.0, r.center().y), Align2::LEFT_CENTER, &title, ch::bold(13.0), TEXT());
                let v = ch::clip(ui, &value, &ch::reg(12.5), r.width() - 190.0);
                p.text(pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, v, ch::reg(12.5), if value.starts_with('—') { MUTED() } else { TEXT() });
            }
        });
    }

    // ───────────────────────────── when you hunt ─────────────────────────────

    fn so_when(&mut self, ui: &mut egui::Ui, d: &Data) {
        let mut wd = [(0u64, 0u64); 7];
        for (day, (e, _)) in d.days.range(d.from..=d.today) {
            let k = stats::weekday(*day);
            wd[k].0 += e;
            if *e > 0 {
                wd[k].1 += 1;
            }
        }
        let any = wd.iter().any(|w| w.0 > 0);
        ch::card(ui, "🕘", "When you hunt", Some((period_word(self.setup.stats_period), MUTED())), |ui| {
            if !any {
                empty_note(ui, "Your hunting days and hours show up here once you've counted some encounters.");
                return;
            }
            let w = ui.available_width();
            let side = w >= 460.0;
            let draw_week = |ui: &mut egui::Ui, w: f32| {
                ui.label(egui::RichText::new("Average by day").font(ch::bold(12.5)).color(MUTED()));
                ui.add_space(6.0);
                let (rect, resp) = ui.allocate_exact_size(vec2(w, 230.0), Sense::hover());
                let p = ui.painter_at(rect);
                let avgs: Vec<f64> = wd.iter().map(|(e, n)| if *n > 0 { *e as f64 / *n as f64 } else { 0.0 }).collect();
                let max = avgs.iter().cloned().fold(1.0, f64::max);
                let best = avgs.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(i, _)| i);
                let slot = rect.width() / 7.0;
                let bw = (slot * 0.58).min(30.0);
                let base = rect.max.y - 20.0;
                let top = rect.min.y + 8.0;
                let hover = resp.hover_pos().map(|h| (((h.x - rect.min.x) / slot).floor() as usize).min(6));
                let grow = kit::grow(ui.ctx(), resp.id, 1.0);
                for (i, v) in avgs.iter().enumerate() {
                    let cx = rect.min.x + slot * (i as f32 + 0.5);
                    let track = Rect::from_min_max(pos2(cx - bw / 2.0, top), pos2(cx + bw / 2.0, base));
                    p.rect_filled(track, bw / 2.0, ch::faint(kit::LINE(), 0.6));
                    let hh = (track.height() * (*v / max) as f32 * grow).max(if *v > 0.0 { bw } else { 0.0 });
                    if hh > 0.0 {
                        let c = if Some(i) == best { YELLOW } else { ch::faint(YELLOW, 0.55) };
                        p.rect_filled(Rect::from_min_max(pos2(track.min.x, base - hh), track.max), bw / 2.0, c);
                    }
                    let wd_name = WEEKDAYS[i];
                    p.text(pos2(cx, base + 11.0), Align2::CENTER_CENTER, &wd_name[..3.min(wd_name.len())], ch::reg(10.5), if hover == Some(i) { TEXT() } else { MUTED() });
                }
                if let Some(i) = hover {
                    resp.on_hover_text_at_pointer(format!("{}: {} encounters on an average hunting day\nHunted on {} of them", WEEKDAYS[i], theme::thousands(avgs[i].round() as u64), wd[i].1));
                }
            };
            let peak = d.clock.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(h, _)| h).unwrap_or(0);
            let label = stats::fmt_hour(peak as i64);
            if side {
                let cw = ((w - 16.0) / 2.0).floor();
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| draw_week(ui, cw));
                    ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.label(egui::RichText::new("By hour").font(ch::bold(12.5)).color(MUTED()));
                        ui.add_space(6.0);
                        ui.set_width(cw);
                        ch::clock(ui, &d.clock, PINK, 230.0, (&label, "busiest"));
                    });
                });
            } else {
                draw_week(ui, w);
                ui.add_space(10.0);
                ch::clock(ui, &d.clock, PINK, 190.0, (&label, "busiest"));
            }
        });
    }

    // ───────────────────────────── Pokémon you meet ─────────────────────────────

    fn so_species(&mut self, ui: &mut egui::Ui, d: &Data) {
        let mut list: Vec<(u32, u64)> = d.species.iter().map(|(k, v)| (*k, *v)).filter(|(_, v)| *v > 0).collect();
        list.sort_by(|a, b| b.1.cmp(&a.1));
        let total: u64 = list.iter().map(|x| x.1).sum();
        let right = (!list.is_empty()).then(|| format!("{} species", list.len()));
        ch::card(ui, "🌿", "Pokémon you meet", right.as_deref().map(|t| (t, GREEN)), |ui| {
            if list.is_empty() {
                empty_note(ui, "The Pokémon you run into on your hunts show up here, most met first.");
                return;
            }
            ui.label(egui::RichText::new("Every hunt, all time").size(12.0).color(MUTED()));
            ui.add_space(6.0);
            let top = list[0].1.max(1) as f32;
            let targets: Vec<String> = self.cfg.target.iter().cloned().chain(self.also_slugs()).collect();
            for (dex, n) in list.iter().take(6) {
                let Some(sp) = self.species.get((*dex as usize).wrapping_sub(1)).cloned() else { continue };
                let share = *n as f64 / total.max(1) as f64;
                let hunting = targets.contains(&sp.slug);
                let mut pic = |p: &egui::Painter, r: Rect| {
                    p.circle_filled(r.center(), r.width() / 2.0, ch::faint(GREEN, 0.14));
                    self.draw_sprite_centered(p, &sp, false, r.center(), r.size());
                };
                ch::hbar_row(ui, sp.name, &format!("{} · {:.0}%", ch::short_num(*n as f64), share * 100.0), *n as f32 / top, if hunting { GOLD } else { GREEN }, hunting, &mut pic);
            }
            if list.len() > 6 {
                let rest: u64 = list.iter().skip(6).map(|x| x.1).sum();
                ui.label(egui::RichText::new(format!("+ {} more species · {} encounters", list.len() - 6, theme::thousands(rest))).size(12.0).color(MUTED()));
            }
        });
    }

    // ───────────────────────────── a year of hunting ─────────────────────────────

    fn so_year(&mut self, ui: &mut egui::Ui, d: &Data) {
        let year_from = d.today - 364;
        let hunted = d.days.range(year_from..=d.today).filter(|(_, v)| v.0 > 0).count();
        let best = longest_streak(&d.days, year_from, d.today);
        let right = format!("{hunted} days hunted · best streak {best}");
        ch::card(ui, "📆", "Your year of hunting", Some((&right, MUTED())), |ui| {
            let days = &d.days;
            let shiny_days: std::collections::BTreeMap<i64, Vec<String>> = d.shinies.iter().fold(BTreeMap::new(), |mut m, c| {
                m.entry(local_day(c.when)).or_insert_with(Vec::new).push(c.name.clone());
                m
            });
            ch::heatmap_marked(
                ui,
                d.today,
                53,
                &|day| days.get(&day).map(|v| v.0).unwrap_or(0),
                GREEN,
                &|day, n| {
                    let mut t = format!("{} · {} encounters", fmt_day_short(day), theme::thousands(n));
                    for name in shiny_days.get(&day).into_iter().flatten() {
                        t.push_str(&format!("\n✨ caught {name}"));
                    }
                    t
                },
                Some(&|day| shiny_days.contains_key(&day)),
            );
            ui.add_space(6.0);
            ch::heat_legend(ui, GREEN, Some("a shiny caught"));
        });
    }

    // ───────────────────────────── shiny shelf ─────────────────────────────

    fn so_shelf(&mut self, ui: &mut egui::Ui, d: &Data) {
        let mut sort = self.setup.shelf_sort;
        let mut list: Vec<&Shiny> = d.shinies.iter().collect();
        match sort {
            ShelfSort::Recent => list.sort_by(|a, b| b.when.cmp(&a.when)),
            ShelfSort::Luckiest => list.sort_by(|a, b| a.mult().unwrap_or(f64::MAX).total_cmp(&b.mult().unwrap_or(f64::MAX))),
            ShelfSort::Toughest => list.sort_by(|a, b| b.mult().unwrap_or(-1.0).total_cmp(&a.mult().unwrap_or(-1.0))),
        }
        let mut open: Option<String> = None;
        let mut show_all = self.setup.shelf_all;
        let right = (!list.is_empty()).then(|| format!("{} caught", list.len()));
        ch::card(ui, "⭐", "Shiny shelf", right.as_deref().map(|t| (t, GOLD)), |ui| {
            if list.is_empty() {
                empty_note(ui, "Every shiny you catch gets a spot on this shelf.");
                return;
            }
            kit::segmented(ui, &mut sort, &[(ShelfSort::Recent, "Recent"), (ShelfSort::Luckiest, "Luckiest"), (ShelfSort::Toughest, "Toughest")]);
            ui.add_space(12.0);
            let gap = 12.0;
            let w = ui.available_width();
            let cols = ((w + gap) / (150.0 + gap)).floor().max(2.0) as usize;
            let tw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor();
            let shown = if show_all { list.len() } else { (cols * 2).min(list.len()) };
            for row in list[..shown].chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for c in row {
                        let (r, resp) = ui.allocate_exact_size(vec2(tw, 170.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let r = r.translate(vec2(0.0, -2.0 * ht));
                        let p = ui.painter_at(r.expand(14.0));
                        let col = c.found().map(ch::luck_col).unwrap_or(GOLD);
                        if ht > 0.0 {
                            p.add(kit::shadow_tier(2).as_shape(r, 18.0));
                        }
                        p.rect_filled(r, 18.0, theme::blend(kit::CARD(), GOLD, 0.06 + 0.05 * ht));
                        p.rect_stroke(r, 18.0, Stroke::new(1.0f32, theme::blend(kit::LINE(), GOLD, 0.35 + 0.4 * ht)));
                        let cc = pos2(r.center().x, r.min.y + 56.0);
                        p.circle_filled(cc, 38.0, ch::faint(col, 0.16));
                        if let Some((_, sp)) = self.species_of(c.slug.as_deref()) {
                            self.draw_sprite_centered(&p, &sp, true, cc, vec2(68.0, 64.0));
                        }
                        theme::sparkle(&p, r.right_top() + vec2(-14.0, 14.0), 5.0, GOLD);
                        if c.phase {
                            let g = p.layout_no_wrap("PHASE".into(), ch::bold(9.0), ch::ink(PURPLE));
                            let chip = Rect::from_min_size(r.min + vec2(10.0, 9.0), vec2(g.size().x + 10.0, 16.0));
                            p.rect_filled(chip, 8.0, ch::faint(PURPLE, 0.2));
                            p.galley(pos2(chip.min.x + 5.0, chip.center().y - g.size().y / 2.0), g, ch::ink(PURPLE));
                        }
                        let name = ch::clip(ui, &c.name, &ch::bold(13.5), tw - 16.0);
                        p.text(pos2(r.center().x, r.min.y + 108.0), Align2::CENTER_CENTER, name, ch::bold(13.5), TEXT());
                        p.text(pos2(r.center().x, r.min.y + 127.0), Align2::CENTER_CENTER, format!("{} encounters", ch::short_num(c.encounters as f64)), ch::reg(11.5), MUTED());
                        if let Some(m) = c.mult() {
                            let t = format!("{m:.2}× odds");
                            let g = p.layout_no_wrap(t, ch::bold(10.5), ch::ink(col));
                            let chip = Rect::from_center_size(pos2(r.center().x, r.min.y + 149.0), vec2(g.size().x + 14.0, 18.0));
                            p.rect_filled(chip, 9.0, ch::faint(col, 0.18));
                            p.galley(pos2(chip.min.x + 7.0, chip.center().y - g.size().y / 2.0), g, ch::ink(col));
                        }
                        let tip = format!("✨ {}{}\n{} encounters{}\n{} · {}\nClick for its trophy", c.name, if c.phase { " (phase)" } else { "" }, theme::thousands(c.encounters), c.mult().map(|m| format!(" · {m:.2}× the odds")).unwrap_or_default(), c.method, fmt_day_short(local_day(c.when)));
                        if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = c.slug.clone();
                        }
                    }
                });
                ui.add_space(gap);
            }
            if list.len() > cols * 2 {
                let t = if show_all { "Show fewer".to_string() } else { format!("Show all {}", list.len()) };
                if ui.add(egui::Button::new(t).fill(kit::btn_fill())).clicked() {
                    show_all = !show_all;
                }
            }
        });
        self.setup.shelf_sort = sort;
        self.setup.shelf_all = show_all;
        if let Some(slug) = open {
            self.open_trophy(slug);
        }
    }

    // ───────────────────────────── milestones ahead ─────────────────────────────

    fn so_milestones(&mut self, ui: &mut egui::Ui, d: &Data) {
        let l = &d.life;
        let hunted_days = d.days.values().filter(|v| v.0 > 0).count() as f64;
        let species = d.species.len() as f64;
        let enc_marks = [1_000.0, 5_000.0, 10_000.0, 25_000.0, 50_000.0, 100_000.0, 250_000.0, 500_000.0, 1_000_000.0];
        let shiny_marks = [1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 200.0, 500.0];
        let hour_marks = [10.0, 24.0, 50.0, 100.0, 250.0, 500.0, 1000.0];
        let day_marks = [7.0, 30.0, 50.0, 100.0, 200.0, 365.0, 500.0, 1000.0];
        let sp_marks = [10.0, 25.0, 50.0, 100.0, 150.0, 200.0, 300.0, 400.0];
        let streak_marks = [3.0, 7.0, 14.0, 30.0, 60.0, 100.0, 200.0, 365.0];
        let best_streak = longest_streak(&d.days, i64::MIN, d.today) as f64;
        let hours = l.hunt_secs as f64 / 3600.0;
        // (icon, colour, value now, milestones, what, hours?)
        let rows: Vec<(&str, Color32, f64, &[f64], &str, bool)> = vec![
            ("⚔", BLUE, l.encounters as f64, &enc_marks, "encounters", false),
            ("✨", GOLD, d.shinies.len() as f64, &shiny_marks, "shinies", false),
            ("⏱", PURPLE, hours, &hour_marks, "hours hunting", true),
            ("📅", GREEN, hunted_days, &day_marks, "days hunted", false),
            ("🌿", CYAN, species, &sp_marks, "species met", false),
            ("🔥", ORANGE, best_streak, &streak_marks, "days in a row", false),
        ];
        ch::card(ui, "🏁", "Milestones ahead", Some(("all time", MUTED())), |ui| {
            let w = ui.available_width();
            let cols = if w >= 900.0 { 3 } else if w >= 560.0 { 2 } else { 1 };
            let gap = 12.0;
            let tw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in rows.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (icon, col, v, marks, what, is_hours) in row {
                        let (r, resp) = ui.allocate_exact_size(vec2(tw, 74.0), Sense::hover());
                        let p = ui.painter_at(r.expand(2.0));
                        p.rect_filled(r, 14.0, kit::CARD_HOVER());
                        let chip = Rect::from_min_size(r.min + vec2(12.0, 12.0), vec2(28.0, 28.0));
                        p.rect_filled(chip, 9.0, ch::faint(*col, 0.18));
                        p.text(chip.center(), Align2::CENTER_CENTER, *icon, egui::FontId::proportional(13.0), TEXT());
                        let num = |x: f64| theme::thousands(x.round() as u64);
                        match next_mark(*v, marks) {
                            Some(n) => {
                                // progress from the milestone you last passed to the next one
                                let last = marks.iter().copied().filter(|m| *m <= *v).fold(0.0, f64::max);
                                let frac = ((v - last) / (n - last)).clamp(0.0, 1.0) as f32;
                                let left = if *is_hours { fmt_duration(((n - v) * 3600.0).max(0.0) as u64) } else { num(n - v) };
                                // "1 shiny", not "1 shinies"
                                let what = if n == 1.0 && *what == "shinies" { "shiny" } else { *what };
                                p.text(pos2(chip.max.x + 10.0, chip.center().y - 7.0), Align2::LEFT_CENTER, format!("{} {what}", num(n)), ch::bold(13.0), TEXT());
                                p.text(pos2(chip.max.x + 10.0, chip.center().y + 9.0), Align2::LEFT_CENTER, format!("{left} to go"), ch::reg(11.5), MUTED());
                                let shown = kit::grow(ui.ctx(), resp.id, frac);
                                let track = Rect::from_min_max(pos2(r.min.x + 12.0, r.max.y - 18.0), pos2(r.max.x - 12.0, r.max.y - 10.0));
                                p.rect_filled(track, 4.0, ch::faint(kit::LINE(), 0.9));
                                p.rect_filled(Rect::from_min_size(track.min, vec2((track.width() * shown).max(8.0), track.height())), 4.0, *col);
                                p.text(pos2(r.max.x - 12.0, chip.center().y - 7.0), Align2::RIGHT_CENTER, format!("{:.0}%", frac * 100.0), ch::bold(11.5), ch::ink(*col));
                                if last > 0.0 {
                                    p.text(pos2(r.max.x - 12.0, chip.center().y + 9.0), Align2::RIGHT_CENTER, format!("from {}", num(last)), ch::reg(10.5), MUTED());
                                }
                            }
                            None => {
                                p.text(pos2(chip.max.x + 10.0, chip.center().y), Align2::LEFT_CENTER, format!("{} {what}: every milestone reached", num(*v)), ch::bold(13.0), TEXT());
                            }
                        }
                    }
                });
                ui.add_space(gap);
            }
        });
    }
}

fn period_word(p: StatsPeriod) -> &'static str {
    match p {
        StatsPeriod::Week => "7 days",
        StatsPeriod::Month => "30 days",
        StatsPeriod::Year => "12 months",
        StatsPeriod::All => "all time",
    }
}

fn empty_note(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).size(12.5).color(MUTED()));
    ui.add_space(4.0);
}

/// Two cards side by side on wide windows (equal height), stacked on narrow ones.
fn two(ui: &mut egui::Ui, wide: bool, gap: f32, mut f: impl FnMut(&mut App, &mut egui::Ui, usize), app: &mut App) {
    if wide {
        let w = ((ui.available_width() - gap) / 2.0).floor();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for col in 0..2 {
                ui.allocate_ui_with_layout(vec2(w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(w);
                    f(app, ui, col);
                });
            }
        });
    } else {
        f(app, ui, 0);
        ui.add_space(gap);
        f(app, ui, 1);
    }
}
