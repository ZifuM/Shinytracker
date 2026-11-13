//! Dashboard widgets for the Pokédex, the Map and Game Info.

use super::info_home::Page;
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::w_kit::{self as wk, head, note, row};
use super::{App, Tab};
use crate::apps::WidgetCfg;
use crate::battle::World;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Rect, RichText, Sense, Stroke};

/// Starter-ish examples for a widget's preview in the gallery.
const SAMPLE_MONS: [&str; 3] = ["TURTWIG", "CHIMCHAR", "PIPLUP"];

impl App {
    // ───────────────────────────── Pokédex ─────────────────────────────

    /// A different Pokémon each day.
    pub(super) fn w_motd(&mut self, ui: &mut egui::Ui) {
        let Some(w) = self.wiki.clone() else { return loading(ui, "📖", "Pokémon of the day") };
        let day = crate::stats::local_day(crate::config::now_unix());
        let all: Vec<&crate::wiki::Mon> = w.all().filter(|m| w.unobtainable(&m.id).is_none()).collect();
        if all.is_empty() {
            return;
        }
        // the same one all day, a different one tomorrow
        let k = (day.unsigned_abs().wrapping_mul(2_654_435_761) >> 7) as usize % all.len();
        let m = all[k].clone();
        let wild = w.wild(&m.id);
        let place = wild.iter().find(|x| x.setting.is_empty()).or(wild.first()).and_then(|x| w.place(&x.area)).map(|p| p.name.clone());
        let how = place.map(|p| format!("Found on {p}")).or_else(|| w.other(&m.id).first().map(|o| format!("{} ({})", o.how, o.place))).unwrap_or_else(|| "Not in the wild".into());
        let mut open = false;
        kit::card(ui, |ui| {
            open = head(ui, "📖", kit::C_RED, "Pokémon of the day", Some("Open ›"));
            ui.add_space(6.0);
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 110.0), Sense::click());
            let p = ui.painter().clone();
            let col = m.types.first().map(|t| super::pokedex::type_color(t)).unwrap_or(kit::C_RED);
            p.rect_filled(r, 14.0, theme::blend(CARD(), col, 0.12));
            self.team_sprite(&p, &m.id, false, pos2(r.min.x + 56.0, r.center().y), vec2(96.0, 96.0));
            let x = r.min.x + 112.0;
            p.text(pos2(x, r.min.y + 22.0), Align2::LEFT_CENTER, format!("#{:03}", m.dex), crate::fonts::sans_font(11.5), MUTED());
            p.text(pos2(x, r.min.y + 42.0), Align2::LEFT_CENTER, &m.name, crate::fonts::sans_bold_font(17.0), TEXT());
            let mut tx = x;
            for t in &m.types {
                tx = wk::type_tag(&p, pos2(tx, r.min.y + 66.0), t, "").max.x + 5.0;
            }
            p.with_clip_rect(r).text(pos2(x, r.min.y + 90.0), Align2::LEFT_CENTER, &how, crate::fonts::sans_font(11.5), MUTED());
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                open = true;
            }
        });
        if open {
            self.open_dex_entry(&m.id);
        }
    }

    /// Look a Pokémon up right here.
    pub(super) fn w_dexsearch(&mut self, ui: &mut egui::Ui, w: &WidgetCfg) {
        let Some(wiki) = self.wiki.clone() else { return loading(ui, "🔍", "Pokédex search") };
        let mut open: Option<String> = None;
        let mut q = self.wtext(w.id, "q").clone();
        kit::card(ui, |ui| {
            head(ui, "🔍", kit::C_RED, "Pokédex search", None);
            ui.add_space(8.0);
            kit::search_box_keyed(ui, Some(egui::Id::new(("w-dexsearch", w.id))), &mut q, "🔍  A name or a dex number", ui.available_width().min(420.0));
            ui.add_space(10.0);
            let ql = q.trim().to_lowercase();
            let num: Option<u32> = ql.trim_start_matches('#').parse().ok();
            let avail = ui.available_width();
            let cols = ((avail + 10.0) / (150.0 + 10.0)).floor().clamp(1.0, 8.0) as usize;
            let hits: Vec<(String, String, u32, Vec<String>)> = if ql.is_empty() {
                // nothing typed: one row from the start of the dex
                wiki.all().take(cols).map(|m| (m.id.clone(), m.name.clone(), m.dex, m.types.clone())).collect()
            } else {
                let mut v: Vec<_> = wiki.all().filter(|m| num.map_or_else(|| m.name.to_lowercase().contains(&ql), |d| m.dex == d)).map(|m| (m.id.clone(), m.name.clone(), m.dex, m.types.clone())).collect();
                v.sort_by_key(|h| (!h.1.to_lowercase().starts_with(&ql), h.1.len()));
                v.truncate(8);
                v
            };
            if hits.is_empty() {
                note(ui, "No Pokémon by that name.");
                return;
            }
            let tw = ((avail - 10.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            for chunk in hits.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for (id, name, dex, types) in chunk {
                        let (r, resp) = ui.allocate_exact_size(vec2(tw, 112.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter().clone();
                        let col = types.first().map(|t| super::pokedex::type_color(t)).unwrap_or(kit::C_RED);
                        p.rect_filled(r, 12.0, theme::blend(CARD(), col, 0.07 + 0.08 * ht));
                        p.rect_stroke(r, 12.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.2 + 0.4 * ht)));
                        self.team_sprite(&p, id, false, pos2(r.center().x, r.min.y + 42.0), vec2(64.0, 64.0));
                        p.text(pos2(r.center().x, r.max.y - 30.0), Align2::CENTER_CENTER, name, crate::fonts::sans_bold_font(12.5), TEXT());
                        p.text(pos2(r.center().x, r.max.y - 13.0), Align2::CENTER_CENTER, format!("#{dex:03}"), crate::fonts::sans_font(11.0), MUTED());
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open = Some(id.clone());
                        }
                    }
                });
                ui.add_space(10.0);
            }
        });
        *self.wtext(w.id, "q") = q;
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
    }

    /// Pokémon you keep here, a click from their page.
    pub(super) fn w_favs(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let list: Vec<String> = if preview { SAMPLE_MONS.iter().map(|s| s.to_string()).collect() } else { w.data.clone() };
        let adding = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new(("w-favs-add", w.id)))).unwrap_or(false);
        let mut open: Option<String> = None;
        let mut remove: Option<usize> = None;
        let mut toggle = false;
        let mut picked: Option<String> = None;
        kit::card(ui, |ui| {
            toggle = head(ui, "❤", kit::C_PINK, "Favourite Pokémon", Some(if adding { "Done" } else { "+ Add" }));
            ui.add_space(8.0);
            if list.is_empty() && !adding {
                note(ui, "Keep the Pokémon you look at most here. Press “+ Add” to pick some.");
            }
            if !list.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    for (i, id) in list.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(vec2(84.0, 96.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter().clone();
                        p.rect_filled(r, 12.0, theme::blend(CARD(), kit::C_PINK, 0.06 + 0.08 * ht));
                        self.team_sprite(&p, id, false, pos2(r.center().x, r.min.y + 38.0), vec2(60.0, 60.0));
                        let name = self.mon_name(id);
                        p.with_clip_rect(r).text(pos2(r.center().x, r.max.y - 14.0), Align2::CENTER_CENTER, name, crate::fonts::sans_bold_font(11.5), TEXT());
                        let resp = resp.on_hover_text("Open its page · right-click to remove").on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            open = Some(id.clone());
                        }
                        resp.context_menu(|ui| {
                            if ui.button("Remove from favourites").clicked() {
                                remove = Some(i);
                                ui.close_menu();
                            }
                        });
                    }
                });
            }
            if adding {
                ui.add_space(8.0);
                picked = self.mon_picker(ui, w.id, "fav", "🔍  Find a Pokémon to add");
            }
        });
        if toggle && !preview {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("w-favs-add", w.id)), !adding));
        }
        if let Some(id) = picked {
            if !list.contains(&id) {
                let mut v = list.clone();
                v.push(id);
                *out = Some(v);
            }
        }
        if let Some(i) = remove {
            let mut v = list.clone();
            v.remove(i);
            *out = Some(v);
        }
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
    }

    /// What lives in a place you pick.
    pub(super) fn w_catchable(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let Some(wiki) = self.wiki.clone() else { return loading(ui, "🌿", "Catchable here") };
        let place = if preview { Some("Route 201".to_string()) } else { w.data.first().cloned() };
        let mut open: Option<String> = None;
        let mut change = false;
        let mut picked: Option<String> = None;
        kit::card(ui, |ui| {
            let title = place.as_deref().unwrap_or("Catchable here");
            change = head(ui, "🌿", kit::C_GREEN, title, place.as_ref().map(|_| "Change"));
            ui.add_space(6.0);
            let Some(place) = place.clone() else {
                note(ui, "Pick a place and the Pokémon there show up.");
                ui.add_space(6.0);
                picked = self.place_picker(ui, w.id, "place", "🔍  Route 210, Mt. Coronet…");
                return;
            };
            let keys: Vec<String> = super::info_places::groups(&wiki).into_iter().find(|g| g.name == place).map(|g| g.keys).unwrap_or_default();
            // each Pokémon once, with how you meet it
            let mut seen: Vec<(String, String, String)> = Vec::new();
            for x in wiki.data.wild.iter().filter(|x| keys.contains(&x.area) && x.setting.is_empty()) {
                if !seen.iter().any(|s| s.0 == x.species) {
                    seen.push((x.species.clone(), method_word(&x.method).to_string(), x.levels.clone()));
                }
            }
            if seen.is_empty() {
                note(ui, "No wild Pokémon here.");
            }
            for (id, how, lv) in seen.iter().take(6) {
                let (r, resp) = row(ui, 38.0);
                let p = ui.painter().clone();
                self.team_sprite(&p, id, false, pos2(r.min.x + 18.0, r.center().y), vec2(36.0, 36.0));
                p.text(pos2(r.min.x + 44.0, r.center().y), Align2::LEFT_CENTER, self.mon_name(id), crate::fonts::sans_bold_font(13.0), TEXT());
                p.text(pos2(r.max.x - 4.0, r.center().y), Align2::RIGHT_CENTER, format!("{how} · Lv {lv}"), crate::fonts::sans_font(11.5), MUTED());
                if resp.clicked() {
                    open = Some(id.clone());
                }
            }
            if seen.len() > 6 {
                note(ui, format!("and {} more", seen.len() - 6));
            }
        });
        if change && !preview {
            *out = Some(Vec::new());
        }
        if let Some(p) = picked {
            *out = Some(vec![p]);
        }
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
    }

    /// What a type hits hard, and what hits it hard.
    pub(super) fn w_typematch(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let world = World::Platinum;
        let types: Vec<&str> = crate::calc::types_in(world).to_vec();
        let cur = if preview { "Fire".to_string() } else { w.data.first().cloned().unwrap_or_else(|| "Water".into()) };
        let mut pick: Option<String> = None;
        kit::card(ui, |ui| {
            head(ui, "⚔", kit::C_ORANGE, "Type matchup", None);
            ui.add_space(6.0);
            // the type to look at: a row of small coloured dots
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for t in &types {
                    let on = *t == cur;
                    let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                    let c = super::pokedex::type_color(t);
                    ui.painter().circle_filled(r.center(), if on { 10.0 } else { 7.0 }, c);
                    if on {
                        ui.painter().circle_stroke(r.center(), 10.5, Stroke::new(2.0f32, TEXT()));
                    }
                    if resp.on_hover_text(*t).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        pick = Some(t.to_string());
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                super::pokedex::type_chip(ui, &cur, "");
                ui.label(RichText::new("attacking and defending").size(kit::fs(12.0)).color(MUTED()));
            });
            ui.add_space(6.0);
            let me = [cur.clone()];
            let hits: Vec<(String, String)> = types.iter().filter(|d| crate::calc::eff(&cur, &[d.to_string()], world) > 1.0).map(|d| (d.to_string(), String::new())).collect();
            let weak: Vec<(String, String)> = types.iter().filter(|a| crate::calc::eff(a, &me, world) > 1.0).map(|a| (a.to_string(), String::new())).collect();
            let resist: Vec<(String, String)> = types.iter().filter(|a| {
                let e = crate::calc::eff(a, &me, world);
                e < 1.0
            }).map(|a| (a.to_string(), if crate::calc::eff(a, &me, world) == 0.0 { "0×".into() } else { String::new() })).collect();
            for (label, list) in [("Hits hard", hits), ("Weak to", weak), ("Takes little from", resist)] {
                ui.label(RichText::new(label).font(crate::fonts::sans_bold_font(12.0)).color(TEXT()));
                if list.is_empty() {
                    note(ui, "Nothing");
                } else {
                    wk::type_row(ui, &list);
                }
                ui.add_space(4.0);
            }
        });
        if let Some(t) = pick {
            if !preview {
                *out = Some(vec![t]);
            }
        }
    }

    // ───────────────────────────── Map ─────────────────────────────

    /// Sinnoh at a glance, today's rare grass marked; click for the Map.
    pub(super) fn w_minimap(&mut self, ui: &mut egui::Ui) {
        let _ = self.map_data();
        let map = self.game_map();
        let mut go = false;
        let spots = self.rare_spots_today();
        kit::card(ui, |ui| {
            let link = if spots.is_empty() { "Open ›".to_string() } else { format!("{} rare grass today ›", spots.len()) };
            go = head(ui, "🗺", kit::C_GREEN, "Mini map", Some(&link));
            ui.add_space(8.0);
            let Some(map) = map else {
                note(ui, "The map is drawn from your game's map files. It shows here once they're read.");
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
                return;
            };
            let Some((tex, size)) = self.mapui.overview_loaded(ui.ctx(), &map) else {
                note(ui, "Open the Map once and the mini map shows here.");
                return;
            };
            let w = ui.available_width();
            let h = (w * size.y / size.x).min(260.0);
            let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
            let p = ui.painter().clone();
            p.rect_filled(r, 12.0, crate::gamemap::OCEAN);
            // fit the whole map in, centred
            let s = (r.width() / size.x).min(r.height() / size.y);
            let img = Rect::from_center_size(r.center(), size * s);
            p.with_clip_rect(r).image(tex, img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), egui::Color32::WHITE);
            // today's rare grass, outdoors
            let t = ui.input(|i| i.time) as f32;
            for sp in spots.iter().filter(|s| s.matrix == 0) {
                let c = img.min + vec2(sp.x as f32 + 0.5, sp.y as f32 + 0.5) * s;
                let pulse = 0.5 + 0.5 * (t * 3.0).sin();
                p.circle_filled(c, 3.5 + 2.0 * pulse, theme::GOLD.gamma_multiply(0.35));
                p.circle_filled(c, 3.0, theme::GOLD);
            }
            if !spots.is_empty() {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
            }
            p.rect_stroke(r, 12.0, Stroke::new(1.0f32, LINE()));
            if resp.on_hover_text("Open the Map").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                go = true;
            }
        });
        if go {
            self.go_to(Tab::Map);
        }
    }

    /// Places you pick, with what lives there.
    pub(super) fn w_mapfavs(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let Some(wiki) = self.wiki.clone() else { return loading(ui, "📍", "Favourite places") };
        let list: Vec<String> = if preview { vec!["Route 201".into(), "Lake Verity".into(), "Eterna Forest".into()] } else { w.data.clone() };
        let adding = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new(("w-mapfavs-add", w.id)))).unwrap_or(false);
        let groups = super::info_places::groups(&wiki);
        let mut toggle = false;
        let mut picked: Option<String> = None;
        let mut show: Option<(Vec<String>, String)> = None;
        let mut remove: Option<usize> = None;
        kit::card(ui, |ui| {
            toggle = head(ui, "📍", kit::C_GREEN, "Favourite places", Some(if adding { "Done" } else { "+ Add" }));
            ui.add_space(6.0);
            if list.is_empty() && !adding {
                note(ui, "Keep the places you go to most here. Press “+ Add” to pick some.");
            }
            for (i, name) in list.iter().enumerate() {
                let keys = groups.iter().find(|g| &g.name == name).map(|g| g.keys.clone()).unwrap_or_default();
                let mons: Vec<&String> = {
                    let mut v: Vec<&String> = Vec::new();
                    for x in wiki.data.wild.iter().filter(|x| keys.contains(&x.area) && x.setting.is_empty()) {
                        if !v.contains(&&x.species) {
                            v.push(&x.species);
                        }
                    }
                    v
                };
                let (r, resp) = row(ui, 42.0);
                let p = ui.painter().clone();
                p.text(pos2(r.min.x + 4.0, r.center().y - 8.0), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(13.0), TEXT());
                p.text(pos2(r.min.x + 4.0, r.center().y + 10.0), Align2::LEFT_CENTER, format!("{} Pokémon", mons.len()), crate::fonts::sans_font(11.0), MUTED());
                let mut x = r.max.x - 18.0;
                for id in mons.iter().take(3) {
                    self.team_sprite(&p, id, false, pos2(x, r.center().y), vec2(32.0, 32.0));
                    x -= 30.0;
                }
                let resp = resp.on_hover_text("Show it on the Map · right-click to remove");
                if resp.clicked() {
                    show = Some((keys.clone(), name.clone()));
                }
                resp.context_menu(|ui| {
                    if ui.button("Remove").clicked() {
                        remove = Some(i);
                        ui.close_menu();
                    }
                });
            }
            if adding {
                ui.add_space(6.0);
                picked = self.place_picker(ui, w.id, "place", "🔍  Find a place to add");
            }
        });
        if toggle && !preview {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("w-mapfavs-add", w.id)), !adding));
        }
        if let Some(pl) = picked {
            if !list.contains(&pl) {
                let mut v = list.clone();
                v.push(pl);
                *out = Some(v);
            }
        }
        if let Some(i) = remove {
            let mut v = list.clone();
            v.remove(i);
            *out = Some(v);
        }
        if let Some((keys, name)) = show {
            self.show_on_map(&keys, name, None);
        }
    }

    /// Where your current target can be found.
    pub(super) fn w_huntwhere(&mut self, ui: &mut egui::Ui) {
        let Some(wiki) = self.wiki.clone() else { return loading(ui, "🔭", "Where's my hunt") };
        let idx = self.shown_target_idx(ui.ctx());
        let target = idx.and_then(|i| wiki.by_dex(self.species[i].dex)).cloned();
        let mut show: Option<(Vec<String>, String)> = None;
        kit::card(ui, |ui| {
            head(ui, "🔭", kit::C_TEAL, "Where's my hunt", None);
            ui.add_space(6.0);
            let Some(m) = target.clone() else {
                note(ui, "Pick a target in the Shiny Hunter and where to find it shows here.");
                return;
            };
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                let p = ui.painter().clone();
                self.team_sprite(&p, &m.id, true, r.center(), vec2(42.0, 42.0));
                ui.label(RichText::new(&m.name).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            });
            ui.add_space(4.0);
            let mut places: Vec<(String, String, String, u32)> = Vec::new();
            for x in wiki.wild(&m.id).iter().filter(|x| x.setting.is_empty() || self.world_has(&x.setting)) {
                let name = wiki.place(&x.area).map(|p| if p.group.is_empty() { p.name.clone() } else { p.group.clone() }).unwrap_or_else(|| x.area.clone());
                let best = x.chance.morning.max(x.chance.day).max(x.chance.night);
                if !places.iter().any(|p| p.0 == name && p.1 == x.method) {
                    places.push((name, x.method.clone(), x.area.clone(), best));
                }
            }
            places.sort_by(|a, b| b.3.cmp(&a.3));
            if places.is_empty() {
                let other = wiki.other(&m.id);
                match other.first() {
                    Some(o) => note(ui, format!("Not in the wild: {} ({}).", o.how, o.place)),
                    None => note(ui, "Not in the wild."),
                }
                return;
            }
            for (name, method, _, best) in places.iter().take(5) {
                let (r, resp) = row(ui, 30.0);
                let p = ui.painter();
                p.text(pos2(r.min.x + 4.0, r.center().y), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(12.5), TEXT());
                p.text(pos2(r.max.x - 4.0, r.center().y), Align2::RIGHT_CENTER, format!("{} · {best}%", method_word(method)), crate::fonts::sans_font(11.5), MUTED());
                if resp.on_hover_text("Show it on the Map").clicked() {
                    let keys = places.iter().filter(|p| &p.0 == name).map(|p| p.2.clone()).collect();
                    show = Some((keys, m.name.clone()));
                }
            }
            if places.len() > 5 && wk::link(ui, &format!("All {} places on the Map ›", places.len())) {
                show = Some((places.iter().map(|p| p.2.clone()).collect(), m.name.clone()));
            }
        });
        if let Some((keys, name)) = show {
            self.show_on_map(&keys, name, None);
        }
    }

    /// The places you looked at last on the Map.
    pub(super) fn w_maprecent(&mut self, ui: &mut egui::Ui) {
        let recent = self.cfg.home.map_recent.clone();
        let map = self.game_map();
        let mut go: Option<u16> = None;
        kit::card(ui, |ui| {
            head(ui, "🕘", kit::C_GREEN, "Recent places", None);
            ui.add_space(6.0);
            let Some(map) = map.clone() else {
                note(ui, "The places you open on the Map show up here.");
                return;
            };
            if recent.is_empty() {
                note(ui, "The places you open on the Map show up here.");
            }
            for z in recent.iter().take(6) {
                let (r, resp) = row(ui, 30.0);
                let label = self.zone_label(&map, *z);
                let (kind, col, _) = super::map_panel::zone_kind(&map, *z);
                let p = ui.painter();
                p.circle_filled(pos2(r.min.x + 10.0, r.center().y), 4.0, col);
                p.text(pos2(r.min.x + 22.0, r.center().y), Align2::LEFT_CENTER, label, crate::fonts::sans_bold_font(12.5), TEXT());
                p.text(pos2(r.max.x - 4.0, r.center().y), Align2::RIGHT_CENTER, kind, crate::fonts::sans_font(11.0), MUTED());
                if resp.clicked() {
                    go = Some(*z);
                }
            }
        });
        if let (Some(z), Some(map)) = (go, map) {
            self.goto_zones(&map, &[z]);
        }
    }

    // ───────────────────────────── Game Info ─────────────────────────────

    /// The newest news from the game's wiki.
    pub(super) fn w_infonews(&mut self, ui: &mut egui::Ui) {
        let Some(w) = self.wiki.clone() else { return loading(ui, "📰", "Latest game news") };
        let mut read: Option<String> = None;
        let mut open = false;
        kit::card(ui, |ui| {
            open = head(ui, "📰", kit::C_VIOLET, "Latest game news", Some("Game Info ›"));
            ui.add_space(4.0);
            ui.label(RichText::new(format!("Game data v{}", w.data.version)).size(kit::fs(11.5)).color(MUTED()));
            ui.add_space(6.0);
            if w.data.home.notices.is_empty() {
                note(ui, "No news right now.");
            }
            for (i, n) in w.data.home.notices.iter().take(3).enumerate() {
                if i > 0 {
                    ui.add_space(6.0);
                    wk::divider(ui);
                    ui.add_space(6.0);
                }
                ui.label(RichText::new(n.title.trim_end_matches(['.', ':'])).font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
                ui.add(egui::Label::new(RichText::new(&n.text).size(kit::fs(12.5)).color(MUTED())).wrap());
                if let Some(g) = n.guide() {
                    let label = if n.label.is_empty() { "Read more".to_string() } else { n.label.clone() };
                    if wk::link(ui, &format!("{label}  ›")) {
                        read = Some(g.to_string());
                    }
                }
            }
        });
        if let Some(g) = read {
            self.go_to(Tab::Info);
            self.info_read(&g, None, None);
        } else if open {
            self.go_to(Tab::Info);
        }
    }

    /// A different how-to-play guide every day.
    pub(super) fn w_tip(&mut self, ui: &mut egui::Ui) {
        let Some(w) = self.wiki.clone() else { return loading(ui, "💡", "Tip of the day") };
        let guides = w.guides_sorted();
        if guides.is_empty() {
            return;
        }
        let day = crate::stats::local_day(crate::config::now_unix());
        let g = guides[day.rem_euclid(guides.len() as i64) as usize].clone();
        let mut read = false;
        kit::card(ui, |ui| {
            head(ui, "💡", kit::C_GOLD, "Tip of the day", None);
            ui.add_space(6.0);
            if !g.group.is_empty() {
                ui.label(RichText::new(g.group.to_uppercase()).font(crate::fonts::sans_bold_font(10.5)).color(theme::blend(MUTED(), kit::C_GOLD, 0.5)));
            }
            ui.label(RichText::new(&g.title).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.add_space(2.0);
            ui.add(egui::Label::new(RichText::new(&g.summary).size(kit::fs(12.5)).color(MUTED())).wrap());
            ui.add_space(8.0);
            read = wk::go_button(ui, "Read the guide  ›", false);
        });
        if read {
            self.go_to(Tab::Info);
            self.info_read(&g.id, None, None);
        }
    }

    /// Search the game: moves, items, places, guides (and Pokémon).
    pub(super) fn w_infosearch(&mut self, ui: &mut egui::Ui, w: &WidgetCfg) {
        let Some(wiki) = self.wiki.clone() else { return loading(ui, "🔎", "Quick lookup") };
        let mut q = self.wtext(w.id, "q").clone();
        let mut open: Option<(super::info_home::Kind, String)> = None;
        let hits = if q.trim().is_empty() { Vec::new() } else { self.info_quick(&wiki, &q, 6) };
        kit::card(ui, |ui| {
            head(ui, "🔎", kit::C_CYAN, "Quick lookup", None);
            ui.add_space(8.0);
            let resp = kit::search_box_keyed(ui, Some(egui::Id::new(("w-infosearch", w.id))), &mut q, "🔍  Surf, Leftovers, Route 210, saving…", ui.available_width().min(520.0));
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                open = hits.first().map(|h| (h.0, h.1.clone()));
            }
            ui.add_space(8.0);
            if q.trim().is_empty() {
                note(ui, "Type a move, an item, a place or a guide. Enter opens the first result.");
            } else if hits.is_empty() {
                note(ui, "Nothing found.");
            }
            for (kind, key, icon, col, title, sub) in &hits {
                let (r, resp) = row(ui, 36.0);
                let p = ui.painter();
                kit::icon_chip(p, Rect::from_center_size(pos2(r.min.x + 16.0, r.center().y), vec2(26.0, 26.0)), icon, *col, false);
                p.text(pos2(r.min.x + 38.0, r.center().y), Align2::LEFT_CENTER, title, crate::fonts::sans_bold_font(13.0), TEXT());
                p.with_clip_rect(r).text(pos2(r.max.x - 6.0, r.center().y), Align2::RIGHT_CENTER, sub, crate::fonts::sans_font(11.5), MUTED());
                if resp.clicked() {
                    open = Some((*kind, key.clone()));
                }
            }
        });
        *self.wtext(w.id, "q") = q;
        if let Some((k, key)) = open {
            self.info_quick_open(k, key);
        }
    }

    /// Where you are in the story (Game Info's own card).
    pub(super) fn w_adventure(&mut self, ui: &mut egui::Ui) {
        let Some(w) = self.wiki.clone() else { return loading(ui, "🏅", "Your adventure") };
        self.info_adventure(ui, &w);
    }

    /// Game Info pages you pinned, then the ones you looked at lately.
    pub(super) fn w_bookmarks(&mut self, ui: &mut egui::Ui) {
        let pins: Vec<Page> = self.cfg.info_pins.iter().filter_map(|k| Page::from_key(k)).collect();
        let recent: Vec<Page> = self.setup.info.recent.iter().filter(|p| !pins.contains(p)).take(4).cloned().collect();
        let mut open: Option<Page> = None;
        kit::card(ui, |ui| {
            head(ui, "🔖", kit::C_CYAN, "Pinned pages", None);
            ui.add_space(6.0);
            if pins.is_empty() && recent.is_empty() {
                note(ui, "Pin a move, item, place or guide in Game Info (📌 on its page) and it shows here.");
                return;
            }
            for (list, title) in [(&pins, "Pinned"), (&recent, "Looked at lately")] {
                if list.is_empty() {
                    continue;
                }
                ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(11.5)).color(MUTED()));
                for pg in list.iter().take(5) {
                    let (icon, col) = match pg {
                        Page::Move(_) => ("💥", kit::C_VIOLET),
                        Page::Item(_) => ("🎒", kit::C_GOLD),
                        Page::Place(_) => ("🗺", kit::C_GREEN),
                        Page::Guide(_) => ("📚", kit::C_CYAN),
                        _ => ("•", kit::C_SLATE),
                    };
                    let label = self.page_label(pg);
                    let (r, resp) = row(ui, 30.0);
                    let p = ui.painter();
                    p.text(pos2(r.min.x + 12.0, r.center().y), Align2::CENTER_CENTER, icon, egui::FontId::proportional(13.0), col);
                    p.text(pos2(r.min.x + 28.0, r.center().y), Align2::LEFT_CENTER, label, crate::fonts::sans_font(13.0), TEXT());
                    if resp.clicked() {
                        open = Some(pg.clone());
                    }
                }
                ui.add_space(4.0);
            }
        });
        if let Some(pg) = open {
            if let Page::Guide(id) = &pg {
                let id = id.clone();
                self.go_to(Tab::Info);
                self.info_read(&id, None, None);
            } else {
                self.info_open_page(pg);
            }
        }
    }
}

/// "grass", "surf"… in words.
pub(super) fn method_word(m: &str) -> &str {
    match m {
        "grass" => "Grass",
        "radar" => "Poké Radar",
        "surf" => "Surfing",
        "old_rod" => "Old Rod",
        "good_rod" => "Good Rod",
        "super_rod" => "Super Rod",
        "cave" => "Cave",
        _ => m,
    }
}

/// A widget waiting for the game's data.
pub(super) fn loading(ui: &mut egui::Ui, icon: &str, title: &str) {
    super::widgets::empty_card(ui, icon, title, "Getting the game's data… (it needs the internet the first time)");
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
}
