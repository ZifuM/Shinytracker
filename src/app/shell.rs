//! The modern window frame: a top bar (brand, where you are, the overlay's
//! Start / Stop and the window buttons) and a sidebar with colour-coded
//! icon chips that can fold down to icons only. The classic and clean looks
//! keep their own frame (settings.rs).

use super::launcher::is_shiny_tab;
use super::ui_kit::{self as kit, modern, MUTED, PAGE, SIDEBAR, TEXT};
use super::{App, Tab};
use crate::apps::AppId;
use crate::theme;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::Duration;

/// Height of the top bar.
pub(super) const TOP_H: f32 = 56.0;
/// Sidebar width, open and folded to icons.
pub(super) const NAV_W: f32 = 236.0;
pub(super) const RAIL_W: f32 = 72.0;

/// Each page's icon, name and colour (sidebar, top bar and page header).
pub(super) fn tab_meta(t: Tab) -> (&'static str, &'static str, Color32) {
    match t {
        Tab::Home => ("🏠", "Dashboard", kit::C_BLUE),
        Tab::Store => ("🏪", "App Store", STORE_COL),
        // (the ⊞ glyph isn't in the icon font: the Library's chip was blank)
        Tab::Library => ("📚", "Library", LIBRARY_COL),
        Tab::Profile => ("🏅", "Profile", kit::C_GOLD),
        Tab::Stats => ("📊", "Stats", kit::C_CYAN),
        Tab::Shiny => ("✨", "Shiny Hunter", kit::C_VIOLET),
        Tab::Hunt => ("🎯", "Hunt", kit::C_PINK),
        Tab::Counting => ("👁", "Counting", kit::C_CYAN),
        Tab::Runs => ("📜", "Runs", kit::C_GREEN),
        Tab::Dex => ("⭐", "Shiny Dex", kit::C_GOLD),
        Tab::Trophies => ("🏆", "Trophies", kit::C_GOLD),
        Tab::Overlay => ("🖵", "Overlay", kit::C_INDIGO),
        Tab::Pokedex => ("📖", "Pokédex", kit::C_RED),
        Tab::Map => ("🗺", "Map", kit::C_GREEN),
        Tab::Info => ("📚", "Game Info", kit::C_ORANGE),
        Tab::Team => ("🛡", "Team Planner", kit::C_TEAL),
        Tab::Sim => ("🎮", "Battle Sim", kit::C_RED),
        Tab::Breed => ("🐣", "Breeding", kit::C_PINK),
        Tab::Calc => ("💥", "Damage Calculator", kit::C_INDIGO),
        Tab::Cards => ("🃏", "Card Collector", kit::C_GOLD),
        Tab::Gotchi => ("🐾", "Pokégotchi", kit::C_PINK),
        Tab::Bingo => ("🎯", "Shiny Bingo", kit::C_VIOLET),
        Tab::RareGrass => ("✨", "Rare Grass", kit::C_GREEN),
        Tab::Replay => ("🎬", "Shiny Replay", super::replay_app::ROSE),
        Tab::Journal => ("📓", "Journal", super::journal_app::JOURNAL_COL),
        Tab::Boxes => ("📦", "Pokémon Boxes", super::boxes_app::BOX_COL),
        Tab::Nuzlocke => ("💀", "Nuzlocke Tracker", super::nuzlocke_app::NUZ_COL),
        Tab::Stickers => ("📌", "Sticker Book", super::sticker_app::STICKER_COL),
        Tab::Appearance => ("🎨", "Appearance", kit::C_PINK),
        Tab::Hotkeys => ("⌨", "Hotkeys", kit::C_SLATE),
        Tab::Advanced => ("⚙", "Advanced", kit::C_SLATE),
        Tab::Sounds => ("🔊", "Sounds", kit::C_PINK),
    }
}

/// Pages that are still experimental (marked in the sidebar).
pub(super) fn is_experimental(t: Tab) -> bool {
    matches!(t, Tab::Sim | Tab::Breed | Tab::Cards | Tab::Gotchi | Tab::Bingo | Tab::RareGrass | Tab::Replay | Tab::Journal | Tab::Boxes | Tab::Nuzlocke | Tab::Stickers)
}

/// The App Store's and the Library's colours.
pub(super) const STORE_COL: Color32 = Color32::from_rgb(255, 112, 92);
pub(super) const LIBRARY_COL: Color32 = Color32::from_rgb(124, 150, 255);

/// Shiny Hunter's own pages, shown under it in the sidebar while it's open.
/// (Counting and Overlay are its settings now: Settings → Shiny Hunter.)
pub(super) const SHINY_PAGES: [Tab; 2] = [Tab::Hunt, Tab::Runs];

/// The app a page belongs to (None: Home, the store, settings and the like).
pub(super) fn app_of(t: Tab) -> Option<AppId> {
    Some(match t {
        Tab::Shiny | Tab::Hunt | Tab::Counting | Tab::Runs | Tab::Overlay => AppId::Shiny,
        Tab::Pokedex | Tab::Dex | Tab::Trophies => AppId::Pokedex,
        Tab::Map => AppId::Map,
        Tab::Info => AppId::Info,
        Tab::Team => AppId::Team,
        Tab::Sim => AppId::Sim,
        Tab::Breed => AppId::Breed,
        Tab::Calc => AppId::Calc,
        Tab::Cards => AppId::Cards,
        Tab::Gotchi => AppId::Gotchi,
        Tab::Bingo => AppId::Bingo,
        Tab::RareGrass => AppId::Rare,
        Tab::Replay => AppId::Replay,
        Tab::Journal => AppId::Journal,
        Tab::Boxes => AppId::Boxes,
        Tab::Nuzlocke => AppId::Nuzlocke,
        Tab::Stickers => AppId::Stickers,
        _ => return None,
    })
}

/// An app's main page.
pub(super) fn app_tab(a: AppId) -> Tab {
    match a {
        AppId::Shiny => Tab::Shiny,
        AppId::Pokedex => Tab::Pokedex,
        AppId::Map => Tab::Map,
        AppId::Info => Tab::Info,
        AppId::Team => Tab::Team,
        AppId::Sim => Tab::Sim,
        AppId::Breed => Tab::Breed,
        AppId::Calc => Tab::Calc,
        AppId::Cards => Tab::Cards,
        AppId::Gotchi => Tab::Gotchi,
        AppId::Bingo => Tab::Bingo,
        AppId::Rare => Tab::RareGrass,
        AppId::Replay => Tab::Replay,
        AppId::Journal => Tab::Journal,
        AppId::Boxes => Tab::Boxes,
        AppId::Nuzlocke => Tab::Nuzlocke,
        AppId::Stickers => Tab::Stickers,
    }
}

/// The page above a sub-page in the sidebar.
pub(super) fn nav_parent(t: Tab) -> Option<Tab> {
    (SHINY_PAGES.contains(&t) || matches!(t, Tab::Counting | Tab::Overlay)).then_some(Tab::Shiny)
}

/// One row of the sidebar.
pub(super) struct NavRow {
    /// A heading above the row.
    pub group: Option<&'static str>,
    pub tab: Tab,
    pub parent: Option<Tab>,
    /// An open app (has a close button).
    pub app: Option<AppId>,
}

impl App {
    /// The sidebar: Home, the store, the Library, Profile and Stats always;
    /// then the apps that are open (closing one takes it off). Settings are
    /// behind the cog, top right.
    pub(super) fn nav_rows(&self) -> Vec<NavRow> {
        let row = |group, tab| NavRow { group, tab, parent: None, app: None };
        let mut v = vec![row(None, Tab::Home), row(None, Tab::Store), row(None, Tab::Library), row(None, Tab::Profile), row(None, Tab::Stats)];
        let mut first = true;
        let sidebar_pins = self.cfg.apps.pins_place == crate::apps::PinsPlace::Sidebar;
        // with the bottom dock, open apps live in the dock instead
        if self.cfg.apps.pins_place == crate::apps::PinsPlace::Dock {
            return v;
        }
        for a in self.cfg.apps.open_list() {
            if sidebar_pins && a != AppId::Shiny && self.cfg.apps.is_pinned(a) {
                continue;
            }
            let tab = app_tab(a);
            v.push(NavRow { group: first.then_some("OPEN APPS"), tab, parent: None, app: Some(a) });
            first = false;
            if a == AppId::Shiny {
                // (Counting and Overlay too, when they're pages of the app)
                let pages: &[Tab] = if self.cfg.apps.shiny_pages_in_app { &[Tab::Hunt, Tab::Counting, Tab::Runs, Tab::Overlay] } else { &SHINY_PAGES };
                for t in pages.iter().copied() {
                    v.push(NavRow { group: None, tab: t, parent: Some(Tab::Shiny), app: None });
                }
            }
        }
        v
    }

    /// Open the settings window at `tab` (Appearance, Hotkeys or Advanced).
    pub(super) fn open_settings(&mut self, tab: Tab) {
        self.setup.settings_open = Some(tab);
        self.setup.scroll_top = true;
    }

    /// The "nothing open" note in the sidebar (where open apps would be).
    pub(super) fn nav_none_open(&mut self, ui: &mut egui::Ui, compact: bool) {
        if compact || !self.cfg.apps.open_list().is_empty() || self.cfg.apps.pins_place == crate::apps::PinsPlace::Dock {
            return;
        }
        kit::nav_group(ui, "OPEN APPS");
        let w = ui.available_width();
        let galley = ui.painter().layout(
            "Nothing open. Open an app from your Library and it shows up here.".to_string(),
            crate::fonts::sans_font(12.0),
            MUTED(),
            w - 24.0,
        );
        let (r, resp) = ui.allocate_exact_size(egui::vec2(w, galley.size().y + 20.0), Sense::click());
        let p = ui.painter();
        p.rect_stroke(r.shrink(1.0), 12.0, Stroke::new(1.0f32, kit::LINE()));
        p.galley(r.min + egui::vec2(12.0, 10.0), galley, MUTED());
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.go_to(Tab::Library);
        }
    }

    /// The ✕ on an open app's row (and middle-click, and its right-click menu).
    /// True when it closed the app.
    pub(super) fn nav_close_button(&mut self, ui: &mut egui::Ui, row: Rect, resp: &egui::Response, a: AppId, show: bool) -> bool {
        let xr = Rect::from_center_size(egui::pos2(row.max.x - 16.0, row.center().y), egui::vec2(22.0, 22.0));
        let mut close = resp.middle_clicked();
        if show {
            let xresp = ui.interact(xr, ui.id().with(("nav-close", a)), Sense::click());
            let hov = xresp.hovered();
            let p = ui.painter();
            if hov {
                p.rect_filled(xr, 7.0, theme::blend(SIDEBAR(), Color32::from_rgb(255, 110, 140), 0.18));
            }
            let c = xr.center();
            let ink = if hov { Color32::from_rgb(255, 150, 170) } else { MUTED() };
            let s = Stroke::new(1.4f32, ink);
            p.line_segment([c - egui::vec2(4.0, 4.0), c + egui::vec2(4.0, 4.0)], s);
            p.line_segment([c + egui::vec2(-4.0, 4.0), c + egui::vec2(4.0, -4.0)], s);
            if xresp.on_hover_text(format!("Close {}", a.name())).clicked() {
                close = true;
            }
        }
        resp.context_menu(|ui| {
            if ui.button(format!("Close {}", a.name())).clicked() {
                close = true;
                ui.close_menu();
            }
            if ui.button("Store page").clicked() {
                self.show_store_page(a);
                ui.close_menu();
            }
        });
        if close {
            self.close_app(a);
        }
        close
    }
}

impl App {
    pub(super) fn nav_width(&self) -> f32 {
        if self.nav_is_compact() {
            RAIL_W
        } else {
            NAV_W
        }
    }

    /// The sidebar shows only icons: as set in Appearance (the same on every
    /// page, the Journal included).
    pub(super) fn nav_is_compact(&self) -> bool {
        self.cfg.appearance.nav_compact
    }

    /// Fold or unfold the sidebar.
    pub(super) fn nav_fold_toggle(&mut self) {
        self.cfg.appearance.nav_compact = !self.cfg.appearance.nav_compact;
        self.dirty = true;
    }

    /// Open a page from the sidebar (same rules as the classic sidebar).
    pub(super) fn go_to(&mut self, tab: Tab) {
        // settings live in their own window now (Shiny Hunter's Counting
        // and Overlay too)
        // (unless they're set to be Shiny Hunter pages again)
        let settings_page = matches!(tab, Tab::Appearance | Tab::Sounds | Tab::Hotkeys | Tab::Advanced) || (matches!(tab, Tab::Counting | Tab::Overlay) && !self.cfg.apps.shiny_pages_in_app);
        if settings_page {
            self.open_settings(tab);
            return;
        }
        let current = self.setup.tab;
        let expandable = tab == Tab::Shiny;
        if tab == Tab::Store {
            self.setup.store.app = None;
        }
        let collapsed = self.setup.nav_collapsed.contains(&tab);
        if expandable {
            if collapsed {
                self.setup.nav_collapsed.retain(|t| *t != tab);
            } else if current == tab {
                self.setup.nav_collapsed.push(tab);
            }
        }
        self.setup.scroll_top = current == tab || tab != Tab::Pokedex;
        if tab == Tab::Info && current == Tab::Info {
            self.setup.info.reset_home();
        }
        self.setup.tab = tab;
        self.setup.picking_target = false;
        self.setup.view_run = None;
        self.setup.share = None;
        self.setup.profile_card = None;
        self.setup.phase_pick = false;
        self.setup.caught_pick = false;
    }

    /// The top bar.
    /// Modern Plus with the pinned apps in the bottom dock: the pages are
    /// tabs in the title bar, there's no sidebar, and the hunt rides on the
    /// dock (Appearance → Look).
    pub(super) fn plus_tabs(&self) -> bool {
        kit::plus() && self.cfg.apps.pins_place == crate::apps::PinsPlace::Dock && !self.takeover_open()
    }

    pub(super) fn modern_header(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let maxed = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let r = if maxed { 0.0 } else { 8.0 };
        let p = ui.painter().clone();
        let tabs = self.plus_tabs();
        // (No sidebar during the first-run welcome, or with Modern Plus's tabs.)
        let nav_w = if !self.takeover_open() && !tabs { self.nav_width() } else { 0.0 };
        let left = Rect::from_min_max(rect.min, egui::pos2(rect.min.x + nav_w, rect.max.y));
        let right = Rect::from_min_max(egui::pos2(left.max.x, rect.min.y), rect.max);
        p.rect_filled(left, egui::Rounding { nw: r, ..Default::default() }, SIDEBAR());
        // (a Modern Plus theme can paint the title bar: Pokédex's red)
        let (hf, hi, hm) = header_colours();
        p.rect_filled(right, egui::Rounding { ne: r, ..Default::default() }, hf);
        // (Aurora glass: its glows run up under the title bar too)
        theme::paint_glows(&p, ctx.screen_rect());
        p.line_segment([right.left_bottom() + egui::vec2(0.0, -0.5), right.right_bottom() + egui::vec2(0.0, -0.5)], Stroke::new(1.0f32, kit::LINE()));
        // The brand's colours as a hairline across the very top.
        let pl = theme::pal();
        if theme::chrome().is_none() {
        p.add(theme::gradient_rect(
            Rect::from_min_size(rect.min, egui::vec2(rect.width(), 2.0)),
            [r, r, 0.0, 0.0],
            pl.accent,
            if pl.gradient { pl.accent2 } else { pl.accent },
        ));
        }

        // Brand.
        let compact = self.nav_is_compact() && !self.takeover_open() && !tabs;
        let logo_c = if compact { left.center() } else { rect.left_center() + egui::vec2(30.0, 0.0) };
        p.image(
            self.icon_tex.id(),
            Rect::from_center_size(logo_c, egui::vec2(28.0, 28.0)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        if !compact {
            p.text(logo_c + egui::vec2(22.0, -8.0), Align2::LEFT_CENTER, "Poke Together", crate::fonts::sans_bold_font(14.5), hi);
            p.text(logo_c + egui::vec2(22.0, 9.0), Align2::LEFT_CENTER, "Companion", crate::fonts::sans_font(12.0), hm);
        }

        // Where you are: "Shiny Hunter › Hunt".
        let tab = self.setup.tab;
        let (_, name, color) = tab_meta(tab);
        let mut x = if tabs { rect.min.x + 196.0 } else if !self.takeover_open() { right.min.x + 28.0 } else { rect.min.x + 200.0 };
        let cy = right.center().y;
        // the sidebar's fold button: a panel picture, first thing after the sidebar
        // (its click is taken after the window-drag area, which would cover it)
        let fold_r = Rect::from_center_size(egui::pos2(right.min.x + 26.0, cy), egui::vec2(32.0, 30.0));
        if !self.takeover_open() && !tabs {
            let hov = ui.rect_contains_pointer(fold_r);
            if hov {
                p.rect_filled(fold_r, 7.0, theme::blend(PAGE(), TEXT(), 0.08));
            }
            panel_icon(&p, fold_r.center(), self.nav_is_compact(), if hov { TEXT() } else { theme::blend(PAGE(), TEXT(), 0.75) });
            x = fold_r.max.x + 14.0;
        }
        if is_shiny_tab(tab) && tab != Tab::Shiny && !tabs {
            let g = p.text(egui::pos2(x, cy), Align2::LEFT_CENTER, "Shiny Hunter", crate::fonts::sans_font(14.0), MUTED());
            x = g.max.x + 10.0;
            let g = p.text(egui::pos2(x, cy), Align2::LEFT_CENTER, "›", crate::fonts::sans_font(15.0), MUTED());
            x = g.max.x + 10.0;
        }
        let mut crumb_end = x;
        if !self.takeover_open() && !tabs {
            p.circle_filled(egui::pos2(x + 4.0, cy), 4.0, color);
            crumb_end = p.text(egui::pos2(x + 16.0, cy), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(15.0), TEXT()).max.x;
        }

        // Window buttons (right edge).
        let bw = 44.0;
        let close = Rect::from_min_size(egui::pos2(rect.max.x - bw, rect.min.y + 2.0), egui::vec2(bw, rect.height() - 3.0));
        let maxb = close.translate(egui::vec2(-bw, 0.0));
        let minb = maxb.translate(egui::vec2(-bw, 0.0));
        for (i, br) in [minb, maxb, close].into_iter().enumerate() {
            let resp = ui.interact(br, ui.id().with(("mhdr-btn", i)), Sense::click());
            let hov = resp.hovered();
            if hov {
                // (on a red title bar the close button's red hover goes deeper instead)
                let close_fill = if theme::chrome().is_some() { theme::blend(hf, Color32::BLACK, 0.32) } else { Color32::from_rgb(220, 50, 60) };
                let fill = if i == 2 { close_fill } else { theme::blend(hf, hi, 0.10) };
                let rr = if i == 2 { egui::Rounding { ne: r, ..Default::default() } } else { egui::Rounding::ZERO };
                p.rect_filled(br, rr, fill);
            }
            let c = br.center();
            let ink = if hov && i == 2 { Color32::WHITE } else { theme::blend(hf, hi, 0.8) };
            let s = Stroke::new(1.3f32, ink);
            match i {
                0 => {
                    p.line_segment([c - egui::vec2(6.0, 0.0), c + egui::vec2(6.0, 0.0)], s);
                    if resp.on_hover_text("Minimise").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                }
                1 => {
                    if maxed {
                        let back = Rect::from_center_size(c + egui::vec2(1.5, -1.5), egui::vec2(9.0, 9.0));
                        let front = Rect::from_center_size(c + egui::vec2(-1.5, 1.5), egui::vec2(9.0, 9.0));
                        p.line_segment([back.left_top(), back.right_top()], s);
                        p.line_segment([back.right_top(), back.right_bottom()], s);
                        p.rect_stroke(front, 1.5, s);
                    } else {
                        p.rect_stroke(Rect::from_center_size(c, egui::vec2(11.0, 11.0)), 1.5, s);
                    }
                    if resp.on_hover_text(if maxed { "Restore" } else { "Maximise" }).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed));
                    }
                }
                _ => {
                    p.line_segment([c - egui::vec2(5.5, 5.5), c + egui::vec2(5.5, 5.5)], s);
                    p.line_segment([c + egui::vec2(-5.5, 5.5), c + egui::vec2(5.5, -5.5)], s);
                    if resp.on_hover_text("Close the Companion (saves everything)").clicked() {
                        self.quit_app(ctx);
                    }
                }
            }
        }

        // Settings: the cog, top right (opens the settings window).
        let cog = Rect::from_center_size(egui::pos2(minb.min.x - 26.0, cy), egui::vec2(36.0, 34.0));
        if !self.takeover_open() {
            let resp = ui.interact(cog, ui.id().with("mhdr-cog"), Sense::click());
            let open = self.setup.settings_open.is_some();
            let hov = resp.hovered();
            if hov || open {
                p.rect_filled(cog, 8.0, theme::blend(hf, hi, if open { 0.14 } else { 0.10 }));
            }
            cog_icon(&p, cog.center(), 8.0, if hov || open { hi } else { theme::blend(hf, hi, 0.75) });
            super::app_tour::mark(ui.ctx(), "top.settings", cog);
            // (it opens at the settings of the app you're on)
            let tip = match app_of(self.setup.tab) {
                Some(a) => format!("Settings: {} and the rest of the Companion", a.name()),
                None => "Settings: every app's, appearance, hotkeys and more".to_string(),
            };
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked() {
                if open {
                    self.close_settings();
                } else {
                    self.open_settings(super::settings_apps::section_for(self.setup.tab));
                }
            }
        }

        // The overlay's status and its Start / Stop, always in reach.
        let controls = Rect::from_min_max(egui::pos2(right.min.x + 240.0, rect.min.y), egui::pos2(cog.min.x - 8.0, rect.max.y));
        let mut drag_until = controls.max.x;
        if tabs {
            // Modern Plus: the search box lives up here (the hunt and Start
            // overlay ride on the dock)
            let sr = Rect::from_min_max(egui::pos2((cog.min.x - 228.0).max(rect.min.x + 560.0), rect.min.y), egui::pos2(cog.min.x - 8.0, rect.max.y));
            if sr.width() > 120.0 {
                let used = ui
                    .allocate_new_ui(egui::UiBuilder::new().max_rect(sr).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
                        kit::settings_style(ui);
                        let w = sr.width().min(220.0);
                        let r = ui.allocate_ui(egui::vec2(w, 34.0), |ui| self.nav_search_bar(ui, w < 160.0)).response.rect;
                        super::app_tour::mark(ui.ctx(), "nav.search", r);
                        ui.min_rect()
                    })
                    .inner;
                drag_until = used.min.x - 6.0;
            }
        } else if !self.takeover_open() && controls.width() > 160.0 {
            let used = ui
                .allocate_new_ui(egui::UiBuilder::new().max_rect(controls).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
                    kit::settings_style(ui);
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let start = self.overlay_button(ui, egui::vec2(146.0, 34.0));
                    super::app_tour::mark(ui.ctx(), "top.start", start);
                    if controls.width() > 420.0 {
                        let (dot, text) = if self.overlay_on && self.ov_hidden {
                            (theme::HP_YELLOW, "Minimised, still counting")
                        } else if self.overlay_on {
                            (theme::GREEN, "Overlay running")
                        } else if self.setup.gotchi.shown {
                            (theme::GREEN, "Pokégotchi out")
                        } else if self.setup.team_ov.shown {
                            (theme::GREEN, "Team checklist out")
                        } else {
                            (MUTED(), "Overlay stopped")
                        };
                        let txt = format!("{} · {} {}", self.target_label(), theme::thousands(self.cfg.count), self.count_word());
                        // Two lines, right-aligned, drawn directly.
                        let f1 = crate::fonts::sans_font(12.5);
                        let f2 = crate::fonts::sans_font(12.0);
                        let g1 = ui.painter().layout_no_wrap(text.to_string(), f1, TEXT());
                        let g2 = ui.painter().layout_no_wrap(txt, f2, MUTED());
                        let w = g1.size().x.max(g2.size().x) + 16.0;
                        let (r, _) = ui.allocate_exact_size(egui::vec2(w, 36.0), Sense::hover());
                        let p = ui.painter();
                        let x = r.max.x;
                        p.galley(egui::pos2(x - g1.size().x, r.min.y), g1.clone(), TEXT());
                        p.circle_filled(egui::pos2(x - g1.size().x - 9.0, r.min.y + g1.size().y / 2.0), 4.0, dot);
                        p.galley(egui::pos2(x - g2.size().x, r.min.y + 18.0), g2, MUTED());
                    }
                    // your party (when there's room for it)
                    if controls.width() > 640.0 {
                        self.party_strip(ui);
                    }
                    ui.min_rect()
                })
                .inner;
            drag_until = used.min.x - 6.0;
        }

        // Everything else drags the window (double-click maximises).
        let drag = Rect::from_min_max(rect.min, egui::pos2(drag_until.max(rect.min.x + 10.0), rect.max.y));
        let dr = ui.interact(drag, ui.id().with("mhdr-drag"), Sense::click_and_drag());
        if dr.double_clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed));
        } else if dr.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        // Modern Plus: the back button, then the pages as tabs (taken after
        // the drag area, so they click)
        if tabs {
            let bx = self.nav_back_button(ui, &p, x, cy);
            self.plus_title_tabs(ui, &p, bx, cy, drag_until - 8.0);
        }
        if !self.takeover_open() && !tabs {
            let tip = if self.nav_is_compact() { "Show the sidebar's names" } else { "Fold the sidebar to icons" };
            if ui.interact(fold_r, ui.id().with("mhdr-fold"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked() {
                self.nav_fold_toggle();
            }
        }
        // Pinned apps, as a dock after the page name (when they live up here).
        if !self.takeover_open() {
            self.top_dock(ui, crumb_end + 22.0, cy, drag_until - 6.0);
        }
    }

    /// The title bar's back button (a round ‹ before the tabs). Greyed out
    /// with nowhere to go back to. Returns where the tabs start.
    fn nav_back_button(&mut self, ui: &mut egui::Ui, p: &egui::Painter, x: f32, cy: f32) -> f32 {
        let (hf, hi, _) = header_colours();
        let r = Rect::from_center_size(egui::pos2(x + 15.0, cy), egui::vec2(30.0, 30.0));
        let can = self.can_go_back();
        let resp = ui.interact(r, ui.id().with("nav-back"), if can { Sense::click() } else { Sense::hover() });
        let hov = can && resp.hovered();
        if hov {
            p.circle_filled(r.center(), 15.0, theme::blend(hf, hi, 0.12));
        }
        let ink = if !can { theme::blend(hf, hi, 0.30) } else if hov { hi } else { theme::blend(hf, hi, 0.78) };
        let c = r.center() + egui::vec2(-1.0, 0.0);
        p.add(egui::Shape::line(vec![c + egui::vec2(3.5, -6.0), c + egui::vec2(-2.5, 0.0), c + egui::vec2(3.5, 6.0)], Stroke::new(2.0f32, ink)));
        super::app_tour::mark(ui.ctx(), "nav.back", r);
        let tip = match self.back_label() {
            Some(name) => format!("Back to {name} (Alt+←, or your mouse's back button)"),
            None => "Nothing to go back to yet".to_string(),
        };
        let resp = if can { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
        if resp.on_hover_text(tip).clicked() {
            self.nav_go_back();
        }
        r.max.x + 8.0
    }

    /// Keep the page history: a page change that isn't a back / forward
    /// step adds the page you left (once in a row, the last 40).
    pub(super) fn nav_track(&mut self) {
        let now = self.setup.tab;
        if now == self.setup.nav_last {
            return;
        }
        let last = self.setup.nav_last;
        if self.setup.nav_back.last() != Some(&last) {
            self.setup.nav_back.push(last);
        }
        if self.setup.nav_back.len() > 40 {
            self.setup.nav_back.remove(0);
        }
        self.setup.nav_fwd.clear();
        self.setup.nav_last = now;
    }

    /// (a page change of any kind, back / forward too: the theme's page sound)
    pub(super) fn nav_sound(&mut self, ctx: &egui::Context) {
        let id = egui::Id::new("nav-sound-tab");
        let now = self.setup.tab;
        if ctx.data(|d| d.get_temp::<Tab>(id)) != Some(now) {
            ctx.data_mut(|d| d.insert_temp(id, now));
            crate::theme::page_turned(ctx);
        }
    }

    /// Something inside the page to step back out of first: an open
    /// Pokédex entry, an app's page in the App Store, or a Game Info page
    /// you followed a link to.
    fn inner_back(&self) -> bool {
        (self.setup.tab == Tab::Pokedex && self.setup.pokedex.open.is_some())
            || (self.setup.tab == Tab::Store && self.setup.store.app.is_some())
            || (self.setup.tab == Tab::Info && !self.setup.info.history.is_empty())
    }

    pub(super) fn can_go_back(&self) -> bool {
        self.inner_back() || self.setup.nav_back.iter().any(|t| *t != self.setup.tab && self.page_still_there(*t))
    }

    /// Where back would take you, for the button's tip.
    fn back_label(&self) -> Option<String> {
        if self.inner_back() {
            return Some(
                match self.setup.tab {
                    Tab::Pokedex => "the Pokédex",
                    Tab::Store => "the App Store",
                    _ => "the last Game Info page",
                }
                .to_string(),
            );
        }
        let t = self.setup.nav_back.iter().rev().find(|t| **t != self.setup.tab && self.page_still_there(**t))?;
        Some(tab_meta(*t).1.to_string())
    }

    /// Can this page still be opened? (Its app may have been uninstalled
    /// since.)
    fn page_still_there(&self, t: Tab) -> bool {
        app_of(t).is_none_or(|a| self.cfg.apps.is_installed(a))
    }

    /// Open a page from the history (opening its app again if it was closed).
    fn nav_open(&mut self, t: Tab) {
        if let Some(a) = app_of(t) {
            if !self.cfg.apps.is_open(a) {
                self.open_app(a);
            }
        }
        self.go_to(t);
        // (a step through the history, not a new visit: nothing to add)
        self.setup.nav_last = self.setup.tab;
    }

    /// Back: out of an open Pokédex entry or Game Info page first, then to
    /// the page before.
    pub(super) fn nav_go_back(&mut self) {
        if self.setup.tab == Tab::Pokedex && self.setup.pokedex.open.is_some() {
            self.setup.pokedex.open = None;
            return;
        }
        if self.setup.tab == Tab::Store && self.setup.store.app.is_some() {
            self.setup.store.app = None;
            self.setup.scroll_top = true;
            return;
        }
        if self.setup.tab == Tab::Info && !self.setup.info.history.is_empty() {
            self.info_back();
            return;
        }
        let here = self.setup.tab;
        while let Some(t) = self.setup.nav_back.pop() {
            if t == here || !self.page_still_there(t) {
                continue;
            }
            self.setup.nav_fwd.push(here);
            self.nav_open(t);
            return;
        }
    }

    /// Forward: the page you just stepped back from.
    pub(super) fn nav_go_forward(&mut self) {
        let here = self.setup.tab;
        while let Some(t) = self.setup.nav_fwd.pop() {
            if t == here || !self.page_still_there(t) {
                continue;
            }
            self.setup.nav_back.push(here);
            self.nav_open(t);
            return;
        }
    }

    /// Alt+← / Alt+→ and the mouse's back / forward buttons. (An open
    /// Pokédex entry and Game Info handle their own keys, so those are left
    /// to them.)
    pub(super) fn nav_keys(&mut self, ctx: &egui::Context) {
        let typing = ctx.wants_keyboard_input();
        let (alt_back, mouse_back, alt_fwd, mouse_fwd) = ctx.input(|i| {
            (
                i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft),
                i.pointer.button_pressed(egui::PointerButton::Extra1),
                i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight),
                i.pointer.button_pressed(egui::PointerButton::Extra2),
            )
        });
        let entry_open = self.setup.tab == Tab::Pokedex && self.setup.pokedex.open.is_some();
        let info_hist = self.setup.tab == Tab::Info && !self.setup.info.history.is_empty();
        if entry_open {
            // (the entry closes itself on the mouse's back button and Esc)
            return;
        }
        let back = (alt_back && !typing && !info_hist) || mouse_back;
        if back {
            self.nav_go_back();
        } else if (alt_fwd && !typing) || mouse_fwd {
            self.nav_go_forward();
        }
    }

    /// Modern Plus's title bar tabs: Dashboard, App Store, Library, Profile
    /// and Stats from `x`, and the app you're in after them. Stops before
    /// `max_x` (tabs that don't fit are left out; the Ctrl K search still
    /// reaches them).
    fn plus_title_tabs(&mut self, ui: &mut egui::Ui, p: &egui::Painter, x: f32, cy: f32, max_x: f32) {
        let cur = self.setup.tab;
        let mut x = x;
        let mut go: Option<Tab> = None;
        let (hf, hi, hm) = header_colours();
        // the open tab's line: the theme's second colour on a painted title bar
        let under = match (theme::chrome(), theme::theme_now()) {
            (Some(_), Some(t)) => t.accent2,
            _ => kit::primary(),
        };
        // (all the tabs together, for the welcome tour)
        let mut all: Option<Rect> = None;
        for (t, key) in [(Tab::Home, "nav.home"), (Tab::Store, "nav.store"), (Tab::Library, "nav.library"), (Tab::Profile, "nav.profile"), (Tab::Stats, "nav.stats")] {
            let (_, name, _) = tab_meta(t);
            let on = cur == t;
            // (a theme's own type: handwriting, Impact, console, pixels)
            let font = theme::label_font(13.5).unwrap_or_else(|| if on { crate::fonts::sans_bold_font(13.5) } else { crate::fonts::sans_font(13.5) });
            let g = p.layout_no_wrap(name.to_string(), font, hi);
            let w = g.size().x + 24.0;
            if x + w > max_x {
                break;
            }
            let r = Rect::from_min_size(egui::pos2(x, cy - 16.0), egui::vec2(w, 32.0));
            let resp = ui.interact(r, ui.id().with(("plus-tab", t as u8)), Sense::click());
            let hov = resp.hovered();
            if hov && !on {
                p.rect_filled(r, 8.0, theme::blend(hf, hi, 0.08));
            }
            let ink = if on || hov { hi } else { hm };
            p.galley(egui::pos2(r.center().x - g.size().x / 2.0, cy - g.size().y / 2.0), g, ink);
            if on {
                p.rect_filled(Rect::from_min_size(egui::pos2(r.min.x + 10.0, r.max.y - 1.0), egui::vec2(w - 20.0, 3.0)), 1.5, under);
            }
            super::app_tour::mark(ui.ctx(), key, r);
            all = Some(all.map_or(r, |a| a.union(r)));
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                go = Some(t);
            }
            x = r.max.x + 2.0;
        }
        if let Some(a) = all {
            super::app_tour::mark(ui.ctx(), "nav.tabs", a);
        }
        // the app you're in, after the tabs
        if !matches!(cur, Tab::Home | Tab::Store | Tab::Library | Tab::Profile | Tab::Stats) {
            let (_, name, color) = tab_meta(cur);
            let x0 = x + 10.0;
            if x0 + 60.0 < max_x {
                p.line_segment([egui::pos2(x0, cy - 9.0), egui::pos2(x0, cy + 9.0)], Stroke::new(1.0f32, theme::blend(hf, hi, 0.25)));
                p.circle_filled(egui::pos2(x0 + 16.0, cy), 4.0, color);
                let clip = p.with_clip_rect(Rect::from_min_max(egui::pos2(x0, cy - 16.0), egui::pos2(max_x, cy + 16.0)));
                clip.text(egui::pos2(x0 + 28.0, cy), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(13.5), hi);
            }
        }
        if let Some(t) = go {
            self.go_to(t);
        }
    }

    /// The sidebar.
    pub(super) fn modern_sidebar(&mut self, ui: &mut egui::Ui) {
        let compact = self.nav_is_compact();
        let current = self.setup.tab;
        let parent_of = nav_parent;
        let rows = self.nav_rows();
        let mut go: Option<Tab> = None;
        // Room for the trainer card at the bottom.
        let footer_h = if compact { 96.0 } else { 108.0 };
        // The search bar (Ctrl K), always at the top.
        ui.add_space(6.0);
        let search_top = ui.cursor().min.y;
        self.nav_search_bar(ui, compact);
        super::app_tour::mark_since(ui, "nav.search", search_top);
        let list_h = (ui.available_height() - footer_h).max(80.0);
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::floating();
        egui::ScrollArea::vertical().id_salt("modern-nav").max_height(list_h).auto_shrink([false, true]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
            ui.add_space(6.0);
            // One pill for the open page, sliding between rows as you go.
            let pill_slot = ui.painter().add(egui::Shape::Noop);
            let nav_anchor = ui.cursor().min;
            let mut open_rect: Option<egui::Rect> = None;
            let mut pins_done = false;
            for NavRow { group, tab, parent, app } in rows {
                if !pins_done && group == Some("OPEN APPS") {
                    pins_done = true;
                    self.nav_pins(ui, compact);
                }
                if let Some(g) = group {
                    if compact {
                        ui.add_space(8.0);
                        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 9.0), Sense::hover());
                        ui.painter().line_segment([r.left_center() + egui::vec2(14.0, 0.0), r.right_center() - egui::vec2(14.0, 0.0)], Stroke::new(1.0f32, kit::LINE()));
                    } else {
                        kit::nav_group(ui, g);
                    }
                }
                if let Some(p) = parent {
                    if self.setup.nav_collapsed.contains(&p) || compact {
                        continue;
                    }
                }
                let (icon, label, color) = tab_meta(tab);
                let has_children = tab == Tab::Shiny;
                let folded = self.setup.nav_collapsed.contains(&tab);
                let selected = current == tab || ((folded || compact) && parent_of(current) == Some(tab));
                let resp = if parent.is_some() {
                    ui.horizontal(|ui| {
                        // A guide line ties the app's pages together.
                        let x = ui.cursor().min.x + 22.0;
                        let y = ui.cursor().min.y;
                        ui.painter().line_segment([egui::pos2(x, y - 2.0), egui::pos2(x, y + 40.0)], Stroke::new(1.0f32, kit::LINE()));
                        ui.add_space(30.0);
                        ui.vertical(|ui| sub_item(ui, icon, label, current == tab, color)).inner
                    })
                    .inner
                } else {
                    modern::nav_item(ui, icon, label, selected, color, compact, false)
                };
                if selected || (parent.is_some() && current == tab) {
                    open_rect = Some(resp.rect);
                }
                // the Companion's tour points at these
                if parent.is_none() && app.is_none() {
                    let key = match tab {
                        Tab::Home => Some("nav.home"),
                        Tab::Store => Some("nav.store"),
                        Tab::Library => Some("nav.library"),
                        Tab::Profile => Some("nav.profile"),
                        Tab::Stats => Some("nav.stats"),
                        _ => None,
                    };
                    if let Some(k) = key {
                        super::app_tour::mark(ui.ctx(), k, resp.rect);
                    }
                }
                // An open app's ✕ (on hover, or while it's the page you're on)
                // takes the chevron's and the warning's place.
                let show_x = app.is_some() && !compact && (resp.hovered() || ui.rect_contains_pointer(resp.rect));
                if let Some(a) = app {
                    if self.nav_close_button(ui, resp.rect, &resp, a, show_x) {
                        ui.add_space(2.0);
                        continue;
                    }
                }
                // Chevron on an app with pages; a dot while the overlay runs.
                if !compact && has_children && !show_x {
                    let c = egui::pos2(resp.rect.max.x - 14.0, resp.rect.center().y);
                    let st = Stroke::new(1.5f32, MUTED());
                    let pts = if folded {
                        vec![c + egui::vec2(-1.5, -3.5), c + egui::vec2(2.0, 0.0), c + egui::vec2(-1.5, 3.5)]
                    } else {
                        vec![c + egui::vec2(-3.5, -1.5), c + egui::vec2(0.0, 2.0), c + egui::vec2(3.5, -1.5)]
                    };
                    ui.painter().add(egui::Shape::line(pts, st));
                }
                if is_experimental(tab) && !show_x {
                    let mark = if compact {
                        let c = resp.rect.center() + egui::vec2(12.0, -11.0);
                        kit::warn_triangle(ui.painter(), c, 11.0, Some(SIDEBAR()))
                    } else {
                        kit::exp_badge(ui.painter(), egui::pos2(resp.rect.max.x - 10.0, resp.rect.center().y))
                    };
                    ui.interact(mark, egui::Id::new(("nav-exp", label)), Sense::hover()).on_hover_text(kit::EXPERIMENTAL_TIP);
                }
                if tab == Tab::Shiny && self.overlay_on {
                    let chip = if compact { resp.rect.center() + egui::vec2(12.0, -12.0) } else { resp.rect.left_center() + egui::vec2(34.0, -12.0) };
                    ui.painter().circle_filled(chip, 5.0, SIDEBAR());
                    ui.painter().circle_filled(chip, 3.5, theme::GREEN);
                }
                if resp.clicked() {
                    go = Some(tab);
                }
                ui.add_space(2.0);
            }
            // no open apps: the pins and the "nothing open" note still show
            if !pins_done {
                self.nav_pins(ui, compact);
            }
            self.nav_none_open(ui, compact);
            if let Some(r) = open_rect {
                let g = kit::glide_rect(ui.ctx(), egui::Id::new("nav-pill"), r, nav_anchor, kit::BASE);
                ui.painter().set(pill_slot, egui::Shape::rect_filled(g, g.height() / 2.0, modern::nav_selected_fill()));
            }
            ui.add_space(6.0);
        });
        if let Some(t) = go {
            self.go_to(t);
        }

        // Footer: trainer card, fold button, saved state.
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(4.0);
            let recent = self.saved_flash.is_some_and(|t| t.elapsed() < Duration::from_millis(1500));
            ui.horizontal(|ui| {
                // a framed button with the panel picture (the header has one too)
                let (fr, fold) = ui.allocate_exact_size(egui::vec2(32.0, 28.0), Sense::click());
                let hov = fold.hovered();
                ui.painter().rect_filled(fr, 7.0, theme::blend(SIDEBAR(), TEXT(), if hov { 0.10 } else { 0.05 }));
                ui.painter().rect_stroke(fr, 7.0, Stroke::new(1.0f32, theme::blend(SIDEBAR(), TEXT(), 0.12)));
                panel_icon(ui.painter(), fr.center(), compact, if hov { TEXT() } else { theme::blend(SIDEBAR(), TEXT(), 0.75) });
                if fold.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(if compact { "Show the page names" } else { "Fold the sidebar to icons" }).clicked() {
                    self.nav_fold_toggle();
                }
                if !compact {
                    if recent {
                        ui.label(RichText::new("✔ Saved").size(12.0).color(theme::GREEN));
                        ui.ctx().request_repaint_after(Duration::from_millis(200));
                    } else {
                        ui.label(RichText::new("Changes save automatically").size(12.0).color(theme::blend(SIDEBAR(), MUTED(), 0.8)));
                    }
                }
            });
            ui.add_space(6.0);
            let w = ui.available_width();
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 58.0), Sense::click());
            let p = ui.painter();
            let hov = resp.hovered();
            p.rect_filled(rect, 12.0, theme::blend(SIDEBAR(), TEXT(), if hov { 0.07 } else { 0.04 }));
            p.rect_stroke(rect, 12.0, Stroke::new(1.0f32, theme::blend(SIDEBAR(), TEXT(), 0.08)));
            let level = self.cfg.level_seen.max(1);
            let bc = if compact { rect.center() } else { rect.left_center() + egui::vec2(28.0, 0.0) };
            let (_, main, _) = crate::progress::badge_metal(level);
            let ctx = ui.ctx().clone();
            let p = &ui.painter().clone();
            self.draw_avatar(&ctx, p, bc, 17.0, level, egui::Color32::from_rgb(main[0], main[1], main[2]));
            if !compact {
                let name = if self.cfg.trainer_name.trim().is_empty() { "Trainer".to_string() } else { self.cfg.trainer_name.clone() };
                let clip = Rect::from_min_max(egui::pos2(bc.x + 26.0, rect.min.y), egui::pos2(rect.max.x - 8.0, rect.max.y));
                let pc = p.with_clip_rect(clip);
                let plate = self.cfg.plate.clone();
                if plate.is_empty() {
                    pc.text(egui::pos2(bc.x + 28.0, rect.center().y - 8.0), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(13.5), TEXT());
                } else {
                    // the battle pass name plate, with the name on it
                    let pc = p.with_clip_rect(clip.with_min_x(bc.x + 19.0).expand2(egui::vec2(0.0, 6.0)));
                    let size = 11.5;
                    let f = crate::fonts::sans_bold_font(size);
                    let nw = pc.layout_no_wrap(name.clone(), f, TEXT()).size().x.min(clip.width() - 30.0);
                    let ctx = ui.ctx().clone();
                    let r = App::plate_rect(bc.x + 31.0, rect.center().y - 8.0, nw, 22.0);
                    self.draw_plate_ui(&ctx, &pc, &plate, r, &name, size);
                }
                // a battle pass title in its look's colour
                let title = self.player_title(level);
                let lr = pc.text(egui::pos2(bc.x + 28.0, rect.center().y + 10.0), Align2::LEFT_CENTER, format!("Level {level} · "), crate::fonts::sans_font(12.0), MUTED());
                pc.text(egui::pos2(lr.max.x, rect.center().y + 10.0), Align2::LEFT_CENTER, &title, crate::fonts::sans_font(12.0), super::pass_page::title_col(&title, MUTED()));
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Your Trainer Profile").clicked() {
                self.go_to(Tab::Profile);
            }
        });
    }
}

/// A page under an app in the sidebar: smaller, with a coloured dot.
fn sub_item(ui: &mut egui::Ui, icon: &str, label: &str, selected: bool, color: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), Sense::click());
    let p = ui.painter();
    let hovered = resp.hovered();
    // (The open page's pill is painted by the sidebar, sliding between rows.)
    let ht = kit::anim(ui.ctx(), resp.id.with("hover"), hovered && !selected, kit::QUICK);
    if ht > 0.0 {
        p.rect_filled(rect, rect.height() / 2.0, theme::blend(SIDEBAR(), TEXT(), 0.04).gamma_multiply(ht));
    }
    p.text(rect.left_center() + egui::vec2(14.0, 0.0), Align2::CENTER_CENTER, icon, egui::FontId::proportional(13.0), if selected { color } else { MUTED() });
    p.text(
        rect.left_center() + egui::vec2(30.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        if selected { crate::fonts::sans_bold_font(13.0) } else { crate::fonts::sans_font(13.0) },
        if selected || hovered { TEXT() } else { theme::blend(SIDEBAR(), TEXT(), 0.7) },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A sidebar-panel picture: a window with its left column (filled when the
/// sidebar's open, an arrow showing which way it goes).
pub(super) fn panel_icon(p: &egui::Painter, c: egui::Pos2, folded: bool, col: Color32) {
    let r = Rect::from_center_size(c, egui::vec2(17.0, 14.0));
    let st = Stroke::new(1.4f32, col);
    p.rect_stroke(r, 3.0, st);
    let x = r.min.x + 6.0;
    p.line_segment([egui::pos2(x, r.min.y), egui::pos2(x, r.max.y)], st);
    if !folded {
        p.rect_filled(Rect::from_min_max(r.min + egui::vec2(1.5, 1.5), egui::pos2(x - 0.5, r.max.y - 1.5)), 1.5, col.gamma_multiply(0.45));
    }
    // a small chevron in the page part: « to fold, » to open
    let cx = r.max.x - 4.5;
    let d = if folded { 1.0 } else { -1.0 };
    p.line_segment([egui::pos2(cx - d * 1.5, c.y - 2.5), egui::pos2(cx + d * 1.5, c.y)], st);
    p.line_segment([egui::pos2(cx + d * 1.5, c.y), egui::pos2(cx - d * 1.5, c.y + 2.5)], st);
}

/// A cog: a ring with eight teeth and a hole.
fn cog_icon(p: &egui::Painter, c: egui::Pos2, r: f32, col: Color32) {
    let teeth = 8;
    for k in 0..teeth {
        let a = std::f32::consts::TAU * k as f32 / teeth as f32;
        let d = egui::vec2(a.cos(), a.sin());
        p.line_segment([c + d * (r * 0.62), c + d * (r * 1.08)], Stroke::new(r * 0.42, col));
    }
    p.circle_stroke(c, r * 0.62, Stroke::new(r * 0.36, col));
}

/// The title bar's colours: fill, ink and quiet ink. A Modern Plus theme can
/// paint it (Pokédex's red) or use its darker chrome; otherwise it's the
/// page's own.
pub(super) fn header_colours() -> (Color32, Color32, Color32) {
    if kit::plus() {
        if let Some((fill, ink)) = theme::chrome() {
            return (fill, ink, theme::blend(fill, ink, 0.90));
        }
        if theme::theme_now().is_some() {
            return (SIDEBAR(), TEXT(), MUTED());
        }
    }
    (PAGE(), TEXT(), MUTED())
}
