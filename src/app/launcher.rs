//! The Companion launcher's own pages: Home, the Shiny Hunter app page
//! (Start / Stop overlay), and placeholders for the Pokédex and Map.

use super::ui_kit::{self as kit, PillKind, CARD_HOVER, MUTED, TEXT};
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
    fn overlay_button(&mut self, ui: &mut egui::Ui, size: egui::Vec2) {
        if self.overlay_on && self.ov_hidden {
            let b = egui::Button::new(RichText::new("▶  Show overlay").size(kit::fs(14.0)).strong().color(theme::button_ink()))
                .min_size(size);
            if kit::accent_button(ui, b, kit::primary_round())
                .on_hover_text("The overlay is minimised and still counting. Bring it back.")
                .clicked()
            {
                self.ov_hidden = false;
            }
        } else if self.overlay_on {
            let b = egui::Button::new(RichText::new("■  Stop overlay").size(kit::fs(14.0)).strong().color(TEXT()))
                .fill(kit::btn_fill())
                .rounding(kit::btn_round(8.0))
                .min_size(size);
            if ui.add(b).on_hover_text("Close the overlay. Counting keeps going if that's on below.").clicked() {
                self.stop_overlay();
            }
        } else {
            let b = egui::Button::new(RichText::new("▶  Start overlay").size(kit::fs(14.0)).strong().color(theme::button_ink()))
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
        self.setup.hunt_tab = super::settings::HuntTab::Target;
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
        match self.target_idx() {
            Some(i) => {
                let sp = self.species[i].clone();
                self.draw_sprite(&p, &sp, true, r.center_bottom() - egui::vec2(0.0, size * 0.06), egui::vec2(size * 0.94, size * 0.9));
            }
            None => {
                p.text(r.center(), egui::Align2::CENTER_CENTER, "?", kit::font(size * 0.4), MUTED());
            }
        }
    }

    fn target_label(&self) -> String {
        self.targets_title().unwrap_or_else(|| "No target yet".to_string())
    }

    // ───────────────────────────── Home ─────────────────────────────

    pub(super) fn page_home(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "HOME", "Your Platinum Together tools in one place.");
        self.home_notices(ui);

        kit::section(ui, "Apps");
        let gap = 12.0;
        let avail = ui.available_width();
        let three = avail >= 760.0;
        let w = if three { ((avail - gap * 2.0) / 3.0).floor() } else { avail };
        // Side by side, the cards all take the tallest one's height (it's
        // measured each frame and applied on the next).
        let h_id = egui::Id::new("home-app-card-h");
        let same_h: f32 = if three { ui.data(|d| d.get_temp(h_id)).unwrap_or(0.0) } else { 0.0 };
        let tallest = std::cell::Cell::new(0.0f32);
        let card = |ui: &mut egui::Ui, which: u8, app: &mut App| {
            ui.allocate_ui_with_layout(egui::vec2(w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(w);
                kit::card(ui, |ui| {
                    let top = ui.cursor().min.y;
                    ui.set_min_height(same_h);
                    match which {
                    0 => app.shiny_app_card(ui),
                    1 => app_card(
                        ui,
                        "📖",
                        "Pokédex",
                        "Every Pokémon in Platinum Together: evolutions, moves and where to find them.",
                        true,
                    ),
                    _ => app_card(
                        ui,
                        "🗺",
                        "Map",
                        "Sinnoh from your game's own map: every area, who lives there, and today's rare grass.",
                        true,
                    ),
                    }
                    // Height of the content itself (not the stretched card).
                    tallest.set(tallest.get().max(ui.cursor().min.y - top));
                });
            });
        };
        if three {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                card(ui, 0, self);
                card(ui, 1, self);
                card(ui, 2, self);
            });
        } else {
            card(ui, 0, self);
            ui.add_space(gap);
            card(ui, 1, self);
            ui.add_space(gap);
            card(ui, 2, self);
        }
        if three {
            let t = tallest.get();
            if (t - same_h).abs() > 0.5 {
                ui.data_mut(|d| d.insert_temp(h_id, t));
                ui.ctx().request_repaint();
            }
        }
        // "Preview" buttons on the soon cards.
        if let Some(t) = ui.ctx().data_mut(|d| d.remove_temp::<u8>(egui::Id::new("home-open"))) {
            self.setup.tab = match t {
                1 => Tab::Pokedex,
                2 => Tab::Map,
                _ => Tab::Shiny,
            };
            if t == 0 {
                self.setup.nav_collapsed.retain(|x| *x != Tab::Shiny);
            }
        }

        kit::section(ui, "Today");
        let (today, _) = self.day_totals();
        let goal = self.cfg.daily_goal;
        let stats: [(&str, String); 5] = [
            ("Encounters today", theme::thousands(today)),
            ("Hunted today", fmt_duration(self.today_hunt_secs())),
            (
                "Daily goal",
                if goal > 0 { format!("{} / {}", theme::thousands(today.min(goal)), theme::thousands(goal)) } else { "Off".to_string() },
            ),
            ("Streak", format!("{} day{}", self.daily_streak(), if self.daily_streak() == 1 { "" } else { "s" })),
            ("Trainer level", self.cfg.level_seen.max(1).to_string()),
        ];
        let cols = if avail >= 760.0 { 5.0 } else { 3.0 };
        let sw = ((avail - 8.0 * (cols - 1.0)) / cols).floor();
        let mut open_stat: Option<usize> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            for (i, (label, value)) in stats.iter().enumerate() {
                let tip = match i {
                    0 | 1 => "Open this hunt's stats",
                    2 => "Open your goals",
                    _ => "Open your Trainer Profile",
                };
                if kit::stat_button(ui, sw, label, value, tip) {
                    open_stat = Some(i);
                }
            }
        });
        if let Some(i) = open_stat {
            self.setup.scroll_top = true;
            match i {
                0 | 1 => {
                    self.setup.tab = Tab::Runs;
                    self.setup.view_run = Some(super::RunRef::Current);
                    self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
                }
                2 => {
                    self.setup.tab = Tab::Profile;
                    self.setup.profile_section = super::trainer::ProfileSection::Goals;
                }
                _ => {
                    self.setup.tab = Tab::Profile;
                    self.setup.profile_section = super::trainer::ProfileSection::Overview;
                }
            }
        }

        kit::section(ui, "Rare grass today");
        self.rare_home(ui);
    }

    /// The Shiny Hunter card on Home.
    fn shiny_app_card(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("✨ Shiny Hunter").size(kit::fs(15.0)).strong().color(TEXT()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.overlay_on && self.ov_hidden {
                    kit::pill(ui, "Minimised", PillKind::Warn).on_hover_text("The overlay is hidden but still counting");
                } else if self.overlay_on {
                    kit::pill(ui, "Running", PillKind::Ok);
                } else {
                    kit::pill(ui, "Stopped", PillKind::Info);
                }
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            self.target_badge(ui, 64.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(self.target_label()).size(kit::fs(14.0)).strong().color(TEXT()));
                ui.label(
                    RichText::new(format!("{} encounters", theme::thousands(self.cfg.count)))
                        .size(kit::fs(12.5))
                        .color(TEXT()),
                );
                let (odds, col) = self.odds_line();
                ui.label(RichText::new(odds).size(kit::fs(11.5)).color(col));
            });
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            self.overlay_button(ui, egui::vec2(150.0, 32.0));
            if ui
                .add(egui::Button::new(RichText::new("Open").size(kit::fs(13.0))).fill(kit::btn_fill()).min_size(egui::vec2(70.0, 32.0)))
                .on_hover_text("Open Shiny Hunter")
                .clicked()
            {
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("home-open"), 0u8));
            }
            if ui
                .add(egui::Button::new(RichText::new("Change target").size(kit::fs(13.0))).fill(kit::btn_fill()).min_size(egui::vec2(0.0, 32.0)))
                .on_hover_text("Pick a different Pokémon to hunt")
                .clicked()
            {
                self.open_target_picker();
            }
        });
    }

    /// Update / break / goal notices, as cards on Home.
    fn home_notices(&mut self, ui: &mut egui::Ui) {
        use super::notices::NoticeAction;
        if self.notices.is_empty() {
            return;
        }
        let mut act: Option<(&'static str, NoticeAction)> = None;
        let mut dismiss: Option<&'static str> = None;
        for n in &self.notices {
            kit::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(n.icon).size(kit::fs(20.0)).color(n.accent));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&n.title).size(kit::fs(13.5)).strong().color(TEXT()));
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
            });
            ui.add_space(8.0);
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
            }
        }
    }

    // ───────────────────────────── Shiny Hunter ─────────────────────────────

    pub(super) fn page_shiny(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "SHINY HUNTER", "Count encounters and catch shinies, with an overlay on top of the game.");
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                self.target_badge(ui, 96.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Current hunt").size(kit::fs(11.0)).color(MUTED()));
                    ui.label(RichText::new(self.target_label()).size(kit::fs(18.0)).strong().color(TEXT()));
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
                "Encounters still count with the overlay closed (the count shows here and on Home). Off: counting pauses until you press Start.",
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
        let pages = [
            (Tab::Hunt, "🎯", "Hunt", "Target, method and odds"),
            (Tab::Counting, "👁", "Counting", "Game log or screen reading"),
            (Tab::Runs, "📜", "Runs", "Every hunt, with stats"),
            (Tab::Dex, "⭐", "Shiny Dex", "Shinies you've caught"),
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
                    ui.label(RichText::new("✨ Shiny Hunter").size(kit::fs(12.5)).strong().color(TEXT()));
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

/// A Home card for another app (`ready` = built; otherwise "Coming soon").
fn app_card(ui: &mut egui::Ui, icon: &str, title: &str, about: &str, ready: bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{icon} {title}")).size(kit::fs(15.0)).strong().color(TEXT()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ready {
                kit::pill(ui, "Experimental", PillKind::Warn).on_hover_text("Still being built: some things may be missing or wrong");
            } else {
                kit::pill(ui, "Coming soon", PillKind::Warn);
            }
        });
    });
    ui.add_space(8.0);
    ui.label(RichText::new(about).size(kit::fs(12.5)).color(MUTED()));
    ui.add_space(10.0);
    let which: u8 = if title == "Map" { 2 } else { 1 };
    if ui
        .add(
            egui::Button::new(RichText::new(if ready { "Open" } else { "Preview" }).size(kit::fs(13.0)))
                .fill(kit::btn_fill())
                .min_size(egui::vec2(80.0, 32.0)),
        )
        .clicked()
    {
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("home-open"), which));
    }
}
