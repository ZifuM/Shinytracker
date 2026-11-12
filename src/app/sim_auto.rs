//! Battle Sim, stage 7: the auto-sim. The battle that's set up, played out
//! hundreds or thousands of times with the AI on both sides (in the
//! background, over every CPU core), then added up: your win rate and how
//! sure it is, how long battles last, and what each Pokémon on both sides
//! tends to do.

use super::sim_home::{thousands, OURS, RED};
use super::team_home::{caps, chip, small, soft};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::App;
use crate::engine::{self, MonReport, Report};
use crate::sim::{Ai, Format};
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Instant;

/// An auto-sim running in the background.
pub(super) struct AutoRun {
    progress: Arc<AtomicU32>,
    cancel: Arc<AtomicBool>,
    n: u32,
    rx: mpsc::Receiver<Report>,
    started: Instant,
    label: String,
    ai: Ai,
}

/// The last auto-sim's results.
pub(super) struct AutoDone {
    report: Report,
    label: String,
    ai: Ai,
    secs: f32,
    stopped: bool,
}

const COUNTS: [u32; 3] = [100, 1000, 5000];

pub(super) fn auto_count(n: u32) -> u32 {
    if n == 0 {
        1000
    } else {
        n
    }
}

fn pct(x: f64) -> String {
    let v = x * 100.0;
    if v >= 99.95 || v <= 0.05 {
        format!("{:.0}%", v)
    } else {
        format!("{:.1}%", v)
    }
}

impl App {
    /// Start the auto-sim for the battle that's set up.
    pub(super) fn sim_auto_start(&mut self) {
        if self.setup.sim.auto.is_some() {
            return;
        }
        let built = match self.sim_build() {
            Ok(b) => b,
            Err(e) => {
                self.setup.sim.note = Some(e);
                return;
            }
        };
        let n = auto_count(self.setup.sim.auto_n);
        let ai = self.setup.sim.auto_ai.unwrap_or(Ai::Expert);
        let format = if self.setup.sim.setup.format == Format::Doubles { "Doubles" } else { "Singles" };
        let label = format!("{} · {format}", built.title);
        let template = built.b;
        let progress = Arc::new(AtomicU32::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let (p2, c2) = (progress.clone(), cancel.clone());
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
        std::thread::spawn(move || {
            let r = engine::simulate(&template, n, ai, seed, &p2, &c2);
            let _ = tx.send(r);
        });
        self.setup.sim.auto = Some(AutoRun { progress, cancel, n, rx, started: Instant::now(), label, ai });
        self.setup.sim.note = None;
        let mut s = self.setup.sim.setup.clone();
        s.when = super::team_home::now_secs();
        self.setup.sim.file().remember(s);
        self.setup.sim.touch();
    }

    /// Pick up the results when they're in.
    fn sim_auto_poll(&mut self) {
        let Some(run) = &self.setup.sim.auto else { return };
        let Ok(report) = run.rx.try_recv() else { return };
        let stopped = run.cancel.load(Ordering::Relaxed);
        let done = AutoDone { secs: run.started.elapsed().as_secs_f32(), label: run.label.clone(), ai: run.ai, report, stopped };
        // the recent setup remembers how it did
        if done.report.n > 0 {
            let cur = self.setup.sim.setup.clone();
            let f = self.setup.sim.file();
            if let Some(r) = f.recent.iter_mut().find(|r| r.mode == cur.mode && r.format == cur.format && r.team == cur.team && r.picks == cur.picks && r.foes == cur.foes && r.trainer == cur.trainer) {
                r.last_sim = Some((done.report.won, done.report.n));
            }
            self.setup.sim.touch();
        }
        self.setup.sim.auto = None;
        self.setup.sim.auto_done = Some(done);
    }

    /// The auto-sim card: run it, watch it go, read the results.
    pub(super) fn sim_auto_card(&mut self, ui: &mut egui::Ui) {
        self.sim_auto_poll();
        let mut start = false;
        let mut stop = false;
        let mut close = false;
        let mut watch = false;
        kit::card(ui, |ui| {
            // the head: what plays you, how many, run
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Auto-sim").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.label(small("this battle, played by the AI on both sides, many times over"));
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.label(caps("YOUR SIDE PLAYS"));
                let cur = self.setup.sim.auto_ai.unwrap_or(Ai::Expert);
                for (a, l, tip) in [(Ai::Smart, "Smart", "Strong hits and KOs"), (Ai::Expert, "Expert", "KOs, status, set-up and switching out of bad matchups")] {
                    if chip(ui, l, cur == a, OURS).on_hover_text(tip).clicked() {
                        self.setup.sim.auto_ai = Some(a);
                    }
                }
                ui.add_space(12.0);
                ui.label(caps("BATTLES"));
                let n = auto_count(self.setup.sim.auto_n);
                for c in COUNTS {
                    if chip(ui, &thousands(c), n == c, OURS).clicked() {
                        self.setup.sim.auto_n = c;
                    }
                }
                ui.add_space(12.0);
                if self.setup.sim.auto.is_some() {
                    stop = soft(ui, "Stop").clicked();
                } else {
                    start = kit::accent_button_w(ui, egui::Button::new(RichText::new(format!("Run {}", thousands(n))).size(13.5)), 8.0, 110.0).clicked();
                    if self.setup.sim.auto_done.is_some() {
                        close = soft(ui, "Close").clicked();
                    }
                }
            });
            ui.add_space(12.0);
            if let Some(run) = &self.setup.sim.auto {
                let done = run.progress.load(Ordering::Relaxed).min(run.n);
                let f = done as f32 / run.n.max(1) as f32;
                ui.label(RichText::new(format!("{} — {} of {} battles…", run.label, thousands(done), thousands(run.n))).size(13.0).color(TEXT()));
                ui.add_space(6.0);
                let w = ui.available_width();
                let (r, _) = ui.allocate_exact_size(egui::vec2(w, 10.0), Sense::hover());
                ui.painter().rect_filled(r, 5.0, kit::sunken());
                ui.painter().rect_filled(Rect::from_min_size(r.min, egui::vec2(r.width() * f, r.height())), 5.0, OURS);
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
                return;
            }
            let Some(done) = self.setup.sim.auto_done.as_ref().map(|d| AutoDone { report: d.report.clone(), label: d.label.clone(), ai: d.ai, secs: d.secs, stopped: d.stopped }) else { return };
            let r = &done.report;
            if r.n == 0 {
                ui.label(small("Stopped before any battle finished."));
                return;
            }
            let wide = ui.available_width() > 760.0;
            // the headline
            let headline = |ui: &mut egui::Ui| {
                let wr = r.win_rate();
                let (lo, hi) = r.win_range();
                let col = if wr >= 0.6 {
                    kit::C_GREEN
                } else if wr >= 0.4 {
                    kit::C_GOLD
                } else {
                    kit::C_RED
                };
                ui.label(caps("YOU WIN"));
                ui.label(RichText::new(pct(wr)).font(crate::fonts::sans_bold_font(40.0)).color(super::charts::ink(col)));
                ui.label(small(format!("of {} battles · 95% sure it's between {} and {}", thousands(r.n), pct(lo), pct(hi))));
                ui.add_space(8.0);
                let w = ui.available_width().min(360.0);
                let (bar, _) = ui.allocate_exact_size(egui::vec2(w, 12.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(bar, 6.0, kit::sunken());
                let mut x = bar.min.x;
                for (n, c) in [(r.won, kit::C_GREEN), (r.draw + r.other, kit::C_SLATE), (r.lost, kit::C_RED)] {
                    let ww = bar.width() * n as f32 / r.n as f32;
                    if ww > 0.5 {
                        p.rect_filled(Rect::from_min_size(egui::pos2(x, bar.min.y), egui::vec2(ww, bar.height())), 0.0, c);
                    }
                    x += ww;
                }
                p.rect_stroke(bar, 6.0, Stroke::new(1.0f32, LINE()));
                ui.add_space(4.0);
                let mut legend = format!("{} won · {} lost", thousands(r.won), thousands(r.lost));
                if r.draw + r.other > 0 {
                    legend.push_str(&format!(" · {} other", thousands(r.draw + r.other)));
                }
                ui.label(small(legend));
                // the takeaway, in words
                let per = |m: &MonReport| m.damage as f64 / r.n.max(1) as f64;
                let threat = r.theirs.iter().max_by(|a, b| a.kos.cmp(&b.kos).then(per(a).partial_cmp(&per(b)).unwrap_or(std::cmp::Ordering::Equal)));
                let star = r.ours.iter().max_by(|a, b| a.kos.cmp(&b.kos).then(per(a).partial_cmp(&per(b)).unwrap_or(std::cmp::Ordering::Equal)));
                let mut words: Vec<String> = Vec::new();
                if let Some(t) = threat.filter(|t| t.kos > 0) {
                    words.push(format!("{} is your biggest problem ({:.1} KOs a battle).", t.name, t.kos as f64 / r.n as f64));
                }
                if let Some(s) = star.filter(|s| s.kos > 0) {
                    words.push(format!("{} is your best answer ({:.1} KOs a battle).", s.name, s.kos as f64 / r.n as f64));
                }
                if !words.is_empty() {
                    ui.add_space(8.0);
                    ui.add(egui::Label::new(RichText::new(words.join(" ")).size(13.0).color(TEXT())).wrap());
                }
            };
            let length = |ui: &mut egui::Ui| {
                let max = r.turns.iter().copied().max().unwrap_or(0);
                let min = r.turns.iter().copied().min().unwrap_or(0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 18.0;
                    for (k, v) in [
                        ("AVERAGE LENGTH", format!("{:.1} turns", r.avg_turns())),
                        ("SHORTEST · LONGEST", format!("{min} · {max} turns")),
                        ("LEFT STANDING WHEN YOU WIN", if r.won > 0 { format!("{:.1} of {}", r.left_standing as f64 / r.won as f64, r.ours.len()) } else { "—".into() }),
                    ] {
                        ui.vertical(|ui| {
                            ui.label(caps(k));
                            ui.label(RichText::new(v).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                        });
                    }
                });
                ui.add_space(8.0);
                histogram(ui, &r.turns);
            };
            if wide {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 28.0;
                    ui.allocate_ui_with_layout(egui::vec2(340.0, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| headline(ui));
                    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| length(ui));
                });
            } else {
                headline(ui);
                ui.add_space(12.0);
                length(ui);
            }
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(10.0);
            // the two teams
            let n = r.n;
            if wide {
                let half = (ui.available_width() - 24.0) / 2.0;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 24.0;
                    ui.allocate_ui_with_layout(egui::vec2(half, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(half);
                        self.auto_team(ui, "YOUR TEAM", &r.ours, n, OURS);
                    });
                    ui.allocate_ui_with_layout(egui::vec2(half, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(half);
                        self.auto_team(ui, "THEIR SIDE", &r.theirs, n, RED);
                    });
                });
            } else {
                self.auto_team(ui, "YOUR TEAM", &r.ours, n, OURS);
                ui.add_space(12.0);
                self.auto_team(ui, "THEIR SIDE", &r.theirs, n, RED);
            }
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                let ai = match done.ai {
                    Ai::Smart => "Smart",
                    Ai::Expert => "Expert",
                    Ai::Random => "Random",
                };
                let mut foot = format!("{} · your side played by the {ai} AI · {} battles in {:.1} s", done.label, thousands(r.n), done.secs);
                if done.stopped {
                    foot.push_str(" · stopped early");
                }
                if r.capped > 0 {
                    foot.push_str(&format!(" · {} hit the 400-turn limit", r.capped));
                }
                ui.label(small(foot));
                ui.add_space(8.0);
                watch = ui.link(RichText::new("Play one yourself").size(12.0)).clicked();
            });
        });
        if start {
            self.sim_auto_start();
        }
        if stop {
            if let Some(run) = &self.setup.sim.auto {
                run.cancel.store(true, Ordering::Relaxed);
            }
        }
        if close {
            self.setup.sim.auto_done = None;
        }
        if watch {
            if let Err(e) = self.sim_begin() {
                self.setup.sim.note = Some(e);
            }
        }
    }

    /// One side's Pokémon over all the battles.
    fn auto_team(&mut self, ui: &mut egui::Ui, title: &str, mons: &[MonReport], n: u32, col: Color32) {
        ui.label(caps(title));
        ui.add_space(6.0);
        let most = mons.iter().map(|m| m.damage).max().unwrap_or(1).max(1);
        let w = ui.available_width();
        for m in mons {
            let (r, _) = ui.allocate_exact_size(egui::vec2(w, 40.0), Sense::hover());
            let p = ui.painter().clone();
            if !self.team_sprite(&p, &m.species, m.shiny, egui::pos2(r.min.x + 18.0, r.center().y), egui::vec2(36.0, 36.0)) {
                p.text(egui::pos2(r.min.x + 18.0, r.center().y), Align2::CENTER_CENTER, m.name.chars().take(2).collect::<String>(), kit::font_bold(12.0), MUTED());
            }
            let x = r.min.x + 44.0;
            p.text(egui::pos2(x, r.min.y + 4.0), Align2::LEFT_TOP, &m.name, crate::fonts::sans_bold_font(13.0), TEXT());
            let per = m.damage as f64 / n.max(1) as f64;
            let kos = m.kos as f64 / n.max(1) as f64;
            let fainted = m.fainted as f64 / n.max(1) as f64;
            let fielded = m.fielded as f64 / n.max(1) as f64;
            let right = format!("{} dmg · {:.2} KOs · faints {}", thousands(per.round() as u32), kos, pct(fainted));
            p.text(egui::pos2(r.max.x, r.min.y + 4.0), Align2::RIGHT_TOP, right, kit::font_small(11.5), MUTED());
            // damage per battle, as a bar against the team's best
            let bar = Rect::from_min_max(egui::pos2(x, r.min.y + 25.0), egui::pos2(r.max.x, r.min.y + 31.0));
            p.rect_filled(bar, 3.0, kit::sunken());
            let f = m.damage as f32 / most as f32;
            if f > 0.0 {
                p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bar.width() * f, bar.height())), 3.0, theme::blend(CARD(), col, 0.85));
            }
            let _ = fielded;
        }
        ui.add_space(2.0);
        ui.label(small("Damage and KOs are per battle; faints is how often it went down."));
    }
}

/// How long battles took: a bar for each length.
fn histogram(ui: &mut egui::Ui, turns: &[u32]) {
    if turns.is_empty() {
        return;
    }
    const CAP: usize = 40;
    let max_t = (*turns.iter().max().unwrap_or(&1) as usize).clamp(1, CAP);
    let mut bins = vec![0u32; max_t + 1];
    for &t in turns {
        bins[(t as usize).min(max_t)] += 1;
    }
    let peak = bins.iter().copied().max().unwrap_or(1).max(1);
    let w = ui.available_width();
    let h = 84.0;
    let (r, resp) = ui.allocate_exact_size(egui::vec2(w, h + 18.0), Sense::hover());
    let p = ui.painter();
    let plot = Rect::from_min_max(r.min, egui::pos2(r.max.x, r.min.y + h));
    p.line_segment([plot.left_bottom(), plot.right_bottom()], Stroke::new(1.0f32, LINE()));
    let n = max_t as f32;
    let bw = plot.width() / n;
    let mut hover: Option<(usize, u32)> = None;
    for t in 1..=max_t {
        let c = bins[t];
        let x0 = plot.min.x + (t - 1) as f32 * bw;
        let bh = plot.height() * c as f32 / peak as f32;
        let bar = Rect::from_min_max(egui::pos2(x0 + bw * 0.12, plot.max.y - bh), egui::pos2(x0 + bw * 0.88, plot.max.y));
        let on = resp.hover_pos().is_some_and(|m| m.x >= x0 && m.x < x0 + bw);
        if on {
            hover = Some((t, c));
        }
        if c > 0 {
            p.rect_filled(bar, 2.0, if on { OURS } else { theme::blend(CARD(), OURS, 0.7) });
        }
    }
    // a few turn labels
    let step = if max_t > 20 { 5 } else if max_t > 10 { 2 } else { 1 };
    for t in (1..=max_t).filter(|t| *t == 1 || t % step == 0) {
        let x = plot.min.x + (t as f32 - 0.5) * bw;
        let label = if t == CAP { format!("{t}+") } else { t.to_string() };
        p.text(egui::pos2(x, plot.max.y + 3.0), Align2::CENTER_TOP, label, kit::font_small(10.0), MUTED());
    }
    let text = match hover {
        Some((t, c)) => format!("{} battle{} took {t}{} turn{}", thousands(c), if c == 1 { "" } else { "s" }, if t == CAP { "+" } else { "" }, if t == 1 { "" } else { "s" }),
        None => "Battle length (turns)".to_string(),
    };
    p.text(plot.left_top(), Align2::LEFT_TOP, text, kit::font_small(11.0), MUTED());
}
