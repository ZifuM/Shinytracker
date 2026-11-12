//! Battle Sim: put your Team Planner teams up against a wild Pokémon, a
//! trainer or a catch test. This is its start screen: who's on your side,
//! what kind of battle, who's on theirs, and the catch odds.

use super::team_home::{chip, now_secs, small, world_badge, x_button};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::battle::World;
use crate::sim::{self, Format, Mode, Setup, SimFile};
use crate::teams::Member;
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};

pub(super) const RED: Color32 = Color32::from_rgb(236, 92, 92);
pub(super) const OURS: Color32 = super::team_home::ACCENT;

#[derive(Default)]
pub(super) struct SimState {
    pub file: Option<SimFile>,
    pub load_error: Option<String>,
    pub dirty: Option<Instant>,
    pub saved_once: bool,
    pub save_error: Option<String>,
    /// The battle being set up.
    pub setup: Setup,
    pub started: bool,
    pub catch: super::sim_catch::CatchView,
    /// A note under the Start button.
    pub note: Option<String>,
    /// Scroll to the Catch Lab's throws once.
    pub scroll_lab: bool,
    /// Picking and building the opponent.
    pub foe: super::sim_foes::FoeView,
    /// The battle on screen, if one's going.
    pub battle: Option<Box<super::sim_battle::BattleView>>,
    pub speed: super::sim_battle::Speed,
    /// The auto-sim running, its last results, how many to run, and which AI plays you.
    pub auto: Option<super::sim_auto::AutoRun>,
    pub auto_done: Option<super::sim_auto::AutoDone>,
    pub auto_n: u32,
    pub auto_ai: Option<crate::sim::Ai>,
    /// No damage odds on the move buttons.
    pub hide_odds: bool,
}

impl SimState {
    pub fn file(&mut self) -> &mut SimFile {
        if self.file.is_none() {
            let (f, note) = sim::load();
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
        if !force && t.elapsed() < Duration::from_millis(800) {
            return;
        }
        let Some(f) = self.file.as_ref() else { return };
        match sim::save(f, !self.saved_once) {
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

    pub fn new_id(&mut self) -> u64 {
        let mut id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(1);
        let f = self.file();
        while f.trainers.iter().any(|t| t.id == id) || f.foes.iter().any(|t| t.id == id) {
            id += 1;
        }
        id
    }
}

fn mode_meta(m: Mode) -> (&'static str, &'static str, &'static str, Color32) {
    match m {
        Mode::Wild => ("🌿", "Wild encounter", "A wild Pokémon: fight it or catch it", kit::C_GREEN),
        Mode::Trainer => ("🏅", "Trainer battle", "Against a trainer's whole team", kit::C_GOLD),
        Mode::Catch => ("🎯", "Catch test", "The odds of every Poké Ball", kit::C_VIOLET),
    }
}

impl App {
    pub(super) fn sim_save_tick(&mut self, force: bool) {
        self.setup.sim.save_tick(force);
    }

    pub(super) fn page_sim(&mut self, ui: &mut egui::Ui) {
        self.setup.sim.file();
        if self.setup.sim.battle.is_some() {
            return self.page_battle(ui);
        }
        kit::page_header(ui, "BATTLE SIM", "Put your teams up against wild Pokémon and trainers, in singles or doubles, and test your catch odds.");
        if let Some(e) = self.setup.sim.load_error.clone() {
            let mut close = false;
            note_bar(ui, kit::C_ORANGE, &e, Some(&mut close));
            if close {
                self.setup.sim.load_error = None;
            }
        }
        if let Some(e) = self.setup.sim.save_error.clone() {
            note_bar(ui, kit::C_RED, &format!("Couldn't save your battle setups: {e}. Trying again…"), None);
        }
        // a fresh setup starts with your newest team and a sensible foe
        if !self.setup.sim.started {
            self.setup.sim.started = true;
            let recent = self.setup.sim.file().recent.first().cloned();
            match recent {
                Some(r) => self.setup.sim.setup = r,
                None => {
                    let teams = self.team_choices();
                    let s = &mut self.setup.sim.setup;
                    s.team = teams.first().map(|t| t.0);
                    s.picks = (0..teams.first().map(|t| t.2).unwrap_or(0).min(6)).collect();
                    let mut g = Member { level: 20, ..Member::new("Gible", World::Platinum) };
                    if let Some(w) = self.wiki.clone() {
                        g.moves = super::sim_foes::wild_set(&w, "Gible", 20, World::Platinum);
                    }
                    self.setup.sim.setup.foes = vec![g];
                }
            }
        }
        self.sim_modes(ui);
        ui.add_space(14.0);
        let mode = self.setup.sim.setup.mode;
        // the auto-sim's results sit above the board: they're what you came for
        if mode != Mode::Catch && (self.setup.sim.auto.is_some() || self.setup.sim.auto_done.is_some()) {
            self.sim_auto_card(ui);
            ui.add_space(14.0);
        }
        self.sim_board(ui);
        ui.add_space(14.0);
        if mode != Mode::Trainer {
            self.sim_catch(ui);
        }
        self.sim_recent(ui);
    }

    /// The three kinds of battle, as big tiles.
    fn sim_modes(&mut self, ui: &mut egui::Ui) {
        let cur = self.setup.sim.setup.mode;
        let gap = 14.0;
        let avail = ui.available_width();
        let w = ((avail - gap * 2.0) / 3.0).floor();
        let mut pick = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for m in [Mode::Wild, Mode::Trainer, Mode::Catch] {
                let (icon, title, sub, col) = mode_meta(m);
                let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 74.0), Sense::click());
                let on = cur == m;
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter();
                p.add(kit::card_shadow().as_shape(r, 14.0));
                p.rect_filled(r, 14.0, if on { theme::blend(CARD(), col, 0.16) } else { theme::blend(CARD(), CARD_HOVER(), ht) });
                p.rect_stroke(r, 14.0, Stroke::new(if on { 2.0f32 } else { 1.0 }, if on { theme::blend(LINE(), col, 0.8) } else { theme::blend(LINE(), col, 0.3 * ht) }));
                kit::icon_chip(p, Rect::from_min_size(r.min + egui::vec2(16.0, 21.0), egui::vec2(32.0, 32.0)), icon, col, on);
                p.text(r.min + egui::vec2(60.0, 27.0), Align2::LEFT_CENTER, title, crate::fonts::sans_bold_font(15.0), TEXT());
                let f = crate::fonts::sans_font(12.0);
                p.text(r.min + egui::vec2(60.0, 48.0), Align2::LEFT_CENTER, super::charts::clip(ui, sub, &f, w - 72.0), f, MUTED());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some(m);
                }
            }
        });
        if let Some(m) = pick {
            let s = &mut self.setup.sim.setup;
            s.mode = m;
            if m == Mode::Catch {
                s.format = Format::Singles;
            }
            self.setup.sim.note = None;
        }
    }

    /// Your side · VS · their side.
    fn sim_board(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_width();
        let wide = avail >= 1000.0;
        let gap = 16.0;
        let mid = if wide { 150.0 } else { avail };
        let side = if wide { ((avail - mid - gap * 2.0) / 2.0).floor() } else { avail };
        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(side, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(side);
                    self.sim_ours(ui);
                    self.sim_preview(ui);
                });
                ui.allocate_ui_with_layout(egui::vec2(mid, 0.0), egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.set_width(mid);
                    self.sim_vs(ui, false);
                });
                ui.allocate_ui_with_layout(egui::vec2(side, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(side);
                    self.sim_theirs(ui);
                });
            });
        } else {
            self.sim_ours(ui);
            self.sim_vs(ui, true);
            ui.add_space(14.0);
            self.sim_theirs(ui);
            self.sim_preview(ui);
        }
    }

    fn sim_ours(&mut self, ui: &mut egui::Ui) {
        let teams = self.team_choices();
        let mut go_planner = false;
        let mut set_team: Option<u64> = None;
        let mut toggle: Option<usize> = None;
        let s = self.setup.sim.setup.clone();
        let team = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).cloned());
        side_card(ui, OURS, "YOUR SIDE", |ui| {
            if teams.is_empty() {
                ui.label(RichText::new("No teams yet").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.label(small("Battle Sim uses your Team Planner teams. Make one there (it can be just one Pokémon) and come back."));
                ui.add_space(8.0);
                go_planner = kit::accent_button_w(ui, egui::Button::new(RichText::new("Open the Team Planner").size(13.0)), 8.0, 160.0).clicked();
                return;
            }
            // which team
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for (id, name, n) in teams.iter().take(8) {
                    let label = format!("{}  {n}", if name.trim().is_empty() { "Untitled team" } else { name });
                    if chip(ui, &label, s.team == Some(*id), OURS).clicked() {
                        set_team = Some(*id);
                    }
                }
            });
            ui.add_space(10.0);
            let Some(t) = &team else {
                ui.label(small("Pick a team."));
                return;
            };
            if t.members.is_empty() {
                ui.label(small("This team has no Pokémon yet."));
                return;
            }
            // the lineup: click to bring one along, the first ones lead
            let full = ui.available_width();
            let n = t.members.len();
            let tile = ((full - 8.0 * 5.0) / 6.0).clamp(56.0, 92.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (i, m) in t.members.iter().enumerate() {
                    let order = s.picks.iter().position(|p| *p == i);
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(tile, tile + 20.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let p = ui.painter().clone();
                    let leads = match s.format {
                        Format::Singles => 1,
                        Format::Doubles => 2,
                    };
                    let col = match order {
                        Some(o) if o < leads => OURS,
                        Some(_) => theme::blend(OURS, MUTED(), 0.4),
                        None => LINE(),
                    };
                    let box_r = Rect::from_min_size(r.min, egui::vec2(tile, tile));
                    p.rect_filled(box_r, 12.0, if order.is_some() { theme::blend(CARD(), OURS, 0.12) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                    p.rect_stroke(box_r, 12.0, Stroke::new(if order.is_some() { 1.5f32 } else { 1.0 }, col));
                    let dim = order.is_none();
                    if !self.team_sprite(&p, &m.species, m.shiny, box_r.center(), egui::vec2(tile - 8.0, tile - 8.0)) {
                        p.text(box_r.center(), Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(13.0), MUTED());
                    }
                    if dim {
                        p.rect_filled(box_r, 12.0, CARD().gamma_multiply(0.55));
                    }
                    if let Some(o) = order {
                        let c = egui::pos2(box_r.max.x - 12.0, box_r.min.y + 12.0);
                        p.circle_filled(c, 10.0, if o < leads { OURS } else { theme::blend(CARD(), MUTED(), 0.5) });
                        p.text(c, Align2::CENTER_CENTER, (o + 1).to_string(), crate::fonts::sans_bold_font(11.0), Color32::WHITE);
                    }
                    let f = crate::fonts::sans_font(11.0);
                    p.text(egui::pos2(box_r.center().x, r.max.y - 8.0), Align2::CENTER_CENTER, super::charts::clip(ui, m.display_name(), &f, tile), f, if dim { MUTED() } else { TEXT() });
                    let tip = match order {
                        Some(o) if o < leads => "Leads the battle. Click to leave it out.".to_string(),
                        Some(_) => "In the back. Click to leave it out.".to_string(),
                        None => "Click to bring it along".to_string(),
                    };
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked() {
                        toggle = Some(i);
                    }
                }
            });
            ui.add_space(8.0);
            let picked = s.picks.iter().filter(|p| **p < n).count();
            let txt = match (picked, s.format) {
                (0, _) => "Click the Pokémon that battle. The first you pick leads.".to_string(),
                (1, Format::Doubles) => "Doubles needs two to lead: pick another one.".to_string(),
                (k, _) => format!("{k} battling · numbers show the order they come out"),
            };
            ui.label(small(txt));
            ui.horizontal(|ui| {
                if ui.link(RichText::new("All").size(12.0)).clicked() {
                    self.setup.sim.setup.picks = (0..n).collect();
                }
                if ui.link(RichText::new("None").size(12.0)).clicked() {
                    self.setup.sim.setup.picks.clear();
                }
                ui.add_space(6.0);
                world_label(ui, t.world);
            });
        });
        if go_planner {
            self.go_to(Tab::Team);
        }
        if let Some(id) = set_team {
            let n = self.setup.team.teams().teams.iter().find(|t| t.id == id).map(|t| t.members.len()).unwrap_or(0);
            let st = &mut self.setup.sim.setup;
            st.team = Some(id);
            st.picks = (0..n.min(6)).collect();
        }
        if let Some(i) = toggle {
            let picks = &mut self.setup.sim.setup.picks;
            if let Some(p) = picks.iter().position(|x| *x == i) {
                picks.remove(p);
            } else {
                picks.push(i);
            }
        }
    }

    /// The middle: VS, singles or doubles, and Start. In a narrow window
    /// it's one row between the two sides.
    fn sim_vs(&mut self, ui: &mut egui::Ui, compact: bool) {
        let s = self.setup.sim.setup.clone();
        let mut f = s.format;
        let ready = self.sim_ready();
        let label = match s.mode {
            Mode::Catch => "Test the catch",
            _ => "Start battle",
        };
        let mut start = false;
        let medal = |ui: &mut egui::Ui, size: f32| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
            let p = ui.painter();
            let c = r.center();
            let rad = size / 2.0 - 2.0;
            p.circle_filled(c, rad, theme::blend(CARD(), RED, 0.10));
            p.circle_stroke(c, rad, Stroke::new(2.0f32, theme::blend(LINE(), OURS, 0.6)));
            p.line_segment([c + egui::vec2(rad * 0.22, -rad * 0.87), c + egui::vec2(-rad * 0.22, rad * 0.87)], Stroke::new(2.0f32, theme::blend(LINE(), RED, 0.5)));
            p.text(c, Align2::CENTER_CENTER, "VS", crate::fonts::sans_bold_font(size * 0.3), TEXT());
        };
        let formats = |ui: &mut egui::Ui, f: &mut Format| {
            if s.mode == Mode::Catch {
                ui.label(small("Catch tests are one on one"));
                return;
            }
            for (v, l) in [(Format::Singles, "Singles"), (Format::Doubles, "Doubles")] {
                if chip(ui, l, *f == v, OURS).clicked() {
                    *f = v;
                }
            }
        };
        let button = |ui: &mut egui::Ui, start: &mut bool| {
            let resp = ui.add_enabled_ui(ready.is_ok(), |ui| kit::accent_button_w(ui, egui::Button::new(RichText::new(label).size(14.0)), 8.0, 130.0)).inner;
            let resp = match &ready {
                Err(why) => resp.on_disabled_hover_text(why.as_str()),
                Ok(()) if s.mode == Mode::Catch => resp.on_hover_text("Go to the throws in the Catch Lab"),
                Ok(()) => resp.on_hover_text("Play this battle yourself"),
            };
            *start = resp.clicked();
        };
        let n_auto = super::sim_auto::auto_count(self.setup.sim.auto_n);
        let mut auto = false;
        let auto_link = |ui: &mut egui::Ui, auto: &mut bool| {
            if s.mode != Mode::Catch && ready.is_ok() {
                *auto = ui.link(RichText::new(format!("or simulate {} battles", thousands(n_auto))).size(12.0)).on_hover_text("Both sides played by the AI, many times over: your win rate and who does what").clicked();
            }
        };
        if compact {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                medal(ui, 56.0);
                formats(ui, &mut f);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    button(ui, &mut start);
                    auto_link(ui, &mut auto);
                });
            });
        } else {
            ui.add_space(30.0);
            medal(ui, 96.0);
            ui.add_space(14.0);
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                formats(ui, &mut f);
            });
            ui.add_space(16.0);
            button(ui, &mut start);
            ui.add_space(6.0);
            ui.vertical_centered(|ui| auto_link(ui, &mut auto));
        }
        if f != s.format {
            self.setup.sim.setup.format = f;
        }
        if start {
            self.sim_start();
        }
        if auto {
            self.sim_auto_start();
        }
        if let Some(n) = &self.setup.sim.note {
            ui.add_space(6.0);
            ui.add(egui::Label::new(small(n.clone())).wrap());
        }
    }

    /// The first turn at a glance: your leads against theirs, both ways.
    fn sim_preview(&mut self, ui: &mut egui::Ui) {
        use super::team_calc::{calc_mon, calc_move, Poke};
        let s = self.setup.sim.setup.clone();
        let Some(team) = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).cloned()) else { return };
        let world = team.world;
        let leads_n = if s.format == Format::Doubles && s.mode != Mode::Catch { 2 } else { 1 };
        let ours: Vec<Member> = s.picks.iter().filter_map(|i| team.members.get(*i).cloned()).take(leads_n).collect();
        let theirs: Vec<Member> = match s.mode {
            Mode::Trainer => s.trainer.and_then(|id| self.setup.sim.file().trainers.iter().find(|t| t.id == id).map(|t| t.members.iter().take(leads_n).cloned().collect())).unwrap_or_default(),
            _ => s.foes.iter().take(leads_n).cloned().collect(),
        };
        if ours.is_empty() || theirs.is_empty() {
            return;
        }
        let wiki = self.wiki.clone();
        let field = crate::calc::Field { doubles: leads_n == 2, ..Default::default() };
        // best hit from a on d: (move, lo %, hi %, ko)
        let best = |a: &Poke, d: &Poke| -> Option<(String, f32, f32, String)> {
            let am = calc_mon(a, world)?;
            let dm = calc_mon(d, world)?;
            let mut out: Option<(String, f32, f32, String)> = None;
            for n in &a.set.moves {
                let Some(mv) = calc_move(wiki.as_deref(), n, &a.set, false) else { continue };
                let h = crate::calc::damage(&am, &dm, &mv, &field, world);
                if h.none.is_some() {
                    continue;
                }
                let max = dm.stats[0].max(1) as f32;
                let (lo, hi) = (h.min() as f32 * 100.0 / max, h.max() as f32 * 100.0 / max);
                if out.as_ref().is_none_or(|o| hi > o.2) {
                    out = Some((n.clone(), lo, hi, crate::calc::ko_desc(&h, &dm, &field, world)));
                }
            }
            out
        };
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("First turn").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.label(small("your leads against theirs: best hit and who moves first"));
            });
            ui.add_space(6.0);
            let full = ui.available_width();
            for a in &ours {
                for d in &theirs {
                    let pa = Poke::new(a.clone(), None);
                    let pd = Poke::new(d.clone(), None);
                    let (Some(am), Some(dm)) = (calc_mon(&pa, world), calc_mon(&pd, world)) else { continue };
                    let (sa, sd) = (crate::calc::final_speed(&am, &field), crate::calc::final_speed(&dm, &field));
                    let (spd, sc) = if sa > sd { ("you move first", kit::C_GREEN) } else if sa < sd { ("they move first", kit::C_ORANGE) } else { ("speed tie", kit::C_GOLD) };
                    let (r, _) = ui.allocate_exact_size(egui::vec2(full, 58.0), Sense::hover());
                    let p = ui.painter().clone();
                    p.rect_filled(r, 10.0, kit::sunken());
                    p.text(egui::pos2(r.min.x + 12.0, r.min.y + 14.0), Align2::LEFT_CENTER, format!("{} vs {}", a.display_name(), d.display_name()), crate::fonts::sans_bold_font(12.5), TEXT());
                    p.text(egui::pos2(r.max.x - 12.0, r.min.y + 14.0), Align2::RIGHT_CENTER, spd, crate::fonts::sans_bold_font(11.0), super::charts::ink(sc));
                    let f = crate::fonts::sans_font(11.5);
                    for (k, (b, arrow, col)) in [(best(&pa, &pd), "→", OURS), (best(&pd, &pa), "←", RED)].into_iter().enumerate() {
                        let txt = match b {
                            Some((n, lo, hi, ko)) => format!("{arrow} {n}: {lo:.0}–{hi:.0}%{}", if ko.is_empty() { String::new() } else { format!(" · {ko}") }),
                            None => format!("{arrow} nothing that hurts"),
                        };
                        p.text(egui::pos2(r.min.x + 12.0, r.min.y + 32.0 + 15.0 * k as f32), Align2::LEFT_CENTER, super::charts::clip(ui, &txt, &f, full - 24.0), f.clone(), super::charts::ink(col));
                    }
                    ui.add_space(4.0);
                }
            }
        });
    }

    /// Whether the setup can start, and why not.
    fn sim_ready(&mut self) -> Result<(), String> {
        let s = self.setup.sim.setup.clone();
        // the catch odds only need the wild Pokémon
        if s.mode == Mode::Catch {
            return if s.foes.is_empty() { Err("Pick a wild Pokémon".into()) } else { Ok(()) };
        }
        let Some(t) = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).cloned()) else {
            return Err("Pick one of your teams".into());
        };
        let picked = s.picks.iter().filter(|p| **p < t.members.len()).count();
        if picked == 0 {
            return Err("Pick who battles on your side".into());
        }
        if s.format == Format::Doubles && picked < 2 && s.mode != Mode::Catch {
            return Err("Doubles needs at least two on your side".into());
        }
        match s.mode {
            Mode::Trainer => {
                let tr = s.trainer.and_then(|id| self.setup.sim.file().trainers.iter().find(|x| x.id == id).cloned());
                match tr {
                    None => Err("Pick a trainer to battle".into()),
                    Some(tr) if tr.members.is_empty() => Err("That trainer has no Pokémon".into()),
                    Some(tr) if s.format == Format::Doubles && tr.members.len() < 2 => Err("Doubles needs a trainer with two or more".into()),
                    _ => Ok(()),
                }
            }
            _ if s.foes.is_empty() => Err("Pick the wild Pokémon".into()),
            _ => Ok(()),
        }
    }

    fn sim_start(&mut self) {
        if self.setup.sim.setup.mode == Mode::Catch {
            // a catch test happens right here, in the Catch Lab
            self.setup.sim.scroll_lab = true;
            let mut s = self.setup.sim.setup.clone();
            s.when = now_secs();
            self.setup.sim.file().remember(s);
            self.setup.sim.touch();
            self.setup.sim.note = None;
            return;
        }
        let mut s = self.setup.sim.setup.clone();
        s.when = now_secs();
        s.world = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).map(|t| t.world)).unwrap_or_default();
        self.setup.sim.file().remember(s);
        self.setup.sim.touch();
        self.setup.sim.note = None;
        if let Err(e) = self.sim_begin() {
            self.setup.sim.note = Some(e);
        }
    }

    fn sim_theirs(&mut self, ui: &mut egui::Ui) {
        let s = self.setup.sim.setup.clone();
        match s.mode {
            Mode::Trainer => self.sim_trainers(ui),
            _ => self.sim_wild(ui),
        }
    }

    /// The last setups, to start again with one click.
    fn sim_recent(&mut self, ui: &mut egui::Ui) {
        let recent: Vec<Setup> = self.setup.sim.file().recent.clone();
        if recent.is_empty() {
            return;
        }
        let teams = self.setup.team.teams().teams.clone();
        let trainers = self.setup.sim.file().trainers.clone();
        let mut load: Option<Setup> = None;
        let mut clear = false;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Recent").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    clear = ui.link(RichText::new("Clear").size(12.0)).clicked();
                });
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                for r in recent.iter().take(8) {
                    let team = r.team.and_then(|id| teams.iter().find(|t| t.id == id)).map(|t| t.name.clone()).unwrap_or_else(|| "a deleted team".into());
                    let them = match r.mode {
                        Mode::Trainer => r.trainer.and_then(|id| trainers.iter().find(|t| t.id == id)).map(|t| t.name.clone()).unwrap_or_else(|| "a trainer".into()),
                        _ => {
                            let who = r.foes.iter().map(|f| format!("{} Lv {}", f.species, f.level)).collect::<Vec<_>>().join(" & ");
                            if r.place.is_empty() {
                                who
                            } else {
                                format!("{who} on {}", r.place)
                            }
                        }
                    };
                    let (icon, _, _, col) = mode_meta(r.mode);
                    let fmt = if r.format == Format::Doubles { " · doubles" } else { "" };
                    let sim = match r.last_sim {
                        Some((won, n)) if n > 0 => format!(" · wins {:.0}%", won as f64 * 100.0 / n as f64),
                        _ => String::new(),
                    };
                    if chip(ui, &format!("{icon}  {team} vs {them}{fmt}{sim}"), false, col).clicked() {
                        load = Some(r.clone());
                    }
                }
            });
        });
        if let Some(r) = load {
            self.setup.sim.setup = r;
            self.setup.sim.note = None;
        }
        if clear {
            self.setup.sim.file().recent.clear();
            self.setup.sim.touch();
        }
    }
}

/// One side of the board: a coloured edge and a heading.
pub(super) fn side_card(ui: &mut egui::Ui, col: Color32, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let r = kit::card(ui, |ui| {
        ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(11.0)).color(super::charts::ink(col)));
        ui.add_space(8.0);
        add(ui);
    });
    let _ = r;
    if let Some(rect) = kit::last_card_rect(ui) {
        ui.painter().rect_filled(Rect::from_min_size(rect.min + egui::vec2(0.0, 18.0), egui::vec2(4.0, 36.0)), 2.0, col);
    }
}

fn note_bar(ui: &mut egui::Ui, col: Color32, text: &str, close: Option<&mut bool>) {
    egui::Frame::none()
        .fill(theme::blend(CARD(), col, 0.12))
        .stroke(Stroke::new(1.0f32, theme::blend(LINE(), col, 0.5)))
        .rounding(10.0)
        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(RichText::new(text).size(13.0).color(TEXT())).wrap());
                if let Some(c) = close {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        *c = x_button(ui).clicked();
                    });
                }
            });
        });
    ui.add_space(10.0);
}

fn world_label(ui: &mut egui::Ui, w: World) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(90.0, 18.0), Sense::hover());
    world_badge(ui.painter(), egui::pos2(r.max.x, r.center().y), w, true);
}

pub(super) fn trim(m: f64) -> String {
    let s = format!("{m:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub(super) fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
