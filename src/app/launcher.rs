//! The Companion launcher's own pages: Home, the Shiny Hunter app page
//! (Start / Stop overlay), and placeholders for the Pokédex and Map.

use super::ui_kit::{self as kit, CARD_HOVER, MUTED, TEXT};
use super::{fmt_one_in, App, Tab};
use crate::stats::fmt_duration;
use crate::theme::{self, GOLD};
use eframe::egui::{self, Color32, RichText, Sense};

/// Pages that belong to the Shiny Hunter app.
pub(super) fn is_shiny_tab(t: Tab) -> bool {
    matches!(t, Tab::Shiny | Tab::Hunt | Tab::Counting | Tab::Runs | Tab::Dex | Tab::Overlay)
}

impl App {
    pub(super) fn start_overlay(&mut self) {
        self.overlay_on = true;
        self.ov_hidden = false;
        self.cfg.mini = false;
        self.dirty = true;
    }

    pub(super) fn stop_overlay(&mut self) {
        self.overlay_on = false;
        self.ov_hidden = false;
        self.save_now();
    }

    /// The big Start / Stop button.
    pub(super) fn overlay_button(&mut self, ui: &mut egui::Ui, size: egui::Vec2) {
        if self.overlay_on && self.ov_hidden {
            let b = egui::Button::new(RichText::new("▶  Show overlay").size(kit::fs(14.0)).family(kit::strong_family(kit::fs(14.0))).strong().color(kit::on_primary()))
                .min_size(size);
            if kit::accent_button(ui, b, kit::primary_round())
                .on_hover_text("The overlay is minimised and still counting. Bring it back.")
                .clicked()
            {
                self.ov_hidden = false;
            }
        } else if self.overlay_on {
            let b = egui::Button::new(RichText::new("■  Stop overlay").size(kit::fs(14.0)).family(kit::strong_family(kit::fs(14.0))).strong().color(TEXT()))
                .fill(kit::btn_fill())
                .rounding(kit::btn_round(8.0))
                .min_size(size);
            if ui.add(b).on_hover_text("Close the overlay. Counting keeps going if that's on below.").clicked() {
                self.stop_overlay();
            }
        } else {
            let b = egui::Button::new(RichText::new("▶  Start overlay").size(kit::fs(14.0)).family(kit::strong_family(kit::fs(14.0))).strong().color(kit::on_primary()))
                .min_size(size);
            if kit::accent_button(ui, b, kit::primary_round())
                .on_hover_text("Open the overlay on top of the game")
                .clicked()
            {
                self.start_overlay();
            }
        }
    }

    /// Jump straight to the target picker on the Hunt page.
    pub(super) fn open_target_picker(&mut self) {
        self.setup.tab = Tab::Hunt;
        self.setup.picking_target = true;
        self.setup.search.clear();
        self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
        self.setup.scroll_top = true;
    }

    /// "Running" / "Stopped" with a light.
    fn overlay_status(&self, ui: &mut egui::Ui) {
        let (c, t) = if self.overlay_on && self.ov_hidden {
            (theme::HP_YELLOW, "Overlay minimised, still counting")
        } else if self.overlay_on {
            (theme::GREEN, "Overlay running")
        } else {
            (MUTED(), "Overlay stopped")
        };
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 4.0, c);
            ui.label(RichText::new(t).size(kit::fs(12.5)).color(TEXT()));
        });
    }

    /// "0.3× the odds" style line for the current hunt.
    fn odds_line(&self) -> (String, Color32) {
        match self.cfg.odds_one_in() {
            None => ("Odds after the first battle".to_string(), MUTED()),
            Some(one_in) => {
                let (n, _) = self.odds_count();
                let mult = n as f64 / one_in;
                let m = if mult < 0.1 { format!("{mult:.2}×") } else { format!("{mult:.1}×") };
                let col = if mult < 0.9 {
                    theme::GREEN
                } else if mult <= 1.1 {
                    theme::HP_YELLOW
                } else {
                    theme::HP_RED
                };
                (format!("{m} the odds · 1 in {}", fmt_one_in(one_in)), col)
            }
        }
    }

    /// Target sprite in a round frame.
    fn target_badge(&mut self, ui: &mut egui::Ui, size: f32) {
        let (r, _) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
        let p = ui.painter_at(r);
        p.rect_filled(r, size / 2.0, CARD_HOVER());
        match self.shown_target_idx(ui.ctx()) {
            Some(i) => {
                let sp = self.species[i].clone();
                self.draw_sprite(&p, &sp, true, r.center_bottom() - egui::vec2(0.0, size * 0.06), egui::vec2(size * 0.94, size * 0.9));
            }
            None => {
                p.text(r.center(), egui::Align2::CENTER_CENTER, "?", kit::font(size * 0.4), MUTED());
            }
        }
    }

    pub(super) fn target_label(&self) -> String {
        self.targets_title().unwrap_or_else(|| "No target yet".to_string())
    }

    // ───────────────────────────── Home ─────────────────────────────

    pub(super) fn page_home(&mut self, ui: &mut egui::Ui) {
        self.page_home_modern(ui)
    }

    /// Update / break / goal notices, as cards on Home.
    pub(super) fn home_notices(&mut self, ui: &mut egui::Ui) {
        use super::notices::NoticeAction;
        if self.notices.is_empty() {
            return;
        }
        let mut act: Option<(&'static str, NoticeAction)> = None;
        let mut dismiss: Option<&'static str> = None;
        for n in self.notices.iter().filter(|n| n.id != "preview") {
            // Modern: a banner with the notice's colour down its left edge.
            let slot = ui.painter().add(egui::Shape::Noop);
            let modern = kit::modern();
            let mut body = |ui: &mut egui::Ui| {
                ui.horizontal(|ui| {
                    if modern {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), Sense::hover());
                        kit::icon_chip(ui.painter(), r, n.icon, n.accent, false);
                        ui.add_space(4.0);
                    } else {
                        ui.label(RichText::new(n.icon).size(kit::fs(20.0)).color(n.accent));
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&n.title).size(kit::fs(13.5)).family(kit::strong_family(kit::fs(13.5))).strong().color(TEXT()));
                        ui.label(RichText::new(&n.body).size(kit::fs(12.0)).color(MUTED()));
                        if let Some(f) = n.progress {
                            ui.add_space(4.0);
                            kit::progress(ui, f);
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("✖").frame(false)).on_hover_text("Dismiss").clicked() {
                            dismiss = Some(n.id);
                        }
                        if let Some((label, a)) = n.action {
                            if ui.add(egui::Button::new(RichText::new(label).size(kit::fs(12.5))).fill(kit::btn_fill())).clicked() {
                                act = Some((n.id, a));
                            }
                        }
                    });
                });
            };
            if modern {
                let r = egui::Frame::none()
                    .fill(kit::tint(n.accent, 0.06))
                    .stroke(egui::Stroke::new(1.0f32, kit::tint(n.accent, 0.3)))
                    .rounding(14.0)
                    .inner_margin(egui::Margin { left: 18.0, right: 14.0, top: 12.0, bottom: 12.0 })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        body(ui)
                    })
                    .response
                    .rect;
                let bar = egui::Rect::from_min_size(r.min + egui::vec2(0.0, 12.0), egui::vec2(4.0, r.height() - 24.0));
                ui.painter().rect_filled(bar, 2.0, n.accent);
                ui.painter().set(slot, egui::Shape::Noop);
                ui.add_space(10.0);
            } else {
                kit::card(ui, body);
                ui.add_space(8.0);
            }
        }
        if let Some(id) = dismiss {
            self.clear_notice(id);
        }
        if let Some((id, a)) = act {
            match a {
                NoticeAction::RestartToUpdate => {
                    let ctx = ui.ctx().clone();
                    self.restart_to_update(&ctx);
                }
                NoticeAction::OpenCounting => {
                    self.setup.tab = Tab::Counting;
                    self.clear_notice(id);
                }
                NoticeAction::UseRunningGame => self.use_running_game_folder(),
                NoticeAction::Preview => self.clear_notice(id),
            }
        }
    }

    // ───────────────────────────── Shiny Hunter ─────────────────────────────

    pub(super) fn page_shiny(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "SHINY HUNTER", "Count encounters and catch shinies, with an overlay on top of the game.");
        if kit::modern() {
            self.hunt_hero(ui, false);
            ui.add_space(18.0);
        } else {
            kit::card(ui, |ui| {
                ui.horizontal(|ui| {
                    self.target_badge(ui, 96.0);
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Current hunt").size(kit::fs(11.0)).color(MUTED()));
                        ui.label(RichText::new(self.target_label()).size(kit::fs(18.0)).family(kit::strong_family(kit::fs(18.0))).strong().color(TEXT()));
                        ui.label(
                            RichText::new(format!(
                                "{} encounters · {} hunted today",
                                theme::thousands(self.cfg.count),
                                fmt_duration(self.today_hunt_secs())
                            ))
                            .size(kit::fs(12.5))
                            .color(TEXT()),
                        );
                        let (odds, col) = self.odds_line();
                        ui.label(RichText::new(odds).size(kit::fs(12.0)).color(col));
                        ui.add_space(4.0);
                        if ui
                            .add(egui::Button::new(RichText::new("Change target").size(kit::fs(12.5))).fill(kit::btn_fill()))
                            .on_hover_text("Pick a different Pokémon to hunt")
                            .clicked()
                        {
                            self.open_target_picker();
                        }
                    });
                    // Button and status flush right, stacked.
                    ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                        ui.add_space(14.0);
                        self.overlay_button(ui, egui::vec2(180.0, 40.0));
                        ui.add_space(4.0);
                        self.overlay_status(ui);
                    });
                });
            });
        }

        kit::section(ui, "Overlay window");
        kit::card(ui, |ui| {
            self.dirty |= kit::toggle_row(
                ui,
                "Start the overlay when the Companion opens",
                "Off: the overlay stays closed until you press Start.",
                &mut self.cfg.overlay_autostart,
            );
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            self.dirty |= kit::toggle_row(
                ui,
                "Keep counting while the overlay is stopped",
                "Encounters still count with the overlay closed (the count shows here and on the Dashboard). Off: counting pauses until you press Start.",
                &mut self.cfg.count_in_background,
            );
            ui.add_space(6.0);
            let key = self.cfg.keybinds.hint(crate::hotkeys::HotAction::ToggleHide);
            ui.label(
                RichText::new(format!(
                    "The overlay's ✖ closes just the overlay. The hide hotkey{key} opens it again, or shrinks it to a badge while it's open."
                ))
                .size(kit::fs(11.5))
                .color(MUTED()),
            );
        });

        kit::section(ui, "Shiny Hunter pages");
        if kit::modern() {
            let w = ui.available_width();
            self.page_tiles(
                ui,
                w,
                &[
                    (Tab::Hunt, "Target, method and odds."),
                    (Tab::Counting, "Game log or screen reading."),
                    (Tab::Runs, "Every hunt, with stats."),
                    (Tab::Dex, "Your shinies, in the Pokédex."),
                    (Tab::Overlay, "Layout, sections and sounds."),
                ],
                3,
            );
            return;
        }
        let pages = [
            (Tab::Hunt, "🎯", "Hunt", "Target, method and odds"),
            (Tab::Counting, "👁", "Counting", "Game log or screen reading"),
            (Tab::Runs, "📜", "Runs", "Every hunt, with stats"),
            (Tab::Dex, "⭐", "Shiny Dex", "Your shinies, in the Pokédex"),
            (Tab::Overlay, "🖵", "Overlay", "Layout, sections, sounds"),
        ];
        let w = ((ui.available_width() - 16.0) / 3.0).floor();
        for row in pages.chunks(3) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (tab, icon, title, sub) in row {
                    if kit::choice_tile(ui, egui::vec2(w, 80.0), false, icon, title, sub, "").clicked() {
                        self.setup.tab = *tab;
                    }
                }
            });
            ui.add_space(8.0);
        }
    }

    /// Slim Start / Stop bar at the top of each Shiny Hunter page.
    pub(super) fn overlay_strip(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(CARD_HOVER())
            .rounding(kit::r(10.0))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("✨ Shiny Hunter").size(kit::fs(12.5)).family(kit::strong_family(kit::fs(12.5))).strong().color(TEXT()));
                    ui.add_space(8.0);
                    self.overlay_status(ui);
                    ui.label(
                        RichText::new(format!("· {} · {}", self.target_label(), theme::thousands(self.cfg.count)))
                            .size(kit::fs(12.0))
                            .color(MUTED()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.overlay_button(ui, egui::vec2(140.0, 28.0));
                    });
                });
            });
        ui.add_space(14.0);
    }

    /// "Which shiny was it?" for hunts with three or more targets (opened by
    /// the overlay's Choose… after Caught!).
    pub(super) fn page_caught_pick(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "WHICH SHINY?", "Pick the one you found. It's logged as caught and the hunt starts again.");
        let all = self.all_target_idx();
        let w = 150.0;
        let mut picked: Option<usize> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
            for &i in &all {
                let sp = self.species[i].clone();
                let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 150.0), Sense::click());
                let p = ui.painter_at(r);
                let hovered = resp.hovered();
                p.rect_filled(r, kit::r(10.0), if hovered { kit::SELECTED_BG() } else { kit::CARD() });
                p.rect_stroke(r, kit::r(10.0), egui::Stroke::new(1.5f32, if hovered { GOLD } else { kit::LINE() }));
                self.draw_sprite(&p, &sp, true, egui::pos2(r.center().x, r.max.y - 42.0), egui::vec2(96.0, 90.0));
                p.text(egui::pos2(r.center().x, r.max.y - 24.0), egui::Align2::CENTER_CENTER, sp.name, kit::font_bold(14.0), TEXT());
                p.text(egui::pos2(r.center().x, r.max.y - 10.0), egui::Align2::CENTER_CENTER, format!("#{:04}", sp.dex), kit::font_small(11.0), MUTED());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("Shiny {}! Log it", sp.name)).clicked() {
                    picked = Some(i);
                }
            }
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new(RichText::new("A different Pokémon (phase)").size(kit::fs(13.0))).fill(kit::btn_fill()))
                .on_hover_text("A shiny that isn't one of your targets: pick which, and this hunt keeps going")
                .clicked()
            {
                self.setup.caught_pick = false;
                self.confirm_caught = false;
                self.setup.phase_pick = true;
                self.setup.search.clear();
            }
            if ui.add(egui::Button::new(RichText::new("Not a shiny after all").size(kit::fs(13.0))).fill(kit::btn_fill())).clicked() {
                self.setup.caught_pick = false;
                self.confirm_caught = false;
            }
        });
        if let Some(i) = picked {
            let slug = self.species[i].slug.clone();
            self.log_hunt_as(Some(slug));
            self.confirm_caught = false;
            self.setup.caught_pick = false;
            self.setup.tab = Tab::Shiny;
        }
    }
}

