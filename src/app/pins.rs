//! Pinned apps: up to 8 apps for quick opening, either as a 2 × 4 grid in
//! the sidebar or as a dock in the top bar (Settings → Appearance chooses).
//! A pin opens its app; a small dash shows it's open and the app you're in
//! is outlined. Right-click a pin for Close, Unpin, Move and its store page;
//! "+" pins another app.

use super::shell::app_of;
use super::store::{app_col, app_icon};
use super::ui_kit::{self as kit, CARD_HOVER, LINE, MUTED, SIDEBAR, TEXT};
use super::App;
use crate::apps::{AppId, PinsPlace, MAX_PINS};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};

const OPEN_DASH: Color32 = Color32::from_rgb(120, 214, 140);

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
                self.pin_cell(ui, Rect::from_center_size(r.center(), vec2(44.0, 40.0)), *a, false);
                ui.add_space(2.0);
            }
            if pins.len() < MAX_PINS {
                let (r, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
                self.pin_add_cell(ui, Rect::from_center_size(r.center(), vec2(44.0, 36.0)), true);
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
                    self.pin_cell(ui, cell, *a, false);
                } else if i < MAX_PINS {
                    self.pin_add_cell(ui, cell, i == pins.len());
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
        let n = pins.len() + (pins.len() < MAX_PINS) as usize;
        let room = ((max_x - x - 12.0) / (cell + gap)).floor().max(0.0) as usize;
        if room == 0 {
            return x;
        }
        let shown = n.min(room);
        let frame = Rect::from_min_size(pos2(x, cy - cell / 2.0 - 4.0), vec2(shown as f32 * (cell + gap) - gap + 8.0, cell + 8.0));
        let p = ui.painter();
        p.rect_filled(frame, 14.0, theme::blend(kit::PAGE(), TEXT(), 0.035));
        p.rect_stroke(frame, 14.0, Stroke::new(1.0f32, LINE()));
        for k in 0..shown {
            let r = Rect::from_min_size(pos2(frame.min.x + 4.0 + k as f32 * (cell + gap), frame.min.y + 4.0), vec2(cell, cell));
            match pins.get(k) {
                Some(a) => self.pin_cell(ui, r, *a, true),
                None => self.pin_add_cell(ui, r, true),
            }
        }
        frame.max.x
    }

    /// One pin.
    fn pin_cell(&mut self, ui: &mut egui::Ui, r: Rect, a: AppId, small: bool) {
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
        let s = if small { 26.0 } else { 30.0 };
        kit::icon_chip(p, Rect::from_center_size(r.center() - vec2(0.0, if small { 1.0 } else { 2.0 }), vec2(s, s)), app_icon(a), app_col(a), false);
        if open {
            let w = if small { 10.0 } else { 14.0 };
            p.rect_filled(Rect::from_center_size(pos2(r.center().x, r.max.y - 3.0), vec2(w, 3.0)), 1.5, OPEN_DASH);
        }
        if hov {
            name_tip(ui, r, a.name(), small);
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
            if ui.button("Unpin").clicked() {
                self.cfg.apps.unpin(a);
                self.dirty = true;
                ui.close_menu();
            }
            if ui.button("Store page").clicked() {
                self.show_store_page(a);
                ui.close_menu();
            }
        });
    }

    /// An empty slot: "+" lists apps to pin.
    fn pin_add_cell(&mut self, ui: &mut egui::Ui, r: Rect, first: bool) {
        let resp = ui.interact(r, ui.id().with(("pin-add", r.min.x as i32, r.min.y as i32)), Sense::click());
        let p = ui.painter();
        let hov = resp.hovered();
        let a = if first || hov { 1.0 } else { 0.4 };
        dashed_round(p, r.shrink(1.0), theme::blend(SIDEBAR(), TEXT(), if hov { 0.35 } else { 0.16 }).gamma_multiply(a));
        p.text(r.center(), Align2::CENTER_CENTER, "+", crate::fonts::sans_font(18.0), theme::blend(SIDEBAR(), TEXT(), if hov { 0.85 } else { 0.4 }).gamma_multiply(a));
        if hov {
            name_tip(ui, r, "Pin an app", r.height() < 40.0);
        }
        let installed = self.cfg.apps.installed();
        let pinned = self.cfg.apps.pin_list();
        let mut pick: Option<AppId> = None;
        let popup = ui.id().with(("pin-pick", r.min.x as i32, r.min.y as i32));
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            ui.memory_mut(|m| m.toggle_popup(popup));
        }
        let anchor = ui.interact(r, popup.with("anchor"), Sense::hover());
        egui::popup::popup_below_widget(ui, popup, &anchor, egui::popup::PopupCloseBehavior::CloseOnClickOutside, |ui| {
            ui.set_min_width(200.0);
            ui.label(egui::RichText::new(format!("PIN AN APP  {}/{MAX_PINS}", pinned.len())).font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
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
        ui.label(egui::RichText::new("Up to 8 apps you open often, one click away. Right-click a pin to move or unpin it.").size(kit::fs(12.0)).color(MUTED()));
        ui.add_space(10.0);
        let w = ui.available_width();
        let tw = ((w - 12.0) / 2.0).floor();
        let cur = self.cfg.apps.pins_place;
        let mut pick: Option<PinsPlace> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            for (place, title, sub) in [
                (PinsPlace::Sidebar, "In the sidebar", "A 2 × 4 grid under Stats"),
                (PinsPlace::TopBar, "In the top bar", "A dock next to the page name"),
            ] {
                let (r, resp) = ui.allocate_exact_size(vec2(tw, 196.0), Sense::click());
                let on = cur == place;
                let hov = resp.hovered();
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
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some(place);
                }
            }
        });
        if let Some(pl) = pick {
            self.cfg.apps.pins_place = pl;
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
    let body = Rect::from_min_max(pos2(sr.max.x + 10.0, r.min.y + 30.0), r.max - vec2(10.0, 10.0));
    let half = (body.width() - 6.0) / 2.0;
    p.rect_filled(Rect::from_min_size(body.min, vec2(body.width(), 26.0)), 5.0, theme::blend(page, TEXT(), 0.07));
    p.rect_filled(Rect::from_min_size(body.min + vec2(0.0, 32.0), vec2(half, body.height() - 32.0)), 5.0, theme::blend(page, TEXT(), 0.05));
    p.rect_filled(Rect::from_min_size(body.min + vec2(half + 6.0, 32.0), vec2(half, body.height() - 32.0)), 5.0, theme::blend(page, TEXT(), 0.05));
}
