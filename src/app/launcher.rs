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
    /// Start overlay: the picker opens (with last time's picks ticked);
    /// nothing opens until Go. While it's running the same menu stops it.
    pub(super) fn start_overlay(&mut self) {
        self.setup.ov_picker = true;
    }

    /// Open the overlays that are picked. False when none are.
    pub(super) fn launch_overlays(&mut self) -> bool {
        let picks = self.overlay_picks();
        let team = self.team_ov_pick();
        if !picks.0 && !picks.2 && !team {
            return false;
        }
        if picks.0 {
            self.overlay_on = true;
            self.ov_hidden = false;
            self.cfg.mini = false;
        }
        self.setup.gotchi.shown = picks.2;
        self.setup.team_ov.shown = team;
        self.dirty = true;
        true
    }

    /// Is any overlay open?
    pub(super) fn any_overlay(&self) -> bool {
        self.overlay_on || self.setup.gotchi.shown || self.setup.team_ov.shown
    }

    /// Stop closes them all.
    pub(super) fn stop_overlay(&mut self) {
        self.overlay_on = false;
        self.ov_hidden = false;
        self.setup.gotchi.shown = false;
        self.setup.team_ov.shown = false;
        self.save_now();
    }

    /// The big Start / Stop button.
    /// Start / Show / Stop overlay. Returns where it was drawn (the top bar's
    /// is in the Companion's tour).
    pub(super) fn overlay_button(&mut self, ui: &mut egui::Ui, size: egui::Vec2) -> egui::Rect {
        if self.overlay_on && self.ov_hidden {
            let b = egui::Button::new(RichText::new("▶  Show overlay").size(kit::fs(14.0)).family(kit::strong_family(kit::fs(14.0))).strong().color(kit::on_primary()))
                .min_size(size);
            let resp = kit::accent_button(ui, b, kit::primary_round()).on_hover_text("The overlay is minimised and still counting. Bring it back.");
            if resp.clicked() {
                self.ov_hidden = false;
            }
            resp.rect
        } else if self.any_overlay() {
            // opens the overlay menu: stop it there, or change what's open
            let b = egui::Button::new(RichText::new("■  Stop overlay…").size(kit::fs(14.0)).family(kit::strong_family(kit::fs(14.0))).strong().color(TEXT()))
                .fill(kit::btn_fill())
                .rounding(kit::btn_round(8.0))
                .min_size(size);
            let resp = ui.add(b).on_hover_text("The overlay menu: stop the overlay, or change what's open. Counting keeps going if that's on.");
            if resp.clicked() {
                self.start_overlay();
            }
            resp.rect
        } else {
            let b = egui::Button::new(RichText::new("▶  Start overlay").size(kit::fs(14.0)).family(kit::strong_family(kit::fs(14.0))).strong().color(kit::on_primary()))
                .min_size(size);
            let resp = kit::accent_button(ui, b, kit::primary_round()).on_hover_text("Pick which overlays to open on top of the game");
            if resp.clicked() {
                self.start_overlay();
            }
            resp.rect
        }
    }

    /// The Shiny Hunter pages' Start / Stop button. While the overlay is
    /// counting it's "Hunt in progress", with a light travelling round its
    /// edge (the UI Lab's orbit border); a click opens the overlay menu, the
    /// same as Stop overlay… did. Otherwise it's the usual button.
    pub(super) fn hunt_button(&mut self, ui: &mut egui::Ui, size: egui::Vec2) {
        let r = ui.scope(|ui| self.hunt_button_body(ui, size)).response.rect;
        super::app_tour::mark(ui.ctx(), "overlay.button", r);
    }

    fn hunt_button_body(&mut self, ui: &mut egui::Ui, size: egui::Vec2) {
        if self.overlay_on && !self.ov_hidden {
            let tip = format!(
                "Hunting {}: {} so far. Click for the overlay menu: stop the overlay, or change what's open. Counting keeps going if that's on.",
                self.target_label(),
                theme::thousands(self.cfg.count)
            );
            if hunt_live_button(ui, size, &tip).clicked() {
                self.start_overlay();
            }
        } else {
            self.overlay_button(ui, size);
        }
    }

    /// New hunt: the Hunt page, with the step-by-step guide open.
    /// (or, with the guide turned off, the Pokémon picker as before).
    pub(super) fn open_target_picker(&mut self) {
        if self.cfg.hunt_guide_off {
            self.open_quick_picker();
        } else {
            self.open_hunt_wizard();
        }
    }

    /// The Hunt page with its Pokémon picker open (the way without the guide).
    pub(super) fn open_quick_picker(&mut self) {
        self.setup.hunt_wizard = None;
        self.setup.tab = Tab::Hunt;
        self.setup.picking_target = true;
        self.setup.picking_also = None;
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
                    self.go_to(Tab::Counting);
                    self.clear_notice(id);
                }
                NoticeAction::UseRunningGame => self.use_running_game_folder(),
                NoticeAction::Preview => self.clear_notice(id),
                NoticeAction::WatchReplay => self.replay_watch_latest(),
                NoticeAction::OpenPacks => self.notice_open_packs(id),
            }
        }
    }

    // ───────────────────────────── Shiny Hunter ─────────────────────────────

    pub(super) fn page_shiny(&mut self, ui: &mut egui::Ui) {

        super::app_tour::here(ui.ctx(), "shiny");
        kit::page_header(ui, "SHINY HUNTER", "Count encounters and catch shinies, with an overlay on top of the game.");
        let tour_top = ui.cursor().min.y;
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
                            .add(egui::Button::new(RichText::new("+ New hunt").size(kit::fs(12.5))).fill(kit::btn_fill()))
                            .on_hover_text("Set up a new hunt, step by step")
                            .clicked()
                        {
                            self.open_target_picker();
                        }
                    });
                    // Button and status flush right, stacked.
                    ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                        ui.add_space(14.0);
                        self.hunt_button(ui, egui::vec2(180.0, 40.0));
                        ui.add_space(4.0);
                        self.overlay_status(ui);
                    });
                });
            });
        }
        super::app_tour::mark_since(ui, "shiny.hero", tour_top);


        let tour_top = ui.cursor().min.y;
        kit::section(ui, "Shiny Hunter pages");
        if kit::modern() {
            let w = ui.available_width();
            let pages = self.cfg.apps.shiny_pages_in_app;
            self.page_tiles(
                ui,
                w,
                &[
                    (Tab::Hunt, "Target, method and odds."),
                    (Tab::Counting, if pages { "Game log or screen reading." } else { "In Settings: game log or screen reading." }),
                    (Tab::Runs, "Every hunt, with stats."),
                    (Tab::Dex, "Your shinies, in the Pokédex."),
                    (Tab::Overlay, if pages { "Layout, sections and sounds." } else { "In Settings: layout, sections and sounds." }),
                ],
                3,
            );
            super::app_tour::mark_since(ui, "shiny.pages", tour_top);
            return;
        }
        let pages = [
            (Tab::Hunt, "🎯", "Hunt", "Target, method and odds"),
            (Tab::Counting, "👁", "Counting", "In Settings: game log or screen reading"),
            (Tab::Runs, "📜", "Runs", "Every hunt, with stats"),
            (Tab::Dex, "⭐", "Shiny Dex", "Your shinies, in the Pokédex"),
            (Tab::Overlay, "🖵", "Overlay", "In Settings: layout, sections, sounds"),
        ];
        let w = ((ui.available_width() - 16.0) / 3.0).floor();
        for row in pages.chunks(3) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (tab, icon, title, sub) in row {
                    if kit::choice_tile(ui, egui::vec2(w, 80.0), false, icon, title, sub, "").clicked() {
                        self.go_to(*tab);
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
                        self.hunt_button(ui, egui::vec2(140.0, 28.0));
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


/// "Hunt in progress": a calm button with a comet of light orbiting its
/// edge (a turning conic gradient seen through a 2px border, like the UI
/// Lab's orbit border), a soft glow behind the comet and a breathing "live"
/// dot. With Animations off the comet holds still.
fn hunt_live_button(ui: &mut egui::Ui, size: egui::Vec2, tip: &str) -> egui::Response {
    use eframe::egui::{pos2, vec2, Align2, Pos2, Rect, Stroke};
    use std::f32::consts::{FRAC_PI_2, TAU};
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    let resp = resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
    if !ui.is_rect_visible(r) {
        return resp;
    }
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let round = kit::btn_round(8.0).min(size.y / 2.0);
    let p = ui.painter();
    let motion = kit::motion();
    let time = ui.input(|i| i.time) as f32;
    let accent = kit::primary();
    let accent2 = Color32::from_rgb(244, 114, 182);
    // one turn every 2.4 s, a little quicker on hover
    let turn = if motion { (time / (2.4 - 0.6 * ht)).fract() } else { 0.12 };

    // the comet's colour at a point on the edge: clear for most of the way
    // round, then the accent, then pink, then fading out
    let comet = |q: Pos2| -> Color32 {
        let d = q - r.center();
        let ang = (d.x.atan2(-d.y) / TAU).rem_euclid(1.0);
        let f = (ang - turn).rem_euclid(1.0);
        let mix = |a: Color32, b: Color32, t: f32| -> Color32 {
            let (a, b) = (a.to_array(), b.to_array());
            let l = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
            Color32::from_rgba_premultiplied(l(0), l(1), l(2), l(3))
        };
        if f < 0.7 {
            Color32::TRANSPARENT
        } else if f < 0.8 {
            mix(Color32::TRANSPARENT, accent, (f - 0.7) / 0.1)
        } else if f < 0.9 {
            mix(accent, accent2, (f - 0.8) / 0.1)
        } else {
            mix(accent2, Color32::TRANSPARENT, (f - 0.9) / 0.1)
        }
    };
    // points round a rounded rectangle, clockwise from the top middle's right
    let ring = |rect: Rect, rad: f32| -> Vec<Pos2> {
        let rad = rad.min(rect.height() / 2.0).min(rect.width() / 2.0).max(0.0);
        let cs = [
            (pos2(rect.max.x - rad, rect.min.y + rad), -FRAC_PI_2),
            (pos2(rect.max.x - rad, rect.max.y - rad), 0.0),
            (pos2(rect.min.x + rad, rect.max.y - rad), FRAC_PI_2),
            (pos2(rect.min.x + rad, rect.min.y + rad), FRAC_PI_2 * 2.0),
        ];
        let mut v = Vec::with_capacity(4 * 13);
        for (c, a0) in cs {
            for i in 0..=12 {
                let a = a0 + FRAC_PI_2 * i as f32 / 12.0;
                v.push(c + vec2(a.cos(), a.sin()) * rad);
            }
        }
        v
    };
    // a band between two rings, coloured by `col` on the outer edge and
    // `col_in` on the inner one
    let band = |outer: &[Pos2], inner: &[Pos2], col: &dyn Fn(Pos2) -> Color32, col_in: &dyn Fn(Pos2) -> Color32| -> egui::Mesh {
        let mut m = egui::Mesh::default();
        let n = outer.len() as u32;
        for (o, i) in outer.iter().zip(inner) {
            m.colored_vertex(*o, col(*o));
            m.colored_vertex(*i, col_in(*i));
        }
        for k in 0..n {
            let (a, b) = (2 * k, 2 * ((k + 1) % n));
            m.add_triangle(a, b, a + 1);
            m.add_triangle(b, b + 1, a + 1);
        }
        m
    };
    let edge = ring(r, round);
    // a soft glow just outside, under the comet
    let halo = ring(r.expand(4.0), round + 4.0);
    p.add(band(&halo, &edge, &|_| Color32::TRANSPARENT, &|q| comet(q).gamma_multiply(0.35)));
    // the button
    p.rect_filled(r, round, theme::blend(kit::CARD(), TEXT(), 0.03 + 0.05 * ht));
    p.rect_stroke(r, round, Stroke::new(1.0f32, theme::blend(kit::CARD(), TEXT(), 0.12)));
    // the comet, seen through a 2px border
    let inner = ring(r.shrink(2.0), (round - 2.0).max(0.0));
    p.add(band(&edge, &inner, &comet, &comet));
    // a breathing "live" dot and the words
    let font = egui::FontId::new(kit::fs(13.5), kit::strong_family(kit::fs(13.5)));
    let g = p.layout_no_wrap("Hunt in progress".to_string(), font, TEXT());
    let dot_r = 3.5;
    let total = dot_r * 2.0 + 8.0 + g.size().x;
    let x0 = r.center().x - total / 2.0;
    let breathe = if motion { 0.55 + 0.45 * (time * 3.0).sin().abs() } else { 1.0 };
    let live = Color32::from_rgb(52, 211, 153);
    p.circle_filled(pos2(x0 + dot_r, r.center().y), dot_r + 3.0 * breathe, live.gamma_multiply(0.18 * breathe));
    p.circle_filled(pos2(x0 + dot_r, r.center().y), dot_r, live);
    p.galley(pos2(x0 + dot_r * 2.0 + 8.0, r.center().y - g.size().y / 2.0), g, TEXT());
    let _ = Align2::CENTER_CENTER;
    if motion {
        // 30 frames a second is plenty for it, and easy on the PC while hunting
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
    resp
}

impl App {
    /// Settings → Shiny Hunter → General: the overlay window's options
    /// (they were on the Shiny Hunter page).
    pub(super) fn shiny_window_settings(&mut self, ui: &mut egui::Ui) {
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
    }
}
