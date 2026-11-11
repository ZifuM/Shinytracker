//! The settings window: sidebar + six pages, and the first-run welcome.

use super::ui_kit::{self as kit, MUTED, PillKind, CARD, CARD_HOVER, LINE, PAGE, SIDEBAR, TEXT};
use super::{fmt_one_in, App, RunRef, Tab};
use crate::config::{BattleType, Config, CountSource, HuntMethod};
use crate::hotkeys::{code_for, HotAction, KeyBind, KeyBinds};
use crate::theme::{self, GOLD};
use eframe::egui::{self, Color32, Rect, RichText, Sense};
use std::time::{Duration, Instant};


fn method_icon(m: HuntMethod) -> &'static str {
    match m {
        HuntMethod::RareGrassSweetScent => "✨",
        HuntMethod::SweetScent => "🌿",
        HuntMethod::FullOdds => "🚶",
        HuntMethod::SoftReset => "🔄",
        HuntMethod::PokeRadar => "📡",
        HuntMethod::Masuda => "🐣",
        HuntMethod::FromGameLog => "📄",
        HuntMethod::Custom => "✏",
    }
}

fn method_short(m: HuntMethod) -> (&'static str, &'static str) {
    match m {
        HuntMethod::RareGrassSweetScent => ("Rare grass", "Sweet Scent"),
        HuntMethod::SweetScent => ("Sweet Scent", "Normal grass"),
        HuntMethod::FullOdds => ("Full odds", "Walking around"),
        HuntMethod::SoftReset => ("Soft resets", "Starters, legendaries"),
        HuntMethod::PokeRadar => ("Poké Radar", "Chaining"),
        HuntMethod::Masuda => ("Masuda", "Breeding"),
        HuntMethod::FromGameLog => ("From the game", "Exact log odds"),
        HuntMethod::Custom => ("Custom", "Enter odds"),
    }
}

/// Tabs on the Hunt settings page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum HuntTab {
    #[default]
    Target,
    Method,
    Session,
}

/// Tabs on the Advanced settings page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum AdvTab {
    #[default]
    Updates,
    Data,
    Counter,
    Reading,
}

/// Tabs on the Overlay settings page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum OverlayTab {
    #[default]
    Layout,
    Sections,
    Behaviour,
    Sounds,
}

/// Methods on offer. "From the game" and "Custom" are no longer offered
/// (they still work for anyone who already picked one).
fn method_offered(m: HuntMethod) -> bool {
    !matches!(m, HuntMethod::FromGameLog | HuntMethod::Custom)
}

/// Egg hunting isn't well tested yet.
fn method_experimental(m: HuntMethod) -> bool {
    matches!(m, HuntMethod::Masuda)
}

const EXPERIMENTAL_NOTE: &str = "Experimental: issues may occur with this hunting style.";

/// Small amber "issues may occur" chip in a method tile's top-right corner
/// (left of the tick when it's selected).
fn warn_chip(ui: &egui::Ui, tile: Rect, selected: bool) {
    let p = ui.painter();
    let font = egui::FontId::proportional(kit::fs(10.5));
    let ink = theme::HP_YELLOW;
    let galley = p.layout_no_wrap("⚠ Issues may occur".to_string(), font, ink);
    let right = tile.right() - if selected { 32.0 } else { 10.0 };
    let r = Rect::from_min_size(
        egui::pos2(right - galley.size().x - 12.0, tile.top() + 9.0),
        galley.size() + egui::vec2(12.0, 4.0),
    );
    p.rect_filled(r, r.height() / 2.0, ink.gamma_multiply(0.16));
    p.rect_stroke(r, r.height() / 2.0, egui::Stroke::new(1.0f32, ink.gamma_multiply(0.6)));
    p.galley(r.min + egui::vec2(6.0, 2.0), galley, ink);
}

/// "1 in 2,048" for a method, using the rest of the current settings.
fn method_badge(cfg: &Config, m: HuntMethod) -> String {
    let mut c = cfg.clone();
    c.hunt_method = m;
    match (m, c.odds_one_in()) {
        (HuntMethod::PokeRadar, _) => "up to 1 in 200".into(),
        (_, Some(x)) => format!("1 in {}", fmt_one_in(x)),
        (_, None) => "after 1st battle".into(),
    }
}

/// "UNDER ODDS · 0.3×" style label + colour for a count.
pub(super) fn odds_state(cfg: &Config, count: u64) -> Option<(String, Color32)> {
    let one_in = cfg.odds_one_in()?;
    let mult = count as f64 / one_in;
    let m = if mult < 0.1 {
        format!("{mult:.2}×")
    } else {
        format!("{mult:.1}×")
    };
    Some(if mult < 0.9 {
        (format!("Under odds · {m}"), theme::GREEN)
    } else if mult <= 1.1 {
        (format!("At odds · {m}"), theme::HP_YELLOW)
    } else if mult < 2.0 {
        (format!("Over odds · {m}"), theme::HP_RED)
    } else {
        (format!("{m} over odds"), theme::HP_RED)
    })
}

impl App {
    // ─────────────────────────── window ───────────────────────────

    /// The launcher: the main window, with the sidebar and every page.
    pub(super) fn launcher_ui(&mut self, ctx: &egui::Context) {
        {
            let screen = ctx.screen_rect();
            let (maxed, minimized, outer, inner) = ctx.input(|i| {
                let v = i.viewport();
                (v.maximized.unwrap_or(false), v.minimized.unwrap_or(false), v.outer_rect, v.inner_rect)
            });
            self.remember_settings_geometry(maxed, minimized, outer, inner);
            // Match Windows 11's own rounded window corners (8px); square when maximized.
            let r = if maxed { 0.0 } else { 8.0 };
            crate::topmost::round_settings_corners(&mut self.settings_rounded);
            // Header bar like the overlay's: drag to move, minimise, close.
            egui::TopBottomPanel::top("settings-header")
                .exact_height(38.0)
                .frame(egui::Frame::none())
                .show_separator_line(false)
                .show(ctx, |ui| self.settings_header(ctx, ui));
            let body_top = screen.min.y + 38.0;
            let bg = ctx.layer_painter(egui::LayerId::background());
            bg.rect_filled(
                Rect::from_min_max(egui::pos2(screen.min.x, body_top - 1.0), screen.max),
                egui::Rounding { nw: 0.0, ne: 0.0, sw: r, se: r },
                PAGE(),
            );
            if !self.cfg.welcome_done {
                self.welcome(ctx);
            } else {
                // (Panels draw on the background layer, so the sidebar fills
                // itself rather than being painted over afterwards.)
                egui::SidePanel::left("settings-nav")
                    .exact_width(184.0)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(
                        egui::Frame::none()
                            .fill(SIDEBAR())
                            .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: r, se: 0.0 })
                            .inner_margin(egui::Margin::symmetric(10.0, 14.0)),
                    )
                    .show(ctx, |ui| {
                        kit::settings_style(ui);
                        self.sidebar(ui)
                    });
                // The Map fills the whole pane, edge to edge, with its
                // controls floating on top.
                let map_full = self.setup.tab == Tab::Map
                    && !self.setup.caught_pick
                    && !self.setup.phase_pick
                    && self.setup.profile_card.is_none()
                    && self.setup.share.is_none();
                if map_full {
                    egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
                        kit::settings_style(ui);
                        self.page_map(ui);
                    });
                } else {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().inner_margin(if kit::clean() {
                        egui::Margin {
                            left: 32.0,
                            // Small: the scroll bar's own lane (8px bar + 12px gap)
                            // already spaces the page from the edge.
                            right: 6.0,
                            top: 26.0,
                            bottom: 12.0,
                        }
                    } else {
                        egui::Margin::symmetric(26.0, 20.0)
                    }))
                    .show(ctx, |ui| {
                        kit::settings_style(ui);
                        // Scroll bar in its own lane beside the page (egui's
                        // default floats it over the content).
                        ui.style_mut().spacing.scroll = egui::style::ScrollStyle {
                            bar_width: 8.0,
                            bar_inner_margin: 12.0,
                            bar_outer_margin: 0.0,
                            ..egui::style::ScrollStyle::solid()
                        };
                        // The Pokédex grid keeps its own scroll spot while an entry is open.
                        let scroll_key = if self.setup.tab == Tab::Pokedex {
                            (self.setup.pokedex.open.clone(), self.setup.pokedex.grid_gen)
                        } else {
                            (None, 0)
                        };
                        // Every page and tab opens at its top: one shared scroll
                        // spot used to carry over, so a new page could open
                        // half-way down with its tabs out of sight.
                        let page_key = format!(
                            "{}{}{}{}{}{}{:?}{}{}{}{}{}{:?}",
                            self.setup.tab as u8,
                            self.setup.hunt_tab as u8,
                            self.setup.overlay_tab as u8,
                            self.setup.appear_tab as u8,
                            self.setup.adv_tab as u8,
                            self.setup.profile_section as u8,
                            self.setup.view_run,
                            self.setup.run_tab as u8,
                            self.setup.share.is_some() as u8,
                            self.setup.profile_card.is_some() as u8,
                            self.setup.phase_pick as u8,
                            self.setup.caught_pick as u8,
                            (self.setup.picking_target, self.setup.picking_also),
                        );
                        let key_id = egui::Id::new("settings-page-key");
                        let last: Option<String> = ui.data(|d| d.get_temp(key_id));
                        if last.as_deref() != Some(page_key.as_str()) {
                            ui.data_mut(|d| d.insert_temp(key_id, page_key));
                            // (The Pokédex keeps its own spot per entry and grid.)
                            if last.is_some() && self.setup.tab != Tab::Pokedex {
                                self.setup.scroll_top = true;
                            }
                        }
                        let mut area = egui::ScrollArea::vertical()
                            .id_salt(("settings-page", scroll_key))
                            .auto_shrink([false, false]);
                        if std::mem::take(&mut self.setup.scroll_top) {
                            area = area.vertical_scroll_offset(0.0);
                        }
                        let page_rect = ui.max_rect();
                        let scrolled = area
                            .show(ui, |ui| {
                                // Cap the page width, but never wider than the window
                                // (set_max_width can also widen, which cut the right side off).
                                // Gallery pages (dex, achievements, trophies, runs) fill the window.
                                let wide = self.setup.phase_pick
                                    || self.setup.caught_pick
                                    || (self.setup.profile_card.is_none()
                                        && self.setup.share.is_none()
                                        && matches!(self.setup.tab, Tab::Home | Tab::Profile | Tab::Dex | Tab::Trophies | Tab::Runs | Tab::Pokedex | Tab::Map));
                                let w = if wide { ui.available_width() } else { ui.available_width().min(900.0) };
                                ui.set_max_width(w);
                                if self.setup.caught_pick {
                                    self.page_caught_pick(ui);
                                } else if self.setup.phase_pick {
                                    self.page_phase(ui);
                                } else if self.setup.profile_card.is_some() {
                                    self.page_profile_card(ui);
                                } else if let Some(src) = self.setup.share.as_ref().map(|s| s.src) {
                                    self.page_share(ui, src);
                                } else if let (Tab::Runs, Some(run)) = (self.setup.tab, self.setup.view_run) {
                                    self.page_run_stats(ui, run);
                                } else {
                                    // Shiny Hunter pages carry the Start / Stop strip.
                                    if super::launcher::is_shiny_tab(self.setup.tab) && self.setup.tab != Tab::Shiny {
                                        self.overlay_strip(ui);
                                    }
                                    match self.setup.tab {
                                        Tab::Home => self.page_home(ui),
                                        Tab::Shiny => self.page_shiny(ui),
                                        Tab::Pokedex => self.page_pokedex(ui),
                                        Tab::Map => self.page_map(ui),
                                        Tab::Profile => self.page_profile(ui),
                                        Tab::Dex => self.page_dex(ui),
                                        Tab::Hunt => self.page_hunt(ui),
                                        Tab::Counting => self.page_counting(ui),
                                        Tab::Runs => self.page_runs(ui),
                                        Tab::Trophies => self.page_dex(ui),
                                        Tab::Overlay => self.page_overlay(ui),
                                        Tab::Appearance => self.page_appearance(ui),
                                        Tab::Hotkeys => self.page_hotkeys(ui),
                                        Tab::Advanced => self.page_advanced(ui),
                                    }
                                }
                                ui.add_space(24.0);
                            })
                            .state
                            .offset
                            .y;
                        // Far down a long page: a button back to the top, where
                        // the page's tabs are.
                        if scrolled > 240.0 {
                            let size = egui::vec2(84.0, 30.0);
                            let at = page_rect.right_bottom() - egui::vec2(size.x + 28.0, size.y + 10.0);
                            let up = egui::Area::new(egui::Id::new("settings-to-top"))
                                .order(egui::Order::Foreground)
                                .fixed_pos(at)
                                .show(ui.ctx(), |ui| {
                                    ui.add(
                                        egui::Button::new(RichText::new("↑  Top").size(kit::fs(12.5)).strong().color(TEXT()))
                                            .fill(CARD_HOVER())
                                            .stroke(egui::Stroke::new(1.0f32, LINE()))
                                            .rounding(15.0)
                                            .min_size(size),
                                    )
                                    .on_hover_text("Back to the top of the page")
                                    .clicked()
                                })
                                .inner;
                            if up {
                                self.setup.scroll_top = true;
                            }
                        }
                    });
                }
            }
            // Outline on top, like the overlay; resize grip bottom-right
            // (neither when maximized).
            let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("settings-frame")));
            if !maxed {
                fg.rect_stroke(screen.shrink(0.5), r, egui::Stroke::new(1.5f32, theme::OUTLINE));
            }
            if !maxed {
                let grip = Rect::from_min_size(screen.max - egui::vec2(16.0, 16.0), egui::vec2(16.0, 16.0));
                for k in 0..3 {
                    let o = 4.0 + k as f32 * 4.0;
                    fg.line_segment(
                        [grip.max - egui::vec2(o, 3.0), grip.max - egui::vec2(3.0, o)],
                        egui::Stroke::new(1.2f32, MUTED().gamma_multiply(0.7)),
                    );
                }
                let gr = egui::Area::new(egui::Id::new("settings-grip"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(grip.min)
                    .show(ctx, |ui| ui.allocate_response(grip.size(), Sense::drag()))
                    .inner;
                if gr.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                }
                if gr.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(egui::viewport::ResizeDirection::SouthEast));
                }
                // Every edge and corner resizes too, like a normal window.
                use egui::viewport::ResizeDirection as Dir;
                use egui::CursorIcon as Cur;
                let (t, c) = (5.0, 14.0);
                let s = screen;
                let zones = [
                    (Rect::from_min_max(egui::pos2(s.min.x + c, s.min.y), egui::pos2(s.max.x - c, s.min.y + t)), Dir::North, Cur::ResizeVertical),
                    (Rect::from_min_max(egui::pos2(s.min.x + c, s.max.y - t), egui::pos2(s.max.x - 16.0, s.max.y)), Dir::South, Cur::ResizeVertical),
                    (Rect::from_min_max(egui::pos2(s.min.x, s.min.y + c), egui::pos2(s.min.x + t, s.max.y - c)), Dir::West, Cur::ResizeHorizontal),
                    (Rect::from_min_max(egui::pos2(s.max.x - t, s.min.y + c), egui::pos2(s.max.x, s.max.y - 16.0)), Dir::East, Cur::ResizeHorizontal),
                    (Rect::from_min_size(s.min, egui::vec2(c, t)), Dir::NorthWest, Cur::ResizeNwSe),
                    (Rect::from_min_size(s.min, egui::vec2(t, c)), Dir::NorthWest, Cur::ResizeNwSe),
                    (Rect::from_min_max(egui::pos2(s.max.x - c, s.min.y), egui::pos2(s.max.x, s.min.y + t)), Dir::NorthEast, Cur::ResizeNeSw),
                    (Rect::from_min_max(egui::pos2(s.max.x - t, s.min.y), egui::pos2(s.max.x, s.min.y + c)), Dir::NorthEast, Cur::ResizeNeSw),
                    (Rect::from_min_max(egui::pos2(s.min.x, s.max.y - t), egui::pos2(s.min.x + c, s.max.y)), Dir::SouthWest, Cur::ResizeNeSw),
                    (Rect::from_min_max(egui::pos2(s.min.x, s.max.y - c), egui::pos2(s.min.x + t, s.max.y)), Dir::SouthWest, Cur::ResizeNeSw),
                ];
                for (k, (r, dir, cur)) in zones.into_iter().enumerate() {
                    let er = egui::Area::new(egui::Id::new(("settings-edge", k)))
                        .order(egui::Order::Foreground)
                        .fixed_pos(r.min)
                        .show(ctx, |ui| ui.allocate_response(r.size(), Sense::drag()))
                        .inner;
                    if er.hovered() || er.dragged() {
                        ctx.set_cursor_icon(cur);
                    }
                    if er.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
                    }
                }
            }
        }
    }

    /// The accent header bar across the top of the settings window.
    fn settings_header(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        theme::header_bar(&p, rect, if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) { 0.0 } else { 8.0 });
        p.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(2.0f32, theme::RED_DARK()),
        );
        p.image(
            self.icon_tex.id(),
            Rect::from_center_size(rect.left_center() + egui::vec2(20.0, 0.0), egui::vec2(22.0, 22.0)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        theme::outlined_text(
            &p,
            rect.left_center() + egui::vec2(36.0, 1.0),
            egui::Align2::LEFT_CENTER,
            "POKE TOGETHER COMPANION",
            10.0,
            Color32::WHITE,
        );
        let maxed = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let bw = 34.0;
        let close = Rect::from_min_size(egui::pos2(rect.max.x - bw - 4.0, rect.min.y + 4.0), egui::vec2(bw, rect.height() - 8.0));
        let max = close.translate(egui::vec2(-bw - 2.0, 0.0));
        let min = max.translate(egui::vec2(-bw - 2.0, 0.0));
        let drag = Rect::from_min_max(rect.min, egui::pos2(min.min.x - 4.0, rect.max.y));
        let dr = ui.interact(drag, ui.id().with("settings-drag"), Sense::click_and_drag());
        if dr.double_clicked() {
            // Double-click the title bar to maximize / restore, like Windows.
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed));
        } else if dr.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        for (i, r) in [min, max, close].into_iter().enumerate() {
            let resp = ui.interact(r, ui.id().with(("hdr-btn", i)), Sense::click());
            if resp.hovered() {
                p.rect_filled(r, 6.0, Color32::from_black_alpha(50));
            }
            let c = r.center();
            let s = egui::Stroke::new(2.0f32, Color32::WHITE);
            if i == 0 {
                p.line_segment([c - egui::vec2(6.0, -1.0), c + egui::vec2(6.0, 1.0)], s);
                if resp.on_hover_text("Minimise").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            } else if i == 1 {
                let thin = egui::Stroke::new(1.6f32, Color32::WHITE);
                if maxed {
                    // "Restore": two overlapping squares
                    let back = Rect::from_center_size(c + egui::vec2(1.5, -1.5), egui::vec2(9.0, 9.0));
                    let front = Rect::from_center_size(c + egui::vec2(-1.5, 1.5), egui::vec2(9.0, 9.0));
                    p.line_segment([back.left_top(), back.right_top()], thin);
                    p.line_segment([back.right_top(), back.right_bottom()], thin);
                    p.rect_stroke(front, 1.0, thin);
                } else {
                    p.rect_stroke(Rect::from_center_size(c, egui::vec2(11.0, 11.0)), 1.0, thin);
                }
                if resp.on_hover_text(if maxed { "Restore" } else { "Maximise" }).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed));
                }
            } else {
                p.line_segment([c - egui::vec2(5.5, 5.5), c + egui::vec2(5.5, 5.5)], s);
                p.line_segment([c + egui::vec2(-5.5, 5.5), c + egui::vec2(5.5, -5.5)], s);
                if resp.on_hover_text("Close the Companion (saves everything)").clicked() {
                    self.quit_app(ctx);
                }
            }
        }
    }

    /// Remember the settings window's spot, size and maximized state so it
    /// reopens the same way (saved quietly, without the "✔ Saved" flash).
    fn remember_settings_geometry(&mut self, maxed: bool, minimized: bool, outer: Option<Rect>, inner: Option<Rect>) {
        if minimized {
            return;
        }
        if maxed != self.cfg.settings_maximized {
            self.cfg.settings_maximized = maxed;
            self.geom_dirty = true;
        }
        // Only the normal (not maximized) size is kept, so "restore" goes back to it.
        if maxed {
            return;
        }
        if let (Some(o), Some(i)) = (outer, inner) {
            let pos = [o.min.x, o.min.y];
            let size = [i.width(), i.height()];
            let differs = |a: Option<[f32; 2]>, b: [f32; 2]| {
                a.map_or(true, |a| (a[0] - b[0]).abs() > 0.5 || (a[1] - b[1]).abs() > 0.5)
            };
            if differs(self.cfg.settings_pos, pos) || differs(self.cfg.settings_size, size) {
                self.cfg.settings_pos = Some(pos);
                self.cfg.settings_size = Some(size);
                self.geom_dirty = true;
            }
        }
    }

    /// Hunt page: extra targets hunted alongside the main one (two Pokémon on
    /// the same route, or more).
    fn second_target_row(&mut self, ui: &mut egui::Ui) {
        const MAX_ALSO: usize = 5;
        let also = self.also_slugs();
        if also.is_empty() {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Also hunting (optional)").size(kit::fs(13.0)).color(TEXT()));
                    ui.label(
                        RichText::new("Hunting more than one Pokémon on the same route? They all count as sightings, and Caught! asks which one.")
                            .size(kit::fs(11.5))
                            .color(MUTED()),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if self.setup.picking_also.is_some() { "Done" } else { "Add" };
                    if ui.add(egui::Button::new(RichText::new(label).size(kit::fs(12.5))).fill(kit::btn_fill())).clicked() {
                        self.setup.picking_also = if self.setup.picking_also.is_some() { None } else { Some(0) };
                        self.setup.picking_target = false;
                    }
                });
            });
        } else {
            ui.label(RichText::new("Also hunting").size(kit::fs(11.0)).color(MUTED()));
            ui.add_space(4.0);
            let mut remove: Option<usize> = None;
            for (k, slug) in also.iter().enumerate() {
                let Some(i) = self.species.iter().position(|s| &s.slug == slug) else { continue };
                let sp = self.species[i].clone();
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::hover());
                    let p = ui.painter_at(r);
                    p.rect_filled(r, 20.0, CARD_HOVER());
                    self.draw_sprite(&p, &sp, false, r.center_bottom() - egui::vec2(0.0, 3.0), egui::vec2(36.0, 36.0));
                    ui.label(RichText::new(format!("{}  #{:04}", sp.name, sp.dex)).size(kit::fs(14.0)).strong().color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("Remove").size(kit::fs(12.5))).fill(kit::btn_fill())).clicked() {
                            remove = Some(k);
                        }
                        let picking = self.setup.picking_also == Some(k);
                        let label = if picking { "Done" } else { "Change" };
                        if ui.add(egui::Button::new(RichText::new(label).size(kit::fs(12.5))).fill(kit::btn_fill())).clicked() {
                            self.setup.picking_also = if picking { None } else { Some(k) };
                            self.setup.picking_target = false;
                        }
                    });
                });
                ui.add_space(4.0);
            }
            if let Some(k) = remove {
                let mut list = also.clone();
                list.remove(k);
                self.set_also(list);
                self.setup.picking_also = None;
            }
            if also.len() < MAX_ALSO {
                ui.add_space(2.0);
                let adding = self.setup.picking_also == Some(also.len());
                let label = if adding { "Done" } else { "+ Add another Pokémon" };
                if ui.add(egui::Button::new(RichText::new(label).size(kit::fs(12.5))).fill(kit::btn_fill())).clicked() {
                    self.setup.picking_also = if adding { None } else { Some(also.len()) };
                    self.setup.picking_target = false;
                }
            } else {
                ui.label(RichText::new(format!("Up to {} Pokémon at once.", MAX_ALSO + 1)).size(kit::fs(11.5)).color(MUTED()));
            }
        }
        if let Some(k) = self.setup.picking_also {
            ui.add_space(8.0);
            if let Some(slug) = self.target_picker(ui, 260.0) {
                let mut list = self.also_slugs();
                if self.cfg.target.as_deref() != Some(slug.as_str()) && !list.contains(&slug) {
                    if k < list.len() {
                        list[k] = slug;
                    } else {
                        list.push(slug);
                    }
                    self.set_also(list);
                }
                self.setup.picking_also = None;
            }
        }
    }

    /// Advanced: save everything to a file / load it back.
    fn backup_card(&mut self, ui: &mut egui::Ui) {
        // A file box answered?
        if let Some((save, rx)) = &self.setup.backup_picker {
            if let Ok(res) = rx.try_recv() {
                let save = *save;
                self.setup.backup_picker = None;
                if let Some(path) = res {
                    if save {
                        self.setup.backup_msg = Some(match serde_json::to_string_pretty(&self.cfg) {
                            Ok(json) => match std::fs::write(&path, json) {
                                Ok(()) => (true, format!("✔ Backup saved: {}", path.display())),
                                Err(e) => (false, format!("Couldn't save the backup: {e}")),
                            },
                            Err(e) => (false, format!("Couldn't save the backup: {e}")),
                        });
                    } else {
                        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| {
                            serde_json::from_str::<crate::config::Config>(&t).map_err(|_| "That file isn't a Companion backup".to_string())
                        }) {
                            Ok(c) => self.setup.restore_pending = Some((path, Box::new(c))),
                            Err(e) => self.setup.backup_msg = Some((false, e)),
                        }
                    }
                }
            }
        }
        kit::section(ui, "Backup");
        kit::card(ui, |ui| {
            ui.label(
                RichText::new("Everything in one file: hunts, runs, stats, achievements and settings. Handy when moving PCs or before trying a test build.")
                    .size(kit::fs(12.0))
                    .color(MUTED()),
            );
            ui.add_space(6.0);
            let busy = self.setup.backup_picker.is_some();
            let start = dirs::document_dir().unwrap_or_default();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!busy, egui::Button::new(RichText::new("Save a backup…").size(kit::fs(12.5))).fill(kit::btn_fill()))
                    .clicked()
                {
                    let (y, m, d) = crate::stats::civil_from_days(crate::stats::local_day(crate::stats::now_unix()));
                    let name = format!("Shiny Tracker backup {y:04}-{m:02}-{d:02}.json");
                    self.setup.backup_picker = Some((true, crate::share::pick_backup(true, start.clone(), name, ui.ctx().clone())));
                }
                if ui
                    .add_enabled(!busy, egui::Button::new(RichText::new("Restore from backup…").size(kit::fs(12.5))).fill(kit::btn_fill()))
                    .clicked()
                {
                    self.setup.backup_picker = Some((false, crate::share::pick_backup(false, start.clone(), String::new(), ui.ctx().clone())));
                }
            });
            if let Some((path, c)) = &self.setup.restore_pending {
                ui.add_space(8.0);
                let run = self.target_name(c.target.as_deref());
                ui.label(
                    RichText::new(format!(
                        "Replace everything with this backup? It has {} caught shinies and a {} run at {} encounters. Your current data is saved first as before-restore.json.",
                        c.history.len(),
                        run,
                        theme::thousands(c.count)
                    ))
                    .size(kit::fs(12.5))
                    .color(TEXT()),
                )
                .on_hover_text(path.display().to_string());
                let mut go = false;
                let mut cancel = false;
                ui.horizontal(|ui| {
                    go = kit::accent_button(ui, egui::Button::new(RichText::new("Replace").strong().color(theme::button_ink())), 6.0).clicked();
                    cancel = ui.add(egui::Button::new("Cancel").fill(kit::btn_fill())).clicked();
                });
                if go {
                    if let Some((_, c)) = self.setup.restore_pending.take() {
                        let safety = crate::config::data_dir().join("before-restore.json");
                        let _ = std::fs::create_dir_all(crate::config::data_dir());
                        if let Ok(json) = serde_json::to_string_pretty(&self.cfg) {
                            let _ = std::fs::write(&safety, json);
                        }
                        self.cfg = *c;
                        self.reset_transient();
                        self.push_settings(true);
                        self.dirty = true;
                        self.setup.backup_msg = Some((true, "✔ Restored from the backup".into()));
                    }
                } else if cancel {
                    self.setup.restore_pending = None;
                }
            }
            if let Some((ok, msg)) = &self.setup.backup_msg {
                ui.add_space(6.0);
                kit::pill(ui, msg, if *ok { PillKind::Ok } else { PillKind::Bad });
            }
        });
    }

    /// Advanced: where saves and downloads are kept, with a way to move them.
    fn data_folder_card(&mut self, ui: &mut egui::Ui) {
        // Folder box answered? Move everything there.
        if let Some(rx) = &self.setup.data_picker {
            if let Ok(res) = rx.try_recv() {
                self.setup.data_picker = None;
                if let Some(target) = res {
                    self.move_data(Some(target));
                }
            }
        }
        let here = crate::config::data_dir();
        let moved = crate::config::data_dir_moved();
        kit::section(ui, "Data folder");
        kit::card(ui, |ui| {
            ui.label(
                RichText::new(
                    "Your hunts, runs, stats and settings are saved here, plus downloaded sprites. Move it anywhere you like (another drive, a synced folder) and everything comes along.",
                )
                .size(kit::fs(12.0))
                .color(MUTED()),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("📁").size(14.0));
                ui.add(egui::Label::new(RichText::new(here.display().to_string()).size(kit::fs(12.5)).color(TEXT())).truncate())
                    .on_hover_text(here.display().to_string());
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("Open folder").size(kit::fs(12.5))).fill(kit::btn_fill())).clicked() {
                    crate::share::open_dir(&here);
                }
                let busy = self.setup.data_picker.is_some();
                if ui
                    .add_enabled(!busy, egui::Button::new(RichText::new("Move to…").size(kit::fs(12.5))).fill(kit::btn_fill()))
                    .on_hover_text("Pick a folder; a ShinyTracker folder is made inside it")
                    .clicked()
                {
                    let start = dirs::document_dir().unwrap_or_default();
                    self.setup.data_picker = Some(crate::share::pick_folder(start, ui.ctx().clone()));
                }
                if moved
                    && ui
                        .add_enabled(!busy, egui::Button::new(RichText::new("Back to default").size(kit::fs(12.5))).fill(kit::btn_fill()))
                        .on_hover_text("Move everything back to AppData")
                        .clicked()
                {
                    self.move_data(None);
                }
            });
            if let Some((ok, msg)) = &self.setup.data_msg {
                ui.add_space(6.0);
                kit::pill(ui, msg, if *ok { PillKind::Ok } else { PillKind::Bad });
            }
        });
    }

    /// Save, move the data folder (None = back to default), save again there.
    fn move_data(&mut self, target: Option<std::path::PathBuf>) {
        self.cfg.save();
        self.setup.data_msg = Some(match crate::config::move_data_dir(target) {
            Ok(dir) => {
                self.cfg.save();
                self.dirty = false;
                (true, format!("✔ Moved to {}", dir.display()))
            }
            Err(e) => (false, e),
        });
    }

    /// Advanced: version and automatic updates.
    fn updates_card(&mut self, ui: &mut egui::Ui) {
        use crate::update::{self, State};
        kit::section(ui, "Updates");
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Poke Together Companion v{}", update::VERSION)).size(kit::fs(13.5)).strong().color(TEXT()));
                let (text, kind) = match update::get(&self.updater) {
                    State::Idle => ("Not checked yet".to_string(), PillKind::Info),
                    State::Checking => ("Checking…".to_string(), PillKind::Info),
                    State::UpToDate => ("✔ Up to date".to_string(), PillKind::Ok),
                    State::Downloading { version, got, total } => (
                        if total > 0 {
                            format!("Downloading v{version}… {}%", got * 100 / total.max(1))
                        } else {
                            format!("Downloading v{version}…")
                        },
                        PillKind::Info,
                    ),
                    State::Ready { version, .. } => (format!("v{version} ready to install"), PillKind::Ok),
                    State::Failed(e) => (e, PillKind::Warn),
                };
                kit::pill(ui, &text, kind);
            });
            ui.add_space(6.0);
            self.dirty |= kit::toggle_row(
                ui,
                "Update automatically",
                "Checks GitHub for a new version now and then and downloads it in the background. You choose when to restart.",
                &mut self.cfg.auto_update,
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let state = update::get(&self.updater);
                if ui
                    .add_enabled(
                        !matches!(state, State::Checking | State::Downloading { .. }),
                        egui::Button::new(RichText::new("Check now").size(kit::fs(12.5))).fill(kit::btn_fill()),
                    )
                    .clicked()
                {
                    if matches!(state, State::UpToDate | State::Failed(_)) {
                        if let Ok(mut g) = self.updater.lock() {
                            *g = State::Idle;
                        }
                    }
                    self.last_update_check = Some(std::time::Instant::now());
                    self.update_dismissed = false;
                    update::check(self.updater.clone(), ui.ctx().clone(), self.cfg.installed_update.clone());
                }
                if matches!(state, State::Ready { .. })
                    && kit::accent_button(ui, egui::Button::new(RichText::new("Restart to update").strong().color(theme::button_ink())), 6.0).clicked()
                {
                    let ctx = ui.ctx().clone();
                    self.restart_to_update(&ctx);
                }
                if ui
                    .add(egui::Button::new(RichText::new("Releases page").size(kit::fs(12.5)).color(theme::LINK())).frame(false))
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(format!("https://github.com/{}/releases", update::REPO)));
                }
            });
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        // (group heading, tab, icon, label, parent: the app a page belongs to)
        let items: [(Option<&str>, Tab, &str, &str, Option<Tab>); 13] = [
            (None, Tab::Home, "🏠", "Home", None),
            (None, Tab::Profile, "🏅", "Profile", None),
            (Some("APPS"), Tab::Shiny, "✨", "Shiny Hunter", None),
            (None, Tab::Hunt, "🎯", "Hunt", Some(Tab::Shiny)),
            (None, Tab::Counting, "👁", "Counting", Some(Tab::Shiny)),
            (None, Tab::Runs, "📜", "Runs", Some(Tab::Shiny)),
            (None, Tab::Dex, "⭐", "Shiny Dex", Some(Tab::Shiny)),
            (None, Tab::Overlay, "🖵", "Overlay", Some(Tab::Shiny)),
            (None, Tab::Pokedex, "📖", "Pokédex", None),
            (None, Tab::Map, "🗺", "Map", None),
            (Some("SETTINGS"), Tab::Appearance, "🎨", "Appearance", None),
            (None, Tab::Hotkeys, "⌨", "Hotkeys", None),
            (None, Tab::Advanced, "⚙", "Advanced", None),
        ];
        let parent_of = |t: Tab| items.iter().find(|i| i.1 == t).and_then(|i| i.4);
        let has_children = |t: Tab| items.iter().any(|i| i.4 == Some(t));
        let current = self.setup.tab;
        for (group, tab, icon, label, parent) in items {
            if let Some(g) = group {
                kit::nav_group(ui, g);
            }
            // Sub-pages only show while their app is expanded.
            if let Some(p) = parent {
                if self.setup.nav_collapsed.contains(&p) {
                    continue;
                }
            }
            let expandable = has_children(tab);
            let collapsed = self.setup.nav_collapsed.contains(&tab);
            // A collapsed app stays highlighted while one of its pages is open.
            let selected = current == tab || (collapsed && parent_of(current) == Some(tab));
            let clicked = if parent.is_some() {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.vertical(|ui| kit::nav_item(ui, icon, label, selected)).inner
                })
                .inner
            } else {
                let r = kit::nav_item(ui, icon, label, selected);
                let rect = ui.min_rect();
                let y = ui.cursor().min.y - 18.0;
                let right = rect.max.x - 12.0;
                // A chevron shows an app has pages under it, open or folded.
                if expandable {
                    let c = egui::pos2(right - 3.0, y - 1.0);
                    let st = egui::Stroke::new(1.5f32, MUTED());
                    let pts = if collapsed {
                        [c + egui::vec2(-1.5, -3.5), c + egui::vec2(2.0, 0.0), c + egui::vec2(-1.5, 3.5)]
                    } else {
                        [c + egui::vec2(-3.5, -1.5), c + egui::vec2(0.0, 2.0), c + egui::vec2(3.5, -1.5)]
                    };
                    ui.painter().add(egui::Shape::line(pts.to_vec(), st));
                }
                // A green light on Shiny Hunter's icon while the overlay runs
                // (a badge on the icon, clear of the name and the chevron).
                if tab == Tab::Shiny && self.overlay_on {
                    let c = egui::pos2(rect.min.x + 27.0, y - 8.0);
                    ui.painter().circle_filled(c, 5.0, if selected { kit::SELECTED_BG() } else { SIDEBAR() });
                    ui.painter().circle_filled(c, 3.5, theme::GREEN);
                }
                r
            };
            if clicked {
                if expandable {
                    // Opening an app shows its pages; only a second click on
                    // the app you're already on folds them away again.
                    if collapsed {
                        self.setup.nav_collapsed.retain(|t| *t != tab);
                    } else if current == tab {
                        self.setup.nav_collapsed.push(tab);
                    }
                }
                // Clicking the page you're on takes you back to its top
                // (coming back to the Pokédex keeps your place in the grid).
                self.setup.scroll_top = current == tab || tab != Tab::Pokedex;
                self.setup.tab = tab;
                self.setup.picking_target = false;
                self.setup.view_run = None;
                self.setup.share = None;
                self.setup.profile_card = None;
                self.setup.phase_pick = false;
                self.setup.caught_pick = false;
            }
            ui.add_space(2.0);
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            let recent = self
                .saved_flash
                .is_some_and(|t| t.elapsed() < Duration::from_millis(1500));
            if recent {
                ui.label(RichText::new("✔ Saved").size(kit::fs(11.5)).color(theme::GREEN));
                ui.ctx().request_repaint_after(Duration::from_millis(200));
            } else {
                ui.label(
                    RichText::new("Changes save automatically")
                        .size(kit::fs(10.5))
                        .color(MUTED()),
                );
            }
        });
    }

    // ─────────────────────────── shared pieces ───────────────────────────

    /// Searchable grid of Pokémon with sprites. Returns the picked slug.
    pub(super) fn target_picker(&mut self, ui: &mut egui::Ui, height: f32) -> Option<String> {
        let mut picked = None;
        let species = self.species.clone();
        if kit::clean() {
            kit::search_box(ui, &mut self.setup.search, "Search by name or number", f32::INFINITY);
        } else {
            ui.add(
                egui::TextEdit::singleline(&mut self.setup.search)
                    .hint_text("Search by name or number")
                    .desired_width(f32::INFINITY)
                    .margin(egui::Margin::symmetric(10.0, 7.0)),
            );
        }
        // Recently hunted targets first.
        let mut recent: Vec<String> = Vec::new();
        for slug in self
            .cfg
            .saved_runs
            .iter()
            .rev()
            .filter_map(|r| r.target.clone())
        {
            if !recent.contains(&slug) {
                recent.push(slug);
            }
        }
        if !recent.is_empty() && self.setup.search.is_empty() {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Recent:").size(kit::fs(11.5)).color(MUTED()));
                for slug in recent.iter().take(6) {
                    if let Some(sp) = species.iter().find(|s| &s.slug == slug) {
                        if ui
                            .add(egui::Button::new(sp.name).fill(kit::btn_fill()))
                            .clicked()
                        {
                            picked = Some(slug.clone());
                        }
                    }
                }
            });
        }
        ui.add_space(8.0);
        let q = self.setup.search.trim().to_lowercase();
        let list: Vec<usize> = (0..species.len())
            .filter(|&i| {
                q.is_empty()
                    || species[i].name.to_lowercase().contains(&q)
                    || (i + 1).to_string() == q.trim_start_matches('#').trim_start_matches('0')
            })
            .collect();
        // same tiles as the Shiny Dex, stretched to fill the width (minus the scrollbar)
        let gap = super::extras::TILE_GAP;
        let (per_row, tw) = super::extras::grid_cols(ui.available_width() - 14.0);
        let tile = egui::vec2(tw, super::extras::TILE_H);
        let rows = list.len().div_ceil(per_row);
        egui::ScrollArea::vertical()
            .id_salt("target-picker")
            .max_height(height)
            .auto_shrink([false, true])
            .show_rows(ui, tile.y + gap, rows, |ui, range| {
                for row in range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for &i in list.iter().skip(row * per_row).take(per_row) {
                            let sp = species[i].clone();
                            let selected = self.cfg.target.as_deref() == Some(sp.slug.as_str());
                            let (rect, resp) = ui.allocate_exact_size(tile, Sense::click());
                            let p = ui.painter_at(rect.expand(2.0));
                            let look = super::extras::TileLook {
                                fill: if selected {
                                    kit::SELECTED_BG()
                                } else if resp.hovered() {
                                    CARD_HOVER()
                                } else {
                                    CARD()
                                },
                                stroke: egui::Stroke::new(
                                    if selected { 2.0f32 } else { 1.0f32 },
                                    if selected { kit::ACCENT() } else { LINE() },
                                ),
                                name: TEXT(),
                                num: MUTED(),
                                tint: Color32::WHITE,
                                shiny: false,
                            };
                            self.grid_tile(&p, rect, &sp, &look);
                            if resp
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                picked = Some(sp.slug.clone());
                            }
                        }
                    });
                    ui.add_space(gap);
                }
            });
        picked
    }

    /// Method tiles + method-specific controls.
    fn method_picker(&mut self, ui: &mut egui::Ui) {
        // Every method on offer, always shown (plus a retired one if it's
        // still selected).
        let methods: Vec<HuntMethod> = HuntMethod::ALL
            .into_iter()
            .filter(|&m| method_offered(m) || m == self.cfg.hunt_method)
            .collect();
        let w = ((ui.available_width() - 16.0) / 3.0).floor();
        for chunk in methods.chunks(3) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for &m in chunk {
                    let (title, sub) = method_short(m);
                    let badge = method_badge(&self.cfg, m);
                    let sel = self.cfg.hunt_method == m;
                    let tile = kit::choice_tile(
                        ui,
                        egui::vec2(w, 104.0),
                        sel,
                        method_icon(m),
                        title,
                        sub,
                        &badge,
                    );
                    if method_experimental(m) {
                        warn_chip(ui, tile.rect, sel);
                    }
                    let tip = if method_experimental(m) {
                        format!("{}\n\n⚠ {EXPERIMENTAL_NOTE}", m.note())
                    } else {
                        m.note().to_string()
                    };
                    if tile.on_hover_text(tip).clicked() {
                        self.cfg.hunt_method = m;
                        self.dirty = true;
                    }
                }
            });
            ui.add_space(8.0);
        }
        ui.add_space(4.0);
        self.dirty |= kit::toggle_row(
            ui,
            "Switch automatically",
            "Changes to Rare grass when the game log shows Sweet Scent on rare grass, and to Soft resets when you pick a starter or reset into a static Pokémon. A short pop-up says when it switches. Your count isn't touched.",
            &mut self.cfg.auto_method,
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(self.cfg.hunt_method.note())
                .size(kit::fs(11.5))
                .color(MUTED()),
        );
        if method_experimental(self.cfg.hunt_method) {
            ui.add_space(4.0);
            kit::pill(ui, &format!("⚠ {EXPERIMENTAL_NOTE}"), PillKind::Warn);
        }
        match self.cfg.hunt_method {
            HuntMethod::PokeRadar => {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Current chain").size(kit::fs(13.0)).color(TEXT()));
                    self.dirty |= kit::stepper(ui, &mut self.cfg.radar_chain, 0, 999);
                    kit::info(ui, "Odds stop improving at a chain of 40.");
                });
            }
            HuntMethod::Custom => {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Odds: 1 in").size(kit::fs(13.0)).color(TEXT()));
                    self.dirty |= ui
                        .add(egui::DragValue::new(&mut self.cfg.odds).range(1..=65536))
                        .changed();
                });
            }
            HuntMethod::FromGameLog => {
                ui.add_space(6.0);
                match self.cfg.last_log_odds {
                    Some((n, d)) => {
                        kit::pill(
                            ui,
                            &format!(
                                "✔ Game says {n} in {d} (1 in {})",
                                fmt_one_in(d as f64 / n.max(1) as f64)
                            ),
                            PillKind::Ok,
                        );
                    }
                    None => {
                        kit::pill(ui, "Waiting for your first battle", PillKind::Warn);
                    }
                }
            }
            _ => {}
        }
    }

    fn battle_type_picker(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            changed = kit::segmented(
                ui,
                &mut self.cfg.battle_type,
                &[(BattleType::Doubles, "  Doubles  "), (BattleType::Singles, "  Singles  "), (BattleType::Auto, "  Not sure  ")],
            );
            kit::info(ui, "Doubles: 2 Pokémon per battle, each gets its own shiny chance.\nNot sure: counts however many enemy names are on screen.");
        });
        changed
    }

    fn count_source_picker(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let w = ((ui.available_width() - 8.0) / 2.0).floor();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let log = self.cfg.count_source == CountSource::GameLog;
            if kit::choice_tile(ui, egui::vec2(w, 104.0), log, "📄", "Game log", "Exact, no setup needed", "Recommended")
                .on_hover_text("The game writes every wild Pokémon to its log, so counts are exact even when the overlay is hidden.")
                .clicked()
                && !log
            {
                self.cfg.count_source = CountSource::GameLog;
                changed = true;
            }
            if kit::choice_tile(ui, egui::vec2(w, 104.0), !log, "👁", "Screen reading", "Reads the enemy names", "Needs setup")
                .on_hover_text("Reads the enemy names from the screen. Needs the name boxes set up once.")
                .clicked()
                && log
            {
                self.cfg.count_source = CountSource::Screen;
                changed = true;
            }
        });
        if changed {
            self.push_settings(true);
        }
        changed
    }

    fn odds_tiles(&self, ui: &mut egui::Ui) {
        let w = ((ui.available_width() - 16.0) / 3.0).floor();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            match self.cfg.odds_one_in() {
                Some(one_in) => {
                    kit::stat(
                        ui,
                        w,
                        "Per Pokémon",
                        &format!("1 in {}", fmt_one_in(one_in)),
                    );
                    let (label, value) = match (self.cfg.per_battle_one_in(), self.cfg.pokemon_per_battle()) {
                        _ if !self.cfg.hunt_method.is_wild_battles() => (
                            if self.cfg.hunt_method == HuntMethod::Masuda { "Per egg" } else { "Per reset" }.to_string(),
                            format!("1 in {}", fmt_one_in(one_in)),
                        ),
                        (Some(b), Some(n)) => (
                            if n == 2.0 {
                                "Per double battle".to_string()
                            } else if n == 1.0 {
                                "Per single battle".to_string()
                            } else {
                                format!("Per battle (~{n:.1} Pokémon)")
                            },
                            format!("1 in {}", fmt_one_in(b)),
                        ),
                        _ => ("Per battle".to_string(), "After 1st battle".to_string()),
                    };
                    kit::stat(ui, w, &label, &value);
                    let state = odds_state(&self.cfg, self.cfg.count)
                        .map(|s| s.0)
                        .unwrap_or_default();
                    kit::stat(ui, w, "This hunt", &state);
                }
                None => kit::stat(ui, w * 3.0 + 16.0, "Odds", "Shown after your first battle"),
            }
        });
    }

    // ─────────────────────────── pages ───────────────────────────

    fn page_hunt(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "HUNT", "What you're hunting and how. The odds update as you change things.");
        let mut tab = self.setup.hunt_tab;
        if kit::tab_bar(
            ui,
            &mut tab,
            &[
                (HuntTab::Target, "Target"),
                (HuntTab::Method, "Method & odds"),
                (HuntTab::Session, "Session"),
            ],
        ) {
            self.setup.hunt_tab = tab;
        }
        ui.add_space(6.0);
        match self.setup.hunt_tab {
            HuntTab::Target => {
                // Target card
                kit::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), Sense::hover());
                        let p = ui.painter_at(r);
                        p.rect_filled(r, 32.0, CARD_HOVER());
                        match self.target_idx() {
                            Some(i) => {
                                let sp = self.species[i].clone();
                                self.draw_sprite(&p, &sp, false, r.center_bottom() - egui::vec2(0.0, 4.0), egui::vec2(60.0, 58.0));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Target").size(kit::fs(11.0)).color(MUTED()));
                                    ui.label(
                                        RichText::new(format!("{}  #{:04}", sp.name, i + 1))
                                            .size(kit::fs(16.5))
                                            .strong()
                                            .color(TEXT()),
                                    );
                                });
                            }
                            None => {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Target").size(kit::fs(11.0)).color(MUTED()));
                                    ui.label(RichText::new("Pick the Pokémon you're hunting").size(kit::fs(14.5)).color(TEXT()));
                                });
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let label = if self.setup.picking_target { "Cancel" } else { "Change" };
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(label).size(kit::fs(13.0)))
                                        .fill(kit::btn_fill())
                                        .min_size(egui::vec2(80.0, 30.0)),
                                )
                                .clicked()
                            {
                                self.setup.picking_target = !self.setup.picking_target;
                            }
                        });
                    });
                    if self.setup.picking_target || self.cfg.target.is_none() {
                        ui.add_space(10.0);
                        if let Some(slug) = self.target_picker(ui, 330.0) {
                            self.set_target(slug);
                            self.setup.picking_target = false;
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Switching target saves your current run. Find it again under Runs.")
                                .size(kit::fs(11.0))
                                .color(MUTED()),
                        );
                    }
                    // Optional second target (two Pokémon on the same route).
                    if self.cfg.target.is_some() {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);
                        self.second_target_row(ui);
                    }
                });
            }
            HuntTab::Method => {
                kit::section(ui, "How are you hunting?");
                self.method_picker(ui);
                ui.add_space(14.0);

                kit::section(ui, "Singles or doubles?");
                if self.battle_type_picker(ui) {
                    self.push_settings(true);
                }
                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Shiny Charms in your Bag").size(kit::fs(13.0)).color(TEXT()));
                    kit::info(ui, "Each Shiny Charm gives wild Pokémon one more chance to be shiny.");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.dirty |= kit::stepper(ui, &mut self.cfg.shiny_charms, 0, 20);
                    });
                });
                ui.add_space(14.0);
                self.odds_tiles(ui);
                ui.add_space(10.0);
                if self.cfg.hunt_method.is_wild_battles() {
                    ui.add_space(10.0);
                    kit::card(ui, |ui| {
                        let name = self.target_name(self.cfg.target.as_deref());
                        self.dirty |= kit::toggle_row(
                    ui,
                    &format!("Odds bar counts only {name}"),
                    &format!(
                        "Only {name} encounters can give you a shiny {name}. On, the odds bar and share images use the {name} you've seen ({} so far) instead of every encounter.",
                        theme::thousands(self.cfg.target_seen)
                    ),
                    &mut self.cfg.odds_target_only,
                );
                    });
                }
            }
            HuntTab::Session => {
                kit::section(ui, "Breaks and goals");
                kit::card(ui, |ui| {
                    self.dirty |= kit::toggle_row(
                        ui,
                        "Break reminder",
                        "A gentle nudge on the overlay after a long stretch of hunting in one session.",
                        &mut self.cfg.break_reminder,
                    );
                    if self.cfg.break_reminder {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Remind me after").size(kit::fs(12.5)).color(TEXT()));
                            self.dirty |= ui
                                .add(egui::DragValue::new(&mut self.cfg.break_minutes).range(15..=600).speed(5.0))
                                .changed();
                            ui.label(RichText::new("minutes of hunting").size(kit::fs(12.5)).color(TEXT()));
                        });
                    }
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Session goal").size(kit::fs(13.0)).color(TEXT()));
                        kit::info(ui, "A pat on the back when a session reaches this many encounters. 0 = off.");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("encounters").size(kit::fs(12.5)).color(MUTED()));
                            self.dirty |= ui
                                .add(egui::DragValue::new(&mut self.cfg.session_goal).range(0..=100_000).speed(10.0))
                                .changed();
                        });
                    });
                });

                kit::section(ui, "Hunt timer");
                kit::card(ui, |ui| {
                    self.dirty |= kit::toggle_row(
                ui,
                "Pause the timer when you stop",
                "Hunt time stops when no encounter comes for a while (a break, a menu, the phone) and carries on with the next one. The wait isn't counted. Your hunt and counting keep going.",
                &mut self.cfg.idle_pause,
            );
                    if self.cfg.idle_pause {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Pause after").size(kit::fs(13.0)).color(TEXT()));
                            let mut m = self.cfg.idle_minutes;
                            kit::segmented(
                                ui,
                                &mut m,
                                &[(2, " 2 min "), (3, " 3 min "), (5, " 5 min "), (10, " 10 min "), (15, " 15 min ")],
                            );
                            if m != self.cfg.idle_minutes {
                                self.cfg.idle_minutes = m;
                                self.dirty = true;
                            }
                            ui.label(RichText::new("with no encounter").size(kit::fs(13.0)).color(MUTED()));
                        });
                    }
                });
            }
        }
    }

    /// Counting: which folder the game log is read from, with a way to
    /// pick it by hand if it isn't found.
    fn log_folder_card(&mut self, ui: &mut egui::Ui) {
        // Folder box answered?
        if let Some(rx) = &self.setup.log_picker {
            if let Ok(res) = rx.try_recv() {
                self.setup.log_picker = None;
                if let Some(dir) = res {
                    self.cfg.log_dir = Some(dir.to_string_lossy().to_string());
                    self.dirty = true;
                    self.push_settings(true);
                }
            }
        }
        let custom = self.cfg.log_dir.clone();
        let using = crate::gamelog::log_dir(custom.as_deref().map(std::path::Path::new));
        let found = using.is_some();
        // Open by default only when there's a problem to fix.
        egui::CollapsingHeader::new(RichText::new("Game log folder").size(kit::fs(13.0)).color(TEXT()))
            .id_salt("log-folder")
            .default_open(!found)
            .show(ui, |ui| {
                kit::card(ui, |ui| {
                    let line = match (&using, &custom) {
                        (Some(d), Some(_)) => format!("Chosen by you: {}", d.display()),
                        (Some(d), None) => format!("Found automatically: {}", d.display()),
                        (None, _) => "Not found. Choose the folder that holds the game's session-….log files.".to_string(),
                    };
                    ui.label(RichText::new(line).size(kit::fs(12.5)).color(if found { TEXT() } else { theme::HP_YELLOW }));
                    ui.label(
                        RichText::new(
                            "Normally AppData\\Local\\PlatinumTogether\\Game\\logs. If your game is installed somewhere else, pick its logs folder (or the game's own folder) here.",
                        )
                        .size(kit::fs(11.5))
                        .color(MUTED()),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let busy = self.setup.log_picker.is_some();
                        if ui
                            .add_enabled(!busy, egui::Button::new(RichText::new("Choose folder…").size(kit::fs(12.5))).fill(kit::btn_fill()))
                            .clicked()
                        {
                            let start = using
                                .clone()
                                .or_else(dirs::data_local_dir)
                                .unwrap_or_default();
                            self.setup.log_picker = Some(crate::share::pick_folder(start, ui.ctx().clone()));
                        }
                        if custom.is_some()
                            && ui
                                .add_enabled(!busy, egui::Button::new(RichText::new("Find automatically").size(kit::fs(12.5))).fill(kit::btn_fill()))
                                .on_hover_text("Forget the chosen folder and look in the usual place")
                                .clicked()
                        {
                            self.cfg.log_dir = None;
                            self.dirty = true;
                            self.push_settings(true);
                        }
                        if let Some(d) = &using {
                            if ui.add(egui::Button::new(RichText::new("Open folder").size(kit::fs(12.5))).fill(kit::btn_fill())).clicked() {
                                crate::share::open_dir(d);
                            }
                        }
                    });
                });
            });
    }

    fn page_counting(&mut self, ui: &mut egui::Ui) {
        kit::page_header(
            ui,
            "COUNTING",
            "Where the Companion gets your encounters from.",
        );
        self.count_source_picker(ui);
        ui.add_space(4.0);

        // Status pill
        let s = &self.status;
        let (text, kind) = match self.cfg.count_source {
            CountSource::GameLog => match &s.error {
                Some(e) => (e.clone(), PillKind::Bad),
                None if !s.text.is_empty() => (format!("✔ Reading {}", s.text), PillKind::Ok),
                None => ("Waiting for the game to start".to_string(), PillKind::Warn),
            },
            CountSource::Screen => {
                if self.cfg.name_regions.is_empty() {
                    (
                        "Name boxes not set up yet (see below)".to_string(),
                        PillKind::Warn,
                    )
                } else if let Some(e) = &s.error {
                    (e.clone(), PillKind::Bad)
                } else {
                    ("✔ Watching the screen".to_string(), PillKind::Ok)
                }
            }
        };
        kit::pill(ui, &text, kind);
        ui.add_space(10.0);
        self.log_folder_card(ui);
        ui.add_space(14.0);

        kit::card(ui, |ui| {
            let mut auto = !self.cfg.paused;
            if kit::toggle_row(
                ui,
                "Auto counting",
                "Turn off to pause counting (same as the pause button on the overlay).",
                &mut auto,
            ) {
                self.set_paused(!auto);
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            if kit::toggle_row(
                ui,
                "Pause when the game is closed",
                "Stops counting (and the hunt timer) while Platinum Together isn't running, and starts again when you open it.",
                &mut self.cfg.auto_pause_game,
            ) {
                self.push_settings(true);
            }
            if self.game_closed() {
                ui.add_space(4.0);
                kit::pill(ui, "Paused: the game isn't running", PillKind::Info);
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            if kit::toggle_row(
                ui,
                "Shiny star alert",
                "Flashes the overlay when the shiny ★ appears next to an enemy name. Uses the name boxes (make each box cover the name and the space after it, up to the ♂/♀ sign).",
                &mut self.cfg.shiny_star_alert,
            ) {
                self.push_settings(false);
            }
            ui.add_space(4.0);
            kit::pill(
                ui,
                "⚠ Experimental: may not work correctly yet",
                PillKind::Warn,
            );
        });

        let needs_boxes = self.cfg.count_source == CountSource::Screen || self.cfg.shiny_star_alert;
        let title = if needs_boxes {
            "Name reading setup"
        } else {
            "Name reading setup (only for screen reading or the star alert)"
        };
        egui::CollapsingHeader::new(RichText::new(title).size(kit::fs(13.0)).color(TEXT()))
            .id_salt("name-wizard")
            .default_open(needs_boxes && self.cfg.name_regions.is_empty())
            .show(ui, |ui| {
                ui.add_space(6.0);
                self.screen_wizard(ui);
            });
    }

    fn page_runs(&mut self, ui: &mut egui::Ui) {
        kit::page_header(
            ui,
            "RUNS",
            "Every hunt is saved. Switch target any time and come back later.",
        );

        // Current run
        kit::section(ui, "Current run");
        let mut view = None;
        let mut share_run: Option<RunRef> = None;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let left = ui.horizontal(|ui| {
                self.run_thumb(ui, self.cfg.target.clone());
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(self.target_name(self.cfg.target.as_deref()))
                            .size(kit::fs(14.5))
                            .strong()
                            .color(TEXT()),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} encounters · {} battles · started {}",
                            theme::thousands(self.cfg.count),
                            theme::thousands(self.cfg.battles),
                            super::days_ago(self.cfg.run_started_unix)
                        ))
                        .size(kit::fs(11.5))
                        .color(MUTED()),
                    );
                    if let Some((label, color)) = odds_state(&self.cfg, self.cfg.count) {
                        ui.label(RichText::new(label).size(kit::fs(11.5)).color(color));
                    }
                });
                }).response;
                if clickable(ui, left.rect, "run-current") {
                    view = Some(RunRef::Current);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("Stats").strong()).fill(kit::btn_fill()).min_size(egui::vec2(64.0, 30.0)))
                        .on_hover_text("Time hunted, encounters per hour, sessions and more")
                        .clicked()
                    {
                        view = Some(RunRef::Current);
                    }
                    if ui
                        .add(egui::Button::new("🖼 Share").fill(kit::btn_fill()).min_size(egui::vec2(72.0, 30.0)))
                        .on_hover_text("Make a share image of this run for Discord")
                        .clicked()
                    {
                        share_run = Some(RunRef::Current);
                    }
                    let empty = self.run_is_empty();
                    if ui
                        .add_enabled(!empty, egui::Button::new("Save and start new").fill(kit::btn_fill()).min_size(egui::vec2(0.0, 30.0)))
                        .on_hover_text("Saves this run below and starts again from 0 with the same target")
                        .on_disabled_hover_text("Nothing to save yet: this run has no encounters")
                        .clicked()
                    {
                        self.park_current();
                        self.start_fresh(self.cfg.target.clone());
                    }
                });
            });
        });

        // Saved runs
        kit::section(ui, "Saved runs");
        let mut runs: Vec<crate::config::SavedRun> = self
            .cfg
            .saved_runs
            .iter()
            .filter(|r| r.id != self.cfg.run_id)
            .cloned()
            .collect();
        runs.sort_by_key(|r| std::cmp::Reverse(r.saved_unix));
        if runs.is_empty() {
            ui.label(
                RichText::new("No saved runs yet. They appear here when you switch target.")
                    .size(kit::fs(12.0))
                    .color(MUTED()),
            );
        }
        let mut resume = None;
        let mut delete = None;
        for r in &runs {
            kit::card(ui, |ui| {
                ui.horizontal(|ui| {
                    let left = ui.horizontal(|ui| {
                    self.run_thumb(ui, r.target.clone());
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(self.target_name(r.target.as_deref()))
                                .size(kit::fs(14.0))
                                .strong()
                                .color(TEXT()),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} encounters · last hunted {}",
                                theme::thousands(r.count),
                                super::days_ago(r.saved_unix)
                            ))
                            .size(kit::fs(11.5))
                            .color(MUTED()),
                        );
                        if let Some((label, color)) = odds_state(&self.cfg, r.count) {
                            ui.label(RichText::new(label).size(kit::fs(11.5)).color(color));
                        }
                    });
                    }).response;
                    if clickable(ui, left.rect, ("run-saved", r.id)) {
                        view = Some(RunRef::Saved(r.id));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.setup.confirm_delete_run == Some(r.id) {
                            if ui.button("Keep").clicked() {
                                self.setup.confirm_delete_run = None;
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Delete").color(Color32::WHITE),
                                    )
                                    .fill(theme::RED_DARK()),
                                )
                                .clicked()
                            {
                                delete = Some(r.id);
                            }
                        } else {
                            if ui
                                .add(egui::Button::new("🗑").fill(kit::btn_fill()).min_size(egui::vec2(32.0, 30.0)))
                                .on_hover_text("Delete this run (asks first)")
                                .clicked()
                            {
                                self.setup.confirm_delete_run = Some(r.id);
                            }
                            if kit::accent_button(
                                ui,
                                egui::Button::new(
                                    RichText::new("▶ Resume").strong().color(theme::button_ink()),
                                )
                                .min_size(egui::vec2(90.0, 30.0)),
                                4.0,
                            )
                            .clicked()
                            {
                                resume = Some(r.id);
                            }
                            if ui
                                .add(egui::Button::new("Stats").fill(kit::btn_fill()).min_size(egui::vec2(60.0, 30.0)))
                                .clicked()
                            {
                                view = Some(RunRef::Saved(r.id));
                            }
                            if ui
                                .add(egui::Button::new("🖼 Share").fill(kit::btn_fill()).min_size(egui::vec2(72.0, 30.0)))
                                .on_hover_text("Make a share image of this run for Discord")
                                .clicked()
                            {
                                share_run = Some(RunRef::Saved(r.id));
                            }
                        }
                    });
                });
            });
        }
        if let Some(id) = resume {
            self.resume_run(id);
        }
        if let Some(src) = share_run.take() {
            self.open_share(src, None);
            return;
        }
        if let Some(v) = view {
            self.setup.view_run = Some(v);
        }
        if let Some(id) = delete {
            self.cfg.saved_runs.retain(|r| r.id != id);
            self.setup.confirm_delete_run = None;
            self.dirty = true;
        }

        // Finished hunts
        ui.add_space(6.0);
        egui::CollapsingHeader::new(RichText::new(format!("Finished hunts ({})", self.cfg.history.len())).size(kit::fs(13.0)).color(TEXT()))
            .id_salt("finished-hunts")
            .show(ui, |ui| {
                if self.cfg.history.is_empty() {
                    ui.label(RichText::new("Press Caught! on the overlay when you get your shiny and it's logged here.").size(kit::fs(12.0)).color(MUTED()));
                    return;
                }
                let mut remove = None;
                let mut open_share = None;
                let mut open_stats = None;
                for (i, h) in self.cfg.history.clone().iter().enumerate().rev() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("★").color(GOLD));
                        ui.label(RichText::new(&h.target).size(kit::fs(13.0)).color(TEXT()));
                        ui.label(
                            RichText::new(format!("{} encounters · {}", theme::thousands(h.encounters), super::days_ago(h.when_unix)))
                                .size(kit::fs(11.5))
                                .color(MUTED()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new("🗑").frame(false)).on_hover_text("Remove from the list").clicked() {
                                remove = Some(i);
                            }
                            if ui.add(egui::Button::new("Share").fill(kit::btn_fill())).clicked() {
                                open_share = Some(i);
                            }
                            if ui.add(egui::Button::new("Stats").fill(kit::btn_fill())).clicked() {
                                open_stats = Some(i);
                            }
                        });
                    });
                }
                if let Some(i) = remove {
                    self.cfg.history.remove(i);
                    self.dirty = true;
                }
                if let Some(i) = open_share {
                    self.open_share(RunRef::Finished(i), None);
                }
                if let Some(i) = open_stats {
                    self.setup.view_run = Some(RunRef::Finished(i));
                }
            });
    }

    /// Small round sprite thumbnail for run rows.
    pub(super) fn run_thumb(&mut self, ui: &mut egui::Ui, slug: Option<String>) {
        let (r, _) = ui.allocate_exact_size(egui::vec2(52.0, 52.0), Sense::hover());
        let p = ui.painter_at(r);
        p.rect_filled(r, 26.0, CARD_HOVER());
        if let Some(sp) = slug.and_then(|s| self.species.iter().find(|x| x.slug == s).cloned()) {
            self.draw_sprite(
                &p,
                &sp,
                false,
                r.center_bottom() - egui::vec2(0.0, 3.0),
                egui::vec2(48.0, 46.0),
            );
        }
    }

    fn page_overlay(&mut self, ui: &mut egui::Ui) {
        use crate::config::{OverlayModules, OverlaySize, OverlayStyle};
        kit::page_header(ui, "OVERLAY", "How the counter window looks and behaves.");

        // Tabs first, right under the title. Sections only exist for the
        // modular overlay.
        let legacy = self.cfg.overlay.legacy();
        if legacy && self.setup.overlay_tab == OverlayTab::Sections {
            self.setup.overlay_tab = OverlayTab::Layout;
        }
        let mut tab = self.setup.overlay_tab;
        let tabs: Vec<(OverlayTab, &str)> = if legacy {
            vec![
                (OverlayTab::Layout, "Layout"),
                (OverlayTab::Behaviour, "Behaviour"),
                (OverlayTab::Sounds, "Sounds"),
            ]
        } else {
            vec![
                (OverlayTab::Layout, "Layout"),
                (OverlayTab::Sections, "Sections"),
                (OverlayTab::Behaviour, "Behaviour"),
                (OverlayTab::Sounds, "Sounds"),
            ]
        };
        if kit::tab_bar(ui, &mut tab, &tabs) {
            self.setup.overlay_tab = tab;
        }
        ui.add_space(10.0);

        match self.setup.overlay_tab {
            OverlayTab::Layout => {
                // One card, one setting per row: name and what it does on the
                // left, the control on the right.
                let row_gap = |ui: &mut egui::Ui| {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);
                };
                let label = |ui: &mut egui::Ui, title: &str, about: &str| {
                    ui.vertical(|ui| {
                        ui.set_max_width((ui.available_width() - 250.0).max(180.0));
                        ui.label(RichText::new(title).size(kit::fs(13.5)).strong().color(TEXT()));
                        ui.label(RichText::new(about).size(kit::fs(12.0)).color(MUTED()));
                    });
                };
                kit::card(ui, |ui| {
                    // Style
                    ui.horizontal(|ui| {
                        label(
                            ui,
                            "Style",
                            if legacy {
                                "Legacy is the original overlay, exactly as it was. Modular lets you pick its sections and size."
                            } else {
                                "Modular lets you pick sections and a size. Legacy is the original overlay, exactly as it was."
                            },
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut style = self.cfg.overlay.style;
                            if kit::segmented(ui, &mut style, &[(OverlayStyle::Legacy, "Legacy"), (OverlayStyle::Modular, "Modular")]) {
                                self.cfg.overlay.style = style;
                                self.dirty = true;
                            }
                        });
                    });
                    if !legacy {
                        // Size
                        row_gap(ui);
                        ui.horizontal(|ui| {
                            let about = match self.cfg.overlay.size {
                                OverlaySize::Full => "The usual size. It grows and shrinks to fit the sections you turn on.",
                                OverlaySize::Compact => "Small and narrow: just the sprites and the counter. Caught! and alerts still pop up when needed.",
                                OverlaySize::Bar => "One slim line: target, count and odds, plus luck, milestone, session, pace, streak and buttons if they're on.",
                            };
                            label(ui, "Size", about);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut size = self.cfg.overlay.size;
                                if kit::segmented(
                                    ui,
                                    &mut size,
                                    &[(OverlaySize::Bar, "Bar"), (OverlaySize::Compact, "Compact"), (OverlaySize::Full, "Full")],
                                ) {
                                    self.cfg.overlay.size = size;
                                    self.dirty = true;
                                }
                            });
                        });
                        // Liquid layout (Full only)
                        if self.cfg.overlay.size == OverlaySize::Full {
                            row_gap(ui);
                            self.dirty |= kit::toggle_row(
                                ui,
                                "Liquid layout (resizable)",
                                "Drag the overlay's right edge to make it wider or narrower; the sections stretch to fit.",
                                &mut self.cfg.liquid,
                            );
                            if self.cfg.liquid {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("Width {:.0}. Drag the overlay's right edge to change it.", self.cfg.liquid_width))
                                            .size(kit::fs(12.0))
                                            .color(MUTED()),
                                    );
                                    if ui.add(egui::Button::new(RichText::new("Reset width").size(kit::fs(12.0))).fill(kit::btn_fill())).clicked() {
                                        self.cfg.liquid_width = super::FULL_SIZE[0];
                                        self.dirty = true;
                                    }
                                });
                            }
                        }
                    }
                    // Opacity
                    row_gap(ui);
                    ui.label(RichText::new("Background opacity").size(kit::fs(13.5)).strong().color(TEXT()));
                    ui.label(RichText::new("Fade the panel so the game shows through.").size(kit::fs(12.0)).color(MUTED()));
                    ui.add_space(4.0);
                    ui.spacing_mut().slider_width = ui.available_width() - 70.0;
                    let lowest = if legacy { 0.4 } else { 0.1 };
                    self.dirty |= ui
                        .add(
                            egui::Slider::new(&mut self.cfg.opacity, lowest..=1.0)
                                .show_value(true)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                                .custom_parser(|t| t.trim().trim_end_matches('%').trim().parse::<f64>().ok().map(|v| v / 100.0)),
                        )
                        .changed();
                });
            }
            OverlayTab::Sections => {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Turn parts of the overlay on or off.")
                            .size(kit::fs(12.0))
                            .color(MUTED()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("Reset to default").size(kit::fs(12.0))).fill(kit::btn_fill()))
                            .clicked()
                        {
                            let keep = self.cfg.overlay.clone();
                            self.cfg.overlay = OverlayModules {
                                style: keep.style,
                                size: keep.size,
                                click_through: keep.click_through,
                                ..OverlayModules::default()
                            };
                            self.cfg.level_on_overlay = true;
                            self.cfg.show_session_timer = true;
                            self.dirty = true;
                        }
                        if ui
                            .add(egui::Button::new(RichText::new("Turn all on").size(kit::fs(12.0))).fill(kit::btn_fill()))
                            .clicked()
                        {
                            let o = &mut self.cfg.overlay;
                            for f in [
                                &mut o.status_dot,
                                &mut o.target,
                                &mut o.sprites,
                                &mut o.counter,
                                &mut o.buttons,
                                &mut o.odds_bar,
                                &mut o.method_line,
                                &mut o.luck,
                                &mut o.milestone,
                                &mut o.session,
                                &mut o.sparkline,
                                &mut o.last_seen,
                                &mut o.phases,
                                &mut o.xp,
                                &mut o.daily_goal,
                                &mut o.streak,
                                &mut o.best,
                            ] {
                                *f = true;
                            }
                            self.cfg.level_on_overlay = true;
                            self.cfg.show_session_timer = true;
                            self.dirty = true;
                        }
                    });
                });

                let sep = |ui: &mut egui::Ui| {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);
                };
                // A setting that only matters when its parent is on.
                let sub = |ui: &mut egui::Ui, enabled: bool, label: &str, help: &str, on: &mut bool| -> bool {
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.add_space(22.0);
                        ui.add_enabled_ui(enabled, |ui| {
                            ui.vertical(|ui| changed = kit::toggle_row(ui, label, help, on));
                        });
                    });
                    changed
                };

                kit::section(ui, "Hunt");
                kit::card(ui, |ui| {
                    let o = &mut self.cfg.overlay;
                    let mut d = kit::toggle_row(
                        ui,
                        "Target name",
                        "The name plate, with how often your target turns up (\"SEEN 12\").",
                        &mut o.target,
                    );
                    sep(ui);
                    d |= kit::toggle_row(ui, "Sprites", "Your target, normal and shiny, animated.", &mut o.sprites);
                    let on = o.sprites;
                    d |= sub(
                        ui,
                        on,
                        "Shiny only",
                        "One big shiny sprite instead of normal + shiny side by side.",
                        &mut o.single_sprite,
                    );
                    sep(ui);
                    d |= kit::toggle_row(
                        ui,
                        "Encounter counter",
                        "The big number, with ENCOUNTERS / PAUSED under it.",
                        &mut o.counter,
                    );
                    let on = o.counter;
                    d |= sub(
                        ui,
                        on,
                        "Today's hunting time",
                        "A small timer next to the counter: how long you've hunted today, across all runs.",
                        &mut self.cfg.show_session_timer,
                    );
                    sep(ui);
                    let o = &mut self.cfg.overlay;
                    d |= kit::toggle_row(
                        ui,
                        "Buttons",
                        "−1, +1, pause and Caught!. Hotkeys still work when they're hidden, and the Caught! question always shows.",
                        &mut o.buttons,
                    );
                    self.dirty |= d;
                });

                kit::section(ui, "Header");
                kit::card(ui, |ui| {
                    let mut d = kit::toggle_row(
                        ui,
                        "Level badge",
                        "Your trainer level next to the title.",
                        &mut self.cfg.level_on_overlay,
                    );
                    sep(ui);
                    d |= kit::toggle_row(
                        ui,
                        "Status light",
                        "Green watching, gold in battle, grey paused, red if something needs fixing.",
                        &mut self.cfg.overlay.status_dot,
                    );
                    self.dirty |= d;
                });

                kit::section(ui, "Odds & luck");
                kit::card(ui, |ui| {
                    let o = &mut self.cfg.overlay;
                    let mut d = kit::toggle_row(ui, "Odds bar", "Under / at / over the odds, filling up as you go.", &mut o.odds_bar);
                    sep(ui);
                    d |= kit::toggle_row(
                        ui,
                        "Method line",
                        "What the odds are based on, e.g. \"Rare grass · Doubles · 1/4,096 each\".",
                        &mut o.method_line,
                    );
                    sep(ui);
                    d |= kit::toggle_row(
                        ui,
                        "Luck",
                        "How many hunters would have found their shiny by now. Under 50% is fine; over 80% is unlucky.",
                        &mut o.luck,
                    );
                    sep(ui);
                    d |= kit::toggle_row(
                        ui,
                        "Next milestone",
                        "The nearer of the next thousand and the next 1×, 2×… the odds, with how many to go.",
                        &mut o.milestone,
                    );
                    self.dirty |= d;
                });

                kit::section(ui, "Session");
                kit::card(ui, |ui| {
                    let o = &mut self.cfg.overlay;
                    let mut d = kit::toggle_row(
                        ui,
                        "Session strip",
                        "This session's encounters, time and pace per hour.",
                        &mut o.session,
                    );
                    sep(ui);
                    d |= kit::toggle_row(
                        ui,
                        "Pace graph",
                        "A tiny bar graph of encounters each minute for the last 20 minutes.",
                        &mut o.sparkline,
                    );
                    sep(ui);
                    d |= kit::toggle_row(
                        ui,
                        "Last encounter",
                        "Which Pokémon were in the last battle, and how long ago.",
                        &mut o.last_seen,
                    );
                    sep(ui);
                    d |= kit::toggle_row(ui, "Phase count", "Shinies of other Pokémon found during this hunt.", &mut o.phases);
                    self.dirty |= d;
                });

                kit::section(ui, "Progress");
                kit::card(ui, |ui| {
                    let o = &mut self.cfg.overlay;
                    let mut d = kit::toggle_row(ui, "Trainer XP bar", "Your level and how close the next one is.", &mut o.xp);
                    sep(ui);
                    d |= kit::toggle_row(
                        ui,
                        "Daily goal bar",
                        "Today's encounters towards your daily goal (set it in Profile > Goals).",
                        &mut o.daily_goal,
                    );
                    sep(ui);
                    d |= kit::toggle_row(ui, "Streak flame", "Days in a row you've hit your daily goal.", &mut o.streak);
                    sep(ui);
                    d |= kit::toggle_row(
                        ui,
                        "Personal best",
                        "Your best single day, turning gold when today beats it.",
                        &mut o.best,
                    );
                    self.dirty |= d;
                });
            }
            OverlayTab::Behaviour => {
                kit::card(ui, |ui| {
                    self.dirty |= kit::toggle_row(
                        ui,
                        "Stay visible over the game",
                        "Keeps the overlay visible in Borderless and Full screen by making the game window 1 pixel taller (off-screen, so you won't see it). Only the window size changes, nothing in the game.",
                        &mut self.cfg.borderless_fix,
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    self.dirty |= kit::toggle_row(
                        ui,
                        "Pin in place",
                        "Stops the overlay being dragged by accident.",
                        &mut self.cfg.pinned,
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    self.dirty |= kit::toggle_row(
                        ui,
                        "Shrink to badge",
                        "Shows just a small badge with your target and count.",
                        &mut self.cfg.mini,
                    );
                    if !self.cfg.overlay.legacy() {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                        let key = self.cfg.keybinds.get(HotAction::ClickThrough).map(|b| b.text());
                        let help = match &key {
                            Some(k) => format!("Clicks go straight through the overlay to the game. Press {k} to turn it off again (Settings > Hotkeys). It turns itself off while a shiny or the Caught! question needs a click."),
                            None => "Clicks go straight through the overlay to the game. Set a hotkey for it in Settings > Hotkeys first, so you can turn it off again.".to_string(),
                        };
                        ui.add_enabled_ui(key.is_some() || self.cfg.overlay.click_through, |ui| {
                            self.dirty |= kit::toggle_row(ui, "Click-through", &help, &mut self.cfg.overlay.click_through);
                        });
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    self.dirty |= kit::toggle_row(
                        ui,
                        "Flash when a target is spotted",
                        "The overlay's border blinks gold for a moment (with SPOTTED!) each time one of your targets turns up in a battle. Off: no flashing; a real shiny alert still flashes.",
                        &mut self.cfg.spot_flash,
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    self.dirty |= kit::toggle_row(
                        ui,
                        "Milestone celebrations",
                        "A pop-up under the overlay every 1,000 encounters and when you reach 1×, 2×, 3×… the odds.",
                        &mut self.cfg.celebrate_milestones,
                    );
                    if ui
                        .add(egui::Button::new(RichText::new("Preview celebration").size(kit::fs(12.0))).fill(kit::btn_fill()))
                        .clicked()
                    {
                        self.push_milestone("1,000 ENCOUNTERS!".to_string());
                        self.cfg.mini = false;
                        self.play(crate::sound::Sfx::Milestone);
                    }
                    if self.cfg.overlay.legacy() {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                        self.dirty |= kit::toggle_row(
                            ui,
                            "Today's hunting time",
                            "A small timer on the overlay showing how long you've hunted today, across all runs.",
                            &mut self.cfg.show_session_timer,
                        );
                    }
                });
            }
            OverlayTab::Sounds => self.sounds_card(ui),
        }
    }

    fn page_hotkeys(&mut self, ui: &mut egui::Ui) {
        kit::page_header(
            ui,
            "HOTKEYS",
            "Shortcuts that work even while the game is focused.",
        );

        // Capture the next key press while recording.
        if let Some(action) = self.setup.recording {
            ui.ctx().request_repaint();
            let pressed = ui.ctx().input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*key, *modifiers)),
                    _ => None,
                })
            });
            if let Some((key, m)) = pressed {
                let name = key.name();
                if key == egui::Key::Escape {
                    self.setup.recording = None;
                    self.setup.bind_msg = None;
                    self.apply_keybinds();
                } else if code_for(name).is_none() {
                    self.setup.bind_msg = Some((false, format!("\"{name}\" can't be used. Try a letter, number, F1–F12, an arrow, Home/End, Page Up/Down, Insert/Delete or Space.")));
                } else {
                    let bind = KeyBind {
                        ctrl: m.ctrl || m.command,
                        shift: m.shift,
                        alt: m.alt,
                        key: name.to_string(),
                    };
                    let is_fkey = name.starts_with('F') && name.len() > 1;
                    let clash = HotAction::ALL
                        .into_iter()
                        .find(|&a| a != action && self.cfg.keybinds.get(a) == Some(&bind));
                    if !bind.has_modifier() && !is_fkey {
                        self.setup.bind_msg = Some((
                            false,
                            format!(
                                "Add Ctrl, Shift or Alt to {} so it doesn't fire while you type.",
                                bind.text()
                            ),
                        ));
                    } else if let Some(other) = clash {
                        self.setup.bind_msg = Some((
                            false,
                            format!("{} is already used for \"{}\".", bind.text(), other.label()),
                        ));
                    } else {
                        self.setup.bind_msg =
                            Some((true, format!("{} → {}", action.label(), bind.text())));
                        self.cfg.keybinds.set(action, Some(bind));
                        self.setup.recording = None;
                        self.apply_keybinds();
                    }
                }
            }
        }

        let mut start = None;
        let mut clear = None;
        kit::card(ui, |ui| {
            for (n, action) in HotAction::ALL.into_iter().enumerate() {
                if n > 0 {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);
                }
                ui.horizontal(|ui| {
                    // Fixed-width, left-aligned label column.
                    ui.allocate_ui_with_layout(
                        egui::vec2(190.0, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_min_width(190.0);
                            ui.label(RichText::new(action.label()).size(kit::fs(13.0)).color(TEXT()));
                        },
                    );
                    if self.setup.recording == Some(action) {
                        ui.label(
                            RichText::new("Press new keys…  (Esc to cancel)")
                                .size(kit::fs(13.0))
                                .strong()
                                .color(GOLD),
                        );
                    } else if let Some(b) = self.cfg.keybinds.get(action) {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        for part in b.text().split('+') {
                            kit::keycap(ui, part);
                        }
                    } else {
                        ui.label(RichText::new("Off").size(kit::fs(12.0)).color(MUTED()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.cfg.keybinds.get(action).is_some()
                            && ui
                                .add(egui::Button::new("Clear").fill(kit::btn_fill()))
                                .clicked()
                        {
                            clear = Some(action);
                        }
                        if self.setup.recording != Some(action)
                            && ui
                                .add(egui::Button::new("Change").fill(kit::btn_fill()))
                                .clicked()
                        {
                            start = Some(action);
                        }
                    });
                });
            }
        });
        if let Some(a) = start {
            self.setup.recording = Some(a);
            self.setup.bind_msg = None;
            self.hotkeys.suspend();
        }
        if let Some(a) = clear {
            self.cfg.keybinds.set(a, None);
            self.setup.bind_msg = Some((true, format!("{} turned off", a.label())));
            self.apply_keybinds();
        }
        if let Some((ok, msg)) = &self.setup.bind_msg {
            kit::pill(ui, msg, if *ok { PillKind::Ok } else { PillKind::Bad });
            ui.add_space(6.0);
        }
        for e in self.hotkeys.errors.clone() {
            kit::pill(ui, &e, PillKind::Bad);
            ui.add_space(4.0);
        }
        ui.add_space(6.0);
        if ui
            .add(egui::Button::new("Reset to defaults").fill(kit::btn_fill()))
            .clicked()
        {
            self.cfg.keybinds = KeyBinds::default();
            self.setup.recording = None;
            self.setup.bind_msg = Some((true, "Hotkeys reset to defaults".into()));
            self.apply_keybinds();
        }
    }

    fn page_advanced(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "ADVANCED", "Fine-tuning. You shouldn't normally need these.");
        let mut tab = self.setup.adv_tab;
        if kit::tab_bar(
            ui,
            &mut tab,
            &[
                (AdvTab::Updates, "Updates"),
                (AdvTab::Data, "Backup & data"),
                (AdvTab::Counter, "Counter"),
                (AdvTab::Reading, "Screen reading"),
            ],
        ) {
            self.setup.adv_tab = tab;
        }
        ui.add_space(6.0);
        match self.setup.adv_tab {
            AdvTab::Updates => self.updates_card(ui),
            AdvTab::Data => {
                self.backup_card(ui);
                self.data_folder_card(ui);
            }
            AdvTab::Counter => {
                kit::section(ui, "Edit the counter");
                kit::card(ui, |ui| {
                    egui::Grid::new("counter-grid").num_columns(2).spacing([24.0, 10.0]).show(ui, |ui| {
                        for (label, v) in [
                            ("Encounters", &mut self.cfg.count),
                            ("Battles", &mut self.cfg.battles),
                            ("Target seen", &mut self.cfg.target_seen),
                        ] {
                            ui.label(RichText::new(label).size(kit::fs(13.0)).color(TEXT()));
                            self.dirty |= ui.add(egui::DragValue::new(v)).changed();
                            ui.end_row();
                        }
                    });
                    ui.add_space(8.0);
                    if self.setup.confirm_reset {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Reset this run to 0? This can't be undone.").color(theme::HP_RED));
                            if ui
                                .add(egui::Button::new(RichText::new("Reset").color(Color32::WHITE)).fill(theme::RED_DARK()))
                                .clicked()
                            {
                                self.cfg.count = 0;
                                self.cfg.battles = 0;
                                self.cfg.target_seen = 0;
                                self.dirty = true;
                                self.setup.confirm_reset = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.setup.confirm_reset = false;
                            }
                        });
                    } else if ui.add(egui::Button::new("Reset counter…").fill(kit::btn_fill())).clicked() {
                        self.setup.confirm_reset = true;
                    }
                });
            }
            AdvTab::Reading => {
                kit::section(ui, "Screen reading");
                let mut changed = false;
                kit::card(ui, |ui| {
                    ui.spacing_mut().slider_width = 220.0;
                    changed |= ui
                        .add(egui::Slider::new(&mut self.cfg.stable_reads, 1..=5).text("matching reads before counting"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut self.cfg.rearm_reads, 1..=10).text("empty reads before a battle ends"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut self.cfg.interval_ms, 100..=1000).text("ms between scans"))
                        .changed();
                });

                kit::section(ui, "Shiny star alert (experimental)");
                kit::card(ui, |ui| {
                    ui.spacing_mut().slider_width = 220.0;
                    changed |= ui
                        .add(egui::Slider::new(&mut self.cfg.star_threshold, 48..=400).text("star size needed"))
                        .changed();
                    ui.label(
                RichText::new("It looks for the ★ shape next to the name, and only alerts when it stays put for a moment. Lower this if your game window is small and real stars are missed.")
                    .size(kit::fs(11.5))
                    .color(MUTED()),
            );
                });
                if changed {
                    self.push_settings(false);
                }
            }
        }
    }

    fn welcome(&mut self, ctx: &egui::Context) {
        const STEPS: usize = 3;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(32.0, 24.0)))
            .show(ctx, |ui| {
                kit::settings_style(ui);
                let step = self.setup.welcome_step.min(STEPS - 1);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
                    ui.painter().image(
                        self.icon_tex.id(),
                        r,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    ui.label(
                        RichText::new(if kit::clean() { "Welcome to Poke Together Companion" } else { "WELCOME TO THE COMPANION" })
                            .font(if kit::clean() { crate::fonts::sans_bold_font(20.0) } else { kit::display_font(11.0) })
                            .color(TEXT()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("Skip").color(MUTED())).frame(false))
                            .clicked()
                        {
                            self.finish_welcome();
                        }
                    });
                });
                ui.add_space(10.0);
                kit::progress(ui, (step + 1) as f32 / STEPS as f32);
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("Step {} of {STEPS}", step + 1))
                        .size(kit::fs(11.0))
                        .color(MUTED()),
                );
                ui.add_space(14.0);

                let footer_h = 52.0;
                let body_h = (ui.available_height() - footer_h).max(120.0);
                egui::ScrollArea::vertical()
                    .id_salt("welcome-body")
                    .max_height(body_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match step {
                        0 => {
                            ui.label(
                                RichText::new("What are you hunting?")
                                    .size(kit::fs(18.5))
                                    .strong()
                                    .color(TEXT()),
                            );
                            ui.label(
                                RichText::new("Pick your target. You can change it any time.")
                                    .size(kit::fs(12.0))
                                    .color(MUTED()),
                            );
                            ui.add_space(12.0);
                            if let Some(slug) = self.target_picker(ui, body_h - 90.0) {
                                self.set_target(slug);
                            }
                        }
                        1 => {
                            ui.label(
                                RichText::new("How are you hunting?")
                                    .size(kit::fs(18.5))
                                    .strong()
                                    .color(TEXT()),
                            );
                            ui.label(
                                RichText::new("This sets your shiny odds.")
                                    .size(kit::fs(12.0))
                                    .color(MUTED()),
                            );
                            ui.add_space(12.0);
                            self.method_picker(ui);
                            ui.add_space(16.0);
                            kit::section(ui, "Singles or doubles?");
                            if self.battle_type_picker(ui) {
                                self.push_settings(true);
                            }
                            ui.add_space(14.0);
                            self.odds_tiles(ui);
                        }
                        _ => {
                            ui.label(
                                RichText::new("How should it count?")
                                    .size(kit::fs(18.5))
                                    .strong()
                                    .color(TEXT()),
                            );
                            ui.label(
                                RichText::new("The game log is exact and needs no setup.")
                                    .size(kit::fs(12.0))
                                    .color(MUTED()),
                            );
                            ui.add_space(12.0);
                            self.count_source_picker(ui);
                            if self.cfg.count_source == CountSource::Screen {
                                ui.add_space(8.0);
                                kit::pill(
                                    ui,
                                    "You'll set up the name boxes next, in the Counting page.",
                                    PillKind::Info,
                                );
                            }
                        }
                    });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if step > 0
                        && ui
                            .add(
                                egui::Button::new("Back")
                                    .fill(kit::btn_fill())
                                    .min_size(egui::vec2(90.0, 34.0)),
                            )
                            .clicked()
                    {
                        self.setup.welcome_step = step - 1;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let last = step + 1 == STEPS;
                        let label = if last { "Start hunting" } else { "Next" };
                        let ready = step != 0 || self.cfg.target.is_some();
                        let ink = if ready { theme::button_ink() } else { Color32::WHITE };
                        let btn = egui::Button::new(RichText::new(label).strong().color(ink))
                            .fill(kit::btn_fill())
                            .min_size(egui::vec2(130.0, 34.0));
                        let resp = if ready { kit::accent_button(ui, btn, 6.0) } else { ui.add(btn) };
                        if !ready {
                            resp.clone().on_hover_text("Pick a target first");
                        }
                        if resp.clicked() && ready {
                            if last {
                                self.finish_welcome();
                            } else {
                                self.setup.welcome_step = step + 1;
                            }
                        }
                    });
                });
            });
    }

    fn finish_welcome(&mut self) {
        self.cfg.welcome_done = true;
        self.setup.tab = if self.cfg.count_source == CountSource::Screen {
            Tab::Counting
        } else {
            Tab::Hunt
        };
        self.dirty = true;
        self.push_settings(true);
    }

    #[allow(dead_code)]
    fn now() -> Instant {
        Instant::now()
    }
}

/// Makes an area of a run card clickable ("View stats").
fn clickable(ui: &mut egui::Ui, rect: Rect, id: impl std::hash::Hash) -> bool {
    let r = ui.interact(rect, ui.id().with(id), Sense::click());
    if r.hovered() {
        ui.painter().rect_stroke(
            rect.expand(4.0),
            8.0,
            egui::Stroke::new(1.0f32, LINE()),
        );
    }
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("View stats")
        .clicked()
}
