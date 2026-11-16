//! The settings window: every setting in one place. A list on the left (the
//! Companion's own settings, then every installed app) and the section on
//! the right. The ⚙ at the top opens it at the app you're on; an app's own
//! ⚙ opens it at that app too.

use super::shell::{app_of, app_tab, tab_meta};
use super::ui_kit::{self as kit, LINE, MUTED, PAGE, SIDEBAR, TEXT};
use super::{App, Tab};
use crate::apps::{self, AppId};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

/// The Companion's own settings, at the top of the list.
const GENERAL: [Tab; 3] = [Tab::Appearance, Tab::Hotkeys, Tab::Advanced];

/// The section a page's ⚙ opens: its app's (Shiny Hunter's Counting and
/// Overlay are sections of their own), else Appearance.
pub(super) fn section_for(t: Tab) -> Tab {
    match t {
        Tab::Appearance | Tab::Hotkeys | Tab::Advanced | Tab::Counting | Tab::Overlay => t,
        _ => app_of(t).map(app_tab).unwrap_or(Tab::Appearance),
    }
}

/// The entry in the list a section is under.
fn rail_of(t: Tab) -> Tab {
    match t {
        Tab::Counting | Tab::Overlay => Tab::Shiny,
        _ => t,
    }
}

/// Words that find a section in the search box (as well as its name).
fn keywords(t: Tab) -> &'static str {
    match t {
        Tab::Appearance => "theme dark light colour color accent font text size modern plus look dock sidebar pinned tours",
        Tab::Hotkeys => "keys keyboard shortcuts hotkey",
        Tab::Advanced => "updates backup data game folder world counter reading help",
        Tab::Shiny => "overlay counting count screen log game break session goal timer idle pause autostart sounds sections theme",
        Tab::Cards => "card art download playmat xp bar overlay packs",
        Tab::Gotchi => "pet name frame pearl overlay perch strip",
        Tab::Replay => "recording clips quality frame rate fps folder video",
        Tab::Journal => "notes text size markdown daily folder",
        Tab::Stickers => "paper sticker size page",
        Tab::Info => "world modern more world.ini",
        _ => "",
    }
}

/// The line under an app's section title.
fn blurb(a: AppId) -> &'static str {
    match a {
        AppId::Shiny => "The overlay window, sessions and breaks. How encounters are counted, and the overlay's look, have their own tabs above.",
        AppId::Cards => "Card art on this PC, the XP bar on the overlay, and the playmat you open packs on.",
        AppId::Gotchi => "Its name, the device's frame, and how it shows on the overlay.",
        AppId::Replay => "How clips are recorded, and where they go.",
        AppId::Journal => "How your notes look and where new ones go.",
        AppId::Stickers => "How your pages look.",
        AppId::Info => "Game Info follows your world's options (from world.ini in your game folder).",
        _ => "Where it lives, and its tour.",
    }
}

impl App {
    /// Close the settings window.
    pub(super) fn close_settings(&mut self) {
        self.setup.settings_open = None;
        self.setup.gotchi.renaming = None;
        // the Card Collector's first-time question: asked, once Settings showed it
        if self.setup.cards.settings && !self.setup.cards.file().art_asked {
            self.setup.cards.file().art_asked = true;
            self.setup.cards.touch();
        }
    }

    /// The settings window, over the page (Esc, ✕ or a click outside closes it).
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        let Some(opened) = self.setup.settings_open else { return };
        let mut tab = opened;
        // (an app that was uninstalled since: the first section instead)
        if app_of(tab).is_some_and(|a| !self.cfg.apps.is_installed(a)) {
            tab = Tab::Appearance;
        }
        let screen = ctx.screen_rect();
        let size = vec2((screen.width() - 64.0).clamp(360.0, 1240.0), (screen.height() - 72.0).clamp(280.0, 900.0));
        let panel = Rect::from_center_size(screen.center(), size);
        // narrow windows: the list shrinks to icons
        let compact = size.x < 780.0;
        let rail_w = if compact { 66.0 } else { 240.0 };
        let mut close = false;
        // (Esc stays with the hotkey being recorded)
        if self.setup.recording.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        let sid = egui::Id::new("settings-search");
        let mut query: String = ctx.data(|d| d.get_temp::<String>(sid)).unwrap_or_default();
        let mut pick: Option<Tab> = None;
        let apps: Vec<AppId> = apps::ALL.into_iter().filter(|a| self.cfg.apps.is_installed(*a)).collect();
        egui::Area::new(egui::Id::new("settings-window")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(150));
            if scrim.clicked() {
                close = true;
            }
            let round = 20.0;
            let p = ui.painter().clone();
            p.add(kit::shadow_tier(3).as_shape(panel, round));
            p.rect_filled(panel, round, PAGE());
            // the window eats its own clicks
            ui.interact(panel, egui::Id::new("settings-window-panel"), Sense::click());
            // ── the list
            let rail = Rect::from_min_size(panel.min, vec2(rail_w, panel.height()));
            p.rect_filled(rail, egui::Rounding { nw: round, sw: round, ne: 0.0, se: 0.0 }, SIDEBAR());
            p.line_segment([pos2(rail.max.x, rail.min.y), pos2(rail.max.x, rail.max.y)], Stroke::new(1.0f32, LINE()));
            p.rect_stroke(panel, round, Stroke::new(1.0f32, kit::CONTROL()));
            let rail_in = rail.shrink2(vec2(if compact { 10.0 } else { 14.0 }, 18.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rail_in).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                settings_rail(ui, rail_of(tab), &apps, &mut query, compact, &mut pick);
            });
            // ── the section
            let body = Rect::from_min_max(pos2(rail.max.x + 30.0, panel.min.y + 22.0), pos2(panel.max.x - 22.0, panel.max.y - 14.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                // Shiny Hunter: its settings in three tabs
                if rail_of(tab) == Tab::Shiny {
                    let before = tab;
                    ui.allocate_ui(vec2((ui.available_width() - 52.0).max(200.0), 40.0), |ui| {
                        kit::tab_bar(ui, &mut tab, &[(Tab::Shiny, "⚙  General"), (Tab::Counting, "👁  Counting"), (Tab::Overlay, "🖵  Overlay")]);
                    });
                    if tab != before {
                        self.setup.scroll_top = true;
                    }
                    ui.add_space(12.0);
                }
                let mut area = egui::ScrollArea::vertical().id_salt(("settings-window-scroll", tab as u8)).auto_shrink([false, false]);
                if std::mem::take(&mut self.setup.scroll_top) {
                    area = area.vertical_scroll_offset(0.0);
                }
                // Counting and Overlay have tours that play in here: a step
                // further down scrolls this window (as the page area does)
                let toured = matches!(tab, Tab::Counting | Tab::Overlay);
                let tour_to_id = egui::Id::new("tour-scroll-to");
                let mut glide: Option<(f32, f32)> = None;
                if toured {
                    if let Some((to, _)) = ui.data(|d| d.get_temp::<(f32, u64)>(tour_to_id)) {
                        let cur: f32 = ui.data(|d| d.get_temp(egui::Id::new("page-scroll-y"))).unwrap_or(0.0);
                        let step = if (to - cur).abs() < 1.5 || !kit::motion() { to } else { cur + (to - cur) * 0.3 };
                        area = area.vertical_scroll_offset(step);
                        glide = Some((to, step));
                    }
                }
                let out = area.show(ui, |ui| {
                    ui.set_max_width(ui.available_width() - 14.0);
                    // the headings show the section's icon (not the page under the window)
                    let (icon, _, color) = tab_meta(rail_of(tab));
                    kit::set_page_icon(kit::modern().then_some((icon, color)));
                    // (room for the ✕ beside the heading)
                    if rail_of(tab) != Tab::Shiny {
                        ui.add_space(2.0);
                    }
                    self.settings_section(ui, tab);
                    ui.add_space(24.0);
                    let (icon, _, color) = tab_meta(self.setup.tab);
                    kit::set_page_icon(kit::modern().then_some((icon, color)));
                });
                if toured {
                    // what's actually in view (under Shiny Hunter's tabs): the
                    // tour fits its spotlight and its scrolling to this
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("tour-page-settings"), out.inner_rect));
                    let y = out.state.offset.y;
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("page-scroll-y"), y));
                    if let Some((to, asked)) = glide {
                        if (y - to).abs() < 1.5 || (y - asked).abs() > 1.0 {
                            ui.data_mut(|d| d.remove::<(f32, u64)>(tour_to_id));
                        } else {
                            ui.ctx().request_repaint();
                        }
                    }
                }
            });
        });
        // ── ✕: in a layer of its own above the window, so nothing in a
        // section (or its scroll bar) can ever cover it or take its click
        egui::Area::new(egui::Id::new("settings-window-x")).order(egui::Order::Middle).constrain(false).fixed_pos(panel.right_top() + vec2(-56.0, 12.0)).show(ctx, |ui| {
            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            let xr = Rect::from_center_size(r.center(), vec2(34.0, 34.0));
            // a ring of the window's colour, so what scrolls under it doesn't show
            ui.painter().circle_filled(xr.center(), 21.0, PAGE());
            if super::store::pill_button(ui, xr, "×", false, "settings-x").on_hover_text("Close (Esc)").clicked() {
                close = true;
            }
        });
        // (it comes after the window in the same layer order: keep it on top)
        ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, egui::Id::new("settings-window-x")));
        ctx.data_mut(|d| d.insert_temp(sid, query));
        if let Some(t) = pick {
            if t != rail_of(tab) {
                tab = t;
                self.setup.scroll_top = true;
            }
        }
        if close {
            self.close_settings();
            ctx.data_mut(|d| d.remove::<String>(sid));
        } else if self.setup.settings_open == Some(opened) {
            self.setup.settings_open = Some(tab);
        }
        // (else a setting moved it this frame, like Shiny Replay's "Hotkeys": that stands)
    }

    /// One section's settings.
    fn settings_section(&mut self, ui: &mut egui::Ui, tab: Tab) {
        match tab {
            Tab::Appearance => self.page_appearance(ui),
            Tab::Hotkeys => self.page_hotkeys(ui),
            Tab::Advanced => self.page_advanced(ui),
            Tab::Counting => self.page_counting(ui),
            Tab::Overlay => self.page_overlay(ui),
            t => match app_of(t) {
                Some(a) => self.app_settings_section(ui, a),
                None => self.page_appearance(ui),
            },
        }
    }

    /// An app's section: its own settings, then where it lives and its tour.
    fn app_settings_section(&mut self, ui: &mut egui::Ui, a: AppId) {
        let (_, name, _) = tab_meta(app_tab(a));
        kit::page_header(ui, &name.to_uppercase(), blurb(a));
        let mut own = true;
        match a {
            AppId::Shiny => {
                // Counting and Overlay: here only, or pages of the app too
                kit::section(ui, "Its pages");
                kit::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let mut on = self.cfg.apps.shiny_pages_in_app;
                    if kit::toggle_row(
                        ui,
                        "Counting and Overlay as Shiny Hunter pages",
                        "On: they're pages of Shiny Hunter again, under it in the sidebar and on its page, as they used to be. They stay here in Settings too.",
                        &mut on,
                    ) {
                        self.cfg.apps.shiny_pages_in_app = on;
                        self.dirty = true;
                    }
                });
                ui.add_space(18.0);
                self.shiny_window_settings(ui);
                ui.add_space(18.0);
                kit::section(ui, "Hunting");
                self.hunt_session(ui);
            }
            AppId::Cards => self.cards_settings_body(ui),
            AppId::Gotchi => self.gotchi_settings_body(ui),
            AppId::Replay => {
                let st = self.setup.replay.status();
                kit::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.replay_settings(ui, &st);
                });
            }
            AppId::Journal => self.journal_settings_body(ui),
            AppId::Stickers => {
                kit::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.stickers_settings(ui);
                });
            }
            AppId::Info => self.world_card(ui),
            _ => own = false,
        }
        if own {
            ui.add_space(22.0);
        }
        self.app_basics(ui, a, own);
    }

    /// Every app: pin it, and see its tour again.
    fn app_basics(&mut self, ui: &mut egui::Ui, a: AppId, own: bool) {
        kit::section(ui, "This app");
        let dock = self.cfg.apps.pins_place == apps::PinsPlace::Dock;
        let place = if dock { "dock" } else { "sidebar" };
        let tours = super::app_tour::tour_ids(a);
        let mut replay = false;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            if !own {
                ui.label(RichText::new(format!("{} has no settings of its own yet: everything is on its page.", a.name())).size(kit::fs(12.5)).color(MUTED()));
                ui.add_space(10.0);
            }
            let mut pinned = self.cfg.apps.is_pinned(a);
            let max = self.cfg.apps.max_pins();
            let full = !pinned && self.cfg.apps.pin_list().len() >= max;
            let help = if full {
                format!("Your {place} is full ({max} apps). Unpin one to make room.")
            } else {
                format!("Keep {} in the {place}, even when it's closed.", a.name())
            };
            let changed = ui.add_enabled_ui(!full, |ui| kit::toggle_row(ui, &format!("Pin to the {place}"), &help, &mut pinned)).inner;
            if changed {
                if pinned {
                    self.cfg.apps.pin(a);
                } else {
                    self.cfg.apps.unpin(a);
                }
                self.dirty = true;
            }
            if !tours.is_empty() {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Its tour").size(kit::fs(13.0)).color(TEXT()));
                        ui.label(RichText::new(format!("A quick walk round {}, from the start.", a.name())).size(kit::fs(11.5)).color(MUTED()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(150.0, 32.0), Sense::hover());
                        if super::store::pill_button(ui, r, "▶  Show it again", false, ("settings-tour", a.slug())).clicked() {
                            replay = true;
                        }
                    });
                });
            }
        });
        if replay {
            self.cfg.apps.tours_seen.retain(|s| !tours.contains(&s.as_str()));
            self.cfg.apps.tours_off = false;
            self.dirty = true;
            self.close_settings();
            self.open_app(a);
        }
    }
}

/// The list on the left: a search box, the Companion's settings, then apps.
fn settings_rail(ui: &mut egui::Ui, on: Tab, apps: &[AppId], query: &mut String, compact: bool, pick: &mut Option<Tab>) {
    let mut enter = false;
    if !compact {
        ui.label(RichText::new("Settings").font(crate::fonts::sans_bold_font(20.0)).color(TEXT()));
        ui.add_space(12.0);
        let resp = kit::search_box_keyed(ui, Some(egui::Id::new("settings-search-box")), query, "Search settings", f32::INFINITY);
        enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.add_space(6.0);
    }
    let q = if compact { String::new() } else { query.trim().to_lowercase() };
    let hit = |t: Tab| {
        let (_, name, _) = tab_meta(t);
        q.is_empty() || name.to_lowercase().contains(&q) || keywords(t).split(' ').any(|k| k.starts_with(&q))
    };
    let general: Vec<Tab> = GENERAL.into_iter().filter(|t| hit(*t)).collect();
    let mine: Vec<Tab> = apps.iter().map(|a| app_tab(*a)).filter(|t| hit(*t)).collect();
    // Enter in the search box: the first one found
    if enter {
        if let Some(t) = general.first().or(mine.first()) {
            *pick = Some(*t);
        }
    }
    egui::ScrollArea::vertical().id_salt("settings-rail").auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        if !general.is_empty() {
            rail_group(ui, "COMPANION", compact);
            for t in general.iter() {
                if rail_item(ui, *t, *t == on, compact) {
                    *pick = Some(*t);
                }
            }
        }
        if !mine.is_empty() {
            rail_group(ui, "APPS", compact);
            for t in mine.iter() {
                if rail_item(ui, *t, *t == on, compact) {
                    *pick = Some(*t);
                }
            }
        }
        if general.is_empty() && mine.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new("Nothing matches.").size(kit::fs(12.5)).color(MUTED()));
        }
    });
}

/// A small heading in the list.
fn rail_group(ui: &mut egui::Ui, text: &str, compact: bool) {
    ui.add_space(14.0);
    if compact {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(r.shrink2(vec2(8.0, 0.0)), 0.0, LINE());
    } else {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new(text).font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(SIDEBAR(), MUTED(), 0.8)));
        });
    }
    ui.add_space(4.0);
}

/// One entry in the list: its icon in its colour, and its name.
fn rail_item(ui: &mut egui::Ui, t: Tab, on: bool, compact: bool) -> bool {
    let (icon, name, col) = tab_meta(t);
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
    let k = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let p = ui.painter();
    if on {
        p.rect_filled(r, 10.0, theme::blend(SIDEBAR(), col, if theme::pal().light { 0.14 } else { 0.2 }));
        // a short bar of its colour on the left
        p.rect_filled(Rect::from_center_size(pos2(r.min.x + 1.5, r.center().y), vec2(3.0, 18.0)), 2.0, col);
    } else if k > 0.0 {
        p.rect_filled(r, 10.0, theme::blend(SIDEBAR(), TEXT(), 0.07 * k));
    }
    let cx = if compact { r.center().x } else { r.min.x + 24.0 };
    let chip = Rect::from_center_size(pos2(cx, r.center().y), vec2(26.0, 26.0));
    kit::icon_chip(p, chip, icon, col, on);
    if !compact {
        let font = if on { crate::fonts::sans_bold_font(13.5) } else { crate::fonts::sans_font(13.5) };
        let c = if on { TEXT() } else { theme::blend(SIDEBAR(), TEXT(), 0.82) };
        p.text(pos2(chip.max.x + 11.0, r.center().y), Align2::LEFT_CENTER, name, font, c);
    }
    let resp = if compact { resp.on_hover_text(name) } else { resp };
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}
