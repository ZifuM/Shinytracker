//! Pinned apps: up to 8 apps for quick opening, as a 2 × 4 grid in the
//! sidebar, a dock in the top bar, or a dock floating at the bottom of the
//! window that also holds every open app (Settings → Appearance chooses).
//! A pin opens its app; a small dash shows it's open and the app you're in
//! is outlined. Right-click a pin for Close, Unpin, Move and its store page;
//! "+" pins another app. In the bottom dock, hovering an app also shows a pin
//! button (keep it in the dock or take it out) and, on an open app that isn't
//! pinned, a ✕ that closes it.

use super::shell::app_of;
use super::store::{app_col, app_icon};
use super::ui_kit::{self as kit, CARD_HOVER, LINE, MUTED, SIDEBAR, TEXT};
use super::App;
use crate::apps::{AppId, PinsPlace, DOCK_MAX, MAX_PINS};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};

const OPEN_DASH: Color32 = Color32::from_rgb(120, 214, 140);
/// The bottom dock: an icon's cell at rest, the space between cells, the
/// dock's padding, how far it floats above the window's bottom edge, the
/// width of the line between pins and open apps, how much the nearest icon
/// grows when magnifying, and how far (px) the swell reaches either side.
const DOCK_CELL: f32 = 46.0;
const DOCK_SPACE: f32 = 6.0;
const DOCK_PAD: f32 = 8.0;
const DOCK_MARGIN: f32 = 14.0;
const DOCK_GAP_W: f32 = 9.0;
const DOCK_GROW: f32 = 0.55;
const DOCK_REACH: f32 = 120.0;
/// The radius of the pin and ✕ buttons on a hovered dock icon's corners.
const DOCK_BADGE: f32 = 8.0;
/// Modern Plus: the hunt capsule's width, and how close (px) to the bottom
/// of the page the pointer has to come for the hidden dock to slide up.
const CAP_W: f32 = 250.0;
const DOCK_REVEAL: f32 = 70.0;
/// The gap between the lone hunt pill and the apps button beside it.
const APPS_GAP: f32 = 8.0;

impl App {
    pub(super) fn pins_place(&self) -> PinsPlace {
        self.cfg.apps.pins_place
    }

    /// The sidebar's PINNED section (when pins live in the sidebar).
    pub(super) fn nav_pins(&mut self, ui: &mut egui::Ui, compact: bool) {
        if self.pins_place() != PinsPlace::Sidebar {
            return;
        }
        let pins = self.cfg.apps.pin_list();
        if !compact {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(egui::RichText::new("PINNED").font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(SIDEBAR(), MUTED(), 0.75)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(format!("{}/{MAX_PINS}", pins.len())).font(crate::fonts::sans_font(11.0)).color(theme::blend(SIDEBAR(), MUTED(), 0.6)));
                });
            });
            ui.add_space(4.0);
        } else {
            ui.add_space(8.0);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 9.0), Sense::hover());
            ui.painter().line_segment([r.left_center() + vec2(14.0, 0.0), r.right_center() - vec2(14.0, 0.0)], Stroke::new(1.0f32, LINE()));
        }
        let w = ui.available_width();
        if compact {
            // one column of icons
            for a in &pins {
                let (r, _) = ui.allocate_exact_size(vec2(w, 44.0), Sense::hover());
                self.pin_cell(ui, Rect::from_center_size(r.center(), vec2(44.0, 40.0)), *a, false, false, true);
                ui.add_space(2.0);
            }
            if pins.len() < MAX_PINS {
                let (r, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
                self.pin_add_cell(ui, Rect::from_center_size(r.center(), vec2(44.0, 36.0)), true, pins.len(), false);
            }
            return;
        }
        let gap = 6.0;
        let cw = ((w - gap * 3.0) / 4.0).floor();
        let rows = if pins.len() >= 4 { 2 } else { 1 };
        for row in 0..rows {
            let (rr, _) = ui.allocate_exact_size(vec2(w, 46.0), Sense::hover());
            for c in 0..4 {
                let i = row * 4 + c;
                let cell = Rect::from_min_size(pos2(rr.min.x + c as f32 * (cw + gap), rr.min.y), vec2(cw, 46.0));
                if let Some(a) = pins.get(i) {
                    self.pin_cell(ui, cell, *a, false, false, true);
                } else if i < MAX_PINS {
                    self.pin_add_cell(ui, cell, i == pins.len(), i, false);
                }
            }
            ui.add_space(gap);
        }
    }

    /// The top bar's dock (when pins live there). Draws from `x` leftwards
    /// to no further than `max_x`; returns where it ended (its right edge).
    pub(super) fn top_dock(&mut self, ui: &mut egui::Ui, x: f32, cy: f32, max_x: f32) -> f32 {
        if self.pins_place() != PinsPlace::TopBar {
            return x;
        }
        let pins = self.cfg.apps.pin_list();
        let cell = 36.0;
        let gap = 4.0;
        // Magnifying dock (Settings → Appearance): icons swell as the pointer
        // passes, their neighbours a little less, all inside the top bar
        let magnify = self.cfg.apps.dock_magnify;
        const GROW: f32 = 0.33;
        let spare = if magnify { cell * GROW * 1.6 } else { 0.0 };
        let n = pins.len() + (pins.len() < self.cfg.apps.max_pins()) as usize;
        let room = ((max_x - x - 12.0 - spare) / (cell + gap)).floor().max(0.0) as usize;
        if room == 0 {
            return x;
        }
        let shown = n.min(room);
        let rest_w = shown as f32 * (cell + gap) - gap + 8.0;
        let ptr = ui.ctx().pointer_hover_pos();
        let near = magnify && ptr.is_some_and(|q| q.x > x - 24.0 && q.x < x + rest_w + spare + 24.0 && (q.y - cy).abs() < 28.0);
        let sizes: Vec<f32> = (0..shown)
            .map(|k| {
                let target = match ptr {
                    Some(q) if near => {
                        // distance from where the icon sits at rest
                        let at = x + 4.0 + k as f32 * (cell + gap) + cell / 2.0;
                        cell * (1.0 + GROW * (1.0 - (q.x - at).abs() / 96.0).max(0.0))
                    }
                    _ => cell,
                };
                if magnify && kit::motion() {
                    ui.ctx().animate_value_with_time(ui.id().with(("dock-mag", k)), target, 0.09)
                } else {
                    target
                }
            })
            .collect();
        let total: f32 = sizes.iter().sum::<f32>() + gap * (shown as f32 - 1.0) + 8.0;
        let tallest = sizes.iter().copied().fold(cell, f32::max);
        let frame = Rect::from_min_size(pos2(x, cy - tallest / 2.0 - 4.0), vec2(total, tallest + 8.0));
        let p = ui.painter();
        p.rect_filled(frame, 14.0, theme::blend(kit::PAGE(), TEXT(), 0.035));
        p.rect_stroke(frame, 14.0, Stroke::new(1.0f32, LINE()));
        let mut left = frame.min.x + 4.0;
        for (k, s) in sizes.iter().enumerate() {
            // each icon centred on the bar, so it grows up and down evenly
            let r = Rect::from_min_size(pos2(left, cy - s / 2.0), vec2(*s, *s));
            left += s + gap;
            match pins.get(k) {
                Some(a) => self.pin_cell(ui, r, *a, true, true, true),
                None => self.pin_add_cell(ui, r, true, k, false),
            }
        }
        frame.max.x
    }

    /// Room the page leaves at its bottom so the floating dock never hides
    /// its last row.
    pub(super) fn dock_pad(&self) -> f32 {
        if self.pins_place() == PinsPlace::Dock && !self.takeover_open() {
            DOCK_CELL + 2.0 * DOCK_PAD + DOCK_MARGIN + 8.0
        } else {
            0.0
        }
    }

    /// The floating dock at the bottom of the window (when pins live there):
    /// your pins, then every other open app, then "+". It's as wide as what's
    /// in it (up to `DOCK_MAX` apps) and sits in the middle of the page.
    pub(super) fn bottom_dock(&mut self, ctx: &egui::Context) {
        if self.pins_place() != PinsPlace::Dock || self.takeover_open() || self.setup.settings_open.is_some() {
            return;
        }
        // the first time the dock is used: every app you have goes on it
        if !self.cfg.apps.dock_seeded {
            self.cfg.apps.seed_dock();
            self.cfg.apps.dock_seeded = true;
            self.dirty = true;
        }
        // where pages are drawn (the page area saves it as it draws)
        let page = ctx.data(|d| d.get_temp::<Rect>(egui::Id::new("tour-page"))).unwrap_or_else(|| ctx.screen_rect());
        enum Item {
            App(AppId, bool),
            Gap,
            Add,
            Edit,
        }
        // Edit mode (the pencil): the pin, unpin and close buttons show on
        // every app; Esc or the pencil again ends it
        // (a tour step about editing turns it on, and off when the tour ends)
        if super::app_tour::wants(ctx, "dock.edit") {
            set_dock_editing(ctx, true);
        }
        if super::app_tour::wants_undo(ctx, "dock.edit") {
            set_dock_editing(ctx, false);
        }
        let editing = dock_editing(ctx);
        if editing && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            set_dock_editing(ctx, false);
        }
        let editing = dock_editing(ctx);
        let pins = self.cfg.apps.pin_list();
        let mut items: Vec<Item> = pins.iter().take(DOCK_MAX).map(|a| Item::App(*a, true)).collect();
        let others: Vec<AppId> = self.cfg.apps.open_list().into_iter().filter(|a| !pins.contains(a)).take(DOCK_MAX.saturating_sub(pins.len())).collect();
        if !others.is_empty() && !items.is_empty() {
            items.push(Item::Gap);
        }
        items.extend(others.into_iter().map(|a| Item::App(a, false)));
        if pins.len() < self.cfg.apps.max_pins() {
            items.push(Item::Add);
        }
        if !items.is_empty() {
            items.push(Item::Gap);
        }
        items.push(Item::Edit);
        let rest = |it: &Item| if matches!(it, Item::Gap) { DOCK_GAP_W } else { DOCK_CELL };
        // Modern Plus: room at the dock's left for the hunt capsule
        let plus = kit::plus();
        let cap_room = if plus { CAP_W + DOCK_SPACE + DOCK_GAP_W + DOCK_SPACE } else { 0.0 };
        let rest_w: f32 = items.iter().map(rest).sum::<f32>() + DOCK_SPACE * (items.len() as f32 - 1.0).max(0.0) + 2.0 * DOCK_PAD + cap_room;
        let base_y = page.max.y - DOCK_MARGIN; // the dock's bottom edge
        let rest_frame = Rect::from_min_size(pos2(page.center().x - rest_w / 2.0, base_y - DOCK_CELL - 2.0 * DOCK_PAD), vec2(rest_w, DOCK_CELL + 2.0 * DOCK_PAD));
        // Magnifying (Appearance): icons swell as the pointer passes, the
        // nearest most, growing up out of the dock like a Mac's
        // (not while editing: icons stay put so they can be dragged)
        let magnify = self.cfg.apps.dock_magnify && !editing;
        let ptr = ctx.pointer_hover_pos();
        let near = magnify && ptr.is_some_and(|q| Rect::from_min_max(pos2(rest_frame.min.x - 30.0, rest_frame.min.y - DOCK_CELL * DOCK_GROW), pos2(rest_frame.max.x + 30.0, rest_frame.max.y)).contains(q));
        let mut x = rest_frame.min.x + DOCK_PAD + cap_room;
        let sizes: Vec<f32> = items
            .iter()
            .enumerate()
            .map(|(k, it)| {
                let w = rest(it);
                let centre = x + w / 2.0;
                x += w + DOCK_SPACE;
                if matches!(it, Item::Gap | Item::Edit) {
                    return w;
                }
                let target = match ptr {
                    Some(q) if near => DOCK_CELL * (1.0 + DOCK_GROW * (1.0 - ((q.x - centre).abs() / DOCK_REACH).powi(2)).max(0.0)),
                    _ => DOCK_CELL,
                };
                if magnify && kit::motion() {
                    ctx.animate_value_with_time(egui::Id::new(("bottom-dock-mag", k)), target, 0.09)
                } else {
                    target
                }
            })
            .collect();
        let total: f32 = sizes.iter().sum::<f32>() + DOCK_SPACE * (items.len() as f32 - 1.0).max(0.0) + 2.0 * DOCK_PAD + cap_room;
        let frame = Rect::from_min_size(pos2(page.center().x - total / 2.0, rest_frame.min.y), vec2(total, rest_frame.height()));
        // Modern Plus: the dock tucks away below the window and slides up when
        // the pointer comes near the bottom (or a dock menu is open); the hunt
        // capsule stays on screen either way
        let shown = if plus {
            let now = ctx.input(|i| i.time);
            let until_id = egui::Id::new("dock-shown-until");
            let near_bottom = ptr.is_some_and(|q| q.y > page.max.y - DOCK_REVEAL && q.x > frame.min.x - 80.0 && q.x < frame.max.x + 80.0);
            let over = ptr.is_some_and(|q| frame.expand(8.0).contains(q));
            // (a tour step pointing at the dock brings it up too)
            let pass = ctx.cumulative_pass_nr();
            let tour = ctx.data(|d| d.get_temp::<u64>(egui::Id::new("tour-dock"))).is_some_and(|n| pass.saturating_sub(n) <= 2);
            if near_bottom || over || editing || tour || ctx.memory(|m| m.any_popup_open()) {
                ctx.data_mut(|d| d.insert_temp(until_id, now + 0.8));
            }
            let until: f64 = ctx.data(|d| d.get_temp(until_id)).unwrap_or(0.0);
            let want = now < until;
            if want {
                ctx.request_repaint_after(std::time::Duration::from_millis(850));
            }
            if kit::motion() {
                ctx.animate_bool_with_time(egui::Id::new("dock-shown"), want, 0.18)
            } else if want {
                1.0
            } else {
                0.0
            }
        } else {
            1.0
        };
        let drop = (1.0 - shown) * (frame.height() + DOCK_MARGIN + 10.0);
        let cap_slot = Rect::from_min_size(pos2(frame.min.x + DOCK_PAD, frame.max.y - DOCK_PAD - DOCK_CELL), vec2(CAP_W, DOCK_CELL));
        let frame = frame.translate(vec2(0.0, drop));
        let tallest = sizes.iter().copied().fold(DOCK_CELL, f32::max);
        // the area reaches up over grown icons only while they're grown, so
        // the page above the dock stays clickable
        let area = Rect::from_min_max(pos2(frame.min.x, frame.min.y.min(frame.max.y - DOCK_PAD - tallest - 4.0)), frame.max);
        // Modern Plus: the hunt capsule, in its place on the dock (moving with
        // it), or alone at the bottom of the page while the dock is tucked away
        // (the pill and the apps button beside it, centred together)
        let alone = Rect::from_min_size(pos2(page.center().x - (CAP_W + APPS_GAP + DOCK_CELL) / 2.0, base_y - DOCK_CELL - 2.0), vec2(CAP_W, DOCK_CELL));
        let docked_at = cap_slot.translate(vec2(0.0, drop));
        let l = |a: f32, b: f32| a + (b - a) * shown;
        let cap = Rect::from_min_size(pos2(l(alone.min.x, docked_at.min.x), l(alone.min.y, docked_at.min.y)), cap_slot.size());
        // (drawn in the dock's own layer while the dock shows, so the dock
        // never paints over it)
        let area = if plus { area.union(cap) } else { area };
        if shown > 0.01 {
        egui::Area::new(egui::Id::new("bottom-dock"))
            .order(egui::Order::Middle)
            .constrain(false)
            .fixed_pos(area.min)
            .show(ctx, |ui| {
                ui.allocate_exact_size(area.size(), Sense::hover());
                let p = ui.painter().clone();
                // (a touch of the accent colour and a firmer edge, so the dock
                // stands apart from the page under it)
                p.add(kit::shadow_tier(4).as_shape(frame, 20.0));
                p.rect_filled(frame, 20.0, theme::blend(kit::CARD(), kit::primary(), 0.07));
                p.rect_stroke(frame, 20.0, Stroke::new(1.2f32, theme::blend(LINE(), kit::primary(), 0.45)));
                super::app_tour::mark(ui.ctx(), "nav.dock", frame);
                let mut left = frame.min.x + DOCK_PAD + cap_room;
                let bottom = frame.max.y - DOCK_PAD;
                // edit mode: pinned apps can be dragged into a new order
                let drag_id = egui::Id::new("dock-drag");
                let mut dragging: Option<AppId> = if editing { ui.ctx().data(|d| d.get_temp::<AppId>(drag_id)) } else { None };
                if dragging.is_some_and(|a| !pins.contains(&a)) {
                    dragging = None;
                }
                let mut centres: Vec<(AppId, f32)> = Vec::new();
                let mut slot: Option<Rect> = None;
                if plus {
                    // the line between the hunt capsule and the apps
                    let cx = frame.min.x + DOCK_PAD + CAP_W + DOCK_SPACE + DOCK_GAP_W / 2.0;
                    p.line_segment([pos2(cx, frame.min.y + 12.0), pos2(cx, frame.max.y - 12.0)], Stroke::new(1.0f32, LINE()));
                }
                for (k, it) in items.iter().enumerate() {
                    let s = sizes[k];
                    match it {
                        Item::Gap => {
                            let cx = left + s / 2.0;
                            p.line_segment([pos2(cx, frame.min.y + 12.0), pos2(cx, frame.max.y - 12.0)], Stroke::new(1.0f32, LINE()));
                        }
                        Item::App(a, pinned) => {
                            let r = Rect::from_min_size(pos2(left, bottom - s), vec2(s, s));
                            if dragging == Some(*a) {
                                // where it'll land: an empty dashed slot
                                dashed_round(&p, r.shrink(4.0), theme::blend(SIDEBAR(), TEXT(), 0.35));
                                slot = Some(r);
                            } else {
                                self.pin_cell(ui, r, *a, true, false, *pinned);
                                self.dock_badges(ui, r, *a, *pinned);
                            }
                            if editing && *pinned {
                                let dresp = ui.interact(r.shrink(8.0), ui.id().with(("dock-drag", *a)), Sense::drag());
                                if dresp.drag_started() {
                                    dragging = Some(*a);
                                }
                                if dragging.is_none() && dresp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                }
                                centres.push((*a, r.center().x));
                            }
                        }
                        Item::Add => {
                            let r = Rect::from_min_size(pos2(left, bottom - s), vec2(s, s));
                            self.pin_add_cell(ui, r.shrink(4.0), true, 100, true);
                        }
                        Item::Edit => {
                            let r = Rect::from_min_size(pos2(left, bottom - s), vec2(s, s));
                            dock_edit_cell(ui, r.shrink(6.0), editing);
                            super::app_tour::mark(ui.ctx(), "nav.dockedit", r);
                        }
                    }
                    left += s + DOCK_SPACE;
                }
                // the one being dragged: follows the pointer along the dock,
                // and the others make room as it passes them
                if let Some(a) = dragging {
                    let held = ui.input(|i| i.pointer.primary_down());
                    match ui.input(|i| i.pointer.hover_pos()).filter(|_| held) {
                        Some(q) => {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            let x = q.x.clamp(frame.min.x + DOCK_PAD + cap_room + DOCK_CELL / 2.0, frame.max.x - DOCK_PAD - DOCK_CELL / 2.0);
                            let want = centres.iter().filter(|(b, c)| *b != a && *c < x).count();
                            if let Some(now) = pins.iter().position(|b| *b == a) {
                                if want < now {
                                    self.cfg.apps.move_pin(a, -1);
                                    self.dirty = true;
                                } else if want > now {
                                    self.cfg.apps.move_pin(a, 1);
                                    self.dirty = true;
                                }
                            }
                            let y = slot.map_or(bottom - DOCK_CELL, |r| r.min.y) - 4.0;
                            let g = Rect::from_min_size(pos2(x - DOCK_CELL / 2.0, y), vec2(DOCK_CELL, DOCK_CELL));
                            p.add(kit::shadow_tier(3).as_shape(g, 12.0));
                            p.rect_filled(g, 12.0, theme::blend(kit::CARD(), TEXT(), 0.06));
                            let cs = (DOCK_CELL * 0.72).round();
                            kit::icon_chip(&p, Rect::from_center_size(g.center(), vec2(cs, cs)), app_icon(a), app_col(a), false);
                            ui.ctx().data_mut(|d| d.insert_temp(drag_id, a));
                            ui.ctx().request_repaint();
                        }
                        None => {
                            // let go: it stays where it is now
                            ui.ctx().data_mut(|d| d.remove::<AppId>(drag_id));
                        }
                    }
                } else {
                    ui.ctx().data_mut(|d| d.remove::<AppId>(drag_id));
                }
                if plus {
                    self.hunt_capsule(ui, cap, shown > 0.5);
                }
            });
        }
        if plus && shown <= 0.01 {
            // the dock is away: the capsule by itself, and beside it a button
            // that says your apps are down here (hover it: up comes the dock)
            let apps_r = Rect::from_min_size(pos2(cap.max.x + APPS_GAP, cap.min.y), vec2(DOCK_CELL, DOCK_CELL));
            let pins_now = self.cfg.apps.pin_list();
            egui::Area::new(egui::Id::new("hunt-capsule"))
                .order(egui::Order::Middle)
                .constrain(false)
                .fixed_pos(cap.min)
                .show(ctx, |ui| {
                    ui.allocate_exact_size(vec2(apps_r.max.x - cap.min.x, cap.height()), Sense::hover());
                    self.hunt_capsule(ui, cap, false);
                    if dock_apps_button(ui, apps_r, &pins_now) {
                        let now = ui.input(|i| i.time);
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("dock-shown-until"), now + 0.8));
                        ui.ctx().request_repaint();
                    }
                });
        }
        if near {
            ctx.request_repaint();
        }
    }

    /// Modern Plus: the hunt on the dock. The target, its count and the
    /// Start overlay button (the only one in this look); click the rest of it
    /// to open Shiny Hunter. It stays on screen when the dock hides.
    fn hunt_capsule(&mut self, ui: &mut egui::Ui, r: Rect, docked: bool) {
        let p = ui.painter().clone();
        let resp = ui.interact(r, ui.id().with("hunt-capsule"), Sense::click());
        let hov = resp.hovered();
        if !docked {
            p.add(kit::shadow_tier(3).as_shape(r, 16.0));
        }
        p.rect_filled(r, 16.0, theme::blend(kit::CARD(), kit::primary(), if hov { 0.32 } else { 0.24 }));
        p.rect_stroke(r, 16.0, Stroke::new(1.5f32, theme::blend(LINE(), kit::primary(), 0.85)));
        // the target
        let sc = pos2(r.min.x + 25.0, r.center().y);
        p.circle_filled(sc, 18.0, theme::blend(kit::CARD(), TEXT(), 0.07));
        if let Some(i) = self.target_idx() {
            let sp = self.species[i].clone();
            self.draw_sprite_centered(&p, &sp, false, sc, vec2(34.0, 34.0));
        }
        let running = self.any_overlay();
        let minimised = self.overlay_on && self.ov_hidden;
        if running {
            p.circle_filled(sc + vec2(13.0, -13.0), 5.0, kit::CARD());
            p.circle_filled(sc + vec2(13.0, -13.0), 3.5, if minimised { theme::HP_YELLOW } else { theme::GREEN });
        }
        // the count and who it's for
        let tx = r.min.x + 50.0;
        let clip = p.with_clip_rect(Rect::from_min_max(pos2(tx, r.min.y), pos2(r.max.x - 128.0, r.max.y)));
        clip.text(pos2(tx, r.center().y - 7.0), Align2::LEFT_CENTER, theme::thousands(self.cfg.count), crate::fonts::sans_bold_font(15.5), TEXT());
        clip.text(pos2(tx, r.center().y + 9.0), Align2::LEFT_CENTER, self.target_label(), crate::fonts::sans_font(10.5), MUTED());
        // Start overlay / the overlay menu / bring a minimised overlay back,
        // in words
        let quiet = running && !minimised;
        let word = if minimised { "Show overlay" } else if running { "Overlay on" } else { "Start overlay" };
        let g = ui.painter().layout_no_wrap(word.to_string(), crate::fonts::sans_bold_font(12.5), TEXT());
        let bw = g.size().x + 36.0;
        let br = Rect::from_min_size(pos2(r.max.x - 8.0 - bw, r.center().y - 15.0), vec2(bw, 30.0));
        let bresp = ui.interact(br, ui.id().with("hunt-capsule-go"), Sense::click());
        let bh = bresp.hovered();
        let fill = if quiet {
            theme::blend(kit::CARD(), TEXT(), if bh { 0.18 } else { 0.11 })
        } else if bh {
            kit::primary_hover()
        } else {
            kit::primary()
        };
        p.rect_filled(br, 15.0, fill);
        let ink = if quiet { TEXT() } else { kit::on_primary() };
        let c = pos2(br.min.x + 16.0, br.center().y);
        if quiet {
            p.rect_filled(Rect::from_center_size(c, vec2(8.0, 8.0)), 1.5, ink);
        } else {
            p.add(egui::Shape::convex_polygon(vec![c + vec2(-3.0, -5.0), c + vec2(5.0, 0.0), c + vec2(-3.0, 5.0)], ink, Stroke::NONE));
        }
        p.text(pos2(br.min.x + 26.0, br.center().y), Align2::LEFT_CENTER, word, crate::fonts::sans_bold_font(12.5), ink);
        super::app_tour::mark(ui.ctx(), "top.start", br);
        let tip = if minimised {
            "The overlay is minimised and still counting. Bring it back."
        } else if running {
            "The overlay menu: stop the overlay, or change what's open"
        } else {
            "Start overlay: pick which overlays to open on top of the game"
        };
        if bresp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked() {
            if minimised {
                self.ov_hidden = false;
            } else {
                self.start_overlay();
            }
        } else if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Your hunt: open Shiny Hunter").clicked() {
            self.open_app(AppId::Shiny);
        }
    }

    /// One pin.
    fn pin_cell(&mut self, ui: &mut egui::Ui, r: Rect, a: AppId, small: bool, tip_below: bool, pinned: bool) {
        let resp = ui.interact(r, ui.id().with(("pin", a, small)), Sense::click());
        let here = app_of(self.setup.tab) == Some(a);
        let open = self.cfg.apps.is_open(a);
        let hov = resp.hovered();
        let p = ui.painter();
        let fill = if here { theme::blend(SIDEBAR(), TEXT(), 0.09) } else if hov { theme::blend(SIDEBAR(), TEXT(), 0.06) } else if small { Color32::TRANSPARENT } else { theme::blend(SIDEBAR(), TEXT(), 0.025) };
        p.rect_filled(r, 12.0, fill);
        if here {
            p.rect_stroke(r, 12.0, Stroke::new(1.0f32, theme::blend(SIDEBAR(), app_col(a), 0.7)));
        }
        let s = if small { (r.height() * 0.72).round() } else { 30.0 };
        kit::icon_chip(p, Rect::from_center_size(r.center() - vec2(0.0, if small { 1.0 } else { 2.0 }), vec2(s, s)), app_icon(a), app_col(a), false);
        if open {
            let w = if small { 10.0 } else { 14.0 };
            p.rect_filled(Rect::from_center_size(pos2(r.center().x, r.max.y - 3.0), vec2(w, 3.0)), 1.5, OPEN_DASH);
        }
        if hov {
            name_tip(ui, r, a.name(), tip_below);
        }
        if resp.clicked() {
            self.open_app(a);
        }
        if resp.middle_clicked() && open {
            self.close_app(a);
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).context_menu(|ui| {
            ui.label(egui::RichText::new(a.name()).strong());
            ui.separator();
            if open && ui.button("Close").clicked() {
                self.close_app(a);
                ui.close_menu();
            }
            if !open && ui.button("Open").clicked() {
                self.open_app(a);
                ui.close_menu();
            }
            let dock = self.pins_place() == PinsPlace::Dock;
            if pinned {
                if ui.button("Move left").clicked() {
                    self.cfg.apps.move_pin(a, -1);
                    self.dirty = true;
                    ui.close_menu();
                }
                if ui.button("Move right").clicked() {
                    self.cfg.apps.move_pin(a, 1);
                    self.dirty = true;
                    ui.close_menu();
                }
                if ui.button(if dock { "Remove from dock" } else { "Unpin" }).clicked() {
                    self.cfg.apps.unpin(a);
                    self.dirty = true;
                    ui.close_menu();
                }
            } else if self.cfg.apps.pin_list().len() < self.cfg.apps.max_pins() && ui.button(if dock { "Keep in dock" } else { "Pin" }).clicked() {
                self.cfg.apps.pin(a);
                self.dirty = true;
                ui.close_menu();
            }
            if ui.button("Store page").clicked() {
                self.show_store_page(a);
                ui.close_menu();
            }
        });
    }

    /// In the dock's edit mode (the pencil): a pin button at each app's top
    /// left that keeps it in the dock (or takes it out), and on an open app
    /// that isn't pinned, a ✕ at its top right that closes it.
    fn dock_badges(&mut self, ui: &mut egui::Ui, r: Rect, a: AppId, pinned: bool) {
        if !dock_editing(ui.ctx()) {
            return;
        }
        let p = ui.painter().clone();
        let size = vec2(2.0 * DOCK_BADGE + 2.0, 2.0 * DOCK_BADGE + 2.0);
        let can_pin = pinned || self.cfg.apps.pin_list().len() < self.cfg.apps.max_pins();
        if can_pin {
            let c = pos2(r.min.x + 6.0, r.min.y + 6.0);
            let resp = ui.interact(Rect::from_center_size(c, size), ui.id().with(("dock-pin", a)), Sense::click());
            let hov = resp.hovered();
            let ink = if hov { kit::primary() } else { TEXT() };
            badge(&p, c, if hov { theme::blend(kit::CARD(), kit::primary(), 0.16) } else { kit::CARD() });
            pin_glyph(&p, c, ink, pinned);
            if hov {
                name_tip(ui, r, if pinned { "Remove from dock" } else { "Keep in dock" }, false);
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if pinned {
                    self.cfg.apps.unpin(a);
                } else {
                    self.cfg.apps.pin(a);
                }
                self.dirty = true;
            }
        }
        if !pinned && self.cfg.apps.is_open(a) {
            let c = pos2(r.max.x - 6.0, r.min.y + 6.0);
            let resp = ui.interact(Rect::from_center_size(c, size), ui.id().with(("dock-close", a)), Sense::click());
            let hov = resp.hovered();
            let red = Color32::from_rgb(255, 110, 140);
            badge(&p, c, if hov { theme::blend(kit::CARD(), red, 0.28) } else { kit::CARD() });
            let ink = if hov { Color32::from_rgb(255, 150, 170) } else { TEXT() };
            let st = Stroke::new(1.5f32, ink);
            let d = 3.2;
            p.line_segment([c - vec2(d, d), c + vec2(d, d)], st);
            p.line_segment([c + vec2(-d, d), c + vec2(d, -d)], st);
            if hov {
                name_tip(ui, r, &format!("Close {}", a.name()), false);
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.close_app(a);
            }
        }
    }

    /// An empty slot: "+" lists apps to pin.
    fn pin_add_cell(&mut self, ui: &mut egui::Ui, r: Rect, first: bool, slot: usize, above: bool) {
        let resp = ui.interact(r, ui.id().with(("pin-add", slot)), Sense::click());
        let p = ui.painter();
        let hov = resp.hovered();
        let a = if first || hov { 1.0 } else { 0.4 };
        dashed_round(p, r.shrink(1.0), theme::blend(SIDEBAR(), TEXT(), if hov { 0.35 } else { 0.16 }).gamma_multiply(a));
        p.text(r.center(), Align2::CENTER_CENTER, "+", crate::fonts::sans_font(18.0), theme::blend(SIDEBAR(), TEXT(), if hov { 0.85 } else { 0.4 }).gamma_multiply(a));
        if hov {
            name_tip(ui, r, if above { "Add an app to the dock" } else { "Pin an app" }, !above && r.height() < 40.0);
        }
        let installed = self.cfg.apps.installed();
        let pinned = self.cfg.apps.pin_list();
        let mut pick: Option<AppId> = None;
        let popup = ui.id().with(("pin-pick", slot));
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            ui.memory_mut(|m| m.toggle_popup(popup));
        }
        let anchor = ui.interact(r, popup.with("anchor"), Sense::hover());
        let side = if above { egui::AboveOrBelow::Above } else { egui::AboveOrBelow::Below };
        egui::popup::popup_above_or_below_widget(ui, popup, &anchor, side, egui::popup::PopupCloseBehavior::CloseOnClickOutside, |ui| {
            ui.set_min_width(200.0);
            let head = if above { "ADD TO THE DOCK" } else { "PIN AN APP" };
            ui.label(egui::RichText::new(format!("{head}  {}/{}", pinned.len(), self.cfg.apps.max_pins())).font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
            ui.add_space(4.0);
            let can: Vec<AppId> = installed.iter().copied().filter(|a| !pinned.contains(a)).collect();
            if can.is_empty() {
                ui.label(egui::RichText::new("All your apps are pinned.").color(MUTED()));
            }
            for a in can {
                let (row, rr) = ui.allocate_exact_size(vec2(200.0, 32.0), Sense::click());
                if rr.hovered() {
                    ui.painter().rect_filled(row, 8.0, CARD_HOVER());
                }
                kit::icon_chip(ui.painter(), Rect::from_center_size(pos2(row.min.x + 16.0, row.center().y), vec2(22.0, 22.0)), app_icon(a), app_col(a), false);
                ui.painter().text(pos2(row.min.x + 36.0, row.center().y), Align2::LEFT_CENTER, a.name(), crate::fonts::sans_font(13.0), TEXT());
                if self.cfg.apps.is_open(a) {
                    // already in the dock as an open app: pinning keeps it there
                    ui.painter().text(pos2(row.max.x - 8.0, row.center().y), Align2::RIGHT_CENTER, "open", crate::fonts::sans_font(11.0), OPEN_DASH);
                }
                if rr.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some(a);
                    ui.memory_mut(|m| m.close_popup());
                }
            }
        });
        if let Some(a) = pick {
            self.cfg.apps.pin(a);
            self.dirty = true;
        }
    }
}

/// The app's name over (or beside) a pin, straight away (no tooltip delay).
fn name_tip(ui: &egui::Ui, r: Rect, name: &str, below: bool) {
    let at = if below { pos2(r.center().x, r.max.y + 8.0) } else { pos2(r.center().x, r.min.y - 8.0) };
    egui::Area::new(egui::Id::new("pin-name-tip"))
        .order(egui::Order::Tooltip)
        .interactable(false)
        .fixed_pos(at)
        .pivot(if below { Align2::CENTER_TOP } else { Align2::CENTER_BOTTOM })
        .show(ui.ctx(), |ui| {
            let g = ui.painter().layout_no_wrap(name.to_string(), crate::fonts::sans_bold_font(12.0), TEXT());
            let (tr, _) = ui.allocate_exact_size(g.size() + vec2(16.0, 10.0), Sense::hover());
            let p = ui.painter();
            p.rect_filled(tr, 8.0, kit::CARD());
            p.rect_stroke(tr, 8.0, Stroke::new(1.0f32, kit::CONTROL()));
            p.galley(tr.min + vec2(8.0, 5.0), g, TEXT());
        });
}

/// A round button on a dock icon's corner.
fn badge(p: &egui::Painter, c: egui::Pos2, fill: Color32) {
    p.circle_filled(c + vec2(0.0, 1.0), DOCK_BADGE + 0.5, Color32::from_black_alpha(60));
    p.circle_filled(c, DOCK_BADGE, fill);
    p.circle_stroke(c, DOCK_BADGE, Stroke::new(1.0f32, kit::CONTROL()));
}

/// A little push pin; struck through for "remove from dock".
fn pin_glyph(p: &egui::Painter, c: egui::Pos2, ink: Color32, struck: bool) {
    let st = Stroke::new(1.3f32, ink);
    p.line_segment([pos2(c.x - 2.8, c.y - 4.0), pos2(c.x + 2.8, c.y - 4.0)], st);
    p.rect_filled(Rect::from_min_max(pos2(c.x - 1.7, c.y - 4.0), pos2(c.x + 1.7, c.y + 0.6)), 0.5, ink);
    p.line_segment([pos2(c.x - 3.6, c.y + 0.8), pos2(c.x + 3.6, c.y + 0.8)], st);
    p.line_segment([pos2(c.x, c.y + 0.8), pos2(c.x, c.y + 4.6)], st);
    if struck {
        p.line_segment([pos2(c.x - 4.6, c.y - 4.6), pos2(c.x + 4.6, c.y + 4.6)], Stroke::new(3.0f32, kit::CARD()));
        p.line_segment([pos2(c.x - 4.6, c.y - 4.6), pos2(c.x + 4.6, c.y + 4.6)], st);
    }
}

/// A dashed rounded outline (straight dashes; the corners stay open).
fn dashed_round(p: &egui::Painter, r: Rect, col: Color32) {
    let s = Stroke::new(1.0f32, col);
    let k = 8.0;
    for seg in [
        [pos2(r.min.x + k, r.min.y), pos2(r.max.x - k, r.min.y)],
        [pos2(r.max.x, r.min.y + k), pos2(r.max.x, r.max.y - k)],
        [pos2(r.max.x - k, r.max.y), pos2(r.min.x + k, r.max.y)],
        [pos2(r.min.x, r.max.y - k), pos2(r.min.x, r.min.y + k)],
    ] {
        p.extend(egui::Shape::dashed_line(&seg, s, 5.0, 4.0));
    }
}

impl App {
    /// Settings → Appearance: where pinned apps go, each choice drawn as a
    /// little window so you can see what you'd get.
    pub(super) fn pins_place_picker(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Pinned apps").font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
        ui.label(egui::RichText::new("Apps you open often, one click away: up to 8 in the sidebar or top bar, up to 15 in the bottom dock. Right-click a pin to move or unpin it. The bottom dock also holds your open apps, so they leave the sidebar; hover one there to pin, unpin or close it.").size(kit::fs(12.0)).color(MUTED()));
        // Modern Plus lives on the bottom dock: the sidebar and top bar are
        // for Modern only
        let plus = self.cfg.appearance.plus;
        if plus {
            ui.add_space(6.0);
            kit::pill(ui, "Modern Plus uses the bottom dock. To pin apps in the sidebar or top bar, switch the Look above to Modern first.", kit::PillKind::Info);
        }
        ui.add_space(10.0);
        let w = ui.available_width();
        let tw = ((w - 24.0) / 3.0).floor();
        let cur = self.cfg.apps.pins_place;
        let mut pick: Option<PinsPlace> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            for (place, title, sub) in [
                (PinsPlace::Dock, "At the bottom", "Recommended: a floating dock with every app"),
                (PinsPlace::Sidebar, "In the sidebar", "Not recommended: a 2 × 4 grid"),
                (PinsPlace::TopBar, "In the top bar", "Not recommended: next to the page name"),
            ] {
                // (in Modern Plus only the dock can be picked)
                let off = plus && place != PinsPlace::Dock;
                let (r, resp) = ui.allocate_exact_size(vec2(tw, 196.0), if off { Sense::hover() } else { Sense::click() });
                let on = cur == place;
                let hov = resp.hovered() && !off;
                let p = ui.painter().clone();
                p.rect_filled(r, 16.0, if on { theme::blend(kit::CARD(), kit::primary(), 0.1) } else { kit::sunken() });
                p.rect_stroke(r, 16.0, Stroke::new(if on { 2.0f32 } else { 1.0f32 }, if on { kit::primary() } else if hov { kit::CONTROL() } else { LINE() }));
                mini_window(&p, Rect::from_min_size(r.min + vec2(14.0, 14.0), vec2(r.width() - 28.0, 120.0)), place);
                p.text(pos2(r.min.x + 16.0, r.max.y - 46.0), Align2::LEFT_TOP, title, crate::fonts::sans_bold_font(13.5), TEXT());
                p.text(pos2(r.min.x + 16.0, r.max.y - 26.0), Align2::LEFT_TOP, sub, crate::fonts::sans_font(12.0), MUTED());
                let cb = Rect::from_center_size(pos2(r.max.x - 26.0, r.max.y - 30.0), vec2(20.0, 20.0));
                p.circle_stroke(cb.center(), 9.0, Stroke::new(1.5f32, if on { kit::primary() } else { kit::CONTROL() }));
                if on {
                    p.circle_filled(cb.center(), 5.0, kit::primary());
                }
                if off {
                    // greyed out, with why
                    p.rect_filled(r, 16.0, theme::blend(kit::CARD(), kit::PAGE(), 0.5).gamma_multiply(0.75));
                    p.text(pos2(r.center().x, r.min.y + 74.0), Align2::CENTER_CENTER, "Modern only", crate::fonts::sans_bold_font(12.5), MUTED());
                    resp.on_hover_cursor(egui::CursorIcon::NotAllowed).on_hover_text("Not in Modern Plus: switch the Look to Modern to pin apps here.");
                } else if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some(place);
                }
            }
        });
        if let Some(pl) = pick {
            self.cfg.apps.pins_place = pl;
            self.dirty = true;
        }
        ui.add_space(12.0);
        let mut mag = self.cfg.apps.dock_magnify;
        if kit::toggle_row(
            ui,
            "Magnifying dock",
            "Icons in the dock swell as your pointer passes over them, their neighbours a little less. It works with either dock, so turning it on with the pins in the sidebar moves them to the bottom dock.",
            &mut mag,
        ) {
            self.cfg.apps.dock_magnify = mag;
            if mag && self.cfg.apps.pins_place == PinsPlace::Sidebar {
                self.cfg.apps.pins_place = PinsPlace::Dock;
            }
            self.dirty = true;
        }
    }
}

/// A tiny window showing where the pins go.
fn mini_window(p: &egui::Painter, r: Rect, place: PinsPlace) {
    let page = kit::PAGE();
    let side = SIDEBAR();
    let line = theme::blend(side, TEXT(), 0.18);
    p.rect_filled(r, 10.0, page);
    let sw = (r.width() * 0.3).round();
    let sr = Rect::from_min_size(r.min, vec2(sw, r.height()));
    p.rect_filled(sr, egui::Rounding { nw: 10.0, sw: 10.0, ..Default::default() }, side);
    p.line_segment([pos2(sr.max.x, r.min.y + 22.0), pos2(r.max.x, r.min.y + 22.0)], Stroke::new(1.0f32, theme::blend(page, TEXT(), 0.08)));
    let cols = [kit::C_VIOLET, kit::C_GREEN, kit::C_RED, kit::C_INDIGO, kit::C_ORANGE, kit::C_TEAL];
    // the sidebar's fixed rows
    let mut y = r.min.y + 12.0;
    for k in 0..4 {
        p.rect_filled(Rect::from_min_size(pos2(sr.min.x + 8.0, y), vec2(7.0, 7.0)), 2.0, line);
        p.rect_filled(Rect::from_min_size(pos2(sr.min.x + 19.0, y + 2.0), vec2(sw * (0.45 - k as f32 * 0.04), 3.0)), 1.5, line);
        y += 13.0;
    }
    // the page name in the top bar
    let crumb = Rect::from_min_size(pos2(sr.max.x + 10.0, r.min.y + 9.0), vec2(28.0, 4.0));
    p.rect_filled(crumb, 2.0, theme::blend(page, TEXT(), 0.4));
    match place {
        PinsPlace::Sidebar => {
            let g = 3.0;
            let cell = ((sw - 16.0 - g * 3.0) / 4.0).floor();
            for k in 0..8 {
                let (cx, cy) = (k % 4, k / 4);
                let cr = Rect::from_min_size(pos2(sr.min.x + 8.0 + cx as f32 * (cell + g), y + 4.0 + cy as f32 * (cell + g)), vec2(cell, cell));
                if k < 6 {
                    p.rect_filled(cr, 3.0, cols[k].gamma_multiply(0.85));
                } else {
                    p.rect_stroke(cr, 3.0, Stroke::new(1.0f32, line));
                }
            }
            y += 4.0 + 2.0 * (cell + g) + 6.0;
            for k in 0..2 {
                p.rect_filled(Rect::from_min_size(pos2(sr.min.x + 8.0, y), vec2(7.0, 7.0)), 2.0, line);
                p.rect_filled(Rect::from_min_size(pos2(sr.min.x + 19.0, y + 2.0), vec2(sw * 0.4, 3.0)), 1.5, line);
                y += 13.0 + k as f32;
            }
        }
        PinsPlace::Dock => {
            // no pins or open apps in the sidebar; a dock floats at the bottom
            let n = 7;
            let cw = 10.0;
            let dw = n as f32 * (cw + 3.0) + 7.0;
            let cx = (sr.max.x + r.max.x) / 2.0;
            let dock = Rect::from_min_size(pos2(cx - dw / 2.0, r.max.y - 20.0), vec2(dw, 15.0));
            p.rect_filled(dock, 5.0, theme::blend(page, TEXT(), 0.12));
            p.rect_stroke(dock, 5.0, Stroke::new(1.0f32, theme::blend(page, TEXT(), 0.22)));
            let mut x = dock.min.x + 5.0;
            for k in 0..n {
                p.rect_filled(Rect::from_min_size(pos2(x, dock.min.y + 2.5), vec2(cw, cw)), 3.0, cols[k % cols.len()].gamma_multiply(0.9));
                x += cw + 3.0;
            }
        }
        PinsPlace::TopBar => {
            let mut x = crumb.max.x + 8.0;
            let dock = Rect::from_min_size(pos2(x - 3.0, r.min.y + 4.0), vec2(6.0 * 13.0 + 3.0, 14.0));
            p.rect_stroke(dock, 5.0, Stroke::new(1.0f32, theme::blend(page, TEXT(), 0.15)));
            for c in cols {
                p.rect_filled(Rect::from_min_size(pos2(x, r.min.y + 6.0), vec2(10.0, 10.0)), 3.0, c.gamma_multiply(0.85));
                x += 13.0;
            }
            for k in 0..2 {
                p.rect_filled(Rect::from_min_size(pos2(sr.min.x + 8.0, y + 6.0), vec2(7.0, 7.0)), 2.0, line);
                p.rect_filled(Rect::from_min_size(pos2(sr.min.x + 19.0, y + 8.0), vec2(sw * 0.4, 3.0)), 1.5, line);
                y += 13.0 + k as f32;
            }
        }
    }
    // the page's cards
    let body = Rect::from_min_max(pos2(sr.max.x + 10.0, r.min.y + 30.0), r.max - vec2(10.0, if place == PinsPlace::Dock { 26.0 } else { 10.0 }));
    let half = (body.width() - 6.0) / 2.0;
    p.rect_filled(Rect::from_min_size(body.min, vec2(body.width(), 26.0)), 5.0, theme::blend(page, TEXT(), 0.07));
    p.rect_filled(Rect::from_min_size(body.min + vec2(0.0, 32.0), vec2(half, body.height() - 32.0)), 5.0, theme::blend(page, TEXT(), 0.05));
    p.rect_filled(Rect::from_min_size(body.min + vec2(half + 6.0, 32.0), vec2(half, body.height() - 32.0)), 5.0, theme::blend(page, TEXT(), 0.05));
}

/// Whether the dock is in edit mode (the pencil).
fn dock_editing(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new("dock-edit"))).unwrap_or(false)
}

fn set_dock_editing(ctx: &egui::Context, on: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("dock-edit"), on));
}

/// The pencil at the dock's right end: edit mode on and off. While it's on
/// it turns into a violet "done" tick.
fn dock_edit_cell(ui: &mut egui::Ui, r: Rect, editing: bool) {
    let resp = ui.interact(r, ui.id().with("dock-edit"), Sense::click());
    let hov = resp.hovered();
    let p = ui.painter();
    let fill = if editing {
        if hov {
            kit::primary_hover()
        } else {
            kit::primary()
        }
    } else if hov {
        theme::blend(SIDEBAR(), TEXT(), 0.1)
    } else {
        Color32::TRANSPARENT
    };
    p.rect_filled(r, 10.0, fill);
    let c = r.center();
    if editing {
        // a tick
        let st = Stroke::new(2.2f32, kit::on_primary());
        p.line_segment([c + vec2(-6.0, 0.5), c + vec2(-2.0, 4.5)], st);
        p.line_segment([c + vec2(-2.0, 4.5), c + vec2(6.0, -4.5)], st);
    } else {
        // a pencil: its body, tip and the end
        let ink = theme::blend(SIDEBAR(), TEXT(), if hov { 0.95 } else { 0.62 });
        let d = vec2(1.0, -1.0) / 2f32.sqrt();
        let n = vec2(1.0, 1.0) / 2f32.sqrt();
        let a = c + vec2(-2.6, 2.6);
        let b = c + vec2(5.2, -5.2);
        p.line_segment([a, b], Stroke::new(4.0f32, ink));
        p.add(egui::Shape::convex_polygon(vec![a + n * 2.0, a - n * 2.0, a - d * 3.4], ink, Stroke::NONE));
        p.line_segment([b - d * 2.2 + n * 2.0, b - d * 2.2 - n * 2.0], Stroke::new(1.2f32, theme::blend(SIDEBAR(), TEXT(), 0.0)));
    }
    if hov {
        name_tip(ui, r, if editing { "Done" } else { "Edit the dock: pin, unpin, close or drag apps into order" }, false);
    }
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        set_dock_editing(ui.ctx(), !editing);
    }
}

/// Modern Plus, with the dock tucked away: a button beside the hunt pill
/// showing your first few apps in little coloured squares, so it's plain
/// they're down here. True while it's hovered or clicked (the dock comes up).
fn dock_apps_button(ui: &mut egui::Ui, r: Rect, pins: &[AppId]) -> bool {
    let resp = ui.interact(r, ui.id().with("dock-apps-hint"), Sense::click());
    let hov = resp.hovered();
    let p = ui.painter();
    p.add(kit::shadow_tier(3).as_shape(r, 16.0));
    p.rect_filled(r, 16.0, theme::blend(kit::CARD(), kit::primary(), if hov { 0.2 } else { 0.1 }));
    p.rect_stroke(r, 16.0, Stroke::new(1.2f32, theme::blend(LINE(), kit::primary(), 0.6)));
    // four little app squares (your first four apps' colours)
    let sq = 9.0;
    let gap = 4.0;
    let o = r.center() - vec2(sq + gap / 2.0, sq + gap / 2.0) + vec2(0.0, 3.0);
    for k in 0..4 {
        let c = pins.get(k).map_or(theme::blend(kit::CARD(), TEXT(), 0.25), |a| app_col(*a));
        let q = Rect::from_min_size(o + vec2((k % 2) as f32 * (sq + gap), (k / 2) as f32 * (sq + gap)), vec2(sq, sq));
        p.rect_filled(q, 2.5, c);
    }
    // a small arrow up: the dock comes up from here
    let tip = pos2(r.center().x, r.min.y + 7.0);
    let ink = theme::blend(kit::CARD(), TEXT(), if hov { 0.9 } else { 0.55 });
    p.line_segment([tip + vec2(-4.0, 3.0), tip], Stroke::new(1.6f32, ink));
    p.line_segment([tip, tip + vec2(4.0, 3.0)], Stroke::new(1.6f32, ink));
    let n = pins.len();
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("Your apps ({n}): they're on the dock. Hover here, or move to the bottom of the window, to bring it up."));
    super::app_tour::mark(ui.ctx(), "nav.dockhint", r);
    hov || resp.clicked()
}
