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
        Tab::Library => ("⊞", "Library", LIBRARY_COL),
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
        Tab::Appearance => ("🎨", "Appearance", kit::C_PINK),
        Tab::Hotkeys => ("⌨", "Hotkeys", kit::C_SLATE),
        Tab::Advanced => ("⚙", "Advanced", kit::C_SLATE),
    }
}

/// Pages that are still experimental (marked in the sidebar).
pub(super) fn is_experimental(t: Tab) -> bool {
    matches!(t, Tab::Sim | Tab::Breed | Tab::Cards | Tab::Gotchi)
}

/// The App Store's and the Library's colours.
pub(super) const STORE_COL: Color32 = Color32::from_rgb(255, 112, 92);
pub(super) const LIBRARY_COL: Color32 = Color32::from_rgb(124, 150, 255);

/// Shiny Hunter's own pages, shown under it in the sidebar while it's open.
pub(super) const SHINY_PAGES: [Tab; 4] = [Tab::Hunt, Tab::Counting, Tab::Runs, Tab::Overlay];

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
    }
}

/// The page above a sub-page in the sidebar.
pub(super) fn nav_parent(t: Tab) -> Option<Tab> {
    SHINY_PAGES.contains(&t).then_some(Tab::Shiny)
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
    /// then the apps that are open (closing one takes it off); then settings.
    pub(super) fn nav_rows(&self) -> Vec<NavRow> {
        let row = |group, tab| NavRow { group, tab, parent: None, app: None };
        let mut v = vec![row(None, Tab::Home), row(None, Tab::Store), row(None, Tab::Library), row(None, Tab::Profile), row(None, Tab::Stats)];
        let mut first = true;
        let sidebar_pins = self.cfg.apps.pins_place == crate::apps::PinsPlace::Sidebar;
        for a in self.cfg.apps.open_list() {
            if sidebar_pins && a != AppId::Shiny && self.cfg.apps.is_pinned(a) {
                continue;
            }
            let tab = app_tab(a);
            v.push(NavRow { group: first.then_some("OPEN APPS"), tab, parent: None, app: Some(a) });
            first = false;
            if a == AppId::Shiny {
                for t in SHINY_PAGES {
                    v.push(NavRow { group: None, tab: t, parent: Some(Tab::Shiny), app: None });
                }
            }
        }
        v.push(row(Some("SETTINGS"), Tab::Appearance));
        v.push(row(None, Tab::Hotkeys));
        v.push(row(None, Tab::Advanced));
        v
    }

    /// The "nothing open" note in the sidebar (where open apps would be).
    pub(super) fn nav_none_open(&mut self, ui: &mut egui::Ui, compact: bool) {
        if compact || !self.cfg.apps.open_list().is_empty() {
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
        if self.cfg.appearance.nav_compact {
            RAIL_W
        } else {
            NAV_W
        }
    }

    /// Open a page from the sidebar (same rules as the classic sidebar).
    pub(super) fn go_to(&mut self, tab: Tab) {
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
    pub(super) fn modern_header(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let maxed = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let r = if maxed { 0.0 } else { 8.0 };
        let p = ui.painter().clone();
        // (No sidebar during the first-run welcome.)
        let nav_w = if !self.takeover_open() { self.nav_width() } else { 0.0 };
        let left = Rect::from_min_max(rect.min, egui::pos2(rect.min.x + nav_w, rect.max.y));
        let right = Rect::from_min_max(egui::pos2(left.max.x, rect.min.y), rect.max);
        p.rect_filled(left, egui::Rounding { nw: r, ..Default::default() }, SIDEBAR());
        p.rect_filled(right, egui::Rounding { ne: r, ..Default::default() }, PAGE());
        p.line_segment([right.left_bottom() + egui::vec2(0.0, -0.5), right.right_bottom() + egui::vec2(0.0, -0.5)], Stroke::new(1.0f32, kit::LINE()));
        // The brand's colours as a hairline across the very top.
        let pl = theme::pal();
        p.add(theme::gradient_rect(
            Rect::from_min_size(rect.min, egui::vec2(rect.width(), 2.0)),
            [r, r, 0.0, 0.0],
            pl.accent,
            if pl.gradient { pl.accent2 } else { pl.accent },
        ));

        // Brand.
        let compact = self.cfg.appearance.nav_compact && !self.takeover_open();
        let logo_c = if compact { left.center() } else { rect.left_center() + egui::vec2(30.0, 0.0) };
        p.image(
            self.icon_tex.id(),
            Rect::from_center_size(logo_c, egui::vec2(28.0, 28.0)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        if !compact {
            p.text(logo_c + egui::vec2(22.0, -8.0), Align2::LEFT_CENTER, "Poke Together", crate::fonts::sans_bold_font(14.5), TEXT());
            p.text(logo_c + egui::vec2(22.0, 9.0), Align2::LEFT_CENTER, "Companion", crate::fonts::sans_font(12.0), MUTED());
        }

        // Where you are: "Shiny Hunter › Hunt".
        let tab = self.setup.tab;
        let (_, name, color) = tab_meta(tab);
        let mut x = if !self.takeover_open() { right.min.x + 28.0 } else { rect.min.x + 200.0 };
        let cy = right.center().y;
        if is_shiny_tab(tab) && tab != Tab::Shiny {
            let g = p.text(egui::pos2(x, cy), Align2::LEFT_CENTER, "Shiny Hunter", crate::fonts::sans_font(14.0), MUTED());
            x = g.max.x + 10.0;
            let g = p.text(egui::pos2(x, cy), Align2::LEFT_CENTER, "›", crate::fonts::sans_font(15.0), MUTED());
            x = g.max.x + 10.0;
        }
        let mut crumb_end = x;
        if !self.takeover_open() {
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
                let fill = if i == 2 { Color32::from_rgb(220, 50, 60) } else { theme::blend(PAGE(), TEXT(), 0.08) };
                let rr = if i == 2 { egui::Rounding { ne: r, ..Default::default() } } else { egui::Rounding::ZERO };
                p.rect_filled(br, rr, fill);
            }
            let c = br.center();
            let ink = if hov && i == 2 { Color32::WHITE } else { theme::blend(PAGE(), TEXT(), 0.8) };
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

        // The overlay's status and its Start / Stop, always in reach.
        let controls = Rect::from_min_max(egui::pos2(right.min.x + 240.0, rect.min.y), egui::pos2(minb.min.x - 10.0, rect.max.y));
        let mut drag_until = controls.max.x;
        if !self.takeover_open() && controls.width() > 160.0 {
            let used = ui
                .allocate_new_ui(egui::UiBuilder::new().max_rect(controls).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
                    kit::settings_style(ui);
                    ui.spacing_mut().item_spacing.x = 10.0;
                    self.overlay_button(ui, egui::vec2(146.0, 34.0));
                    if controls.width() > 420.0 {
                        let (dot, text) = if self.overlay_on && self.ov_hidden {
                            (theme::HP_YELLOW, "Minimised, still counting")
                        } else if self.overlay_on {
                            (theme::GREEN, "Overlay running")
                        } else if self.setup.gotchi.shown {
                            (theme::GREEN, "Pokégotchi out")
                        } else {
                            (MUTED(), "Overlay stopped")
                        };
                        let txt = format!("{} · {} encounters", self.target_label(), theme::thousands(self.cfg.count));
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
        // Pinned apps, as a dock after the page name (when they live up here).
        if !self.takeover_open() {
            self.top_dock(ui, crumb_end + 22.0, cy, drag_until - 6.0);
        }
    }

    /// The sidebar.
    pub(super) fn modern_sidebar(&mut self, ui: &mut egui::Ui) {
        let compact = self.cfg.appearance.nav_compact;
        let current = self.setup.tab;
        let parent_of = nav_parent;
        let rows = self.nav_rows();
        let mut go: Option<Tab> = None;
        // Room for the trainer card at the bottom.
        let footer_h = if compact { 96.0 } else { 108.0 };
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
                if !pins_done && (group == Some("OPEN APPS") || tab == Tab::Appearance) {
                    pins_done = true;
                    self.nav_pins(ui, compact);
                }
                if tab == Tab::Appearance {
                    self.nav_none_open(ui, compact);
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
                let fold = ui
                    .add(egui::Button::new(RichText::new(if compact { "»" } else { "«" }).size(14.0).color(MUTED())).frame(false).min_size(egui::vec2(28.0, 22.0)))
                    .on_hover_text(if compact { "Show the page names" } else { "Fold the sidebar to icons" });
                if fold.clicked() {
                    self.cfg.appearance.nav_compact = !compact;
                    self.dirty = true;
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
