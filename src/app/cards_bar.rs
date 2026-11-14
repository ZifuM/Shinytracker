//! The Card Collector's XP bar: a slim always-on-top strip pinned under the
//! shiny overlay, at its left or right end and a third of its width. It
//! follows the overlay wherever it goes; it can't be dragged off on its
//! own. Click it to open the Card Collector.

use super::cards_app::xp_bar;
use super::{App, Tab};
use crate::cards::XP_PER_PACK;
use crate::theme::GOLD;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use std::time::{Duration, Instant};

/// The bar's height and the gap under the overlay.
pub(super) const BAR_H: f32 = 24.0;
pub(super) const GAP: f32 = 4.0;
/// The narrowest it gets (under the small badge, say), so it stays readable.
const MIN_W: f32 = 150.0;

#[derive(Default)]
pub(super) struct BarState {
    /// Where the window opened, and its size.
    geom: Option<([f32; 2], [f32; 2])>,
    /// The shiny overlay's window this frame (None when it isn't showing).
    pub shiny: Option<Rect>,
    applied: Option<Rect>,
    applied_pass: Option<bool>,
    keeper: Option<isize>,
    /// The bar is on this frame (installed, ticked on the Start overlay
    /// screen, overlay running): pop-ups under the overlay make room for it.
    pub on: bool,
    /// The overlay's own height, without the room under it for pop-ups
    /// (the overlay's units), so the bar pins to the overlay itself and
    /// never drops down with a pop-up. None: the whole window.
    pub card_h: Option<f32>,
}

/// Where the bar goes for the overlay at `s`.
pub(super) fn spot(s: Rect, right: bool) -> Rect {
    let w = (s.width() / 3.0).max(MIN_W).round();
    let x = if right { s.max.x - w } else { s.min.x };
    Rect::from_min_size(pos2(x, s.max.y + GAP), vec2(w, BAR_H))
}

impl App {
    /// Room to leave between the overlay and a pop-up under it while the bar
    /// is there (the overlay's own units).
    pub(super) fn bar_room(&self) -> f32 {
        if self.setup.cards.bar_ui.on {
            (BAR_H + GAP * 2.0) / self.ov_scale()
        } else {
            0.0
        }
    }

    /// The bar's window: shown while the overlay runs (and the app is on).
    pub(super) fn cards_bar_viewport(&mut self, ctx: &egui::Context) {
        let bar = self.setup.cards.file().bar.clone();
        let shiny = self.setup.cards.bar_ui.shiny;
        // Only with Card Collector installed, ticked on the Start overlay
        // screen, and the shiny overlay showing.
        let on = crate::apps::live(crate::apps::AppId::Cards) && bar.on && self.overlay_on && !self.ov_hidden;
        self.setup.cards.bar_ui.on = on;
        let show = on && shiny.is_some();
        let Some(s) = shiny.filter(|_| show) else {
            let b = &mut self.setup.cards.bar_ui;
            b.geom = None;
            b.keeper = None;
            b.applied = None;
            b.applied_pass = None;
            return;
        };
        crate::topmost::ensure_keeper_titled(crate::topmost::CARD_BAR_TITLE, &mut self.setup.cards.bar_ui.keeper, &self.borderless_fix);
        let want = spot(s, bar.right);
        if self.setup.cards.bar_ui.geom.is_none() {
            let b = &mut self.setup.cards.bar_ui;
            b.geom = Some(([want.min.x, want.min.y], [want.width(), want.height()]));
            b.applied = Some(want);
        }
        let Some((pos, size)) = self.setup.cards.bar_ui.geom else { return };
        let builder = egui::ViewportBuilder::default()
            .with_title(crate::topmost::CARD_BAR_TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false)
            .with_position(pos)
            .with_inner_size(size);
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("card-bar"), builder, |ctx, _class| {
            self.cards_bar_follow(ctx, want);
            self.cards_bar_ui(ctx);
        });
    }

    /// Keep it pinned: same spot under the overlay, a third of its width.
    fn cards_bar_follow(&mut self, ctx: &egui::Context, want: Rect) {
        let b = &mut self.setup.cards.bar_ui;
        let here = ctx.input(|i| i.viewport().outer_rect);
        let off = here.map_or(true, |me| (me.min - want.min).length() > 0.5);
        if off || b.applied.map_or(true, |a| (a.min - want.min).length() > 0.5) {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(want.min));
        }
        if b.applied.map_or(true, |a| (a.width() - want.width()).abs() > 0.5) {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(want.size()));
        }
        b.applied = Some(want);
        // click-through when the overlay is
        let pass = self.click_through_active();
        if self.setup.cards.bar_ui.applied_pass != Some(pass) {
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(pass));
            self.setup.cards.bar_ui.applied_pass = Some(pass);
        }
    }

    /// The bar itself.
    fn cards_bar_ui(&mut self, ctx: &egui::Context) {
        let f = self.setup.cards.file().clone();
        let gain = self.setup.cards.gains.back().cloned();
        let pack_at = self.setup.cards.pack_at;
        let mut open = false;
        let retro = self.cfg.overlay.theme == crate::config::OverlayTheme::Retro;
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let r = ui.max_rect().shrink(1.0);
            let resp = ui.interact(r, egui::Id::new("card-bar"), Sense::click());
            let p = ui.painter();
            let t = ui.input(|i| i.time) as f32;
            let fresh_pack = pack_at.is_some_and(|t| t.elapsed() < Duration::from_secs(4));
            paint_bar(p, r, &f, gain.as_ref().map(|g| (&g.0, g.1)), pack_at, false, None, retro, resp.hovered(), t);
            if fresh_pack || gain.is_some() {
                ui.ctx().request_repaint_after(Duration::from_millis(33));
            }
            if resp.on_hover_text("Click to open the Card Collector").clicked() {
                open = true;
            }
        });
        if open {
            self.go_to(Tab::Cards);
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
        }
    }
}

/// The bar's look (also drawn as a preview in the Card Collector).
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_bar(p: &egui::Painter, r: Rect, f: &crate::cards::CardFile, gain: Option<(&crate::cards::Gain, Instant)>, pack_at: Option<Instant>, near: bool, said: Option<&str>, retro: bool, hovered: bool, t: f32) {
    let fresh_pack = pack_at.is_some_and(|t| t.elapsed() < Duration::from_secs(4));
    let msg: Option<String> = if near {
        Some("Let go to snap".into())
    } else if let Some(s) = said {
        Some(if s.starts_with("Snapped") { "Snapped".into() } else { s.to_string() })
    } else if fresh_pack {
        Some("Pack!".into())
    } else {
        gain.filter(|g| g.1.elapsed() < Duration::from_millis(1800)).map(|g| format!("+{} XP", g.0.xp))
    };
    let glow = pack_at.map_or(0.0, |at| {
        let e = at.elapsed().as_secs_f32();
        if e < 2.0 {
            (1.0 - e / 2.0) * (0.5 + 0.5 * (e * 12.0).sin())
        } else {
            0.0
        }
    });
    if retro {
        super::ov_retro::card_xp_strip(p, r, f.xp as f32 / XP_PER_PACK as f32, f.xp, XP_PER_PACK, f.packs, msg.as_deref(), glow, near);
        return;
    }
    p.rect_filled(r, 8.0, Color32::from_rgba_unmultiplied(16, 18, 30, 228));
    let edge = if near { Color32::from_rgb(120, 220, 255) } else if hovered { GOLD.gamma_multiply(0.7) } else { Color32::from_white_alpha(28) };
    p.rect_stroke(r, 8.0, Stroke::new(if near { 2.0f32 } else { 1.0 }, edge));
    // a little card, with how many packs wait
    let card = Rect::from_center_size(pos2(r.min.x + 13.0, r.center().y), vec2(10.0, 14.0));
    p.rect_filled(card.translate(vec2(2.0, -1.5)), 2.0, Color32::from_rgb(54, 86, 196));
    p.rect_filled(card, 2.0, Color32::from_rgb(255, 214, 90));
    if f.packs > 0 {
        let bc = pos2(card.max.x + 3.0, card.min.y + 1.0);
        p.circle_filled(bc, 6.0, Color32::from_rgb(232, 72, 120));
        p.text(bc, Align2::CENTER_CENTER, if f.packs > 9 { "9+".to_string() } else { f.packs.to_string() }, egui::FontId::proportional(8.0), Color32::WHITE);
    }
    let right_w = 60.0;
    let bar = Rect::from_min_max(pos2(r.min.x + 28.0, r.center().y - 3.5), pos2(r.max.x - right_w - 4.0, r.center().y + 3.5));
    xp_bar(p, bar, f.xp as f32 / XP_PER_PACK as f32, t, pack_at);
    let rx = r.max.x - 7.0;
    let (text, col) = match &msg {
        Some(m) => (m.clone(), if near || said.is_some() { Color32::from_rgb(120, 220, 255) } else { GOLD }),
        None => (format!("{} / {}", f.xp, XP_PER_PACK), Color32::from_gray(225)),
    };
    let g = p.layout_no_wrap(text, crate::fonts::sans_bold_font(9.5), col);
    let at = pos2(rx - g.size().x, r.center().y - g.size().y / 2.0);
    if g.size().x > right_w {
        p.rect_filled(Rect::from_min_size(at - vec2(5.0, 2.0), g.size() + vec2(10.0, 4.0)), 5.0, Color32::from_rgba_unmultiplied(16, 18, 30, 235));
    }
    p.galley(at, g, col);
}
