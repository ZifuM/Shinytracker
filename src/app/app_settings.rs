//! An app's own settings: a cog button at the top right of its page opens
//! them in a window over the page. Every app with settings uses this, so
//! they're always in the same place.

use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

/// The round cog button. Flips `open` when clicked.
pub(super) fn cog_button(ui: &mut egui::Ui, open: &mut bool, tip: &str) {
    let (r, resp) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::click());
    let p = ui.painter();
    let hov = resp.hovered() || *open;
    p.circle_filled(r.center(), 19.0, if hov { kit::CARD_HOVER() } else { kit::CARD() });
    p.circle_stroke(r.center(), 18.5, Stroke::new(1.0f32, LINE()));
    p.text(r.center(), Align2::CENTER_CENTER, "⚙", egui::FontId::proportional(18.0), if hov { TEXT() } else { MUTED() });
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked() {
        *open = !*open;
    }
}

/// The settings window, over the page (Esc, ✕ or a click outside closes it).
pub(super) fn window(ctx: &egui::Context, id: &str, title: &str, subtitle: &str, accent: Color32, open: &mut bool, add: impl FnOnce(&mut egui::Ui)) {
    if !*open {
        return;
    }
    let screen = ctx.screen_rect();
    let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    egui::Area::new(egui::Id::new(("app-settings", id))).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        kit::settings_style(ui);
        let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
        ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(160));
        let w = 700.0f32.min(sr.width() - 40.0);
        let h = 880.0f32.min(sr.height() - 60.0);
        let panel = Rect::from_center_size(sr.center(), vec2(w, h));
        let _ = ui.interact(panel, ui.id().with("app-settings-panel"), Sense::click());
        let p = ui.painter().clone();
        p.add(kit::shadow_tier(4).as_shape(panel, 16.0));
        p.rect_filled(panel, 16.0, kit::CARD());
        p.rect_stroke(panel, 16.0, Stroke::new(1.0f32, LINE()));
        // a thin line of the app's colour along the top
        p.rect_filled(Rect::from_min_size(panel.min + vec2(24.0, 0.0), vec2(56.0, 3.0)), 2.0, accent);
        let inner = panel.shrink2(vec2(24.0, 20.0));
        p.text(inner.min, Align2::LEFT_TOP, title, crate::fonts::sans_bold_font(19.0), TEXT());
        p.text(inner.min + vec2(0.0, 28.0), Align2::LEFT_TOP, subtitle, crate::fonts::sans_font(12.5), MUTED());
        // ✕
        let xc = pos2(inner.max.x - 10.0, inner.min.y + 10.0);
        let xr = Rect::from_center_size(xc, vec2(28.0, 28.0));
        let xresp = ui.interact(xr, ui.id().with("app-settings-x"), Sense::click());
        if xresp.hovered() {
            p.rect_filled(xr, 8.0, kit::CARD_HOVER());
        }
        for (a, b) in [(vec2(-5.0, -5.0), vec2(5.0, 5.0)), (vec2(-5.0, 5.0), vec2(5.0, -5.0))] {
            p.line_segment([xc + a, xc + b], Stroke::new(1.6f32, if xresp.hovered() { TEXT() } else { MUTED() }));
        }
        if xresp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Close (Esc)").clicked() {
            close = true;
        }
        let body = Rect::from_min_max(pos2(inner.min.x, inner.min.y + 58.0), inner.max);
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(body.width() - 14.0);
                add(ui);
                ui.add_space(8.0);
            });
        });
        if scrim.clicked() && !panel.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default())) {
            close = true;
        }
    });
    if close {
        *open = false;
    }
}

/// A group heading inside the settings window.
pub(super) fn group(ui: &mut egui::Ui, text: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(text.to_uppercase()).font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
    ui.add_space(2.0);
}

/// A row: label and help on the left, the controls in a column on the right.
pub(super) fn row(ui: &mut egui::Ui, label: &str, help: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(6.0);
    let left = (ui.available_width() * 0.42).clamp(180.0, 300.0);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(vec2(left, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(left);
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(RichText::new(label).size(13.0).color(TEXT()));
            if !help.is_empty() {
                ui.add(egui::Label::new(RichText::new(help).size(11.5).color(MUTED())).wrap());
            }
        });
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            add(ui)
        });
    });
    ui.add_space(6.0);
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, LINE());
}
