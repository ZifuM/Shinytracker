//! Start overlay: a picker with every overlay as a tile with a preview.
//! Tick the ones you want and press Go. Your picks are kept, so next time
//! they're ticked already (Go still opens them). An overlay from an app you
//! don't have is greyed out, with a way to the App Store.
//!
//! The Card Collector's XP bar stays pinned under the Shiny Hunter overlay
//! (left or right end, or off), so it can only open with it. Pokégotchi is
//! its own window.

use super::gotchi_app::mock_game;
use super::store::{app_col, app_icon, pill_button};
use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::apps::AppId;
use crate::gotchi::OvStyle;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};

/// The overlays, in the picker's order.
const TILES: [AppId; 3] = [AppId::Shiny, AppId::Cards, AppId::Gotchi];

fn title(a: AppId) -> &'static str {
    match a {
        AppId::Cards => "Card Collector XP bar",
        AppId::Gotchi => "Pokégotchi",
        _ => "Shiny Hunter overlay",
    }
}

fn blurb(a: AppId) -> &'static str {
    match a {
        AppId::Cards => "Your XP to the next booster pack, pinned under the shiny counter.",
        AppId::Gotchi => "Your pet in its own little window. Drag it anywhere.",
        _ => "Your count, odds and the shiny alert, on top of the game.",
    }
}

/// The shiny counter as a small bar (previews).
fn mock_bar(p: &egui::Painter, r: Rect, name: &str, count: u64) {
    p.rect_filled(r, 10.0, Color32::from_rgba_unmultiplied(18, 19, 28, 238));
    p.rect_stroke(r, 10.0, Stroke::new(1.0f32, Color32::from_white_alpha(24)));
    p.with_clip_rect(r.shrink(2.0)).text(r.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(12.0), Color32::WHITE);
    p.text(r.right_center() - vec2(12.0, 0.0), Align2::RIGHT_CENTER, theme::thousands(count), crate::fonts::sans_bold_font(13.0), kit::C_GOLD);
}

impl App {
    /// What's picked and can open: (shiny overlay, XP bar, Pokégotchi).
    pub(super) fn overlay_picks(&mut self) -> (bool, bool, bool) {
        let shiny = self.cfg.ov_pick_shiny;
        let cards = shiny && self.has_app(AppId::Cards) && self.setup.cards.file().bar.on;
        let gotchi = self.has_app(AppId::Gotchi) && {
            let f = self.setup.gotchi.file();
            f.overlay.show && f.partner().is_some()
        };
        (shiny, cards, gotchi)
    }

    pub(super) fn ov_picker_window(&mut self, ctx: &egui::Context) {
        if !self.setup.ov_picker {
            return;
        }
        let screen = ctx.screen_rect();
        let t = ctx.input(|i| i.time) as f32;
        ctx.request_repaint_after(std::time::Duration::from_millis(66));
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut go = false;
        let mut store: Option<AppId> = None;
        let mut open_app: Option<Tab> = None;
        // what's ticked right now
        let has_cards = self.has_app(AppId::Cards);
        let has_gotchi = self.has_app(AppId::Gotchi);
        let mut shiny = self.cfg.ov_pick_shiny;
        let mut toasts_on = self.cfg.all_toasts;
        let mut bar = if has_cards { Some(self.setup.cards.file().bar.clone()) } else { None };
        let mut gov = if has_gotchi { Some(self.setup.gotchi.file().overlay.clone()) } else { None };
        let pet = has_gotchi && self.setup.gotchi.file().partner().is_some();
        let hunt = self.target_label();
        let count = self.cfg.count;
        egui::Area::new(egui::Id::new("ov-picker")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(170));
            let w = 960.0f32.min(sr.width() - 40.0);
            let h = 510.0f32.min(sr.height() - 40.0);
            let panel = Rect::from_center_size(sr.center(), vec2(w, h));
            let _ = ui.interact(panel, ui.id().with("ov-picker-panel"), Sense::click());
            let p = ui.painter().clone();
            p.add(kit::shadow_tier(4).as_shape(panel, 16.0));
            p.rect_filled(panel, 16.0, kit::CARD());
            p.rect_stroke(panel, 16.0, Stroke::new(1.0f32, LINE()));
            let inner = panel.shrink2(vec2(24.0, 22.0));
            p.text(inner.min, Align2::LEFT_TOP, "Start overlay", crate::fonts::sans_bold_font(19.0), TEXT());
            p.text(inner.min + vec2(0.0, 28.0), Align2::LEFT_TOP, "Pick what to open on top of the game, then press Go. Your picks are kept for next time.", crate::fonts::sans_font(12.5), MUTED());
            // close
            let xc = pos2(inner.max.x - 10.0, inner.min.y + 10.0);
            let xr = Rect::from_center_size(xc, vec2(28.0, 28.0));
            let xresp = ui.interact(xr, ui.id().with("ov-picker-x"), Sense::click());
            if xresp.hovered() {
                p.rect_filled(xr, 8.0, kit::CARD_HOVER());
            }
            for (a, b) in [(vec2(-5.0, -5.0), vec2(5.0, 5.0)), (vec2(-5.0, 5.0), vec2(5.0, -5.0))] {
                p.line_segment([xc + a, xc + b], Stroke::new(1.6f32, if xresp.hovered() { TEXT() } else { MUTED() }));
            }
            if xresp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                close = true;
            }
            // the tiles
            let top = inner.min.y + 64.0;
            let foot_h = 54.0;
            let n = TILES.len() as f32;
            let tw = (inner.width() - 16.0 * (n - 1.0)) / n;
            let th = inner.max.y - foot_h - 14.0 - top;
            for (i, a) in TILES.iter().enumerate() {
                let r = Rect::from_min_size(pos2(inner.min.x + i as f32 * (tw + 16.0), top), vec2(tw, th));
                let installed = self.has_app(*a);
                // can it be ticked, and is it?
                let (usable, ticked, why) = match a {
                    AppId::Cards => (installed && shiny, bar.as_ref().is_some_and(|b| b.on), if !shiny { Some("Opens with the Shiny Hunter overlay") } else { None }),
                    AppId::Gotchi => (installed && pet, gov.as_ref().is_some_and(|g| g.show), if installed && !pet { Some("Adopt a shiny in Pokégotchi first") } else { None }),
                    _ => (true, shiny, None),
                };
                let on = usable && ticked;
                let resp = ui.interact(r, ui.id().with(("ov-tile", i)), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && usable, kit::QUICK);
                let col = app_col(*a);
                p.rect_filled(r, 14.0, if on { theme::blend(kit::sunken(), col, 0.12) } else { theme::blend(kit::sunken(), TEXT(), 0.03 * ht) });
                p.rect_stroke(r, 14.0, Stroke::new(if on { 2.0f32 } else { 1.0 }, if on { col } else { theme::blend(LINE(), col, 0.4 * ht) }));
                // the preview
                let pv = Rect::from_min_size(r.min + vec2(12.0, 12.0), vec2(tw - 24.0, 170.0));
                mock_game(&p, pv);
                let pc = p.with_clip_rect(pv.shrink(1.0));
                let barw = (pv.width() - 40.0).min(240.0);
                match a {
                    AppId::Shiny => {
                        mock_bar(&pc, Rect::from_center_size(pv.center(), vec2(barw, 40.0)), &hunt, count);
                    }
                    AppId::Cards => {
                        let sb = Rect::from_center_size(pv.center() - vec2(0.0, 18.0), vec2(barw, 40.0));
                        mock_bar(&pc, sb, &hunt, count);
                        if let Some(b) = &bar {
                            let f = self.setup.cards.file().clone();
                            let spot = super::cards_bar::spot(sb, b.right);
                            let spot = Rect::from_min_size(spot.min, vec2(spot.width().min(barw * 0.6), spot.height()));
                            let spot = if b.right { spot.translate(vec2(sb.max.x - spot.max.x, 0.0)) } else { spot };
                            super::cards_bar::paint_bar(&pc, spot, &f, None, None, false, None, false, false, t);
                        } else {
                            let spot = Rect::from_min_size(pos2(sb.min.x, sb.max.y + 4.0), vec2(barw * 0.5, 24.0));
                            pc.rect_filled(spot, 8.0, Color32::from_rgba_unmultiplied(16, 18, 30, 228));
                            super::cards_app::xp_bar(&pc, spot.shrink2(vec2(10.0, 8.5)), 0.6, t, None);
                        }
                    }
                    _ => {
                        let mut g = gov.clone().unwrap_or_default();
                        g.scale = crate::gotchi::OvScale::Small;
                        if pet {
                            let size = self.gotchi_ov_size(&g);
                            let fit = ((pv.height() - 16.0) / size.y).min((pv.width() - 16.0) / size.x).min(1.0);
                            let spot = Rect::from_center_size(pv.center(), if g.style == OvStyle::Perch { size } else { size * fit });
                            let frame = self.setup.gotchi.file().frame;
                            self.gotchi_overlay_paint(&pc, spot, &g, frame, t, false);
                        } else {
                            let dev = Rect::from_center_size(pv.center(), vec2(92.0, 112.0));
                            let parts = super::gotchi_app::paint_device(&pc, dev, crate::gotchi::Frame::Pearl, t, None, None, false);
                            pc.text(parts.screen.center(), Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(16.0), Color32::from_rgb(70, 90, 60));
                        }
                    }
                }
                // greyed when it can't open
                if !usable {
                    p.rect_filled(pv, 12.0, Color32::from_black_alpha(140));
                }
                // name, tag and tick
                let ny = pv.max.y + 14.0;
                let ic = Rect::from_min_size(pos2(r.min.x + 14.0, ny), vec2(30.0, 30.0));
                kit::icon_chip(&p, ic, app_icon(*a), if installed { col } else { MUTED() }, false);
                let g = p.text(pos2(ic.max.x + 10.0, ny + 15.0), Align2::LEFT_CENTER, title(*a), crate::fonts::sans_bold_font(14.0), if installed { TEXT() } else { MUTED() });
                let _ = g;
                let tick = pos2(r.max.x - 26.0, ny + 15.0);
                if on {
                    p.circle_filled(tick, 11.0, col);
                    p.line_segment([tick + vec2(-5.0, 0.0), tick + vec2(-1.5, 4.0)], Stroke::new(2.0f32, Color32::WHITE));
                    p.line_segment([tick + vec2(-1.5, 4.0), tick + vec2(5.5, -4.5)], Stroke::new(2.0f32, Color32::WHITE));
                } else {
                    p.circle_stroke(tick, 10.5, Stroke::new(1.5f32, if usable { theme::blend(LINE(), TEXT(), 0.3 + 0.3 * ht) } else { LINE() }));
                }
                let by = ny + 42.0;
                let gal = p.layout(blurb(*a).to_string(), crate::fonts::sans_font(12.0), MUTED(), tw - 28.0);
                p.galley(pos2(r.min.x + 14.0, by), gal, MUTED());
                // options, or why it can't open
                let oy = r.max.y - 46.0;
                let orow = Rect::from_min_max(pos2(r.min.x + 14.0, oy), pos2(r.max.x - 14.0, r.max.y - 12.0));
                let mut tile_click = resp.clicked();
                if !installed {
                    p.text(pos2(orow.min.x, orow.center().y), Align2::LEFT_CENTER, "Not installed", crate::fonts::sans_font(12.0), MUTED());
                    let b = Rect::from_min_size(pos2(orow.max.x - 150.0, orow.min.y), vec2(150.0, orow.height()));
                    if pill_button(ui, b, "Get it in the App Store", true, ("ov-get", i)).clicked() {
                        store = Some(*a);
                    }
                    tile_click = false;
                } else if let Some(why) = why {
                    p.text(pos2(orow.min.x, orow.center().y), Align2::LEFT_CENTER, why, crate::fonts::sans_font(12.0), kit::C_GOLD);
                    if *a == AppId::Gotchi {
                        let b = Rect::from_min_size(pos2(orow.max.x - 70.0, orow.min.y), vec2(70.0, orow.height()));
                        if pill_button(ui, b, "Open", false, "ov-open-gotchi").clicked() {
                            open_app = Some(Tab::Gotchi);
                        }
                    }
                    tile_click = false;
                } else {
                    let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(orow).layout(egui::Layout::left_to_right(egui::Align::Center)));
                    match a {
                        AppId::Cards => {
                            if let Some(b) = bar.as_mut() {
                                if kit::segmented(&mut cu, &mut b.right, &[(false, "Left end"), (true, "Right end")]) {
                                    b.on = true;
                                    tile_click = false;
                                }
                            }
                        }
                        AppId::Gotchi => {
                            if let Some(g) = gov.as_mut() {
                                if kit::segmented(&mut cu, &mut g.style, &[(OvStyle::Device, "Device"), (OvStyle::Perch, "Shiny"), (OvStyle::Strip, "Strip")]) {
                                    g.show = true;
                                    tile_click = false;
                                }
                            }
                        }
                        _ => {
                            cu.label(egui::RichText::new("Its look is set on the Overlay page").size(kit::fs(12.0)).color(MUTED()));
                        }
                    }
                    if cu.min_rect().contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default())) {
                        tile_click = false;
                    }
                }
                if tile_click && usable {
                    match a {
                        AppId::Cards => {
                            if let Some(b) = bar.as_mut() {
                                b.on = !b.on;
                            }
                        }
                        AppId::Gotchi => {
                            if let Some(g) = gov.as_mut() {
                                g.show = !g.show;
                            }
                        }
                        _ => shiny = !shiny,
                    }
                }
                let _ = resp.on_hover_cursor(if usable { egui::CursorIcon::PointingHand } else { egui::CursorIcon::Default });
            }
            // footer
            let fy = inner.max.y - foot_h / 2.0;
            let picked = shiny as usize + (shiny && bar.as_ref().is_some_and(|b| b.on)) as usize + (pet && gov.as_ref().is_some_and(|g| g.show)) as usize;
            let note = match self.cfg.keybinds.get(crate::hotkeys::HotAction::StartStop) {
                Some(k) => format!("{picked} picked · in the game, {} opens your picks straight away", k.text()),
                None => format!("{picked} picked"),
            };
            p.text(pos2(inner.min.x, fy), Align2::LEFT_CENTER, note, crate::fonts::sans_font(12.5), MUTED());
            let gb = Rect::from_min_size(pos2(inner.max.x - 150.0, fy - 21.0), vec2(150.0, 42.0));
            let cb = Rect::from_min_size(pos2(gb.min.x - 110.0, fy - 21.0), vec2(100.0, 42.0));
            if pill_button(ui, cb, "Cancel", false, "ov-cancel").clicked() {
                close = true;
            }
            // Master switch for every pop-up, just left of Cancel.
            let tr = Rect::from_min_max(pos2(cb.min.x - 150.0, fy - 21.0), pos2(cb.min.x - 16.0, fy + 21.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(tr).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
                kit::toggle(ui, &mut toasts_on).on_hover_text(
                    "Every pop-up: achievement, level-up and goal toasts, notices on the overlay, and the app's little messages. On by default.",
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(if toasts_on { "Toasts on" } else { "Toasts off" })
                        .font(crate::fonts::sans_bold_font(12.5))
                        .color(if toasts_on { TEXT() } else { MUTED() }),
                );
            });
            if picked > 0 {
                if pill_button(ui, gb, "▶  Go", true, "ov-go").clicked() {
                    go = true;
                }
            } else {
                p.rect_filled(gb, gb.height() / 2.0, kit::sunken());
                p.text(gb.center(), Align2::CENTER_CENTER, "Pick one to go", crate::fonts::sans_bold_font(12.5), MUTED());
            }
            if scrim.clicked() && !panel.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default())) {
                close = true;
            }
        });
        // keep the picks
        if toasts_on != self.cfg.all_toasts {
            self.cfg.all_toasts = toasts_on;
            self.dirty = true;
            if !toasts_on {
                // anything already up goes away too
                self.toasts.clear();
                self.notices.retain(|n| matches!(n.action, Some((_, super::notices::NoticeAction::Preview))));
                self.setup.store.toast = None;
            }
        }
        if shiny != self.cfg.ov_pick_shiny {
            self.cfg.ov_pick_shiny = shiny;
            self.dirty = true;
        }
        if let Some(b) = bar {
            if b != self.setup.cards.file().bar {
                self.setup.cards.file().bar = b;
                self.setup.cards.touch();
            }
        }
        if let Some(g) = gov {
            if g != self.setup.gotchi.file().overlay {
                self.setup.gotchi.file().overlay = g;
                self.setup.gotchi.touch();
            }
        }
        if go {
            self.setup.ov_picker = false;
            self.launch_overlays();
        } else if let Some(a) = store {
            self.setup.ov_picker = false;
            self.show_store_page(a);
        } else if let Some(tab) = open_app {
            self.setup.ov_picker = false;
            self.go_to(tab);
        } else if close {
            self.setup.ov_picker = false;
        }
    }
}
