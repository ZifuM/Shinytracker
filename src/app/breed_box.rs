//! Breeding Planner: your Breeders (breeding Pokémon kept to use again)
//! and the Hatch log (each Egg as it hatches, which ones hit the goal,
//! which beat a parent). Everything is entered by hand; nothing reads the
//! game.

use super::breed_home::{
    cap_first, iv_chips, iv_text, move_name, name_of, nature_label, one_in, role, BreedView, DAD, GOLD, MOM,
};
use super::team_calc::search_pick;
use super::team_home::{caps, chip, small, soft, x_button};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{World, STATS};
use crate::breed::{self, Breeder, Gender, Hatch, Held, Parent, Slot};
use crate::config::HuntMethod;
use crate::theme;
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum Filter {
    #[default]
    All,
    Ditto,
    Masuda,
    Five,
}

#[derive(Default)]
pub(super) struct BoxView {
    pub q: String,
    pub filter: Filter,
    /// The breeder being added (id None) or changed.
    pub editing: Option<(Option<u64>, Breeder)>,
    pub importing: bool,
    /// Just removed, for Undo: (where it was, what it was).
    pub removed: Option<(usize, Breeder)>,
    /// The Egg being logged.
    pub entry: Option<Hatch>,
    /// The log shows every pair, not just the one in the Day Care.
    pub log_all: bool,
    pub removed_hatch: Option<(usize, Hatch)>,
    /// How many log rows to show.
    pub shown: usize,
}

/// "Jolly Garchomp ♀ · 5×31" for pickers.
pub(super) fn breeder_label(w: &Wiki, b: &Breeder) -> String {
    let sp = name_of(w, &b.parent.species);
    let who = if b.name.trim().is_empty() { sp } else { format!("{} ({sp})", b.name.trim()) };
    let g = match b.parent.gender() {
        Gender::Male => " ♂",
        Gender::Female => " ♀",
        Gender::Genderless => "",
    };
    let nat = b.parent.nature.as_deref().map(|n| format!("{n} ")).unwrap_or_default();
    format!("{nat}{who}{g} · {}×31  #{}", b.parent.perfect(), b.id % 1000)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Six small stat pills: gold for 31.
fn stat_pills(p: &egui::Painter, at: egui::Pos2, ivs: &[Option<u8>; 6], w: f32) -> f32 {
    let gap = 3.0;
    let pw = ((w - gap * 5.0) / 6.0).max(20.0);
    for (s, v) in ivs.iter().enumerate() {
        let r = Rect::from_min_size(at + egui::vec2(s as f32 * (pw + gap), 0.0), egui::vec2(pw, 30.0));
        let on = *v == Some(31);
        p.rect_filled(r, 6.0, if on { theme::blend(CARD(), GOLD, 0.24) } else { kit::sunken() });
        if on {
            p.rect_stroke(r, 6.0, Stroke::new(1.0f32, theme::blend(LINE(), GOLD, 0.7)));
        }
        p.text(egui::pos2(r.center().x, r.min.y + 8.0), Align2::CENTER_CENTER, STATS[s], crate::fonts::sans_bold_font(8.5), MUTED());
        p.text(
            egui::pos2(r.center().x, r.min.y + 20.0),
            Align2::CENTER_CENTER,
            iv_text(*v),
            crate::fonts::sans_bold_font(11.5),
            if on { super::charts::ink(GOLD) } else { MUTED() },
        );
    }
    pw * 6.0 + gap * 5.0
}

/// A small rounded tag; returns its width.
pub(super) fn tag(p: &egui::Painter, at: egui::Pos2, text: &str, col: Color32) -> f32 {
    let g = p.layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(10.5), super::charts::ink(col));
    let r = Rect::from_min_size(at, g.size() + egui::vec2(14.0, 8.0));
    p.rect_filled(r, r.height() / 2.0, super::charts::faint(col, 0.16));
    p.galley(r.min + egui::vec2(7.0, 4.0), g, Color32::WHITE);
    r.width()
}

/// Where a Pokémon goes in the Day Care: an empty spot, else the parent
/// it would stand in for (same role), else the second spot.
fn place(dc: &mut breed::DayCare, p: Parent) -> (usize, Option<String>) {
    let slot = if dc.a.species.is_empty() {
        0
    } else if dc.b.species.is_empty() {
        1
    } else {
        let same = |q: &Parent| {
            if p.is_ditto() || q.is_ditto() {
                p.is_ditto() == q.is_ditto()
            } else {
                q.gender() == p.gender()
            }
        };
        if same(&dc.a) {
            0
        } else if same(&dc.b) {
            1
        } else {
            1
        }
    };
    // keep what the old one held: items belong to the Day Care plan
    let old = if slot == 0 { &mut dc.a } else { &mut dc.b };
    let carried = (p.held == Held::Nothing && old.held != Held::Nothing).then(|| old.held.name());
    let held = if p.held == Held::Nothing { old.held.clone() } else { p.held.clone() };
    *old = Parent { held, ..p };
    (slot, carried)
}

impl App {
    // ───────────────────────────── Breeders ─────────────────────────────

    pub(super) fn page_breeders(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let setup = self.setup.breed.file().setup;
        if self.setup.breed.boxv.editing.is_some() {
            self.breeder_editor(ui, w, setup);
        }
        // Undo a removal.
        if let Some((i, b)) = self.setup.breed.boxv.removed.clone() {
            let mut undo = false;
            let mut close = false;
            egui::Frame::none()
                .fill(theme::blend(CARD(), kit::C_SLATE, 0.12))
                .stroke(Stroke::new(1.0f32, LINE()))
                .rounding(10.0)
                .inner_margin(egui::Margin::symmetric(14.0, 8.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Removed {}.", breeder_label(w, &b))).size(13.0).color(TEXT()));
                        undo = ui.link(RichText::new("Undo").size(13.0)).clicked();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| close = x_button(ui).clicked());
                    });
                });
            ui.add_space(10.0);
            if undo {
                let f = self.setup.breed.file();
                let at = i.min(f.breeders.len());
                f.breeders.insert(at, b);
                self.setup.breed.touch();
                self.setup.breed.boxv.removed = None;
            } else if close {
                self.setup.breed.boxv.removed = None;
            }
        }

        // Toolbar: search, filters, add, import.
        let all = self.setup.breed.file().breeders.clone();
        let mut add = false;
        let mut import = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            kit::search_box(ui, &mut self.setup.breed.boxv.q, "Search your Breeders…", 220.0);
            let f = &mut self.setup.breed.boxv.filter;
            for (v, l, tip) in [
                (Filter::All, "All", "Every one"),
                (Filter::Ditto, "Ditto", "Your Ditto"),
                (Filter::Masuda, "Masuda", "Traded from another player: Eggs are likelier to be shiny"),
                (Filter::Five, "5+ at 31", "Five or six stats at 31"),
            ] {
                if chip(ui, l, *f == v, GOLD).on_hover_text(tip).clicked() {
                    *f = v;
                }
            }
            ui.add_space(6.0);
            add = kit::accent_button_w(ui, egui::Button::new(RichText::new("+  Add one").size(13.0)), 8.0, 110.0).clicked();
            if super::app_modal::have(crate::apps::AppId::Team) {
                import = soft(ui, "From your teams").on_hover_text("Add Pokémon from your Team Planner teams").clicked();
            } else {
                super::app_modal::lock_btn(ui, crate::apps::AppId::Team, "From your teams");
            }
        });
        ui.add_space(12.0);
        if add {
            let f = self.setup.breed.file();
            let id = f.new_id();
            self.setup.breed.boxv.editing = Some((None, Breeder { id, added: id / 1000, ..Default::default() }));
            self.setup.breed.boxv.importing = false;
            self.setup.scroll_top = true;
        }
        if import {
            self.setup.breed.boxv.importing = !self.setup.breed.boxv.importing;
        }
        if self.setup.breed.boxv.importing {
            self.breeder_import(ui, w, setup.world);
        }

        if all.is_empty() {
            super::sim_home::side_card(ui, GOLD, "YOUR BREEDERS", |ui| {
                ui.label(RichText::new("Keep your breeding Pokémon here").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Your perfect Ditto, a parent with the right nature, one that knows an egg move: add them once and put them in the Day Care with one click. Add one here, pick them from your Team Planner teams, use \"Save to your Breeders\" in the Day Care, or keep a good Egg from the Hatch log.",
                        )
                        .size(13.0)
                        .color(MUTED()),
                    )
                    .wrap(),
                );
            });
            return;
        }

        // The cards.
        let q = self.setup.breed.boxv.q.trim().to_lowercase();
        let filter = self.setup.breed.boxv.filter;
        let shown: Vec<&Breeder> = all
            .iter()
            .rev()
            .filter(|b| q.is_empty() || breeder_label(w, b).to_lowercase().contains(&q) || b.notes.to_lowercase().contains(&q))
            .filter(|b| match filter {
                Filter::All => true,
                Filter::Ditto => b.parent.is_ditto(),
                Filter::Masuda => b.parent.masuda,
                Filter::Five => b.parent.perfect() >= 5,
            })
            .collect();
        ui.label(small(format!("{} of {} · newest first", shown.len(), all.len())));
        ui.add_space(6.0);
        if shown.is_empty() {
            ui.label(small("Nothing matches."));
            return;
        }
        let avail = ui.available_width();
        let gap = 14.0;
        let cols = ((avail + gap) / (330.0 + gap)).floor().max(1.0) as usize;
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut act: Option<(u64, char)> = None;
        for row in shown.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for b in row {
                    ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        if let Some(a) = self.breeder_card(ui, w, b, setup.world) {
                            act = Some((b.id, a));
                        }
                    });
                }
            });
            ui.add_space(gap);
        }
        if let Some((id, a)) = act {
            let f = self.setup.breed.file();
            let Some(i) = f.breeders.iter().position(|b| b.id == id) else { return };
            match a {
                'u' => {
                    let p = f.breeders[i].parent.clone();
                    let name = name_of(w, &p.species);
                    let (slot, carried) = place(&mut f.daycare, p);
                    self.setup.breed.touch();
                    self.setup.breed.view = BreedView::DayCare;
                    self.setup.scroll_top = true;
                    let side = if slot == 0 { "left" } else { "right" };
                    let msg = match carried {
                        Some(item) => format!("Put {name} in the Day Care ({side}), holding the {item} the last one had."),
                        None => format!("Put {name} in the Day Care ({side})."),
                    };
                    self.setup.breed.flash = Some((msg, Instant::now()));
                }
                'e' => {
                    let b = f.breeders[i].clone();
                    self.setup.breed.boxv.editing = Some((Some(id), b));
                    self.setup.scroll_top = true;
                }
                'x' => {
                    let b = f.breeders.remove(i);
                    self.setup.breed.boxv.removed = Some((i, b));
                    self.setup.breed.touch();
                }
                _ => {}
            }
        }
    }

    /// One breeder: returns 'u' (use), 'e' (edit) or 'x' (remove).
    fn breeder_card(&mut self, ui: &mut egui::Ui, w: &Wiki, b: &Breeder, world: World) -> Option<char> {
        let p = &b.parent;
        let (_, col) = role(p, 0);
        let mut out = None;
        egui::Frame::none()
            .fill(CARD())
            .rounding(14.0)
            .stroke(Stroke::new(1.0f32, LINE()))
            .shadow(kit::card_shadow())
            .inner_margin(egui::Margin::same(14.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let full = ui.available_width();
                let (r, _) = ui.allocate_exact_size(egui::vec2(full, 70.0), Sense::hover());
                let pnt = ui.painter().clone();
                let tile = Rect::from_min_size(r.min, egui::vec2(66.0, 66.0));
                pnt.rect_filled(tile, 12.0, theme::blend(kit::sunken(), col, 0.08));
                if !self.team_sprite(&pnt, &p.species, false, tile.center(), egui::vec2(58.0, 58.0)) {
                    pnt.text(tile.center(), Align2::CENTER_CENTER, "…", crate::fonts::sans_bold_font(14.0), MUTED());
                }
                let x = tile.max.x + 12.0;
                let sp = name_of(w, &p.species);
                let title = if b.name.trim().is_empty() { sp.clone() } else { b.name.trim().to_string() };
                let f = crate::fonts::sans_bold_font(15.5);
                let tw = full - 80.0 - 24.0;
                let g = pnt.text(egui::pos2(x, r.min.y + 13.0), Align2::LEFT_CENTER, super::charts::clip(ui, &title, &f, tw), f, TEXT());
                let gs = match p.gender() {
                    Gender::Male => Some(("♂", DAD)),
                    Gender::Female => Some(("♀", MOM)),
                    Gender::Genderless => None,
                };
                if let Some((s, c)) = gs.filter(|_| !p.is_ditto()) {
                    pnt.text(egui::pos2(g.max.x + 6.0, r.min.y + 13.0), Align2::LEFT_CENTER, s, crate::fonts::sans_bold_font(14.0), super::charts::ink(c));
                }
                let mut line2 = vec![];
                if !b.name.trim().is_empty() {
                    line2.push(sp.clone());
                }
                line2.push(p.nature.clone().unwrap_or_else(|| "Nature not set".into()));
                if !p.is_ditto() {
                    let ab = match p.ability {
                        Slot::Hidden => breed::hidden_ability(&p.species).map(|h| format!("{h} (hidden)")),
                        Slot::Second => breed::normal_abilities(&p.species, world).get(1).cloned(),
                        Slot::First => breed::normal_abilities(&p.species, world).first().cloned(),
                    };
                    if let Some(a) = ab {
                        line2.push(a);
                    }
                }
                let f2 = crate::fonts::sans_font(12.0);
                pnt.text(egui::pos2(x, r.min.y + 33.0), Align2::LEFT_CENTER, super::charts::clip(ui, &line2.join(" · "), &f2, tw), f2, MUTED());
                // Tags.
                let mut tx = x;
                let ty = r.min.y + 46.0;
                let mut tags: Vec<(String, Color32)> = vec![];
                if p.held != Held::Nothing {
                    tags.push((p.held.name(), kit::C_TEAL));
                }
                if !p.is_ditto() && !p.moves.is_empty() {
                    tags.push((format!("{} move{}", p.moves.len(), if p.moves.len() == 1 { "" } else { "s" }), kit::C_TEAL));
                }
                if p.masuda {
                    tags.push(("Masuda".into(), kit::C_VIOLET));
                } else if p.other_trainer {
                    tags.push(("Other trainer".into(), kit::C_SLATE));
                }
                if p.partner {
                    tags.push(("Partner".into(), kit::C_RED));
                }
                for (t, c) in tags {
                    let need = pnt.layout_no_wrap(t.clone(), crate::fonts::sans_bold_font(10.5), TEXT()).size().x + 14.0;
                    if tx + need > r.max.x {
                        break;
                    }
                    tx += tag(&pnt, egui::pos2(tx, ty), &t, c) + 5.0;
                }
                ui.add_space(6.0);
                let (sr, _) = ui.allocate_exact_size(egui::vec2(full, 30.0), Sense::hover());
                stat_pills(ui.painter(), sr.min, &p.ivs, full);
                if !p.moves.is_empty() && !p.is_ditto() {
                    ui.add_space(6.0);
                    let names: Vec<String> = p.moves.iter().map(|m| move_name(w, m)).collect();
                    ui.add(egui::Label::new(small(names.join(" · "))).truncate());
                }
                if !b.notes.trim().is_empty() {
                    ui.add_space(4.0);
                    ui.add(egui::Label::new(RichText::new(b.notes.trim()).size(12.0).italics().color(MUTED())).truncate());
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if kit::accent_button_w(ui, egui::Button::new(RichText::new("Use in the Day Care").size(12.5)), 8.0, 0.0).clicked() {
                        out = Some('u');
                    }
                    if soft(ui, "Edit").clicked() {
                        out = Some('e');
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if x_button(ui).on_hover_text("Remove (you can undo)").clicked() {
                            out = Some('x');
                        }
                    });
                });
            });
        out
    }

    fn breeder_editor(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, setup: breed::Setup) {
        let Some((id, mut b)) = self.setup.breed.boxv.editing.take() else { return };
        let mut save = false;
        let mut cancel = false;
        ui.label(caps(if id.is_some() { "CHANGE A BREEDER" } else { "ADD A BREEDER" }));
        ui.add_space(6.0);
        let other = Parent::default();
        self.parent_card(ui, w, &mut b.parent, &other, 2, None, &setup, None);
        kit::card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
                ui.label(RichText::new("Name").size(12.5).color(MUTED()));
                ui.add(egui::TextEdit::singleline(&mut b.name).hint_text("Optional, e.g. \"Box 3 Ditto\"").desired_width(200.0));
                ui.label(RichText::new("Notes").size(12.5).color(MUTED()));
                ui.add(egui::TextEdit::singleline(&mut b.notes).hint_text("Where it came from, what it's for…").desired_width(300.0));
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let ready = !b.parent.species.is_empty();
                save = ui
                    .add_enabled_ui(ready, |ui| kit::accent_button_w(ui, egui::Button::new(RichText::new(if id.is_some() { "Save changes" } else { "Add it" }).size(13.0)), 8.0, 120.0))
                    .inner
                    .on_disabled_hover_text("Pick a Pokémon first")
                    .clicked();
                cancel = soft(ui, "Cancel").clicked();
            });
        });
        ui.add_space(6.0);
        if save {
            let name = name_of(w, &b.parent.species);
            let f = self.setup.breed.file();
            match id.and_then(|id| f.breeders.iter().position(|x| x.id == id)) {
                Some(i) => f.breeders[i] = b,
                None => f.breeders.push(b),
            }
            self.setup.breed.touch();
            self.setup.breed.flash = Some((format!("Saved {name}."), Instant::now()));
        } else if !cancel {
            self.setup.breed.boxv.editing = Some((id, b));
        }
    }

    /// Pick Pokémon from your Team Planner teams.
    fn breeder_import(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, world: World) {
        let teams = self.setup.team.teams().teams.clone();
        let mut add: Option<Parent> = None;
        let mut done = false;
        super::sim_home::side_card(ui, kit::C_TEAL, "FROM YOUR TEAMS", |ui| {
            if teams.iter().all(|t| t.members.is_empty()) {
                ui.label(small("Your Team Planner has no Pokémon yet."));
            }
            for t in teams.iter().filter(|t| !t.members.is_empty()) {
                ui.label(RichText::new(if t.name.trim().is_empty() { "Untitled team" } else { t.name.trim() }).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for m in &t.members {
                        let parent = breed::from_member(m, t.world, w);
                        let tile = 74.0;
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(tile, tile + 16.0), if parent.is_some() { Sense::click() } else { Sense::hover() });
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter().clone();
                        let box_r = Rect::from_min_size(r.min, egui::vec2(tile, tile));
                        p.rect_filled(box_r, 12.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                        if !self.team_sprite(&p, &m.species, m.shiny, box_r.center(), egui::vec2(tile - 10.0, tile - 10.0)) {
                            p.text(box_r.center(), Align2::CENTER_CENTER, "…", crate::fonts::sans_bold_font(14.0), MUTED());
                        }
                        if parent.is_none() {
                            p.rect_filled(box_r, 12.0, CARD().gamma_multiply(0.6));
                        }
                        let f = crate::fonts::sans_font(11.0);
                        p.text(egui::pos2(box_r.center().x, r.max.y - 6.0), Align2::CENTER_CENTER, super::charts::clip(ui, m.display_name(), &f, tile), f, TEXT());
                        let resp = match &parent {
                            Some(_) => resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Add to your Breeders (nature, IVs, Ability, item and moves come along)"),
                            None => resp.on_hover_text("Can't breed"),
                        };
                        if resp.clicked() {
                            add = parent;
                        }
                    }
                });
                ui.add_space(8.0);
            }
            done = soft(ui, "Done").clicked();
        });
        let _ = world;
        if let Some(p) = add {
            let name = name_of(w, &p.species);
            let f = self.setup.breed.file();
            let id = f.new_id();
            f.breeders.push(Breeder { id, parent: p, added: id / 1000, ..Default::default() });
            self.setup.breed.touch();
            self.setup.breed.flash = Some((format!("Added {name} to your Breeders."), Instant::now()));
        }
        if done {
            self.setup.breed.boxv.importing = false;
        }
    }

    // ───────────────────────────── Hatch log ─────────────────────────────

    pub(super) fn page_hatch_log(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let (setup, dc) = {
            let f = self.setup.breed.file();
            (f.setup, f.daycare.clone())
        };
        let egg = (!dc.a.species.is_empty() && !dc.b.species.is_empty()).then(|| breed::egg(&dc.a, &dc.b, &setup, w)).and_then(|r| r.ok());
        let pair = breed::family_pair(w, &dc.a, &dc.b);
        let goal = if dc.goal_set { dc.goal.clone() } else { breed::Goal::from_parents(&dc.a, &dc.b) };

        match &egg {
            Some(e) => self.hatch_entry(ui, w, &dc, e, &pair),
            None => {
                let mut go = false;
                super::sim_home::side_card(ui, kit::C_SLATE, "LOG A HATCHED EGG", |ui| {
                    ui.label(small("Put two Pokémon that can breed in the Day Care first: the log fills in what the Egg can be."));
                    ui.add_space(6.0);
                    go = soft(ui, "Go to the Day Care").clicked();
                });
                if go {
                    self.setup.breed.view = BreedView::DayCare;
                }
            }
        }

        // This pair's numbers.
        let log = self.setup.breed.file().log.clone();
        let mine: Vec<&Hatch> = log.iter().filter(|h| h.pair == pair).collect();
        if let Some(e) = &egg {
            if !mine.is_empty() {
                let p_one = goal.chance(&dc.a, &dc.b, e);
                let hits = mine.iter().filter(|h| h.hits(&goal)).count();
                let shinies = mine.iter().filter(|h| h.shiny).count();
                let wanted = goal.ivs.iter().filter(|x| **x).count() + goal.nature.is_some() as usize;
                let best = mine.iter().map(|h| h.to_parent().goal_score(&goal)).max().unwrap_or(0);
                let expect = mine.len() as f64 * p_one;
                let tiles = [
                    ("EGGS", mine.len().to_string(), format!("from {} × {}", name_of(w, &dc.a.species), name_of(w, &dc.b.species))),
                    ("THE ONE YOU WANT", hits.to_string(), format!("about {:.1} expected by now ({} each)", expect, one_in(p_one).to_lowercase())),
                    ("BEST SO FAR", if wanted > 0 { format!("{best} of {wanted}") } else { "–".into() }, "wanted stats at 31, and the nature".into()),
                    ("SHINY", shinies.to_string(), if shinies > 0 { "nice!".into() } else { format!("1 in {} each", super::sim_home::thousands(e.shiny_one_in.round() as u32)) }),
                ];
                let gap = 12.0;
                let avail = ui.available_width();
                let cols = if avail >= 900.0 { 4 } else { 2 };
                let tw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
                for row in tiles.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for (label, big, sub) in row {
                            let (r, _) = ui.allocate_exact_size(egui::vec2(tw, 84.0), Sense::hover());
                            let p = ui.painter();
                            p.add(kit::card_shadow().as_shape(r, 14.0));
                            p.rect_filled(r, 14.0, CARD());
                            p.rect_stroke(r, 14.0, Stroke::new(1.0f32, LINE()));
                            p.text(r.min + egui::vec2(16.0, 18.0), Align2::LEFT_CENTER, *label, crate::fonts::sans_bold_font(10.5), MUTED());
                            p.text(r.min + egui::vec2(16.0, 44.0), Align2::LEFT_CENTER, big, crate::fonts::sans_bold_font(24.0), TEXT());
                            let f = crate::fonts::sans_font(11.5);
                            p.text(r.min + egui::vec2(16.0, 68.0), Align2::LEFT_CENTER, super::charts::clip(ui, sub, &f, tw - 28.0), f, MUTED());
                        }
                    });
                    ui.add_space(gap);
                }
            }
        }

        // Undo a removal.
        if let Some((i, h)) = self.setup.breed.boxv.removed_hatch.clone() {
            let mut undo = false;
            let mut close = false;
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Removed a {} from the log.", name_of(w, &h.species))).size(13.0).color(TEXT()));
                undo = ui.link(RichText::new("Undo").size(13.0)).clicked();
                close = x_button(ui).clicked();
            });
            ui.add_space(6.0);
            if undo {
                let f = self.setup.breed.file();
                let at = i.min(f.log.len());
                f.log.insert(at, h);
                self.setup.breed.touch();
                self.setup.breed.boxv.removed_hatch = None;
            } else if close {
                self.setup.breed.boxv.removed_hatch = None;
            }
        }

        if log.is_empty() {
            ui.add_space(4.0);
            ui.label(small("Nothing logged yet. Log each Egg as it hatches: you'll see which ones hit your goal, which beat a parent, and how your luck compares."));
            return;
        }
        // The list.
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(caps("HATCHED"));
            ui.add_space(6.0);
            let all = &mut self.setup.breed.boxv.log_all;
            if chip(ui, "This pair", !*all, kit::C_TEAL).clicked() {
                *all = false;
            }
            if chip(ui, &format!("Everything  {}", log.len()), *all, kit::C_TEAL).clicked() {
                *all = true;
            }
        });
        ui.add_space(8.0);
        let all = self.setup.breed.boxv.log_all;
        // numbered per pair, oldest = 1
        let mut numbered: Vec<(usize, usize, &Hatch)> = vec![];
        let mut per_pair: std::collections::HashMap<&str, usize> = Default::default();
        for (i, h) in log.iter().enumerate() {
            let n = per_pair.entry(h.pair.as_str()).or_insert(0);
            *n += 1;
            if all || h.pair == pair {
                numbered.push((i, *n, h));
            }
        }
        numbered.reverse();
        if numbered.is_empty() {
            ui.label(small("Nothing logged from this pair yet. Pick \"Everything\" to see the rest."));
            return;
        }
        let limit = self.setup.breed.boxv.shown.max(50);
        let mut act: Option<(usize, char)> = None;
        for (i, n, h) in numbered.iter().take(limit) {
            if let Some(a) = self.hatch_row(ui, w, h, *n, &dc, &goal, all) {
                act = Some((*i, a));
            }
        }
        if numbered.len() > limit && ui.link(RichText::new(format!("Show {} more", (numbered.len() - limit).min(50))).size(12.5)).clicked() {
            self.setup.breed.boxv.shown = limit + 50;
        }
        if let Some((i, a)) = act {
            let f = self.setup.breed.file();
            match a {
                'k' => {
                    let h = f.log[i].clone();
                    let id = f.new_id();
                    f.breeders.push(Breeder { id, parent: h.to_parent(), added: id / 1000, notes: "Hatched".into(), ..Default::default() });
                    f.log[i].kept = Some(id);
                    self.setup.breed.flash = Some((format!("Kept the {} in your Breeders.", name_of(w, &h.species)), Instant::now()));
                }
                's' | 't' => {
                    let h = f.log[i].clone();
                    let child = h.to_parent();
                    let name = name_of(w, &h.species);
                    let old = if a == 's' { &mut f.daycare.a } else { &mut f.daycare.b };
                    let was = if old.is_ditto() { "Ditto".to_string() } else { name_of(w, &old.species) };
                    let held = old.held.clone();
                    let ability = if old.is_ditto() { child.ability } else { old.ability };
                    *old = Parent { held: held.clone(), ability, ..child };
                    let msg = match held {
                        Held::Nothing => format!("The {name} took the {was}'s place in the Day Care."),
                        h => format!("The {name} took the {was}'s place in the Day Care, holding the {}.", h.name()),
                    };
                    self.setup.breed.flash = Some((msg, Instant::now()));
                }
                'x' => {
                    let h = f.log.remove(i);
                    self.setup.breed.boxv.removed_hatch = Some((i, h));
                }
                _ => {}
            }
            self.setup.breed.touch();
        }
        ui.add_space(6.0);
        ui.label(small("Removing an Egg from the log doesn't change your hunt count."));
    }

    /// Log the Egg that just hatched.
    fn hatch_entry(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, dc: &breed::DayCare, e: &breed::Egg, pair: &str) {
        // A fresh draft for a new pair, keeping what tends to repeat.
        let mut h = match self.setup.breed.boxv.entry.take() {
            Some(h) if h.pair == pair => h,
            _ => {
                let nature = [&dc.a, &dc.b].iter().find(|p| p.held.keeps_nature()).and_then(|p| p.nature.clone());
                let gender = match e.female {
                    Some(f) if f >= 1.0 => Gender::Female,
                    Some(f) if f <= 0.0 => Gender::Male,
                    Some(f) if f >= 0.5 => Gender::Female,
                    Some(_) => Gender::Male,
                    None => Gender::Genderless,
                };
                let ability = e.abilities.iter().max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal)).map(|a| a.0.clone());
                Hatch {
                    species: e.species[0].0.clone(),
                    gender,
                    nature,
                    ability,
                    ball: e.ball.first().map(|b| b.0.clone()).unwrap_or_default(),
                    moves: e.moves.clone(),
                    pair: pair.to_string(),
                    ..Default::default()
                }
            }
        };
        if !e.species.iter().any(|(s, _)| *s == h.species) {
            h.species = e.species[0].0.clone();
        }
        let masuda_hunt = self.cfg.hunt_method == HuntMethod::Masuda;
        let mut count_hunt = self.setup.breed.file().count_hunt;
        let mut log_it = false;
        super::sim_home::side_card(ui, super::breed_home::EGG, "LOG A HATCHED EGG", |ui| {
            let full = ui.available_width();
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), Sense::hover());
                let p = ui.painter().clone();
                p.rect_filled(r, 12.0, theme::blend(kit::sunken(), super::breed_home::EGG, 0.1));
                if !self.team_sprite(&p, &h.species, h.shiny, r.center(), egui::vec2(56.0, 56.0)) {
                    p.text(r.center(), Align2::CENTER_CENTER, "…", crate::fonts::sans_bold_font(14.0), MUTED());
                }
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        if e.species.len() > 1 {
                            for (s, _) in &e.species {
                                if chip(ui, &name_of(w, s), h.species == *s, kit::C_TEAL).clicked() {
                                    h.species = s.clone();
                                    h.gender = breed::info(s).map(|i| if i.ratio == 0 { Gender::Male } else { Gender::Female }).unwrap_or(h.gender);
                                }
                            }
                        } else {
                            ui.label(RichText::new(name_of(w, &h.species)).font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                        }
                        ui.add_space(6.0);
                        let ratio = breed::info(&h.species).map(|i| i.ratio).unwrap_or(4);
                        if ratio > 0 && ratio < 8 {
                            for (g, l, c) in [(Gender::Female, "♀ Female", MOM), (Gender::Male, "♂ Male", DAD)] {
                                if chip(ui, l, h.gender == g, c).clicked() {
                                    h.gender = g;
                                }
                            }
                        }
                        ui.add_space(6.0);
                        if chip(ui, "✨ Shiny", h.shiny, GOLD).on_hover_text("It hatched shiny!").clicked() {
                            h.shiny = !h.shiny;
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                        ui.label(caps("NATURE"));
                        let natures: Vec<String> = std::iter::once("Don't know".to_string()).chain(crate::battle::data().natures.iter().map(|n| nature_label(&n.name))).collect();
                        let cur = h.nature.as_deref().map(nature_label).unwrap_or_else(|| "Don't know".into());
                        if let Some(n) = search_pick(ui, egui::Id::new("hatch-nature"), &cur, &natures, 180.0) {
                            h.nature = if n == "Don't know" { None } else { n.split_whitespace().next().map(str::to_string) };
                        }
                        if e.abilities.len() > 1 {
                            ui.add_space(6.0);
                            ui.label(caps("ABILITY"));
                            for (a, slot, _) in &e.abilities {
                                let l = if *slot == Slot::Hidden { format!("{a} (hidden)") } else { a.clone() };
                                if chip(ui, &l, h.ability.as_deref() == Some(a.as_str()), kit::C_TEAL).clicked() {
                                    h.ability = Some(a.clone());
                                }
                            }
                        }
                    });
                });
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(caps("STATS"));
                ui.add_space(4.0);
                ui.label(small("Tap the ones at 31"));
            });
            ui.add_space(2.0);
            iv_chips(ui, &mut h.ivs, full.min(560.0));
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 6.0);
                log_it = kit::accent_button_w(ui, egui::Button::new(RichText::new("Log this Egg").size(13.5)), 8.0, 130.0).on_hover_text("Or press Enter").clicked();
                // Enter logs it too, unless you're typing somewhere.
                if !ui.ctx().wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    log_it = true;
                }
                if masuda_hunt {
                    kit::checkbox(ui, &mut count_hunt, RichText::new(format!("Add it to your Masuda hunt too (now {})", super::sim_home::thousands(self.cfg.count.min(u32::MAX as u64) as u32))).size(12.5))
                        .on_hover_text("Each Egg you log adds one to the Shiny Hunter's count, like the +1 key. Off unless you turn it on.");
                }
            });
        });
        if count_hunt != self.setup.breed.file().count_hunt {
            self.setup.breed.file().count_hunt = count_hunt;
            self.setup.breed.touch();
        }
        if log_it {
            let f = self.setup.breed.file();
            let mut done = h.clone();
            done.id = f.new_id();
            done.when = now_secs();
            f.log.push(done);
            self.setup.breed.touch();
            if count_hunt && masuda_hunt {
                self.add(1);
            }
            // ready for the next one: same nature and Ability, fresh stats
            h.ivs = [None; 6];
            h.shiny = false;
        }
        self.setup.breed.boxv.entry = Some(h);
    }

    /// One logged Egg: 'k' keep, 's' swap into the Day Care, 'x' remove.
    #[allow(clippy::too_many_arguments)]
    fn hatch_row(&mut self, ui: &mut egui::Ui, w: &Wiki, h: &Hatch, n: usize, dc: &breed::DayCare, goal: &breed::Goal, show_pair: bool) -> Option<char> {
        let full = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 58.0), Sense::hover());
        let mut out = None;
        let p = ui.painter().clone();
        let hit = h.hits(goal) && h.pair == breed::family_pair(w, &dc.a, &dc.b);
        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
        let fill = if h.shiny {
            theme::blend(CARD(), GOLD, 0.14)
        } else if hit {
            theme::blend(CARD(), kit::C_GREEN, 0.10)
        } else {
            theme::blend(CARD(), CARD_HOVER(), ht)
        };
        p.rect_filled(r, 12.0, fill);
        p.rect_stroke(r, 12.0, Stroke::new(1.0f32, if h.shiny { theme::blend(LINE(), GOLD, 0.7) } else { LINE() }));
        p.text(egui::pos2(r.min.x + 22.0, r.center().y), Align2::CENTER_CENTER, format!("{n}"), crate::fonts::sans_bold_font(12.0), MUTED());
        if !self.team_sprite(&p, &h.species, h.shiny, egui::pos2(r.min.x + 66.0, r.center().y), egui::vec2(46.0, 46.0)) {
            p.text(egui::pos2(r.min.x + 66.0, r.center().y), Align2::CENTER_CENTER, "…", crate::fonts::sans_bold_font(12.0), MUTED());
        }
        let x = r.min.x + 98.0;
        let name_w = 190.0;
        let mut title = name_of(w, &h.species);
        if h.shiny {
            title = format!("✨ {title}");
        }
        let f = crate::fonts::sans_bold_font(14.0);
        let g = p.text(egui::pos2(x, r.center().y - 9.0), Align2::LEFT_CENTER, super::charts::clip(ui, &title, &f, name_w - 20.0), f, TEXT());
        if let Some((s, c)) = match h.gender {
            Gender::Male => Some(("♂", DAD)),
            Gender::Female => Some(("♀", MOM)),
            Gender::Genderless => None,
        } {
            p.text(egui::pos2(g.max.x + 5.0, r.center().y - 9.0), Align2::LEFT_CENTER, s, crate::fonts::sans_bold_font(13.0), super::charts::ink(c));
        }
        let mut sub = vec![h.nature.clone().unwrap_or_else(|| "nature ?".into())];
        if let Some(a) = &h.ability {
            sub.push(a.clone());
        }
        if show_pair {
            let parts: Vec<String> = h.pair.split('+').map(|s| name_of(w, s)).collect();
            sub.push(parts.join(" × "));
        }
        let f2 = crate::fonts::sans_font(11.5);
        p.text(egui::pos2(x, r.center().y + 10.0), Align2::LEFT_CENTER, super::charts::clip(ui, &sub.join(" · "), &f2, name_w), f2, MUTED());
        // Stats, then buttons on the right.
        let pills_w = (full - 98.0 - name_w - 300.0).clamp(150.0, 260.0);
        stat_pills(&p, egui::pos2(x + name_w, r.center().y - 15.0), &h.ivs, pills_w);
        let mut tx = x + name_w + pills_w + 10.0;
        if hit {
            tx += tag(&p, egui::pos2(tx, r.center().y - 10.0), "THE ONE", kit::C_GREEN) + 4.0;
        }
        let _ = tx;
        // Better than a parent it could stand in for?
        let mine = h.to_parent();
        let slot = (h.pair == breed::family_pair(w, &dc.a, &dc.b)).then(|| crate::breed_plan::swap_slot(w, &dc.a, &dc.b, &mine, goal)).flatten();
        let rival = slot.map(|i| if i == 0 { &dc.a } else { &dc.b });
        let better = slot.is_some();
        let br = Rect::from_min_max(egui::pos2(r.max.x - 300.0, r.min.y), r.max);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(br.shrink2(egui::vec2(8.0, 0.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
        child.spacing_mut().item_spacing.x = 6.0;
        if x_button(&mut child).on_hover_text("Remove from the log (you can undo)").clicked() {
            out = Some('x');
        }
        if h.kept.is_some() {
            child.label(small("In your Breeders ✔"));
        } else if soft(&mut child, "Keep").on_hover_text("Add it to your Breeders").clicked() {
            out = Some('k');
        }
        if better {
            let who = rival.map(|q| cap_first(&role(q, 0).0.trim_start_matches(['♀', '♂', ' ']).to_lowercase())).unwrap_or_default();
            if kit::accent_button_w(&mut child, egui::Button::new(RichText::new("Swap it in").size(12.5)), 8.0, 0.0)
                .on_hover_text(format!("Better than your {} for what you want: put it in the Day Care instead", who.to_lowercase()))
                .clicked()
            {
                out = Some(if slot == Some(0) { 's' } else { 't' });
            }
        }
        ui.add_space(6.0);
        out
    }
}
