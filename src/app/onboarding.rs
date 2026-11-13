//! The onboarding tour: a few steps that set up everything a new hunter
//! needs (how counting works, the app's look, the overlay, the target and
//! the method and odds) and end with a summary and a Start overlay button.
//!
//! It opens by itself once: for new users on their first start, and for
//! people updating, once after the update (with "What's new" in place of
//! the welcome, and their settings already filled in). Advanced > Updates
//! can run it again.
//!
//! The look follows the Cirrus design: the page is an open sky (Horizon
//! blue into Sky, with soft clouds; a night sky in the dark modes), a
//! floating step rail on the left and the step itself on a large white
//! slab, titles in Inter Tight and everything else in Inter.

use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::config::{CountSource, OverlaySize, OverlayStyle, OverlayTheme, UiMode, ONBOARDING_VERSION};
use crate::fonts::{sans_bold_font, sans_font, title_font};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke};

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Welcome,
    Apps,
    WhatsNew,
    Game,
    Target,
    Odds,
    Overlay,
    Look,
    Done,
}

impl Step {
    /// Icon, name and a few words for the rail.
    fn meta(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Step::Welcome => ("👋", "Welcome", "Hello, trainer"),
            Step::Apps => ("🏪", "Your apps", "What you play for"),
            Step::WhatsNew => ("✨", "What's new", "In version 0.5"),
            Step::Game => ("🎮", "Your game", "How counting works"),
            Step::Target => ("🎯", "Your target", "What you're hunting"),
            Step::Odds => ("🎲", "Method & odds", "Your shiny chances"),
            Step::Overlay => ("🖥", "The overlay", "A counter on the game"),
            Step::Look => ("🎨", "Your look", "Mode and colours"),
            Step::Done => ("🏁", "All set", "Start hunting"),
        }
    }
}

/// The page's corner and spacing.
const SLAB_R: f32 = 28.0;
const GAP: f32 = 24.0;

impl App {
    /// Is the tour showing?
    pub(super) fn onboarding_open(&self) -> bool {
        !self.cfg.welcome_done || self.cfg.onboarding_seen < ONBOARDING_VERSION
    }

    /// People who'd set things up before the tour (or who already took it)
    /// get "What's new" instead of the welcome.
    fn returning(&self) -> bool {
        self.cfg.welcome_done
    }

    fn tour_steps(&self) -> Vec<Step> {
        if self.returning() {
            vec![Step::WhatsNew, Step::Game, Step::Look, Step::Overlay, Step::Target, Step::Odds, Step::Done]
        } else {
            vec![Step::Welcome, Step::Apps, Step::Game, Step::Look, Step::Overlay, Step::Target, Step::Odds, Step::Done]
        }
    }

    /// Draw the tour in the window body (under the top bar).
    pub(super) fn onboarding(&mut self, ctx: &egui::Context) {
        let steps = self.tour_steps();
        let n = steps.len();
        let i = self.setup.welcome_step.min(n - 1);
        let step = steps[i];
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            kit::settings_style(ui);
            let area = ui.max_rect();
            let maxed = ctx.input(|inp| inp.viewport().maximized.unwrap_or(false));
            sky(ui.painter(), area, if maxed { 0.0 } else { 8.0 });

            let pad = if area.width() > 980.0 { 28.0 } else { 18.0 };
            let inner = area.shrink(pad);
            let wide = inner.width() > 900.0;
            let rail_w = if wide { 252.0 } else { 74.0 };
            let rail = Rect::from_min_size(inner.min, vec2(rail_w, inner.height()));
            let slab = Rect::from_min_max(pos2(rail.max.x + GAP, inner.min.y), inner.max);
            self.tour_rail(ui, rail, &steps, i, wide);
            self.tour_slab(ui, slab, &steps, i, step);
        });
    }

    // ───────────────────────────── the step rail ─────────────────────────────

    fn tour_rail(&mut self, ui: &mut egui::Ui, rect: Rect, steps: &[Step], at: usize, wide: bool) {
        let p = ui.painter().clone();
        p.add(kit::shadow_tier(1).as_shape(rect, SLAB_R));
        p.rect_filled(rect, SLAB_R, CARD());
        p.rect_stroke(rect, SLAB_R, Stroke::new(1.0f32, LINE()));
        let n = steps.len();
        let x = rect.min.x + if wide { 22.0 } else { 0.0 };
        let mut y = rect.min.y + 26.0;
        if wide {
            p.text(pos2(x, y), Align2::LEFT_TOP, if self.returning() { "Quick tour" } else { "Getting started" }, title_font(18.0), TEXT());
            y += 28.0;
            p.text(pos2(x, y), Align2::LEFT_TOP, format!("Step {} of {n}", at + 1), sans_font(12.5), MUTED());
            y += 24.0;
            let bar = Rect::from_min_size(pos2(x, y), vec2(rect.width() - 44.0, 6.0));
            p.rect_filled(bar, 3.0, kit::sunken());
            let f = kit::glide(ui.ctx(), egui::Id::new("tour-progress"), (at + 1) as f32 / n as f32, Some(0.0), kit::ENTER);
            p.add(theme::button_shape(Rect::from_min_size(bar.min, vec2((bar.width() * f).max(6.0), 6.0)), 3.0, false));
            y += 26.0;
        } else {
            y += 4.0;
        }
        let row_h = if wide { 58.0 } else { 54.0 };
        let chip = 34.0;
        // The current step's pill, sliding from row to row.
        if wide {
            let r = Rect::from_min_size(pos2(rect.min.x + 10.0, y + at as f32 * row_h), vec2(rect.width() - 20.0, row_h - 6.0));
            let g = kit::glide_rect(ui.ctx(), egui::Id::new("tour-rail-pill"), r, rect.min, kit::BASE);
            p.rect_filled(g, g.height() / 2.0, kit::sunken());
        }
        for (k, s) in steps.iter().enumerate() {
            let row = if wide {
                Rect::from_min_size(pos2(rect.min.x + 10.0, y), vec2(rect.width() - 20.0, row_h - 6.0))
            } else {
                Rect::from_center_size(pos2(rect.center().x, y + (row_h - 6.0) / 2.0), vec2(rect.width() - 16.0, row_h - 6.0))
            };
            let c = if wide { pos2(row.min.x + 12.0 + chip / 2.0, row.center().y) } else { row.center() };
            // the line joining the chips
            if k + 1 < n {
                let col = if k < at { kit::primary() } else { LINE() };
                p.line_segment([c + vec2(0.0, chip / 2.0 + 3.0), c + vec2(0.0, row_h - chip / 2.0 - 3.0)], Stroke::new(2.0f32, col));
            }
            let reachable = k <= at || (k == at + 1 && self.tour_can_continue(steps[at]));
            let resp = ui.interact(row, ui.id().with(("tour-step", k)), Sense::click());
            let hovered = resp.hovered() && reachable && k != at;
            let ht = kit::anim(ui.ctx(), ui.id().with(("tour-step-hover", k)), hovered && wide, kit::QUICK);
            if ht > 0.0 {
                p.rect_filled(row, row.height() / 2.0, theme::blend(CARD(), TEXT(), 0.03).gamma_multiply(ht));
            }
            let (icon, name, about) = s.meta();
            let cr = Rect::from_center_size(c, vec2(chip, chip));
            if k < at {
                p.add(theme::button_shape(cr, chip / 2.0, hovered));
                tick(&p, c, 6.0, kit::on_primary());
            } else if k == at {
                p.circle_filled(c, chip / 2.0 + 3.0, theme::BUTTON().gamma_multiply(0.18));
                p.add(theme::button_shape(cr, chip / 2.0, false));
                p.text(c, Align2::CENTER_CENTER, icon, egui::FontId::proportional(16.0), kit::on_primary());
            } else {
                p.circle_filled(c, chip / 2.0, kit::sunken());
                p.circle_stroke(c, chip / 2.0, Stroke::new(1.0f32, LINE()));
                p.text(c, Align2::CENTER_CENTER, icon, egui::FontId::proportional(15.0), MUTED());
            }
            if wide {
                let tx = c.x + chip / 2.0 + 12.0;
                let ink = if k <= at { TEXT() } else { MUTED() };
                p.text(pos2(tx, c.y - 8.0), Align2::LEFT_CENTER, name, if k == at { sans_bold_font(13.5) } else { kit::font(13.5) }, ink);
                p.text(pos2(tx, c.y + 9.0), Align2::LEFT_CENTER, about, sans_font(11.5), MUTED());
            }
            let resp = if wide { resp } else { resp.on_hover_text(name) };
            if reachable && k != at && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.setup.welcome_step = k;
            }
            y += row_h;
        }
        // Skip, at the bottom.
        if wide {
            let foot = Rect::from_min_max(pos2(rect.min.x + 22.0, rect.max.y - 62.0), pos2(rect.max.x - 22.0, rect.max.y - 18.0));
            if foot.min.y > y {
                p.line_segment([pos2(foot.min.x, foot.min.y - 6.0), pos2(foot.max.x, foot.min.y - 6.0)], Stroke::new(1.0f32, LINE()));
                let skip = Rect::from_min_size(pos2(foot.min.x, foot.min.y + 2.0), vec2(foot.width(), 20.0));
                let r = ui.interact(skip, ui.id().with("tour-skip"), Sense::click());
                p.text(skip.left_center(), Align2::LEFT_CENTER, "Skip the tour", sans_bold_font(12.5), if r.hovered() { TEXT() } else { MUTED() });
                p.text(pos2(foot.min.x, foot.min.y + 30.0), Align2::LEFT_CENTER, "Run it again any time: Advanced › Updates", sans_font(11.0), MUTED());
                if r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Close the tour; everything stays as it is now").clicked() {
                    self.finish_welcome();
                }
            }
        }
    }

    /// May the tour move past this step? (A target is needed to hunt.)
    fn tour_can_continue(&self, step: Step) -> bool {
        step != Step::Target || self.cfg.target.is_some()
    }

    // ───────────────────────────── the slab ─────────────────────────────

    fn tour_slab(&mut self, ui: &mut egui::Ui, rect: Rect, steps: &[Step], at: usize, step: Step) {
        let p = ui.painter().clone();
        p.add(kit::shadow_tier(2).as_shape(rect, SLAB_R));
        p.add(kit::shadow_tier(1).as_shape(rect, SLAB_R));
        p.rect_filled(rect, SLAB_R, CARD());
        p.rect_stroke(rect, SLAB_R, Stroke::new(1.0f32, LINE()));
        let n = steps.len();
        let foot_h = 76.0;
        let body = Rect::from_min_max(rect.min + vec2(40.0, 34.0), pos2(rect.max.x - 40.0, rect.max.y - foot_h));
        let foot = Rect::from_min_max(pos2(rect.min.x + 32.0, rect.max.y - foot_h), pos2(rect.max.x - 32.0, rect.max.y));

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body), |ui| {
            ui.set_clip_rect(body.expand2(vec2(16.0, 6.0)));
            egui::ScrollArea::vertical()
                .id_salt(("tour-body", at))
                .auto_shrink([false, false])
                .max_height(body.height())
                .show(ui, |ui| {
                    ui.set_width(body.width() - 14.0);
                    // Each step fades in and lifts into place.
                    let t = kit::entering(ui.ctx(), egui::Id::new("tour-step-enter"), at as u64, kit::ENTER);
                    kit::enter_ui(ui, t, 16.0);
                    match step {
                        Step::Welcome => self.tour_welcome(ui),
                        Step::Apps => self.tour_apps(ui),
                        Step::WhatsNew => self.tour_whats_new(ui),
                        Step::Game => self.tour_game(ui),
                        Step::Target => self.tour_target(ui, body.height()),
                        Step::Odds => self.tour_odds(ui),
                        Step::Overlay => self.tour_overlay(ui),
                        Step::Look => self.tour_look(ui),
                        Step::Done => self.tour_done(ui, steps),
                    }
                    ui.add_space(8.0);
                });
        });

        // Footer: Back, the progress dots, Next.
        p.line_segment([pos2(foot.min.x - 8.0, foot.min.y), pos2(foot.max.x + 8.0, foot.min.y)], Stroke::new(1.0f32, LINE()));
        let cy = foot.center().y + 2.0;
        let dots_w = n as f32 * 14.0;
        // (The last step's two buttons need the room; the rail shows where you are.)
        if step != Step::Done {
            for k in 0..n {
                let c = pos2(foot.center().x - dots_w / 2.0 + 7.0 + k as f32 * 14.0, cy);
                p.circle_filled(c, 3.0, if k < at { theme::BUTTON().gamma_multiply(0.55) } else { LINE() });
            }
            // the current step's dash, sliding along the dots
            let c = pos2(foot.center().x - dots_w / 2.0 + 7.0 + at as f32 * 14.0, cy);
            let r = kit::glide_rect(ui.ctx(), egui::Id::new("tour-dot"), Rect::from_center_size(c, vec2(20.0, 6.0)), foot.min, kit::BASE);
            p.add(theme::button_shape(r, 3.0, false));
        }
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(Rect::from_center_size(pos2(foot.center().x, cy), vec2(foot.width(), 44.0))), |ui| {
            ui.horizontal_centered(|ui| {
                if at > 0
                    && ui
                        .add(egui::Button::new(RichText::new("‹  Back").size(13.5)).fill(kit::btn_fill()).min_size(vec2(104.0, 40.0)))
                        .clicked()
                {
                    self.setup.welcome_step = at - 1;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if step == Step::Done {
                        let b = egui::Button::new(RichText::new("▶  Start the overlay").family(kit::strong_family(14.0)).strong().size(14.0).color(kit::on_primary())).min_size(vec2(190.0, 42.0));
                        if kit::accent_button(ui, b, 21.0).clicked() {
                            self.finish_welcome();
                            self.start_overlay();
                        }
                        if ui.add(egui::Button::new(RichText::new("Go to the Dashboard").size(13.5)).fill(kit::btn_fill()).min_size(vec2(120.0, 40.0))).clicked() {
                            self.finish_welcome();
                            self.setup.tab = Tab::Home;
                        }
                    } else {
                        let ready = self.tour_can_continue(step);
                        let label = match step {
                            Step::Welcome => "Let's get set up  ›",
                            Step::WhatsNew => "Check my setup  ›",
                            _ => "Next  ›",
                        };
                        let b = egui::Button::new(RichText::new(label).family(kit::strong_family(14.0)).strong().size(14.0).color(if ready { kit::on_primary() } else { MUTED() }))
                            .min_size(vec2(150.0, 42.0));
                        let resp = if ready { kit::accent_button(ui, b, 21.0) } else { ui.add_enabled(false, b) };
                        let resp = resp.on_disabled_hover_text("Pick a target first");
                        if resp.clicked() && ready {
                            self.setup.welcome_step = at + 1;
                        }
                    }
                });
            });
        });
    }

    // ───────────────────────────── steps ─────────────────────────────

    fn tour_welcome(&mut self, ui: &mut egui::Ui) {
        // A glowing orb with the app's icon and a few sparkles.
        let (hero, _) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
        let c = pos2(hero.min.x + 70.0, hero.center().y);
        orb(ui.painter(), c, 58.0, ui.ctx().input(|i| i.time) as f32);
        ui.painter().image(self.icon_tex.id(), Rect::from_center_size(c, vec2(64.0, 64.0)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        let tx = hero.min.x + 160.0;
        ui.painter().text(pos2(tx, hero.center().y - 26.0), Align2::LEFT_CENTER, "Welcome to the Companion", title_font(32.0), TEXT());
        let sub = ui.painter().layout(
            "Your shiny hunting partner for Platinum Together. It counts every encounter, shows your odds and celebrates every shiny.".into(),
            sans_font(15.0),
            MUTED(),
            (hero.max.x - tx).max(200.0),
        );
        ui.painter().galley(pos2(tx, hero.center().y - 2.0), sub, MUTED());
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
        ui.add_space(10.0);

        section(ui, "What should we call you?");
        ui.horizontal(|ui| {
            let mut name = self.cfg.trainer_name.clone();
            let r = ui.add(
                egui::TextEdit::singleline(&mut name)
                    .hint_text(RichText::new("Your trainer name").color(MUTED()))
                    .char_limit(16)
                    .desired_width(280.0)
                    .min_size(vec2(0.0, 40.0))
                    .vertical_align(egui::Align::Center)
                    .font(sans_font(15.0))
                    .margin(egui::Margin::symmetric(18.0, 8.0)),
            );
            if r.changed() {
                self.cfg.trainer_name = name;
                self.dirty = true;
            }
            ui.label(RichText::new("It goes on your trainer card and profile.").size(12.5).color(MUTED()));
        });
        ui.add_space(18.0);

        section(ui, "What it does");
        feature_grid(
            ui,
            3,
            &[
                ("🔢", kit::C_BLUE, "Counts for you", "Reads every wild encounter from the game, even with the overlay hidden."),
                ("🖥", kit::C_VIOLET, "Sits on the game", "A small overlay with your count, odds and a Caught! button."),
                ("📊", kit::C_CYAN, "Shows your story", "Stats, a Pokédex, the Sinnoh map and today's rare grass."),
            ],
        );
    }

    fn tour_whats_new(&mut self, ui: &mut egui::Ui) {
        let hi = if self.cfg.trainer_name.trim().is_empty() { "Welcome back!".to_string() } else { format!("Welcome back, {}!", self.cfg.trainer_name.trim()) };
        heading(ui, &hi, "Version 0.5 is a big one. Here's what's new, then a quick check of your setup. Your hunts and settings are all kept.");
        feature_grid(
            ui,
            2,
            &[
                ("✨", kit::C_VIOLET, "A fresh look", "A cleaner design with softer controls, new fonts and a proper light mode."),
                ("🖼", kit::C_PINK, "10 overlay themes", "Sticker, Glass, Retro DS, Neon, Pokétch, Field journal, Holo card, Broadcast and Terminal."),
                ("🔔", kit::C_ORANGE, "Themed pop-ups", "Achievements, level ups and notices now match your overlay's theme."),
                ("📊", kit::C_CYAN, "A new Stats page", "Luck, pace, habits and records as dashboards, plus your recent runs."),
                ("📸", kit::C_GOLD, "Share in style", "Hunt cards, trainer cards and rare grass posters in 10 looks."),
                ("🗺", kit::C_GREEN, "A friendlier Map", "Floating search, layers, a key and today's rare grass in reach."),
            ],
        );
    }

    /// New users: which apps to start with. Shiny Hunter is always there;
    /// each pack adds its apps (all can be changed in the App Store).
    fn tour_apps(&mut self, ui: &mut egui::Ui) {
        use crate::apps::{AppId, PACKS};
        heading(ui, "What do you play for?", "Shiny Hunter is always here. Pick what else you'd like and those apps are added; you can add or remove apps any time in the App Store.");
        let gap = 14.0;
        let cols = 2;
        let w = ((ui.available_width() - gap) / cols as f32).floor();
        let mut toggle: Option<usize> = None;
        for (ri, row) in PACKS.chunks(cols).enumerate() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (k, pk) in row.iter().enumerate() {
                    let i = ri * cols + k;
                    let on = pk.apps.iter().all(|a| self.cfg.apps.is_installed(*a));
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 124.0), Sense::click());
                    let p = ui.painter();
                    let hov = resp.hovered();
                    p.rect_filled(r, 20.0, if on { theme::blend(kit::sunken(), kit::primary(), 0.14) } else { kit::sunken() });
                    p.rect_stroke(r, 20.0, Stroke::new(if on { 1.5f32 } else { 1.0f32 }, if on { kit::primary() } else if hov { kit::CONTROL() } else { LINE() }));
                    let mut x = r.min.x + 18.0;
                    for a in pk.apps {
                        let (icon, _, col) = super::shell::tab_meta(super::shell::app_tab(*a));
                        kit::icon_chip(p, Rect::from_min_size(pos2(x, r.min.y + 18.0), vec2(32.0, 32.0)), icon, col, true);
                        x += 38.0;
                    }
                    p.text(pos2(r.min.x + 18.0, r.min.y + 66.0), Align2::LEFT_TOP, format!("{} {}", pk.icon, pk.name), title_font(16.0), TEXT());
                    let names: Vec<&str> = pk.apps.iter().filter(|a| **a != AppId::Shiny).map(|a| a.name()).collect();
                    let g = p.layout(format!("{} · {}", pk.blurb, names.join(", ")), sans_font(12.5), MUTED(), w - 36.0);
                    p.galley(pos2(r.min.x + 18.0, r.min.y + 90.0), g, MUTED());
                    let cb = Rect::from_center_size(pos2(r.max.x - 26.0, r.min.y + 34.0), vec2(22.0, 22.0));
                    p.rect_filled(cb, 7.0, if on { kit::primary() } else { Color32::TRANSPARENT });
                    p.rect_stroke(cb, 7.0, Stroke::new(1.5f32, if on { kit::primary() } else { kit::CONTROL() }));
                    if on {
                        tick(p, cb.center(), 6.0, kit::on_primary());
                    }
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        toggle = Some(i);
                    }
                }
            });
            ui.add_space(gap);
        }
        if let Some(i) = toggle {
            let pk = &PACKS[i];
            let mut list = self.cfg.apps.installed();
            if pk.apps.iter().all(|a| list.contains(a)) {
                // take this pack's apps off, unless another chosen pack has them
                let keep: Vec<AppId> = PACKS
                    .iter()
                    .enumerate()
                    .filter(|(j, o)| *j != i && o.apps.iter().all(|a| list.contains(a)))
                    .flat_map(|(_, o)| o.apps.iter().copied())
                    .collect();
                list.retain(|a| !pk.apps.contains(a) || keep.contains(a));
                let had = list.clone();
                list.retain(|a| a.meta().requires.iter().all(|r| had.contains(r)));
            } else {
                for a in pk.apps {
                    for x in crate::apps::with_requirements(*a, &list.clone()) {
                        list.push(x);
                    }
                }
            }
            self.cfg.apps.set_installed(&list);
            self.dirty = true;
        }
        let n = self.cfg.apps.installed().len();
        ui.label(RichText::new(format!("{n} app{} to start with. Nothing here is final.", if n == 1 { "" } else { "s" })).font(sans_font(13.0)).color(MUTED()));
    }

    fn tour_game(&mut self, ui: &mut egui::Ui) {
        heading(ui, "How counting works", "The Companion counts wild encounters by itself. Choose how it reads them; the game log is exact and needs no setup.");
        self.count_source_picker(ui);
        ui.add_space(16.0);

        // The game folder: where the log and the Map's files are.
        if let Some(rx) = &self.setup.log_picker {
            if let Ok(res) = rx.try_recv() {
                self.setup.log_picker = None;
                if let Some(dir) = res {
                    self.cfg.game_dir = Some(dir.to_string_lossy().to_string());
                    self.game_folder_changed();
                }
            }
        }
        let dir = self.game_folder();
        let logs_ok = dir.join("logs").is_dir();
        section(ui, "Your game");
        info_card(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                kit::icon_chip(ui.painter(), r, "📁", if logs_ok { kit::C_GREEN } else { kit::C_ORANGE }, false);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(RichText::new(if logs_ok { "Found your game" } else { "Game not found yet" }).font(sans_bold_font(13.5)).color(TEXT()));
                    ui.add(egui::Label::new(RichText::new(dir.display().to_string()).size(12.0).color(MUTED())).truncate());
                });
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let s = &self.status;
                match (&s.error, self.cfg.count_source) {
                    (Some(e), _) => {
                        kit::pill(ui, e, kit::PillKind::Bad);
                    }
                    (None, CountSource::GameLog) if !s.text.is_empty() => {
                        kit::pill(ui, &format!("Reading {}", s.text), kit::PillKind::Ok);
                    }
                    (None, CountSource::GameLog) => {
                        kit::pill(ui, "Waiting for the game to start", kit::PillKind::Warn);
                    }
                    (None, CountSource::Screen) => {
                        kit::pill(ui, "You'll draw the name boxes on the Counting page after the tour", kit::PillKind::Info);
                    }
                }
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new("Moved the game? Start it and the Companion finds its folder by itself, or choose the folder with Platinum.exe in it.")
                    .size(12.5)
                    .color(MUTED()),
            );
            ui.add_space(8.0);
            let busy = self.setup.log_picker.is_some();
            if ui.add_enabled(!busy, egui::Button::new(RichText::new("📁  Choose folder…").size(13.0)).fill(kit::btn_fill())).clicked() {
                let start = if dir.is_dir() { dir.clone() } else { dirs::data_local_dir().unwrap_or_default() };
                self.setup.log_picker = Some(crate::share::pick_folder(start, ui.ctx().clone()));
            }
        });
    }

    fn tour_target(&mut self, ui: &mut egui::Ui, h: f32) {
        heading(ui, "What are you hunting?", "Pick the Pokémon you want shiny. You can switch any time, and add more targets from the Hunt page.");
        if let Some(slug) = self.cfg.target.clone() {
            let name = self.species.iter().find(|s| s.slug == slug).map(|s| s.name.to_string()).unwrap_or(slug);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Hunting").size(13.0).color(MUTED()));
                egui::Frame::none()
                    .fill(theme::BUTTON().gamma_multiply(0.14))
                    .stroke(Stroke::new(1.0f32, theme::BUTTON().gamma_multiply(0.5)))
                    .rounding(14.0)
                    .inner_margin(egui::Margin::symmetric(12.0, 4.0))
                    .show(ui, |ui| ui.label(RichText::new(name).font(sans_bold_font(13.0)).color(TEXT())));
            });
            ui.add_space(8.0);
        }
        if let Some(slug) = self.target_picker(ui, (h - 150.0).max(240.0)) {
            self.set_target(slug);
        }
    }

    fn tour_odds(&mut self, ui: &mut egui::Ui) {
        heading(ui, "How are you hunting?", "Your method sets your shiny odds. The Companion shows how you're doing against them as you go.");
        self.method_picker(ui);
        ui.add_space(14.0);
        section(ui, "Singles or doubles?");
        if self.battle_type_picker(ui) {
            self.push_settings(true);
        }
        ui.add_space(16.0);
        section(ui, "Your odds");
        self.odds_tiles(ui);
    }

    fn tour_overlay(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Meet the overlay", "A small counter that sits on top of the game: your count, odds and a Caught! button. Pick a look; you can change it any time.");
        if self.cfg.overlay.style == OverlayStyle::Legacy {
            info_card(ui, |ui| {
                ui.label(RichText::new("You're using the original Legacy overlay. Themes and sizes are part of the Modular one.").size(12.5).color(MUTED()));
                ui.add_space(6.0);
                if ui.add(egui::Button::new(RichText::new("Switch to Modular").size(13.0)).fill(kit::btn_fill())).clicked() {
                    self.cfg.overlay.style = OverlayStyle::Modular;
                    self.ov_size = None;
                    self.dirty = true;
                }
            });
            ui.add_space(14.0);
        } else {
            section(ui, "Theme");
            let avail = ui.available_width();
            let cols = ((avail + 12.0) / (150.0 + 12.0)).floor().clamp(2.0, 5.0);
            let tw = ((avail - 12.0 * (cols - 1.0)) / cols).floor();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                for t in OverlayTheme::ALL {
                    if super::ov_common::theme_tile(ui, t, self.cfg.overlay.theme == t, tw).clicked() && self.cfg.overlay.theme != t {
                        self.cfg.overlay.theme = t;
                        self.ov_size = None;
                        self.dirty = true;
                    }
                }
            });
            ui.add_space(16.0);
            section(ui, "Size");
            let mut size = self.cfg.overlay.size;
            if kit::segmented(ui, &mut size, &[(OverlaySize::Full, "Full"), (OverlaySize::Compact, "Compact"), (OverlaySize::Bar, "Bar")]) {
                self.cfg.overlay.size = size;
                self.ov_size = None;
                self.dirty = true;
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(match size {
                    OverlaySize::Full => "Everything you turn on in Overlay › Sections.",
                    OverlaySize::Compact => "The count, odds and buttons in a smaller box.",
                    OverlaySize::Bar => "One slim line along the top or bottom of the game.",
                })
                .size(12.5)
                .color(MUTED()),
            );
            ui.add_space(16.0);
        }
        section(ui, "Hotkeys");
        info_card(ui, |ui| {
            use crate::hotkeys::HotAction;
            for a in [HotAction::Plus, HotAction::Minus, HotAction::ToggleHide, HotAction::Caught] {
                ui.horizontal(|ui| {
                    ui.set_min_height(28.0);
                    ui.label(RichText::new(a.label()).size(13.0).color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match self.cfg.keybinds.get(a) {
                        Some(b) => kit::keycap(ui, &b.text()),
                        None => {
                            ui.label(RichText::new("Not set").size(12.0).color(MUTED()));
                        }
                    });
                });
            }
            ui.add_space(4.0);
            ui.label(RichText::new("They work while you play. Change them under Hotkeys.").size(12.0).color(MUTED()));
        });
    }

    fn tour_look(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Make it yours", "Light or dark, and your colours. They're used for this window, the overlay and your share images.");
        section(ui, "Mode");
        let w = ((ui.available_width() - 24.0) / 3.0).floor().min(220.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            for (m, label) in [(UiMode::Light, "Light"), (UiMode::SemiDark, "Semi-dark"), (UiMode::Dark, "Dark")] {
                if super::hunt_pages::mode_tile(ui, w, m, label, &self.cfg.appearance).clicked() {
                    self.cfg.appearance.mode = m;
                    self.dirty = true;
                }
            }
        });
        ui.add_space(16.0);
        section(ui, "Your colour");
        let before = self.cfg.appearance.clone();
        {
            let a = &mut self.cfg.appearance;
            super::hunt_pages::swatch_row(ui, &mut a.accent, "tour-accent");
            ui.add_space(10.0);
            kit::toggle_row(ui, "Gradient", "Blend your colour into a second one, on the header and buttons.", &mut a.gradient);
            if a.gradient {
                ui.add_space(6.0);
                ui.label(RichText::new("Second colour").size(12.5).color(MUTED()));
                ui.add_space(4.0);
                super::hunt_pages::swatch_row(ui, &mut a.accent2, "tour-accent2");
            }
        }
        if self.cfg.appearance != before {
            self.dirty = true;
        }
        ui.add_space(16.0);
        section(ui, "Preview");
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(520.0), 64.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(r, 18.0, kit::sunken());
        p.rect_stroke(r, 18.0, Stroke::new(1.0f32, LINE()));
        let b1 = Rect::from_min_size(r.min + vec2(14.0, 14.0), vec2(150.0, 36.0));
        p.add(theme::button_shape(b1, 18.0, false));
        p.text(b1.center(), Align2::CENTER_CENTER, "▶  Start overlay", sans_bold_font(13.0), kit::on_primary());
        let t = Rect::from_min_size(pos2(b1.max.x + 14.0, r.center().y - 12.0), vec2(44.0, 24.0));
        p.add(theme::button_shape(t, 12.0, false));
        p.circle_filled(pos2(t.max.x - 12.0, t.center().y), 9.0, Color32::WHITE);
        let bar = Rect::from_min_size(pos2(t.max.x + 16.0, r.center().y - 4.0), vec2((r.max.x - t.max.x - 32.0).max(40.0), 8.0));
        p.rect_filled(bar, 4.0, CARD());
        p.add(theme::button_shape(Rect::from_min_size(bar.min, vec2(bar.width() * 0.62, 8.0)), 4.0, false));
    }

    fn tour_done(&mut self, ui: &mut egui::Ui, steps: &[Step]) {
        let (hero, _) = ui.allocate_exact_size(vec2(ui.available_width(), 120.0), Sense::hover());
        let c = pos2(hero.min.x + 56.0, hero.center().y);
        orb(ui.painter(), c, 46.0, ui.ctx().input(|i| i.time) as f32);
        ui.painter().add(theme::button_shape(Rect::from_center_size(c, vec2(56.0, 56.0)), 28.0, false));
        tick(ui.painter(), c, 12.0, kit::on_primary());
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
        let tx = hero.min.x + 132.0;
        let name = self.cfg.trainer_name.trim().to_string();
        ui.painter().text(
            pos2(tx, hero.center().y - 18.0),
            Align2::LEFT_CENTER,
            if name.is_empty() { "You're all set!".to_string() } else { format!("You're all set, {name}!") },
            title_font(30.0),
            TEXT(),
        );
        ui.painter().text(pos2(tx, hero.center().y + 14.0), Align2::LEFT_CENTER, "Start the overlay, open the game and go find that shiny.", sans_font(15.0), MUTED());
        ui.add_space(6.0);

        section(ui, "Your setup");
        let target = self
            .cfg
            .target
            .as_ref()
            .map(|slug| self.species.iter().find(|s| &s.slug == slug).map(|s| s.name.to_string()).unwrap_or(slug.clone()))
            .unwrap_or_else(|| "Not picked yet".into());
        let odds = self.cfg.odds_one_in().map(|o| format!("{} · 1 in {}", self.cfg.hunt_method.label(), super::fmt_one_in(o))).unwrap_or_else(|| self.cfg.hunt_method.label().to_string());
        let counting = match self.cfg.count_source {
            CountSource::GameLog => "From the game log",
            CountSource::Screen => "Screen reading (set up the name boxes next)",
        };
        let overlay = if self.cfg.overlay.style == OverlayStyle::Legacy {
            "Legacy".to_string()
        } else {
            format!("{} · {}", self.cfg.overlay.theme.name(), match self.cfg.overlay.size {
                OverlaySize::Full => "Full",
                OverlaySize::Compact => "Compact",
                OverlaySize::Bar => "Bar",
            })
        };
        let look = match self.cfg.appearance.mode {
            UiMode::Light => "Light",
            UiMode::SemiDark => "Semi-dark",
            UiMode::Dark => "Dark",
        };
        let rows: [(Step, &str, egui::Color32, &str, String); 5] = [
            (Step::Game, "🎮", kit::C_GREEN, "Counting", counting.to_string()),
            (Step::Look, "🎨", kit::C_ORANGE, "Look", look.to_string()),
            (Step::Overlay, "🖥", kit::C_VIOLET, "Overlay", overlay),
            (Step::Target, "🎯", kit::C_PINK, "Target", target),
            (Step::Odds, "🎲", kit::C_GOLD, "Method & odds", odds),
        ];
        let mut go = None;
        info_card(ui, |ui| {
            for (k, (s, icon, col, label, value)) in rows.iter().enumerate() {
                if k > 0 {
                    let y = ui.cursor().min.y + 2.0;
                    let x0 = ui.max_rect().min.x;
                    ui.painter().line_segment([pos2(x0, y), pos2(ui.max_rect().max.x, y)], Stroke::new(1.0f32, LINE()));
                    ui.add_space(5.0);
                }
                ui.horizontal(|ui| {
                    ui.set_min_height(38.0);
                    let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                    kit::icon_chip(ui.painter(), r, icon, *col, false);
                    ui.add_space(4.0);
                    ui.label(RichText::new(*label).font(sans_bold_font(13.0)).color(TEXT()));
                    ui.add_space(8.0);
                    ui.add(egui::Label::new(RichText::new(value).size(13.0).color(MUTED())).truncate());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("Change").size(12.5)).fill(kit::btn_fill())).clicked() {
                            go = steps.iter().position(|x| x == s);
                        }
                    });
                });
            }
        });
        if let Some(k) = go {
            self.setup.welcome_step = k;
        }
        ui.add_space(16.0);
        section(ui, "Good to know");
        let hide = self.cfg.keybinds.get(crate::hotkeys::HotAction::ToggleHide).map(|b| b.text()).unwrap_or_else(|| "a hotkey".into());
        let hide_tip = format!("Press {hide} to hide or show it. It keeps counting while hidden.");
        feature_grid(
            ui,
            3,
            &[
                ("▶", kit::C_BLUE, "Start / Stop", "The overlay's button is always at the top right of this window."),
                ("⌨", kit::C_SLATE, "Hide it", &hide_tip),
                ("✨", kit::C_GOLD, "Caught one?", "Press Caught! on the overlay to log it. Then the confetti."),
            ],
        );
    }
}

// ───────────────────────────── pieces ─────────────────────────────

/// A step's title (Inter Tight) and its line under it (Inter).
fn heading(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.label(RichText::new(title).font(title_font(30.0)).color(TEXT()));
    ui.add_space(4.0);
    ui.label(RichText::new(sub).font(sans_font(15.0)).color(MUTED()));
    ui.add_space(22.0);
}

/// A small section title inside a step.
fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).font(title_font(16.0)).color(TEXT()));
    ui.add_space(8.0);
}

/// A Sunken card for grouped details inside the slab.
fn info_card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(kit::sunken())
        .stroke(Stroke::new(1.0f32, LINE()))
        .rounding(20.0)
        .inner_margin(egui::Margin::symmetric(20.0, 16.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
}

/// Feature tiles: icon chip, title, a line about it.
fn feature_grid(ui: &mut egui::Ui, cols: usize, items: &[(&str, Color32, &str, &str)]) {
    let gap = 14.0;
    let w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
    for row in items.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (icon, col, title, about) in row {
                let h = if cols >= 3 { 132.0 } else { 104.0 };
                let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                let p = ui.painter();
                p.rect_filled(r, 20.0, kit::sunken());
                p.rect_stroke(r, 20.0, Stroke::new(1.0f32, LINE()));
                let chip = Rect::from_min_size(r.min + vec2(18.0, 18.0), vec2(36.0, 36.0));
                kit::icon_chip(p, chip, icon, *col, true);
                let (tx, ty, tw) = if cols >= 3 { (r.min.x + 18.0, chip.max.y + 16.0, w - 36.0) } else { (chip.max.x + 14.0, chip.min.y + 9.0, w - 36.0 - 50.0) };
                p.text(pos2(tx, ty), Align2::LEFT_CENTER, *title, title_font(15.0), TEXT());
                let g = p.layout(about.to_string(), sans_font(12.5), MUTED(), tw);
                p.galley(pos2(tx, ty + 12.0), g, MUTED());
            }
        });
        ui.add_space(gap);
    }
}

fn tick(p: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let st = Stroke::new((s / 3.0).max(2.0), col);
    p.line_segment([c + vec2(-s * 0.75, 0.0), c + vec2(-s * 0.2, s * 0.55)], st);
    p.line_segment([c + vec2(-s * 0.2, s * 0.55), c + vec2(s * 0.8, -s * 0.6)], st);
}

/// A soft round glow (a fan of triangles fading out).
fn glow(p: &egui::Painter, c: Pos2, rx: f32, ry: f32, col: Color32) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(c, col);
    let n = 48;
    for k in 0..=n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        m.colored_vertex(c + vec2(a.cos() * rx, a.sin() * ry), Color32::TRANSPARENT);
    }
    for k in 1..=n as u32 {
        m.add_triangle(0, k, k + 1);
    }
    p.add(egui::Shape::mesh(m));
}

/// The hero orb: the accent glowing, two thin rings and drifting sparkles.
fn orb(p: &egui::Painter, c: Pos2, r: f32, t: f32) {
    let pl = theme::pal();
    glow(p, c, r * 1.5, r * 1.5, pl.accent.gamma_multiply(0.35));
    glow(p, c + vec2(r * 0.4, r * 0.3), r * 1.2, r * 1.2, pl.accent2.gamma_multiply(0.25));
    p.circle_filled(c, r * 0.82, CARD());
    p.circle_stroke(c, r * 0.82, Stroke::new(1.0f32, LINE()));
    p.circle_stroke(c, r, Stroke::new(1.0f32, pl.accent.gamma_multiply(0.35)));
    for (k, (dx, dy, s)) in [(-0.95, -0.7, 5.0), (1.0, -0.85, 6.5), (1.1, 0.75, 4.5), (-1.05, 0.8, 4.0)].into_iter().enumerate() {
        let tw = 0.55 + 0.45 * (t * 2.2 + k as f32 * 1.7).sin();
        theme::sparkle(p, c + vec2(dx * r, dy * r), s * (0.7 + 0.3 * tw), theme::GOLD.gamma_multiply(0.5 + 0.5 * tw));
    }
}

/// The Cirrus sky behind the tour: Horizon blue into Sky with soft clouds;
/// in the dark modes a night sky with the accent glowing low.
fn sky(p: &egui::Painter, rect: Rect, corner: f32) {
    let light = theme::pal().light;
    let (top, bottom) = if light {
        (Color32::from_rgb(184, 212, 241), Color32::from_rgb(237, 242, 247))
    } else {
        let page = kit::PAGE();
        (theme::blend(page, Color32::from_rgb(40, 52, 96), 0.55), page)
    };
    // the gradient (square), then the rounded bottom strip in the bottom colour
    let body = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.max.y - corner));
    let mut m = egui::Mesh::default();
    let mid = rect.min.y + rect.height() * 0.78;
    m.colored_vertex(body.left_top(), top);
    m.colored_vertex(body.right_top(), top);
    m.colored_vertex(pos2(body.max.x, mid.min(body.max.y)), bottom);
    m.colored_vertex(pos2(body.min.x, mid.min(body.max.y)), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(m));
    p.rect_filled(Rect::from_min_max(pos2(rect.min.x, mid.min(body.max.y)), rect.max), egui::Rounding { nw: 0.0, ne: 0.0, sw: corner, se: corner }, bottom);
    let clip = p.with_clip_rect(rect.shrink2(vec2(corner, 0.0)));
    let (w, h) = (rect.width(), rect.height());
    if light {
        glow(&clip, rect.min + vec2(w * 0.18, h * 0.78), w * 0.38, h * 0.26, Color32::from_white_alpha(235));
        glow(&clip, rect.min + vec2(w * 0.82, h * 0.70), w * 0.32, h * 0.24, Color32::from_white_alpha(220));
        glow(&clip, rect.min + vec2(w * 0.50, h * 0.16), w * 0.30, h * 0.20, Color32::from_white_alpha(180));
    } else {
        let pl = theme::pal();
        glow(&clip, rect.min + vec2(w * 0.2, h * 0.9), w * 0.4, h * 0.3, pl.accent.gamma_multiply(0.16));
        glow(&clip, rect.min + vec2(w * 0.85, h * 0.15), w * 0.32, h * 0.25, pl.accent2.gamma_multiply(0.12));
        // a few stars
        for k in 0..28u32 {
            let fx = ((k * 7919) % 997) as f32 / 997.0;
            let fy = ((k * 104729) % 991) as f32 / 991.0 * 0.6;
            clip.circle_filled(rect.min + vec2(fx * w, fy * h), if k % 5 == 0 { 1.4 } else { 0.9 }, Color32::from_white_alpha(if k % 3 == 0 { 70 } else { 40 }));
        }
    }
}
