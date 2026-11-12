//! Team Planner: your teams, a builder like Pokémon Showdown's, what the
//! team covers, a checklist of how to get each one in Platinum Together,
//! and a damage calculator. Teams live in their own file (`teams.json`).

use super::pokedex::type_color;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::battle::{self, World};
use crate::teams::{self, Member, Team, TeamFile};
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};

pub(super) const ACCENT: Color32 = Color32::from_rgb(92, 196, 160);

/// The parts of an open team.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum View {
    #[default]
    Build,
    Analysis,
    Checklist,
    Calc,
}

/// What the builder's picker is choosing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Picker {
    Species,
    Item,
    Move(usize),
}

#[derive(Default)]
pub(super) struct TeamState {
    /// Loaded the first time it's needed.
    pub file: Option<TeamFile>,
    pub load_error: Option<String>,
    /// The open team's id.
    pub open: Option<u64>,
    pub slot: usize,
    pub view: View,
    pub picker: Option<Picker>,
    pub query: String,
    /// The library: search and folder filter.
    pub lib_query: String,
    pub folder: Option<String>,
    /// Unsaved changes since.
    pub dirty: Option<Instant>,
    pub saved_once: bool,
    pub save_error: Option<String>,
    /// The paste-a-team box (Some while open) and what it said last.
    pub import: Option<String>,
    pub import_note: Option<String>,
    pub export_open: bool,
    pub confirm_delete: Option<u64>,
    /// The last team deleted, for Undo.
    pub deleted: Option<(usize, Team)>,
    /// Picker options.
    pub all_moves: bool,
    pub sort: Option<(usize, bool)>,
    pub calc: super::team_calc::CalcState,
}

impl TeamState {
    pub fn teams(&mut self) -> &mut TeamFile {
        if self.file.is_none() {
            let (f, note) = teams::load();
            self.load_error = note;
            self.file = Some(f);
        }
        self.file.as_mut().expect("loaded")
    }

    pub fn touch(&mut self) {
        self.dirty.get_or_insert_with(Instant::now);
        if let Some(id) = self.open {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            if let Some(t) = self.teams().teams.iter_mut().find(|t| t.id == id) {
                t.updated = now;
            }
        }
    }

    pub fn team(&mut self) -> Option<&mut Team> {
        let id = self.open?;
        self.teams().teams.iter_mut().find(|t| t.id == id)
    }

    pub fn new_id(&mut self) -> u64 {
        let base = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(1);
        let mut id = base;
        while self.teams().teams.iter().any(|t| t.id == id) {
            id += 1;
        }
        id
    }

    /// Save now if anything changed (and at least a moment has passed).
    pub fn save_tick(&mut self, force: bool) {
        let Some(t) = self.dirty else { return };
        if !force && t.elapsed() < Duration::from_millis(800) {
            return;
        }
        let Some(f) = self.file.as_ref() else { return };
        match teams::save(f, !self.saved_once) {
            Ok(()) => {
                self.saved_once = true;
                self.dirty = None;
                self.save_error = None;
            }
            Err(e) => {
                self.save_error = Some(e);
                // try again in a bit
                self.dirty = Some(Instant::now());
            }
        }
    }

    fn open_team(&mut self, id: u64) {
        self.open = Some(id);
        self.slot = 0;
        self.view = View::Build;
        self.picker = None;
        self.export_open = false;
        self.confirm_delete = None;
        let empty = self.team().is_some_and(|t| t.members.is_empty());
        if empty {
            self.picker = Some(Picker::Species);
            self.query.clear();
        }
    }
}

/// Small helpers shared by the Team Planner pages.
pub(super) fn small(t: impl Into<String>) -> RichText {
    RichText::new(t).size(12.0).color(MUTED())
}

pub(super) fn caps(t: &str) -> RichText {
    RichText::new(t).font(crate::fonts::sans_bold_font(10.5)).color(MUTED())
}

pub(super) fn chip(ui: &mut egui::Ui, text: &str, on: bool, col: Color32) -> egui::Response {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(12.0), if on { TEXT() } else { MUTED() });
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(20.0, 12.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let fill = if on { theme::blend(CARD(), col, 0.22) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) };
    ui.painter().rect_filled(r, r.height() / 2.0, fill);
    ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, if on { theme::blend(LINE(), col, 0.7) } else { LINE() }));
    ui.painter().galley(r.min + egui::vec2(10.0, 6.0), g, TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A small × to close or clear something.
pub(super) fn x_button(ui: &mut egui::Ui) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
    let c = if resp.hovered() { TEXT() } else { MUTED() };
    let k = 4.5;
    let p = ui.painter();
    p.line_segment([r.center() + egui::vec2(-k, -k), r.center() + egui::vec2(k, k)], Stroke::new(1.6f32, c));
    p.line_segment([r.center() + egui::vec2(-k, k), r.center() + egui::vec2(k, -k)], Stroke::new(1.6f32, c));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A soft button.
pub(super) fn soft(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).size(13.0)).fill(kit::btn_fill()).min_size(egui::vec2(0.0, 30.0)))
}

/// A small coloured type label painted at `at` (left-centre); returns its width.
pub(super) fn type_tag(p: &egui::Painter, at: egui::Pos2, t: &str, size: f32) -> f32 {
    let col = type_color(t);
    let g = p.layout_no_wrap(t.to_uppercase(), crate::fonts::sans_bold_font(size), Color32::WHITE);
    let r = Rect::from_min_size(egui::pos2(at.x, at.y - (size + 8.0) / 2.0), egui::vec2(g.size().x + 12.0, size + 8.0));
    p.rect_filled(r, 4.0, col);
    p.galley(egui::pos2(r.min.x + 6.0, r.center().y - g.size().y / 2.0), g, Color32::WHITE);
    r.width()
}

/// "Physical" / "Special" / "Status" as a coloured tag; returns its width.
pub(super) fn cat_tag(p: &egui::Painter, at: egui::Pos2, c: &str) -> f32 {
    let (txt, col) = match c {
        "Physical" => ("PHYS", Color32::from_rgb(234, 104, 64)),
        "Special" => ("SPEC", Color32::from_rgb(86, 120, 230)),
        _ => ("STAT", Color32::from_rgb(150, 150, 160)),
    };
    let g = p.layout_no_wrap(txt.into(), crate::fonts::sans_bold_font(9.5), Color32::WHITE);
    let r = Rect::from_min_size(egui::pos2(at.x, at.y - 8.5), egui::vec2(g.size().x + 10.0, 17.0));
    p.rect_filled(r, 4.0, col);
    p.galley(egui::pos2(r.min.x + 5.0, r.center().y - g.size().y / 2.0), g, Color32::WHITE);
    r.width()
}

pub(super) fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn ago(t: u64) -> String {
    let d = now_secs().saturating_sub(t);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86399 => format!("{} h ago", d / 3600),
        86400..=1_209_599 => format!("{} days ago", d / 86400),
        _ => format!("{} weeks ago", d / 604_800),
    }
}

impl App {
    /// Load and save the teams file from anywhere (called every frame).
    pub(super) fn team_save_tick(&mut self, force: bool) {
        self.setup.team.save_tick(force);
    }

    /// The app's species for a battle name, falling back to the base species.
    pub(super) fn team_species(&self, name: &str) -> Option<crate::species::Species> {
        if name.is_empty() {
            return None;
        }
        if let Some(s) = self.info_species(name) {
            return Some(s);
        }
        let d = battle::data();
        let sp = d.species(name)?;
        let base = if sp.base.is_empty() { sp.num } else { d.species(&sp.base).map(|b| b.num).unwrap_or(sp.num) };
        self.species.get((base as usize).wrapping_sub(1)).cloned()
    }

    /// A team member's sprite, once loaded.
    pub(super) fn team_sprite(&mut self, p: &egui::Painter, name: &str, shiny: bool, center: egui::Pos2, max: egui::Vec2) -> bool {
        let Some(sp) = self.team_species(name) else { return false };
        if self.sprites.get(&sp, shiny).is_none() {
            return false;
        }
        self.draw_sprite_centered(p, &sp, shiny, center, max)
    }

    pub(super) fn page_team(&mut self, ui: &mut egui::Ui) {
        self.setup.team.teams();
        if self.setup.team.open.is_some() && self.setup.team.team().is_none() {
            self.setup.team.open = None;
        }
        if self.setup.team.open.is_some() {
            return self.team_page(ui);
        }
        self.team_library(ui);
    }

    // ───────────────────────────── the library ─────────────────────────────

    fn team_library(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "TEAM PLANNER", "Build a team, see what it covers, plan how to get every part of it in the game, and do the damage maths.");
        if let Some(e) = self.setup.team.load_error.clone() {
            self.team_notice(ui, kit::C_ORANGE, &e, true);
        }
        if let Some(e) = self.setup.team.save_error.clone() {
            self.team_notice(ui, kit::C_RED, &format!("Couldn't save your teams: {e}. Trying again…"), false);
        }
        if let Some((_, t)) = self.setup.team.deleted.clone() {
            let mut undo = false;
            let mut close = false;
            kit::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Deleted \"{}\"", t.name)).size(13.0).color(TEXT()));
                    ui.add_space(8.0);
                    undo = ui.link(RichText::new("Undo").size(13.0)).clicked();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = x_button(ui).clicked();
                    });
                });
            });
            ui.add_space(10.0);
            if undo {
                if let Some((i, t)) = self.setup.team.deleted.take() {
                    let f = self.setup.team.teams();
                    let i = i.min(f.teams.len());
                    f.teams.insert(i, t);
                    self.setup.team.touch();
                }
            } else if close {
                self.setup.team.deleted = None;
            }
        }
        let count = self.setup.team.teams().teams.len();
        if count == 0 && self.setup.team.import.is_none() {
            self.team_welcome(ui);
            return;
        }
        // actions
        let mut new_team = false;
        let mut paste = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            new_team = kit::accent_button_w(ui, egui::Button::new(RichText::new("+  New team").size(13.5)), 8.0, 120.0).clicked();
            paste = soft(ui, "Paste a team").clicked();
            ui.add_space(6.0);
            let w = (ui.available_width() - 4.0).clamp(160.0, 300.0);
            kit::search_box_keyed(ui, Some(egui::Id::new("team-lib-search")), &mut self.setup.team.lib_query, "Find a team or a Pokémon", w);
        });
        if new_team {
            self.team_new(None);
            return;
        }
        if paste {
            self.setup.team.import = Some(String::new());
            self.setup.team.import_note = None;
        }
        ui.add_space(10.0);
        if self.setup.team.import.is_some() {
            self.team_import_box(ui, None);
            ui.add_space(12.0);
        }
        // folders
        let mut folders: Vec<String> = self.setup.team.teams().teams.iter().map(|t| t.folder.trim().to_string()).filter(|f| !f.is_empty()).collect();
        folders.sort();
        folders.dedup();
        if !folders.is_empty() {
            let mut set: Option<Option<String>> = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                if chip(ui, &format!("All teams  {count}"), self.setup.team.folder.is_none(), ACCENT).clicked() {
                    set = Some(None);
                }
                for f in &folders {
                    let n = self.setup.team.teams().teams.iter().filter(|t| t.folder.trim() == f).count();
                    let on = self.setup.team.folder.as_deref() == Some(f.as_str());
                    if chip(ui, &format!("📁  {f}  {n}"), on, ACCENT).clicked() {
                        set = Some(if on { None } else { Some(f.clone()) });
                    }
                }
            });
            if let Some(s) = set {
                self.setup.team.folder = s;
            }
            ui.add_space(12.0);
        } else {
            self.setup.team.folder = None;
        }
        // the cards
        let q = self.setup.team.lib_query.trim().to_lowercase();
        let folder = self.setup.team.folder.clone();
        let mut list: Vec<Team> = self
            .setup
            .team
            .teams()
            .teams
            .iter()
            .filter(|t| folder.as_ref().is_none_or(|f| t.folder.trim() == f))
            .filter(|t| q.is_empty() || t.name.to_lowercase().contains(&q) || t.members.iter().any(|m| m.species.to_lowercase().contains(&q) || m.nickname.to_lowercase().contains(&q)))
            .cloned()
            .collect();
        list.sort_by(|a, b| b.updated.cmp(&a.updated));
        if list.is_empty() {
            kit::card(ui, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("No team has \"{}\" in it", self.setup.team.lib_query.trim())).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.add_space(6.0);
            });
            return;
        }
        let avail = ui.available_width();
        let gap = 14.0;
        let cols = ((avail + gap) / (300.0 + gap)).floor().max(1.0) as usize;
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut open: Option<u64> = None;
        let mut dup: Option<u64> = None;
        let mut del: Option<u64> = None;
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for t in row {
                    let resp = self.team_card(ui, t, cw);
                    if resp.clicked() {
                        open = Some(t.id);
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Open").clicked() {
                            open = Some(t.id);
                            ui.close_menu();
                        }
                        if ui.button("Duplicate").clicked() {
                            dup = Some(t.id);
                            ui.close_menu();
                        }
                        if ui.button("Delete").clicked() {
                            del = Some(t.id);
                            ui.close_menu();
                        }
                    });
                }
            });
            ui.add_space(gap);
        }
        ui.label(small("Right-click a team to duplicate or delete it. Teams are saved on this PC, with backups of the last few versions."));
        if let Some(id) = open {
            self.setup.team.open_team(id);
            self.setup.scroll_top = true;
        }
        if let Some(id) = dup {
            self.team_duplicate(id);
        }
        if let Some(id) = del {
            self.team_delete(id);
        }
    }

    fn team_notice(&mut self, ui: &mut egui::Ui, col: Color32, text: &str, closable: bool) {
        let mut close = false;
        egui::Frame::none()
            .fill(theme::blend(CARD(), col, 0.12))
            .stroke(Stroke::new(1.0f32, theme::blend(LINE(), col, 0.5)))
            .rounding(10.0)
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(text).size(13.0).color(TEXT()));
                    if closable {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            close = x_button(ui).clicked();
                        });
                    }
                });
            });
        ui.add_space(10.0);
        if close {
            self.setup.team.load_error = None;
        }
    }

    /// No teams yet: what the planner does, and ways to start.
    fn team_welcome(&mut self, ui: &mut egui::Ui) {
        let full = ui.available_width();
        let mut start = false;
        let mut paste = false;
        let mut sample: Option<&str> = None;
        kit::card(ui, |ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("START HERE").font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(MUTED(), ACCENT, 0.6)));
            ui.add_space(4.0);
            ui.label(RichText::new("Plan your first team").font(crate::fonts::sans_bold_font(26.0)).color(TEXT()));
            ui.add_space(4.0);
            ui.label(RichText::new("Pick up to six Pokémon, give them moves, items and EVs, and the planner works out the rest.").size(13.0).color(MUTED()));
            ui.add_space(16.0);
            let parts = [
                ("🐾", ACCENT, "Build", "Showdown-style sets: moves each one can really learn here, live stats."),
                ("📊", kit::C_CYAN, "Analysis", "What hits you hard, what you hit hard, who's fastest."),
                ("📜", kit::C_GOLD, "Checklist", "Where to catch, which TMs and tutors, items to buy: tick it off."),
                ("💥", kit::C_RED, "Damage calc", "Platinum's own damage maths, against any set."),
            ];
            let gap = 12.0;
            let cols = if full > 820.0 { 4 } else { 2 };
            let w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in parts.chunks(cols) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (icon, col, title, text) in row {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(w, 112.0), Sense::hover());
                        let p = ui.painter();
                        p.rect_filled(r, 12.0, theme::blend(kit::sunken(), *col, 0.08));
                        kit::icon_chip(p, Rect::from_min_size(r.min + egui::vec2(14.0, 14.0), egui::vec2(30.0, 30.0)), icon, *col, false);
                        p.text(r.min + egui::vec2(52.0, 29.0), Align2::LEFT_CENTER, *title, crate::fonts::sans_bold_font(14.0), TEXT());
                        let g = p.layout(text.to_string(), crate::fonts::sans_font(12.0), MUTED(), w - 28.0);
                        p.galley(r.min + egui::vec2(14.0, 56.0), g, MUTED());
                    }
                });
                ui.add_space(gap);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                start = kit::accent_button_w(ui, egui::Button::new(RichText::new("+  New team").size(14.0)), 8.0, 140.0).clicked();
                paste = soft(ui, "Paste a team from Showdown").clicked();
            });
            ui.add_space(14.0);
            ui.label(caps("OR START FROM A CLASSIC"));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                for (name, _) in SAMPLES {
                    if chip(ui, name, false, ACCENT).clicked() {
                        sample = Some(name);
                    }
                }
            });
            ui.add_space(8.0);
        });
        if start {
            self.team_new(None);
        } else if paste {
            self.setup.team.import = Some(String::new());
            self.setup.team.import_note = None;
        } else if let Some(name) = sample {
            self.team_new(Some(name));
        }
    }

    /// One team in the library.
    fn team_card(&mut self, ui: &mut egui::Ui, t: &Team, w: f32) -> egui::Response {
        let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 164.0), Sense::click());
        if !ui.is_rect_visible(r) {
            return resp;
        }
        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
        let r2 = r.translate(egui::vec2(0.0, -2.0 * ht));
        let p = ui.painter().clone();
        p.add(egui::Shape::Rect(egui::epaint::RectShape::filled(r2, 14.0, theme::blend(CARD(), CARD_HOVER(), ht * 0.6))));
        p.rect_stroke(r2, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), ACCENT, 0.15 + 0.45 * ht)));
        // name and badges
        let f = crate::fonts::sans_bold_font(15.5);
        let name = if t.name.trim().is_empty() { "Untitled team" } else { t.name.as_str() };
        let wb = world_badge(&p, egui::pos2(r2.max.x - 14.0, r2.min.y + 22.0), t.world, true);
        p.text(r2.min + egui::vec2(16.0, 22.0), Align2::LEFT_CENTER, super::charts::clip(ui, name, &f, w - 46.0 - wb), f, TEXT());
        let sub = format!(
            "{}{} Pokémon  ·  edited {}",
            if t.folder.trim().is_empty() { String::new() } else { format!("📁 {}  ·  ", t.folder.trim()) },
            t.members.len(),
            ago(t.updated)
        );
        let f2 = crate::fonts::sans_font(11.5);
        p.text(r2.min + egui::vec2(16.0, 42.0), Align2::LEFT_CENTER, super::charts::clip(ui, &sub, &f2, w - 32.0), f2, MUTED());
        // the six
        let slot = ((w - 32.0) / 6.0).min(52.0);
        for i in 0..6 {
            let c = egui::pos2(r2.min.x + 16.0 + slot * (i as f32 + 0.5), r2.min.y + 88.0);
            p.circle_filled(c, slot / 2.0 - 3.0, kit::sunken());
            if let Some(m) = t.members.get(i) {
                let col = battle::data().species(&m.species).and_then(|s| s.types_in(t.world).first().cloned()).map(|t| type_color(&t)).unwrap_or(kit::C_SLATE);
                p.circle_filled(c, slot / 2.0 - 3.0, theme::blend(kit::sunken(), col, 0.22));
                if !self.team_sprite(&p, &m.species, m.shiny, c, egui::vec2(slot + 6.0, slot + 6.0)) {
                    p.text(c, Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(12.0), MUTED());
                }
            }
        }
        // checklist progress
        let (done, total) = super::team_check::team_progress(self.wiki.as_deref(), t);
        let bar = Rect::from_min_size(egui::pos2(r2.min.x + 16.0, r2.max.y - 22.0), egui::vec2(w - 32.0 - 90.0, 6.0));
        p.rect_filled(bar, 3.0, kit::sunken());
        if total > 0 {
            let frac = done as f32 / total as f32;
            p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bar.width() * frac, bar.height())), 3.0, if done == total { kit::C_GREEN } else { ACCENT });
        }
        let label = if total == 0 { "No checklist yet".to_string() } else if done == total { "Ready ✔".into() } else { format!("{done}/{total} done") };
        p.text(egui::pos2(r2.max.x - 16.0, bar.center().y), Align2::RIGHT_CENTER, label, crate::fonts::sans_font(11.5), MUTED());
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    pub(super) fn team_new(&mut self, sample: Option<&str>) {
        let id = self.setup.team.new_id();
        let n = self.setup.team.teams().teams.len() + 1;
        let world = if self.cfg.info_modern { World::Modern } else { World::Platinum };
        let mut t = Team::new(id, &format!("Team {n}"), world);
        if let Some(s) = sample.and_then(|s| SAMPLES.iter().find(|x| x.0 == s)) {
            t.name = s.0.to_string();
            t.world = World::Platinum;
            t.members = teams::import(s.1);
            for m in &mut t.members {
                m.level = 50;
            }
        }
        if let Some(f) = self.setup.team.folder.clone() {
            t.folder = f;
        }
        self.setup.team.teams().teams.push(t);
        self.setup.team.touch();
        self.setup.team.open_team(id);
        self.setup.scroll_top = true;
    }

    fn team_duplicate(&mut self, id: u64) {
        let new = self.setup.team.new_id();
        let f = self.setup.team.teams();
        if let Some(i) = f.teams.iter().position(|t| t.id == id) {
            let mut t = f.teams[i].clone();
            t.id = new;
            t.name = format!("{} (copy)", t.name);
            t.created = now_secs();
            t.updated = now_secs();
            f.teams.insert(i + 1, t);
            self.setup.team.touch();
        }
    }

    pub(super) fn team_delete(&mut self, id: u64) {
        let f = self.setup.team.teams();
        if let Some(i) = f.teams.iter().position(|t| t.id == id) {
            let t = f.teams.remove(i);
            self.setup.team.deleted = Some((i, t));
            if self.setup.team.open == Some(id) {
                self.setup.team.open = None;
            }
            self.setup.team.touch();
        }
    }

    /// The paste-a-team box. With a team open, the members go into it.
    pub(super) fn team_import_box(&mut self, ui: &mut egui::Ui, into: Option<u64>) {
        let mut go = false;
        let mut cancel = false;
        kit::card(ui, |ui| {
            ui.label(RichText::new(if into.is_some() { "Add Pokémon from text" } else { "Paste a team" }).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.add_space(2.0);
            ui.label(small("Showdown's export format works, one Pokémon or a whole team. Copy it from Showdown's teambuilder (Import/Export)."));
            ui.add_space(8.0);
            if let Some(text) = self.setup.team.import.as_mut() {
                egui::ScrollArea::vertical().id_salt("team-import").max_height(260.0).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(text)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(10)
                            .desired_width(f32::INFINITY)
                            .hint_text("Garchomp @ Choice Scarf\nAbility: Sand Veil\nEVs: 252 Atk / 4 SpD / 252 Spe\nJolly Nature\n- Outrage\n- Earthquake\n…"),
                    );
                });
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                go = kit::accent_button_w(ui, egui::Button::new(RichText::new(if into.is_some() { "Add to team" } else { "Make the team" }).size(13.0)), 8.0, 110.0).clicked();
                cancel = soft(ui, "Cancel").clicked();
                if let Some(n) = &self.setup.team.import_note {
                    ui.label(RichText::new(n).size(12.5).color(kit::C_ORANGE));
                }
            });
        });
        if cancel {
            self.setup.team.import = None;
            self.setup.team.import_note = None;
            return;
        }
        if !go {
            return;
        }
        let text = self.setup.team.import.clone().unwrap_or_default();
        let ms = teams::import(&text);
        if ms.is_empty() {
            self.setup.team.import_note = Some("Couldn't find a Pokémon in that text.".into());
            return;
        }
        let unknown: Vec<String> = ms.iter().filter(|m| battle::data().species(&m.species).is_none()).map(|m| m.species.clone()).collect();
        let ms: Vec<Member> = ms.into_iter().filter(|m| battle::data().species(&m.species).is_some()).collect();
        let world = ms.iter().any(|m| battle::data().species(&m.species).is_some_and(|s| s.modern_only()));
        match into {
            Some(id) => {
                let room;
                {
                    let Some(t) = self.setup.team.teams().teams.iter_mut().find(|t| t.id == id) else { return };
                    room = 6usize.saturating_sub(t.members.len());
                    t.members.extend(ms.iter().take(room).cloned());
                    if world {
                        t.world = World::Modern;
                    }
                }
                self.setup.team.touch();
                self.setup.team.import = None;
                self.setup.team.import_note = None;
                if ms.len() > room {
                    self.setup.team.import_note = Some(format!("The team was full, so {} were left out.", ms.len() - room));
                }
            }
            None => {
                let id = self.setup.team.new_id();
                let n = self.setup.team.teams().teams.len() + 1;
                let mut t = Team::new(id, &format!("Team {n}"), if world { World::Modern } else { World::Platinum });
                t.members = ms.into_iter().take(6).collect();
                if let Some(f) = self.setup.team.folder.clone() {
                    t.folder = f;
                }
                self.setup.team.teams().teams.push(t);
                self.setup.team.touch();
                self.setup.team.import = None;
                self.setup.team.import_note = None;
                self.setup.team.open_team(id);
                self.setup.team.picker = None;
            }
        }
        if !unknown.is_empty() {
            self.setup.team.import_note = Some(format!("Not found, left out: {}", unknown.join(", ")));
        }
    }

    // ───────────────────────────── an open team ─────────────────────────────

    fn team_page(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.setup.team.team().cloned() else { return };
        // keys: Esc closes a picker, then the team
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.setup.team.picker.is_some() && !t.members.is_empty() {
                self.setup.team.picker = None;
            } else if self.setup.team.import.is_none() {
                self.setup.team.open = None;
                return;
            }
        }
        // the bar: back, name, world, actions
        let mut back = false;
        let mut name = t.name.clone();
        let mut folder = t.folder.clone();
        let mut world = t.world;
        let mut export = false;
        let mut dup = false;
        let mut del = false;
        let mut battle = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            back = ui.add(egui::Button::new(RichText::new("‹  Teams").size(13.0)).fill(kit::btn_fill()).min_size(egui::vec2(0.0, 30.0))).clicked();
            ui.add_space(4.0);
            let w = (ui.available_width() - 580.0).clamp(160.0, 420.0);
            ui.add(
                egui::TextEdit::singleline(&mut name)
                    .font(crate::fonts::sans_bold_font(20.0))
                    .frame(false)
                    .hint_text("Name this team")
                    .desired_width(w),
            )
            .on_hover_text("Rename the team");
            kit::segmented(ui, &mut world, &[(World::Platinum, "Platinum"), (World::Modern, "MODERN")]);
            battle = soft(ui, "⚔  Battle Sim").on_hover_text("Take this team into the Battle Sim").clicked();
            export = soft(ui, "Export").clicked();
            dup = soft(ui, "Duplicate").clicked();
            del = soft(ui, "Delete").clicked();
        });
        if battle {
            let n = t.members.len().min(6);
            self.setup.sim.file();
            self.setup.sim.started = true;
            let s = &mut self.setup.sim.setup;
            s.team = Some(t.id);
            s.picks = (0..n).collect();
            if s.mode == crate::sim::Mode::Catch {
                s.mode = crate::sim::Mode::Wild;
            }
            self.go_to(super::Tab::Sim);
            self.setup.scroll_top = true;
            return;
        }
        ui.horizontal(|ui| {
            ui.add_space(96.0);
            ui.label(caps("FOLDER"));
            ui.add(egui::TextEdit::singleline(&mut folder).hint_text("none").desired_width(170.0).font(egui::FontId::proportional(12.5)));
            ui.add_space(8.0);
            ui.label(small(match world {
                World::Platinum => "Platinum's own Pokémon, stats and abilities.",
                World::Modern => "MODERN POKéMON world: newer Pokémon, Megas, the Fairy type and newer stats.",
            }));
        });
        if back {
            self.setup.team.open = None;
            self.setup.team.picker = None;
            self.setup.scroll_top = true;
            return;
        }
        if name != t.name || folder != t.folder || world != t.world {
            if let Some(tm) = self.setup.team.team() {
                tm.name = name;
                tm.folder = folder;
                tm.world = world;
            }
            self.setup.team.touch();
        }
        if export {
            self.setup.team.export_open = !self.setup.team.export_open;
        }
        if dup {
            self.team_duplicate(t.id);
        }
        if del {
            self.setup.team.confirm_delete = Some(t.id);
        }
        ui.add_space(10.0);
        if self.setup.team.confirm_delete == Some(t.id) {
            let mut yes = false;
            let mut no = false;
            egui::Frame::none()
                .fill(theme::blend(CARD(), kit::C_RED, 0.10))
                .stroke(Stroke::new(1.0f32, theme::blend(LINE(), kit::C_RED, 0.5)))
                .rounding(10.0)
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Delete \"{}\"?", t.name)).size(13.5).color(TEXT()));
                        ui.label(small("You can undo it from the teams list."));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            no = soft(ui, "Keep it").clicked();
                            yes = ui.add(egui::Button::new(RichText::new("Delete").color(Color32::WHITE).size(13.0)).fill(kit::C_RED).min_size(egui::vec2(0.0, 30.0))).clicked();
                        });
                    });
                });
            ui.add_space(10.0);
            if yes {
                self.setup.team.confirm_delete = None;
                self.team_delete(t.id);
                return;
            }
            if no {
                self.setup.team.confirm_delete = None;
            }
        }
        if self.setup.team.export_open {
            self.team_export_box(ui, &t);
            ui.add_space(12.0);
        }
        let mut view = self.setup.team.view;
        kit::tab_bar(ui, &mut view, &[(View::Build, "🐾  Build"), (View::Analysis, "📊  Analysis"), (View::Checklist, "📜  Checklist"), (View::Calc, "💥  Damage calc")]);
        if view != self.setup.team.view {
            self.setup.team.view = view;
            self.setup.team.picker = None;
        }
        ui.add_space(14.0);
        let world = self.setup.team.team().map(|t| t.world).unwrap_or_default();
        match self.setup.team.view {
            View::Build => self.team_build(ui),
            View::Analysis => self.team_analysis(ui),
            View::Checklist => self.team_checklist(ui),
            View::Calc => self.team_calc(ui, world),
        }
    }

    fn team_export_box(&mut self, ui: &mut egui::Ui, t: &Team) {
        let mut text = teams::export_team(t);
        let mut copy = false;
        let mut close = false;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Export").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.label(small("Showdown's format: paste it into Showdown's teambuilder, or share it."));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = x_button(ui).clicked();
                    copy = kit::accent_button_w(ui, egui::Button::new(RichText::new("Copy").size(13.0)), 8.0, 70.0).clicked();
                });
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical().id_salt("team-export").max_height(300.0).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut text).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).interactive(true));
            });
        });
        if copy {
            ui.ctx().copy_text(teams::export_team(t));
            self.setup.team.import_note = None;
        }
        if close {
            self.setup.team.export_open = false;
        }
    }

    /// Add a Pokémon to a team from elsewhere in the app (Pokédex, Game Info).
    pub(super) fn team_add_from(&mut self, species: &str, team: Option<u64>) {
        if battle::data().species(species).is_none() {
            return;
        }
        let world = if self.cfg.info_modern { World::Modern } else { World::Platinum };
        let id = match team {
            Some(id) => id,
            None => {
                let id = self.setup.team.new_id();
                let n = self.setup.team.teams().teams.len() + 1;
                self.setup.team.teams().teams.push(Team::new(id, &format!("Team {n}"), world));
                id
            }
        };
        let slot;
        {
            let Some(t) = self.setup.team.teams().teams.iter_mut().find(|t| t.id == id) else { return };
            if t.members.len() >= 6 {
                return;
            }
            let m = Member::new(species, t.world);
            if battle::data().species(species).is_some_and(|s| s.modern_only()) {
                t.world = World::Modern;
            }
            t.members.push(m);
            slot = t.members.len() - 1;
        }
        self.setup.team.touch();
        self.go_to(Tab::Team);
        self.setup.team.open = Some(id);
        self.setup.team.slot = slot;
        self.setup.team.view = View::Build;
        self.setup.team.picker = None;
        self.setup.scroll_top = true;
    }

    /// Teams with room, for "Add to team" menus: (id, name, members).
    pub(super) fn team_choices(&mut self) -> Vec<(u64, String, usize)> {
        let mut v: Vec<(u64, String, usize, u64)> = self.setup.team.teams().teams.iter().map(|t| (t.id, t.name.clone(), t.members.len(), t.updated)).collect();
        v.sort_by(|a, b| b.3.cmp(&a.3));
        v.into_iter().map(|(a, b, c, _)| (a, b, c)).collect()
    }
}

/// "+ Add to team" with a menu of teams that have room, and "New team".
pub(super) fn add_to_team_menu(ui: &mut egui::Ui, teams: &[(u64, String, usize)], out: &mut Option<Option<u64>>) {
    ui.menu_button(RichText::new("+  Add to team").size(13.0), |ui| {
        ui.set_min_width(200.0);
        for (id, name, n) in teams.iter().take(12) {
            let label = format!("{}  ({n}/6)", if name.trim().is_empty() { "Untitled team" } else { name });
            if ui.add_enabled(*n < 6, egui::Button::new(label)).on_disabled_hover_text("That team is full").clicked() {
                *out = Some(Some(*id));
                ui.close_menu();
            }
        }
        if !teams.is_empty() {
            ui.separator();
        }
        if ui.button("New team").clicked() {
            *out = Some(None);
            ui.close_menu();
        }
    })
    .response
    .on_hover_text("Put it in a Team Planner team");
}

/// "PLATINUM" / "MODERN" badge, right-aligned at `right`; returns its width.
pub(super) fn world_badge(p: &egui::Painter, right: egui::Pos2, w: World, small: bool) -> f32 {
    let col = match w {
        World::Platinum => kit::C_SLATE,
        World::Modern => kit::C_PINK,
    };
    let g = p.layout_no_wrap(w.label().to_uppercase(), crate::fonts::sans_bold_font(if small { 9.5 } else { 10.5 }), super::charts::ink(col));
    let r = Rect::from_min_size(egui::pos2(right.x - g.size().x - 14.0, right.y - 9.0), egui::vec2(g.size().x + 14.0, 18.0));
    p.rect_filled(r, 9.0, super::charts::faint(col, 0.18));
    p.galley(egui::pos2(r.min.x + 7.0, r.center().y - g.size().y / 2.0), g, Color32::WHITE);
    r.width()
}

/// Ready-made teams to start from (well-known Platinum-era sets).
pub(super) const SAMPLES: [(&str, &str); 3] = [
    (
        "Sandstorm",
        "Tyranitar @ Choice Band\nAbility: Sand Stream\nEVs: 252 Atk / 4 SpD / 252 Spe\nAdamant Nature\n- Stone Edge\n- Crunch\n- Pursuit\n- Earthquake\n\nGarchomp @ Choice Scarf\nAbility: Sand Veil\nEVs: 252 Atk / 4 SpD / 252 Spe\nJolly Nature\n- Outrage\n- Earthquake\n- Stone Edge\n- Fire Fang\n\nSkarmory @ Leftovers\nAbility: Keen Eye\nEVs: 252 HP / 252 Def / 4 SpD\nImpish Nature\n- Spikes\n- Stealth Rock\n- Roost\n- Whirlwind\n\nRotom-Wash @ Leftovers\nAbility: Levitate\nEVs: 252 HP / 4 SpA / 252 Spe\nTimid Nature\n- Thunderbolt\n- Shadow Ball\n- Will-O-Wisp\n- Pain Split\n\nLucario @ Life Orb\nAbility: Inner Focus\nEVs: 252 Atk / 4 Def / 252 Spe\nJolly Nature\n- Swords Dance\n- Close Combat\n- Crunch\n- Extreme Speed\n\nVaporeon @ Leftovers\nAbility: Water Absorb\nEVs: 252 HP / 252 Def / 4 SpD\nBold Nature\n- Surf\n- Wish\n- Protect\n- Roar\n",
    ),
    (
        "Rain dance",
        "Kingdra @ Life Orb\nAbility: Swift Swim\nEVs: 252 SpA / 4 SpD / 252 Spe\nModest Nature\n- Hydro Pump\n- Draco Meteor\n- Surf\n- Rain Dance\n\nLudicolo @ Leftovers\nAbility: Swift Swim\nEVs: 4 HP / 252 SpA / 252 Spe\nModest Nature\n- Surf\n- Energy Ball\n- Ice Beam\n- Rain Dance\n\nBronzong @ Damp Rock\nAbility: Levitate\nEVs: 252 HP / 252 Def / 4 SpD\nRelaxed Nature\n- Rain Dance\n- Gyro Ball\n- Earthquake\n- Explosion\n\nKabutops @ Choice Band\nAbility: Swift Swim\nEVs: 4 HP / 252 Atk / 252 Spe\nAdamant Nature\n- Waterfall\n- Stone Edge\n- X-Scissor\n- Aqua Jet\n\nMagnezone @ Choice Specs\nAbility: Magnet Pull\nEVs: 4 HP / 252 SpA / 252 Spe\nModest Nature\n- Thunderbolt\n- Flash Cannon\n- Hidden Power\n- Thunder\n\nGyarados @ Leftovers\nAbility: Intimidate\nEVs: 4 HP / 252 Atk / 252 Spe\nAdamant Nature\n- Dragon Dance\n- Waterfall\n- Earthquake\n- Stone Edge\n",
    ),
    (
        "Story run (balanced)",
        "Infernape @ Life Orb\nAbility: Blaze\nEVs: 252 Atk / 4 SpA / 252 Spe\nNaive Nature\n- Close Combat\n- Flamethrower\n- Mach Punch\n- U-turn\n\nStaraptor @ Choice Band\nAbility: Intimidate\nEVs: 252 Atk / 4 SpD / 252 Spe\nJolly Nature\n- Brave Bird\n- Close Combat\n- Return\n- U-turn\n\nLuxray @ Choice Band\nAbility: Intimidate\nEVs: 252 Atk / 4 SpD / 252 Spe\nAdamant Nature\n- Spark\n- Crunch\n- Ice Fang\n- Fire Fang\n\nRoserade @ Life Orb\nAbility: Natural Cure\nEVs: 4 HP / 252 SpA / 252 Spe\nTimid Nature\n- Leaf Storm\n- Sludge Bomb\n- Shadow Ball\n- Sleep Powder\n\nGarchomp @ Yache Berry\nAbility: Sand Veil\nEVs: 252 Atk / 4 SpD / 252 Spe\nJolly Nature\n- Earthquake\n- Dragon Claw\n- Stone Edge\n- Swords Dance\n\nFloatzel @ Leftovers\nAbility: Swift Swim\nEVs: 252 Atk / 4 SpD / 252 Spe\nAdamant Nature\n- Waterfall\n- Crunch\n- Ice Punch\n- Bulk Up\n",
    ),
];
