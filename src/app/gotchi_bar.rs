//! Pokégotchi on the overlay: its own small always-on-top window, separate
//! from the shiny overlay and the XP bar. Drag it anywhere; it remembers
//! where. Three looks (the mini device, just your shiny, or a slim strip
//! with its needs) in three sizes. Opened from the Start overlay picker.
//!
//! The window is cut to exactly what's drawn (the device's outline, or the
//! Pokémon's own pixels), so nothing of the window shows around it: on some
//! PCs see-through window parts come out black.

use super::gotchi_app::{hop_y, look, meter, mood_col, need_col, paint_device, Act};
use super::{App, Tab};
use crate::gotchi::{Frame, Need, OvCfg, OvStyle};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Pos2, Rect, Sense, Stroke};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct BarState {
    geom: Option<([f32; 2], [f32; 2])>,
    applied: Option<egui::Vec2>,
    keeper: Option<isize>,
    /// The cut last applied to the window (pixels), so it's only redone on change.
    clip: Option<(isize, Vec<i32>)>,
    /// The partner's sprite outline (slug, mask), and when it was last tried.
    mask: Option<(String, Option<crate::sprites::Mask>)>,
    mask_tried: Option<Instant>,
    /// The device's button under the pointer, and held down (last frame).
    btn_hover: Option<usize>,
    btn_down: Option<usize>,
}

/// What was drawn, for cutting the window to it (points).
#[derive(Default)]
pub(super) struct Drawn {
    /// The mini device's buttons and screen.
    pub buttons: Vec<Rect>,
    pub screen: Option<Rect>,
    pub polys: Vec<Vec<Pos2>>,
    pub boxes: Vec<Rect>,
    pub rounded: Vec<(Rect, f32)>,
}

/// `s` cut to `n` characters (with "…"), for the overlay's small bubbles.
fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n - 1).collect::<String>().trim_end())
    }
}

/// The window's size for the device and the strip (the shiny alone sizes to
/// the Pokémon, see `gotchi_ov_size`).
pub(super) fn size_of(ov: &OvCfg) -> egui::Vec2 {
    let f = ov.scale.factor();
    match ov.style {
        OvStyle::Device => vec2(160.0, 196.0) * f,
        OvStyle::Strip => vec2(200.0, 26.0) * f,
        OvStyle::Perch => vec2(64.0, 76.0) * ov.scale.pixels(),
    }
}

/// Room above the shiny alone for its speech bubble and hops.
fn perch_top(ov: &OvCfg) -> f32 {
    22.0 + 6.0 * ov.scale.pixels()
}

impl App {
    /// The partner's sprite outline (read once it's downloaded).
    fn gotchi_mask(&mut self, slug: &str) -> Option<&crate::sprites::Mask> {
        let b = &mut self.setup.gotchi.bar_ui;
        let fresh = b.mask.as_ref().map_or(true, |(s, _)| s != slug);
        let retry = b.mask.as_ref().is_some_and(|(_, m)| m.is_none()) && b.mask_tried.map_or(true, |t| t.elapsed() > Duration::from_secs(2));
        if fresh || retry {
            b.mask = Some((slug.to_string(), crate::sprites::mask(slug, true)));
            b.mask_tried = Some(Instant::now());
        }
        b.mask.as_ref().and_then(|(_, m)| m.as_ref())
    }

    /// The overlay window's size for its look and scale.
    pub(super) fn gotchi_ov_size(&mut self, ov: &OvCfg) -> egui::Vec2 {
        if ov.style != OvStyle::Perch {
            return size_of(ov);
        }
        let slug = self.setup.gotchi.file().partner().map(|p| p.slug.clone()).unwrap_or_default();
        let k = ov.scale.pixels();
        match self.gotchi_mask(&slug) {
            Some(m) => {
                let (bw, bh) = ((m.bbox[2] - m.bbox[0]) as f32, (m.bbox[3] - m.bbox[1]) as f32);
                vec2((bw * k).max(70.0) + 8.0, bh * k + perch_top(ov))
            }
            None => size_of(ov),
        }
    }

    pub(super) fn gotchi_viewport(&mut self, ctx: &egui::Context) {
        let on = crate::apps::live(crate::apps::AppId::Gotchi) && self.setup.gotchi.shown;
        let has_pet = on && self.setup.gotchi.file().partner().is_some();
        if !has_pet {
            let b = &mut self.setup.gotchi.bar_ui;
            b.geom = None;
            b.keeper = None;
            b.applied = None;
            b.clip = None;
            return;
        }
        let ov = self.setup.gotchi.file().overlay.clone();
        let size = self.gotchi_ov_size(&ov);
        crate::topmost::ensure_keeper_titled(crate::topmost::GOTCHI_TITLE, &mut self.setup.gotchi.bar_ui.keeper, &self.borderless_fix);
        if self.setup.gotchi.bar_ui.geom.is_none() {
            // where it was left, else the lower right of the screen
            let want = ov.pos.or_else(|| {
                let m = ctx.input(|i| i.viewport().monitor_size)?;
                Some([m.x - size.x - 48.0, m.y - size.y - 140.0])
            });
            let at = crate::topmost::fit_on_screen(want, [size.x, size.y], ctx.pixels_per_point(), true);
            let b = &mut self.setup.gotchi.bar_ui;
            b.geom = Some((at, [size.x, size.y]));
            b.applied = Some(size);
        }
        let Some((pos, first_size)) = self.setup.gotchi.bar_ui.geom else { return };
        let builder = egui::ViewportBuilder::default()
            .with_title(crate::topmost::GOTCHI_TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false)
            .with_position(pos)
            .with_inner_size(first_size);
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("gotchi-overlay"), builder, |ctx, _class| {
            // its size follows the look; its spot is remembered when moved
            if self.setup.gotchi.bar_ui.applied.map_or(true, |a| (a - size).length() > 0.5) {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                self.setup.gotchi.bar_ui.applied = Some(size);
            }
            if let Some(here) = ctx.input(|i| i.viewport().outer_rect) {
                let now = [here.min.x, here.min.y];
                let saved = self.setup.gotchi.file().overlay.pos;
                if saved.map_or(true, |s| (s[0] - now[0]).abs() > 0.5 || (s[1] - now[1]).abs() > 0.5) {
                    self.setup.gotchi.file().overlay.pos = Some(now);
                    self.setup.gotchi.touch();
                }
            }
            let drawn = self.gotchi_overlay_ui(ctx, &ov);
            self.gotchi_clip(ctx, &drawn);
        });
    }

    /// Cut the window to what was drawn.
    fn gotchi_clip(&mut self, ctx: &egui::Context, d: &Drawn) {
        let Some(hwnd) = self.setup.gotchi.bar_ui.keeper else { return };
        let ppp = ctx.pixels_per_point();
        let px = |v: f32| (v * ppp).round() as i32;
        let polys: Vec<Vec<[i32; 2]>> = d.polys.iter().map(|p| p.iter().map(|q| [px(q.x), px(q.y)]).collect()).collect();
        let boxes: Vec<[i32; 4]> = d
            .boxes
            .iter()
            .map(|r| {
                let (x0, y0) = ((r.min.x * ppp).floor() as i32, (r.min.y * ppp).floor() as i32);
                let (x1, y1) = ((r.max.x * ppp).ceil() as i32, (r.max.y * ppp).ceil() as i32);
                [x0, y0, x1 - x0, y1 - y0]
            })
            .collect();
        let rounded: Vec<[i32; 5]> = d.rounded.iter().map(|(r, rad)| [px(r.min.x), px(r.min.y), px(r.width()), px(r.height()), px(*rad)]).collect();
        let mut key: Vec<i32> = Vec::new();
        for p in &polys {
            key.push(-1);
            key.extend(p.iter().flatten());
        }
        key.push(-2);
        key.extend(boxes.iter().flatten());
        key.push(-3);
        key.extend(rounded.iter().flatten());
        let b = &mut self.setup.gotchi.bar_ui;
        if b.clip.as_ref().is_some_and(|(h, k)| *h == hwnd && *k == key) {
            return;
        }
        crate::topmost::clip_window_shapes(hwnd, &polys, &boxes, &rounded);
        b.clip = Some((hwnd, key));
    }

    fn gotchi_overlay_ui(&mut self, ctx: &egui::Context, ov: &OvCfg) -> Drawn {
        let frame = self.setup.gotchi.file().frame;
        let mut act: Option<Act> = None;
        let mut press: Option<usize> = None;
        let mut open = false;
        let mut drag = false;
        let mut drawn = Drawn::default();
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let r = ui.max_rect();
            let t = ui.input(|i| i.time) as f32;
            ui.ctx().request_repaint_after(Duration::from_millis(66));
            let p = ui.painter().clone();
            match ov.style {
                OvStyle::Device => {
                    // the same device as on the page: drag it by its shell,
                    // its three buttons pick and do, click the screen to pat
                    if ui.interact(r, egui::Id::new("gotchi-ov-shell"), Sense::drag()).drag_started() {
                        drag = true;
                    }
                    drawn = self.gotchi_overlay_paint(&p, r, ov, frame, t, true);
                    let (mut hov, mut down) = (None, None);
                    for (i, b) in drawn.buttons.clone().iter().enumerate() {
                        let resp = ui.interact(b.expand(3.0), egui::Id::new(("gotchi-ov-btn", i)), Sense::click());
                        if resp.hovered() {
                            hov = Some(i);
                        }
                        if resp.is_pointer_button_down_on() {
                            down = Some(i);
                        }
                        let tip = self.gotchi_button_tip(i);
                        if resp.on_hover_text(tip).clicked() {
                            press = Some(i);
                        }
                    }
                    self.setup.gotchi.bar_ui.btn_hover = hov;
                    self.setup.gotchi.bar_ui.btn_down = down;
                    if let Some(sr) = drawn.screen {
                        let resp = ui.interact(sr, egui::Id::new("gotchi-ov-screen"), Sense::click_and_drag());
                        if resp.drag_started() {
                            drag = true;
                        }
                        if resp.clicked() && self.setup.gotchi.game.is_none() {
                            act = Some(Act::Pat);
                        }
                        if resp.on_hover_text("Click to pat · drag to move · right-click to open Pokégotchi").secondary_clicked() {
                            open = true;
                        }
                    }
                }
                OvStyle::Perch => {
                    drawn = self.gotchi_overlay_paint(&p, r, ov, frame, t, true);
                    let resp = ui.interact(r, egui::Id::new("gotchi-ov-perch"), Sense::click_and_drag());
                    if resp.drag_started() {
                        drag = true;
                    }
                    if resp.clicked() {
                        act = Some(Act::Pat);
                    }
                    if resp.on_hover_text("Click to pat · drag to move · right-click to open Pokégotchi").secondary_clicked() {
                        open = true;
                    }
                }
                OvStyle::Strip => {
                    drawn = self.gotchi_overlay_paint(&p, r, ov, frame, t, true);
                    let resp = ui.interact(r, egui::Id::new("gotchi-ov-strip"), Sense::click_and_drag());
                    if resp.drag_started() {
                        drag = true;
                    }
                    if resp.on_hover_text("Drag to move · click to open Pokégotchi").clicked() {
                        open = true;
                    }
                }
            }
        });
        if drag {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if let Some(a) = act {
            self.gotchi_act(a);
        }
        if let Some(i) = press {
            self.gotchi_press(i);
        }
        if open {
            self.go_to(Tab::Gotchi);
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
        }
        drawn
    }

    /// Draw the overlay look in `r` (also the previews). `window`: on its own
    /// window (no swaying leaves, so the window's cut fits it exactly).
    /// Returns what was drawn.
    pub(super) fn gotchi_overlay_paint(&mut self, p: &egui::Painter, r: Rect, ov: &OvCfg, frame: Frame, t: f32, window: bool) -> Drawn {
        let mut d = Drawn::default();
        let f = self.setup.gotchi.file().clone();
        let Some(pet) = f.partner().cloned() else { return d };
        let idle = self.setup.gotchi.last_enc.map_or(true, |e| e.elapsed() > Duration::from_secs(300));
        match ov.style {
            OvStyle::Device => {
                let (hov, down) = if window { (self.setup.gotchi.bar_ui.btn_hover, self.setup.gotchi.bar_ui.btn_down) } else { (None, None) };
                let parts = paint_device(p, r, frame, t, down, hov, window);
                self.gotchi_screen(p, parts.screen, frame, t, false);
                d.buttons = parts.buttons.to_vec();
                d.screen = Some(parts.screen);
                // a hair inside the drawn edge, so no window shows past it
                d.polys = parts
                    .outline
                    .iter()
                    .map(|poly| {
                        let c = poly.iter().fold(egui::Vec2::ZERO, |a, q| a + q.to_vec2()) / poly.len().max(1) as f32;
                        poly.iter().map(|q| c.to_pos2() + (*q - c.to_pos2()) * 0.995).collect()
                    })
                    .collect();
            }
            OvStyle::Perch => {
                let k = ov.scale.pixels();
                let hop = self.setup.gotchi.hop.map_or(0.0, |h| hop_y(h.elapsed().as_secs_f32()));
                let party = self.setup.gotchi.party.is_some_and(|h| h.elapsed() < Duration::from_secs(8));
                let lift = ((hop * 8.0 + if party { (t * 7.0).sin().abs() * 6.0 } else { 0.0 }) * k).round();
                let sp = self.gotchi_species(&pet.slug).map(|i| self.species[i].clone());
                let t_ms = self.t_ms();
                let mut placed = false;
                if let Some(sp) = &sp {
                    let tex = self.sprites.get(sp, true).map(|a| (a.frame(t_ms).id(), a.size));
                    let mask = self.gotchi_mask(&pet.slug).map(|m| (m.w, m.h, m.bbox, m.index(t_ms)));
                    if let (Some((tex, size)), Some((w, h, bb, idx))) = (tex, mask) {
                        if size == vec2(w as f32, h as f32) {
                            // the Pokémon's own box sits on the window's floor, whole pixels only
                            let (bw, bh) = ((bb[2] - bb[0]) as f32, (bb[3] - bb[1]) as f32);
                            let origin = pos2((r.center().x - bw * k / 2.0).round(), (r.max.y - bh * k - lift).round());
                            let img = Rect::from_min_size(origin - vec2(bb[0] as f32 * k, bb[1] as f32 * k), vec2(w as f32 * k, h as f32 * k));
                            p.image(tex, img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            // the window: exactly the pixels this frame draws
                            if let Some(m) = self.setup.gotchi.bar_ui.mask.as_ref().and_then(|(_, m)| m.as_ref()) {
                                let on = &m.frames[idx.min(m.frames.len() - 1)].0;
                                for y in bb[1]..bb[3] {
                                    let mut x = bb[0];
                                    while x < bb[2] {
                                        if on[y * w + x] {
                                            let start = x;
                                            while x < bb[2] && on[y * w + x] {
                                                x += 1;
                                            }
                                            d.boxes.push(Rect::from_min_size(
                                                pos2(origin.x + (start - bb[0]) as f32 * k, origin.y + (y - bb[1]) as f32 * k),
                                                vec2((x - start) as f32 * k, k),
                                            ));
                                        } else {
                                            x += 1;
                                        }
                                    }
                                }
                            }
                            placed = true;
                        }
                    }
                    if !placed {
                        // not downloaded yet: the sprite in a small rounded box
                        let box_r = Rect::from_min_max(pos2(r.min.x, r.min.y + perch_top(ov)), r.max);
                        p.rect_filled(box_r, 8.0, Color32::from_rgba_unmultiplied(16, 18, 30, 230));
                        self.draw_sprite(p, sp, true, pos2(box_r.center().x, box_r.max.y - 2.0), box_r.size());
                        d.rounded.push((box_r, 8.0));
                    }
                }
                let asleep = f.asleep || idle;
                let says = if self.setup.gotchi.hop.is_some_and(|h| h.elapsed() < Duration::from_millis(900)) && self.setup.gotchi.said.as_ref().map_or(true, |s| s.1.elapsed() > Duration::from_millis(900)) {
                    Some("+1!".to_string())
                } else if self.setup.gotchi.saying().is_some() || party {
                    Some(short(&self.setup.gotchi.bubble(pet.mood(f.asleep)), 16))
                } else {
                    None
                };
                let fs = 9.0 + 2.0 * (k - 1.0);
                if asleep && says.is_none() {
                    let ph = (t * 0.6).fract();
                    let zr = Rect::from_center_size(pos2(r.max.x - 12.0, r.min.y + 16.0 - ph * 8.0), vec2(14.0, 14.0) * (1.0 + 0.3 * (k - 1.0)));
                    p.rect_filled(zr, 4.0, Color32::from_rgba_unmultiplied(16, 18, 30, 220));
                    p.text(zr.center(), Align2::CENTER_CENTER, "z", crate::fonts::sans_bold_font(fs), Color32::WHITE);
                    d.rounded.push((zr, 4.0));
                }
                if let Some(s) = says {
                    let g = p.layout_no_wrap(s, crate::fonts::sans_bold_font(fs), Color32::from_rgb(30, 30, 40));
                    let br = Rect::from_min_size(pos2((r.max.x - g.size().x - 10.0).max(r.min.x + 1.0), r.min.y + 1.0), g.size() + vec2(8.0, 4.0));
                    p.rect_filled(br, 6.0, Color32::WHITE);
                    p.galley(br.min + vec2(4.0, 2.0), g, Color32::from_rgb(30, 30, 40));
                    d.rounded.push((br, 6.0));
                }
            }
            OvStyle::Strip => {
                let s = r.height() / 26.0;
                p.rect_filled(r, 8.0 * s, Color32::from_rgba_unmultiplied(16, 18, 30, 236));
                let lk = look(frame);
                p.rect_stroke(r.shrink(0.5), 8.0 * s, Stroke::new(1.0f32, theme::blend(Color32::from_white_alpha(28), lk.stage, 0.25)));
                d.rounded.push((r, 8.0 * s));
                let mood = pet.mood(f.asleep);
                let face = Rect::from_min_size(r.min + vec2(3.0, 1.0) * s, vec2(26.0, 24.0) * s);
                if let Some(i) = self.gotchi_species(&pet.slug) {
                    let sp = self.species[i].clone();
                    let hop = self.setup.gotchi.hop.map_or(0.0, |h| hop_y(h.elapsed().as_secs_f32()));
                    self.draw_sprite(p, &sp, true, pos2(face.center().x, face.max.y - hop * 3.0 * s), face.size());
                }
                p.circle_filled(pos2(face.max.x + 2.0 * s, r.min.y + 6.0 * s), 3.0 * s, mood_col(mood));
                let x0 = face.max.x + 10.0 * s;
                let gap = 5.0 * s;
                let w = (r.max.x - 8.0 * s - x0 - 3.0 * gap) / 4.0;
                for (i, n) in Need::ALL.iter().enumerate() {
                    let mr = Rect::from_min_size(pos2(x0 + i as f32 * (w + gap), r.center().y - 3.0 * s), vec2(w, 6.0 * s));
                    meter(p, mr, pet.need(*n) / 100.0, need_col(*n), Color32::from_rgb(45, 48, 70));
                }
                if let Some(msg) = self.setup.gotchi.saying().map(|m| short(m, 22)) {
                    let g = p.layout_no_wrap(msg, crate::fonts::sans_bold_font(9.5 * s), Color32::from_rgb(255, 205, 120));
                    let at = pos2(r.max.x - 7.0 * s - g.size().x, r.center().y - g.size().y / 2.0);
                    p.rect_filled(Rect::from_min_size(at - vec2(5.0, 2.0) * s, g.size() + vec2(10.0, 4.0) * s), 5.0 * s, Color32::from_rgba_unmultiplied(16, 18, 30, 240));
                    p.galley(at, g, Color32::from_rgb(255, 205, 120));
                }
            }
        }
        d
    }
}
