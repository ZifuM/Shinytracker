//! Team Planner → Build: the six slots, a set editor (moves, item, ability,
//! nature, EVs and IVs with live stats) and Showdown-style pickers that only
//! offer what each Pokémon can really have in Platinum Together.

use super::pokedex::type_color;
use super::team_home::{caps, cat_tag, chip, small, soft, type_tag, Picker, ACCENT};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{self, World, STATS};
use crate::teams::Member;
use crate::theme;
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

/// How a move is learned.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum How {
    Level(String),
    Tm(String),
    Tutor,
    Egg,
}

impl How {
    pub fn label(&self) -> String {
        match self {
            How::Level(l) if l.chars().all(|c| c.is_ascii_digit()) => format!("Lv {l}"),
            How::Level(l) => l.clone(),
            How::Tm(t) => t.clone(),
            How::Tutor => "Tutor".into(),
            How::Egg => "Egg".into(),
        }
    }
    fn rank(&self) -> u8 {
        match self {
            How::Level(_) => 0,
            How::Tm(_) => 1,
            How::Tutor => 2,
            How::Egg => 3,
        }
    }
}

/// One way to learn a move: how, and as which Pokémon (a pre-evolution).
#[derive(Clone, Debug)]
pub(super) struct Learn {
    pub key: String,
    pub how: How,
    /// The pre-evolution that learns it, when it isn't this one.
    pub via: Option<String>,
    /// Only in MODERN worlds.
    pub modern: bool,
}

/// The wiki id of a battle species (forms use their base species).
pub(super) fn wiki_id(name: &str) -> Option<String> {
    let d = battle::data();
    let s = d.species(name)?;
    if !s.wiki.is_empty() {
        return Some(s.wiki.clone());
    }
    d.species(&s.base).map(|b| b.wiki.clone()).filter(|w| !w.is_empty())
}

/// Every way this Pokémon (or what it evolves from) learns each move,
/// best way first per move.
pub(super) fn learnable(w: &Wiki, species: &str, world: World) -> Vec<Learn> {
    let mut out: Vec<Learn> = Vec::new();
    let Some(mut id) = wiki_id(species) else { return out };
    let mut via: Option<String> = None;
    for _ in 0..4 {
        if let Some(ls) = w.learnset(&id) {
            let mut add = |key: &str, how: How, modern: bool| {
                if key.is_empty() {
                    return;
                }
                out.push(Learn { key: key.to_string(), how, via: via.clone(), modern });
            };
            for l in &ls.lv {
                add(&l.key, How::Level(l.level.clone()), false);
            }
            for t in &ls.tm {
                if let Some(k) = w.data.tms.get(t) {
                    add(k, How::Tm(t.clone()), false);
                }
            }
            for k in &ls.tutor {
                add(k, How::Tutor, false);
            }
            for k in &ls.egg {
                add(k, How::Egg, false);
            }
            if world == World::Modern {
                for l in &ls.mlv {
                    add(&l.key, How::Level(l.level.clone()), true);
                }
                for t in &ls.mtm {
                    if let Some(k) = w.data.tms.get(t) {
                        add(k, How::Tm(t.clone()), true);
                    }
                }
                for k in &ls.mtutor {
                    add(k, How::Tutor, true);
                }
                for k in &ls.megg {
                    add(k, How::Egg, true);
                }
            }
        }
        match w.evolves_from(&id) {
            Some(e) if !e.from.is_empty() => {
                id = e.from.clone();
                via = Some(w.mon(&id).map(|m| m.name.clone()).unwrap_or_else(|| id.clone()));
            }
            _ => break,
        }
    }
    // best way first: this Pokémon before a pre-evolution, Platinum before MODERN,
    // then level < TM < tutor < egg
    out.sort_by(|a, b| (a.key.as_str(), a.via.is_some(), a.modern, a.how.rank()).cmp(&(b.key.as_str(), b.via.is_some(), b.modern, b.how.rank())));
    out
}

/// The best way to learn each move, by move key.
pub(super) fn best_ways(all: &[Learn]) -> Vec<&Learn> {
    let mut v: Vec<&Learn> = Vec::new();
    for l in all {
        if v.last().is_none_or(|x| x.key != l.key) {
            v.push(l);
        }
    }
    v
}

/// Species you can pick in this world.
pub(super) fn pickable(world: World) -> Vec<&'static battle::Species> {
    let d = battle::data();
    let mut v: Vec<&battle::Species> = d
        .species
        .iter()
        .filter(|s| world == World::Modern || !s.modern_only())
        .filter(|s| s.stats.iter().any(|x| *x > 0))
        .collect();
    v.sort_by_key(|s| (s.num, !s.base.is_empty(), s.name.clone()));
    v
}

/// Hold items in the game (by the wiki's list), Mega Stones only in MODERN.
pub(super) fn holdable(w: Option<&Wiki>, world: World) -> Vec<&'static battle::ItemInfo> {
    let d = battle::data();
    let names: std::collections::HashSet<String> = w.map(|w| w.data.items.iter().map(|i| battle::to_id(&i.name)).collect()).unwrap_or_default();
    let mut v: Vec<&battle::ItemInfo> = d
        .items
        .iter()
        .filter(|i| world == World::Modern || i.mega.is_empty())
        .filter(|i| names.is_empty() || names.contains(&i.id) || (!i.mega.is_empty() && world == World::Modern))
        .collect();
    v.sort_by(|a, b| a.n.cmp(&b.n));
    v
}

/// A move's wiki key from any spelling ("Extreme Speed", "ExtremeSpeed",
/// "Hi Jump Kick").
pub(super) fn mkey(w: &Wiki, name: &str) -> Option<String> {
    if let Some(k) = w.move_key(name) {
        return Some(k);
    }
    let id = battle::to_id(name);
    let want = battle::move_alias(&id);
    w.data.moves.iter().find(|(_, m)| {
        let mid = battle::to_id(&m.n);
        mid == id || battle::move_alias(&mid) == want
    }).map(|(k, _)| k.clone())
}

/// What an item does, from the game's own text.
pub(super) fn item_text<'a>(w: Option<&'a Wiki>, name: &str) -> Option<&'a crate::wiki::Item> {
    let id = battle::to_id(name);
    w?.data.items.iter().find(|i| battle::to_id(&i.name) == id)
}

impl App {
    pub(super) fn team_build(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.setup.team.team().cloned() else { return };
        self.team_slots(ui, &t);
        ui.add_space(14.0);
        if self.setup.team.import.is_some() {
            self.team_import_box(ui, Some(t.id));
            ui.add_space(14.0);
        } else if let Some(n) = self.setup.team.import_note.clone() {
            ui.label(RichText::new(n).size(12.5).color(kit::C_ORANGE));
            ui.add_space(8.0);
        }
        let slot = self.setup.team.slot.min(t.members.len());
        self.setup.team.slot = slot;
        if t.members.is_empty() && self.setup.team.picker.is_none() {
            self.setup.team.picker = Some(Picker::Species);
        }
        if slot >= t.members.len() && self.setup.team.picker != Some(Picker::Species) {
            self.setup.team.slot = t.members.len().saturating_sub(1);
        }
        match self.setup.team.picker {
            Some(p) => self.team_picker(ui, &t, p),
            None => {
                if let Some(m) = t.members.get(self.setup.team.slot).cloned() {
                    self.team_editor(ui, t.world, m);
                }
            }
        }
    }

    // ───────────────────────────── the six slots ─────────────────────────────

    fn team_slots(&mut self, ui: &mut egui::Ui, t: &crate::teams::Team) {
        let avail = ui.available_width();
        let gap = 10.0;
        let w = ((avail - gap * 5.0) / 6.0).floor();
        let h = 104.0;
        let adding = self.setup.team.picker == Some(Picker::Species) && self.setup.team.slot >= t.members.len();
        let mut pick: Option<usize> = None;
        let mut add = false;
        let mut action: Option<(usize, &str)> = None;
        let caught = self.dex_caught();
        let owned: Vec<bool> = t.members.iter().map(|m| self.team_species(&m.species).is_some_and(|s| caught.contains_key(&s.slug))).collect();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for i in 0..6 {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                match t.members.get(i) {
                    Some(m) => {
                        let on = i == self.setup.team.slot && !adding;
                        let ty = battle::data().species(&m.species).map(|s| s.types_in(t.world).to_vec()).unwrap_or_default();
                        let col = ty.first().map(|x| type_color(x)).unwrap_or(kit::C_SLATE);
                        let fill = if on { theme::blend(CARD(), col, 0.16) } else { theme::blend(CARD(), CARD_HOVER(), ht) };
                        p.rect_filled(r, 12.0, fill);
                        p.rect_stroke(r, 12.0, Stroke::new(if on { 2.0f32 } else { 1.0 }, if on { theme::blend(LINE(), col, 0.8) } else { LINE() }));
                        let c = egui::pos2(r.min.x + 30.0, r.min.y + 34.0);
                        p.circle_filled(c, 22.0, theme::blend(kit::sunken(), col, 0.22));
                        if !self.team_sprite(&p, &m.species, m.shiny, c, egui::vec2(52.0, 52.0)) {
                            p.text(c, Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(13.0), MUTED());
                        }
                        let x = r.min.x + 58.0;
                        let tw = r.max.x - x - if owned.get(i).copied().unwrap_or(false) { 30.0 } else { 8.0 };
                        let f = crate::fonts::sans_bold_font(13.0);
                        p.text(egui::pos2(x, r.min.y + 24.0), Align2::LEFT_CENTER, super::charts::clip(ui, m.display_name(), &f, tw), f, TEXT());
                        let f2 = crate::fonts::sans_font(11.0);
                        let lv = format!("Lv {}{}", m.level, if m.shiny { "  ✨" } else { "" });
                        p.text(egui::pos2(x, r.min.y + 42.0), Align2::LEFT_CENTER, lv, f2.clone(), MUTED());
                        let mut tx = r.min.x + 10.0;
                        for ty in &ty {
                            tx += type_tag(&p, egui::pos2(tx, r.min.y + 70.0), ty, 8.5) + 4.0;
                        }
                        let item = if m.item.is_empty() { "No item".to_string() } else { m.item.clone() };
                        p.text(egui::pos2(r.min.x + 10.0, r.min.y + 90.0), Align2::LEFT_CENTER, super::charts::clip(ui, &item, &f2, w - 20.0), f2, if m.item.is_empty() { theme::blend(MUTED(), CARD(), 0.4) } else { MUTED() });
                        let resp = if owned.get(i).copied().unwrap_or(false) {
                            let b = Rect::from_center_size(egui::pos2(r.max.x - 16.0, r.min.y + 16.0), egui::vec2(20.0, 20.0));
                            p.circle_filled(b.center(), 10.0, super::charts::faint(kit::C_GOLD, 0.25));
                            p.text(b.center(), Align2::CENTER_CENTER, "✨", crate::fonts::sans_font(11.0), kit::C_GOLD);
                            resp.on_hover_text("You have this one shiny (Shiny Dex)")
                        } else {
                            resp
                        };
                        if resp.clicked() {
                            pick = Some(i);
                        }
                        let n = t.members.len();
                        resp.on_hover_cursor(egui::CursorIcon::PointingHand).context_menu(|ui| {
                            if i > 0 && ui.button("◀  Move left").clicked() {
                                action = Some((i, "left"));
                                ui.close_menu();
                            }
                            if i + 1 < n && ui.button("Move right  ▶").clicked() {
                                action = Some((i, "right"));
                                ui.close_menu();
                            }
                            if n < 6 && ui.button("Duplicate").clicked() {
                                action = Some((i, "dup"));
                                ui.close_menu();
                            }
                            if ui.button("Remove from team").clicked() {
                                action = Some((i, "remove"));
                                ui.close_menu();
                            }
                        });
                    }
                    None if i == t.members.len() => {
                        let on = adding;
                        p.rect_filled(r, 12.0, if on { theme::blend(CARD(), ACCENT, 0.14) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) });
                        let stroke = Stroke::new(1.5f32, if on { ACCENT } else { theme::blend(LINE(), ACCENT, 0.3 + 0.5 * ht) });
                        let pts = dashed(r.shrink(1.0));
                        for seg in pts.chunks(2) {
                            if seg.len() == 2 {
                                p.line_segment([seg[0], seg[1]], stroke);
                            }
                        }
                        p.text(r.center() - egui::vec2(0.0, 10.0), Align2::CENTER_CENTER, "+", crate::fonts::sans_bold_font(22.0), if on { ACCENT } else { MUTED() });
                        p.text(r.center() + egui::vec2(0.0, 16.0), Align2::CENTER_CENTER, "Add Pokémon", crate::fonts::sans_font(12.0), if on { TEXT() } else { MUTED() });
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            add = true;
                        }
                    }
                    None => {
                        p.rect_filled(r, 12.0, theme::blend(kit::sunken(), CARD(), 0.4));
                    }
                }
            }
        });
        if let Some(i) = pick {
            self.setup.team.slot = i;
            self.setup.team.picker = None;
        }
        if add {
            self.setup.team.slot = t.members.len();
            self.setup.team.picker = Some(Picker::Species);
            self.setup.team.query.clear();
        }
        if let Some((i, what)) = action {
            if let Some(tm) = self.setup.team.team() {
                match what {
                    "left" => tm.members.swap(i, i - 1),
                    "right" => tm.members.swap(i, i + 1),
                    "dup" => {
                        let m = tm.members[i].clone();
                        tm.members.insert(i + 1, m);
                    }
                    _ => {
                        tm.members.remove(i);
                    }
                }
            }
            self.setup.team.slot = match what {
                "left" => i - 1,
                "right" | "dup" => i + 1,
                _ => i.saturating_sub(1),
            };
            self.setup.team.touch();
        }
        // the row under the slots: paste, and counts
        ui.add_space(8.0);
        let mut paste = false;
        ui.horizontal(|ui| {
            ui.label(small(format!("{} of 6", t.members.len())));
            ui.add_space(6.0);
            if t.members.len() < 6 && ui.link(RichText::new("Paste Pokémon from text").size(12.0)).clicked() {
                paste = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(small("Right-click a slot to move, copy or remove it."));
            });
        });
        if paste {
            self.setup.team.import = Some(String::new());
            self.setup.team.import_note = None;
        }
    }

    // ───────────────────────────── the set editor ─────────────────────────────

    fn team_editor(&mut self, ui: &mut egui::Ui, world: World, orig: Member) {
        let d = battle::data();
        let Some(sp) = d.species(&orig.species) else {
            kit::card(ui, |ui| {
                ui.label(RichText::new(format!("\"{}\" isn't a Pokémon the planner knows.", orig.species)).size(14.0).color(TEXT()));
            });
            return;
        };
        let mut m = orig.clone();
        let wiki = self.wiki.clone();
        let avail = ui.available_width();
        let wide = avail >= 980.0;
        let gap = 16.0;
        let left_w = if wide { 300.0 } else { avail };
        let mut open: Option<Picker> = None;
        let mut open_r: Option<Picker> = None;
        let mut to_dex = false;
        let mut to_breed = false;
        let mut left = |app: &mut App, ui: &mut egui::Ui, m: &mut Member| {
            kit::card(ui, |ui| {
                ui.set_width(left_w - 32.0);
                // sprite and species
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 120.0), Sense::click());
                let ty = sp.types_in(world).to_vec();
                let col = ty.first().map(|x| type_color(x)).unwrap_or(kit::C_SLATE);
                let p = ui.painter().clone();
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                p.rect_filled(r, 12.0, theme::blend(kit::sunken(), col, 0.12 + 0.08 * ht));
                let c = egui::pos2(r.min.x + 60.0, r.center().y);
                if !app.team_sprite(&p, &m.species, m.shiny, c, egui::vec2(104.0, 104.0)) {
                    p.circle_filled(c, 40.0, theme::blend(kit::sunken(), col, 0.25));
                    p.text(c, Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(22.0), MUTED());
                }
                let x = r.min.x + 124.0;
                let f = crate::fonts::sans_bold_font(17.0);
                p.text(egui::pos2(x, r.min.y + 30.0), Align2::LEFT_CENTER, super::charts::clip(ui, &sp.name, &f, r.max.x - x - 8.0), f, TEXT());
                p.text(egui::pos2(x, r.min.y + 50.0), Align2::LEFT_CENTER, format!("#{:03}  ·  BST {}", sp.num, sp.bst(world)), crate::fonts::sans_font(11.5), MUTED());
                let mut tx = x;
                for t in &ty {
                    tx += type_tag(&p, egui::pos2(tx, r.min.y + 74.0), t, 9.5) + 4.0;
                }
                p.text(egui::pos2(x, r.min.y + 100.0), Align2::LEFT_CENTER, "Change ›", crate::fonts::sans_font(11.5), theme::blend(MUTED(), ACCENT, 0.4 + 0.6 * ht));
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Pick a different Pokémon (keeps the moves it can still learn)").clicked() {
                    open = Some(Picker::Species);
                }
                ui.add_space(12.0);
                egui::Grid::new("team-ed-left").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui| {
                    ui.label(caps("NICKNAME"));
                    ui.add(egui::TextEdit::singleline(&mut m.nickname).desired_width(150.0).char_limit(12)).on_hover_text("Leave empty to use its name");
                    ui.end_row();
                    ui.label(caps("LEVEL"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut m.level).range(1..=100).speed(0.2));
                        for l in [5u8, 50, 100] {
                            if chip(ui, &l.to_string(), m.level == l, ACCENT).clicked() {
                                m.level = l;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(caps("ITEM"));
                    let item = if m.item.is_empty() { "None" } else { m.item.as_str() };
                    if pick_button(ui, item, 150.0).clicked() {
                        open = Some(Picker::Item);
                    }
                    ui.end_row();
                    ui.label(caps("ABILITY"));
                    let abs = sp.abilities_in(world);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for a in &abs {
                            let label = if world == World::Modern && *a == sp.hidden && abs.len() > 1 { format!("{a} (H)") } else { a.clone() };
                            if chip(ui, &label, m.ability == *a, ACCENT).clicked() {
                                m.ability = a.clone();
                            }
                        }
                        if !abs.contains(&m.ability) && !m.ability.is_empty() {
                            chip(ui, &format!("{} ⚠", m.ability), true, kit::C_ORANGE).on_hover_text("Not one of this Pokémon's abilities in this world");
                        }
                    });
                    ui.end_row();
                    ui.label(caps("GENDER"));
                    ui.horizontal(|ui| {
                        for (g, l) in [(None, "Any"), (Some('M'), "♂"), (Some('F'), "♀")] {
                            if chip(ui, l, m.gender == g, ACCENT).clicked() {
                                m.gender = g;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(caps("SHINY"));
                    ui.horizontal(|ui| {
                        kit::toggle(ui, &mut m.shiny);
                        if m.shiny {
                            ui.label(small("✨"));
                        }
                    });
                    ui.end_row();
                    ui.label(caps("HAPPINESS"));
                    ui.add(egui::DragValue::new(&mut m.happiness).range(0..=255).speed(1.0)).on_hover_text("Return is strongest at 255, Frustration at 0");
                    ui.end_row();
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.link(RichText::new("Open in the Pokédex ›").size(12.0)).clicked() {
                        to_dex = true;
                    }
                    ui.add_space(8.0);
                    if ui.link(RichText::new("Plan breeding it ›").size(12.0)).on_hover_text("Open the Breeding Planner with this set as the goal: nature, stats at 31, Ability and egg moves").clicked() {
                        to_breed = true;
                    }
                });
            });
        };
        let mut right = |app: &mut App, ui: &mut egui::Ui, m: &mut Member| {
            app.team_moves_card(ui, world, sp, m, wiki.as_deref(), &mut open_r);
            ui.add_space(gap);
            app.team_stats_card(ui, world, sp, m);
            ui.add_space(gap);
            app.team_sets_card(ui, world, sp, m);
            ui.add_space(gap);
            kit::card(ui, |ui| {
                ui.label(caps("NOTES"));
                ui.add_space(4.0);
                let nw = ui.available_width();
                ui.add(egui::TextEdit::multiline(&mut m.notes).hint_text("Why it's here, what it beats, things to remember…").desired_rows(2).desired_width(nw));
            });
        };
        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(left_w);
                    left(self, ui, &mut m);
                });
                let rw = ui.available_width();
                ui.allocate_ui_with_layout(egui::vec2(rw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(rw);
                    right(self, ui, &mut m);
                });
            });
        } else {
            left(self, ui, &mut m);
            ui.add_space(gap);
            right(self, ui, &mut m);
        }
        if m != orig {
            let slot = self.setup.team.slot;
            if let Some(t) = self.setup.team.team() {
                if let Some(x) = t.members.get_mut(slot) {
                    *x = m;
                }
            }
            self.setup.team.touch();
        }
        if let Some(p) = open.or(open_r) {
            self.setup.team.picker = Some(p);
            self.setup.team.query.clear();
            self.setup.team.sort = None;
        }
        if to_dex {
            let id = wiki_id(&orig.species);
            self.info_open_dex(id);
        }
        if to_breed {
            self.plan_from_member(&orig, world);
        }
    }

    fn team_moves_card(&mut self, ui: &mut egui::Ui, world: World, sp: &battle::Species, m: &mut Member, w: Option<&Wiki>, open: &mut Option<Picker>) {
        let learn = w.map(|w| learnable(w, &sp.name, world)).unwrap_or_default();
        let ways = best_ways(&learn);
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Moves").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.label(small(format!("{} it can learn here", ways.len())));
            });
            ui.add_space(8.0);
            let full = ui.available_width();
            for i in 0..4 {
                let name = m.moves.get(i).cloned().unwrap_or_default();
                let (r, resp) = ui.allocate_exact_size(egui::vec2(full, 44.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                p.rect_filled(r, 10.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                if name.is_empty() {
                    p.text(egui::pos2(r.min.x + 14.0, r.center().y), Align2::LEFT_CENTER, format!("+  Move {}", i + 1), crate::fonts::sans_font(13.0), MUTED());
                } else {
                    let key = w.and_then(|w| mkey(w, &name));
                    let info = key.as_deref().and_then(|k| w.and_then(|w| w.move_info(k)));
                    let t = info.map(|i| i.t.clone()).unwrap_or_default();
                    let col = if t.is_empty() { kit::C_SLATE } else { type_color(&t) };
                    p.rect_filled(Rect::from_min_size(r.min, egui::vec2(5.0, r.height())), egui::Rounding { nw: 10.0, sw: 10.0, ne: 0.0, se: 0.0 }, col);
                    let f = crate::fonts::sans_bold_font(13.5);
                    p.text(egui::pos2(r.min.x + 16.0, r.center().y), Align2::LEFT_CENTER, &name, f, TEXT());
                    let mut x = r.min.x + 190.0;
                    if let Some(info) = info {
                        x += type_tag(&p, egui::pos2(x, r.center().y), &info.t, 9.0) + 6.0;
                        x += cat_tag(&p, egui::pos2(x, r.center().y), &info.c) + 10.0;
                        let pw = if info.p.is_empty() || info.p == "0" { "—".to_string() } else { info.p.clone() };
                        let ac = if info.a.is_empty() || info.a == "0" { "—".to_string() } else { format!("{}%", info.a) };
                        if full > 520.0 {
                            p.text(egui::pos2(x, r.center().y), Align2::LEFT_CENTER, format!("Power {pw}   Acc {ac}"), crate::fonts::sans_font(11.5), MUTED());
                        }
                    }
                    // how it's learned
                    let way = key.as_ref().and_then(|k| ways.iter().find(|l| &l.key == k));
                    let (txt, c) = match way {
                        Some(l) => (
                            format!("{}{}", l.how.label(), l.via.as_ref().map(|v| format!(" as {v}")).unwrap_or_default()),
                            if l.modern { kit::C_PINK } else { ACCENT },
                        ),
                        None if name.starts_with("Hidden Power") && ways.iter().any(|l| l.key == "hidden_power") => ("TM10".into(), ACCENT),
                        None if w.is_some() => ("Can't learn ⚠".into(), kit::C_ORANGE),
                        None => (String::new(), MUTED()),
                    };
                    p.text(egui::pos2(r.max.x - 34.0, r.center().y), Align2::RIGHT_CENTER, txt, crate::fonts::sans_bold_font(11.0), super::charts::ink(c));
                }
                // clear
                let xr = Rect::from_center_size(egui::pos2(r.max.x - 16.0, r.center().y), egui::vec2(22.0, 22.0));
                let mut cleared = false;
                if !name.is_empty() {
                    let xresp = ui.interact(xr, resp.id.with("x"), Sense::click());
                    let xc = if xresp.hovered() { TEXT() } else { theme::blend(MUTED(), CARD(), 0.3) };
                    let k = 4.5;
                    p.line_segment([xr.center() + egui::vec2(-k, -k), xr.center() + egui::vec2(k, k)], Stroke::new(1.6f32, xc));
                    p.line_segment([xr.center() + egui::vec2(-k, k), xr.center() + egui::vec2(k, -k)], Stroke::new(1.6f32, xc));
                    if xresp.on_hover_text("Clear this move").clicked() {
                        cleared = true;
                        if i < m.moves.len() {
                            m.moves.remove(i);
                        }
                    }
                }
                if !cleared && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    *open = Some(Picker::Move(i));
                }
                ui.add_space(6.0);
            }
            // Hidden Power's type
            if m.moves.iter().any(|x| x.starts_with("Hidden Power")) {
                let (t, pw) = battle::hidden_power(m.ivs);
                ui.horizontal(|ui| {
                    ui.label(small(format!("Hidden Power with these IVs: {t}, power {pw}.")));
                    if ui.link(RichText::new("Set IVs for a type…").size(12.0)).on_hover_text("Pick a type below").clicked() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("team-hp-pick"), true));
                    }
                });
                let showing = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("team-hp-pick")).unwrap_or(false));
                if showing {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for ty in battle::HP_TYPES {
                            if chip(ui, ty, t == ty, type_color(ty)).clicked() {
                                if let Some(ivs) = hp_ivs(ty, m.ivs) {
                                    m.ivs = ivs;
                                }
                                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("team-hp-pick"), false));
                            }
                        }
                    });
                }
            }
        });
    }

    fn team_stats_card(&mut self, ui: &mut egui::Ui, world: World, sp: &battle::Species, m: &mut Member) {
        let d = battle::data();
        let base = sp.base_stats(world);
        kit::card(ui, |ui| {
            let left = battle::ev_left(m.evs);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Stats").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                let (txt, col) = if left < 0 { (format!("{} EVs over the limit", -left), kit::C_RED) } else if left == 0 { ("All 510 EVs spent".into(), kit::C_GREEN) } else { (format!("{left} EVs left"), MUTED()) };
                ui.label(RichText::new(txt).size(12.0).color(col));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // nature
                    let cur = m.nature.clone();
                    kit::Dropdown::from_id_salt("team-nature")
                        .selected_text(nature_label(&cur))
                        .width(190.0)
                        .height(360.0)
                        .show_ui(ui, |ui| {
                            for n in &d.natures {
                                ui.selectable_value(&mut m.nature, n.name.clone(), nature_label(&n.name));
                            }
                        });
                    ui.label(caps("NATURE"));
                    if soft(ui, "Reset").on_hover_text("EVs to 0, IVs to 31").clicked() {
                        m.evs = [0; 6];
                        m.ivs = [31; 6];
                    }
                });
            });
            ui.add_space(8.0);
            let nat = d.nature(&m.nature).cloned();
            let fin = battle::stats(base, m.ivs, m.evs, m.level, nat.as_ref(), sp.id == "shedinja");
            let full = ui.available_width();
            let label_w = 36.0;
            let base_w = 110.0;
            let num_w = 46.0;
            let iv_w = 46.0;
            let out_w = 64.0;
            let nat_w = 44.0;
            let slider_w = (full - label_w - base_w - num_w - iv_w - out_w - nat_w - 6.0 * 8.0 - 16.0).max(80.0);
            // header
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.add_sized([label_w, 14.0], egui::Label::new(caps("")));
                ui.add_sized([base_w, 14.0], egui::Label::new(caps("BASE")));
                ui.add_sized([slider_w, 14.0], egui::Label::new(caps("EVS")));
                ui.add_sized([num_w, 14.0], egui::Label::new(caps("")));
                ui.add_sized([iv_w, 14.0], egui::Label::new(caps("IVS")));
                ui.add_sized([nat_w, 14.0], egui::Label::new(caps("")));
                ui.add_sized([out_w, 14.0], egui::Label::new(caps("STAT")));
            });
            for i in 0..6 {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let plus = nat.as_ref().is_some_and(|n| !n.neutral() && n.plus == i);
                    let minus = nat.as_ref().is_some_and(|n| !n.neutral() && n.minus == i);
                    let lc = if plus { kit::C_RED } else if minus { kit::C_BLUE } else { TEXT() };
                    ui.add_sized([label_w, 24.0], egui::Label::new(RichText::new(STATS[i]).font(crate::fonts::sans_bold_font(12.5)).color(lc)));
                    // base bar
                    let (r, _) = ui.allocate_exact_size(egui::vec2(base_w, 24.0), Sense::hover());
                    let p = ui.painter();
                    let b = base[i] as f32;
                    p.text(egui::pos2(r.min.x + 26.0, r.center().y), Align2::RIGHT_CENTER, base[i].to_string(), crate::fonts::sans_font(12.0), MUTED());
                    let bar = Rect::from_min_size(egui::pos2(r.min.x + 32.0, r.center().y - 4.0), egui::vec2(r.width() - 32.0, 8.0));
                    p.rect_filled(bar, 4.0, kit::sunken());
                    p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bar.width() * (b / 180.0).min(1.0), 8.0)), 4.0, stat_color(base[i]));
                    // EV slider
                    let room = (left + m.evs[i] as i32).clamp(0, 252) as u16;
                    if ev_slider(ui, egui::vec2(slider_w, 24.0), &mut m.evs[i], room) {
                        // nothing else
                    }
                    ui.add_sized([num_w, 24.0], egui::DragValue::new(&mut m.evs[i]).range(0..=252).speed(1.0));
                    ui.add_sized([iv_w, 24.0], egui::DragValue::new(&mut m.ivs[i]).range(0..=31).speed(0.2));
                    // nature buttons: + / −
                    ui.allocate_ui_with_layout(egui::vec2(nat_w, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if i > 0 {
                            if nat_btn(ui, "+", plus, kit::C_RED).on_hover_text(format!("Nature that raises {}", STATS[i])).clicked() {
                                let mn = nat.as_ref().filter(|n| !n.neutral()).map(|n| n.minus).unwrap_or(if i == 3 { 1 } else { 3 });
                                m.nature = nature_for(i, if mn == i { if i == 3 { 1 } else { 3 } } else { mn });
                            }
                            if nat_btn(ui, "−", minus, kit::C_BLUE).on_hover_text(format!("Nature that lowers {}", STATS[i])).clicked() {
                                let pl = nat.as_ref().filter(|n| !n.neutral()).map(|n| n.plus).unwrap_or(if i == 1 { 3 } else { 1 });
                                m.nature = nature_for(if pl == i { if i == 1 { 3 } else { 1 } } else { pl }, i);
                            }
                        }
                    });
                    ui.add_sized([out_w, 24.0], egui::Label::new(RichText::new(fin[i].to_string()).font(crate::fonts::sans_bold_font(15.0)).color(lc)));
                });
            }
            ui.add_space(8.0);
            // quick spreads
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.label(caps("QUICK SPREADS"));
                let atk = if base[1] >= base[3] { 1 } else { 3 };
                let spreads: [(&str, [u16; 6]); 4] = [
                    ("Fast attacker", ev6(&[(atk, 252), (5, 252), (0, 4)])),
                    ("Bulky attacker", ev6(&[(0, 252), (atk, 252), (4, 4)])),
                    ("Physical wall", ev6(&[(0, 252), (2, 252), (4, 4)])),
                    ("Special wall", ev6(&[(0, 252), (4, 252), (2, 4)])),
                ];
                for (n, ev) in spreads {
                    if chip(ui, n, m.evs == ev, ACCENT).clicked() {
                        m.evs = ev;
                    }
                }
            });
        });
    }

    fn team_sets_card(&mut self, ui: &mut egui::Ui, world: World, sp: &battle::Species, m: &mut Member) {
        let d = battle::data();
        let mut sets = d.sets_for(&sp.name);
        if sets.is_empty() && !sp.base.is_empty() {
            sets = d.sets_for(&sp.base);
        }
        if sets.is_empty() {
            return;
        }
        kit::card(ui, |ui| {
            ui.label(RichText::new("Sets to start from").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.label(small("Smogon's Platinum-era sets. Loading one replaces the moves, item, ability, nature and EVs."));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                for s in sets {
                    let on = m.moves.len() == s.m.len() && m.moves.iter().zip(&s.m).all(|(a, b)| a == b) && m.item == s.item;
                    let r = chip(ui, &s.n, on, ACCENT).on_hover_text(format!("{} @ {}\n{} · {}\n{}", sp.name, s.item, s.ab, s.nat, s.m.join(" / ")));
                    if r.clicked() {
                        m.moves = s.m.clone();
                        m.item = s.item.clone();
                        let abs = sp.abilities_in(world);
                        m.ability = if abs.contains(&s.ab) { s.ab.clone() } else { abs.first().cloned().unwrap_or_default() };
                        m.nature = s.nat.clone();
                        m.evs = s.ev;
                        m.ivs = s.iv;
                        if s.lv > 0 {
                            m.level = s.lv;
                        }
                    }
                }
            });
        });
    }

    // ───────────────────────────── pickers ─────────────────────────────

    fn team_picker(&mut self, ui: &mut egui::Ui, t: &crate::teams::Team, which: Picker) {
        let slot = self.setup.team.slot;
        let member = t.members.get(slot).cloned();
        let (title, hint) = match which {
            Picker::Species => (if member.is_some() { "Change Pokémon".to_string() } else { format!("Pick Pokémon {}", slot + 1) }, "Search a name, type or ability"),
            Picker::Item => (format!("Item for {}", member.as_ref().map(|m| m.display_name().to_string()).unwrap_or_default()), "Search an item"),
            Picker::Move(i) => (format!("Move {} for {}", i + 1, member.as_ref().map(|m| m.display_name().to_string()).unwrap_or_default()), "Search a move, type or how it's learned"),
        };
        let mut close = false;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&title).font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
                ui.add_space(10.0);
                let w = (ui.available_width() - 120.0).clamp(160.0, 360.0);
                let r = kit::search_box_keyed(ui, Some(egui::Id::new("team-pick-search")), &mut self.setup.team.query, hint, w);
                // focus the box when a picker opens
                let key = egui::Id::new("team-pick-last");
                let last = ui.ctx().data(|d| d.get_temp::<String>(key));
                let now = format!("{which:?}{slot}");
                if last.as_deref() != Some(now.as_str()) {
                    r.request_focus();
                    ui.ctx().data_mut(|d| d.insert_temp(key, now));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !t.members.is_empty() && soft(ui, "Cancel").clicked() {
                        close = true;
                    }
                    if let Picker::Move(_) = which {
                        kit::checkbox(ui, &mut self.setup.team.all_moves, "Show moves it can't learn");
                    }
                });
            });
            ui.add_space(10.0);
            match which {
                Picker::Species => self.pick_species(ui, t.world, member),
                Picker::Item => self.pick_item(ui, t.world, member),
                Picker::Move(i) => self.pick_move(ui, t.world, member, i),
            }
        });
        if close {
            self.setup.team.picker = None;
        }
    }

    fn pick_species(&mut self, ui: &mut egui::Ui, world: World, member: Option<Member>) {
        let q = self.setup.team.query.trim().to_lowercase();
        let wiki = self.wiki.clone();
        let mut list: Vec<&battle::Species> = pickable(world)
            .into_iter()
            .filter(|s| {
                q.is_empty()
                    || s.name.to_lowercase().contains(&q)
                    || s.types_in(world).iter().any(|t| t.to_lowercase() == q)
                    || s.abilities_in(world).iter().any(|a| a.to_lowercase().contains(&q))
                    || q.trim_start_matches('#').parse::<u32>().is_ok_and(|n| n == s.num)
            })
            .collect();
        // exact name first
        if let Some(i) = list.iter().position(|s| s.name.to_lowercase() == q) {
            let s = list.remove(i);
            list.insert(0, s);
        }
        if let Some((col, desc)) = self.setup.team.sort {
            list.sort_by(|a, b| {
                let (x, y) = if col < 6 { (a.base_stats(world)[col] as u32, b.base_stats(world)[col] as u32) } else { (a.bst(world), b.bst(world)) };
                if desc { y.cmp(&x) } else { x.cmp(&y) }
            });
        }
        let full = ui.available_width();
        let show_ab = full > 900.0;
        let stat_w = 40.0;
        let x_stats = full - 12.0 - stat_w * 7.0;
        // header with sortable stats
        let (hr, _) = ui.allocate_exact_size(egui::vec2(full - 12.0, 22.0), Sense::hover());
        {
            let p = ui.painter().clone();
            p.text(egui::pos2(hr.min.x + 60.0, hr.center().y), Align2::LEFT_CENTER, format!("{} POKÉMON", list.len()), crate::fonts::sans_bold_font(10.5), MUTED());
            p.text(egui::pos2(hr.min.x + 250.0, hr.center().y), Align2::LEFT_CENTER, "TYPES", crate::fonts::sans_bold_font(10.5), MUTED());
            if show_ab {
                p.text(egui::pos2(hr.min.x + 380.0, hr.center().y), Align2::LEFT_CENTER, "ABILITIES", crate::fonts::sans_bold_font(10.5), MUTED());
            }
            for (k, label) in STATS.iter().chain(["BST"].iter()).enumerate() {
                let r = Rect::from_min_size(egui::pos2(hr.min.x + x_stats + stat_w * k as f32, hr.min.y), egui::vec2(stat_w, 22.0));
                let resp = ui.interact(r, egui::Id::new(("team-sort", k)), Sense::click());
                let on = self.setup.team.sort.map(|s| s.0) == Some(k);
                let arrow = if on { if self.setup.team.sort.is_some_and(|s| s.1) { "▼" } else { "▲" } } else { "" };
                p.text(r.center(), Align2::CENTER_CENTER, format!("{}{}", label.to_uppercase(), arrow), crate::fonts::sans_bold_font(10.0), if on || resp.hovered() { TEXT() } else { MUTED() });
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Sort by this").clicked() {
                    self.setup.team.sort = match self.setup.team.sort {
                        Some((c, true)) if c == k => Some((k, false)),
                        Some((c, false)) if c == k => None,
                        _ => Some((k, true)),
                    };
                }
            }
        }
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let mut chosen: Option<String> = if enter && !q.is_empty() { list.first().map(|s| s.name.clone()) } else { None };
        let row_h = 46.0;
        egui::ScrollArea::vertical().id_salt("team-pick-species").max_height(560.0).auto_shrink([false, true]).show_rows(ui, row_h, list.len(), |ui, range| {
            for s in &list[range] {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(full - 12.0, row_h), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                let cur = member.as_ref().is_some_and(|m| battle::to_id(&m.species) == s.id);
                if cur {
                    p.rect_filled(r, 8.0, theme::blend(CARD(), ACCENT, 0.16));
                } else if ht > 0.0 {
                    p.rect_filled(r, 8.0, theme::blend(CARD(), CARD_HOVER(), ht));
                }
                let c = egui::pos2(r.min.x + 26.0, r.center().y);
                self.team_sprite(&p, &s.name, false, c, egui::vec2(40.0, 40.0));
                let f = crate::fonts::sans_bold_font(13.5);
                p.text(egui::pos2(r.min.x + 60.0, r.center().y - 7.0), Align2::LEFT_CENTER, super::charts::clip(ui, &s.name, &f, 180.0), f, TEXT());
                let note = match wiki.as_deref().zip(wiki_id(&s.name)) {
                    Some((w, id)) if w.unobtainable(&id).is_some() => "can't be caught yet",
                    _ if s.is_mega() => "Mega Evolution",
                    _ => "",
                };
                p.text(egui::pos2(r.min.x + 60.0, r.center().y + 10.0), Align2::LEFT_CENTER, format!("#{:03}  {note}", s.num), crate::fonts::sans_font(11.0), MUTED());
                let mut tx = r.min.x + 250.0;
                for t in s.types_in(world) {
                    tx += type_tag(&p, egui::pos2(tx, r.center().y), t, 9.0) + 4.0;
                }
                if show_ab {
                    let ab = s.abilities_in(world).join(" / ");
                    let f2 = crate::fonts::sans_font(11.5);
                    p.text(egui::pos2(r.min.x + 380.0, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &ab, &f2, x_stats - 390.0), f2, MUTED());
                }
                let bs = s.base_stats(world);
                for k in 0..7 {
                    let v = if k < 6 { bs[k] as u32 } else { s.bst(world) };
                    let col = if k < 6 { super::charts::ink(stat_color(bs[k])) } else { TEXT() };
                    p.text(egui::pos2(r.min.x + x_stats + stat_w * (k as f32 + 0.5), r.center().y), Align2::CENTER_CENTER, v.to_string(), crate::fonts::sans_font(12.0), col);
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    chosen = Some(s.name.clone());
                }
            }
        });
        if list.is_empty() {
            ui.label(small("Nothing matches. Try part of a name, a type (\"dragon\") or an ability."));
        }
        let Some(name) = chosen else { return };
        let slot = self.setup.team.slot;
        let wiki = self.wiki.clone();
        if let Some(t) = self.setup.team.team() {
            let world = t.world;
            match t.members.get_mut(slot) {
                Some(m) => {
                    // keep what still works
                    let old = m.clone();
                    let mut new = Member::new(&name, world);
                    new.level = old.level;
                    new.nickname = String::new();
                    new.item = if new.item.is_empty() { old.item.clone() } else { new.item };
                    new.nature = old.nature.clone();
                    new.evs = old.evs;
                    new.ivs = old.ivs;
                    new.shiny = old.shiny;
                    new.notes = old.notes.clone();
                    if let Some(w) = wiki.as_deref() {
                        let ways = learnable(w, &name, world);
                        new.moves = old.moves.iter().filter(|mv| mkey(w, mv).is_some_and(|k| ways.iter().any(|l| l.key == k))).cloned().collect();
                    }
                    *m = new;
                }
                None => {
                    if t.members.len() < 6 {
                        t.members.push(Member::new(&name, world));
                        self.setup.team.slot = t.members.len() - 1;
                    }
                }
            }
        }
        self.setup.team.touch();
        // straight on to the first move for a new Pokémon
        let slot = self.setup.team.slot;
        let fresh = self.setup.team.team().and_then(|t| t.members.get(slot)).is_some_and(|m| m.moves.is_empty());
        self.setup.team.picker = if fresh { Some(Picker::Move(0)) } else { None };
        self.setup.team.query.clear();
        self.setup.team.sort = None;
    }

    fn pick_item(&mut self, ui: &mut egui::Ui, world: World, member: Option<Member>) {
        let Some(m) = member else { return };
        let wiki = self.wiki.clone();
        let q = self.setup.team.query.trim().to_lowercase();
        let d = battle::data();
        let in_sets: Vec<String> = {
            let mut v: Vec<String> = d.sets_for(&m.species).iter().map(|s| s.item.clone()).collect();
            v.sort();
            v.dedup();
            v
        };
        let all = holdable(wiki.as_deref(), world);
        let mut list: Vec<(&battle::ItemInfo, bool)> = all
            .into_iter()
            .filter(|i| {
                // a Mega Stone only for its own Pokémon
                i.mega.is_empty() || battle::to_id(&i.mega).starts_with(&battle::to_id(&m.species))
            })
            .filter(|i| q.is_empty() || i.n.to_lowercase().contains(&q) || item_text(wiki.as_deref(), &i.n).is_some_and(|w| w.description.to_lowercase().contains(&q)))
            .map(|i| (i, in_sets.contains(&i.n)))
            .collect();
        list.sort_by(|a, b| (!a.1, a.0.n.clone()).cmp(&(!b.1, b.0.n.clone())));
        let full = ui.available_width();
        let mut chosen: Option<String> = None;
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !q.is_empty() {
            chosen = list.first().map(|i| i.0.n.clone());
        }
        if chip(ui, "No item", m.item.is_empty(), ACCENT).clicked() {
            chosen = Some(String::new());
        }
        ui.add_space(8.0);
        let row_h = 44.0;
        egui::ScrollArea::vertical().id_salt("team-pick-item").max_height(560.0).auto_shrink([false, true]).show_rows(ui, row_h, list.len(), |ui, range| {
            for (it, popular) in &list[range] {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(full - 12.0, row_h), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                if m.item == it.n {
                    p.rect_filled(r, 8.0, theme::blend(CARD(), ACCENT, 0.16));
                } else if ht > 0.0 {
                    p.rect_filled(r, 8.0, theme::blend(CARD(), CARD_HOVER(), ht));
                }
                let f = crate::fonts::sans_bold_font(13.0);
                p.text(egui::pos2(r.min.x + 14.0, r.center().y - 8.0), Align2::LEFT_CENTER, &it.n, f, TEXT());
                let wi = item_text(wiki.as_deref(), &it.n);
                let sub = match wi.and_then(|w| w.found.first()) {
                    Some(s) => format!("{}: {}", s.how, s.place),
                    None if !it.mega.is_empty() => "Mega Stone".into(),
                    None => String::new(),
                };
                let f2 = crate::fonts::sans_font(11.0);
                p.text(egui::pos2(r.min.x + 14.0, r.center().y + 9.0), Align2::LEFT_CENTER, super::charts::clip(ui, &sub, &f2, 220.0), f2.clone(), MUTED());
                let mut x = r.min.x + 250.0;
                if *popular {
                    let g = p.layout_no_wrap("IN SETS".into(), crate::fonts::sans_bold_font(9.0), super::charts::ink(ACCENT));
                    let br = Rect::from_min_size(egui::pos2(x, r.center().y - 8.0), egui::vec2(g.size().x + 10.0, 16.0));
                    p.rect_filled(br, 8.0, super::charts::faint(ACCENT, 0.2));
                    p.galley(br.min + egui::vec2(5.0, (16.0 - g.size().y) / 2.0), g, Color32::WHITE);
                    x = br.max.x + 10.0;
                } else {
                    x += 64.0;
                }
                if let Some(wi) = wi {
                    p.text(egui::pos2(x, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &wi.description, &f2, r.max.x - x - 10.0), f2, MUTED());
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    chosen = Some(it.n.clone());
                }
            }
        });
        let Some(name) = chosen else { return };
        let slot = self.setup.team.slot;
        if let Some(t) = self.setup.team.team() {
            if let Some(x) = t.members.get_mut(slot) {
                x.item = name;
            }
        }
        self.setup.team.touch();
        self.setup.team.picker = None;
        self.setup.team.query.clear();
    }

    fn pick_move(&mut self, ui: &mut egui::Ui, world: World, member: Option<Member>, idx: usize) {
        let Some(m) = member else { return };
        let Some(w) = self.wiki.clone() else {
            ui.label(small("Getting the game's data…"));
            return;
        };
        let q = self.setup.team.query.trim().to_lowercase();
        let learn = learnable(&w, &m.species, world);
        let ways = best_ways(&learn);
        let mut rows: Vec<(String, Option<&Learn>)> = ways.iter().map(|l| (l.key.clone(), Some(*l))).collect();
        if self.setup.team.all_moves {
            let mut extra: Vec<String> = w.data.moves.keys().filter(|k| !ways.iter().any(|l| &l.key == *k)).cloned().collect();
            extra.sort();
            rows.extend(extra.into_iter().map(|k| (k, None)));
        }
        let taken: Vec<String> = m.moves.iter().enumerate().filter(|(i, _)| *i != idx).map(|(_, n)| n.clone()).collect();
        rows.retain(|(k, l)| {
            let Some(info) = w.move_info(k) else { return false };
            if world == World::Platinum && info.t == "Fairy" && l.is_none() {
                return false;
            }
            q.is_empty()
                || info.n.to_lowercase().contains(&q)
                || info.t.to_lowercase() == q
                || info.c.to_lowercase() == q
                || l.is_some_and(|l| l.how.label().to_lowercase().contains(&q))
        });
        // strongest first within each kind: attacks by power, then status
        rows.sort_by(|a, b| {
            let ia = w.move_info(&a.0);
            let ib = w.move_info(&b.0);
            let pa = ia.and_then(|i| i.p.parse::<u32>().ok()).unwrap_or(0);
            let pb = ib.and_then(|i| i.p.parse::<u32>().ok()).unwrap_or(0);
            (a.1.is_none(), pa == 0, std::cmp::Reverse(pa), ia.map(|i| i.n.clone())).cmp(&(b.1.is_none(), pb == 0, std::cmp::Reverse(pb), ib.map(|i| i.n.clone())))
        });
        if let Some(i) = rows.iter().position(|(k, _)| w.move_info(k).is_some_and(|i| i.n.to_lowercase() == q)) {
            let r = rows.remove(i);
            rows.insert(0, r);
        }
        let stab: Vec<String> = battle::data().species(&m.species).map(|s| s.types_in(world).to_vec()).unwrap_or_default();
        let full = ui.available_width();
        let show_desc = full > 940.0;
        let mut chosen: Option<String> = None;
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !q.is_empty() {
            chosen = rows.iter().find(|(k, _)| w.move_info(k).is_some_and(|i| !taken.contains(&i.n))).and_then(|(k, _)| w.move_info(k).map(|i| i.n.clone()));
        }
        // header
        let (hr, _) = ui.allocate_exact_size(egui::vec2(full - 12.0, 20.0), Sense::hover());
        {
            let p = ui.painter();
            let f = crate::fonts::sans_bold_font(10.5);
            p.text(egui::pos2(hr.min.x + 14.0, hr.center().y), Align2::LEFT_CENTER, format!("{} MOVES", rows.len()), f.clone(), MUTED());
            p.text(egui::pos2(hr.min.x + 200.0, hr.center().y), Align2::LEFT_CENTER, "TYPE", f.clone(), MUTED());
            p.text(egui::pos2(hr.min.x + 330.0, hr.center().y), Align2::LEFT_CENTER, "POW", f.clone(), MUTED());
            p.text(egui::pos2(hr.min.x + 376.0, hr.center().y), Align2::LEFT_CENTER, "ACC", f.clone(), MUTED());
            p.text(egui::pos2(hr.min.x + 420.0, hr.center().y), Align2::LEFT_CENTER, "PP", f.clone(), MUTED());
            p.text(egui::pos2(hr.min.x + 456.0, hr.center().y), Align2::LEFT_CENTER, "HOW", f, MUTED());
        }
        let row_h = 40.0;
        egui::ScrollArea::vertical().id_salt(("team-pick-move", idx)).max_height(560.0).auto_shrink([false, true]).show_rows(ui, row_h, rows.len(), |ui, range| {
            for (k, l) in &rows[range] {
                let Some(info) = w.move_info(k) else { continue };
                let (r, resp) = ui.allocate_exact_size(egui::vec2(full - 12.0, row_h), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter().clone();
                let have = taken.contains(&info.n);
                let cur = m.moves.get(idx).is_some_and(|x| *x == info.n);
                if cur {
                    p.rect_filled(r, 8.0, theme::blend(CARD(), ACCENT, 0.16));
                } else if ht > 0.0 && !have {
                    p.rect_filled(r, 8.0, theme::blend(CARD(), CARD_HOVER(), ht));
                }
                let dim = if have || l.is_none() { 0.45 } else { 0.0 };
                let tc = theme::blend(TEXT(), CARD(), dim);
                let f = crate::fonts::sans_bold_font(13.0);
                let is_stab = info.c != "Status" && stab.contains(&info.t);
                p.text(egui::pos2(r.min.x + 14.0, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &info.n, &f, 170.0), f, tc);
                let mut x = r.min.x + 200.0;
                x += type_tag(&p, egui::pos2(x, r.center().y), &info.t, 8.5) + 4.0;
                cat_tag(&p, egui::pos2(x, r.center().y), &info.c);
                let f2 = crate::fonts::sans_font(12.0);
                let pw = if info.p.is_empty() || info.p == "0" { "—".to_string() } else if info.p == "1" { "var".into() } else { info.p.clone() };
                p.text(egui::pos2(r.min.x + 330.0, r.center().y), Align2::LEFT_CENTER, pw, if is_stab { crate::fonts::sans_bold_font(12.0) } else { f2.clone() }, tc);
                let ac = if info.a.is_empty() || info.a == "0" { "—".to_string() } else { info.a.clone() };
                p.text(egui::pos2(r.min.x + 376.0, r.center().y), Align2::LEFT_CENTER, ac, f2.clone(), MUTED());
                p.text(egui::pos2(r.min.x + 420.0, r.center().y), Align2::LEFT_CENTER, info.pp.clone(), f2.clone(), MUTED());
                let (how, hc) = match l {
                    Some(l) => (format!("{}{}", l.how.label(), l.via.as_ref().map(|v| format!(" ({v})")).unwrap_or_default()), if l.modern { kit::C_PINK } else { ACCENT }),
                    None => ("can't learn".into(), kit::C_ORANGE),
                };
                let hf = crate::fonts::sans_bold_font(11.0);
                p.text(egui::pos2(r.min.x + 456.0, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &how, &hf, 150.0), hf, super::charts::ink(hc));
                if show_desc {
                    let f3 = crate::fonts::sans_font(11.0);
                    p.text(egui::pos2(r.min.x + 620.0, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &info.d, &f3, r.max.x - r.min.x - 630.0), f3, MUTED());
                }
                let resp = resp.on_hover_text(if have { "Already on this set".to_string() } else { info.d.clone() });
                if resp.clicked() && !have {
                    chosen = Some(info.n.clone());
                }
                if is_stab && ht > 0.0 {
                    // a hint for STAB moves
                }
            }
        });
        if rows.is_empty() {
            ui.label(small("Nothing matches. Try a name, a type (\"fire\"), \"physical\", or \"tm\"."));
        }
        let Some(name) = chosen else { return };
        let slot = self.setup.team.slot;
        let mut next: Option<usize> = None;
        if let Some(t) = self.setup.team.team() {
            if let Some(x) = t.members.get_mut(slot) {
                if idx < x.moves.len() {
                    x.moves[idx] = name;
                } else {
                    x.moves.push(name);
                }
                if x.moves.len() < 4 {
                    next = Some(x.moves.len());
                }
            }
        }
        self.setup.team.touch();
        self.setup.team.query.clear();
        // fill the next empty move straight away
        self.setup.team.picker = next.filter(|n| *n > idx).map(Picker::Move);
    }
}

fn ev6(v: &[(usize, u16)]) -> [u16; 6] {
    let mut out = [0u16; 6];
    for (i, n) in v {
        out[*i] = *n;
    }
    out
}

/// A box that looks like a field and opens a picker.
fn pick_button(ui: &mut egui::Ui, text: &str, w: f32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 28.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let p = ui.painter();
    p.rect_filled(r, 6.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
    p.rect_stroke(r, 6.0, Stroke::new(1.0f32, theme::blend(LINE(), ACCENT, 0.5 * ht)));
    let f = crate::fonts::sans_font(13.0);
    p.text(egui::pos2(r.min.x + 10.0, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, text, &f, w - 30.0), f, TEXT());
    p.text(egui::pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, "›", crate::fonts::sans_bold_font(14.0), MUTED());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The EV bar: drag or click to set (steps of 4), up to `room`.
fn ev_slider(ui: &mut egui::Ui, size: egui::Vec2, v: &mut u16, room: u16) -> bool {
    let (r, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let track = Rect::from_min_max(egui::pos2(r.min.x + 6.0, r.center().y - 3.0), egui::pos2(r.max.x - 6.0, r.center().y + 3.0));
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.dragged() || resp.clicked() {
            let f = ((pos.x - track.min.x) / track.width()).clamp(0.0, 1.0);
            let mut nv = ((f * 252.0 / 4.0).round() * 4.0) as u16;
            if nv == 252 || f > 0.985 {
                nv = 252;
            }
            let nv = nv.min(room.max(*v));
            if nv != *v {
                *v = nv;
                changed = true;
            }
        }
    }
    let p = ui.painter();
    p.rect_filled(track, 3.0, kit::sunken());
    // how far you could go
    let room_x = track.min.x + track.width() * (room.max(*v) as f32 / 252.0);
    p.rect_filled(Rect::from_min_max(track.min, egui::pos2(room_x, track.max.y)), 3.0, theme::blend(kit::sunken(), ACCENT, 0.12));
    let x = track.min.x + track.width() * (*v as f32 / 252.0);
    p.rect_filled(Rect::from_min_max(track.min, egui::pos2(x, track.max.y)), 3.0, ACCENT);
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() || resp.dragged(), kit::QUICK);
    p.circle_filled(egui::pos2(x, r.center().y), 6.0 + 1.5 * ht, if *v > 0 { ACCENT } else { theme::blend(LINE(), ACCENT, ht) });
    p.circle_stroke(egui::pos2(x, r.center().y), 6.0 + 1.5 * ht, Stroke::new(2.0f32, CARD()));
    resp.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
    changed
}

fn nat_btn(ui: &mut egui::Ui, t: &str, on: bool, col: Color32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let p = ui.painter();
    if on {
        p.rect_filled(r, 5.0, col);
    } else {
        p.rect_filled(r, 5.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
    }
    p.text(r.center(), Align2::CENTER_CENTER, t, crate::fonts::sans_bold_font(13.0), if on { Color32::WHITE } else { theme::blend(MUTED(), col, ht) });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// "Adamant (+Atk −SpA)".
pub(super) fn nature_label(name: &str) -> String {
    match battle::data().nature(name) {
        Some(n) if !n.neutral() => format!("{} (+{} −{})", n.name, STATS[n.plus], STATS[n.minus]),
        Some(n) => format!("{} (no change)", n.name),
        None if name.is_empty() => "Pick a nature".into(),
        None => name.to_string(),
    }
}

/// The nature that raises `plus` and lowers `minus`.
fn nature_for(plus: usize, minus: usize) -> String {
    battle::data()
        .natures
        .iter()
        .find(|n| !n.neutral() && n.plus == plus && n.minus == minus)
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "Hardy".into())
}

/// Base stat colour, red (low) to green to cyan (high), like Showdown.
pub(super) fn stat_color(b: u16) -> Color32 {
    let h = (b as f32 * 180.0 / 255.0).min(360.0);
    let c = egui::ecolor::Hsva::new(h / 360.0, 0.75, 0.85, 1.0);
    Color32::from(c)
}

/// IVs closest to these that give Hidden Power `ty` (fewest changed, 30/31 only).
pub(super) fn hp_ivs(ty: &str, cur: [u16; 6]) -> Option<[u16; 6]> {
    let mut best: Option<([u16; 6], u32)> = None;
    for mask in 0u32..64 {
        let mut ivs = [31u16; 6];
        for (k, iv) in ivs.iter_mut().enumerate() {
            if mask & (1 << k) != 0 {
                *iv = 30;
            }
        }
        // keep a 0-ish IV the set already had (Trick Room, special attackers)
        for k in 0..6 {
            if cur[k] < 30 {
                ivs[k] = if ivs[k] == 30 { cur[k] & !1 } else { cur[k] | 1 };
            }
        }
        if battle::hidden_power(ivs).0 != ty {
            continue;
        }
        let cost: u32 = (0..6).map(|k| (cur[k] as i32 - ivs[k] as i32).unsigned_abs()).sum::<u32>() * 10 + (0..6).map(|k| 31 - ivs[k] as u32).sum::<u32>();
        if best.is_none_or(|b| cost < b.1) {
            best = Some((ivs, cost));
        }
    }
    best.map(|b| b.0)
}

/// The edge of a rect as dashes.
fn dashed(r: Rect) -> Vec<egui::Pos2> {
    let mut pts = Vec::new();
    let dash = 6.0;
    let gap = 5.0;
    let edges = [(r.left_top(), r.right_top()), (r.right_top(), r.right_bottom()), (r.right_bottom(), r.left_bottom()), (r.left_bottom(), r.left_top())];
    for (a, b) in edges {
        let len = (b - a).length();
        let dir = (b - a) / len;
        let mut t = 10.0;
        while t + dash < len - 10.0 {
            pts.push(a + dir * t);
            pts.push(a + dir * (t + dash));
            t += dash + gap;
        }
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hp_ivs_hit_every_type() {
        for t in battle::HP_TYPES {
            let ivs = hp_ivs(t, [31; 6]).expect(t);
            assert_eq!(battle::hidden_power(ivs).0, t);
        }
        // keeps a 0 Speed
        let ivs = hp_ivs("Fire", [31, 31, 31, 31, 31, 0]).expect("fire");
        assert!(ivs[5] <= 1);
    }

    #[test]
    fn learns_through_prevolutions() {
        let w = crate::wiki::Wiki::parse(include_bytes!("../../assets/wiki.json")).expect("wiki");
        let l = learnable(&w, "Garchomp", World::Platinum);
        assert!(l.iter().any(|l| l.key == "earthquake"));
        assert!(l.iter().any(|l| l.key == "outrage"));
        // Gible's egg moves come along
        assert!(l.iter().any(|l| l.via.as_deref() == Some("Gible") && l.how == How::Egg));
        assert_eq!(mkey(&w, "Extreme Speed").as_deref(), Some("extreme_speed"));
        assert_eq!(mkey(&w, "High Jump Kick").as_deref(), mkey(&w, "Hi Jump Kick").as_deref());
        let r = learnable(&w, "Rotom-Wash", World::Platinum);
        assert!(r.iter().any(|l| l.key == "thunderbolt"));
    }
}
