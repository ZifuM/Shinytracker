//! The Dashboard's widgets: under the hunt banner, cards you pick in a
//! waterfall grid (each goes into the highest open space). "Customize" lets
//! you drag them into a new order, make them 1, 2 or full width and remove
//! them; "Add widget" opens the gallery, with a live preview of each one.
//! The Dashboard starts empty, with a big "Add your first widget".
//!
//! Each widget is a `DEFS` entry plus an arm in `draw_widget`; the widgets
//! themselves live in `w_hunt` (Shiny Hunter and general), `w_explore`
//! (Pokédex, Map, Game Info) and `w_battle` (Team Planner, Battle Sim,
//! Breeding, Damage Calc).

use super::store::{app_col, app_icon, card_bg, pill_button};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::apps::{AppId, WidgetCfg};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use std::collections::HashMap;

/// Full width (wider than any grid has columns).
pub(super) const FULL: u8 = 9;
const GAP: f32 = 18.0;
const MIN_COL: f32 = 330.0;
const MAX_COLS: usize = 3;

/// A kind of widget.
pub(super) struct WidgetDef {
    pub kind: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    pub icon: &'static str,
    /// The app it belongs to (None: always there).
    pub app: Option<AppId>,
    /// Its width when added.
    pub span: u8,
    /// Can be added more than once (each keeps its own settings).
    pub multi: bool,
}

const fn wd(kind: &'static str, name: &'static str, about: &'static str, icon: &'static str, app: Option<AppId>, span: u8, multi: bool) -> WidgetDef {
    WidgetDef { kind, name, about, icon, app, span, multi }
}

use AppId::{Breed, Calc, Cards, Info, Map, Pokedex, Shiny, Sim, Team};

pub(super) const DEFS: &[WidgetDef] = &[
    // the Dashboard's own
    wd("today", "Today at a glance", "Today's encounters, your daily goal, streak and level", "📅", None, FULL, false),
    wd("almost", "Almost there", "The achievements you're closest to", "🏆", None, 1, false),
    wd("jump", "Jump to", "Every app you have, one click away", "📌", None, FULL, false),
    wd("clock", "Clock & session", "The time, how long the app's been open and today's hunting", "🕘", None, 1, false),
    wd("notes", "Notes", "Jot things down; they stay here", "📝", None, 1, true),
    // Shiny Hunter
    wd("hunt_ctrl", "Current hunt", "Your count, odds and time, with Start overlay", "🎯", Some(Shiny), 2, false),
    wd("hunts", "Pick up a hunt", "Hunts you can resume, or the ones you've finished", "📜", Some(Shiny), 2, false),
    wd("shinies", "Recent shinies", "Your latest catches", "✨", Some(Shiny), 1, false),
    wd("showcase", "Shiny showcase", "Your latest shinies, big, one after another", "🌟", Some(Shiny), 1, false),
    wd("week", "This week", "Encounters each day for the last 7 days, and your best day", "📊", Some(Shiny), 1, false),
    wd("shinydex", "Shiny Dex progress", "How much of the Shiny Dex you've filled, and what's next", "💎", Some(Shiny), 1, false),
    wd("luck", "Luck meter", "How your current hunt compares with everyone's luck", "🍀", Some(Shiny), 1, false),
    // Pokédex
    wd("motd", "Pokémon of the day", "A different Pokémon every day: its types and where to find it", "📖", Some(Pokedex), 1, false),
    wd("dexsearch", "Pokédex search", "Look a Pokémon up without leaving the Dashboard", "🔍", Some(Pokedex), FULL, false),
    wd("favs", "Favourite Pokémon", "The Pokémon you pick, a click from their page", "❤", Some(Pokedex), 2, true),
    wd("catchable", "Catchable here", "What lives in a place you pick", "🌿", Some(Pokedex), 1, true),
    wd("typematch", "Type matchup", "What a type hits hard, and what hits it hard", "⚔", Some(Pokedex), 1, true),
    // Map
    wd("rare", "Rare grass today", "Today's rare grass spots", "🌾", Some(Map), 2, false),
    wd("minimap", "Mini map", "Sinnoh at a glance; click to open the Map", "🗺", Some(Map), 2, false),
    wd("mapfavs", "Favourite places", "Places you pick, with what lives there", "📍", Some(Map), 1, true),
    wd("huntwhere", "Where's my hunt", "Where your current target can be found", "🔭", Some(Map), 1, false),
    wd("maprecent", "Recent places", "The places you looked at last on the Map", "🕘", Some(Map), 1, false),
    // Game Info
    wd("infonews", "Latest game news", "The newest news from the game's wiki", "📰", Some(Info), 2, false),
    wd("tip", "Tip of the day", "A different how-to-play guide every day", "💡", Some(Info), 1, false),
    wd("infosearch", "Quick lookup", "Search moves, items, places and guides", "🔎", Some(Info), FULL, false),
    wd("adventure", "Your adventure", "Where you are in the story", "🏅", Some(Info), 2, false),
    wd("bookmarks", "Pinned pages", "Game Info pages you pinned, and ones you looked at lately", "🔖", Some(Info), 1, false),
    // Team Planner
    wd("mainteam", "Main team", "Your team at a glance", "🛡", Some(Team), 2, false),
    wd("teamweak", "Team weaknesses", "Types your team struggles against", "⚠", Some(Team), 1, false),
    wd("teamswitch", "Team switcher", "Pick which team the Dashboard shows", "🔁", Some(Team), 1, false),
    wd("coverage", "Coverage check", "How hard your team's moves hit every type", "🎯", Some(Team), 2, false),
    wd("teamidea", "Team ideas", "A type to add that covers your team's gaps", "💭", Some(Team), 1, false),
    // Battle Sim
    wd("lastbattle", "Last battle", "How your last battle went, with Rematch", "⚔", Some(Sim), 1, false),
    wd("winrecord", "Win record", "Wins and losses, and your best team", "🏆", Some(Sim), 1, false),
    wd("quickbattle", "Quick battle", "Battle a random wild Pokémon in one click", "⚡", Some(Sim), 1, false),
    wd("trainers", "Your trainers", "Trainers you made, ready to battle", "🎖", Some(Sim), 2, false),
    wd("battlehistory", "Battle history", "Your last five battles", "📜", Some(Sim), 2, false),
    // Breeding
    wd("breeding", "Breeding", "What's in the Day Care", "🐣", Some(Breed), 1, false),
    wd("breeders", "Breeders box", "Your breeders, with their stats and natures", "📦", Some(Breed), 1, false),
    wd("eggmoves", "Egg move finder", "The egg moves a Pokémon you pick can get", "💥", Some(Breed), 1, true),
    wd("nextstep", "Next step", "The one next thing to do in your breeding plan", "👣", Some(Breed), 1, false),
    wd("hatches", "Hatch tracker", "Eggs hatched, and shinies from Eggs", "🐥", Some(Breed), 1, false),
    // Damage Calc
    wd("quickcalc", "Quick calc", "One Pokémon's best hit on another, right here", "🔢", Some(Calc), 2, true),
    wd("savedcalcs", "Saved calcs", "Calcs you saved from Quick calc", "⭐", Some(Calc), 1, false),
    wd("speedtiers", "Speed tiers", "Who your main team outspeeds", "💨", Some(Calc), 1, false),
    wd("lastcalc", "Last result", "The calculation open in the Damage Calc", "📋", Some(Calc), 1, false),
    wd("threat", "Threat check", "Your main team against one Pokémon you pick", "🚨", Some(Calc), 2, true),
    // Card Collector
    wd("cards_xp", "Booster progress", "XP towards your next booster pack", "⭐", Some(Cards), 1, false),
    wd("cards_packs", "Packs to open", "Booster packs waiting, one click to open", "🎁", Some(Cards), 1, false),
    wd("cards_pulls", "Latest pulls", "The best cards from your last packs", "🃏", Some(Cards), 2, false),
    wd("cards_sets", "Set completion", "How much of each set is in your binder", "📚", Some(Cards), 1, false),
    wd("cards_missions", "Card missions", "Today's missions for more XP", "🎯", Some(Cards), 1, false),
];

pub(super) fn def(kind: &str) -> Option<&'static WidgetDef> {
    DEFS.iter().find(|d| d.kind == kind)
}

/// The layout everyone starts with: nothing (you pick your own).
pub(super) fn default_widgets() -> Vec<WidgetCfg> {
    Vec::new()
}

/// How the gallery lists the widgets.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Sort {
    #[default]
    App,
    Name,
    Size,
}

/// The "Add widgets" gallery.
#[derive(Default)]
pub(super) struct Gallery {
    pub open: bool,
    pub query: String,
    pub sort: Sort,
    /// Only this app's widgets (Some(None): the Dashboard's own).
    pub app: Option<Option<AppId>>,
    /// Hide the ones already on the Dashboard.
    pub not_added: bool,
}

#[derive(Default)]
pub(super) struct HomeState {
    pub edit: bool,
    /// Each widget's height at each width (columns) last time it was drawn.
    pub heights: HashMap<(u64, usize), f32>,
    /// The widget being dragged, where it was grabbed (from its corner),
    /// and where it would land: (place in the order, width in columns).
    pub drag: Option<u64>,
    pub grab: egui::Vec2,
    pub target: Option<(usize, usize)>,
    /// The "Add widgets" gallery.
    pub gallery: Gallery,
    /// "Clear all" was pressed once (press again to confirm), and when.
    pub clear_armed: Option<std::time::Instant>,
    /// How many columns the grid has (as of the last frame).
    pub cols: usize,
    /// Widgets' working state (search boxes and the like), by widget id.
    pub text: HashMap<(u64, &'static str), String>,
    /// The breeding plan the Next step widget shows, and what it was made from.
    pub plan: Option<(String, crate::breed_plan::Plan)>,
    /// The full plan (with the Egg estimate) being worked out.
    pub plan_rx: Option<(String, std::sync::mpsc::Receiver<crate::breed_plan::Plan>)>,
}

impl App {
    pub(super) fn home_widgets(&self) -> Vec<WidgetCfg> {
        self.cfg.home.widgets.clone().unwrap_or_else(default_widgets)
    }

    fn set_home_widgets(&mut self, w: Vec<WidgetCfg>) {
        self.cfg.home.widgets = Some(w);
        self.dirty = true;
    }

    /// Put a widget at the end of the Dashboard.
    fn add_widget(&mut self, d: &'static WidgetDef) {
        let mut w = self.home_widgets();
        let id = w.iter().map(|x| x.id).max().unwrap_or(0) + 1;
        w.push(WidgetCfg { id, kind: d.kind.to_string(), span: d.span, data: Vec::new() });
        self.set_home_widgets(w);
        self.app_toast(format!("Added {} to your Dashboard", d.name));
    }

    /// The edit bar: Customize / Done, Add, Clear all. (An empty Dashboard
    /// has its own big start instead.)
    pub(super) fn home_widgets_bar(&mut self, ui: &mut egui::Ui) {
        let edit = self.setup.home.edit;
        let none = self.home_widgets().is_empty();
        if none && !edit {
            return;
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(if edit { "Customize your Dashboard" } else { "Your widgets" }).font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
            if edit {
                ui.label(egui::RichText::new("Drag to move · pick a width · × to remove").size(kit::fs(12.5)).color(MUTED()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(if edit { 84.0 } else { 116.0 }, 32.0), Sense::hover());
                if pill_button(ui, r, if edit { "Done" } else { "✏  Customize" }, edit, "home-edit").clicked() {
                    self.setup.home.edit = !edit;
                    self.setup.home.clear_armed = None;
                }
                let (r, _) = ui.allocate_exact_size(vec2(118.0, 32.0), Sense::hover());
                if pill_button(ui, r, "+  Add widget", !edit, "home-add").on_hover_text("Open the widget gallery").clicked() {
                    self.setup.home.gallery.open = true;
                }
                if edit && !none {
                    let armed = self.setup.home.clear_armed.is_some_and(|t| t.elapsed().as_secs_f32() < 3.0);
                    let (r, _) = ui.allocate_exact_size(vec2(if armed { 118.0 } else { 86.0 }, 32.0), Sense::hover());
                    let resp = pill_button(ui, r, if armed { "Sure? Clear all" } else { "Clear all" }, false, "home-clear").on_hover_text("Take every widget off the Dashboard");
                    if resp.clicked() {
                        if armed {
                            self.set_home_widgets(Vec::new());
                            self.setup.home.clear_armed = None;
                            self.setup.home.edit = false;
                        } else {
                            self.setup.home.clear_armed = Some(std::time::Instant::now());
                        }
                    }
                    if armed {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                    }
                }
            });
        });
        ui.add_space(12.0);
    }

    /// An empty Dashboard: faint outlines of widgets to come, and one big
    /// button. A few suggestions underneath add with one click.
    fn home_empty(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width();
        let h = 330.0;
        let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(r, 20.0, theme::blend(kit::PAGE(), CARD(), 0.55));
        dashed(&p, r, theme::blend(LINE(), kit::primary(), 0.35));
        // ghost widgets: a little waterfall of outlines
        let ghost = Rect::from_center_size(pos2(r.center().x, r.min.y + 92.0), vec2(300.0_f32.min(w - 60.0), 112.0));
        let gw = (ghost.width() - 16.0) / 3.0;
        let t = ui.input(|i| i.time) as f32;
        let boxes = [(0.0, 0.0, 1.0, 46.0), (1.0, 0.0, 2.0, 30.0), (1.0, 38.0, 1.0, 66.0), (2.0, 38.0, 1.0, 66.0), (0.0, 54.0, 1.0, 50.0)];
        for (k, (c, y, span, bh)) in boxes.iter().enumerate() {
            let br = Rect::from_min_size(pos2(ghost.min.x + c * (gw + 8.0), ghost.min.y + y), vec2(gw * span + 8.0 * (span - 1.0), *bh));
            let pulse = 0.5 + 0.5 * (t * 1.6 - k as f32 * 0.6).sin();
            let col = [kit::C_VIOLET, kit::C_CYAN, kit::C_GOLD, kit::C_GREEN, kit::C_PINK][k];
            p.rect_filled(br, 10.0, theme::blend(CARD(), col, 0.10 + 0.08 * pulse));
            p.rect_stroke(br, 10.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.35 + 0.3 * pulse)));
            p.rect_filled(Rect::from_min_size(br.min + vec2(8.0, 8.0), vec2((br.width() * 0.45).min(60.0), 6.0)), 3.0, theme::blend(CARD(), col, 0.45));
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
        p.text(pos2(r.center().x, r.min.y + 178.0), Align2::CENTER_CENTER, "Your Dashboard is empty", crate::fonts::sans_bold_font(20.0), TEXT());
        p.text(pos2(r.center().x, r.min.y + 204.0), Align2::CENTER_CENTER, "Add widgets from any of your apps and arrange them how you like.", crate::fonts::sans_font(13.0), MUTED());
        let br = Rect::from_center_size(pos2(r.center().x, r.min.y + 248.0), vec2(230.0, 44.0));
        if pill_button(ui, br, "+  Add your first widget", true, "home-first").clicked() {
            self.setup.home.gallery.open = true;
        }
        // one-click suggestions
        let picks: Vec<&'static WidgetDef> = ["hunt_ctrl", "today", "rare", "shinies"].iter().filter_map(|k| def(k)).filter(|d| d.app.map_or(true, |a| self.has_app(a))).take(3).collect();
        let lw = p.layout_no_wrap("Or start with:".into(), crate::fonts::sans_font(12.0), MUTED()).size().x;
        let sizes: Vec<f32> = picks.iter().map(|d| p.layout_no_wrap(format!("{}  {}", d.icon, d.name), crate::fonts::sans_bold_font(12.0), TEXT()).size().x + 26.0).collect();
        let total = lw + 10.0 + sizes.iter().sum::<f32>() + 8.0 * sizes.len().saturating_sub(1) as f32;
        let mut x = r.center().x - total / 2.0;
        let y = r.min.y + 298.0;
        p.text(pos2(x, y), Align2::LEFT_CENTER, "Or start with:", crate::fonts::sans_font(12.0), MUTED());
        x += lw + 10.0;
        let mut add = None;
        for (d, sw) in picks.iter().zip(sizes) {
            let cr = Rect::from_min_size(pos2(x, y - 14.0), vec2(sw, 28.0));
            if pill_button(ui, cr, &format!("{}  {}", d.icon, d.name), false, ("home-sugg", d.kind)).on_hover_text(d.about).clicked() {
                add = Some(*d);
            }
            x += sw + 8.0;
        }
        if let Some(d) = add {
            self.add_widget(d);
        }
    }

    /// The widget gallery: every widget with a live preview, a search box,
    /// sorting and a filter for each app.
    pub(super) fn home_gallery(&mut self, ctx: &egui::Context) {
        if !self.setup.home.gallery.open {
            return;
        }
        let screen = ctx.screen_rect();
        let size = vec2((screen.width() - 80.0).clamp(360.0, 1180.0), (screen.height() - 70.0).clamp(300.0, 860.0));
        let panel = Rect::from_center_size(screen.center(), size);
        let mut close = false;
        let mut add: Option<&'static WidgetDef> = None;
        let mut get: Option<(AppId, &'static str)> = None;
        let have: Vec<String> = self.home_widgets().into_iter().map(|w| w.kind).collect();
        // one area: the scrim, with the panel drawn on it (so the panel
        // never ends up under its own scrim)
        egui::Area::new(egui::Id::new("widget-gallery")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(150));
            if scrim.clicked() {
                close = true;
            }
            ui.painter().add(kit::shadow_tier(3).as_shape(panel, 22.0));
            ui.painter().rect_filled(panel, 22.0, CARD());
            ui.painter().rect_stroke(panel, 22.0, Stroke::new(1.0f32, kit::CONTROL()));
            // the panel eats its own clicks
            ui.interact(panel, egui::Id::new("widget-gallery-panel"), Sense::click());
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(panel.shrink2(vec2(26.0, 22.0))).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                // header
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Add widgets").font(crate::fonts::sans_bold_font(22.0)).color(TEXT()));
                        ui.label(egui::RichText::new("Pick what your Dashboard shows. Each app brings its own; add as many as you like.").size(kit::fs(12.5)).color(MUTED()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                        if pill_button(ui, r, "×", false, "gal-x").on_hover_text("Close (Esc)").clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(12.0);
                // search, sort, not-added
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    kit::search_box_keyed(ui, Some(egui::Id::new("gal-search")), &mut self.setup.home.gallery.query, "🔍  Search widgets", 260.0);
                    ui.label(egui::RichText::new("Sort").size(kit::fs(12.0)).color(MUTED()));
                    let mut sort = self.setup.home.gallery.sort;
                    kit::segmented(ui, &mut sort, &[(Sort::App, "By app"), (Sort::Name, "A–Z"), (Sort::Size, "Size")]);
                    self.setup.home.gallery.sort = sort;
                    ui.add_space(6.0);
                    let mut na = self.setup.home.gallery.not_added;
                    if ui.checkbox(&mut na, egui::RichText::new("Hide added").size(kit::fs(12.5)).color(TEXT())).changed() {
                        self.setup.home.gallery.not_added = na;
                    }
                });
                ui.add_space(10.0);
                // app filters
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    let mut opts: Vec<(Option<Option<AppId>>, String, Color32)> = vec![(None, "All".into(), kit::primary()), (Some(None), "📅  Dashboard".into(), kit::C_CYAN)];
                    for a in crate::apps::ALL {
                        opts.push((Some(Some(a)), format!("{}  {}", app_icon(a), a.name()), app_col(a)));
                    }
                    for (val, label, col) in opts {
                        let n = DEFS.iter().filter(|d| val.map_or(true, |v| d.app == v)).count();
                        let on = self.setup.home.gallery.app == val;
                        let text = format!("{label}  {n}");
                        let g = ui.painter().layout_no_wrap(text, crate::fonts::sans_bold_font(12.0), if on { Color32::WHITE } else { TEXT() });
                        let (r, resp) = ui.allocate_exact_size(g.size() + vec2(22.0, 12.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        ui.painter().rect_filled(r, r.height() / 2.0, if on { col } else { theme::blend(kit::sunken(), col, 0.08 + 0.12 * ht) });
                        ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, if on { col } else { theme::blend(LINE(), col, 0.2 + 0.4 * ht) }));
                        ui.painter().galley(r.min + vec2(11.0, 6.0), g, TEXT());
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            self.setup.home.gallery.app = if on { None } else { val };
                        }
                    }
                });
                ui.add_space(12.0);
                // which widgets, in what order
                let g = &self.setup.home.gallery;
                let q = g.query.trim().to_lowercase();
                let mut list: Vec<&'static WidgetDef> = DEFS
                    .iter()
                    .filter(|d| g.app.map_or(true, |v| d.app == v))
                    .filter(|d| !g.not_added || d.multi || !have.iter().any(|k| k == d.kind))
                    .filter(|d| q.is_empty() || d.name.to_lowercase().contains(&q) || d.about.to_lowercase().contains(&q) || d.app.map_or("dashboard", |a| a.name()).to_lowercase().contains(&q))
                    .collect();
                let app_rank = |d: &WidgetDef| d.app.map_or(0, |a| 1 + crate::apps::ALL.iter().position(|x| *x == a).unwrap_or(0));
                match g.sort {
                    Sort::App => list.sort_by_key(|d| app_rank(d)),
                    Sort::Name => list.sort_by_key(|d| d.name),
                    Sort::Size => list.sort_by_key(|d| (std::cmp::Reverse(d.span), app_rank(d))),
                }
                let shown = list.len();
                ui.label(egui::RichText::new(format!("{shown} widget{}", if shown == 1 { "" } else { "s" })).size(kit::fs(12.0)).color(MUTED()));
                ui.add_space(6.0);
                if list.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| ui.label(egui::RichText::new("No widgets match. Try another word or app.").color(MUTED())));
                    return;
                }
                // the grid of tiles, each with a live preview
                const TILE_H: f32 = 300.0;
                const PREVIEW_H: f32 = 184.0;
                let gap = 14.0;
                egui::ScrollArea::vertical().id_salt("gal-scroll").auto_shrink([false, false]).show(ui, |ui| {
                    let w = ui.available_width() - 6.0;
                    let cols = ((w + gap) / (300.0 + gap)).floor().max(1.0) as usize;
                    let tw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor();
                    for (row_i, row) in list.chunks(cols).enumerate() {
                        let (row_r, _) = ui.allocate_exact_size(vec2(w, TILE_H), Sense::hover());
                        ui.add_space(gap);
                        if !ui.is_rect_visible(row_r) {
                            continue;
                        }
                        for (c, d) in row.iter().enumerate() {
                            let r = Rect::from_min_size(pos2(row_r.min.x + c as f32 * (tw + gap), row_r.min.y), vec2(tw, TILE_H));
                            let added = have.iter().filter(|k| *k == d.kind).count();
                            let installed = d.app.map_or(true, |a| self.has_app(a));
                            let col = d.app.map_or(kit::C_CYAN, app_col);
                            let tile = ui.interact(r, ui.id().with(("gal-tile", d.kind)), Sense::hover());
                            let ht = kit::anim(ui.ctx(), tile.id.with("h"), tile.hovered(), kit::QUICK);
                            let p = ui.painter().clone();
                            p.rect_filled(r, 16.0, theme::blend(kit::sunken(), CARD_HOVER(), 0.4 * ht));
                            p.rect_stroke(r, 16.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.15 + 0.5 * ht)));
                            // the live preview, clipped and faded at the bottom
                            let pr = Rect::from_min_size(r.min + vec2(10.0, 10.0), vec2(tw - 20.0, PREVIEW_H));
                            p.rect_filled(pr, 12.0, theme::blend(kit::PAGE(), col, 0.05));
                            let fake = WidgetCfg { id: 9_000_000 + (row_i * 16 + c) as u64, kind: d.kind.to_string(), span: d.span, data: Vec::new() };
                            let inner = pr.shrink(8.0);
                            // drawn in a child that takes no room in the grid
                            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(inner.min, vec2(inner.width(), 2000.0))).layout(egui::Layout::top_down(egui::Align::Min)));
                            child.set_clip_rect(inner.intersect(ui.clip_rect()));
                            child.set_width(inner.width());
                            if installed {
                                let _ = self.draw_widget(&mut child, &fake, true);
                            } else {
                                widget_ghost(&mut child, d);
                            }
                            // nothing in the preview clicks
                            ui.interact(pr, ui.id().with(("gal-shield", d.kind)), Sense::click());
                            fade_bottom(&p, Rect::from_min_max(pos2(pr.min.x, pr.max.y - 46.0), pr.max), theme::blend(kit::sunken(), CARD_HOVER(), 0.4 * ht));
                            // size tag on the preview
                            let size = match d.span {
                                1 => "⅓ width",
                                2 => "⅔ width",
                                _ => "Full width",
                            };
                            let sg = p.layout_no_wrap(size.into(), crate::fonts::sans_bold_font(10.5), TEXT());
                            let sr = Rect::from_min_size(pos2(pr.max.x - sg.size().x - 22.0, pr.min.y + 8.0), sg.size() + vec2(14.0, 6.0));
                            p.rect_filled(sr, 8.0, CARD().gamma_multiply(0.92));
                            p.rect_stroke(sr, 8.0, Stroke::new(1.0f32, LINE()));
                            p.galley(sr.min + vec2(7.0, 3.0), sg, TEXT());
                            // name, app, what it does
                            let ty = pr.max.y + 12.0;
                            let chip = Rect::from_min_size(pos2(r.min.x + 12.0, ty), vec2(30.0, 30.0));
                            kit::icon_chip(&p, chip, d.icon, col, false);
                            let clip = p.with_clip_rect(Rect::from_min_max(pos2(chip.max.x + 8.0, ty - 2.0), pos2(r.max.x - 12.0, r.max.y)));
                            clip.text(pos2(chip.max.x + 10.0, ty + 6.0), Align2::LEFT_CENTER, d.name, crate::fonts::sans_bold_font(14.0), TEXT());
                            clip.text(pos2(chip.max.x + 10.0, ty + 24.0), Align2::LEFT_CENTER, d.app.map_or("Dashboard", |a| a.name()), crate::fonts::sans_font(11.5), theme::blend(MUTED(), col, 0.5));
                            let ab = p.layout(d.about.to_string(), crate::fonts::sans_font(12.0), MUTED(), tw - 24.0);
                            p.with_clip_rect(Rect::from_min_max(pos2(r.min.x, ty + 40.0), pos2(r.max.x, ty + 72.0))).galley(pos2(r.min.x + 12.0, ty + 40.0), ab, MUTED());
                            // the button
                            let br = Rect::from_min_size(pos2(r.max.x - 112.0, r.max.y - 40.0), vec2(100.0, 30.0));
                            if added > 0 {
                                p.text(pos2(r.min.x + 14.0, br.center().y), Align2::LEFT_CENTER, if added > 1 { format!("✓ On your Dashboard ×{added}") } else { "✓ On your Dashboard".to_string() }, crate::fonts::sans_bold_font(11.5), theme::GREEN);
                            }
                            if !installed {
                                if pill_button(ui, br, "🔒  Get app", false, ("gal-get", d.kind)).on_hover_text(format!("Needs the {} app", d.app.map(|a| a.name()).unwrap_or(""))).clicked() {
                                    get = d.app.map(|a| (a, d.name));
                                }
                            } else if added > 0 && !d.multi {
                                p.text(br.center(), Align2::CENTER_CENTER, "Added", crate::fonts::sans_bold_font(12.5), MUTED());
                            } else if pill_button(ui, br, if added > 0 { "+  Add another" } else { "+  Add" }, true, ("gal-add", d.kind)).clicked() {
                                add = Some(*d);
                            }
                        }
                    }
                });
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.setup.store.modal.is_none() {
            close = true;
        }
        if let Some(d) = add {
            self.add_widget(d);
        }
        if let Some((a, name)) = get {
            self.ask_for_app(a, name);
        }
        if close {
            self.setup.home.gallery.open = false;
        }
    }

    /// A widget's height at `span` columns: measured, or its height at
    /// another width, or a guess.
    fn widget_h(&self, id: u64, span: usize) -> f32 {
        let hs = &self.setup.home.heights;
        hs.get(&(id, span)).copied().or_else(|| hs.iter().find(|((i, _), _)| *i == id).map(|(_, h)| *h)).unwrap_or(160.0)
    }

    /// The waterfall grid of widgets. Each goes into the highest open space
    /// it fits (gaps fill themselves). While one is dragged, the rest slide
    /// to show where it would land, and it takes the width that best fits
    /// the space under the pointer.
    pub(super) fn home_widget_grid(&mut self, ui: &mut egui::Ui) {
        let edit = self.setup.home.edit;
        let all = self.home_widgets();
        if all.is_empty() {
            self.home_empty(ui);
            return;
        }
        // Widgets whose app isn't installed hide (they show, locked, while
        // customising; they come back when the app does).
        let shown: Vec<WidgetCfg> = all.iter().filter(|w| def(&w.kind).is_some_and(|d| edit || d.app.map_or(true, |a| self.has_app(a)))).cloned().collect();
        let width = ui.available_width();
        let cols = crate::masonry::columns(width, MIN_COL, GAP, MAX_COLS);
        let col_w = (width - GAP * (cols as f32 - 1.0)) / cols as f32;
        self.setup.home.cols = cols;
        let origin = ui.cursor().min;
        let pointer = ui.ctx().pointer_interact_pos();
        if !edit || !shown.iter().any(|w| Some(w.id) == self.setup.home.drag) {
            self.setup.home.drag = None;
            self.setup.home.target = None;
        }
        let dragged = self.setup.home.drag;

        // This frame's order and widths.
        let mut order: Vec<(WidgetCfg, usize)> = shown.iter().map(|w| (w.clone(), (w.span as usize).clamp(1, cols))).collect();
        if let (Some(id), Some(pp)) = (dragged, pointer) {
            let others: Vec<(WidgetCfg, usize)> = order.iter().filter(|(w, _)| w.id != id).cloned().collect();
            let me = order.iter().find(|(w, _)| w.id == id).cloned().expect("dragged is shown");
            let grab = self.setup.home.grab;
            // where the card's corner would be, by the pointer
            let want = pp - grab;
            let mut best: Option<(f32, usize, usize)> = None;
            for i in 0..=others.len() {
                for s in 1..=cols {
                    let mut test = others.clone();
                    test.insert(i, (me.0.clone(), s));
                    let spans: Vec<usize> = test.iter().map(|t| t.1).collect();
                    let hs: Vec<f32> = test.iter().map(|(w, sp)| self.widget_h(w.id, *sp)).collect();
                    let (pl, total) = crate::masonry::pack(&spans, &hs, cols, GAP);
                    let p = pl[i];
                    let r = Rect::from_min_size(pos2(origin.x + p.col as f32 * (col_w + GAP), origin.y + p.y), vec2(col_w * s as f32 + GAP * (s as f32 - 1.0), hs[i]));
                    // close to where the pointer holds it, inside if possible;
                    // keep its own width unless another fits clearly better;
                    // and prefer layouts with fewer holes
                    let mut score = (r.min - want).length() + if r.contains(pp) { 0.0 } else { r.distance_to_pos(pp) * 2.0 };
                    if s != me.1 {
                        score += 40.0;
                    }
                    score += total * 0.02;
                    if best.map_or(true, |b| score < b.0) {
                        best = Some((score, i, s));
                    }
                }
            }
            if let Some((_, i, s)) = best {
                self.setup.home.target = Some((i, s));
                order = others;
                order.insert(i, (me.0, s));
            }
        }
        let spans: Vec<usize> = order.iter().map(|o| o.1).collect();
        let heights: Vec<f32> = order.iter().map(|(w, s)| self.widget_h(w.id, *s)).collect();
        let (places, total) = crate::masonry::pack(&spans, &heights, cols, GAP);
        let rect_of = |i: usize| {
            let pl = places[i];
            let w = col_w * pl.span as f32 + GAP * (pl.span as f32 - 1.0);
            Rect::from_min_size(pos2(origin.x + pl.col as f32 * (col_w + GAP), origin.y + pl.y), vec2(w, heights[i]))
        };
        let mut new_heights: Vec<((u64, usize), f32)> = Vec::new();
        let mut action: Option<Action> = None;
        let mut floating: Option<(usize, Rect)> = None;
        for (i, (w, span)) in order.iter().enumerate() {
            let target = rect_of(i);
            if Some(w.id) == dragged {
                // where it would land: a dashed space with its width
                let p = ui.painter();
                p.rect_filled(target, 16.0, kit::primary().gamma_multiply(0.08));
                dashed(p, target, kit::primary());
                let label = match (*span, cols) {
                    (s, c) if s >= c => "Full width".to_string(),
                    (1, 3) => "⅓ width".to_string(),
                    (2, 3) => "⅔ width".to_string(),
                    _ => "½ width".to_string(),
                };
                p.text(target.center(), Align2::CENTER_CENTER, label, crate::fonts::sans_bold_font(13.0), kit::primary());
                let at = pointer.map_or(target.min, |pp| pp - self.setup.home.grab);
                floating = Some((i, Rect::from_min_size(at, target.size())));
                continue;
            }
            // others glide to their new places
            let r = if edit { kit::glide_rect(ui.ctx(), egui::Id::new(("w-pos", w.id)), target, origin, kit::BASE) } else { target };
            let h = self.draw_widget_at(ui, w, r, *span, cols, edit, false, &mut action);
            new_heights.push(((w.id, *span), h));
        }
        // the dragged card, last so it's on top, following the pointer
        if let Some((i, r)) = floating {
            let (w, span) = order[i].clone();
            ui.painter().add(kit::shadow_tier(3).as_shape(r, 16.0));
            let h = self.draw_widget_at(ui, &w, r, span, cols, edit, true, &mut action);
            new_heights.push(((w.id, span), h));
            if ui.input(|inp| inp.pointer.any_released()) {
                if let Some((to, s)) = self.setup.home.target {
                    action = Some(Action::Place { id: w.id, to, span: s });
                }
                self.setup.home.drag = None;
                self.setup.home.target = None;
            }
        }
        for (k, h) in new_heights {
            let old = self.setup.home.heights.insert(k, h);
            if old.map_or(true, |o| (o - h).abs() > 0.5) {
                ui.ctx().request_repaint();
            }
        }
        ui.allocate_rect(Rect::from_min_size(origin, vec2(width, total.max(0.0))), Sense::hover());
        if let Some(a) = action {
            self.apply_widget_action(a, &order.iter().map(|o| o.0.clone()).collect::<Vec<_>>());
        }
    }

    /// Draw one widget at `r` (top-left and width; it grows to its height),
    /// with its Customize bar when editing. Returns its height.
    #[allow(clippy::too_many_arguments)]
    fn draw_widget_at(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, r: Rect, span: usize, cols: usize, edit: bool, lifted: bool, action: &mut Option<Action>) -> f32 {
        let Some(d) = def(&w.kind) else { return 0.0 };
        let locked = d.app.is_some_and(|a| !self.has_app(a));
        kit::note_card(ui, Rect::NOTHING);
        let resp = ui.allocate_new_ui(egui::UiBuilder::new().max_rect(Rect::from_min_size(r.min, vec2(r.width(), 4000.0))).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
            ui.set_width(r.width());
            if locked {
                let (lr, _) = ui.allocate_exact_size(vec2(r.width(), 150.0), Sense::hover());
                card_bg(ui.painter(), lr, false);
                ui.painter().text(lr.min + vec2(20.0, 22.0), Align2::LEFT_TOP, format!("{}  {}", d.icon, d.name), crate::fonts::sans_bold_font(15.0), TEXT());
                if let (Some(a), false) = (d.app, edit) {
                    self.lock_veil(ui, lr, a, d.name);
                }
            } else if let Some(data) = self.draw_widget(ui, w, false) {
                *action = Some(Action::Data(w.id, data));
            }
        });
        // the card's own bottom edge (cards add space after themselves)
        let drawn = resp.response.rect;
        let h = match kit::last_card_rect(ui) {
            Some(c) if c.is_positive() && c.min.y >= drawn.min.y - 1.0 && c.max.y <= drawn.max.y + 1.0 => c.max.y - drawn.min.y,
            _ => drawn.height(),
        };
        if edit {
            let r = Rect::from_min_size(r.min, vec2(r.width(), h));
            if let Some(a) = self.widget_edit_chrome(ui, r, w, d, span, cols, lifted) {
                *action = Some(a);
            }
        }
        h
    }

    /// In Customize: the dashed edge, the bar on top (widths, remove), and
    /// picking the card up to drag it. Returns what to change.
    #[allow(clippy::too_many_arguments)]
    fn widget_edit_chrome(&mut self, ui: &mut egui::Ui, r: Rect, w: &WidgetCfg, d: &WidgetDef, span: usize, cols: usize, lifted: bool) -> Option<Action> {
        let mut out = None;
        // The whole card is the handle (and nothing inside it clicks).
        let body = ui.interact(r, ui.id().with(("w-body", w.id)), Sense::click_and_drag());
        let p = ui.painter().clone();
        p.rect_filled(r, 16.0, theme::blend(kit::PAGE(), CARD(), 0.3).gamma_multiply(if lifted { 0.1 } else { 0.25 }));
        dashed(&p, r.expand(4.0), if lifted || body.hovered() { kit::primary() } else { kit::CONTROL() });
        if body.drag_started() {
            if let Some(pp) = ui.ctx().pointer_interact_pos() {
                self.setup.home.drag = Some(w.id);
                self.setup.home.grab = pp - r.min;
            }
        }
        if body.hovered() || body.dragged() {
            ui.ctx().set_cursor_icon(if body.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        // the bar: name, widths, ×
        let bar = Rect::from_min_size(pos2(r.min.x + 10.0, r.min.y + 10.0), vec2(r.width() - 20.0, 34.0));
        p.rect_filled(bar, 12.0, CARD_HOVER());
        p.rect_stroke(bar, 12.0, Stroke::new(1.0f32, LINE()));
        p.with_clip_rect(bar.shrink(2.0)).text(pos2(bar.min.x + 12.0, bar.center().y), Align2::LEFT_CENTER, format!("☰  {}", d.name), crate::fonts::sans_bold_font(12.5), TEXT());
        if lifted {
            return None;
        }
        let mut x = bar.max.x - 6.0;
        let xr = Rect::from_min_size(pos2(x - 26.0, bar.min.y + 4.0), vec2(26.0, 26.0));
        if pill_button(ui, xr, "×", false, ("w-x", w.id)).on_hover_text("Remove this widget").clicked() {
            out = Some(Action::Remove(w.id));
        }
        x = xr.min.x - 8.0;
        let opts: Vec<(u8, &str)> = match cols {
            1 => vec![],
            2 => vec![(1, "½"), (FULL, "Full")],
            _ => vec![(1, "⅓"), (2, "⅔"), (FULL, "Full")],
        };
        for (sp, label) in opts.iter().rev() {
            let on = span == (*sp as usize).clamp(1, cols);
            let g = p.layout_no_wrap(label.to_string(), crate::fonts::sans_bold_font(11.5), TEXT());
            let br = Rect::from_min_size(pos2(x - g.size().x - 18.0, bar.min.y + 5.0), vec2(g.size().x + 18.0, 24.0));
            if br.min.x < bar.min.x + 40.0 {
                break;
            }
            let resp = ui.interact(br, ui.id().with(("w-span", w.id, *sp)), Sense::click());
            p.rect_filled(br, 12.0, if on { kit::primary() } else if resp.hovered() { CARD() } else { Color32::TRANSPARENT });
            p.galley(br.center() - g.size() / 2.0, g, if on { kit::on_primary() } else { MUTED() });
            if resp.on_hover_text("Width (dragging picks one for you too)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !on {
                out = Some(Action::Span(w.id, *sp));
            }
            x = br.min.x - 4.0;
        }
        out
    }

    fn apply_widget_action(&mut self, a: Action, shown: &[WidgetCfg]) {
        let mut all = self.home_widgets();
        let cols_full = |s: usize| -> u8 {
            if s >= MAX_COLS {
                FULL
            } else {
                s as u8
            }
        };
        match a {
            Action::Remove(id) => all.retain(|w| w.id != id),
            Action::Data(id, data) => {
                if let Some(w) = all.iter_mut().find(|w| w.id == id) {
                    w.data = data;
                }
            }
            Action::Span(id, s) => {
                if let Some(w) = all.iter_mut().find(|w| w.id == id) {
                    w.span = s;
                }
            }
            Action::Place { id, to, span } => {
                // `to` is its place among the widgets on show; hidden ones
                // keep their spots in the full list
                let Some(from) = all.iter().position(|w| w.id == id) else { return };
                let mut item = all.remove(from);
                // a width as wide as the grid means full width (so it stays
                // full on a wider window)
                item.span = if span >= self.home_cols_last() { FULL } else { cols_full(span) };
                let after: Vec<&WidgetCfg> = shown.iter().filter(|w| w.id != id).collect();
                let at = match after.get(to) {
                    Some(b) => all.iter().position(|w| w.id == b.id).unwrap_or(all.len()),
                    None => after.last().and_then(|l| all.iter().position(|w| w.id == l.id)).map_or(all.len(), |p| p + 1),
                };
                all.insert(at.min(all.len()), item);
            }
        }
        self.set_home_widgets(all);
    }

    /// How many columns the grid had last time (for "full width").
    fn home_cols_last(&self) -> usize {
        self.setup.home.cols.max(1)
    }

    /// Draw one widget's card. `preview`: in the gallery (no settings of
    /// its own yet; show something even when there's nothing to show).
    /// Returns its settings when it changed them.
    pub(super) fn draw_widget(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool) -> Option<Vec<String>> {
        let mut out: Option<Vec<String>> = None;
        match w.kind.as_str() {
            "today" => self.home_today(ui),
            "hunts" => {
                if !self.home_runs(ui, 0.0) {
                    empty_card(ui, "🎯", "Pick up a hunt", "Hunts you pause show here, ready to resume.");
                }
            }
            "almost" => self.home_almost(ui, 0.0),
            "rare" => self.home_rare(ui),
            "shinies" => self.recent_shinies(ui, 0.0),
            "breeding" => {
                if !(self.home_breeding_on() && self.home_breeding(ui, 0.0)) {
                    empty_card(ui, "🐣", "Breeding", "Put two Pokémon in the Day Care in Breeding and it shows here.");
                }
            }
            "jump" => self.home_jump(ui),
            "clock" => self.w_clock(ui),
            "notes" => self.w_notes(ui, w, preview, &mut out),
            "hunt_ctrl" => self.w_hunt_ctrl(ui),
            "showcase" => self.w_showcase(ui),
            "week" => self.w_week(ui),
            "shinydex" => self.w_shinydex(ui),
            "luck" => self.w_luck(ui),
            "motd" => self.w_motd(ui),
            "dexsearch" => self.w_dexsearch(ui, w),
            "favs" => self.w_favs(ui, w, preview, &mut out),
            "catchable" => self.w_catchable(ui, w, preview, &mut out),
            "typematch" => self.w_typematch(ui, w, preview, &mut out),
            "minimap" => self.w_minimap(ui),
            "mapfavs" => self.w_mapfavs(ui, w, preview, &mut out),
            "huntwhere" => self.w_huntwhere(ui),
            "maprecent" => self.w_maprecent(ui),
            "infonews" => self.w_infonews(ui),
            "tip" => self.w_tip(ui),
            "infosearch" => self.w_infosearch(ui, w),
            "adventure" => self.w_adventure(ui),
            "bookmarks" => self.w_bookmarks(ui),
            "mainteam" => self.w_mainteam(ui),
            "teamweak" => self.w_teamweak(ui),
            "teamswitch" => self.w_teamswitch(ui),
            "coverage" => self.w_coverage(ui),
            "teamidea" => self.w_teamidea(ui),
            "lastbattle" => self.w_lastbattle(ui),
            "winrecord" => self.w_winrecord(ui),
            "quickbattle" => self.w_quickbattle(ui),
            "trainers" => self.w_trainers(ui),
            "battlehistory" => self.w_battlehistory(ui),
            "breeders" => self.w_breeders(ui),
            "eggmoves" => self.w_eggmoves(ui, w, preview, &mut out),
            "nextstep" => self.w_nextstep(ui),
            "hatches" => self.w_hatches(ui),
            "quickcalc" => self.w_quickcalc(ui, w, preview, &mut out),
            "savedcalcs" => self.w_savedcalcs(ui),
            "speedtiers" => self.w_speedtiers(ui),
            "lastcalc" => self.w_lastcalc(ui),
            "threat" => self.w_threat(ui, w, preview, &mut out),
            "cards_xp" => self.w_cards_xp(ui),
            "cards_packs" => self.w_cards_packs(ui),
            "cards_pulls" => self.w_cards_pulls(ui),
            "cards_sets" => self.w_cards_sets(ui),
            "cards_missions" => self.w_cards_missions(ui),
            _ => {}
        }
        out
    }
}

enum Action {
    Remove(u64),
    /// A widget changed its own settings.
    Data(u64, Vec<String>),
    Span(u64, u8),
    /// Put a widget at a place in the order, at a width (in columns).
    Place { id: u64, to: usize, span: usize },
}

/// A placeholder card for a widget with nothing to show yet.
pub(super) fn empty_card(ui: &mut egui::Ui, icon: &str, title: &str, text: &str) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 130.0), Sense::hover());
    let p = ui.painter();
    card_bg(p, r, false);
    p.text(r.min + vec2(20.0, 22.0), Align2::LEFT_TOP, format!("{icon}  {title}"), crate::fonts::sans_bold_font(15.0), TEXT());
    let g = p.layout(text.to_string(), crate::fonts::sans_font(12.5), MUTED(), w - 40.0);
    p.galley(r.min + vec2(20.0, 56.0), g, MUTED());
}

/// A dashed rounded-ish outline.
fn dashed(p: &egui::Painter, r: Rect, col: Color32) {
    let s = Stroke::new(1.5f32, col);
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    for w in pts.windows(2) {
        p.extend(egui::Shape::dashed_line(&[w[0], w[1]], s, 8.0, 6.0));
    }
}

/// A widget whose app isn't installed, in the gallery: its name on a
/// plain card with a lock.
fn widget_ghost(ui: &mut egui::Ui, d: &WidgetDef) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 160.0), Sense::hover());
    let p = ui.painter();
    card_bg(p, r, false);
    p.text(r.min + vec2(18.0, 20.0), Align2::LEFT_TOP, format!("{}  {}", d.icon, d.name), crate::fonts::sans_bold_font(14.5), TEXT());
    p.text(r.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "🔒", egui::FontId::proportional(26.0), MUTED());
    if let Some(a) = d.app {
        p.text(r.center() + vec2(0.0, 44.0), Align2::CENTER_CENTER, format!("Comes with {}", a.name()), crate::fonts::sans_font(12.0), MUTED());
    }
}

/// From clear at the top to `col` at the bottom.
fn fade_bottom(p: &egui::Painter, r: Rect, col: Color32) {
    let mut m = egui::Mesh::default();
    let clear = col.gamma_multiply(0.0);
    m.colored_vertex(r.left_top(), clear);
    m.colored_vertex(r.right_top(), clear);
    m.colored_vertex(r.right_bottom(), col);
    m.colored_vertex(r.left_bottom(), col);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(m);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widgets_are_unique_and_sized() {
        for (i, d) in DEFS.iter().enumerate() {
            assert!(DEFS.iter().skip(i + 1).all(|e| e.kind != d.kind), "{} twice", d.kind);
            assert!(matches!(d.span, 1 | 2 | FULL), "{} has width {}", d.kind, d.span);
        }
        // five (or more) for every app
        for a in crate::apps::ALL {
            assert!(DEFS.iter().filter(|d| d.app == Some(a)).count() >= 5, "{a:?} needs five widgets");
        }
    }

    #[test]
    fn dashboard_starts_empty_and_old_layouts_load() {
        assert!(default_widgets().is_empty());
        let w: WidgetCfg = serde_json::from_str(r#"{"id":1,"kind":"today","span":9}"#).unwrap();
        assert!(w.data.is_empty());
    }
}
