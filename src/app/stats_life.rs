//! Lifetime stats on the Profile page, as a dashboard: headline numbers,
//! every shiny's luck on one chart (with the Pokémon under each point),
//! your luck record against the odds, encounters by month, habits, a
//! hunting calendar, top shinies and records with fun facts.

use super::charts::{self as ch, Bars, Line, BLUE, CYAN, GOLD, GREEN, LIME, ORANGE, PINK, PURPLE, RED, YELLOW};
use super::stats_hunt::pct;
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::App;
use crate::stats::{self, fmt_day_short, fmt_duration, local_day, now_unix, RunStats, WEEKDAYS};
use crate::theme;
use eframe::egui::{self, Color32, Rect, Sense};
use std::collections::BTreeMap;

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// The Stats page's tabs.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum StatsTab {
    #[default]
    Lifetime,
    Hunt,
}

/// Which list the "Top shinies" card shows.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum TopView {
    #[default]
    Luckiest,
    Toughest,
    Recent,
}

/// A caught shiny (not a phase) with what's needed for the charts.
struct Catch {
    name: String,
    slug: Option<String>,
    encounters: u64,
    odds: f64,
    when: u64,
    method: String,
}

impl Catch {
    fn mult(&self) -> f64 {
        self.encounters as f64 / self.odds
    }
    fn found(&self) -> f64 {
        stats::found_by(self.encounters, self.odds)
    }
}

/// Encounters as a multiple of the odds where a share `f` of hunters have it.
fn mult_at(f: f64) -> f64 {
    -(1.0 - f.min(0.999)).ln()
}

fn month_of(day: i64) -> (i64, u32) {
    let (y, m, _) = stats::civil_from_days(day);
    (y, m)
}

impl App {
    /// The Stats page: every hunt together, or the current hunt.
    pub(super) fn page_stats(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "STATS", "Your hunting in numbers: luck, pace, habits and records.");
        self.recent_runs(ui);
        ui.add_space(16.0);
        let mut tab = self.setup.stats_tab;
        let current = self.run_info(super::RunRef::Current);
        let hunt_label = match &current {
            Some(i) if !i.name.is_empty() => format!("Current hunt · {}", i.name),
            _ => "Current hunt".to_string(),
        };
        kit::tab_bar(ui, &mut tab, &[(StatsTab::Lifetime, "Lifetime"), (StatsTab::Hunt, hunt_label.as_str())]);
        self.setup.stats_tab = tab;
        ui.add_space(14.0);
        match tab {
            StatsTab::Lifetime => self.lifetime_dashboard(ui),
            StatsTab::Hunt => match current {
                Some(info) => {
                    let st = info.stats.clone().unwrap_or_default();
                    self.hunt_dashboard(ui, &info, &st);
                    ui.add_space(6.0);
                    if ui.link(egui::RichText::new("Older hunts' stats are under Shiny Hunter › Runs.").size(kit::fs(12.0))).clicked() {
                        self.go_to(super::Tab::Runs);
                    }
                }
                None => {
                    ch::card(ui, "🎯", "No hunt yet", None, |ui| ch::empty(ui, "Pick a target under Shiny Hunter › Hunt and its stats show up here."));
                }
            },
        }
    }

    /// The three most recently active runs as cards, each with a button to
    /// its stats.
    fn recent_runs(&mut self, ui: &mut egui::Ui) {
        use super::RunRef;
        let mut refs: Vec<(u64, RunRef)> = Vec::new();
        if self.cfg.target.is_some() || self.cfg.count > 0 {
            let t = if self.cfg.run_stats.last_unix > 0 { self.cfg.run_stats.last_unix } else { now_unix() };
            refs.push((t, RunRef::Current));
        }
        for r in self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id) {
            refs.push((r.stats.last_unix.max(r.saved_unix), RunRef::Saved(r.id)));
        }
        for (i, h) in self.cfg.history.iter().enumerate().filter(|(_, h)| !h.phase) {
            refs.push((h.when_unix, RunRef::Finished(i)));
        }
        refs.sort_by_key(|r| std::cmp::Reverse(r.0));
        let runs: Vec<(u64, RunRef, super::hunt_pages::RunInfo)> = refs.into_iter().filter_map(|(t, r)| self.run_info(r).map(|i| (t, r, i))).take(3).collect();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Recent runs").font(ch::bold(16.0)).color(TEXT()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.link(egui::RichText::new("All runs ›").size(kit::fs(12.5))).clicked() {
                    self.go_to(super::Tab::Runs);
                }
            });
        });
        ui.add_space(8.0);
        if runs.is_empty() {
            kit::card(ui, |ui| ch::empty(ui, "No runs yet. Pick a target under Shiny Hunter › Hunt to start one."));
            return;
        }
        let per = if ui.available_width() >= 760.0 { 3 } else { 1 };
        let mut open: Option<RunRef> = None;
        for row in runs.chunks(per) {
            let w = ch::tile_w(ui, per);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for (t, r, info) in row {
                    if self.run_card(ui, w, *t, info) {
                        open = Some(*r);
                    }
                }
            });
            ui.add_space(12.0);
        }
        if let Some(r) = open {
            self.go_to(super::Tab::Runs);
            self.setup.view_run = Some(r);
        }
    }

    /// One recent run. Returns true when its stats button was clicked.
    fn run_card(&mut self, ui: &mut egui::Ui, w: f32, last: u64, info: &super::hunt_pages::RunInfo) -> bool {
        let h = 168.0;
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
        let p = ui.painter_at(rect.expand(12.0));
        let found = info.odds.map(|o| stats::found_by(info.count, o));
        let col = found.map_or(PURPLE, ch::luck_col);
        p.add(kit::card_shadow().as_shape(rect, 18.0));
        p.rect_filled(rect, 18.0, kit::CARD());
        p.rect_stroke(rect, 18.0, egui::Stroke::new(1.0f32, if resp.hovered() { ch::faint(col, 0.8) } else { kit::LINE() }));
        // sprite in a soft circle of the luck colour
        let sc = egui::pos2(rect.min.x + 58.0, rect.min.y + 62.0);
        p.circle_filled(sc, 42.0, ch::faint(col, 0.14));
        if let Some((_, sp)) = self.species_of(info.slug.as_deref()) {
            self.draw_sprite_centered(&p, &sp, true, sc, egui::vec2(76.0, 76.0));
        }
        if info.caught {
            theme::sparkle(&p, sc + egui::vec2(32.0, -30.0), 7.0, GOLD);
        }
        let x = rect.min.x + 116.0;
        let room = rect.max.x - 16.0 - x;
        let (st_t, st_c) = if info.caught {
            ("★ CAUGHT", GOLD)
        } else if info.current {
            ("● HUNTING", GREEN)
        } else {
            ("SAVED", MUTED())
        };
        let g = p.layout_no_wrap(st_t.to_string(), ch::bold(10.5), ch::ink(st_c));
        let cw = g.size().x + 14.0;
        p.rect_filled(Rect::from_min_size(egui::pos2(x, rect.min.y + 18.0), egui::vec2(cw, 20.0)), 10.0, ch::faint(st_c, 0.18));
        p.galley(egui::pos2(x + 7.0, rect.min.y + 21.0), g, ch::ink(st_c));
        let name = if info.name.is_empty() { "No target".to_string() } else { info.name.clone() };
        p.text(egui::pos2(x, rect.min.y + 46.0), egui::Align2::LEFT_TOP, ch::clip(ui, &name, &ch::bold(18.0), room), ch::bold(18.0), TEXT());
        let num = theme::thousands(info.count);
        let nf = ch::fit_font(ui, &num, ch::heavy, 28.0, 16.0, room * 0.6);
        let nr = p.text(egui::pos2(x, rect.min.y + 72.0), egui::Align2::LEFT_TOP, &num, nf, ch::ink(col));
        if let Some(o) = info.odds {
            p.text(egui::pos2(nr.max.x + 8.0, nr.max.y - 6.0), egui::Align2::LEFT_BOTTOM, format!("{:.2}×", info.count as f64 / o), ch::bold(13.0), ch::ink(col));
        }
        let ago = if info.current && now_unix().saturating_sub(last) < 1800 { "active now".to_string() } else { super::days_ago(last) };
        p.text(egui::pos2(x, rect.min.y + 112.0), egui::Align2::LEFT_CENTER, ch::clip(ui, &format!("{} · {ago}", if info.method.is_empty() { "—" } else { &info.method }), &ch::reg(11.5), room), ch::reg(11.5), MUTED());
        // the stats button
        let br = Rect::from_min_max(egui::pos2(rect.min.x + 16.0, rect.max.y - 40.0), egui::pos2(rect.max.x - 16.0, rect.max.y - 12.0));
        let bresp = ui.interact(br, ui.id().with(("run-card", info.run_id, info.started, info.name.as_str())), Sense::click());
        let fill = if bresp.hovered() { ch::faint(col, 0.32) } else { ch::faint(col, 0.18) };
        p.rect_filled(br, 10.0, fill);
        p.text(br.center(), egui::Align2::CENTER_CENTER, "📊  View stats  ›", ch::bold(13.0), ch::ink(col));
        (bresp.clicked() || resp.clicked()) && (bresp.clicked() || !bresp.hovered())
    }

    pub(super) fn lifetime_dashboard(&mut self, ui: &mut egui::Ui) {
        let l = self.lifetime_full();
        // every run's stats once (current, other saved runs, caught hunts)
        let mut all: Vec<RunStats> = vec![self.cfg.run_stats.clone()];
        all.extend(self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id).map(|r| r.stats.clone()));
        all.extend(self.cfg.history.iter().filter(|h| !h.phase).filter_map(|h| h.stats.clone()));
        let mut days: BTreeMap<i64, (u64, u64)> = BTreeMap::new();
        let mut species: BTreeMap<u32, u64> = BTreeMap::new();
        let mut clock = [0f64; 24];
        let mut fastest: Option<f64> = None;
        for st in &all {
            for d in &st.days {
                let e = days.entry(d.day).or_insert((0, 0));
                e.0 += d.encounters;
                e.1 += d.secs;
            }
            for (d, n) in &st.species {
                *species.entry(*d).or_insert(0) += n;
            }
            for (i, s) in st.by_hour_of_day().iter().enumerate() {
                clock[i] += s.encounters;
            }
            if let Some((_, r)) = st.fastest_session() {
                fastest = Some(fastest.map_or(r, |f: f64| f.max(r)));
            }
        }
        let mut caught: Vec<Catch> = self
            .cfg
            .history
            .iter()
            .filter(|h| !h.phase && h.encounters > 0)
            .filter_map(|h| {
                Some(Catch { name: h.target.clone(), slug: h.slug.clone(), encounters: h.encounters, odds: h.odds_one_in.filter(|o| *o > 1.0)?, when: h.when_unix, method: h.method.clone() })
            })
            .collect();
        caught.sort_by_key(|c| c.when);

        self.life_hero(ui, &l, &days, &caught);
        ui.add_space(12.0);
        if !caught.is_empty() {
            self.life_luck_chart(ui, &caught);
            ui.add_space(12.0);
            self.life_luck_record(ui, &caught);
            ui.add_space(12.0);
        }
        if !days.is_empty() {
            self.life_months(ui, &days);
            ui.add_space(12.0);
            let wide = ui.available_width() >= 900.0;
            if wide {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    ui.columns(2, |cols| {
                        life_weekdays(&mut cols[0], &days);
                        life_clock(&mut cols[1], &clock);
                    });
                });
            } else {
                life_weekdays(ui, &days);
                ui.add_space(12.0);
                life_clock(ui, &clock);
            }
            ui.add_space(12.0);
            life_calendar(ui, &days);
            ui.add_space(12.0);
        }
        if !caught.is_empty() {
            self.life_top(ui, &caught);
            ui.add_space(12.0);
        }
        self.life_records(ui, &l, &days, &species, &caught, fastest);
    }

    // ───────────────────────────── headline numbers ─────────────────────────────

    fn life_hero(&mut self, ui: &mut egui::Ui, l: &crate::progress::Lifetime, days: &BTreeMap<i64, (u64, u64)>, caught: &[Catch]) {
        let today = local_day(now_unix());
        let (ty, tm) = month_of(today);
        let this_month: u64 = days.iter().filter(|(d, _)| month_of(**d) == (ty, tm)).map(|(_, v)| v.0).sum();
        let month_secs: u64 = days.iter().filter(|(d, _)| month_of(**d) == (ty, tm)).map(|(_, v)| v.1).sum();
        let shinies_year = caught.iter().filter(|c| month_of(local_day(c.when)).0 == ty).count();
        let avg = if l.shinies > 0 { l.encounters / l.shinies } else { 0 };
        let items: [(&str, String, &str, String, Color32); 4] = [
            ("⚔", theme::thousands(l.encounters), "Encounters", format!("{} this month", theme::thousands(this_month)), BLUE),
            ("⏱", fmt_duration(l.hunt_secs), "Hunt time", format!("{} this month", fmt_duration(month_secs)), PURPLE),
            ("✨", l.shinies.to_string(), "Shinies", format!("{shinies_year} this year"), GOLD),
            ("📊", if avg > 0 { theme::thousands(avg) } else { "—".into() }, "Per shiny", "encounters on average".into(), GREEN),
        ];
        let per = if ui.available_width() >= 900.0 { 4 } else { 2 };
        for row in items.chunks(per) {
            let w = ch::tile_w(ui, per);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for (icon, value, label, note, col) in row {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 116.0), Sense::hover());
                    let p = ui.painter_at(rect.expand(12.0));
                    p.add(kit::card_shadow().as_shape(rect, 18.0));
                    p.rect_filled(rect, 18.0, kit::CARD());
                    p.rect_stroke(rect, 18.0, egui::Stroke::new(1.0f32, kit::LINE()));
                    p.circle_filled(rect.right_top() + egui::vec2(-30.0, 30.0), 18.0, ch::faint(*col, 0.18));
                    p.text(rect.right_top() + egui::vec2(-30.0, 30.0), egui::Align2::CENTER_CENTER, *icon, egui::FontId::proportional(17.0), TEXT());
                    p.text(rect.min + egui::vec2(18.0, 18.0), egui::Align2::LEFT_TOP, label.to_uppercase(), ch::bold(11.5), MUTED());
                    let f = ch::fit_font(ui, value, ch::heavy, 36.0, 18.0, w - 36.0);
                    p.text(rect.min + egui::vec2(16.0, 38.0), egui::Align2::LEFT_TOP, value, f, ch::ink(*col));
                    p.text(rect.min + egui::vec2(18.0, 92.0), egui::Align2::LEFT_CENTER, note, ch::reg(12.0), MUTED());
                }
            });
            ui.add_space(12.0);
        }
    }

    // ───────────────────────────── every shiny's luck ─────────────────────────────

    fn life_luck_chart(&mut self, ui: &mut egui::Ui, caught: &[Catch]) {
        let shown: Vec<&Catch> = caught.iter().rev().take(14).rev().collect();
        let vals: Vec<f64> = shown.iter().map(|c| c.mult()).collect();
        let hi = (vals.iter().cloned().fold(0.0, f64::max) * 1.12).max(3.0).ceil();
        // trend: are later shinies luckier (lower multiples)?
        let right = ch::fit_line(&vals).and_then(|(b, _)| {
            if shown.len() < 3 {
                None
            } else if b < -0.02 {
                Some(("↗ Getting luckier".to_string(), GREEN))
            } else if b > 0.02 {
                Some(("↘ Tougher lately".to_string(), ORANGE))
            } else {
                Some(("→ Steady".to_string(), MUTED()))
            }
        });
        let bands: Vec<(f64, f64, Color32)> = {
            let mut lo = 0.0;
            ch::LUCK_BANDS
                .iter()
                .map(|(f, _, c)| {
                    let top = if *f >= 1.0 { hi } else { mult_at(*f) };
                    let r = (lo, top, *c);
                    lo = top;
                    r
                })
                .collect()
        };
        let ticks: Vec<(f64, String)> = (0..=hi as u32).map(|k| (k as f64, format!("{k}×"))).collect();
        let labels: Vec<String> = shown.iter().map(|c| MONTHS[(month_of(local_day(c.when)).1 as usize + 11) % 12].to_string()).collect();
        let hovers: Vec<String> = shown
            .iter()
            .map(|c| format!("{} · {}\n{} encounters at 1 in {} ({:.2}× the odds)\nLuckier than {} of hunters", c.name, fmt_day_short(local_day(c.when)), theme::thousands(c.encounters), theme::thousands(c.odds.round() as u64), c.mult(), pct(1.0 - c.found())))
            .collect();
        let found: Vec<f64> = shown.iter().map(|c| c.found()).collect();
        let sprites: Vec<Option<crate::species::Species>> = shown.iter().map(|c| self.species_of(c.slug.as_deref()).map(|(_, s)| s)).collect();
        ch::card(ui, "🍀", "Shiny luck", right.as_ref().map(|(t, c)| (t.as_str(), *c)), |ui| {
            ui.label(egui::RichText::new("Each shiny: how many encounters it took, as a multiple of its odds. Lower is luckier.").size(kit::fs(12.5)).color(MUTED()));
            ui.add_space(6.0);
            let mut pic = |p: &egui::Painter, i: usize, at: egui::Pos2| {
                if let Some(Some(sp)) = sprites.get(i) {
                    self.draw_sprite_centered(p, sp, true, at, egui::vec2(40.0, 40.0));
                }
            };
            ch::line_chart(
                ui,
                &mut Line {
                    values: &vals,
                    labels: &labels,
                    bands: &bands,
                    lo: 0.0,
                    hi,
                    ticks: &ticks,
                    col: &|i, _| ch::luck_col(found[i]),
                    trend: shown.len() >= 3,
                    hovers: &hovers,
                    height: 300.0,
                    area: None,
                    stars: &[],
                    you: None,
                    pic: Some(&mut pic),
                    dots: true,
                    line: Some(ch::faint(TEXT(), 0.45)),
                },
            );
            ch::legend(ui, &ch::LUCK_BANDS.map(|b| (b.2, b.1)));
        });
    }

    fn life_luck_record(&mut self, ui: &mut egui::Ui, caught: &[Catch]) {
        let mut counts = [0f64; 5];
        for c in caught {
            counts[ch::luck_band(c.found())] += 1.0;
        }
        let expected: f64 = caught.iter().map(|c| c.mult()).sum();
        let n = caught.len() as f64;
        let diff = n - expected;
        let avg_beat: f64 = caught.iter().map(|c| 1.0 - c.found()).sum::<f64>() / n.max(1.0);
        let right = if diff >= 0.0 { (format!("+{diff:.1} ahead of the odds"), GREEN) } else { (format!("{diff:.1} behind the odds"), ORANGE) };
        ch::card(ui, "🎯", "Your luck record", Some((&right.0, right.1)), |ui| {
            let w = ch::tile_w(ui, 3);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                ch::fact_tile(ui, w, "✨", &format!("{n:.0}"), "shinies found", GOLD);
                ch::fact_tile(ui, w, "🎲", &format!("{expected:.1}"), "you'd expect for those encounters", CYAN);
                ch::fact_tile(ui, w, "🍀", &pct(avg_beat), "of hunters beaten, on average", ch::luck_col(1.0 - avg_beat));
            });
            ui.add_space(14.0);
            let segs: Vec<(String, f64, Color32)> = ch::LUCK_BANDS.iter().enumerate().map(|(i, b)| (b.1.to_string(), counts[i], b.2)).collect();
            ch::dist_bar(ui, &segs, 3);
        });
    }

    // ───────────────────────────── by month ─────────────────────────────

    fn life_months(&mut self, ui: &mut egui::Ui, days: &BTreeMap<i64, (u64, u64)>) {
        let today = local_day(now_unix());
        let (ty, tm) = month_of(today);
        let months: Vec<(i64, u32)> = (0..12)
            .rev()
            .map(|k| {
                let idx = ty * 12 + tm as i64 - 1 - k;
                (idx.div_euclid(12), (idx.rem_euclid(12) + 1) as u32)
            })
            .collect();
        let vals: Vec<f64> = months.iter().map(|ym| days.iter().filter(|(d, _)| month_of(**d) == *ym).map(|(_, v)| v.0 as f64).sum()).collect();
        let shiny_m: Vec<usize> = months
            .iter()
            .enumerate()
            .filter(|(_, ym)| self.cfg.history.iter().any(|h| !h.phase && h.when_unix > 0 && month_of(local_day(h.when_unix)) == **ym))
            .map(|(i, _)| i)
            .collect();
        let labels: Vec<String> = months.iter().map(|(_, m)| MONTHS[*m as usize - 1].to_string()).collect();
        let hovers: Vec<String> = months
            .iter()
            .zip(&vals)
            .map(|((y, m), v)| {
                let sh = self.cfg.history.iter().filter(|h| !h.phase && h.when_unix > 0 && month_of(local_day(h.when_unix)) == (*y, *m)).map(|h| h.target.clone()).collect::<Vec<_>>();
                format!("{} {y}: {} encounters{}", MONTHS[*m as usize - 1], theme::thousands(*v as u64), if sh.is_empty() { String::new() } else { format!("\n✨ {}", sh.join(", ")) })
            })
            .collect();
        let max = vals.iter().cloned().fold(0.0, f64::max).max(1.0);
        let hi = {
            let e = 10f64.powf(max.log10().floor());
            (max / e * 1.15).ceil() * e
        };
        let ticks: Vec<(f64, String)> = (0..=2).map(|k| (hi * k as f64 / 2.0, ch::short_num(hi * k as f64 / 2.0))).collect();
        let right = ch::fit_line(&vals).map(|(b, _)| {
            let avg = vals.iter().sum::<f64>() / vals.len() as f64;
            let r = if avg > 0.0 { b / avg * 100.0 } else { 0.0 };
            (format!("{} {:.0}%/month", if r >= 0.0 { "↑" } else { "↓" }, r.abs()), if r >= 0.0 { GREEN } else { RED })
        });
        ch::card(ui, "📈", "Encounters by month", right.as_ref().map(|(t, c)| (t.as_str(), *c)), |ui| {
            ch::line_chart(
                ui,
                &mut Line {
                    values: &vals,
                    labels: &labels,
                    bands: &[],
                    lo: 0.0,
                    hi,
                    ticks: &ticks,
                    col: &|_, _| BLUE,
                    trend: true,
                    hovers: &hovers,
                    height: 220.0,
                    area: Some(BLUE),
                    stars: &shiny_m,
                    you: None,
                    pic: None,
                    dots: true,
                    line: Some(BLUE),
                },
            );
            ch::legend(ui, &[(BLUE, "Encounters"), (GOLD, "Month with a shiny"), (RED, "Trend")]);
        });
    }

    // ───────────────────────────── top shinies ─────────────────────────────

    fn life_top(&mut self, ui: &mut egui::Ui, caught: &[Catch]) {
        let mut view = self.setup.top_view;
        let mut list: Vec<&Catch> = caught.iter().collect();
        match view {
            TopView::Luckiest => list.sort_by(|a, b| a.mult().total_cmp(&b.mult())),
            TopView::Toughest => list.sort_by(|a, b| b.mult().total_cmp(&a.mult())),
            TopView::Recent => list.sort_by(|a, b| b.when.cmp(&a.when)),
        }
        ch::card(ui, "⭐", "Top shinies", None, |ui| {
            kit::segmented(ui, &mut view, &[(TopView::Luckiest, " Luckiest "), (TopView::Toughest, " Toughest "), (TopView::Recent, " Recent ")]);
            ui.add_space(10.0);
            for (k, c) in list.iter().take(5).enumerate() {
                let sp = self.species_of(c.slug.as_deref()).map(|(_, s)| s);
                let col = ch::luck_col(c.found());
                let sub = format!("{} encounters  ·  {}  ·  {}", theme::thousands(c.encounters), c.method, fmt_day_short(local_day(c.when)));
                let mut pic = |p: &egui::Painter, r: Rect| {
                    p.circle_filled(r.center(), r.width() / 2.0, ch::faint(col, 0.15));
                    if let Some(sp) = &sp {
                        self.draw_sprite_centered(p, sp, true, r.center(), r.size());
                    }
                };
                ch::rank_row(ui, k + 1, &c.name, &sub, &format!("{:.2}×", c.mult()), &format!("beat {}", pct(1.0 - c.found())), col, &mut pic);
                ui.add_space(8.0);
            }
        });
        self.setup.top_view = view;
    }

    // ───────────────────────────── records and fun facts ─────────────────────────────

    fn life_records(&mut self, ui: &mut egui::Ui, l: &crate::progress::Lifetime, days: &BTreeMap<i64, (u64, u64)>, species: &BTreeMap<u32, u64>, caught: &[Catch], fastest: Option<f64>) {
        let mut facts: Vec<(&str, String, String, Color32)> = Vec::new();
        if let Some((d, (n, _))) = days.iter().max_by_key(|(_, v)| v.0) {
            facts.push(("🏆", theme::thousands(*n), format!("best day · {}", fmt_day_short(*d)), GOLD));
        }
        facts.push(("🔥", format!("{} days", l.longest_streak), "longest streak".into(), ORANGE));
        if let Some(r) = fastest {
            facts.push(("🚀", format!("{}/h", theme::thousands(r.round() as u64)), "fastest session".into(), CYAN));
        }
        let mut whens: Vec<u64> = caught.iter().map(|c| c.when).filter(|w| *w > 0).collect();
        whens.sort_unstable();
        if let Some(gap) = whens.windows(2).map(|w| w[1] - w[0]).max() {
            facts.push(("⏳", if gap >= 86_400 { format!("{} days", gap / 86_400) } else { fmt_duration(gap) }, "longest wait between shinies".into(), PURPLE));
        }
        if whens.len() >= 2 {
            let span = (whens[whens.len() - 1] - whens[0]) as f64 / 86_400.0;
            facts.push(("📅", format!("{:.0} days", span / (whens.len() - 1) as f64), "between shinies on average".into(), LIME));
        }
        if let Some((d, n)) = species.iter().max_by_key(|(_, n)| **n) {
            if let Some(sp) = self.species.get(*d as usize - 1) {
                facts.push(("🐾", format!("{} ×{}", sp.name, theme::thousands(*n)), "your most-met Pokémon".into(), YELLOW));
            }
        }
        if !species.is_empty() {
            facts.push(("🔎", species.len().to_string(), "species encountered".into(), GREEN));
        }
        if l.battles > 0 {
            facts.push(("⚔", theme::thousands(l.battles), "wild battles".into(), RED));
        }
        if l.hunt_secs >= 1800 {
            facts.push(("📺", format!("{:.0}", l.hunt_secs as f64 / 1320.0), "anime episodes of hunting".into(), PINK));
        }
        if let Some(best) = caught.iter().min_by(|a, b| a.mult().total_cmp(&b.mult())) {
            facts.push(("🍀", best.name.clone(), format!("luckiest: {:.2}× the odds", best.mult()), BLUE));
        }
        let hunted = days.values().filter(|v| v.0 > 0).count() as u64;
        if hunted > 0 {
            facts.push(("📆", theme::thousands(l.encounters / hunted.max(1)), "encounters per hunting day".into(), CYAN));
        }
        if l.targets > 0 {
            facts.push(("🎯", l.targets.to_string(), "different Pokémon hunted".into(), GREEN));
        }
        ch::card(ui, "🏅", "Records & fun facts", None, |ui| {
            let per = if ui.available_width() >= 900.0 { 4 } else if ui.available_width() >= 560.0 { 2 } else { 1 };
            for row in facts.chunks(per) {
                let w = ch::tile_w(ui, per);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for (i, v, lb, c) in row {
                        ch::fact_tile(ui, w, i, v, lb, *c);
                    }
                });
                ui.add_space(10.0);
            }
        });
    }
}

fn life_weekdays(ui: &mut egui::Ui, days: &BTreeMap<i64, (u64, u64)>) {
    let mut sum = [0u64; 7];
    let mut n = [0u64; 7];
    for (d, (e, _)) in days {
        if *e > 0 {
            let w = stats::weekday(*d);
            sum[w] += e;
            n[w] += 1;
        }
    }
    let avg: Vec<f64> = (0..7).map(|w| if n[w] > 0 { sum[w] as f64 / n[w] as f64 } else { 0.0 }).collect();
    let best = avg.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).filter(|(_, v)| **v > 0.0).map(|(i, _)| i);
    let right = best.map(|b| format!("Best: {}", WEEKDAYS[b]));
    ch::card(ui, "📅", "Average by day", right.as_deref().map(|r| (r, YELLOW)), |ui| {
        let labels: Vec<String> = WEEKDAYS.iter().map(|s| s.to_string()).collect();
        let subs: Vec<String> = n.iter().map(|k| if *k > 0 { format!("{k}×") } else { "–".into() }).collect();
        let hovers: Vec<String> = (0..7).map(|w| format!("{}: {} encounters on average over {} day{}", WEEKDAYS[w], theme::thousands(avg[w].round() as u64), n[w], if n[w] == 1 { "" } else { "s" })).collect();
        let hot = |i: usize| Some(i) == best;
        ch::bars(ui, &Bars { values: &avg, labels: &labels, col: YELLOW, hot: Some(&hot), hovers: &hovers, tops: true, axis: false, height: 210.0, cols: None, subs: &subs });
        ch::empty(ui, "Encounters on an average hunting day, across every hunt.");
    });
}

fn life_clock(ui: &mut egui::Ui, clock: &[f64; 24]) {
    let best = clock.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).filter(|(_, v)| **v > 0.0).map(|(i, _)| i);
    // night owl or early bird?
    let night: f64 = (0..6).chain(22..24).map(|h| clock[h]).sum();
    let morning: f64 = (5..11).map(|h| clock[h]).sum();
    let total: f64 = clock.iter().sum::<f64>().max(1.0);
    let kind = if night / total > 0.3 {
        ("🦉 Night owl", PURPLE)
    } else if morning / total > 0.3 {
        ("🐦 Early bird", YELLOW)
    } else {
        ("☀ Day hunter", ORANGE)
    };
    ch::card(ui, "🕘", "Hunt clock", Some(kind), |ui| {
        let centre = best.map_or(("—".to_string(), String::new()), |b| (stats::fmt_hour(b as i64), "busiest hour".into()));
        ch::clock(ui, clock, PINK, 236.0, (&centre.0, &centre.1));
        ch::empty(ui, "When your encounters happen, across every hunt.");
    });
}

fn life_calendar(ui: &mut egui::Ui, days: &BTreeMap<i64, (u64, u64)>) {
    let today = local_day(now_unix());
    let hunted = days.iter().filter(|(d, v)| v.0 > 0 && **d > today - 182).count();
    let right = format!("{hunted} days in 6 months");
    ch::card(ui, "📆", "Hunting calendar", Some((&right, MUTED())), |ui| {
        ch::heatmap(ui, today, 26, &|d| days.get(&d).map_or(0, |v| v.0), GREEN, &|d, v| {
            if v == 0 {
                format!("{}: no hunting", fmt_day_short(d))
            } else {
                format!("{}: {} encounters · {}", fmt_day_short(d), theme::thousands(v), fmt_duration(days.get(&d).map_or(0, |x| x.1)))
            }
        });
    });
}
