//! The settings window: sidebar + six pages, and the first-run welcome.

use super::ui_kit::{self as kit, MUTED, PillKind, CARD, CARD_HOVER, LINE, PAGE, SIDEBAR, TEXT};
use super::{fmt_one_in, App, Tab};
use crate::config::{BattleType, Config, CountSource, HuntMethod};
use crate::theme;
use eframe::egui::{self, Color32, Rect, RichText, Sense};
use std::time::{Duration, Instant};


pub(super) fn method_icon(m: HuntMethod) -> &'static str {
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

pub(super) fn method_short(m: HuntMethod) -> (&'static str, &'static str) {
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

/// Sections of the Advanced settings page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum AdvTab {
    #[default]
    Updates,
    Data,
    Game,
    Counter,
    Reading,
    Help,
}

/// Methods on offer. "From the game" and "Custom" are no longer offered
/// (they still work for anyone who already picked one).
pub(super) fn method_offered(m: HuntMethod) -> bool {
    !matches!(m, HuntMethod::FromGameLog | HuntMethod::Custom)
}

/// Two paths are the same folder (Windows paths ignore case).
fn same_folder(a: &std::path::Path, b: &std::path::Path) -> bool {
    let norm = |p: &std::path::Path| {
        std::fs::canonicalize(p)
            .unwrap_or_else(|_| p.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
            .trim_end_matches(['\\', '/'])
            .to_string()
    };
    norm(a) == norm(b)
}

/// Egg hunting isn't well tested yet.
pub(super) fn method_experimental(m: HuntMethod) -> bool {
    matches!(m, HuntMethod::Masuda)
}

pub(super) const EXPERIMENTAL_NOTE: &str = "Experimental: issues may occur with this hunting style.";

/// Methods that don't work yet: shown greyed out and can't be picked
/// (a hunt already set to one is left as it is).
pub(super) fn method_unavailable(m: HuntMethod) -> bool {
    matches!(m, HuntMethod::PokeRadar | HuntMethod::Masuda)
}

pub(super) const UNAVAILABLE_NOTE: &str = "Not working yet: this hunting style comes in a later update.";

/// Grey a method tile out and mark it "Not working yet".
pub(super) fn grey_tile(ui: &egui::Ui, tile: Rect, rounding: f32) {
    let p = ui.painter();
    p.rect_filled(tile, rounding, theme::blend(kit::PAGE(), kit::CARD(), 0.5).gamma_multiply(0.72));
    let g = p.layout_no_wrap("Not working yet".to_string(), egui::FontId::proportional(kit::fs(10.5)), MUTED());
    // bottom-left, where the odds would be (they aren't drawn for these)
    let r = Rect::from_min_size(egui::pos2(tile.min.x + 14.0, tile.max.y - g.size().y - 20.0), g.size() + egui::vec2(12.0, 8.0));
    p.rect_filled(r, r.height() / 2.0, kit::sunken());
    p.rect_stroke(r, r.height() / 2.0, egui::Stroke::new(1.0f32, kit::LINE()));
    p.galley(r.min + egui::vec2(6.0, 4.0), g, MUTED());
}

/// Small amber "issues may occur" chip in a method tile's bottom-right
/// corner (clear of the title, beside the odds badge).
fn warn_chip(ui: &egui::Ui, tile: Rect, _selected: bool) {
    let p = ui.painter();
    let font = egui::FontId::proportional(kit::fs(10.5));
    let ink = if theme::pal().light { theme::blend(theme::HP_YELLOW, Color32::BLACK, 0.35) } else { theme::HP_YELLOW };
    let galley = p.layout_no_wrap("⚠ Issues may occur".to_string(), font, ink);
    let right = tile.right() - 12.0;
    let r = Rect::from_min_size(
        egui::pos2(right - galley.size().x - 12.0, tile.bottom() - 27.0),
        galley.size() + egui::vec2(12.0, 4.0),
    );
    p.rect_filled(r, r.height() / 2.0, ink.gamma_multiply(0.16));
    p.rect_stroke(r, r.height() / 2.0, egui::Stroke::new(1.0f32, ink.gamma_multiply(0.6)));
    p.galley(r.min + egui::vec2(6.0, 2.0), galley, ink);
}

/// "1 in 2,048" for a method, using the rest of the current settings.
pub(super) fn method_badge(cfg: &Config, m: HuntMethod) -> String {
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
            // First frame: put the window back where it was (in this
            // screen's own scale, and fully on screen), then start noting
            // where it goes once it has settled.
            if !self.launcher_placed {
                self.launcher_placed = true;
                if !maxed {
                    if let Some(pos) = self.cfg.settings_pos {
                        let size = self.cfg.settings_size.unwrap_or([1100.0, 740.0]);
                        let size = [size[0].max(720.0), size[1].max(600.0)];
                        let to = crate::topmost::fit_on_screen(Some(pos), size, ctx.pixels_per_point(), false);
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(size[0], size[1])));
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(to[0], to[1])));
                    }
                }
            } else if self.started.elapsed() > std::time::Duration::from_millis(1500) {
                self.remember_settings_geometry(maxed, minimized, outer, inner);
            }
            // Match Windows 11's own rounded window corners (8px); square when maximized.
            let r = if maxed { 0.0 } else { 8.0 };
            crate::topmost::round_settings_corners(&mut self.settings_rounded);
            // Header bar like the overlay's: drag to move, minimise, close.
            // (Modern: a top bar with the overlay's controls.)
            let modern = kit::modern();
            let top_h = if modern { super::shell::TOP_H } else { 38.0 };
            egui::TopBottomPanel::top("settings-header")
                .exact_height(top_h)
                .frame(egui::Frame::none())
                .show_separator_line(false)
                .show(ctx, |ui| {
                    self.modern_header(ctx, ui)
                });
            let body_top = screen.min.y + top_h;
            let bg = ctx.layer_painter(egui::LayerId::background());
            bg.rect_filled(
                Rect::from_min_max(egui::pos2(screen.min.x, body_top - 1.0), screen.max),
                egui::Rounding { nw: 0.0, ne: 0.0, sw: r, se: r },
                PAGE(),
            );
            self.apps_tick(ctx);
            self.app_overlays(ctx);
            if self.apps_welcome_open() {
                self.apps_welcome(ctx);
            } else if self.onboarding_open() {
                self.onboarding(ctx);
            } else {
                // (Panels draw on the background layer, so the sidebar fills
                // itself rather than being painted over afterwards.)
                egui::SidePanel::left("settings-nav")
                    .exact_width(if modern { self.nav_width() } else { 184.0 })
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(
                        egui::Frame::none()
                            .fill(SIDEBAR())
                            .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: r, se: 0.0 })
                            .inner_margin(if modern { egui::Margin { left: 12.0, right: 12.0, top: 4.0, bottom: 12.0 } } else { egui::Margin::symmetric(10.0, 14.0) }),
                    )
                    .show(ctx, |ui| {
                        kit::settings_style(ui);
                        self.modern_sidebar(ui)
                    });
                kit::set_page_icon(if modern {
                    let (icon, _, color) = super::shell::tab_meta(self.setup.tab);
                    Some((icon, color))
                } else {
                    None
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
                    .frame(egui::Frame::none().inner_margin(if modern {
                        egui::Margin {
                            left: 32.0,
                            right: 8.0,
                            top: 26.0,
                            bottom: 12.0,
                        }
                    } else if kit::clean() {
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
                            0u8,
                            0u8,
                            0u8,
                            0u8,
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
                        // (Each page fades in and lifts into place as it opens.)
                        // (Only a new page fades in; a page's own tabs slide instead.)
                        let page_hash = {
                            use std::hash::{Hash, Hasher};
                            let mut h = std::collections::hash_map::DefaultHasher::new();
                            (
                                self.setup.tab as u8,
                                self.setup.view_run.is_some(),
                                self.setup.share.is_some(),
                                self.setup.profile_card.is_some(),
                                self.setup.phase_pick,
                                self.setup.caught_pick,
                            )
                                .hash(&mut h);
                            h.finish()
                        };
                        if last.as_deref() != Some(page_key.as_str()) {
                            kit::bump_page_visit(ui.ctx());
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
                                        && matches!(self.setup.tab, Tab::Home | Tab::Store | Tab::Library | Tab::Profile | Tab::Stats | Tab::Dex | Tab::Trophies | Tab::Runs | Tab::Pokedex | Tab::Map | Tab::Info | Tab::Team | Tab::Sim | Tab::Breed | Tab::Calc | Tab::Cards));
                                let cap = if modern { 1060.0 } else { 900.0 };
                                let w = if wide { ui.available_width() } else { ui.available_width().min(cap) };
                                ui.set_max_width(w);
                                // The Map fills the window itself, so it only fades.
                                let t = kit::entering(ui.ctx(), egui::Id::new("page-enter"), page_hash, kit::ENTER);
                                kit::enter_ui(ui, t, if self.setup.tab == Tab::Map { 0.0 } else { 14.0 });
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
                                    if super::launcher::is_shiny_tab(self.setup.tab) && self.setup.tab != Tab::Shiny && !modern {
                                        self.overlay_strip(ui);
                                    }
                                    match self.setup.tab {
                                        Tab::Home => self.page_home(ui),
                                        Tab::Store => self.page_store(ui),
                                        Tab::Library => self.page_library(ui),
                                        Tab::Shiny => self.page_shiny(ui),
                                        Tab::Pokedex => self.page_pokedex(ui),
                                        Tab::Map => self.page_map(ui),
                                        Tab::Info => self.page_info(ui),
                                        Tab::Team => self.page_team(ui),
                                        Tab::Sim => self.page_sim(ui),
                                        Tab::Breed => self.page_breed(ui),
                                        Tab::Calc => self.page_damage_calc(ui),
                                        Tab::Cards => self.page_cards(ui),
                                        Tab::Profile => self.page_profile(ui),
                                        Tab::Stats => self.page_stats(ui),
                                        Tab::Dex | Tab::Trophies => {
                                            // the Shiny Dex lives in the Pokédex now
                                            self.setup.tab = Tab::Pokedex;
                                            self.setup.pokedex.shiny = true;
                                            self.setup.pokedex.open = None;
                                            self.page_pokedex(ui)
                                        }
                                        Tab::Hunt => self.page_hunt(ui),
                                        Tab::Counting => self.page_counting(ui),
                                        Tab::Runs => self.page_runs(ui),
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
                                        egui::Button::new(RichText::new("↑  Top").size(kit::fs(12.5)).family(kit::strong_family(kit::fs(12.5))).strong().color(TEXT()))
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
                let edge = if kit::modern() { theme::blend(SIDEBAR(), egui::Color32::BLACK, 0.5) } else { theme::OUTLINE };
                fg.rect_stroke(screen.shrink(0.5), r, egui::Stroke::new(if kit::modern() { 1.0f32 } else { 1.5f32 }, edge));
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

    /// Save, move the data folder (None = back to default), save again there.
    pub(super) fn move_data(&mut self, target: Option<std::path::PathBuf>) {
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
    pub(super) fn method_picker(&mut self, ui: &mut egui::Ui) {
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
                    let badge = if method_unavailable(m) { String::new() } else { method_badge(&self.cfg, m) };
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
                    let off = method_unavailable(m);
                    if off {
                        grey_tile(ui, tile.rect, 10.0);
                    } else if method_experimental(m) {
                        warn_chip(ui, tile.rect, sel);
                    }
                    let tip = if off {
                        format!("{UNAVAILABLE_NOTE}\n\n{}", m.note())
                    } else if method_experimental(m) {
                        format!("{}\n\n⚠ {EXPERIMENTAL_NOTE}", m.note())
                    } else {
                        m.note().to_string()
                    };
                    let tile = if off { tile.on_hover_cursor(egui::CursorIcon::NotAllowed) } else { tile };
                    if tile.on_hover_text(tip).clicked() && !off {
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

    pub(super) fn battle_type_picker(&mut self, ui: &mut egui::Ui) -> bool {
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

    pub(super) fn count_source_picker(&mut self, ui: &mut egui::Ui) -> bool {
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

    pub(super) fn odds_tiles(&self, ui: &mut egui::Ui) {
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

    /// Open Advanced settings at the game folder (from the Map and Counting).
    pub(super) fn open_game_folder_settings(&mut self) {
        self.setup.tab = Tab::Advanced;
        self.setup.adv_jump = Some(AdvTab::Game);
    }

    /// The game folder in use (chosen in Advanced settings, else the usual place).
    pub(super) fn game_folder(&self) -> std::path::PathBuf {
        crate::gamemap::game_dir(self.cfg.game_dir_setting().as_deref())
    }

    /// Compare where the game is running from with the folder in use. With
    /// no folder chosen, follow the game; otherwise offer to switch.
    pub(super) fn check_game_location(&mut self) {
        use super::notices::{Notice, NoticeAction};
        let Some(dir) = crate::topmost::game_exe().and_then(|e| e.parent().map(|d| d.to_path_buf())) else { return };
        let using = self.game_folder();
        if same_folder(&dir, &using) {
            return;
        }
        let dir_s = dir.to_string_lossy().to_string();
        if self.cfg.game_dir.is_none() {
            self.cfg.game_dir_found = Some(dir_s);
            self.game_folder_changed();
            self.notify(Notice {
                id: "game-folder",
                icon: "📁",
                title: "Found the game".into(),
                body: format!("It's running from {}. Counting and the Map read from there now.", dir.display()),
                progress: None,
                action: None,
                expires: Some(Instant::now() + Duration::from_secs(12)),
                accent: theme::GREEN,
            });
        } else if self.game_dir_offer.as_deref() != Some(dir_s.as_str()) {
            self.game_dir_offer = Some(dir_s);
            self.notify(Notice {
                id: "game-folder",
                icon: "📁",
                title: "The game is in a different folder".into(),
                body: format!("It's running from {}, but your game folder setting is {}.", dir.display(), using.display()),
                progress: None,
                action: Some(("Use that folder", NoticeAction::UseRunningGame)),
                expires: None,
                accent: theme::HP_YELLOW,
            });
        }
    }

    /// "Use that folder": switch the game folder setting to where the game runs.
    pub(super) fn use_running_game_folder(&mut self) {
        if let Some(d) = self.game_dir_offer.clone() {
            self.cfg.game_dir = Some(d);
            self.game_folder_changed();
        }
        self.clear_notice("game-folder");
    }

    /// After the game folder changes: counting and the Map look there now.
    pub(super) fn game_folder_changed(&mut self) {
        self.dirty = true;
        self.push_settings(true);
        self.gamemap = None;
        self.gamemap_rx = crate::gamemap::start(self.cfg.game_dir_setting());
        self.mapui = Default::default();
    }

    /// End the onboarding tour: it won't open again until Advanced > Updates
    /// asks for it (or a release raises ONBOARDING_VERSION).
    pub(super) fn finish_welcome(&mut self) {
        self.cfg.welcome_done = true;
        self.cfg.onboarding_seen = crate::config::ONBOARDING_VERSION;
        self.setup.welcome_step = 0;
        self.setup.tab = if self.cfg.count_source == CountSource::Screen && self.cfg.name_regions.is_empty() {
            Tab::Counting
        } else {
            Tab::Home
        };
        self.dirty = true;
        self.push_settings(true);
    }

    #[allow(dead_code)]
    fn now() -> Instant {
        Instant::now()
    }
}
