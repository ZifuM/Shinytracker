//! The Map's side panel (the area you picked, today's rare grass), its
//! minimap, the status line and the little drawn icons they share.

use super::charts::{self as ch, CYAN, GOLD, GREEN, ORANGE, PINK, PURPLE};
use super::map::{Area, Layer};
use super::rare::RareSpot;
use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::gamemap::{self, GameMap, CHUNK};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

/// The side panel's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum MapTab {
    Area,
    #[default]
    Rare,
}

/// What a kind of area looks like: icon, colour, name.
pub(super) fn zone_kind(map: &GameMap, z: u16) -> (&'static str, Color32, &'static str) {
    let Some(zone) = map.zone(z) else { return ("📍", CYAN, "Place") };
    if zone.matrix != 0 {
        if let Some(o) = map.overlays.iter().find(|o| o.matrix == zone.matrix) {
            return match o.kind.as_str() {
                "cave" => ("🗻", ORANGE, "Cave"),
                "ruins" => ("🏰", PURPLE, "Ruins"),
                _ => ("🌲", GREEN, "Side area"),
            };
        }
        return ("🚪", CYAN, "Indoors");
    }
    match zone.code.chars().next() {
        Some('C') => ("🏠", PINK, "City"),
        Some('T') => ("🏠", PINK, "Town"),
        Some('R') => ("🌿", GREEN, "Route"),
        Some('W') => ("🌊", CYAN, "Water route"),
        Some('L') => ("💧", CYAN, "Lake"),
        _ => ("📍", GOLD, "Landmark"),
    }
}

/// A chevron drawn with lines (the arrow glyphs aren't in every font).
pub(super) fn chevron(p: &egui::Painter, c: Pos2, dir: Vec2, size: f32, st: Stroke) {
    let side = vec2(-dir.y, dir.x);
    let tip = c + dir * size * 0.5;
    p.line_segment([c - dir * size * 0.5 + side * size, tip], st);
    p.line_segment([c - dir * size * 0.5 - side * size, tip], st);
}

/// A round icon button (drawn, so every one is the same size).
pub(super) fn round_button(ui: &mut egui::Ui, size: f32, tip: &str, draw: impl FnOnce(&egui::Painter, Rect, Color32)) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let t = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let p = ui.painter();
    if t > 0.0 {
        p.circle_filled(r.center(), size / 2.0, theme::blend(kit::CARD(), kit::CARD_HOVER(), t));
    }
    let ink = theme::blend(MUTED(), TEXT(), t);
    draw(p, r, ink);
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Small grey heading inside the panel.
fn head(ui: &mut egui::Ui, t: &str, n: Option<usize>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(t).font(ch::bold(10.5)).color(MUTED()));
        if let Some(n) = n {
            let g = ui.painter().layout_no_wrap(n.to_string(), ch::bold(10.0), MUTED());
            let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 12.0, 16.0), Sense::hover());
            ui.painter().rect_filled(r, 8.0, kit::CARD_HOVER());
            ui.painter().galley(r.center() - g.size() / 2.0, g, MUTED());
        }
    });
}

fn hairline(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, LINE()));
}

/// One wild Pokémon of an area.
struct WildMon {
    id: String,
    name: String,
    dex: u32,
    best: u32,
    times: [u32; 3],
    levels: String,
}

/// What the panel asks the map to do.
#[derive(Default)]
pub(super) struct PanelActs {
    pub go_inside: Option<u16>,
    pub zoom_to: Option<u16>,
    pub close: bool,
    pub collapse: bool,
}

impl App {
    /// The side panel: tabs, then the area you picked or today's rare grass.
    pub(super) fn map_panel(&mut self, ui: &mut egui::Ui, map: &GameMap, spots: &[RareSpot], left: i64, height: f32) -> PanelActs {
        let mut acts = PanelActs::default();
        ui.spacing_mut().item_spacing.y = 6.0;
        // tabs and the fold button
        ui.horizontal(|ui| {
            let mut tab = self.mapui.panel_tab;
            let rare_label = format!("Rare grass · {}", spots.len());
            kit::segmented(ui, &mut tab, &[(MapTab::Area, "Area"), (MapTab::Rare, rare_label.as_str())]);
            self.mapui.panel_tab = tab;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if round_button(ui, 32.0, "Hide the panel (more room for the map)", |p, r, ink| {
                    chevron(p, r.center() - vec2(3.0, 0.0), vec2(1.0, 0.0), 6.0, Stroke::new(1.8f32, ink));
                    chevron(p, r.center() + vec2(3.0, 0.0), vec2(1.0, 0.0), 6.0, Stroke::new(1.8f32, ink));
                }) {
                    acts.collapse = true;
                }
            });
        });
        ui.add_space(8.0);
        let body_h = (height - 60.0).max(120.0);
        egui::ScrollArea::vertical()
            .id_salt(("map-panel", self.mapui.panel_tab == MapTab::Rare))
            .max_height(body_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                match self.mapui.panel_tab {
                    MapTab::Area => match self.mapui.selected {
                        Some(z) => self.panel_area(ui, map, z, spots, &mut acts),
                        None => self.panel_area_empty(ui, map),
                    },
                    MapTab::Rare => self.panel_rare(ui, spots, left),
                }
                ui.add_space(12.0);
            });
        acts
    }

    // ───────────────────────────── area ─────────────────────────────

    fn panel_area(&mut self, ui: &mut egui::Ui, map: &GameMap, z: u16, spots: &[RareSpot], acts: &mut PanelActs) {
        let Some(zone) = map.zone(z).cloned() else { return };
        let label = self.zone_label(map, z);
        let (icon, col, kind) = zone_kind(map, z);
        let places = map.zone_places(z);
        let w = self.wiki.clone();
        let stage = w
            .as_ref()
            .and_then(|w| places.first().and_then(|p| w.place(p)).map(|pl| pl.stage.clone()))
            .filter(|s| !s.is_empty());
        let inside = (zone.matrix == 0).then(|| map.zones.iter().find(|o| o.matrix != 0 && o.name == zone.name).map(|o| o.id)).flatten();
        // header
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            kit::icon_chip(ui.painter(), r, icon, col, false);
            ui.add_space(4.0);
            let tw = (ui.available_width() - 36.0).max(120.0);
            ui.allocate_ui_with_layout(vec2(tw, 44.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(tw);
                ui.add(egui::Label::new(RichText::new(&label).font(ch::bold(17.0)).color(TEXT())).truncate()).on_hover_text(&label);
                ui.label(RichText::new(kind).size(12.0).color(MUTED()));
            });
            if round_button(ui, 28.0, "Close (Esc)", |p, r, ink| {
                let c = r.center();
                let st = Stroke::new(1.8f32, ink);
                p.line_segment([c + vec2(-4.5, -4.5), c + vec2(4.5, 4.5)], st);
                p.line_segment([c + vec2(4.5, -4.5), c + vec2(-4.5, 4.5)], st);
            }) {
                acts.close = true;
            }
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            if let Some(st) = &stage {
                chip(ui, &format!("Reach it: {st}"), CYAN);
            }
            let rare_here: Vec<&RareSpot> = spots.iter().filter(|s| s.zone == Some(z)).collect();
            if !rare_here.is_empty() {
                chip(ui, &format!("{} on rare grass today", rare_here.len()), GOLD);
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if let Some(first) = inside {
                if kit::accent_button(ui, egui::Button::new(RichText::new("Go inside").font(ch::bold(13.0)).color(kit::on_primary())), 6.0).clicked() {
                    acts.go_inside = Some(first);
                }
            }
            if ui.add(egui::Button::new("Zoom to it").fill(kit::btn_fill())).clicked() {
                acts.zoom_to = Some(z);
            }
        });

        // rare grass here today
        let rare_here: Vec<RareSpot> = spots.iter().filter(|s| s.zone == Some(z)).cloned().collect();
        if !rare_here.is_empty() {
            ui.add_space(12.0);
            hairline(ui);
            ui.add_space(10.0);
            head(ui, "RARE GRASS HERE TODAY", Some(rare_here.len()));
            ui.add_space(4.0);
            self.rare_rows(ui, &rare_here, false);
        }

        // wild Pokémon by how you meet them
        ui.add_space(12.0);
        hairline(ui);
        ui.add_space(10.0);
        let mut by_method: Vec<(String, Vec<WildMon>)> = Vec::new();
        if let Some(w) = &w {
            for e in &w.data.wild {
                if !places.contains(&e.area.as_str()) || !e.setting.is_empty() {
                    continue;
                }
                let how = super::dex_tabs::method_label(&e.method).to_string();
                let Some(mon) = w.mon(&e.species) else { continue };
                let times = [e.chance.morning, e.chance.day, e.chance.night];
                let best = times.into_iter().max().unwrap_or(0);
                let list = match by_method.iter_mut().position(|(m, _)| *m == how) {
                    Some(i) => &mut by_method[i].1,
                    None => {
                        by_method.push((how.clone(), Vec::new()));
                        &mut by_method.last_mut().unwrap().1
                    }
                };
                match list.iter_mut().find(|m| m.id == e.species) {
                    Some(m) => {
                        for k in 0..3 {
                            m.times[k] += times[k];
                        }
                        m.best = m.times.into_iter().max().unwrap_or(0);
                    }
                    None => list.push(WildMon { id: e.species.clone(), name: mon.name.clone(), dex: mon.dex, best, times, levels: e.levels.clone() }),
                }
            }
        }
        for (_, list) in by_method.iter_mut() {
            list.sort_by(|a, b| b.best.cmp(&a.best).then(a.name.cmp(&b.name)));
        }
        let total: usize = by_method.iter().map(|(_, l)| l.len()).sum();
        head(ui, "WILD POKÉMON", (total > 0).then_some(total));
        if total == 0 {
            ui.add_space(2.0);
            ui.label(RichText::new("None here. (Pokémon only added by MORE POKéMON or MODERN POKéMON are in the Pokédex.)").size(12.0).color(MUTED()));
        }
        let mut open: Option<String> = None;
        let mut hunt: Option<u32> = None;
        for (how, list) in &by_method {
            ui.add_space(6.0);
            ui.label(RichText::new(how).font(ch::bold(12.5)).color(TEXT()));
            ui.add_space(2.0);
            let avail = ui.available_width();
            let cols = ((avail + 8.0) / (96.0 + 8.0)).floor().max(2.0) as usize;
            let tw = ((avail - 8.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in list.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for m in row {
                        let (r, resp) = ui.allocate_exact_size(vec2(tw, 92.0), Sense::click());
                        let t = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter().clone();
                        p.rect_filled(r, 12.0, theme::blend(kit::CARD_HOVER(), kit::SELECTED_BG(), t * 0.6));
                        p.rect_stroke(r, 12.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::ACCENT(), t)));
                        if let Some(sp) = self.species.get((m.dex as usize).wrapping_sub(1)).cloned() {
                            self.draw_sprite(&p, &sp, false, pos2(r.center().x, r.min.y + 54.0), Vec2::splat(52.0));
                        }
                        let name = p.layout_no_wrap(m.name.clone(), ch::bold(11.5), TEXT());
                        let nx = r.center().x - name.size().x.min(tw - 10.0) / 2.0;
                        p.with_clip_rect(r.shrink(4.0)).galley(pos2(nx, r.max.y - 34.0), name, TEXT());
                        let pct = if m.best > 0 { format!("{}%", m.best.min(100)) } else { "—".into() };
                        p.text(pos2(r.center().x, r.max.y - 12.0), Align2::CENTER_CENTER, pct, ch::bold(10.5), ch::ink(GREEN));
                        let tip = format!(
                            "{}{}\nMorning {}% · Day {}% · Night {}%\nClick for its Pokédex page · right-click to hunt it",
                            m.name,
                            if m.levels.is_empty() { String::new() } else { format!(" · Lv. {}", m.levels) },
                            m.times[0].min(100),
                            m.times[1].min(100),
                            m.times[2].min(100)
                        );
                        let resp = resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            open = Some(m.id.clone());
                        }
                        resp.context_menu(|ui| {
                            if ui.button("Open in the Pokédex").clicked() {
                                open = Some(m.id.clone());
                                ui.close_menu();
                            }
                            if ui.button("Hunt it (make it the target)").clicked() {
                                hunt = Some(m.dex);
                                ui.close_menu();
                            }
                        });
                    }
                });
                ui.add_space(8.0);
            }
        }

        // items lying here
        let items: Vec<gamemap::MapItem> = map
            .items
            .iter()
            .filter(|it| {
                if zone.matrix == 0 {
                    it.matrix == 0 && map.world_zone_at(it.x as usize, it.y as usize) == Some(z)
                } else {
                    it.matrix == zone.matrix
                        && map
                            .matrices
                            .get(zone.matrix as usize)
                            .and_then(|m| m.header_at(it.x as usize / CHUNK, it.y as usize / CHUNK))
                            .map_or(true, |h| h == z || map.zone(h).is_none())
                }
            })
            .cloned()
            .collect();
        if !items.is_empty() {
            ui.add_space(6.0);
            hairline(ui);
            ui.add_space(10.0);
            head(ui, "ITEMS", Some(items.len()));
            ui.add_space(2.0);
            for it in &items {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                    super::map::item_icon(ui.painter(), r.center(), 6.5, it.hidden);
                    ui.label(RichText::new(&it.name).size(12.5).color(TEXT()));
                    if it.hidden {
                        ui.label(RichText::new("hidden").size(11.0).color(MUTED()));
                    }
                    if !it.note.is_empty() {
                        ui.label(RichText::new(&it.note).size(11.0).color(ch::ink(ORANGE)));
                    }
                });
            }
        }
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
        if let Some(dex) = hunt {
            if let Some(sp) = self.species.get((dex as usize).wrapping_sub(1)) {
                let slug = sp.slug.clone();
                self.set_target(slug);
                self.setup.tab = Tab::Hunt;
            }
        }
    }

    /// Nothing picked yet: how to use the map, and your target.
    fn panel_area_empty(&mut self, ui: &mut egui::Ui, map: &GameMap) {
        ui.add_space(18.0);
        ui.vertical_centered(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
            kit::icon_chip(ui.painter(), r, "📍", CYAN, false);
            ui.add_space(8.0);
            ui.label(RichText::new("Pick a place").font(ch::bold(16.0)).color(TEXT()));
            ui.label(RichText::new("Click any area on the map to see its Pokémon, today's rare grass and the items lying there.").size(12.5).color(MUTED()));
        });
        // your target: where it lives
        if let Some(w) = self.wiki.clone() {
            if let Some(sp) = self.target_idx().and_then(|i| self.species.get(i)).cloned() {
                if let Some(mon) = w.by_dex(sp.dex) {
                    let mut places: Vec<String> = Vec::new();
                    for e in w.wild(&mon.id) {
                        if e.setting.is_empty() && !places.contains(&e.area) {
                            places.push(e.area.clone());
                        }
                    }
                    ui.add_space(16.0);
                    egui::Frame::none().fill(kit::CARD_HOVER()).rounding(16.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(52.0), Sense::hover());
                            let p = ui.painter().clone();
                            p.circle_filled(r.center(), 24.0, kit::CARD());
                            self.draw_sprite(&p, &sp, true, r.center_bottom() - vec2(0.0, 2.0), Vec2::splat(48.0));
                            ui.vertical(|ui| {
                                ui.label(RichText::new("YOUR TARGET").font(ch::bold(10.0)).color(MUTED()));
                                ui.label(RichText::new(sp.name).font(ch::bold(15.0)).color(TEXT()));
                                let n = places.len();
                                ui.label(RichText::new(if n == 0 { "Not in the wild".to_string() } else { format!("Lives in {n} place{}", if n == 1 { "" } else { "s" }) }).size(12.0).color(MUTED()));
                            });
                        });
                        if !places.is_empty() {
                            ui.add_space(8.0);
                            if kit::accent_button_w(ui, egui::Button::new(RichText::new("Show where it lives").font(ch::bold(13.0)).color(kit::on_primary())), 6.0, ui.available_width()).clicked() {
                                self.show_on_map(&places, format!("Where {} lives", sp.name), None);
                            }
                        }
                    });
                }
            }
        }
        let _ = map;
        ui.add_space(16.0);
        head(ui, "GETTING AROUND", None);
        ui.add_space(4.0);
        for (keys, what) in [
            ("Drag · scroll", "Move around and zoom"),
            ("Double-click", "Go inside a town, cave or building"),
            ("+  −  F", "Zoom in, out, or see it all"),
            ("/", "Search for a place or Pokémon"),
            ("Esc", "Let go of the picked place"),
        ] {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(108.0, 22.0), Sense::hover());
                ui.painter().text(r.left_center(), Align2::LEFT_CENTER, keys, egui::FontId::monospace(11.5), TEXT());
                ui.label(RichText::new(what).size(12.0).color(MUTED()));
            });
        }
    }

    // ───────────────────────────── rare grass ─────────────────────────────

    fn panel_rare(&mut self, ui: &mut egui::Ui, spots: &[RareSpot], left: i64) {
        // when the patches change
        egui::Frame::none().fill(ch::faint(GOLD, 0.12)).rounding(16.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                kit::icon_chip(ui.painter(), r, "✨", GOLD, true);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("New patches in {}h {:02}m", left / 3600, left % 3600 / 60)).font(ch::bold(14.5)).color(TEXT()));
                    ui.label(RichText::new("Every day at midnight, Central European time").size(11.5).color(MUTED()));
                });
            });
            ui.add_space(8.0);
            let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
            let done = (1.0 - left as f32 / 86400.0).clamp(0.0, 1.0);
            ui.painter().rect_filled(bar, 3.0, ch::faint(GOLD, 0.2));
            ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * done, 6.0)), 3.0, GOLD);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if ui
                    .add(egui::Button::new("📷  Infographic").fill(kit::btn_fill()))
                    .on_hover_text("Share today's rare grass as an image: the map with every spot, the reset time around the world, the top 3 and where to find them")
                    .clicked()
                {
                    self.mapui.want_poster = true;
                }
                if ui
                    .add(egui::Button::new("⬇  Save list").fill(kit::btn_fill()))
                    .on_hover_text("Save today's list as rare-today.json for sharing (into the GitHub folder if you have one)")
                    .clicked()
                {
                    self.mapui.want_export = true;
                }
            });
            if let Some(m) = self.mapui.export_msg() {
                ui.add_space(4.0);
                ui.label(RichText::new(m).size(11.5).color(theme::LINK()));
            }
        });
        ui.add_space(12.0);
        if spots.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.label(RichText::new("Nothing met on rare grass yet today").font(ch::bold(14.0)).color(TEXT()));
                ui.label(RichText::new("Use Sweet Scent on a rare grass patch and it shows up here (and on the map).").size(12.0).color(MUTED()));
            });
            return;
        }
        if spots.len() > 6 {
            kit::search_box(ui, &mut self.mapui.rare_query, "Filter by Pokémon or place", ui.available_width() - 56.0);
            ui.add_space(8.0);
        }
        let q = self.mapui.rare_query.trim().to_lowercase();
        let target = self.target_idx().and_then(|i| self.species.get(i)).map(|s| s.slug.clone());
        let mut list: Vec<RareSpot> = spots
            .iter()
            .filter(|s| q.is_empty() || s.species.name.to_lowercase().contains(&q) || s.place.to_lowercase().contains(&q))
            .cloned()
            .collect();
        // your target first
        list.sort_by_key(|s| Some(&s.species.slug) != target.as_ref());
        if list.is_empty() {
            ui.label(RichText::new("No match.").size(12.0).color(MUTED()));
        }
        head(ui, "MET TODAY", Some(list.len()));
        ui.add_space(2.0);
        self.rare_rows(ui, &list, true);
        ui.add_space(6.0);
        ui.label(RichText::new("Yours plus the shared list. Right-click one to hunt it there.").size(11.5).color(MUTED()));
    }

    /// Rare grass rows: sprite, name, where, and a Show button.
    fn rare_rows(&mut self, ui: &mut egui::Ui, list: &[RareSpot], show_place: bool) {
        let target = self.target_idx().and_then(|i| self.species.get(i)).map(|s| s.slug.clone());
        let mut go: Option<RareSpot> = None;
        let mut open: Option<String> = None;
        let mut hunt: Option<RareSpot> = None;
        for s in list {
            let mine = Some(&s.species.slug) == target.as_ref();
            let w = ui.available_width();
            let (r, resp) = ui.allocate_exact_size(vec2(w, 54.0), Sense::click());
            let t = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let p = ui.painter().clone();
            let base = if mine { ch::faint(GOLD, 0.14) } else { kit::CARD() };
            p.rect_filled(r, 12.0, theme::blend(base, kit::CARD_HOVER(), t));
            if mine {
                p.rect_stroke(r, 12.0, Stroke::new(1.0f32, ch::faint(GOLD, 0.6)));
            }
            let disc = pos2(r.min.x + 28.0, r.center().y);
            p.circle_filled(disc, 21.0, theme::blend(kit::CARD_HOVER(), ch::faint(GOLD, 0.25), 0.6));
            self.draw_sprite(&p, &s.species, false, pos2(disc.x, r.max.y - 6.0), Vec2::splat(44.0));
            // Show button on the right
            let br = Rect::from_min_size(pos2(r.max.x - 66.0, r.center().y - 14.0), vec2(58.0, 28.0));
            let bresp = ui.interact(br, resp.id.with("show"), Sense::click());
            let bt = kit::anim(ui.ctx(), bresp.id.with("h"), bresp.hovered(), kit::QUICK);
            p.rect_filled(br, 14.0, theme::blend(kit::btn_fill(), kit::primary(), bt));
            p.rect_stroke(br, 14.0, Stroke::new(1.0f32, kit::CONTROL()));
            p.text(br.center(), Align2::CENTER_CENTER, "Show", ch::bold(12.0), theme::blend(TEXT(), kit::on_primary(), bt));
            let tx = r.min.x + 58.0;
            let tw = br.min.x - tx - 8.0;
            let name = if mine { format!("{}  · your target", s.species.name) } else { s.species.name.to_string() };
            let g = p.layout(name, ch::bold(13.0), TEXT(), tw);
            p.with_clip_rect(Rect::from_min_size(pos2(tx, r.min.y), vec2(tw, r.height()))).galley(pos2(tx, r.min.y + 9.0), g, TEXT());
            let sub = if show_place {
                format!("{}{}", s.place, if s.tiles > 1 { format!(" · {} spots", s.tiles) } else { String::new() })
            } else if s.tiles > 1 {
                format!("{} spots", s.tiles)
            } else {
                "1 spot".into()
            };
            let g2 = p.layout_no_wrap(sub.clone(), ch::reg(11.5), MUTED());
            p.with_clip_rect(Rect::from_min_size(pos2(tx, r.min.y), vec2(tw, r.height()))).galley(pos2(tx, r.min.y + 29.0), g2, MUTED());
            if bresp.on_hover_text("Zoom the map in on it").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                go = Some(s.clone());
            }
            let resp = resp.on_hover_text(format!("{} · {sub}\nClick to show it · right-click for more", s.species.name)).on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                go = Some(s.clone());
            }
            resp.context_menu(|ui| {
                if ui.button("Show on the map").clicked() {
                    go = Some(s.clone());
                    ui.close_menu();
                }
                if ui.button("Open in the Pokédex").clicked() {
                    open = s.wiki_id.clone();
                    ui.close_menu();
                }
                if ui.button("Hunt it on rare grass").clicked() {
                    hunt = Some(s.clone());
                    ui.close_menu();
                }
            });
            ui.add_space(6.0);
        }
        if let Some(s) = go {
            self.go_to_rare(&s);
        }
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
        if let Some(s) = hunt {
            let all = self.rare_spots_for(&s.species.slug);
            self.hunt_on_rare_grass(&s.species.slug, s.species.name, &all);
        }
    }

    // ───────────────────────────── minimap ─────────────────────────────

    /// All of Sinnoh, small, with the part on screen framed. Click or drag
    /// on it to move there.
    pub(super) fn map_minimap(&mut self, ui: &mut egui::Ui, map: &GameMap, view: Rect, w: f32) {
        let Some(ov) = self.mapui.overview_tex() else { return };
        let world = map.world();
        let (tw, th) = ((world.w * CHUNK) as f32, (world.h * CHUNK) as f32);
        let h = (w * th / tw).round();
        let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
        let p = ui.painter_at(r.expand(2.0));
        p.rect_filled(r, 10.0, gamemap::OCEAN);
        p.image(ov, r.shrink(1.0), Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
        let to_mini = |t: Pos2| pos2(r.min.x + t.x / tw * r.width(), r.min.y + t.y / th * r.height());
        // what's on screen
        let zoom = self.mapui.zoom.max(0.01);
        let c = self.mapui.center;
        let a = c - view.size() / 2.0 / zoom;
        let b = c + view.size() / 2.0 / zoom;
        let vr = Rect::from_min_max(to_mini(a), to_mini(b)).intersect(r);
        p.rect_filled(vr, 3.0, Color32::from_white_alpha(14));
        p.rect_stroke(vr, 3.0, Stroke::new(1.5f32, Color32::WHITE));
        // the picked place
        if let Some(rr) = self.mapui.selected.and_then(|z| super::map::zones_rect_pub(map, z)) {
            p.circle_filled(to_mini(rr.center()), 3.5, theme::LINK());
        }
        p.rect_stroke(r, 10.0, Stroke::new(1.0f32, Color32::from_white_alpha(60)));
        if let Some(pp) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let t = pos2((pp.x - r.min.x) / r.width() * tw, (pp.y - r.min.y) / r.height() * th);
            if resp.dragged() {
                self.mapui.center = t;
                self.mapui.stop_flying();
            } else {
                self.mapui.fly_to(t, self.mapui.zoom);
            }
        }
        resp.on_hover_text("All of Sinnoh: click or drag to move there").on_hover_cursor(egui::CursorIcon::Crosshair);
    }

    /// Bottom-left line: where you are and help.
    pub(super) fn map_status(&mut self, ui: &mut egui::Ui, map: &GameMap, area: Area, layer: Layer) {
        let wher = match area {
            Area::World => match layer {
                Layer::Overworld => "Sinnoh".to_string(),
                Layer::Caves => "Caves".to_string(),
                Layer::Ruins => "Ruins".to_string(),
                Layer::Side => "Side areas".to_string(),
            },
            Area::Inside(z) => format!("Inside: {}", self.zone_label(map, z)),
        };
        egui::Frame::none()
            .fill(kit::CARD())
            .stroke(Stroke::new(1.0f32, LINE()))
            .rounding(40.0)
            .inner_margin(egui::Margin::symmetric(12.0, 5.0))
            .shadow(super::map::float_shadow_pub())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.label(RichText::new(wher).font(ch::bold(12.0)).color(TEXT()));
                    let (q, qresp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                    ui.painter().circle_stroke(q.center(), 8.0, Stroke::new(1.2f32, MUTED()));
                    ui.painter().text(q.center(), Align2::CENTER_CENTER, "?", ch::bold(11.0), MUTED());
                    qresp.on_hover_text(
                        "Scroll to zoom, drag to move.\nClick a place to see what lives there; double-click to go inside.\n\nKeys: + and − zoom, F shows everything, / searches, Esc lets go.",
                    );
                });
            });
    }
}

/// A small coloured label.
fn chip(ui: &mut egui::Ui, text: &str, c: Color32) {
    let g = ui.painter().layout_no_wrap(text.to_string(), ch::bold(11.0), ch::ink(c));
    let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 16.0, 22.0), Sense::hover());
    ui.painter().rect_filled(r, 11.0, ch::faint(c, 0.16));
    ui.painter().galley(r.center() - g.size() / 2.0, g, ch::ink(c));
}
