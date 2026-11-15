//! Nuzlocke Tracker: your Nuzlocke runs from the game's own files, read
//! only. The route log as a trail (each route, its Pokémon, in or out),
//! with the run's rules and your team beside it. Every world the game keeps
//! files for can be picked at the top.

use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::App;
use crate::nuzlocke::{Pair, RouteMon, World};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};

pub(super) const NUZ_COL: Color32 = Color32::from_rgb(239, 90, 90);
const IN_COL: Color32 = Color32::from_rgb(52, 211, 153);
const OUT_COL: Color32 = Color32::from_rgb(248, 113, 113);
const GREEN: Color32 = Color32::from_rgb(52, 211, 153);
/// The two sides of a Soul Link.
const YOU_COL: Color32 = Color32::from_rgb(96, 165, 250);
const FRIEND_COL: Color32 = Color32::from_rgb(192, 132, 252);

#[derive(Default)]
pub(super) struct NuzState {
    pub worlds: Vec<World>,
    loaded: Option<Instant>,
    /// The world shown (None: the first run).
    pub sel: Option<u32>,
    pub search: String,
    /// 0 every route, 1 in, 2 out.
    pub filter: u8,
}

impl App {
    /// Read the worlds again every few seconds while they're looked at.
    pub(super) fn nuz_refresh(&mut self) {
        if self.setup.nuz.loaded.is_some_and(|t| t.elapsed() < Duration::from_secs(8)) {
            return;
        }
        self.setup.nuz.loaded = Some(Instant::now());
        let root = crate::gamemap::game_dir(self.cfg.game_dir_setting().as_deref());
        self.setup.nuz.worlds = crate::nuzlocke::worlds(&root, self.setup.world.opts.as_ref());
    }

    /// The world shown: the picked one, else the one being played if it's a
    /// run, else the first run.
    pub(super) fn nuz_world(&self) -> Option<&World> {
        let ws = &self.setup.nuz.worlds;
        self.setup
            .nuz
            .sel
            .and_then(|id| ws.iter().find(|w| w.id == id))
            .or_else(|| ws.iter().find(|w| w.playing && w.is_run()))
            .or_else(|| ws.iter().find(|w| w.is_run()))
    }

    pub(super) fn page_nuzlocke(&mut self, ui: &mut egui::Ui) {
        self.nuz_refresh();
        kit::page_header(ui, "NUZLOCKE TRACKER", "Your runs from the game's own files: every route, who's still in, and the rules.");
        egui::Frame::none()
            .fill(theme::blend(kit::CARD(), GREEN, 0.10))
            .stroke(Stroke::new(1.0f32, theme::blend(kit::CARD(), GREEN, 0.45)))
            .rounding(10.0)
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("🔒  Read only, and only on this PC").strong().size(12.5).color(GREEN));
                ui.label(
                    RichText::new("Read from your worlds' files in the game folder, never written. It keeps up as you play. Your team shows for the world you're playing (it's from the save).")
                        .size(12.0)
                        .color(TEXT()),
                );
            });
        ui.add_space(12.0);
        // the worlds
        let worlds: Vec<(u32, String, String, bool, bool)> = self.setup.nuz.worlds.iter().map(|w| (w.id, w.title(), w.kind(), w.playing, w.is_run())).collect();
        let shown = self.nuz_world().map(|w| w.id);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for (id, title, kind, playing, run) in &worlds {
                let on = Some(*id) == shown;
                let label = format!("{}{title} · {kind}", if *playing { "🌍 " } else { "" });
                let b = egui::Button::new(RichText::new(label).size(12.5).color(if on { TEXT() } else if *run { TEXT() } else { MUTED() }))
                    .fill(if on { theme::blend(kit::CARD(), NUZ_COL, 0.18) } else { kit::btn_fill() })
                    .stroke(Stroke::new(1.0f32, if on { NUZ_COL } else { LINE() }))
                    .rounding(14.0);
                let tip = if *playing { "The world you're playing now" } else { "Another world the game keeps" };
                if ui.add_enabled(*run, b).on_hover_text(tip).on_disabled_hover_text("Not a Nuzlocke run: no route log").clicked() {
                    self.setup.nuz.sel = Some(*id);
                }
            }
        });
        ui.add_space(12.0);
        let Some(world) = self.nuz_world().cloned() else {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("No Nuzlocke runs yet").font(super::charts::bold(15.0)).color(TEXT()));
                ui.label(
                    RichText::new("Start one in the game (a new world with a Nuzlocke, Duallocke or Soul Link challenge) and every route shows up here as you play.")
                        .size(12.5)
                        .color(MUTED()),
                );
            });
            return;
        };
        let w = ui.available_width();
        let side_w = 300.0;
        let gap = 16.0;
        if w >= 760.0 {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(w - side_w - gap, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(w - side_w - gap);
                    self.nuz_any_trail(ui, &world);
                });
                ui.allocate_ui_with_layout(vec2(side_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(side_w);
                    self.nuz_run_card(ui, &world);
                });
            });
        } else {
            self.nuz_run_card(ui, &world);
            ui.add_space(gap);
            self.nuz_any_trail(ui, &world);
        }
    }

    /// A Soul Link's pairs side by side, else the plain trail.
    fn nuz_any_trail(&mut self, ui: &mut egui::Ui, world: &World) {
        match &world.friend {
            Some(f) => {
                let ps = crate::nuzlocke::pairs(&world.routes, &f.routes);
                let you = if world.trainer.trim().is_empty() { "You".to_string() } else { world.trainer.trim().to_string() };
                let them = if f.name.trim().is_empty() { "Your friend".to_string() } else { f.name.trim().to_string() };
                self.nuz_soul_trail(ui, world.id, &ps, &you, &them);
            }
            None => self.nuz_trail(ui, world),
        }
    }

    /// A Soul Link: each area's linked pair in two columns, a trainer each,
    /// on the same trail.
    fn nuz_soul_trail(&mut self, ui: &mut egui::Ui, wid: u32, ps: &[Pair], you: &str, them: &str) {
        let total = ps.len();
        let ins = ps.iter().filter(|p| p.alive()).count();
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                kit::search_box(ui, &mut self.setup.nuz.search, "Search a route or Pokémon", 240.0);
                let all = format!("All {total}");
                let i = format!("In {ins}");
                let o = format!("Out {}", total - ins);
                kit::segmented(ui, &mut self.setup.nuz.filter, &[(0, all.as_str()), (1, i.as_str()), (2, o.as_str())]);
            });
            ui.add_space(14.0);
            let full = ui.available_width();
            let lx = 12.0;
            let route_w = (full * 0.2).clamp(104.0, 160.0);
            let link_w = 28.0;
            let col_x = lx + 18.0 + route_w;
            let col_w = ((full - col_x - link_w) / 2.0).floor();
            let cols = [(col_x, you, YOU_COL, true), (col_x + col_w + link_w, them, FRIEND_COL, false)];
            // the column heads: whose is whose
            let (head, _) = ui.allocate_exact_size(vec2(full, 46.0), Sense::hover());
            let p = ui.painter().clone();
            p.text(pos2(head.min.x + lx + 18.0, head.center().y), Align2::LEFT_CENTER, "ROUTE", super::charts::bold(10.5), MUTED());
            for (x, name, col, mine) in cols {
                let r = Rect::from_min_size(pos2(head.min.x + x, head.min.y), vec2(col_w - 6.0, 40.0));
                p.rect_filled(r, 10.0, theme::blend(kit::CARD(), col, 0.12));
                p.rect_filled(Rect::from_min_size(r.min, vec2(4.0, r.height())), egui::Rounding { nw: 10.0, sw: 10.0, ne: 0.0, se: 0.0 }, col);
                let side_in = ps.iter().filter_map(|p| if mine { p.mine.as_ref() } else { p.theirs.as_ref() }).filter(|m| m.alive).count();
                let side_all = ps.iter().filter(|p| if mine { p.mine.is_some() } else { p.theirs.is_some() }).count();
                let nf = super::charts::bold(14.0);
                let g = p.text(pos2(r.min.x + 16.0, r.center().y - 7.0), Align2::LEFT_CENTER, super::charts::clip(ui, name, &nf, r.width() - 30.0), nf, TEXT());
                let _ = g;
                let who = match (mine, name) {
                    (true, "You") => String::new(),
                    (true, _) => "You · ".into(),
                    (false, _) => "Friend · ".into(),
                };
                p.text(pos2(r.min.x + 16.0, r.center().y + 10.0), Align2::LEFT_CENTER, format!("{who}{side_in} of {side_all} in"), crate::fonts::sans_font(11.0), MUTED());
            }
            ui.add_space(6.0);
            let q = self.setup.nuz.search.trim().to_lowercase();
            let filter = self.setup.nuz.filter;
            let matches = |m: &Option<RouteMon>, app: &App| {
                m.as_ref().is_some_and(|r| r.nickname.to_lowercase().contains(&q) || app.box_species(r.species).is_some_and(|s| s.name.to_lowercase().contains(&q)))
            };
            let rows: Vec<(usize, &Pair)> = ps
                .iter()
                .enumerate()
                .filter(|(_, p)| match filter {
                    1 => p.alive(),
                    2 => !p.alive(),
                    _ => true,
                })
                .filter(|(_, p)| q.is_empty() || p.route.to_lowercase().contains(&q) || matches(&p.mine, self) || matches(&p.theirs, self))
                .collect();
            if rows.is_empty() {
                ui.label(RichText::new(if total == 0 { "No routes yet: catch your first Pokémon in the game." } else { "Nothing matches." }).size(12.5).color(MUTED()));
                return;
            }
            let row_h = 52.0;
            let (area, _) = ui.allocate_exact_size(vec2(full, row_h * rows.len() as f32), Sense::hover());
            let p = ui.painter().clone();
            let tx = area.min.x + lx;
            p.line_segment([pos2(tx, area.min.y + row_h / 2.0), pos2(tx, area.max.y - row_h / 2.0)], Stroke::new(2.0f32, LINE()));
            for (k, (n, pair)) in rows.iter().enumerate() {
                let rr = Rect::from_min_size(pos2(area.min.x, area.min.y + k as f32 * row_h), vec2(full, row_h));
                let resp = ui.interact(rr, ui.id().with(("nuz-pair", wid, *n)), Sense::hover());
                if resp.hovered() {
                    p.rect_filled(rr.shrink2(vec2(0.0, 2.0)), 10.0, kit::CARD_HOVER());
                }
                let cy = rr.center().y;
                let alive = pair.alive();
                let col = if !alive { OUT_COL } else if pair.whole() { IN_COL } else { MUTED() };
                p.circle_filled(pos2(tx, cy), 6.5, kit::CARD());
                p.circle_filled(pos2(tx, cy), 5.0, col);
                let rf = crate::fonts::sans_font(12.5);
                p.text(pos2(tx + 18.0, cy), Align2::LEFT_CENTER, super::charts::clip(ui, &pair.route, &rf, route_w - 8.0), rf, MUTED());
                for (i, (x, _, scol, _)) in cols.iter().enumerate() {
                    let cell = Rect::from_min_size(pos2(area.min.x + x, rr.min.y + 4.0), vec2(col_w - 6.0, row_h - 8.0));
                    let m = if i == 0 { &pair.mine } else { &pair.theirs };
                    self.nuz_cell(&p, ui, cell, m.as_ref(), *scol);
                }
                // the link between the two halves: whole while both are in
                let lc = pos2(area.min.x + col_x + col_w - 3.0 + link_w / 2.0 - 3.0, cy);
                let lcol = if !pair.whole() {
                    LINE()
                } else if alive {
                    theme::blend(LINE(), IN_COL, 0.5)
                } else {
                    theme::blend(LINE(), OUT_COL, 0.4)
                };
                let gap = if alive && pair.whole() { 0.0 } else { 3.0 };
                p.circle_stroke(lc - vec2(4.0 + gap, 0.0), 4.0, Stroke::new(1.6f32, lcol));
                p.circle_stroke(lc + vec2(4.0 + gap, 0.0), 4.0, Stroke::new(1.6f32, lcol));
                let name = |m: &Option<RouteMon>| m.as_ref().map(|r| self.box_species(r.species).map_or_else(|| format!("#{}", r.species), |s| s.name.to_string()));
                let tip = match (name(&pair.mine), name(&pair.theirs)) {
                    (Some(a), Some(b)) if alive => format!("{}: your {a} and {them}'s {b} are linked, and both still in.", pair.route),
                    (Some(a), Some(b)) => format!("{}: your {a} and {them}'s {b} are linked; one is out, so the pair is out.", pair.route),
                    (Some(a), None) => format!("{}: your {a}. {them} hasn't got one here yet (as the game last saw it).", pair.route),
                    (None, Some(b)) => format!("{}: {them}'s {b}. You haven't got one here yet.", pair.route),
                    (None, None) => pair.route.clone(),
                };
                resp.on_hover_text(tip);
            }
            ui.add_space(8.0);
            ui.label(RichText::new(format!("{them}'s side is as the game last saw it, from your world's files.")).size(11.5).color(MUTED()));
        });
    }

    /// One trainer's Pokémon in a Soul Link row (or an empty slot).
    fn nuz_cell(&mut self, p: &egui::Painter, ui: &egui::Ui, cell: Rect, m: Option<&RouteMon>, side: Color32) {
        let cy = cell.center().y;
        let Some(r) = m else {
            p.rect_stroke(cell.shrink(1.0), 10.0, Stroke::new(1.0f32, theme::blend(kit::CARD(), LINE(), 0.7)));
            p.text(pos2(cell.min.x + 14.0, cy), Align2::LEFT_CENTER, "Nothing yet", crate::fonts::sans_font(12.0), MUTED().gamma_multiply(0.8));
            return;
        };
        p.rect_filled(cell, 10.0, theme::blend(kit::CARD(), side, if r.alive { 0.06 } else { 0.02 }));
        let fade: f32 = if r.alive { 1.0 } else { 0.55 };
        let sx = cell.min.x + 26.0;
        let sp = self.box_species(r.species);
        let name = sp.as_ref().map_or_else(|| format!("#{}", r.species), |s| s.name.to_string());
        match &sp {
            Some(s) if self.sprites.get(s, false).is_some() => {
                let tint = if r.alive { Color32::WHITE } else { Color32::from_rgba_unmultiplied(140, 140, 150, 200) };
                self.draw_sprite_tinted(p, s, false, pos2(sx, cy + 17.0), vec2(40.0, 40.0), tint);
            }
            _ => {
                p.circle_filled(pos2(sx, cy), 15.0, kit::sunken());
                let ini: String = name.chars().filter(|c| c.is_alphanumeric()).take(2).collect();
                p.text(pos2(sx, cy), Align2::CENTER_CENTER, ini, super::charts::bold(11.5), MUTED().gamma_multiply(fade.max(0.7)));
            }
        }
        let col = if r.alive { IN_COL } else { OUT_COL };
        let label = if r.alive { "In" } else { "Out" };
        let pg = p.layout_no_wrap(label.to_string(), super::charts::bold(11.0), col);
        let pill = Rect::from_min_size(pos2(cell.max.x - 10.0 - pg.size().x - 16.0, cy - 10.0), vec2(pg.size().x + 16.0, 20.0));
        p.rect_filled(pill, 10.0, theme::blend(kit::CARD(), col, 0.16));
        p.galley(pos2(pill.min.x + 8.0, cy - pg.size().y / 2.0), pg, col);
        let tx = sx + 26.0;
        let room = (pill.min.x - 8.0 - tx).max(20.0);
        let nf = super::charts::bold(13.0);
        let g = p.text(pos2(tx, cy - 7.0), Align2::LEFT_CENTER, super::charts::clip(ui, &name, &nf, room), nf, TEXT().gamma_multiply(fade));
        let plain = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_uppercase();
        let sub = if !r.nickname.is_empty() && plain(&r.nickname) != plain(&name) && !plain(&name).starts_with(&plain(&r.nickname)) {
            format!("\"{}\" · Lv {}", r.nickname, r.level)
        } else {
            format!("Lv {} when met", r.level)
        };
        let _ = g;
        let sf = crate::fonts::sans_font(11.0);
        p.text(pos2(tx, cy + 10.0), Align2::LEFT_CENTER, super::charts::clip(ui, &sub, &sf, room), sf, MUTED());
    }

    /// The route log as a trail: in the order you got them, a pin each.
    fn nuz_trail(&mut self, ui: &mut egui::Ui, world: &World) {
        let total = world.routes.len();
        let ins = world.routes.iter().filter(|r| r.alive).count();
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                kit::search_box(ui, &mut self.setup.nuz.search, "Search a route or Pokémon", 240.0);
                let all = format!("All {total}");
                let i = format!("In {ins}");
                let o = format!("Out {}", total - ins);
                kit::segmented(ui, &mut self.setup.nuz.filter, &[(0, all.as_str()), (1, i.as_str()), (2, o.as_str())]);
            });
            ui.add_space(12.0);
            let q = self.setup.nuz.search.trim().to_lowercase();
            let filter = self.setup.nuz.filter;
            let rows: Vec<(usize, RouteMon)> = world
                .routes
                .iter()
                .cloned()
                .enumerate()
                .filter(|(_, r)| match filter {
                    1 => r.alive,
                    2 => !r.alive,
                    _ => true,
                })
                .filter(|(_, r)| {
                    q.is_empty() || r.route.to_lowercase().contains(&q) || r.nickname.to_lowercase().contains(&q) || self.box_species(r.species).is_some_and(|s| s.name.to_lowercase().contains(&q))
                })
                .collect();
            if rows.is_empty() {
                ui.label(RichText::new(if total == 0 { "No routes yet: catch your first Pokémon in the game." } else { "Nothing matches." }).size(12.5).color(MUTED()));
                return;
            }
            let row_h = 48.0;
            let full = ui.available_width();
            let (area, _) = ui.allocate_exact_size(vec2(full, row_h * rows.len() as f32), Sense::hover());
            let p = ui.painter().clone();
            let lx = area.min.x + 12.0;
            // the trail's line, top pin to bottom pin
            p.line_segment([pos2(lx, area.min.y + row_h / 2.0), pos2(lx, area.max.y - row_h / 2.0)], Stroke::new(2.0f32, LINE()));
            let route_w = (full * 0.24).clamp(120.0, 190.0);
            for (k, (n, r)) in rows.iter().enumerate() {
                let rr = Rect::from_min_size(pos2(area.min.x, area.min.y + k as f32 * row_h), vec2(full, row_h));
                let resp = ui.interact(rr, ui.id().with(("nuz-row", world.id, *n)), Sense::hover());
                if resp.hovered() {
                    p.rect_filled(rr.shrink2(vec2(0.0, 2.0)), 10.0, kit::CARD_HOVER());
                }
                let cy = rr.center().y;
                let col = if r.alive { IN_COL } else { OUT_COL };
                p.circle_filled(pos2(lx, cy), 6.5, kit::CARD());
                p.circle_filled(pos2(lx, cy), 5.0, col);
                let fade: f32 = if r.alive { 1.0 } else { 0.55 };
                let rf = crate::fonts::sans_font(12.5);
                p.text(pos2(lx + 20.0, cy), Align2::LEFT_CENTER, super::charts::clip(ui, &r.route, &rf, route_w - 10.0), rf, MUTED().gamma_multiply(fade.max(0.8)));
                let sx = lx + 20.0 + route_w + 20.0;
                let sp = self.box_species(r.species);
                let name = sp.as_ref().map_or_else(|| format!("#{}", r.species), |s| s.name.to_string());
                match &sp {
                    Some(s) if self.sprites.get(s, false).is_some() => {
                        let tint = if r.alive { Color32::WHITE } else { Color32::from_rgba_unmultiplied(140, 140, 150, 200) };
                        self.draw_sprite_tinted(&p, s, false, pos2(sx, cy + 17.0), vec2(40.0, 40.0), tint);
                    }
                    _ => {
                        // no picture (yet): its first letters, like a PC box slot
                        p.circle_filled(pos2(sx, cy), 15.0, kit::sunken());
                        let ini: String = name.chars().filter(|c| c.is_alphanumeric()).take(2).collect();
                        p.text(pos2(sx, cy), Align2::CENTER_CENTER, ini, super::charts::bold(11.5), MUTED().gamma_multiply(fade.max(0.7)));
                    }
                }
                let nf = super::charts::bold(13.5);
                let g = p.text(pos2(sx + 28.0, cy - 7.0), Align2::LEFT_CENTER, &name, nf, TEXT().gamma_multiply(fade));
                // a real nickname (not just its name in capitals)
                let plain = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_uppercase();
                if !r.nickname.is_empty() && plain(&r.nickname) != plain(&name) && !plain(&name).starts_with(&plain(&r.nickname)) {
                    p.text(pos2(g.max.x + 8.0, cy - 7.0), Align2::LEFT_CENTER, format!("\"{}\"", r.nickname), crate::fonts::sans_font(12.5), theme::GOLD.gamma_multiply(fade));
                }
                p.text(pos2(sx + 28.0, cy + 10.0), Align2::LEFT_CENTER, format!("Lv {} when met", r.level), crate::fonts::sans_font(11.5), MUTED());
                // in / out
                let label = if r.alive { "In" } else { "Out" };
                let pg = p.layout_no_wrap(label.to_string(), super::charts::bold(11.5), col);
                let pill = Rect::from_min_size(pos2(rr.max.x - 14.0 - pg.size().x - 20.0, cy - 11.0), vec2(pg.size().x + 20.0, 22.0));
                p.rect_filled(pill, 11.0, theme::blend(kit::CARD(), col, 0.16));
                p.galley(pos2(pill.min.x + 10.0, cy - pg.size().y / 2.0), pg, col);
                resp.on_hover_text(if r.alive {
                    format!("{name} from {}: still in the run.", r.route)
                } else {
                    format!("{name} from {}: out of the run (the game marks it gone: fainted, or not caught).", r.route)
                });
            }
        });
    }

    /// The run's rules, its numbers, and your team.
    fn nuz_run_card(&mut self, ui: &mut egui::Ui, world: &World) {
        let o = world.opts.clone();
        let word = |k: &str| o.as_ref().and_then(|o| o.word(k).map(|s| s.to_string()));
        let total = world.routes.len();
        let ins = world.routes.iter().filter(|r| r.alive).count();
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(world.title()).font(super::charts::bold(16.0)).color(TEXT()));
            ui.label(RichText::new(if world.playing { "Playing now" } else { "Not the world you're playing" }).size(12.0).color(if world.playing { GREEN } else { MUTED() }));
            ui.add_space(12.0);
            // the numbers
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let cw = (ui.available_width() - 16.0) / 3.0;
                for (v, l, c) in [(total.to_string(), "routes", TEXT()), (ins.to_string(), "in", IN_COL), ((total - ins).to_string(), "out", OUT_COL)] {
                    let (r, _) = ui.allocate_exact_size(vec2(cw, 54.0), Sense::hover());
                    ui.painter().rect_filled(r, 10.0, kit::sunken());
                    ui.painter().text(pos2(r.min.x + 10.0, r.min.y + 20.0), Align2::LEFT_CENTER, v, super::charts::heavy(18.0), c);
                    ui.painter().text(pos2(r.min.x + 10.0, r.max.y - 13.0), Align2::LEFT_CENTER, l, crate::fonts::sans_font(11.0), MUTED());
                }
            });
            ui.add_space(12.0);
            let pretty = |s: String| {
                let s = s.replace('_', " ");
                let mut c = s.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            };
            let mut rows: Vec<(&str, String)> = vec![("Type", world.kind())];
            if let Some(v) = word("level_caps") {
                rows.push(("Level caps", pretty(v)));
            }
            if let Some(v) = word("lives") {
                rows.push(("Lives", if v == "0" { "None".into() } else { v }));
            }
            if let Some(v) = word("party").filter(|v| v != "1") {
                rows.push(("Players", v));
            }
            if let Some(v) = word("friend_deaths").filter(|_| word("party").is_some_and(|p| p != "1")) {
                rows.push(("Friends' losses", v));
            }
            if let Some(v) = word("starter_region") {
                rows.push(("Starters from", pretty(v)));
            }
            if let Some(v) = word("random_species").filter(|_| word("preset").is_some_and(|p| p.contains("random"))) {
                rows.push(("Swaps", pretty(v)));
            }
            for (k, v) in rows {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(k).size(12.5).color(MUTED()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(v).size(12.5).color(TEXT()));
                    });
                });
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter().rect_filled(r, 0.0, LINE());
            }
            ui.add_space(12.0);
            ui.label(RichText::new("TEAM").size(10.5).strong().color(MUTED()));
            ui.add_space(6.0);
            let party = if world.playing { self.setup.boxes.save.as_ref().map(|s| s.party.clone()) } else { None };
            match party {
                Some(party) if !party.is_empty() => {
                    let n = party.len();
                    let cw = (ui.available_width() / 3.0).floor();
                    for row in party.chunks(3) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            for m in row {
                                let (r, resp) = ui.allocate_exact_size(vec2(cw, 64.0), Sense::hover());
                                let p = ui.painter().clone();
                                if m.egg {
                                    super::eggs::paint_egg(&p, pos2(r.center().x, r.min.y + 24.0), 26.0, 0.0, kit::sunken(), theme::GOLD, MUTED());
                                } else if let Some(s) = self.box_species(m.species) {
                                    self.draw_sprite(&p, &s, m.shiny(), pos2(r.center().x, r.min.y + 44.0), vec2(44.0, 44.0));
                                    resp.on_hover_text(s.name);
                                }
                                if let Some(l) = m.level {
                                    p.text(pos2(r.center().x, r.max.y - 8.0), Align2::CENTER_CENTER, format!("Lv {l}"), crate::fonts::sans_font(11.0), MUTED());
                                }
                            }
                        });
                    }
                    let _ = n;
                }
                _ => {
                    ui.label(
                        RichText::new(if world.playing { "Reading your save…" } else { "Your team shows for the world you're playing (it's from the save)." })
                            .size(12.0)
                            .color(MUTED()),
                    );
                }
            }
        });
    }
}
