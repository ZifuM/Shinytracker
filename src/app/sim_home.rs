//! Battle Sim: put your Team Planner teams up against a wild Pokémon, a
//! trainer or a catch test. This is its start screen: who's on your side,
//! what kind of battle, who's on theirs, and the catch odds.

use super::team_home::{caps, chip, now_secs, small, world_badge, x_button};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::battle::{self, World};
use crate::calc::Status;
use crate::sim::{self, Ball, CatchSituation, Format, Mode, Setup, SimFile, BALLS};
use crate::teams::Member;
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};

pub(super) const RED: Color32 = Color32::from_rgb(236, 92, 92);
pub(super) const OURS: Color32 = super::team_home::ACCENT;

/// The catch-odds controls.
#[derive(Clone, Copy)]
pub(super) struct CatchView {
    pub hp: f32,
    pub status: Status,
    pub sit: CatchSituation,
}

impl Default for CatchView {
    fn default() -> Self {
        CatchView { hp: 1.0, status: Status::None, sit: CatchSituation::default() }
    }
}

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
    pub catch: CatchView,
    /// A note under the Start button.
    pub note: Option<String>,
    /// Picking and building the opponent.
    pub foe: super::sim_foes::FoeView,
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
        self.sim_board(ui);
        ui.add_space(14.0);
        let mode = self.setup.sim.setup.mode;
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
                Ok(()) => resp.on_hover_text("The battle screen is still being built: for now this saves the setup to Recent"),
            };
            *start = resp.clicked();
        };
        if compact {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                medal(ui, 56.0);
                formats(ui, &mut f);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    button(ui, &mut start);
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
        }
        if f != s.format {
            self.setup.sim.setup.format = f;
        }
        if start {
            self.sim_start();
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
        let mut s = self.setup.sim.setup.clone();
        s.when = now_secs();
        s.world = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).map(|t| t.world)).unwrap_or_default();
        self.setup.sim.file().remember(s);
        self.setup.sim.touch();
        self.setup.sim.note = Some("Saved to Recent. The battle screen comes in a coming update.".into());
    }

    fn sim_theirs(&mut self, ui: &mut egui::Ui) {
        let s = self.setup.sim.setup.clone();
        match s.mode {
            Mode::Trainer => self.sim_trainers(ui),
            _ => self.sim_wild(ui),
        }
    }

    /// Every ball's chance against the wild Pokémon, in this situation.
    fn sim_catch(&mut self, ui: &mut egui::Ui) {
        let s = self.setup.sim.setup.clone();
        let sel = if s.format == Format::Doubles && s.mode == Mode::Wild { self.setup.sim.foe.sel } else { 0 };
        let Some(f) = s.foes.get(sel).or(s.foes.first()).cloned() else { return };
        let world = s.team.and_then(|id| self.setup.team.teams().teams.iter().find(|t| t.id == id).map(|t| t.world)).unwrap_or_default();
        let Some(sp) = battle::data().species(&f.species) else { return };
        let Some(rate) = sim::data().catch_rate(sp.num) else { return };
        let max_hp = f.final_stats(world).map(|s| s[0] as u32).unwrap_or(100).max(1);
        let ty = sp.types_in(world).to_vec();
        let mut cv = self.setup.sim.catch;
        cv.sit.level = f.level;
        cv.sit.water_or_bug = ty.iter().any(|t| t == "Water" || t == "Bug");
        let prices: Vec<(String, u32)> = self.wiki.as_ref().map(|w| w.data.items.iter().filter(|i| i.pocket == "Poké Balls").map(|i| (i.name.clone(), i.price.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))).collect()).unwrap_or_default();
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Catch odds: {}", f.species)).font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.label(small(format!("catch rate {rate} · Platinum's formula, one throw")));
            });
            ui.add_space(10.0);
            let avail = ui.available_width();
            let wide = avail >= 900.0;
            let ctl_w = if wide { 300.0 } else { avail };
            let controls = |ui: &mut egui::Ui, cv: &mut CatchView| {
                ui.label(caps("ITS HP"));
                ui.horizontal(|ui| {
                    let mut pct = (cv.hp * 100.0).round() as u32;
                    let label = if pct == 0 { "1 HP".to_string() } else { format!("{pct}%") };
                    if kit::slider_u32(ui, &mut pct, 0..=100, &label).changed() {
                        cv.hp = pct as f32 / 100.0;
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for (v, l) in [(1.0f32, "Full"), (0.5, "Half"), (0.2, "Red HP"), (0.0, "1 HP")] {
                        if chip(ui, l, (cv.hp - v).abs() < 0.005, kit::C_VIOLET).clicked() {
                            cv.hp = v;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(caps("STATUS"));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for (v, l) in [(Status::None, "None"), (Status::Sleep, "Asleep ×2"), (Status::Freeze, "Frozen ×2"), (Status::Paralysis, "Paralyzed ×1.5"), (Status::Poison, "Poisoned ×1.5"), (Status::Burn, "Burned ×1.5")] {
                        if chip(ui, l, cv.status == v, kit::C_VIOLET).clicked() {
                            cv.status = v;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(caps("THE BATTLE"));
                ui.horizontal(|ui| {
                    ui.label(small("Turn"));
                    let mut t = cv.sit.turn + 1;
                    ui.add(egui::DragValue::new(&mut t).range(1..=99).speed(0.2));
                    cv.sit.turn = t - 1;
                    ui.label(small(if cv.sit.turn == 0 { "(first throw)" } else { "" }));
                });
                kit::checkbox(ui, &mut cv.sit.night_or_cave, "At night or in a cave");
                kit::checkbox(ui, &mut cv.sit.water, "Surfing or fishing");
                kit::checkbox(ui, &mut cv.sit.caught_before, "Caught one before");
            };
            let table = |ui: &mut egui::Ui, cv: &CatchView| {
                let hp = ((max_hp as f32 * cv.hp).round() as u32).clamp(1, max_hp);
                let sm = sim::status_mult(cv.status);
                let mut rows: Vec<(Ball, f64, f64)> = BALLS
                    .iter()
                    .filter(|b| **b != Ball::Safari && **b != Ball::Cherish)
                    .map(|b| {
                        let m = sim::ball_mult(*b, &cv.sit);
                        (*b, m, sim::catch_chance(rate, max_hp, hp, m, sm))
                    })
                    .collect();
                rows.sort_by(|a, b| (a.0 == Ball::Master).cmp(&(b.0 == Ball::Master)).then(b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal)));
                let best = rows.first().map(|r| r.2).unwrap_or(0.0);
                let full = ui.available_width();
                for (b, m, p) in rows {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 30.0), Sense::hover());
                    let pt = ui.painter().clone();
                    let top = (p - best).abs() < 1e-9 && b != Ball::Master;
                    if top {
                        pt.rect_filled(r, 8.0, theme::blend(CARD(), kit::C_VIOLET, 0.12));
                    } else if resp.hovered() {
                        pt.rect_filled(r, 8.0, kit::sunken());
                    }
                    pt.text(egui::pos2(r.min.x + 10.0, r.center().y), Align2::LEFT_CENTER, b.name(), crate::fonts::sans_bold_font(12.5), TEXT());
                    let ms = if b == Ball::Master { "—".to_string() } else { format!("×{}", trim(m)) };
                    pt.text(egui::pos2(r.min.x + 118.0, r.center().y), Align2::LEFT_CENTER, ms, crate::fonts::sans_font(11.5), if m > 1.0 { super::charts::ink(kit::C_VIOLET) } else { MUTED() });
                    let bx = r.min.x + 160.0;
                    let bw = (full - 160.0 - 230.0).max(60.0);
                    let bar = Rect::from_min_size(egui::pos2(bx, r.center().y - 5.0), egui::vec2(bw, 10.0));
                    pt.rect_filled(bar, 5.0, kit::sunken());
                    pt.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bw * p as f32, 10.0)), 5.0, if top { kit::C_VIOLET } else { theme::blend(kit::C_VIOLET, MUTED(), 0.45) });
                    pt.text(egui::pos2(bar.max.x + 10.0, r.center().y), Align2::LEFT_CENTER, format!("{:.1}%", p * 100.0), crate::fonts::sans_bold_font(12.5), TEXT());
                    let need = sim::balls_for(p, 0.9);
                    let price = prices.iter().find(|(n, _)| n == b.name()).map(|x| x.1).unwrap_or(0);
                    let txt = match need {
                        Some(1) => "1 is enough".to_string(),
                        Some(n) if n <= 999 => {
                            if price > 0 {
                                format!("{n} for 90%  ·  ₽{}", thousands(n * price))
                            } else {
                                format!("{n} for 90%")
                            }
                        }
                        _ => "almost never".to_string(),
                    };
                    pt.text(egui::pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, txt, crate::fonts::sans_font(11.5), MUTED());
                    if !b.when().is_empty() {
                        resp.on_hover_text(b.when());
                    }
                }
            };
            if wide {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 24.0;
                    ui.allocate_ui_with_layout(egui::vec2(ctl_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(ctl_w);
                        controls(ui, &mut cv);
                    });
                    let rw = ui.available_width();
                    ui.allocate_ui_with_layout(egui::vec2(rw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(rw);
                        table(ui, &cv);
                    });
                });
            } else {
                controls(ui, &mut cv);
                ui.add_space(10.0);
                table(ui, &cv);
            }
        });
        self.setup.sim.catch = cv;
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
                    if chip(ui, &format!("{icon}  {team} vs {them}{fmt}"), false, col).clicked() {
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

fn trim(m: f64) -> String {
    let s = format!("{m:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn thousands(n: u32) -> String {
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
