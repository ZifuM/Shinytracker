//! Battle Sim → setting up the other side: five designs for building a
//! trainer and five for picking the wild Pokémon, while one of each is
//! being chosen. The parts inside (the trainer list, its settings, its
//! team, a Pokémon's editor, the route tables) are shared.

use super::sim_foes::{method_label, places, roll, table, FoeTab, Slot, Time, TrAct};
use super::sim_home::RED;
use super::team_home::{caps, chip, now_secs, small, soft};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::battle::World;
use crate::sim::{self, Format, SavedFoe, Trainer};
use crate::teams::Member;
use crate::theme;
use super::team_calc::search_pick;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

/// A switch between the designs of one part, while one is being chosen.
#[allow(dead_code)]
pub(super) fn design_switch(ui: &mut egui::Ui, what: &str, design: &mut u8, names: &[&str], env: &str) {
    if *design == 0 {
        *design = std::env::var(env).ok().and_then(|v| v.parse().ok()).filter(|d| (1..=names.len() as u8).contains(d)).unwrap_or(1);
    }
    egui::Frame::none()
        .fill(theme::blend(CARD(), kit::C_GOLD, 0.08))
        .stroke(Stroke::new(1.0f32, theme::blend(LINE(), kit::C_GOLD, 0.45)))
        .rounding(10.0)
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new(format!("PREVIEW · {what} DESIGN")).font(crate::fonts::sans_bold_font(11.0)).color(super::charts::ink(kit::C_GOLD)));
                ui.add_space(6.0);
                for (i, n) in names.iter().enumerate() {
                    let d = i as u8 + 1;
                    if chip(ui, &format!("{d}  {n}"), *design == d, kit::C_GOLD).clicked() {
                        *design = d;
                    }
                }
            });
        });
    ui.add_space(14.0);
}

/// Lay `parts` out side by side, `widths` points each (0 = share the rest).
fn columns(ui: &mut egui::Ui, widths: &[f32], gap: f32, mut part: impl FnMut(&mut egui::Ui, usize)) {
    let full = ui.available_width();
    let fixed: f32 = widths.iter().sum::<f32>() + gap * (widths.len() as f32 - 1.0);
    let flex = widths.iter().filter(|w| **w == 0.0).count().max(1) as f32;
    let rest = ((full - fixed) / flex).max(160.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (i, w) in widths.iter().enumerate() {
            let w = if *w == 0.0 { rest } else { *w };
            ui.allocate_ui_with_layout(egui::vec2(w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(w);
                part(ui, i);
            });
        }
    });
}

/// Where the wild one will show, before there is one.
fn ws_empty(ui: &mut egui::Ui, h: f32) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 12.0, kit::sunken());
    p.rect_stroke(r, 12.0, Stroke::new(1.0f32, theme::blend(LINE(), RED, 0.25)));
    p.text(r.center() - egui::vec2(0.0, 10.0), Align2::CENTER_CENTER, "No wild Pokémon yet", crate::fonts::sans_bold_font(15.0), TEXT());
    p.text(r.center() + egui::vec2(0.0, 12.0), Align2::CENTER_CENTER, "Roll one, pick one it could be, or build it", crate::fonts::sans_font(12.0), MUTED());
}

/// A small heading inside a card.
fn head(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t).font(crate::fonts::sans_bold_font(11.0)).color(super::charts::ink(RED)));
    ui.add_space(6.0);
}

impl App {
    // ===============================================================
    // Trainers

    /// What every trainer design starts from.
    fn tr_begin(&mut self) -> (Vec<Trainer>, Vec<crate::teams::Team>, Option<Trainer>) {
        let trainers: Vec<Trainer> = self.setup.sim.file().trainers.clone();
        let teams: Vec<crate::teams::Team> = self.setup.team.teams().teams.clone();
        let cur = self.setup.sim.setup.trainer.and_then(|id| trainers.iter().find(|t| t.id == id).cloned());
        (trainers, teams, cur)
    }

    fn tr_end(&mut self, mut act: TrAct, teams: &[crate::teams::Team], before: &Option<Trainer>, after: Option<Trainer>, v: super::sim_foes::FoeView) {
        if let (Some(a), Some(mut b)) = (before.clone(), after) {
            if a != b {
                b.updated = now_secs();
                act.edit = Some(b);
            }
        }
        self.setup.sim.foe = v;
        self.trainer_apply(act, teams);
    }

    /// One member's editor in `cols` columns: 1 stacks everything; 2 puts
    /// the basics beside the moves, stats and sets; 3 gives stats and sets
    /// their own column.
    fn member_columns(&mut self, ui: &mut egui::Ui, m: &mut Member, world: World, key: &str, cols: usize) {
        if cols <= 1 {
            self.foe_editor(ui, m, world, key, RED);
            return;
        }
        let third = cols >= 3;
        let widths = vec![0.0; cols];
        columns(ui, &widths, 20.0, |ui, i| match i {
            0 => self.foe_basics(ui, m, world, key, RED),
            1 => {
                self.foe_moves(ui, m, world, key);
                if !third {
                    ui.add_space(10.0);
                    self.foe_stats(ui, m, world, key);
                    self.foe_sets(ui, m, world, RED);
                }
            }
            _ => {
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("foe-stats-plain", key)), true));
                self.foe_stats(ui, m, world, key);
                ui.add_space(8.0);
                self.foe_sets(ui, m, world, RED);
            }
        });
    }

    /// The selected member of `t`: sprite, then its editor in columns.
    fn tr_member_cols(&mut self, ui: &mut egui::Ui, t: &mut Trainer, sel: usize, cols: usize) {
        let world = t.world;
        if let Some(m) = t.members.get_mut(sel) {
            let mut m2 = m.clone();
            self.foe_hero(ui, &m2, world);
            ui.add_space(10.0);
            self.member_columns(ui, &mut m2, world, &format!("tr{sel}"), cols);
            if m2 != *m {
                *m = m2;
            }
        }
    }

    /// The trainer's settings in two columns: who it is | how it battles.
    fn tr_settings_split(&mut self, ui: &mut egui::Ui, t: &mut Trainer, items: bool) {
        columns(ui, &[0.0, 0.0], 24.0, |ui, i| {
            if i == 0 {
                self.trainer_rows(ui, t, 0..2, "tr-split-a");
            } else {
                self.trainer_rows(ui, t, 2..if items { 5 } else { 4 }, "tr-split-b");
            }
        });
    }

    fn tr_empty(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            ui.label(RichText::new("Pick a trainer").font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
            ui.label(small("Pick one of your trainers, or make a new one, to set it up here."));
        });
    }

    /// The trainer setup: your trainers as cards across the top, the
    /// chosen one's settings in one row under them, then its team down the
    /// side with the Pokémon you're editing beside it.
    pub(super) fn tr_final(&mut self, ui: &mut egui::Ui) {
        let (trainers, teams, cur) = self.tr_begin();
        let mut act = TrAct::default();
        let mut v = self.setup.sim.foe.clone();
        let mut t2 = cur.clone();
        let sel = self.setup.sim.setup.trainer;
        // the roster
        egui::ScrollArea::horizontal().id_salt("tr-roster").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for t in &trainers {
                    let on = sel == Some(t.id);
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(230.0, 104.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let p = ui.painter().clone();
                    p.rect_filled(r, 14.0, if on { theme::blend(CARD(), RED, 0.16) } else { theme::blend(CARD(), CARD_HOVER(), ht) });
                    p.rect_stroke(r, 14.0, Stroke::new(if on { 2.0f32 } else { 1.0 }, if on { RED } else { LINE() }));
                    let f1 = crate::fonts::sans_bold_font(14.0);
                    p.text(r.min + egui::vec2(14.0, 20.0), Align2::LEFT_CENTER, super::charts::clip(ui, &t.name, &f1, 200.0), f1, TEXT());
                    p.text(r.min + egui::vec2(14.0, 38.0), Align2::LEFT_CENTER, format!("{} · {} Pokémon", if t.class.is_empty() { "Trainer" } else { t.class.as_str() }, t.members.len()), crate::fonts::sans_font(11.0), MUTED());
                    for (i, m) in t.members.iter().take(6).enumerate() {
                        let c = egui::pos2(r.min.x + 30.0 + 34.0 * i as f32, r.max.y - 28.0);
                        if !self.team_sprite(&p, &m.species, m.shiny, c, egui::vec2(34.0, 34.0)) {
                            p.circle_filled(c, 12.0, theme::blend(CARD(), RED, 0.12));
                        }
                    }
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if resp.clicked() {
                        act.pick = Some(t.id);
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Duplicate").clicked() {
                            act.copy = Some(t.id);
                            ui.close_menu();
                        }
                        if ui.button("Delete").clicked() {
                            act.delete = Some(t.id);
                            ui.close_menu();
                        }
                    });
                }
                // add
                let (r, resp) = ui.allocate_exact_size(egui::vec2(150.0, 104.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                ui.painter().rect_stroke(r, 14.0, Stroke::new(1.0 + ht, theme::blend(LINE(), RED, 0.3 + 0.5 * ht)));
                ui.painter().text(r.center() - egui::vec2(0.0, 10.0), Align2::CENTER_CENTER, "+", crate::fonts::sans_bold_font(24.0), MUTED());
                ui.painter().text(r.center() + egui::vec2(0.0, 18.0), Align2::CENTER_CENTER, "New trainer", crate::fonts::sans_bold_font(12.5), MUTED());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    act.new_blank = true;
                }
            });
        });
        ui.horizontal(|ui| {
            kit::menu(ui, RichText::new("Make one from a team").size(12.5), |ui| {
                for t in teams.iter().filter(|t| !t.members.is_empty()).take(15) {
                    if ui.button(format!("{}  ({})", t.name, t.members.len())).clicked() {
                        act.from_team = Some(t.id);
                        ui.close_menu();
                    }
                }
            });
            ui.label(small("·  right-click a trainer to duplicate or delete it"));
        });
        ui.add_space(12.0);
        if let Some(t) = t2.as_mut() {
            // who they are | how they battle | the items they can use
            kit::card(ui, |ui| {
                head(ui, "THE TRAINER");
                columns(ui, &[0.0, 0.0, 0.0], 24.0, |ui, i| match i {
                    0 => self.trainer_rows(ui, t, 0..2, "trf-who"),
                    1 => self.trainer_rows(ui, t, 2..4, "trf-how"),
                    _ => self.trainer_rows(ui, t, 4..5, "trf-items"),
                });
            });
            ui.add_space(12.0);
            let tab_id = egui::Id::new("trf-tab");
            let mut tab: u8 = ui.ctx().data(|d| d.get_temp(tab_id)).unwrap_or(0);
            columns(ui, &[260.0, 0.0], 16.0, |ui, i| {
                if i == 0 {
                    kit::card(ui, |ui| {
                        head(ui, &format!("ITS TEAM  ·  {} OF 6", t.members.len()));
                        let mut remove = None;
                        for (k, m) in t.members.iter().enumerate() {
                            let on = v.sel == k;
                            let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 54.0), Sense::click());
                            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                            let p = ui.painter().clone();
                            p.rect_filled(r, 10.0, if on { theme::blend(CARD(), RED, 0.16) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                            if on {
                                p.rect_stroke(r, 10.0, Stroke::new(1.5f32, RED));
                            }
                            let c = egui::pos2(r.min.x + 28.0, r.center().y);
                            if !self.team_sprite(&p, &m.species, m.shiny, c, egui::vec2(46.0, 46.0)) {
                                p.circle_filled(c, 14.0, theme::blend(CARD(), RED, 0.12));
                            }
                            p.text(egui::pos2(r.min.x + 58.0, r.min.y + 19.0), Align2::LEFT_CENTER, m.display_name(), crate::fonts::sans_bold_font(13.0), TEXT());
                            p.text(egui::pos2(r.min.x + 58.0, r.min.y + 37.0), Align2::LEFT_CENTER, format!("Lv {}{}", m.level, if m.item.is_empty() { String::new() } else { format!(" · {}", m.item) }), crate::fonts::sans_font(11.0), MUTED());
                            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                            if resp.clicked() {
                                v.sel = k;
                            }
                            resp.context_menu(|ui| {
                                if ui.button("Remove").clicked() {
                                    remove = Some(k);
                                    ui.close_menu();
                                }
                            });
                            ui.add_space(4.0);
                        }
                        if let Some(k) = remove {
                            t.members.remove(k);
                            v.sel = v.sel.min(t.members.len().saturating_sub(1));
                        }
                        if t.members.len() < 6 && soft(ui, "+  Add a Pokémon").clicked() {
                            let lv = t.members.last().map(|m| m.level).unwrap_or(30);
                            let mut m = Member { level: lv, ..Member::new("Bibarel", t.world) };
                            if let Some(w) = self.wiki.clone() {
                                m.moves = super::sim_foes::wild_set(&w, "Bibarel", lv, t.world);
                            }
                            t.members.push(m);
                            v.sel = t.members.len() - 1;
                        }
                    });
                } else {
                    kit::card(ui, |ui| {
                        let world = t.world;
                        let s = v.sel;
                        if let Some(m) = t.members.get_mut(s) {
                            let mut m2 = m.clone();
                            self.foe_hero(ui, &m2, world);
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                for (k, l) in [(0u8, "Basics & moves"), (1, "Stats & sets")] {
                                    if chip(ui, l, tab == k, RED).clicked() {
                                        tab = k;
                                    }
                                }
                            });
                            ui.add_space(12.0);
                            let key = format!("trf{s}");
                            columns(ui, &[0.0, 0.0], 24.0, |ui, c| match (tab, c) {
                                (1, 0) => {
                                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("foe-stats-plain", key.as_str())), true));
                                    self.foe_stats(ui, &mut m2, world, &key);
                                }
                                (1, _) => self.foe_sets(ui, &mut m2, world, RED),
                                (_, 0) => self.foe_basics(ui, &mut m2, world, &key, RED),
                                _ => self.foe_moves(ui, &mut m2, world, &key),
                            });
                            if m2 != *m {
                                *m = m2;
                            }
                        }
                    });
                }
            });
            ui.ctx().data_mut(|d| d.insert_temp(tab_id, tab));
        } else {
            self.tr_empty(ui);
        }
        self.tr_end(act, &teams, &cur, t2, v);
    }

    // ===============================================================
    // Wild Pokémon

    /// The wild ones as the setup has them (two in doubles), their world,
    /// doubles, and which one is being set up.
    fn ws_begin(&mut self) -> (World, bool, Vec<Member>, usize) {
        let world = self.sim_world();
        let doubles = self.setup.sim.setup.format == Format::Doubles;
        let want = if doubles { 2 } else { 1 };
        let mut foes = self.setup.sim.setup.foes.clone();
        let lv0 = foes.first().map(|f| f.level).unwrap_or(20);
        foes.resize_with(want, || Member { level: lv0, ..Member::new("Bidoof", world) });
        let sel = self.setup.sim.foe.sel.min(want - 1);
        self.setup.sim.foe.sel = sel;
        (world, doubles, foes, sel)
    }

    fn ws_put(&mut self, foes: Vec<Member>) {
        if foes != self.setup.sim.setup.foes {
            self.setup.sim.setup.foes = foes;
        }
    }

    fn ws_save(&mut self, m: Member, world: World) {
        let id = self.setup.sim.new_id();
        let f = self.setup.sim.file();
        if !f.foes.iter().any(|x| x.member == m) {
            f.foes.insert(0, SavedFoe { id, member: m, world, updated: now_secs() });
            self.setup.sim.touch();
        }
    }

    /// The route part: place, method and time, roll, and the kinds as
    /// tiles (`tiles`) or rows.
    fn ws_route(&mut self, ui: &mut egui::Ui, tiles: bool, place_list: bool) {
        let Some((w, world, list, slots)) = self.wild_where() else {
            ui.label(small("Getting the game's data…"));
            return;
        };
        let mut meet: Option<Option<Slot>> = None;
        let v = self.setup.sim.foe.clone();
        let pname = list.iter().find(|p| p.0 == v.area).map(|p| p.1.clone()).unwrap_or_default();
        let methods = list.iter().find(|p| p.0 == v.area).map(|p| p.2.clone()).unwrap_or_default();
        if !place_list {
            ui.horizontal(|ui| {
                ui.label(caps("PLACE"));
                let names: Vec<String> = list.iter().map(|p| p.1.clone()).collect();
                if let Some(n) = search_pick(ui, egui::Id::new("ws-place"), &pname, &names, 240.0) {
                    if let Some(p) = list.iter().find(|p| p.1 == n) {
                        self.setup.sim.foe.area = p.0.clone();
                        self.setup.sim.foe.rolled = None;
                    }
                }
                if let Some(stage) = w.place(&v.area).map(|p| p.stage.clone()).filter(|s| !s.is_empty()) {
                    ui.label(small(stage));
                }
            });
            ui.add_space(8.0);
        }
        self.wild_method_time(ui, &methods);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let doubles = self.setup.sim.setup.format == Format::Doubles;
            if kit::accent_button_w(ui, egui::Button::new(RichText::new(if doubles { "🎲  Roll two" } else { "🎲  Roll one" }).size(13.0)), 8.0, 120.0).on_hover_text("By the game's odds").clicked() {
                meet = Some(None);
            }
            if let Some(r) = &self.setup.sim.foe.rolled {
                ui.label(RichText::new(r).size(12.0).color(super::charts::ink(RED)));
            } else {
                ui.label(small("or pick one:"));
            }
        });
        ui.add_space(10.0);
        if slots.is_empty() {
            ui.label(small("Nothing to meet this way at this time of day."));
        }
        if tiles {
            if let Some(s) = self.wild_slot_tiles(ui, &slots, 132.0) {
                meet = Some(Some(s));
            }
        } else {
            let full = ui.available_width();
            let cur = self.setup.sim.setup.foes.get(self.setup.sim.foe.sel).map(|f| crate::battle::to_id(&f.species));
            for s in &slots {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 38.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let on = cur.as_deref() == Some(crate::battle::to_id(&s.name).as_str());
                let p = ui.painter().clone();
                p.rect_filled(r, 8.0, if on { theme::blend(CARD(), RED, 0.14) } else { theme::blend(CARD(), CARD_HOVER(), ht) });
                let c = egui::pos2(r.min.x + 20.0, r.center().y);
                if !self.team_sprite(&p, &s.name, false, c, egui::vec2(34.0, 34.0)) {
                    p.text(c, Align2::CENTER_CENTER, s.name.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(10.0), MUTED());
                }
                p.text(egui::pos2(r.min.x + 44.0, r.center().y), Align2::LEFT_CENTER, &s.name, crate::fonts::sans_bold_font(12.5), TEXT());
                let lv = if s.lv.0 == s.lv.1 { format!("Lv {}", s.lv.0) } else { format!("Lv {}–{}", s.lv.0, s.lv.1) };
                p.text(egui::pos2(r.min.x + 170.0, r.center().y), Align2::LEFT_CENTER, lv, crate::fonts::sans_font(11.5), MUTED());
                let bx = r.min.x + 250.0;
                let bw = (r.max.x - bx - 50.0).max(30.0);
                let bar = Rect::from_min_size(egui::pos2(bx, r.center().y - 4.0), egui::vec2(bw, 8.0));
                p.rect_filled(bar, 4.0, kit::sunken());
                p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bw * (s.chance.min(100) as f32 / 100.0), 8.0)), 4.0, theme::blend(RED, MUTED(), 0.2));
                p.text(egui::pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, format!("{}%", s.chance), crate::fonts::sans_bold_font(12.0), TEXT());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Meet this one").clicked() {
                    meet = Some(Some(s.clone()));
                }
            }
        }
        if let Some(m) = meet {
            self.wild_meet(&w, world, &slots, m.as_ref());
        }
    }

    /// The places as a searchable list.
    fn ws_places(&mut self, ui: &mut egui::Ui, h: f32) {
        let Some((_, _, list, _)) = self.wild_where() else { return };
        let qid = egui::Id::new("ws-place-q");
        let mut q: String = ui.ctx().data(|d| d.get_temp(qid)).unwrap_or_default();
        ui.add(egui::TextEdit::singleline(&mut q).hint_text("Search places…").desired_width(f32::INFINITY));
        ui.ctx().data_mut(|d| d.insert_temp(qid, q.clone()));
        ui.add_space(6.0);
        let ql = q.to_lowercase();
        egui::ScrollArea::vertical().id_salt("ws-places").max_height(h).auto_shrink([false, true]).show(ui, |ui| {
            for (key, name, _) in list.iter().filter(|p| ql.is_empty() || p.1.to_lowercase().contains(&ql)) {
                let on = self.setup.sim.foe.area == *key;
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter();
                if on || ht > 0.01 {
                    p.rect_filled(r, 8.0, if on { theme::blend(CARD(), RED, 0.18) } else { theme::blend(CARD(), CARD_HOVER(), ht) });
                }
                if on {
                    p.rect_filled(Rect::from_min_size(r.min + egui::vec2(0.0, 7.0), egui::vec2(3.0, 16.0)), 2.0, RED);
                }
                p.text(egui::pos2(r.min.x + 12.0, r.center().y), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(12.5), if on { TEXT() } else { MUTED() });
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.setup.sim.foe.area = key.clone();
                    self.setup.sim.foe.rolled = None;
                }
            }
        });
    }

    /// The selected wild one's sprite and details.
    fn ws_hero(&mut self, ui: &mut egui::Ui) {
        let (world, _, foes, sel) = self.ws_begin();
        if self.ws_none() {
            ws_empty(ui, 96.0);
        } else {
            self.foe_hero(ui, &foes[sel], world);
        }
    }

    /// No wild one picked yet for the slot being set up.
    fn ws_none(&self) -> bool {
        self.setup.sim.setup.foes.get(self.setup.sim.foe.sel).is_none()
    }

    /// Build the selected one by hand: basics and moves side by side, or
    /// with `third`, stats and sets in a third column.
    fn ws_build(&mut self, ui: &mut egui::Ui, cols: usize) {
        let (world, _, mut foes, sel) = self.ws_begin();
        let mut m = foes[sel].clone();
        match cols {
            1 => self.foe_editor(ui, &mut m, world, &format!("ws{sel}"), RED),
            n => self.member_columns(ui, &mut m, world, &format!("ws{sel}"), n),
        }
        if m != foes[sel] {
            foes[sel] = m;
            self.ws_put(foes);
        }
    }

    fn ws_saved(&mut self, ui: &mut egui::Ui) {
        let (_, _, mut foes, sel) = self.ws_begin();
        let mut m = foes[sel].clone();
        self.foe_saved(ui, &mut m);
        if m != foes[sel] {
            foes[sel] = m;
            self.ws_put(foes);
        }
    }

    fn ws_save_button(&mut self, ui: &mut egui::Ui) {
        if self.ws_none() {
            return;
        }
        let (world, _, foes, sel) = self.ws_begin();
        if soft(ui, "📌  Save this Pokémon").on_hover_text("Keep it under Saved to use again").clicked() {
            self.ws_save(foes[sel].clone(), world);
        }
    }

    /// How you're picking it: from a route, by hand, or a saved one.
    fn ws_tab_chips(&mut self, ui: &mut egui::Ui) {
        let mut tab = self.setup.sim.foe.tab;
        ui.horizontal(|ui| {
            for (t, l) in [(FoeTab::Route, "🗺  From a route"), (FoeTab::Build, "🐾  Build it"), (FoeTab::Saved, "📌  Saved")] {
                if chip(ui, l, tab == t, RED).clicked() {
                    tab = t;
                }
            }
        });
        self.setup.sim.foe.tab = tab;
    }

    /// The wild Pokémon setup: what you'll meet on the left (both of them
    /// in doubles), how you find it on the right.
    pub(super) fn ws_final(&mut self, ui: &mut egui::Ui) {
        columns(ui, &[370.0, 0.0], 16.0, |ui, i| {
            if i == 0 {
                kit::card(ui, |ui| {
                    let doubles = self.setup.sim.setup.format == Format::Doubles;
                    head(ui, if doubles { "THE WILD POKÉMON  ·  TWO OF THEM" } else { "THE WILD POKÉMON" });
                    self.ws_pair(ui);
                    self.ws_hero(ui);
                    if !self.ws_none() {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            self.ws_save_button(ui);
                            if soft(ui, "Edit it").on_hover_text("Level, ability, nature, moves and stats").clicked() {
                                self.setup.sim.foe.tab = FoeTab::Build;
                            }
                        });
                    }
                });
                ui.add_space(12.0);
                kit::card(ui, |ui| {
                    let n = self.setup.sim.file().foes.len();
                    head(ui, &if n > 0 { format!("SAVED  ·  {n}") } else { "SAVED".to_string() });
                    self.ws_saved(ui);
                });
            } else {
                kit::card(ui, |ui| {
                    let mut tab = if self.setup.sim.foe.tab == FoeTab::Saved { FoeTab::Route } else { self.setup.sim.foe.tab };
                    kit::tab_bar(ui, &mut tab, &[(FoeTab::Route, "🗺  Meet one on a route"), (FoeTab::Build, "🐾  Build one by hand")]);
                    self.setup.sim.foe.tab = tab;
                    ui.add_space(14.0);
                    match tab {
                        FoeTab::Build => self.ws_build(ui, 2),
                        _ => self.ws_route_grid(ui),
                    }
                });
            }
        });
    }

    /// In doubles: the two wild ones side by side; click one to set it up.
    fn ws_pair(&mut self, ui: &mut egui::Ui) {
        if self.setup.sim.setup.format != Format::Doubles {
            self.setup.sim.foe.sel = 0;
            return;
        }
        let foes = self.setup.sim.setup.foes.clone();
        let gap = 8.0;
        let w = ((ui.available_width() - gap) / 2.0).floor();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for i in 0..2 {
                let on = self.setup.sim.foe.sel == i;
                let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 50.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                p.rect_filled(r, 10.0, if on { theme::blend(CARD(), RED, 0.16) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                p.rect_stroke(r, 10.0, Stroke::new(if on { 1.5f32 } else { 1.0 }, if on { RED } else { LINE() }));
                let c = egui::pos2(r.min.x + 26.0, r.center().y);
                match foes.get(i) {
                    Some(f) => {
                        if !self.team_sprite(&p, &f.species, f.shiny, c, egui::vec2(42.0, 42.0)) {
                            p.circle_filled(c, 13.0, theme::blend(CARD(), RED, 0.12));
                        }
                        let fb = crate::fonts::sans_bold_font(12.5);
                        p.text(egui::pos2(r.min.x + 52.0, r.min.y + 17.0), Align2::LEFT_CENTER, super::charts::clip(ui, f.display_name(), &fb, w - 60.0), fb, TEXT());
                        p.text(egui::pos2(r.min.x + 52.0, r.min.y + 34.0), Align2::LEFT_CENTER, format!("Lv {}", f.level), crate::fonts::sans_font(11.0), MUTED());
                    }
                    None => {
                        p.circle_stroke(c, 13.0, Stroke::new(1.0f32, LINE()));
                        p.text(c, Align2::CENTER_CENTER, format!("{}", i + 1), crate::fonts::sans_bold_font(12.0), MUTED());
                        p.text(egui::pos2(r.min.x + 52.0, r.center().y), Align2::LEFT_CENTER, "Not picked", crate::fonts::sans_font(12.0), MUTED());
                    }
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Set up this one").clicked() {
                    self.setup.sim.foe.sel = i;
                }
            }
        });
        ui.add_space(10.0);
    }

    /// Where, how and when in one aligned grid, then what lives there with
    /// the roll button beside the heading, as tiles that fill the row.
    fn ws_route_grid(&mut self, ui: &mut egui::Ui) {
        let Some((w, world, list, slots)) = self.wild_where() else {
            ui.label(small("Getting the game's data…"));
            return;
        };
        let v = self.setup.sim.foe.clone();
        let pname = list.iter().find(|p| p.0 == v.area).map(|p| p.1.clone()).unwrap_or_default();
        let methods = list.iter().find(|p| p.0 == v.area).map(|p| p.2.clone()).unwrap_or_default();
        let mut meet: Option<Option<Slot>> = None;
        egui::Grid::new("ws-route-grid").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
            ui.label(caps("WHERE"));
            ui.horizontal(|ui| {
                let names: Vec<String> = list.iter().map(|p| p.1.clone()).collect();
                if let Some(n) = search_pick(ui, egui::Id::new("ws-place"), &pname, &names, 260.0) {
                    if let Some(p) = list.iter().find(|p| p.1 == n) {
                        self.setup.sim.foe.area = p.0.clone();
                        self.setup.sim.foe.rolled = None;
                    }
                }
                if let Some(stage) = w.place(&v.area).map(|p| p.stage.clone()).filter(|s| !s.is_empty()) {
                    ui.label(small(stage));
                }
            });
            ui.end_row();
            ui.label(caps("HOW"));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for m in &methods {
                    if chip(ui, method_label(m), v.method == *m, RED).clicked() {
                        self.setup.sim.foe.method = m.clone();
                        self.setup.sim.foe.rolled = None;
                    }
                }
            });
            ui.end_row();
            ui.label(caps("WHEN"));
            ui.horizontal(|ui| {
                let time = v.time.unwrap_or_default();
                for t in [Time::Morning, Time::Day, Time::Night] {
                    if chip(ui, t.label(), time == t, kit::C_INDIGO).on_hover_text("Some Pokémon only come out at some times of day").clicked() {
                        self.setup.sim.foe.time = Some(t);
                    }
                }
            });
            ui.end_row();
        });
        ui.add_space(14.0);
        let (lr, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(lr, 0.0, LINE());
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(caps("WHAT LIVES HERE"));
            ui.label(small(format!("·  {} kinds  ·  click one to meet it", slots.len())));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let doubles = self.setup.sim.setup.format == Format::Doubles;
                if kit::accent_button_w(ui, egui::Button::new(RichText::new(if doubles { "🎲  Roll two" } else { "🎲  Roll one" }).size(13.0)), 8.0, 120.0).on_hover_text("By the game's odds").clicked() {
                    meet = Some(None);
                }
            });
        });
        if let Some(r) = &self.setup.sim.foe.rolled {
            ui.add_space(4.0);
            ui.label(RichText::new(r).size(12.0).color(super::charts::ink(RED)));
        }
        ui.add_space(10.0);
        if slots.is_empty() {
            ui.label(small("Nothing to meet this way at this time of day."));
        } else {
            // as big as fits on one row, but not too big or too small
            let n = slots.len() as f32;
            let tile = ((ui.available_width() - 10.0 * (n - 1.0)) / n).clamp(122.0, 168.0);
            if let Some(s) = self.wild_slot_tiles(ui, &slots, tile) {
                meet = Some(Some(s));
            }
        }
        if let Some(m) = meet {
            self.wild_meet(&w, world, &slots, m.as_ref());
        }
    }

    /// The place, method and time you're meeting wild Pokémon in, made
    /// valid; and the places in story order.
    fn wild_where(&mut self) -> Option<(std::sync::Arc<crate::wiki::Wiki>, crate::battle::World, Vec<(String, String, Vec<String>)>, Vec<Slot>)> {
        let w = self.wiki.clone()?;
        let world = self.sim_world();
        let more = self.cfg.info_more;
        let list = places(&w, world, more);
        if list.is_empty() {
            return None;
        }
        let v = &mut self.setup.sim.foe;
        if !list.iter().any(|p| p.0 == v.area) {
            v.area = list[0].0.clone();
        }
        let time = *v.time.get_or_insert_with(Time::now);
        let methods = list.iter().find(|p| p.0 == v.area).map(|p| p.2.clone()).unwrap_or_default();
        if !methods.contains(&v.method) {
            v.method = methods.first().cloned().unwrap_or_default();
        }
        let slots = table(&w, &v.area.clone(), &v.method.clone(), time, world, more);
        Some((w, world, list, slots))
    }

    /// Meet one kind (`slot`), or roll by the game's odds (both in doubles).
    fn wild_meet(&mut self, w: &crate::wiki::Wiki, world: crate::battle::World, slots: &[Slot], slot: Option<&Slot>) {
        let doubles = self.setup.sim.setup.format == Format::Doubles;
        let sel = self.setup.sim.foe.sel;
        let mut rng = sim::Rng::from_time();
        let mut made: Vec<(usize, Member)> = Vec::new();
        if let Some(s) = slot {
            if let Some(m) = roll(w, &s.id, s.lv, world, &mut rng) {
                made.push((sel, m));
            }
        } else if !slots.is_empty() {
            let total: u32 = slots.iter().map(|s| s.chance).sum();
            for k in 0..if doubles { 2 } else { 1 } {
                let mut pick = rng.below(total.max(1));
                let hit = slots.iter().find(|s| {
                    if pick < s.chance {
                        true
                    } else {
                        pick -= s.chance;
                        false
                    }
                });
                if let Some(s) = hit {
                    if let Some(m) = roll(w, &s.id, s.lv, world, &mut rng) {
                        made.push((if doubles { k } else { sel }, m));
                    }
                }
            }
        }
        if made.is_empty() {
            return;
        }
        let want = if doubles { 2 } else { 1 };
        let mut foes = self.setup.sim.setup.foes.clone();
        let lv0 = foes.first().map(|f| f.level).unwrap_or(20);
        foes.resize_with(want, || Member { level: lv0, ..Member::new("Bidoof", world) });
        let names: Vec<String> = made.iter().map(|(_, m)| format!("{} Lv {}", m.species, m.level)).collect();
        for (i, m) in made {
            if let Some(f) = foes.get_mut(i) {
                *f = m;
            }
        }
        let v = &mut self.setup.sim.foe;
        v.rolled = Some(format!("A wild {} appeared!", names.join(" and a wild ")));
        v.synced = None;
        let pname = w.place(&v.area).map(|p| p.name.clone()).unwrap_or_else(|| v.area.clone());
        let method = v.method.clone();
        let st = &mut self.setup.sim.setup;
        st.foes = foes;
        st.place = pname;
        st.method = method;
        self.setup.sim.catch.sit.turn = 0;
        self.setup.sim.catch.hp = 1.0;
    }

    /// Method and time chips for the current place.
    fn wild_method_time(&mut self, ui: &mut egui::Ui, methods: &[String]) {
        let v = &mut self.setup.sim.foe;
        let time = v.time.unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for m in methods {
                if chip(ui, method_label(m), v.method == *m, RED).clicked() {
                    v.method = m.clone();
                    v.rolled = None;
                }
            }
            ui.add_space(10.0);
            for t in [Time::Morning, Time::Day, Time::Night] {
                if chip(ui, t.label(), time == t, kit::C_INDIGO).on_hover_text("Some Pokémon only come out at some times of day").clicked() {
                    v.time = Some(t);
                }
            }
        });
    }

    /// The kinds you can meet, as tiles with a sprite: click one to meet it.
    fn wild_slot_tiles(&mut self, ui: &mut egui::Ui, slots: &[Slot], tile: f32) -> Option<Slot> {
        let gap = 10.0;
        let full = ui.available_width();
        let cols = ((full + gap) / (tile + gap)).floor().max(1.0) as usize;
        let w = ((full - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let cur = self.setup.sim.setup.foes.get(self.setup.sim.foe.sel).map(|f| crate::battle::to_id(&f.species));
        let mut out = None;
        for row in slots.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for s in row {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(w, tile * 0.95), Sense::click());
                    let on = cur.as_deref() == Some(crate::battle::to_id(&s.name).as_str());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let r = r.translate(egui::vec2(0.0, -2.0 * ht));
                    let p = ui.painter().clone();
                    p.rect_filled(r, 14.0, if on { theme::blend(CARD(), RED, 0.16) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                    p.rect_stroke(r, 14.0, Stroke::new(if on { 2.0f32 } else { 1.0 + ht }, if on { RED } else { theme::blend(LINE(), RED, 0.5 * ht) }));
                    let c = egui::pos2(r.center().x, r.min.y + tile * 0.36);
                    if !self.team_sprite(&p, &s.name, false, c, egui::vec2(tile * 0.6, tile * 0.6)) {
                        p.text(c, Align2::CENTER_CENTER, s.name.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(14.0), MUTED());
                    }
                    let f = crate::fonts::sans_bold_font(13.0);
                    p.text(egui::pos2(r.center().x, r.max.y - 34.0), Align2::CENTER_CENTER, super::charts::clip(ui, &s.name, &f, w - 12.0), f, TEXT());
                    let lv = if s.lv.0 == s.lv.1 { format!("Lv {}", s.lv.0) } else { format!("Lv {}–{}", s.lv.0, s.lv.1) };
                    p.text(egui::pos2(r.center().x, r.max.y - 16.0), Align2::CENTER_CENTER, format!("{lv}  ·  {}%", s.chance), crate::fonts::sans_font(11.5), MUTED());
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Meet this one").clicked() {
                        out = Some(s.clone());
                    }
                }
            });
            ui.add_space(gap);
        }
        out
    }

    /// In doubles: which of the two wild ones the next pick replaces.
    fn wild_which(&mut self, ui: &mut egui::Ui) {
        if self.setup.sim.setup.format != Format::Doubles {
            self.setup.sim.foe.sel = 0;
            return;
        }
        let foes = self.setup.sim.setup.foes.clone();
        ui.horizontal(|ui| {
            ui.label(caps("PICKING FOR"));
            for i in 0..2 {
                let l = foes.get(i).map(|f| format!("{}  {} Lv {}", i + 1, f.species, f.level)).unwrap_or_else(|| format!("{}  —", i + 1));
                if chip(ui, &l, self.setup.sim.foe.sel == i, RED).clicked() {
                    self.setup.sim.foe.sel = i;
                }
            }
        });
        ui.add_space(8.0);
    }

}
