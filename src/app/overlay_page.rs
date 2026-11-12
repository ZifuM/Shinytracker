//! The Overlay page: how the counter on top of the game looks and behaves,
//! with a live preview of the real overlay that follows you down the page.

use super::charts::{self as ch, CYAN, GOLD, GREEN, ORANGE, PINK, PURPLE};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::App;
use crate::config::{OverlayModules, OverlaySize, OverlayStyle, OverlayTheme};
use crate::hotkeys::HotAction;
use crate::theme;
use eframe::egui::{self, emath::TSTransform, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

/// The layer the live preview draws in (the overlay code draws there when
/// `ov_preview` is set).
pub(super) const PREVIEW_AREA: &str = "overlay-preview";

/// Section presets.
#[derive(Clone, Copy, PartialEq)]
enum Preset {
    Minimal,
    Standard,
    Everything,
}

fn preset_of(m: &OverlayModules) -> Option<Preset> {
    let flags = section_flags(m);
    let d = section_flags(&OverlayModules::default());
    if flags.iter().all(|f| *f) {
        Some(Preset::Everything)
    } else if flags == d {
        Some(Preset::Standard)
    } else if flags == section_flags(&minimal(m)) {
        Some(Preset::Minimal)
    } else {
        None
    }
}

fn section_flags(m: &OverlayModules) -> [bool; 17] {
    [
        m.status_dot, m.target, m.sprites, m.counter, m.buttons, m.odds_bar, m.method_line, m.luck, m.milestone, m.session, m.sparkline, m.last_seen, m.phases, m.xp, m.daily_goal, m.streak, m.best,
    ]
}

fn minimal(keep: &OverlayModules) -> OverlayModules {
    OverlayModules {
        style: keep.style,
        theme: keep.theme,
        size: keep.size,
        scale: keep.scale,
        click_through: keep.click_through,
        single_sprite: keep.single_sprite,
        status_dot: true,
        target: true,
        sprites: true,
        counter: true,
        buttons: false,
        odds_bar: true,
        method_line: false,
        luck: false,
        milestone: false,
        session: false,
        sparkline: false,
        last_seen: false,
        phases: false,
        xp: false,
        daily_goal: false,
        streak: false,
        best: false,
    }
}

impl App {
    pub(super) fn page_overlay(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "OVERLAY", "How the counter on top of your game looks and behaves. Changes show in the preview straight away.");
        let w = ui.available_width();
        if w >= 1000.0 {
            let gap = 20.0;
            let pw = (w * 0.42).clamp(360.0, 480.0);
            let lw = w - pw - gap;
            let x0 = ui.cursor().min.x;
            let top = ui.cursor().min.y;
            let h = ui.available_height();
            let left = ui
                .allocate_ui_with_layout(vec2(lw, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(lw);
                    self.ov_settings(ui);
                })
                .response
                .rect;
            let col = Rect::from_min_max(pos2(x0 + lw + gap, top), pos2(x0 + w, left.max.y));
            self.ov_sticky_preview(ui, col);
        } else {
            let view = ui.clip_rect();
            self.ov_preview_card(ui, view);
            ui.add_space(18.0);
            self.ov_settings(ui);
        }
        ui.add_space(30.0);
    }

    /// The preview card, floating in its column and following the page as
    /// it scrolls (it stops at the column's ends).
    fn ov_sticky_preview(&mut self, ui: &mut egui::Ui, col: Rect) {
        let ctx = ui.ctx().clone();
        let view = ui.clip_rect();
        let hid = egui::Id::new("ov-prev-card-h");
        let card_h = ctx.data(|d| d.get_temp::<f32>(hid)).unwrap_or(520.0);
        // the card floats in its own layer: give it the page's look
        let style = ui.style().clone();
        let y = (view.min.y + 16.0).max(col.min.y).min((col.max.y - card_h).max(col.min.y));
        let resp = egui::Area::new(egui::Id::new("ov-prev-card"))
            .order(egui::Order::Middle)
            .fixed_pos(pos2(col.min.x, y))
            .constrain(false)
            .show(&ctx, |aui| {
                aui.set_style(style);
                aui.set_clip_rect(view);
                aui.set_width(col.width());
                aui.set_max_width(col.width());
                self.ov_preview_card(aui, view);
            });
        let h = resp.response.rect.height();
        if (h - card_h).abs() > 0.5 {
            ctx.data_mut(|d| d.insert_temp(hid, h));
            ctx.request_repaint();
        }
        // the overlay itself sits on top of its card
        ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, egui::Id::new(PREVIEW_AREA)));
        // the mouse wheel still scrolls the page over the card
        if resp.response.contains_pointer() {
            let d = ctx.input(|i| i.smooth_scroll_delta);
            if d != egui::Vec2::ZERO {
                ui.scroll_with_delta(d);
            }
        }
    }

    fn ov_preview_card(&mut self, ui: &mut egui::Ui, view: Rect) {
        egui::Frame::none()
            .fill(kit::CARD())
            .stroke(Stroke::new(1.0f32, kit::LINE()))
            .rounding(24.0)
            .shadow(kit::card_shadow())
            .inner_margin(egui::Margin::same(18.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // title and whether the real overlay is on
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
                    kit::icon_chip(ui.painter(), r, "👁", CYAN, false);
                    ui.add_space(4.0);
                    ui.label(RichText::new("Live preview").font(ch::bold(16.0)).color(TEXT()));
                    let (txt, c) = if self.overlay_on { ("Overlay on", GREEN) } else { ("Overlay off", MUTED()) };
                    let g = ui.painter().layout_no_wrap(txt.to_string(), ch::bold(11.5), ch::ink(c));
                    ui.add_space((ui.available_width() - g.size().x - 26.0).max(4.0));
                    let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 26.0, 22.0), Sense::hover());
                    ui.painter().rect_filled(r, 11.0, ch::faint(c, 0.15));
                    ui.painter().circle_filled(pos2(r.min.x + 11.0, r.center().y), 3.5, c);
                    ui.painter().galley(pos2(r.min.x + 19.0, r.center().y - g.size().y / 2.0), g, ch::ink(c));
                });
                ui.add_space(12.0);

                // the stage: a stand-in game picture with the overlay on it
                let inner_w = ui.available_width();
                let sid = egui::Id::new("ov-prev-size");
                let size: [f32; 2] = ui.ctx().data(|d| d.get_temp(sid)).unwrap_or_else(|| self.overlay_size());
                let s = ((inner_w - 32.0) / size[0].max(1.0)).min(1.0);
                let stage_h = (size[1] * s + 40.0).max(150.0);
                let (stage, _) = ui.allocate_exact_size(vec2(inner_w, stage_h), Sense::hover());
                backdrop(ui.painter(), stage);
                let legacy = self.cfg.overlay.legacy();
                if legacy {
                    let p = ui.painter();
                    p.rect_filled(Rect::from_center_size(stage.center(), vec2(inner_w - 60.0, 64.0)), 16.0, Color32::from_black_alpha(150));
                    p.text(stage.center() - vec2(0.0, 9.0), Align2::CENTER_CENTER, "The Legacy overlay has no preview", ch::bold(13.0), Color32::WHITE);
                    p.text(stage.center() + vec2(0.0, 11.0), Align2::CENTER_CENTER, "Start the overlay to see it", ch::reg(12.0), Color32::from_gray(200));
                } else {
                    let origin = pos2(stage.center().x - size[0] * s / 2.0, (stage.center().y - size[1] * s / 2.0).max(stage.min.y + 20.0));
                    let clip = stage.shrink(1.0).intersect(view);
                    if let Some(m) = self.draw_overlay_preview(ui.ctx(), origin, size, clip, s) {
                        if (m[0] - size[0]).abs() > 0.5 || (m[1] - size[1]).abs() > 0.5 {
                            ui.ctx().data_mut(|d| d.insert_temp(sid, m));
                            ui.ctx().request_repaint();
                        }
                    }
                }
                ui.add_space(10.0);
                let mut cap = vec![if legacy { "Legacy".to_string() } else { format!("{} theme", self.cfg.overlay.theme.name()) }];
                if !legacy {
                    cap.push(match self.cfg.overlay.size {
                        OverlaySize::Full => "Full".into(),
                        OverlaySize::Compact => "Compact".into(),
                        OverlaySize::Bar => "Bar".into(),
                    });
                }
                cap.push(format!("{:.0}% opacity", self.cfg.opacity * 100.0));
                if s < 0.99 {
                    cap.push(format!("shown at {:.0}%", s * 100.0));
                }
                if let Some(k) = self.cfg.keybinds.get(HotAction::ToggleHide).map(|b| b.text()) {
                    cap.push(format!("{k} hides it"));
                }
                ui.label(RichText::new(cap.join(" · ")).size(12.0).color(MUTED()));
                if self.cfg.mini {
                    ui.label(RichText::new("The overlay is shrunk to a badge right now.").size(12.0).color(ch::ink(ORANGE)));
                }
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    if self.overlay_on {
                        if ui.add(egui::Button::new("⏹  Stop overlay").fill(kit::btn_fill())).clicked() {
                            self.stop_overlay();
                        }
                    } else if kit::accent_button(ui, egui::Button::new(RichText::new("▶  Start overlay").font(ch::bold(13.0)).color(kit::on_primary())), 4.0).clicked() {
                        self.start_overlay();
                    }
                    if ui.add(egui::Button::new("🎉  Celebration").fill(kit::btn_fill())).on_hover_text("Shows the 1,000 encounters pop-up").clicked() {
                        self.push_milestone("1,000 ENCOUNTERS!".to_string());
                        self.cfg.mini = false;
                        self.play(crate::sound::Sfx::Milestone);
                    }
                    ui.menu_button("💬  Try a pop-up", |ui| {
                        ui.set_min_width(200.0);
                        ui.label(RichText::new("Toasts").size(11.5).color(MUTED()));
                        for (i, name) in ["🏅 Achievement", "⬆ Level up", "✔ Daily goal", "📅 Weekly challenge", "⭐ Milestone", "👋 Welcome"].into_iter().enumerate() {
                            if ui.button(name).clicked() {
                                self.preview_toast(i, None);
                                self.cfg.mini = false;
                                self.play(if i == 4 { crate::sound::Sfx::Milestone } else { crate::sound::Sfx::Achievement });
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label(RichText::new("Notices").size(11.5).color(MUTED()));
                        for (i, name) in ["☕ Break reminder", "⬇ Update download", "🔎 With a button"].into_iter().enumerate() {
                            if ui.button(name).clicked() {
                                self.preview_notice(i);
                                self.cfg.mini = false;
                                ui.close_menu();
                            }
                        }
                    });
                });
                if !self.overlay_on {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Pop-ups show under the real overlay, so start it to try them.").size(11.5).color(MUTED()));
                }
            });
    }

    /// Draw the real overlay at `origin` (scaled by `s`), clipped to `clip`.
    /// Returns the overlay's size as it laid itself out.
    fn draw_overlay_preview(&mut self, ctx: &egui::Context, origin: egui::Pos2, size: [f32; 2], clip: Rect, s: f32) -> Option<[f32; 2]> {
        let layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new(PREVIEW_AREA));
        let t = TSTransform::new(origin.to_vec2() * (1.0 - s), s);
        ctx.set_transform_layer(layer, t);
        let clip_l = t.inverse() * clip;
        // The preview mustn't change anything the real overlay uses.
        let shape = std::mem::take(&mut self.ov_shape);
        let ov_size = self.ov_size.take();
        let toasts = std::mem::take(&mut self.toasts);
        let notices = std::mem::take(&mut self.notices);
        let mini = self.cfg.mini;
        self.cfg.mini = false;
        self.ov_preview = Some((Rect::from_min_size(origin, vec2(size[0], size[1].max(60.0))), clip_l));
        self.modular_ui(ctx);
        self.ov_preview = None;
        let measured = self.ov_size;
        self.cfg.mini = mini;
        self.ov_shape = shape;
        self.ov_size = ov_size;
        self.toasts = toasts;
        self.notices = notices;
        measured
    }

    // ───────────────────────────── settings ─────────────────────────────

    fn ov_settings(&mut self, ui: &mut egui::Ui) {
        self.ov_look_card(ui);
        ui.add_space(18.0);
        self.ov_layout_card(ui);
        if !self.cfg.overlay.legacy() {
            ui.add_space(18.0);
            self.ov_sections_card(ui);
        }
        ui.add_space(18.0);
        self.ov_behaviour_card(ui);
        ui.add_space(4.0);
        self.sounds_card(ui);
    }

    fn ov_look_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "🎨", PINK, "Look");
            ui.add_space(12.0);
            let mut style = self.cfg.overlay.style;
            setting_row(ui, "Style", "Modular lets you pick a theme, sections and a size. Legacy is the original overlay, exactly as it was.", |ui| {
                kit::segmented(ui, &mut style, &[(OverlayStyle::Modular, "Modular"), (OverlayStyle::Legacy, "Legacy")]);
            });
            if style != self.cfg.overlay.style {
                self.cfg.overlay.style = style;
                self.ov_size = None;
                self.dirty = true;
            }
            if self.cfg.overlay.legacy() {
                return;
            }
            ui.add_space(14.0);
            ui.label(RichText::new("THEME").font(ch::bold(10.5)).color(MUTED()));
            ui.add_space(8.0);
            let avail = ui.available_width();
            let cols = ((avail + 10.0) / (150.0 + 10.0)).floor().clamp(2.0, 5.0);
            let tw = ((avail - 10.0 * (cols - 1.0)) / cols).floor();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                for t in OverlayTheme::ALL {
                    if super::ov_common::theme_tile(ui, t, self.cfg.overlay.theme == t, tw).clicked() && self.cfg.overlay.theme != t {
                        self.cfg.overlay.theme = t;
                        self.ov_size = None;
                        self.dirty = true;
                        if t != OverlayTheme::Classic {
                            self.add_event("trendsetter");
                        }
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new("Every theme works in all three sizes. Your sections stay the same when you switch.").size(11.5).color(MUTED()));
        });
    }

    fn ov_layout_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "📐", PURPLE, "Size & layout");
            ui.add_space(12.0);
            let legacy = self.cfg.overlay.legacy();
            if !legacy {
                let full = ui.available_width();
                let tw = ((full - 20.0) / 3.0).floor();
                let mut pick = None;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for (sz, title, sub) in [
                        (OverlaySize::Full, "Full", "Everything you turn on"),
                        (OverlaySize::Compact, "Compact", "Sprites and the count"),
                        (OverlaySize::Bar, "Bar", "One slim line"),
                    ] {
                        if size_tile(ui, tw, sz, title, sub, self.cfg.overlay.size == sz) {
                            pick = Some(sz);
                        }
                    }
                });
                if let Some(sz) = pick {
                    self.cfg.overlay.size = sz;
                    self.ov_size = None;
                    self.dirty = true;
                }
                if self.cfg.overlay.size == OverlaySize::Full {
                    ui.add_space(14.0);
                    self.dirty |= kit::toggle_row(ui, "Liquid layout", "Drag the overlay's right edge to make it wider or narrower; the sections stretch to fit.", &mut self.cfg.liquid);
                    if self.cfg.liquid {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Width {:.0}", self.cfg.liquid_width)).size(12.5).color(MUTED()));
                            if ui.add(egui::Button::new(RichText::new("Reset width").size(12.0)).fill(kit::btn_fill())).clicked() {
                                self.cfg.liquid_width = super::FULL_SIZE[0];
                                self.dirty = true;
                            }
                        });
                    }
                }
                ui.add_space(14.0);
                divider(ui);
                ui.add_space(12.0);
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("Overlay size").size(13.0).color(TEXT()));
                kit::info(ui, "How big the overlay is drawn on screen. Small is the original size; Medium and Large scale everything up for big or high-resolution screens.");
                ui.add_space((ui.available_width() - 230.0).max(8.0));
                use crate::config::OverlayScale;
                if kit::segmented(ui, &mut self.cfg.overlay.scale, &[(OverlayScale::Small, "Small"), (OverlayScale::Medium, "Medium"), (OverlayScale::Large, "Large")]) {
                    self.dirty = true;
                }
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Background opacity").size(13.0).color(TEXT()));
                kit::info(ui, "Fade the panel so the game shows through.");
            });
            ui.add_space(4.0);
            ui.spacing_mut().slider_width = ui.available_width() - kit::SLIDER_VALUE_W - 16.0;
            let lowest = if legacy { 0.4 } else { 0.1 };
            let text = format!("{:.0}%", self.cfg.opacity * 100.0);
            self.dirty |= kit::slider_f32(ui, &mut self.cfg.opacity, lowest..=1.0, &text).changed();
        });
    }

    fn ov_sections_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "📜", GOLD, "Sections");
            ui.add_space(10.0);
            // presets
            let cur = preset_of(&self.cfg.overlay);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                ui.label(RichText::new("Start from").size(12.5).color(MUTED()));
                for (pr, label, tip) in [
                    (Preset::Minimal, "Minimal", "Target, sprites, the count and the odds bar"),
                    (Preset::Standard, "Standard", "The default sections"),
                    (Preset::Everything, "Everything", "Every section on"),
                ] {
                    if preset_chip(ui, label, cur == Some(pr)).on_hover_text(tip).clicked() {
                        let keep = self.cfg.overlay.clone();
                        self.cfg.overlay = match pr {
                            Preset::Minimal => minimal(&keep),
                            Preset::Standard => OverlayModules { style: keep.style, theme: keep.theme, size: keep.size, click_through: keep.click_through, ..OverlayModules::default() },
                            Preset::Everything => {
                                let mut m = keep.clone();
                                for f in [
                                    &mut m.status_dot, &mut m.target, &mut m.sprites, &mut m.counter, &mut m.buttons, &mut m.odds_bar, &mut m.method_line, &mut m.luck, &mut m.milestone, &mut m.session, &mut m.sparkline, &mut m.last_seen, &mut m.phases, &mut m.xp, &mut m.daily_goal, &mut m.streak, &mut m.best,
                                ] {
                                    *f = true;
                                }
                                m
                            }
                        };
                        self.cfg.level_on_overlay = pr != Preset::Minimal;
                        self.cfg.show_session_timer = pr != Preset::Minimal;
                        self.ov_size = None;
                        self.dirty = true;
                    }
                }
                if cur.is_none() {
                    ui.label(RichText::new("· your own mix").size(12.5).color(MUTED()));
                }
            });
            let size = self.cfg.overlay.size;
            if size != OverlaySize::Full {
                ui.add_space(10.0);
                let note = match size {
                    OverlaySize::Compact => "Compact shows the sprites and the count; the other sections come back in Full.",
                    _ => "The Bar shows target, count and odds, plus luck, milestone, session, pace, streak and buttons if they're on.",
                };
                ui.label(RichText::new(note).size(12.0).color(ch::ink(ORANGE)));
            }
            let in_size = |f: &str| -> bool {
                match size {
                    OverlaySize::Full => true,
                    OverlaySize::Compact => matches!(f, "status" | "sprites" | "single" | "counter"),
                    OverlaySize::Bar => matches!(f, "target" | "sprites" | "counter" | "odds" | "luck" | "milestone" | "session" | "sparkline" | "streak" | "buttons" | "status"),
                }
            };
            let mut d = false;
            for (title, items) in SECTION_GROUPS {
                ui.add_space(14.0);
                ui.label(RichText::new(title.to_uppercase()).font(ch::bold(10.5)).color(MUTED()));
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    for (key, label, help) in items {
                        let parent = match *key {
                            "single" => Some(self.cfg.overlay.sprites),
                            "time" => Some(self.cfg.overlay.counter),
                            _ => None,
                        };
                        let enabled = parent.unwrap_or(true) && in_size(key);
                        let on = self.ov_flag(key);
                        if section_chip(ui, label, help, on, enabled, parent.is_some()) {
                            d = true;
                        }
                    }
                });
            }
            if d {
                self.ov_size = None;
                self.dirty = true;
            }
        });
    }

    fn ov_behaviour_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "⚙", ORANGE, "Behaviour");
            ui.add_space(12.0);
            let key = self.cfg.keybinds.get(HotAction::ClickThrough).map(|b| b.text());
            let ct_help = match &key {
                Some(k) => format!("Clicks go straight through the overlay to the game. Press {k} to turn it off again. It turns itself off while a shiny or the Caught! question needs a click."),
                None => "Clicks go straight through the overlay to the game. Set a hotkey for it in Hotkeys first, so you can turn it off again.".to_string(),
            };
            let ct_ok = key.is_some() || self.cfg.overlay.click_through;
            let legacy = self.cfg.overlay.legacy();
            let mut rows: Vec<(&str, &str, String, bool)> = vec![
                ("borderless", "Stay visible over the game", "Keeps the overlay visible in Borderless and Full screen by making the game window 1 pixel taller (off-screen, so you won't see it). Only the window size changes, nothing in the game.".into(), true),
                ("pinned", "Pin in place", "Stops the overlay being dragged by accident.".into(), true),
                ("mini", "Shrink to badge", "Shows just a small badge with your target and count.".into(), true),
            ];
            if !legacy {
                rows.push(("click", "Click-through", ct_help, ct_ok));
            }
            rows.push(("flash", "Flash when a target is spotted", "The border blinks gold for a moment (with SPOTTED!) each time one of your targets turns up. Off: no flashing; a real shiny alert still flashes.".into(), true));
            rows.push(("celebrate", "Milestone celebrations", "A pop-up under the overlay every 1,000 encounters and when you reach 1×, 2×, 3×… the odds.".into(), true));
            if legacy {
                rows.push(("time", "Today's hunting time", "A small timer on the overlay showing how long you've hunted today, across all runs.".into(), true));
            }
            let n = rows.len();
            for (k, (key, label, help, ok)) in rows.into_iter().enumerate() {
                let mut v = *self.ov_flag(key);
                ui.add_enabled_ui(ok, |ui| {
                    if kit::toggle_row(ui, label, &help, &mut v) {
                        *self.ov_flag(key) = v;
                        self.dirty = true;
                    }
                });
                if k + 1 < n {
                    ui.add_space(10.0);
                    divider(ui);
                    ui.add_space(10.0);
                }
            }
        });
    }
}

impl App {
    /// An overlay on/off setting by name.
    fn ov_flag(&mut self, key: &str) -> &mut bool {
        let o = &mut self.cfg.overlay;
        match key {
            "status" => &mut o.status_dot,
            "target" => &mut o.target,
            "sprites" => &mut o.sprites,
            "single" => &mut o.single_sprite,
            "counter" => &mut o.counter,
            "buttons" => &mut o.buttons,
            "odds" => &mut o.odds_bar,
            "method" => &mut o.method_line,
            "luck" => &mut o.luck,
            "milestone" => &mut o.milestone,
            "session" => &mut o.session,
            "sparkline" => &mut o.sparkline,
            "last" => &mut o.last_seen,
            "phases" => &mut o.phases,
            "xp" => &mut o.xp,
            "goal" => &mut o.daily_goal,
            "streak" => &mut o.streak,
            "best" => &mut o.best,
            "click" => &mut o.click_through,
            "level" => &mut self.cfg.level_on_overlay,
            "time" => &mut self.cfg.show_session_timer,
            "borderless" => &mut self.cfg.borderless_fix,
            "pinned" => &mut self.cfg.pinned,
            "mini" => &mut self.cfg.mini,
            "flash" => &mut self.cfg.spot_flash,
            _ => &mut self.cfg.celebrate_milestones,
        }
    }
}

/// The section chips, by group: (key, label, what it shows).
const SECTION_GROUPS: [(&str, &[(&str, &str, &str)]); 5] = [
    (
        "Hunt",
        &[
            ("target", "Target name", "The name plate, with how often your target turns up (\"SEEN 12\")."),
            ("sprites", "Sprites", "Your target, normal and shiny, animated."),
            ("single", "Shiny only", "One big shiny sprite instead of normal + shiny side by side."),
            ("counter", "Counter", "The big number, with ENCOUNTERS / PAUSED under it."),
            ("time", "Today's time", "A small timer next to the counter: how long you've hunted today, across all runs."),
            ("buttons", "Buttons", "−1, +1, pause and Caught!. Hotkeys still work when they're hidden, and the Caught! question always shows."),
        ],
    ),
    ("Header", &[("level", "Level badge", "Your trainer level next to the title."), ("status", "Status light", "Green watching, gold in battle, grey paused, red if something needs fixing.")]),
    (
        "Odds & luck",
        &[
            ("odds", "Odds bar", "Under / at / over the odds, filling up as you go."),
            ("method", "Method line", "What the odds are based on, e.g. \"Rare grass · Doubles · 1/4,096 each\"."),
            ("luck", "Luck", "How many hunters would have found their shiny by now. Under 50% is fine; over 80% is unlucky."),
            ("milestone", "Next milestone", "The nearer of the next thousand and the next 1×, 2×… the odds, with how many to go."),
        ],
    ),
    (
        "Session",
        &[
            ("session", "Session strip", "This session's encounters, time and pace per hour."),
            ("sparkline", "Pace graph", "A tiny bar graph of encounters each minute for the last 20 minutes."),
            ("last", "Last encounter", "Which Pokémon were in the last battle, and how long ago."),
            ("phases", "Phase count", "Shinies of other Pokémon found during this hunt."),
        ],
    ),
    (
        "Progress",
        &[
            ("xp", "Trainer XP", "Your level and how close the next one is."),
            ("goal", "Daily goal", "Today's encounters towards your daily goal (set it in Profile > Goals)."),
            ("streak", "Streak flame", "Days in a row you've hit your daily goal."),
            ("best", "Personal best", "Your best single day, turning gold when today beats it."),
        ],
    ),
];

fn card_head(ui: &mut egui::Ui, icon: &str, col: Color32, title: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        kit::icon_chip(ui.painter(), r, icon, col, false);
        ui.add_space(4.0);
        ui.label(RichText::new(title).font(ch::bold(16.0)).color(TEXT()));
    });
}

fn divider(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, kit::LINE()));
}

/// Label and help on the left, the control flush right (laid out left to
/// right, with the control's width measured the frame before).
fn setting_row(ui: &mut egui::Ui, label: &str, help: &str, control: impl FnOnce(&mut egui::Ui)) {
    let id = ui.id().with(("ov-setting-row-w", label));
    let cw = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(160.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(TEXT()));
        kit::info(ui, help);
        ui.add_space((ui.available_width() - cw).max(8.0));
        let start = ui.cursor().min.x;
        control(ui);
        let used = ui.min_rect().max.x - start;
        if (used - cw).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(id, used));
            ui.ctx().request_repaint();
        }
    });
}

/// A stand-in for the game behind the preview: sky, distant hills and a
/// grassy field, softly lit.
fn backdrop(p: &egui::Painter, r: Rect) {
    let rad = 16.0;
    let horizon = r.min.y + r.height() * 0.42;
    let sky = Rect::from_min_max(r.min, pos2(r.max.x, horizon));
    let field = Rect::from_min_max(pos2(r.min.x, horizon), r.max);
    p.add(theme::gradient_rect(sky, [rad, rad, 0.0, 0.0], Color32::from_rgb(92, 140, 196), Color32::from_rgb(150, 196, 226)));
    p.add(theme::gradient_rect(field, [0.0, 0.0, rad, rad], Color32::from_rgb(86, 160, 96), Color32::from_rgb(56, 122, 74)));
    // gentle hills along the horizon
    let p2 = p.with_clip_rect(r.intersect(p.clip_rect()));
    for (k, (cx, w, h, c)) in [(0.18, 0.5, 26.0, (74, 140, 90)), (0.62, 0.6, 34.0, (66, 132, 84)), (0.95, 0.4, 22.0, (78, 146, 92))].iter().enumerate() {
        let _ = k;
        let c = Color32::from_rgb(c.0, c.1, c.2);
        let center = pos2(r.min.x + r.width() * cx, horizon + 6.0);
        let hw = r.width() * w / 2.0;
        let n = 24;
        let mut pts = vec![pos2(center.x - hw, horizon + 6.0)];
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let x = center.x - hw + t * hw * 2.0;
            let y = horizon + 6.0 - h * (std::f32::consts::PI * t).sin();
            pts.push(pos2(x, y));
        }
        pts.push(pos2(center.x + hw, horizon + 6.0));
        p2.add(egui::Shape::convex_polygon(pts, c, Stroke::NONE));
    }
    // a worn path through the grass
    let path = Rect::from_center_size(pos2(r.center().x, r.max.y - r.height() * 0.12), vec2(r.width() * 0.5, r.height() * 0.16));
    p2.add(egui::Shape::ellipse_filled(path.center(), path.size() / 2.0, Color32::from_rgba_unmultiplied(196, 176, 120, 60)));
}

/// A size choice with a little drawing of its shape.
fn size_tile(ui: &mut egui::Ui, w: f32, sz: OverlaySize, title: &str, sub: &str, sel: bool) -> bool {
    let h = 112.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let t = kit::anim(ui.ctx(), resp.id.with("sel"), sel, kit::BASE);
    let ht = kit::anim(ui.ctx(), resp.id.with("hov"), resp.hovered() && !sel, kit::QUICK);
    let p = ui.painter_at(rect.expand(4.0));
    p.rect_filled(rect, 18.0, theme::blend(theme::blend(kit::CARD(), TEXT(), 0.035 * ht), kit::primary(), 0.10 * t));
    p.rect_stroke(rect, 18.0, Stroke::new(1.0 + t, theme::blend(theme::blend(kit::LINE(), TEXT(), 0.18 * ht), kit::primary(), t)));
    // the shape
    let ink = theme::blend(MUTED(), kit::primary(), t);
    let fill = theme::blend(kit::CARD(), ink, 0.18);
    let c = pos2(rect.center().x, rect.min.y + 36.0);
    let shape = match sz {
        OverlaySize::Full => Rect::from_center_size(c, vec2(34.0, 44.0)),
        OverlaySize::Compact => Rect::from_center_size(c, vec2(30.0, 26.0)),
        OverlaySize::Bar => Rect::from_center_size(c, vec2(64.0, 14.0)),
    };
    p.rect_filled(shape, 5.0, fill);
    p.rect_stroke(shape, 5.0, Stroke::new(1.5f32, ink));
    match sz {
        OverlaySize::Full => {
            p.circle_filled(pos2(shape.center().x, shape.min.y + 13.0), 6.0, ink);
            for k in 0..3 {
                let y = shape.min.y + 26.0 + k as f32 * 5.0;
                p.line_segment([pos2(shape.min.x + 6.0, y), pos2(shape.max.x - 6.0 - k as f32 * 4.0, y)], Stroke::new(1.5f32, ink));
            }
        }
        OverlaySize::Compact => {
            p.circle_filled(pos2(shape.center().x - 6.0, shape.center().y), 5.0, ink);
            p.line_segment([pos2(shape.center().x + 2.0, shape.center().y), pos2(shape.max.x - 4.0, shape.center().y)], Stroke::new(2.0f32, ink));
        }
        OverlaySize::Bar => {
            p.circle_filled(pos2(shape.min.x + 8.0, shape.center().y), 3.5, ink);
            p.line_segment([pos2(shape.min.x + 16.0, shape.center().y), pos2(shape.max.x - 8.0, shape.center().y)], Stroke::new(2.0f32, ink));
        }
    }
    p.text(pos2(rect.center().x, rect.max.y - 34.0), Align2::CENTER_CENTER, title, ch::bold(13.5), TEXT());
    p.text(pos2(rect.center().x, rect.max.y - 16.0), Align2::CENTER_CENTER, ch::clip(ui, sub, &ch::reg(11.5), w - 16.0), ch::reg(11.5), MUTED());
    if t > 0.0 {
        let cc = pos2(rect.max.x - 16.0, rect.min.y + 16.0);
        p.circle_filled(cc, 8.0 * t, kit::primary());
        p.text(cc, Align2::CENTER_CENTER, "✔", egui::FontId::proportional(9.0 * t.max(0.01)), kit::on_primary());
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !sel
}

/// A preset pill.
fn preset_chip(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response {
    let g = ui.painter().layout_no_wrap(label.to_string(), ch::bold(12.5), TEXT());
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 26.0, 30.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id, resp.hovered(), kit::QUICK);
    let p = ui.painter();
    if on {
        p.add(kit::primary_shape(r, false));
        p.galley(pos2(r.center().x - g.size().x / 2.0, r.center().y - g.size().y / 2.0), g, kit::on_primary());
    } else {
        p.rect_filled(r, 15.0, theme::blend(kit::sunken(), TEXT(), 0.05 * ht));
        p.rect_stroke(r, 15.0, Stroke::new(1.0f32, kit::LINE()));
        let ink = theme::blend(MUTED(), TEXT(), 0.5 + 0.5 * ht);
        p.galley(pos2(r.center().x - g.size().x / 2.0, r.center().y - g.size().y / 2.0), g, ink);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A section you can turn on or off: a pill with a tick when it's on.
/// `sub` = an option of the section before it.
fn section_chip(ui: &mut egui::Ui, label: &str, help: &str, on: &mut bool, enabled: bool, sub: bool) -> bool {
    let font = ch::bold(12.5);
    let text = if sub { format!("↳ {label}") } else { label.to_string() };
    let g = ui.painter().layout_no_wrap(text, font, TEXT());
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 44.0, 32.0), if enabled { Sense::click() } else { Sense::hover() });
    let t = kit::anim(ui.ctx(), resp.id.with("on"), *on, kit::QUICK);
    let ht = kit::anim(ui.ctx(), resp.id.with("hov"), resp.hovered() && enabled, kit::QUICK);
    let a: f32 = if enabled { 1.0 } else { 0.4 };
    let p = ui.painter();
    let acc = kit::primary();
    p.rect_filled(r, 16.0, theme::blend(theme::blend(kit::CARD(), TEXT(), 0.04 * ht), acc, 0.14 * t).gamma_multiply(a.max(0.6)));
    p.rect_stroke(r, 16.0, Stroke::new(1.0f32, theme::blend(kit::LINE(), acc, 0.8 * t).gamma_multiply(a)));
    // tick circle
    let c = pos2(r.min.x + 16.0, r.center().y);
    p.circle_stroke(c, 7.0, Stroke::new(1.5f32, theme::blend(MUTED(), acc, t).gamma_multiply(a)));
    if t > 0.0 {
        p.circle_filled(c, 7.0 * t, acc.gamma_multiply(a));
        p.text(c, Align2::CENTER_CENTER, "✔", egui::FontId::proportional(8.5 * t.max(0.01)), kit::on_primary().gamma_multiply(a));
    }
    let ink = theme::blend(MUTED(), TEXT(), 0.4 + 0.6 * t.max(ht)).gamma_multiply(a);
    p.galley(pos2(r.min.x + 30.0, r.center().y - g.size().y / 2.0), g, ink);
    let resp = if enabled { resp.on_hover_text(help).on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp.on_hover_text(format!("{help}\n\nNot shown in this size.")) };
    if enabled && resp.clicked() {
        *on = !*on;
        return true;
    }
    false
}
