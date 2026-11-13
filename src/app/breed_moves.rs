//! Breeding Planner: the egg move finder. Pick a Pokémon and one of its
//! egg moves; see who can pass it on, easiest first: your Breeders, its own
//! family, fathers that learn it, and chains of fathers.

use super::breed_home::{breedable, move_name, name_of, BreedView};
use super::team_calc::search_pick;
use super::team_home::{caps, small, soft, type_tag};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::breed::{self, Gender, Parent};
use crate::breed_plan::{self, Learn, MoveAnswer, Route};
use crate::theme;
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::sync::Arc;
use std::time::Instant;

#[derive(Default)]
pub(super) struct MovesView {
    /// Filter for the move chips.
    pub q: String,
    /// (species, move, world, breeders) → the answer.
    cache: Option<(String, MoveAnswer)>,
}

impl App {
    pub(super) fn page_egg_moves(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let (setup, dc, mut species, mut mv, breeders) = {
            let f = self.setup.breed.file();
            (f.setup, f.daycare.clone(), f.finder_species.clone(), f.finder_move.clone(), f.breeders.clone())
        };
        // Start from the Day Care's Egg, or the Plan's goal.
        if species.is_empty() {
            let base = [&dc.a, &dc.b].into_iter().find(|p| !p.species.is_empty() && !p.is_ditto()).map(|p| p.species.clone());
            species = base.or_else(|| Some(self.setup.breed.file().target.species.clone()).filter(|s| !s.is_empty())).unwrap_or_default();
        }
        let baby = if species.is_empty() { String::new() } else { breed::baby_of(w, &species) };
        let list = if baby.is_empty() { vec![] } else { breed::egg_moves(w, &baby, setup.world) };
        if !mv.is_empty() && !list.contains(&mv) {
            mv.clear();
        }

        // Who and which move.
        let names: Vec<(String, String)> = breedable(w, setup.world).into_iter().filter(|(_, id)| id != breed::DITTO).collect();
        let name_list: Vec<String> = names.iter().map(|n| n.0.clone()).collect();
        let mut pick_species: Option<String> = None;
        let mut pick_move: Option<String> = None;
        super::sim_home::side_card(ui, kit::C_TEAL, "TEACH AN EGG MOVE", |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(72.0, 72.0), Sense::hover());
                let p = ui.painter().clone();
                p.rect_filled(r, 12.0, theme::blend(kit::sunken(), kit::C_TEAL, 0.08));
                if baby.is_empty() || !self.team_sprite(&p, &baby, false, r.center(), egui::vec2(64.0, 64.0)) {
                    p.text(r.center(), Align2::CENTER_CENTER, if baby.is_empty() { "?" } else { "…" }, crate::fonts::sans_bold_font(20.0), MUTED());
                }
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let cur = if species.is_empty() { "Pick a Pokémon".to_string() } else { name_of(w, &species) };
                    if let Some(n) = search_pick(ui, egui::Id::new("finder-species"), &cur, &name_list, 240.0) {
                        pick_species = names.iter().find(|x| x.0 == n).map(|x| x.1.clone());
                    }
                    if baby.is_empty() {
                        ui.label(small("Pick the Pokémon you're breeding."));
                    } else {
                        ui.label(small(format!(
                            "The Egg is a {}. {} egg move{} it can get from a parent.",
                            name_of(w, &baby),
                            list.len(),
                            if list.len() == 1 { "" } else { "s" }
                        )));
                    }
                });
            });
            if list.is_empty() {
                return;
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(caps("ITS EGG MOVES"));
                ui.add_space(8.0);
                if list.len() > 12 {
                    kit::search_box(ui, &mut self.setup.breed.moves.q, "Filter…", 180.0);
                }
            });
            ui.add_space(6.0);
            let q = self.setup.breed.moves.q.trim().to_lowercase();
            let mut sorted: Vec<(String, String)> = list.iter().map(|k| (move_name(w, k), k.clone())).filter(|(n, _)| q.is_empty() || n.to_lowercase().contains(&q)).collect();
            sorted.sort();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for (n, k) in &sorted {
                    let t = w.move_info(k).map(|m| m.t.clone()).unwrap_or_default();
                    if move_chip(ui, n, &t, *k == mv).clicked() {
                        pick_move = Some(k.clone());
                    }
                }
            });
        });
        let mut changed = false;
        if let Some(s) = pick_species {
            species = s;
            mv.clear();
            self.setup.breed.moves.q.clear();
            changed = true;
        }
        if let Some(m) = pick_move {
            mv = m;
            changed = true;
        }
        if changed || self.setup.breed.file().finder_species != species {
            let f = self.setup.breed.file();
            f.finder_species = species.clone();
            f.finder_move = mv.clone();
            self.setup.breed.touch();
        }
        if mv.is_empty() || baby.is_empty() {
            if !list.is_empty() {
                ui.add_space(4.0);
                ui.label(small("Pick a move to see who can pass it on."));
            }
            return;
        }

        // The answer (cached: it looks at every family).
        let key = format!("{baby}|{mv}|{:?}|{}", setup.world, breeders.iter().map(|b| format!("{}:{}", b.id, b.parent.moves.join(","))).collect::<Vec<_>>().join(";"));
        let ans = match &self.setup.breed.moves.cache {
            Some((k, a)) if *k == key => a.clone(),
            _ => {
                let a = breed_plan::move_routes(w, &baby, &mv, setup.world, &breeders);
                self.setup.breed.moves.cache = Some((key, a.clone()));
                a
            }
        };
        ui.add_space(14.0);
        let mut use_father: Option<String> = None;
        let mut use_breeder: Option<u64> = None;
        let title = move_name(w, &mv).to_uppercase();
        super::sim_home::side_card(ui, kit::C_VIOLET, &title, |ui| {
            if let Some(m) = w.move_info(&mv) {
                let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), Sense::hover());
                let p = ui.painter();
                let mut x = r.min.x;
                if !m.t.is_empty() {
                    x += type_tag(p, egui::pos2(x, r.center().y), &m.t, 10.0) + 6.0;
                }
                if !m.c.is_empty() {
                    x += super::team_home::cat_tag(p, egui::pos2(x, r.center().y), &m.c) + 10.0;
                }
                let stats = [("Power", &m.p), ("Accuracy", &m.a), ("PP", &m.pp)]
                    .iter()
                    .filter(|(_, v)| !v.is_empty() && !matches!(v.trim(), "—" | "-" | "0"))
                    .map(|(l, v)| format!("{l} {v}"))
                    .collect::<Vec<_>>()
                    .join("  ·  ");
                p.text(egui::pos2(x, r.center().y), Align2::LEFT_CENTER, stats, crate::fonts::sans_font(12.0), MUTED());
                if !m.d.is_empty() {
                    ui.add(egui::Label::new(RichText::new(&m.d).size(12.5).color(MUTED())).wrap());
                }
                ui.add_space(10.0);
            }
            let mine: Vec<&Route> = ans.routes.iter().filter(|r| r.breeder.is_some()).collect();
            let own: Vec<&Route> = ans.routes.iter().filter(|r| r.own_family && r.breeder.is_none()).collect();
            let direct: Vec<&Route> = ans.routes.iter().filter(|r| !r.own_family && r.breeder.is_none() && r.hops.len() == 1).collect();
            let chains: Vec<&Route> = ans.routes.iter().filter(|r| !r.own_family && r.breeder.is_none() && r.hops.len() > 1).collect();
            if ans.routes.is_empty() {
                ui.label(RichText::new("Nothing in this world can pass it on").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.label(small("No Pokémon that shares an egg group learns it. Try a MODERN POKéMON world, or teach it another way after hatching."));
            }
            if !mine.is_empty() {
                section(ui, "ALREADY IN YOUR BREEDERS", "Easiest: one of yours knows it.");
                for r in &mine {
                    let b = breeders.iter().find(|b| Some(b.id) == r.breeder);
                    let Some(b) = b else { continue };
                    let sp = name_of(w, &b.parent.species);
                    let label = if b.name.trim().is_empty() { format!("Your {sp}") } else { format!("{} ({sp})", b.name.trim()) };
                    ui.horizontal(|ui| {
                        let (rr, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), Sense::hover());
                        let p = ui.painter().clone();
                        self.team_sprite(&p, &b.parent.species, false, rr.center(), egui::vec2(40.0, 40.0));
                        ui.label(RichText::new(label).size(13.5).color(TEXT()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if soft(ui, "Use in the Day Care").clicked() {
                                use_breeder = Some(b.id);
                            }
                        });
                    });
                }
                ui.add_space(10.0);
            }
            if let Some(r) = own.first() {
                section(ui, "IN THE FAMILY", "No father needed: teach it to the parent and it passes to every Egg.");
                ui.horizontal(|ui| {
                    let (rr, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), Sense::hover());
                    let p = ui.painter().clone();
                    self.team_sprite(&p, &r.hops[0].species, false, rr.center(), egui::vec2(40.0, 40.0));
                    ui.label(RichText::new(format!("{} {}", name_of(w, &r.hops[0].species), r.hops[0].how.text())).size(13.5).color(TEXT()));
                });
                ui.add_space(10.0);
            }
            if !direct.is_empty() {
                section(ui, "FATHERS THAT LEARN IT", "Breed a male that knows it with a female of yours. Easiest first.");
                let avail = ui.available_width();
                let gap = 10.0;
                let cols = ((avail + gap) / (250.0 + gap)).floor().max(1.0) as usize;
                let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
                for row in direct.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for r in row {
                            if self.father_tile(ui, w, r, cw) {
                                use_father = Some(r.hops[0].species.clone());
                            }
                        }
                    });
                    ui.add_space(gap);
                }
            }
            if !chains.is_empty() {
                section(ui, "THROUGH ANOTHER POKÉMON", "When no father learns it straight away: pass it along a chain, a male each time.");
                for r in chains.iter().take(12) {
                    self.chain_row(ui, w, r, &baby);
                }
            }
        });
        if let Some(sp) = use_father {
            let f = self.setup.breed.file();
            let mut dad = Parent::new(&sp, Gender::Male);
            dad.gender = dad.gender();
            dad.moves = vec![mv.clone()];
            let fam_mom = |p: &Parent| !p.is_ditto() && breed::baby_of(w, &p.species) == baby;
            let dc = &mut f.daycare;
            if !fam_mom(&dc.a) {
                dc.a = Parent::new(&baby_parent(w, &baby), Gender::Female);
                dc.a.gender = dc.a.gender();
            }
            dc.b = dad;
            self.setup.breed.touch();
            self.setup.breed.view = BreedView::DayCare;
            self.setup.scroll_top = true;
            self.setup.breed.flash = Some((format!("A male {} that knows {} is the father now. Set him up as you have him.", name_of(w, &sp), move_name(w, &mv)), Instant::now()));
        }
        if let Some(id) = use_breeder {
            let f = self.setup.breed.file();
            if let Some(b) = f.breeders.iter().find(|b| b.id == id).cloned() {
                let slot = if b.parent.gender() == Gender::Female || breed::baby_of(w, &b.parent.species) == baby && f.daycare.a.species.is_empty() { 0 } else { 1 };
                let held = if slot == 0 { f.daycare.a.held.clone() } else { f.daycare.b.held.clone() };
                let p = Parent { held: if b.parent.held == breed::Held::Nothing { held } else { b.parent.held.clone() }, ..b.parent.clone() };
                if slot == 0 {
                    f.daycare.a = p;
                } else {
                    f.daycare.b = p;
                }
                self.setup.breed.touch();
                self.setup.breed.view = BreedView::DayCare;
                self.setup.scroll_top = true;
                self.setup.breed.flash = Some((format!("Put your {} in the Day Care.", name_of(w, &b.parent.species)), Instant::now()));
            }
        }
    }

    /// A father that learns the move: returns true for "Use as the father".
    fn father_tile(&mut self, ui: &mut egui::Ui, w: &Wiki, r: &Route, width: f32) -> bool {
        let hop = &r.hops[0];
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 112.0), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(rect, 12.0, kit::sunken());
        p.rect_stroke(rect, 12.0, Stroke::new(1.0f32, LINE()));
        let tile = Rect::from_min_size(rect.min + egui::vec2(10.0, 10.0), egui::vec2(56.0, 56.0));
        if !self.team_sprite(&p, &hop.species, false, tile.center(), egui::vec2(54.0, 54.0)) {
            p.text(tile.center(), Align2::CENTER_CENTER, "…", crate::fonts::sans_bold_font(13.0), MUTED());
        }
        let x = tile.max.x + 10.0;
        let tw = rect.max.x - x - 8.0;
        let f = crate::fonts::sans_bold_font(14.0);
        p.text(egui::pos2(x, rect.min.y + 20.0), Align2::LEFT_CENTER, super::charts::clip(ui, &name_of(w, &hop.species), &f, tw), f, TEXT());
        let how = how_tag(&hop.how);
        super::breed_box::tag(&p, egui::pos2(x, rect.min.y + 34.0), &how.0, how.1);
        let f2 = crate::fonts::sans_font(11.5);
        let place = r.place.as_ref().map(|(pl, st)| if st.is_empty() { pl.clone() } else { format!("{pl} · {st}") }).unwrap_or_else(|| "not in the wild".into());
        p.text(egui::pos2(x, rect.min.y + 64.0), Align2::LEFT_CENTER, super::charts::clip(ui, &place, &f2, tw), f2, MUTED());
        let br = Rect::from_min_max(egui::pos2(rect.min.x + 10.0, rect.max.y - 36.0), egui::pos2(rect.max.x - 10.0, rect.max.y - 8.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::left_to_right(egui::Align::Center)));
        soft(&mut child, "Use as the father").on_hover_text("Put a male one in the Day Care with this move").clicked()
    }

    /// A chain: who learns it → who carries it → your Pokémon.
    fn chain_row(&mut self, ui: &mut egui::Ui, w: &Wiki, r: &Route, baby: &str) {
        let full = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(full, 64.0), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(rect, 12.0, kit::sunken());
        let mut x = rect.min.x + 12.0;
        let cy = rect.center().y;
        let steps: Vec<(String, Option<Learn>)> = r.hops.iter().enumerate().map(|(i, h)| (h.species.clone(), (i == 0).then(|| h.how.clone()))).chain(std::iter::once((baby.to_string(), None))).collect();
        for (i, (sp, how)) in steps.iter().enumerate() {
            let c = egui::pos2(x + 22.0, cy - 6.0);
            if !self.team_sprite(&p, sp, false, c, egui::vec2(40.0, 40.0)) {
                p.text(c, Align2::CENTER_CENTER, "…", crate::fonts::sans_bold_font(12.0), MUTED());
            }
            let f = crate::fonts::sans_bold_font(11.5);
            let label = match how {
                Some(l) => format!("{} · {}", name_of(w, sp), l.short()),
                None if i == steps.len() - 1 => name_of(w, sp),
                None => format!("{} ♂", name_of(w, sp)),
            };
            let g = p.text(egui::pos2(x + 48.0, cy), Align2::LEFT_CENTER, label, f, if i == steps.len() - 1 { TEXT() } else { MUTED() });
            x = g.max.x + 12.0;
            if i + 1 < steps.len() {
                p.text(egui::pos2(x, cy), Align2::LEFT_CENTER, "→", crate::fonts::sans_bold_font(16.0), kit::C_VIOLET);
                x += 26.0;
            }
        }
        if let Some((pl, _)) = &r.place {
            let f2 = crate::fonts::sans_font(11.5);
            p.text(egui::pos2(rect.max.x - 12.0, cy), Align2::RIGHT_CENTER, super::charts::clip(ui, &format!("{} at {pl}", name_of(w, &r.hops[0].species)), &f2, (rect.max.x - x - 24.0).max(60.0)), f2, MUTED());
        }
        ui.add_space(6.0);
    }
}

/// A family's species to use as the mother (one that can breed).
fn baby_parent(w: &Wiki, baby: &str) -> String {
    let fam: Vec<String> = w.family(baby).into_iter().flatten().collect();
    fam.into_iter().find(|s| breed::info(s).is_some_and(|i| !i.no_eggs())).unwrap_or_else(|| baby.to_string())
}

fn how_tag(l: &Learn) -> (String, Color32) {
    match l {
        Learn::Level(_) => (l.short(), kit::C_GREEN),
        Learn::Tm(_) => (l.short(), kit::C_BLUE),
        Learn::Tutor => ("Move tutor".into(), kit::C_ORANGE),
        Learn::Egg => ("Egg move".into(), kit::C_VIOLET),
    }
}

fn section(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.horizontal(|ui| {
        ui.label(caps(title));
        ui.add_space(6.0);
        ui.label(small(sub));
    });
    ui.add_space(6.0);
}

/// A move chip with its type's colour.
fn move_chip(ui: &mut egui::Ui, text: &str, ty: &str, on: bool) -> egui::Response {
    let col = super::pokedex::type_color(ty);
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(12.0), TEXT());
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(30.0, 12.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let fill = if on { theme::blend(CARD(), col, 0.3) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) };
    let p = ui.painter();
    p.rect_filled(r, r.height() / 2.0, fill);
    p.rect_stroke(r, r.height() / 2.0, Stroke::new(if on { 1.5f32 } else { 1.0 }, if on { col } else { LINE() }));
    p.circle_filled(egui::pos2(r.min.x + 12.0, r.center().y), 4.0, col);
    p.galley(r.min + egui::vec2(21.0, 6.0), g, TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(if ty.is_empty() { String::new() } else { format!("{ty} type") })
}
