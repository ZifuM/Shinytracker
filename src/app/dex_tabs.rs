//! The Pokédex entry's "Where to find" and "Moves" tabs, from the
//! Platinum Together Wiki data.

use super::pokedex::{card_title, type_chip};
use super::ui_kit::{self as kit, PillKind, LINE, MUTED, TEXT};
use super::App;
use crate::theme;
use crate::gamemap::Spots;
use crate::wiki::{Mon, Wiki, Wild};
use eframe::egui::{self, Color32, RichText, Sense};
use std::sync::Arc;

/// Which move list the Moves tab shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum MoveKind {
    #[default]
    Level,
    Tm,
    Tutor,
    Egg,
}

pub(super) fn method_label(m: &str) -> &'static str {
    match m {
        "grass" => "Grass",
        "radar" => "Poké Radar",
        "surf" => "Surfing",
        "old_rod" => "Old Rod",
        "good_rod" => "Good Rod",
        "super_rod" => "Super Rod",
        "cave" => "Cave",
        _ => "Other",
    }
}

fn method_order(m: &str) -> usize {
    ["grass", "cave", "radar", "surf", "old_rod", "good_rod", "super_rod"].iter().position(|x| *x == m).unwrap_or(9)
}

fn small(t: impl Into<String>) -> RichText {
    RichText::new(t.into()).size(kit::fs(11.0)).color(MUTED())
}

fn cell(t: impl Into<String>) -> RichText {
    RichText::new(t.into()).size(kit::fs(12.0)).color(TEXT())
}

fn head(t: &str) -> RichText {
    RichText::new(t).size(kit::fs(11.0)).strong().color(MUTED())
}

/// "MORE" / "MODERN" tag for spots that need a world option.
fn setting_tag(ui: &mut egui::Ui, setting: &str) {
    let (text, tip) = match setting {
        "more" => ("MORE", "Only with MORE POKéMON on in your world"),
        "modern" => ("MODERN", "Only with MODERN POKéMON on in your world"),
        _ => return,
    };
    kit::pill(ui, text, PillKind::Info).on_hover_text(tip);
}

/// The "Map" button on each place. True when clicked.
fn map_pin(ui: &mut egui::Ui, ready: bool) -> bool {
    ui.add_enabled(ready, egui::Button::new(RichText::new("Map").size(kit::fs(11.0))).small())
        .on_hover_text("Show this place on the Map, flashing")
        .on_disabled_hover_text("The Map needs Platinum Together installed on this PC")
        .clicked()
}

/// Wiki places a free-text place ("Route 201 (Prof. Rowan's briefcase…)") is in.
fn places_for_text(w: &Wiki, text: &str) -> Vec<String> {
    let t = text.trim();
    let mut best: Option<(usize, &str)> = None;
    for (k, p) in &w.data.places {
        if !p.name.is_empty() && t.starts_with(p.name.as_str()) && best.map_or(true, |(n, _)| p.name.len() > n) {
            best = Some((p.name.len(), k.as_str()));
        }
    }
    if let Some((_, k)) = best {
        return vec![k.to_string()];
    }
    w.data.places.iter().filter(|(_, p)| !p.group.is_empty() && t.starts_with(p.group.as_str())).map(|(k, _)| k.clone()).collect()
}

/// Is this a real place in Sinnoh (so it can go on the map), rather than
/// something like "RiftPad MAIL" or "any tall grass"?
fn on_map(w: &Wiki, place: &str) -> bool {
    let p = place.trim();
    !p.is_empty()
        && w.data.places.values().any(|x| {
            (!x.name.is_empty() && p.starts_with(x.name.as_str())) || (!x.group.is_empty() && p.starts_with(x.group.as_str()))
        })
}

fn rule(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, LINE());
    ui.add_space(6.0);
}

fn category_color(c: &str) -> Color32 {
    match c {
        "Physical" => Color32::from_rgb(232, 120, 84),
        "Special" => Color32::from_rgb(110, 150, 240),
        _ => MUTED(),
    }
}

fn shard_color(c: &str) -> Color32 {
    match c {
        "red" => Color32::from_rgb(232, 92, 84),
        "blue" => Color32::from_rgb(92, 140, 240),
        "yellow" => Color32::from_rgb(240, 200, 64),
        "green" => Color32::from_rgb(100, 196, 100),
        _ => MUTED(),
    }
}

impl App {
    // ───────────────────────────── where to find ─────────────────────────────

    pub(super) fn entry_where(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, m: &Mon) {
        let text = |k: &str| w.data.text.get(k).cloned().unwrap_or_default();

        let map_ready = self.gamemap.as_ref().is_some_and(|r| r.is_ok());
        let mut go_map: Option<(Vec<String>, String, Option<Spots>)> = None;

        if let Some(why) = w.unobtainable(&m.id) {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    kit::pill(ui, "Can't get yet", PillKind::Warn);
                });
                ui.add_space(4.0);
                ui.add(egui::Label::new(cell(why)).wrap());
            });
            ui.add_space(10.0);
        }

        // ── wild
        let mut wild: Vec<&Wild> = w.wild(&m.id);
        let extra_count = wild.iter().filter(|e| !e.setting.is_empty()).count();
        if self.setup.pokedex.hide_extra_spots {
            wild.retain(|e| e.setting.is_empty());
        }
        let place_name = |key: &str| w.place(key).map(|p| p.name.clone()).unwrap_or_else(|| key.replace('_', " "));
        wild.sort_by(|a, b| {
            (w.stage_index(&a.stage), place_name(&a.area), method_order(&a.method)).cmp(&(
                w.stage_index(&b.stage),
                place_name(&b.area),
                method_order(&b.method),
            ))
        });
        if !w.wild(&m.id).is_empty() {
            let mut hide = self.setup.pokedex.hide_extra_spots;
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    card_title(ui, "In the wild");
                    if extra_count > 0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            kit::checkbox(ui, &mut hide, "Hide MORE / MODERN spots")
                                .on_hover_text("Spots that only show up with MORE POKéMON or MODERN POKéMON on in your world");
                        });
                    }
                });
                if wild.is_empty() {
                    ui.label(small("Only in spots that need MORE POKéMON or MODERN POKéMON."));
                    return;
                }
                egui::ScrollArea::horizontal().id_salt("pdx-wild").show(ui, |ui| {
                    egui::Grid::new("pdx-wild-grid").striped(true).spacing([14.0, 8.0]).min_col_width(0.0).show(ui, |ui| {
                        for h in ["Place", "How", "Levels", "Morning", "Day", "Night", "Can get there", ""] {
                            ui.label(head(h));
                        }
                        ui.end_row();
                        let mut last_area = String::new();
                        for e in &wild {
                            let same = e.area == last_area;
                            last_area = e.area.clone();
                            ui.vertical(|ui| {
                                ui.set_min_width(140.0);
                                ui.set_max_width(220.0);
                                let name = if same { small(place_name(&e.area)) } else { cell(place_name(&e.area)).strong() };
                                ui.add(egui::Label::new(name).extend());
                                if !e.note.is_empty() {
                                    ui.add(egui::Label::new(small(&e.note)).wrap());
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label(cell(method_label(&e.method)));
                                setting_tag(ui, &e.setting);
                            });
                            ui.label(cell(if e.levels.is_empty() { "—".to_string() } else { format!("Lv. {}", e.levels) }));
                            let best = e.chance.morning.max(e.chance.day).max(e.chance.night);
                            for c in [e.chance.morning, e.chance.day, e.chance.night] {
                                let t = if c == 0 { "—".to_string() } else { format!("{c}%") };
                                let mut rt = cell(t);
                                if c == best && c > 0 && (e.chance.morning != e.chance.day || e.chance.day != e.chance.night) {
                                    rt = rt.strong().color(theme::LINK());
                                }
                                ui.label(rt);
                            }
                            ui.label(small(&e.stage));
                            if !same {
                                if map_pin(ui, map_ready) {
                                    go_map = Some((vec![e.area.clone()], place_name(&e.area), Some(Spots::for_method(&e.method))));
                                }
                            } else {
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
                });
                ui.add_space(4.0);
                let mut note = text("chances");
                if wild.iter().any(|e| e.method == "radar") {
                    note.push(' ');
                    note.push_str(&text("radar"));
                }
                if !note.trim().is_empty() {
                    ui.add(egui::Label::new(small(note.trim())).wrap());
                }
            });
            self.setup.pokedex.hide_extra_spots = hide;
            ui.add_space(10.0);
            self.catch_spots(ui, w, m, &wild);
        }
        self.rare_spots(ui, w, m);

        // ── gifts, eggs, swarms, trades…
        let other = w.other(&m.id);
        if !other.is_empty() {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                card_title(ui, "Other ways");
                for (i, o) in other.iter().enumerate() {
                    if i > 0 {
                        rule(ui);
                    }
                    ui.horizontal_wrapped(|ui| {
                        kit::pill(ui, if o.how.is_empty() { "Other" } else { o.how.as_str() }, PillKind::Info);
                        if o.works == Some(false) {
                            kit::pill(ui, "Doesn't work", PillKind::Bad).on_hover_text("Doesn't work in the game right now");
                        }
                        ui.label(cell(&o.place).strong());
                        if !o.level.is_empty() {
                            ui.label(small(format!("Lv. {}", o.level)));
                        }
                        if on_map(w, &o.place) && map_pin(ui, map_ready) {
                            go_map = Some((places_for_text(w, &o.place), o.place.clone(), None));
                        }
                    });
                    if !o.detail.is_empty() {
                        ui.add(egui::Label::new(cell(&o.detail)).wrap());
                    }
                    egui::Grid::new(("pdx-other", i)).num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
                        for (k, v) in [("When", &o.when), ("Needs", &o.needs)] {
                            if !v.is_empty() {
                                ui.label(small(k));
                                ui.add(egui::Label::new(small(v.as_str()).color(TEXT())).wrap());
                                ui.end_row();
                            }
                        }
                    });
                }
            });
            ui.add_space(10.0);
        }

        // ── nothing in the wild or otherwise: evolve it
        if w.wild(&m.id).is_empty() && other.is_empty() {
            let mut go: Option<String> = None;
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                card_title(ui, "How to get it");
                match w.evolves_from(&m.id) {
                    Some(e) => {
                        let from = w.mon(&e.from).map(|x| x.name.clone()).unwrap_or_else(|| e.from.clone());
                        ui.horizontal_wrapped(|ui| {
                            ui.label(cell(format!("Evolve it from {from}.")));
                            if ui.link(RichText::new(format!("Where to find {from}")).size(kit::fs(12.0))).clicked() {
                                go = Some(e.from.clone());
                            }
                        });
                        ui.add(egui::Label::new(small(&e.method)).wrap());
                    }
                    None if w.unobtainable(&m.id).is_none() => {
                        ui.label(small("The wiki doesn't list anywhere to get it."));
                    }
                    None => {}
                }
            });
            if let Some(id) = go {
                self.setup.pokedex.open = Some(id);
            }
        }
        if let Some((places, title, spots)) = go_map {
            self.show_on_map(&places, title, spots);
        }
    }

    // ───────────────────────────── moves ─────────────────────────────

    pub(super) fn entry_moves(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, m: &Mon) {
        let Some(ls) = w.learnset(&m.id).cloned() else {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(small("The wiki doesn't have a move list for this one yet."));
            });
            return;
        };
        let has_modern = !ls.mlv.is_empty() || !ls.mtm.is_empty() || !ls.mtutor.is_empty() || !ls.megg.is_empty();
        let modern_default = m.modern || ls.lv.is_empty();
        let mut modern = self.setup.pokedex.modern_moves.unwrap_or(modern_default) && has_modern;
        if ls.lv.is_empty() && !ls.mlv.is_empty() {
            modern = true;
        }

        let level: Vec<(String, String, bool)> = if modern && !ls.mlv.is_empty() {
            ls.mlv.iter().map(|l| (l.level.clone(), l.key.clone(), false)).collect()
        } else {
            ls.lv.iter().map(|l| (l.level.clone(), l.key.clone(), false)).collect()
        };
        // TMs, tutors and egg moves: the Platinum list, plus the MODERN extras when that's on.
        let with_extra = |base: &Vec<String>, extra: &Vec<String>| -> Vec<(String, bool)> {
            let mut v: Vec<(String, bool)> = base.iter().map(|k| (k.clone(), false)).collect();
            if modern {
                v.extend(extra.iter().filter(|k| !base.contains(k)).map(|k| (k.clone(), true)));
            }
            v
        };
        let tms = with_extra(&ls.tm, &ls.mtm);
        let tutors = with_extra(&ls.tutor, &ls.mtutor);
        let eggs = with_extra(&ls.egg, &ls.megg);

        // ── controls
        let st = &mut self.setup.pokedex;
        ui.horizontal_wrapped(|ui| {
            kit::segmented(
                ui,
                &mut st.move_kind,
                &[
                    (MoveKind::Level, &format!("  Level up ({})  ", level.len())),
                    (MoveKind::Tm, &format!("  TMs & HMs ({})  ", tms.len())),
                    (MoveKind::Tutor, &format!("  Tutors ({})  ", tutors.len())),
                    (MoveKind::Egg, &format!("  Egg moves ({})  ", eggs.len())),
                ],
            );
            if has_modern && !(ls.lv.is_empty() && !ls.mlv.is_empty()) {
                ui.add_space(8.0);
                let mut on = modern;
                kit::checkbox(ui, &mut on, "MODERN POKéMON on")
                    .on_hover_text(w.data.text.get("modernLevel").cloned().unwrap_or_default());
                if on != modern {
                    st.modern_moves = Some(on);
                }
            }
        });
        ui.add_space(8.0);

        let kind = self.setup.pokedex.move_kind;
        let text = |k: &str| w.data.text.get(k).cloned().unwrap_or_default();
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            let (rows, first_head, note): (Vec<(String, String, bool)>, &str, String) = match kind {
                MoveKind::Level => (level.clone(), "Lv.", if modern { text("modernLevel") } else { String::new() }),
                MoveKind::Tm => (
                    tms.iter()
                        .map(|(tm, extra)| (tm.clone(), w.data.tms.get(tm).cloned().unwrap_or_else(|| tm.to_lowercase()), *extra))
                        .collect(),
                    "TM",
                    text("tm"),
                ),
                MoveKind::Tutor => (tutors.iter().map(|(k, x)| (String::new(), k.clone(), *x)).collect(), "Tutor", text("tutor")),
                MoveKind::Egg => (eggs.iter().map(|(k, x)| (String::new(), k.clone(), *x)).collect(), "", text("egg")),
            };
            if rows.is_empty() {
                ui.label(small("None."));
                return;
            }
            egui::ScrollArea::horizontal().id_salt("pdx-moves").show(ui, |ui| {
                egui::Grid::new(("pdx-moves-grid", kind as u8)).striped(true).spacing([14.0, 7.0]).min_col_width(0.0).show(ui, |ui| {
                    let mut heads: Vec<&str> = vec!["Move", "Type", "Kind", "Power", "Acc.", "PP"];
                    match kind {
                        MoveKind::Tutor => heads.extend(["Where", "Shards"]),
                        MoveKind::Egg => {}
                        _ => heads.insert(0, first_head),
                    }
                    for h in heads {
                        ui.label(head(h));
                    }
                    ui.end_row();
                    for (lead, key, extra) in &rows {
                        let mv = w.move_info(key).cloned().unwrap_or_default();
                        let name = if mv.n.is_empty() { key.replace('_', " ") } else { mv.n.clone() };
                        if matches!(kind, MoveKind::Level | MoveKind::Tm) {
                            ui.label(cell(lead));
                        }
                        // Bold when it's the Pokémon's own type (hits harder).
                        let stab = m.types.contains(&mv.t);
                        ui.horizontal(|ui| {
                            let mut rt = cell(&name);
                            if stab {
                                rt = rt.strong();
                            }
                            let r = ui.label(rt);
                            if !mv.d.is_empty() {
                                r.on_hover_text(&mv.d);
                            }
                            if *extra {
                                kit::pill(ui, "MODERN", PillKind::Info).on_hover_text("Only with MODERN POKéMON on");
                            }
                        });
                        if mv.t.is_empty() {
                            ui.label("");
                        } else {
                            type_chip(ui, &mv.t, "");
                        }
                        ui.label(RichText::new(&mv.c).size(kit::fs(11.5)).color(category_color(&mv.c)));
                        let dash = |s: &str| if s.is_empty() || s == "0" { "—".to_string() } else { s.to_string() };
                        ui.label(cell(dash(&mv.p)));
                        ui.label(cell(dash(&mv.a)));
                        ui.label(cell(dash(&mv.pp)));
                        if kind == MoveKind::Tutor {
                            let tutor = w.data.tutors.get(key);
                            ui.label(small(tutor.map(|t| t.place.clone()).unwrap_or_default()));
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                if let Some(t) = tutor {
                                    for (colour, n) in &t.cost {
                                        let (r, _) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), Sense::hover());
                                        ui.painter().circle_filled(r.center(), 4.0, shard_color(colour));
                                        ui.label(cell(n.to_string())).on_hover_text(format!("{n} {colour} shards"));
                                        ui.add_space(4.0);
                                    }
                                }
                            });
                        }
                        ui.end_row();
                    }
                });
            });
            ui.add_space(4.0);
            let mut foot = text("sameType");
            if !note.is_empty() {
                foot = format!("{note} {foot}");
            }
            if rows.iter().any(|r| r.2) {
                foot.push(' ');
                foot.push_str(&text("modernMore"));
            }
            foot.push_str(" Hover a move to see what it does.");
            ui.add(egui::Label::new(small(foot.trim())).wrap());
        });
    }
}
