//! The Pokédex app: every Pokémon in Platinum Together from the wiki data,
//! in a sprite grid like the Shiny Dex. Clicking one opens its details.

use super::extras::{grid_cols, TileLook, TILE_GAP, TILE_H};
use super::ui_kit::{self as kit, PillKind, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::theme::{self, GOLD};
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, RichText, Sense, Stroke};
use std::sync::Arc;

/// How a Pokémon can be got (for the filter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum HowFilter {
    #[default]
    Any,
    Grass,
    Radar,
    Water,
    Cave,
    Gift,
    Evolve,
    CantGet,
}

impl HowFilter {
    const ALL: [HowFilter; 8] = [
        HowFilter::Any,
        HowFilter::Grass,
        HowFilter::Radar,
        HowFilter::Water,
        HowFilter::Cave,
        HowFilter::Gift,
        HowFilter::Evolve,
        HowFilter::CantGet,
    ];
    fn label(self) -> &'static str {
        match self {
            HowFilter::Any => "Any way",
            HowFilter::Grass => "Grass",
            HowFilter::Radar => "Poké Radar",
            HowFilter::Water => "Water (surf, rods)",
            HowFilter::Cave => "Caves",
            HowFilter::Gift => "Gifts, eggs, trades…",
            HowFilter::Evolve => "Only by evolving",
            HowFilter::CantGet => "Can't get yet",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum EraFilter {
    #[default]
    All,
    Platinum,
    Modern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum CaughtFilter {
    #[default]
    All,
    Caught,
    NotYet,
}

/// The Pokédex page's state.
#[derive(Default)]
pub(super) struct DexState {
    pub search: String,
    pub shiny: bool,
    pub type_filter: Option<String>,
    pub how: HowFilter,
    /// Only Pokémon you can get by this badge stage (index into stages).
    pub stage: Option<usize>,
    pub era: EraFilter,
    pub caught: CaughtFilter,
    /// The open entry (wiki id, e.g. "SCYTHER").
    pub open: Option<String>,
    /// The entry page's tab.
    pub tab: EntryTab,
    /// The grid's order last time it was drawn (for previous / next).
    list_ids: Vec<String>,
    /// Bumped to start the grid back at the top (its scroll spot is kept
    /// while an entry is open).
    pub grid_gen: u32,
    /// Where to find: hide spots that need MORE / MODERN POKéMON.
    pub hide_extra_spots: bool,
    /// Moves tab: which list.
    pub move_kind: super::dex_tabs::MoveKind,
    /// Where to find: show every catch-spot preview.
    pub all_spots: bool,
    /// Moves tab: show the MODERN POKéMON lists.
    pub modern_moves: Option<bool>,
    /// Summary of every Pokémon, built once per data load.
    index: Option<(usize, Vec<MonInfo>)>,
}

/// The tabs on a Pokémon's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum EntryTab {
    #[default]
    Overview,
    Evolution,
    Where,
    Moves,
}

/// What the list needs to know about a Pokémon, worked out once.
#[derive(Debug, Clone)]
struct MonInfo {
    id: String,
    name: String,
    dex: u32,
    types: Vec<String>,
    modern: bool,
    grass: bool,
    radar: bool,
    water: bool,
    cave: bool,
    gift: bool,
    evolves_only: bool,
    cant_get: bool,
    /// Earliest badge stage it's found in the wild (usize::MAX = not wild).
    first_stage: usize,
}

fn build_index(w: &Wiki) -> Vec<MonInfo> {
    w.all()
        .map(|m| {
            let wild = w.wild(&m.id);
            let has = |f: &dyn Fn(&str) -> bool| wild.iter().any(|e| f(&e.method));
            let other = w.other(&m.id);
            let cant_get = w.unobtainable(&m.id).is_some();
            let gift = !other.is_empty();
            let evolves_only = wild.is_empty() && !gift && !cant_get && w.evolves_from(&m.id).is_some();
            MonInfo {
                id: m.id.clone(),
                name: m.name.clone(),
                dex: m.dex,
                types: m.types.clone(),
                modern: m.modern,
                grass: has(&|x| x == "grass"),
                radar: has(&|x| x == "radar"),
                water: has(&|x| x == "surf" || x.ends_with("_rod")),
                cave: has(&|x| x == "cave"),
                gift,
                evolves_only,
                cant_get,
                first_stage: wild.iter().map(|e| w.stage_index(&e.stage)).min().unwrap_or(usize::MAX),
            }
        })
        .collect()
}

/// The usual type colours, a touch softer on light pages.
pub(super) fn type_color(t: &str) -> Color32 {
    let c = match t {
        "Normal" => (168, 167, 122),
        "Fire" => (238, 129, 48),
        "Water" => (99, 144, 240),
        "Electric" => (247, 208, 44),
        "Grass" => (122, 199, 76),
        "Ice" => (150, 217, 214),
        "Fighting" => (194, 46, 40),
        "Poison" => (163, 62, 161),
        "Ground" => (226, 191, 101),
        "Flying" => (169, 143, 243),
        "Psychic" => (249, 85, 135),
        "Bug" => (166, 185, 26),
        "Rock" => (182, 161, 54),
        "Ghost" => (115, 87, 151),
        "Dragon" => (111, 53, 252),
        "Dark" => (112, 87, 70),
        "Steel" => (183, 183, 206),
        "Fairy" => (214, 133, 173),
        _ => (140, 140, 140),
    };
    Color32::from_rgb(c.0, c.1, c.2)
}

/// A small rounded type label.
pub(super) fn type_pill(ui: &mut egui::Ui, t: &str) -> egui::Response {
    type_chip(ui, t, "")
}

/// A type pill with something after the name, e.g. "FIRE  ×2".
pub(super) fn type_chip(ui: &mut egui::Ui, t: &str, extra: &str) -> egui::Response {
    let c = type_color(t);
    let label = if extra.is_empty() { t.to_uppercase() } else { format!("{}  {extra}", t.to_uppercase()) };
    // Dark text on the light types (Electric, Ice, Steel…), white on the rest.
    let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    let ink = if lum > 150.0 { Color32::from_rgb(34, 34, 40) } else { Color32::WHITE };
    let galley = ui.painter().layout_no_wrap(label, kit::font_bold(10.5), ink);
    let (r, resp) = ui.allocate_exact_size(galley.size() + egui::vec2(16.0, 6.0), Sense::click());
    ui.painter().rect_filled(r, r.height() / 2.0, c);
    ui.painter().galley(r.min + egui::vec2(8.0, 3.0), galley, ink);
    resp
}

impl App {
    /// The Pokédex page: the grid, or one Pokémon's details.
    pub(super) fn page_pokedex(&mut self, ui: &mut egui::Ui) {
        let Some(w) = self.wiki.clone() else {
            kit::page_header(ui, "POKÉDEX", "Every Pokémon in Platinum Together.");
            kit::experimental(ui, "The Pokédex");
            kit::pill(ui, "Loading the Pokédex data…", PillKind::Info);
            return;
        };
        if w.is_empty() {
            kit::page_header(ui, "POKÉDEX", "Every Pokémon in Platinum Together.");
            kit::experimental(ui, "The Pokédex");
            kit::card(ui, |ui| {
                ui.label(
                    RichText::new("The Pokédex data hasn't downloaded yet. Check your internet connection; it downloads by itself when it can.")
                        .size(kit::fs(12.5))
                        .color(MUTED()),
                );
            });
            return;
        }
        if let Some(id) = self.setup.pokedex.open.clone() {
            self.pokedex_entry(ui, &w, &id);
            return;
        }
        self.pokedex_list(ui, &w);
    }

    fn pokedex_list(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        // (Re)build the summary when new data arrives.
        let key = Arc::as_ptr(w) as usize;
        if self.setup.pokedex.index.as_ref().map(|(k, _)| *k) != Some(key) {
            self.setup.pokedex.index = Some((key, build_index(w)));
        }
        kit::page_header(
            ui,
            "POKÉDEX",
            &format!("Every Pokémon in Platinum Together. Data from the Platinum Together Wiki (v{}).", w.data.version),
        );
        kit::experimental(ui, "The Pokédex");

        // ── search, sprite style, filters
        let st = &mut self.setup.pokedex;
        ui.horizontal(|ui| {
            kit::search_box(ui, &mut st.search, "Search name or number", 220.0);
            kit::segmented(ui, &mut st.shiny, &[(false, "  Normal  "), (true, "  Shiny  ")]);
            kit::segmented(
                ui,
                &mut st.caught,
                &[(CaughtFilter::All, "  All  "), (CaughtFilter::Caught, "  Shiny caught  "), (CaughtFilter::NotYet, "  Not caught  ")],
            );
        });
        ui.add_space(6.0);
        let stages = w.data.stages.clone();
        let mut types: Vec<String> = Vec::new();
        for m in w.all() {
            for t in &m.types {
                if !types.contains(t) {
                    types.push(t.clone());
                }
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            egui::ComboBox::from_id_salt("pdx-type")
                .selected_text(st.type_filter.clone().unwrap_or_else(|| "Any type".to_string()))
                .width(130.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut st.type_filter, None, "Any type");
                    for t in &types {
                        ui.selectable_value(&mut st.type_filter, Some(t.clone()), t.as_str());
                    }
                });
            egui::ComboBox::from_id_salt("pdx-how")
                .selected_text(st.how.label())
                .width(170.0)
                .show_ui(ui, |ui| {
                    for h in HowFilter::ALL {
                        ui.selectable_value(&mut st.how, h, h.label());
                    }
                });
            egui::ComboBox::from_id_salt("pdx-stage")
                .selected_text(match st.stage.and_then(|i| stages.get(i)) {
                    Some(s) => format!("Wild by: {s}"),
                    None => "Any point in the game".to_string(),
                })
                .width(220.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut st.stage, None, "Any point in the game");
                    for (i, s) in stages.iter().enumerate() {
                        ui.selectable_value(&mut st.stage, Some(i), format!("Wild by: {s}"));
                    }
                })
                .response
                .on_hover_text("Only Pokémon found in the wild by this point (badge stage)");
            kit::segmented(
                ui,
                &mut st.era,
                &[(EraFilter::All, "  All  "), (EraFilter::Platinum, "  Platinum  "), (EraFilter::Modern, "  Modern  ")],
            );
        });
        ui.add_space(10.0);

        // ── filter
        let caught = self.dex_caught();
        let st = &self.setup.pokedex;
        let q = st.search.trim().to_lowercase();
        let qnum = q.trim_start_matches('#').trim_start_matches('0').to_string();
        let list: Vec<MonInfo> = st
            .index
            .as_ref()
            .map(|(_, v)| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .filter(|m| q.is_empty() || m.name.to_lowercase().contains(&q) || (!qnum.is_empty() && m.dex.to_string() == qnum))
            .filter(|m| st.type_filter.as_ref().map_or(true, |t| m.types.contains(t)))
            .filter(|m| match st.how {
                HowFilter::Any => true,
                HowFilter::Grass => m.grass,
                HowFilter::Radar => m.radar,
                HowFilter::Water => m.water,
                HowFilter::Cave => m.cave,
                HowFilter::Gift => m.gift,
                HowFilter::Evolve => m.evolves_only,
                HowFilter::CantGet => m.cant_get,
            })
            .filter(|m| st.stage.map_or(true, |s| m.first_stage <= s))
            .filter(|m| match st.era {
                EraFilter::All => true,
                EraFilter::Platinum => !m.modern,
                EraFilter::Modern => m.modern,
            })
            .filter(|m| {
                let has = self.species.get((m.dex as usize).wrapping_sub(1)).is_some_and(|sp| caught.contains_key(&sp.slug));
                match st.caught {
                    CaughtFilter::All => true,
                    CaughtFilter::Caught => has,
                    CaughtFilter::NotYet => !has,
                }
            })
            .cloned()
            .collect();
        let shiny = st.shiny;
        let ids: Vec<String> = list.iter().map(|m| m.id.clone()).collect();
        self.setup.pokedex.list_ids = ids;
        ui.label(
            RichText::new(format!("{} Pokémon", list.len()))
                .size(kit::fs(12.0))
                .color(MUTED()),
        );
        ui.add_space(6.0);
        if list.is_empty() {
            ui.label(RichText::new("Nothing matches.").size(kit::fs(12.5)).color(MUTED()));
            return;
        }

        // ── grid (same tiles as the Shiny Dex)
        let gap = TILE_GAP;
        let (cols, tw) = grid_cols(ui.available_width());
        let tile = egui::vec2(tw, TILE_H);
        let mut open: Option<String> = None;
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for m in row {
                    let (r, resp) = ui.allocate_exact_size(tile, Sense::click());
                    if !ui.is_rect_visible(r) {
                        continue;
                    }
                    let Some(sp) = self.species.get((m.dex as usize).wrapping_sub(1)).cloned() else { continue };
                    let has = caught.contains_key(&sp.slug);
                    let hovered = resp.hovered();
                    let p = ui.painter_at(r.expand(2.0));
                    let look = TileLook {
                        fill: if hovered { CARD_HOVER() } else { CARD() },
                        stroke: Stroke::new(1.0f32, if hovered { theme::LINK() } else if has { GOLD.gamma_multiply(0.7) } else { LINE() }),
                        name: TEXT(),
                        num: MUTED(),
                        tint: if m.cant_get { Color32::from_white_alpha(110) } else { Color32::WHITE },
                        shiny,
                    };
                    self.grid_tile(&p, r, &sp, &look);
                    // Type dots, top left.
                    for (k, t) in m.types.iter().enumerate() {
                        p.circle_filled(r.min + egui::vec2(10.0 + k as f32 * 11.0, 10.0), 4.0, type_color(t));
                    }
                    if has {
                        theme::sparkle(&p, r.right_top() + egui::vec2(-9.0, 9.0), 4.5, GOLD);
                    }
                    if m.modern {
                        p.text(r.right_bottom() + egui::vec2(-7.0, -24.0), Align2::RIGHT_CENTER, "NEW", kit::font_bold(8.5), MUTED());
                    }
                    let mut tip = format!("#{:04} {}  ·  {}", m.dex, m.name, m.types.join(" / "));
                    if m.cant_get {
                        tip.push_str("\nCan't be got in the game yet");
                    }
                    if has {
                        tip.push_str("\nShiny caught ✨");
                    }
                    tip.push_str("\nClick for details");
                    if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some(m.id.clone());
                    }
                }
            });
            ui.add_space(gap);
        }
        if let Some(id) = open {
            self.setup.pokedex.open = Some(id);
            self.setup.pokedex.tab = EntryTab::Overview;
        }
    }

    /// One Pokémon's page: header, actions and tabs.
    fn pokedex_entry(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, id: &str) {
        let Some(m) = w.mon(id).cloned() else {
            self.setup.pokedex.open = None;
            return;
        };
        let sp = self.species.get((m.dex as usize).wrapping_sub(1)).cloned();

        // ── back, previous / next
        let mut ids = self.setup.pokedex.list_ids.clone();
        if !ids.iter().any(|x| x == id) {
            ids = w.all().map(|m| m.id.clone()).collect();
        }
        let pos = ids.iter().position(|x| x == id);
        let prev = pos.and_then(|i| i.checked_sub(1)).and_then(|i| ids.get(i)).and_then(|x| w.mon(x)).cloned();
        let next = pos.and_then(|i| ids.get(i + 1)).and_then(|x| w.mon(x)).cloned();
        let mut go: Option<String> = None;
        let mut back = false;
        ui.horizontal(|ui| {
            back = super::hunt_pages::back_button(ui, "Pokédex");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let nav = |ui: &mut egui::Ui, text: String| {
                    ui.add(egui::Button::new(RichText::new(text).size(kit::fs(12.0)).color(MUTED())).frame(false))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                };
                if let Some(n) = &next {
                    if nav(ui, format!("#{:04} {}  ›", n.dex, n.name)).on_hover_text("Next (→ key)").clicked() {
                        go = Some(n.id.clone());
                    }
                    ui.add_space(14.0);
                }
                if let Some(p) = &prev {
                    if nav(ui, format!("‹  #{:04} {}", p.dex, p.name)).on_hover_text("Previous (← key)").clicked() {
                        go = Some(p.id.clone());
                    }
                }
            });
        });
        let typing = ui.ctx().wants_keyboard_input();
        let (left, right, esc) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::Escape)
                    || (!typing && i.key_pressed(egui::Key::Backspace))
                    || i.pointer.button_pressed(egui::PointerButton::Extra1),
            )
        });
        if esc {
            back = true;
        }
        if left && !ui.ctx().wants_keyboard_input() {
            go = prev.as_ref().map(|p| p.id.clone()).or(go);
        }
        if right && !ui.ctx().wants_keyboard_input() {
            go = next.as_ref().map(|n| n.id.clone()).or(go);
        }
        if back {
            self.setup.pokedex.open = None;
            return;
        }
        if let Some(g) = go {
            self.setup.pokedex.open = Some(g);
            return;
        }

        // pokemondb facts (fetched the first time, then cached)
        let extra = match &sp {
            Some(sp) => self.dexdb.get(&sp.slug),
            None => crate::dexextra::State::Failed("unknown Pokémon".into()),
        };
        if matches!(extra, crate::dexextra::State::Loading) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
        let ready = match &extra {
            crate::dexextra::State::Ready(e) => Some(e.as_ref().clone()),
            _ => None,
        };
        // Get the neighbours ready too, so previous / next open straight away.
        if !matches!(extra, crate::dexextra::State::Loading) {
            for n in [&prev, &next].into_iter().flatten() {
                if let Some(nsp) = self.species.get((n.dex as usize).wrapping_sub(1)) {
                    let slug = nsp.slug.clone();
                    self.dexdb.get(&slug);
                }
            }
        }

        ui.add_space(6.0);
        self.entry_header(ui, w, &m, sp.as_ref(), ready.as_ref());
        ui.add_space(10.0);
        kit::tab_bar(
            ui,
            &mut self.setup.pokedex.tab,
            &[
                (EntryTab::Overview, "Overview"),
                (EntryTab::Evolution, "Evolution"),
                (EntryTab::Where, "Where to find"),
                (EntryTab::Moves, "Moves"),
            ],
        );
        ui.add_space(10.0);
        let tab = self.setup.pokedex.tab;
        match tab {
            EntryTab::Overview => self.entry_overview(ui, &m, sp.as_ref(), &extra),
            EntryTab::Evolution => self.entry_evolution(ui, w, &m),
            EntryTab::Where => self.entry_where(ui, w, &m),
            EntryTab::Moves => self.entry_moves(ui, w, &m),
        }
    }

    /// Big sprite, name, types, badges and the action buttons.
    fn entry_header(
        &mut self,
        ui: &mut egui::Ui,
        w: &Arc<Wiki>,
        m: &crate::wiki::Mon,
        sp: Option<&crate::species::Species>,
        extra: Option<&crate::dexextra::Extra>,
    ) {
        let caught = sp.is_some_and(|sp| self.dex_caught().contains_key(&sp.slug));
        let cant_get = w.unobtainable(&m.id).map(|s| s.to_string());
        let hunting = sp.is_some_and(|sp| self.cfg.target.as_deref() == Some(sp.slug.as_str()));
        let mut hunt = false;
        let mut show_map: Option<Vec<String>> = None;
        let rare_spots = sp.map(|s| self.rare_spots_for(&s.slug)).unwrap_or_default();
        let mut rare_hunt = false;
        let mut rare_show = false;
        let map_ok = self.gamemap.as_ref().is_some_and(|r| r.is_ok());
        let mut show_type: Option<String> = None;
        let mut toggle = false;
        let mut open_wiki = false;
        let shiny = self.setup.pokedex.shiny;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(140.0, 140.0), Sense::click());
                let p = ui.painter_at(r);
                p.rect_filled(r, kit::r(12.0), CARD_HOVER());
                if let Some(sp) = sp {
                    self.draw_sprite(&p, sp, shiny, r.center_bottom() - egui::vec2(0.0, 20.0), egui::vec2(128.0, 112.0));
                }
                if shiny {
                    theme::sparkle(&p, r.right_top() + egui::vec2(-12.0, 12.0), 5.5, GOLD);
                }
                p.text(
                    r.center_bottom() - egui::vec2(0.0, 2.0),
                    Align2::CENTER_BOTTOM,
                    if shiny { "shiny" } else { "normal" },
                    kit::font_small(9.5),
                    MUTED(),
                );
                if resp
                    .on_hover_text(if shiny { "Click to see the normal sprite" } else { "Click to see the shiny sprite" })
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    toggle = true;
                }
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("#{:04}", m.dex)).size(kit::fs(13.0)).color(MUTED()));
                    ui.label(RichText::new(&m.name).size(kit::fs(24.0)).family(kit::strong_family(kit::fs(24.0))).strong().color(TEXT()));
                    if let Some(e) = extra.filter(|e| !e.category.is_empty()) {
                        ui.label(RichText::new(&e.category).size(kit::fs(12.5)).color(MUTED()));
                    }
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        for t in &m.types {
                            if type_pill(ui, t)
                                .on_hover_text(format!("Show every {t}-type Pokémon"))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                show_type = Some(t.clone());
                            }
                        }
                        if m.modern {
                            kit::pill(ui, "Modern", PillKind::Info).on_hover_text("Added by the MODERN POKéMON option");
                        }
                        if caught {
                            kit::badge(ui, "✨ Shiny caught", true);
                        }
                        if let Some(why) = &cant_get {
                            let why = if why.trim().is_empty() { "Can't be got in the game yet" } else { why.as_str() };
                            kit::pill(ui, "Can't get yet", PillKind::Warn).on_hover_text(why);
                        }
                    });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if let Some(sp) = sp {
                            let label = if hunting { "Go to my hunt" } else { "Hunt this shiny" };
                            let b = egui::Button::new(RichText::new(label).family(kit::strong_family(13.5)).strong().color(kit::on_primary()));
                            let tip = if hunting {
                                format!("You're hunting {} now", m.name)
                            } else {
                                let mut t = format!(
                                    "Make {} your Shiny Hunter target. Your current hunt is saved, so you can switch back to it from Runs.",
                                    sp.name
                                );
                                if cant_get.is_some() {
                                    t.push_str("\n\nHeads up: the wiki says you can't get this one in the game yet.");
                                }
                                t
                            };
                            if kit::accent_button(ui, b, 6.0).on_hover_text(tip).clicked() {
                                hunt = true;
                            }
                        }
                        let secondary = |t: &str| {
                            egui::Button::new(RichText::new(t).color(TEXT())).fill(kit::btn_fill()).rounding(kit::btn_round(6.0))
                        };
                        let wild_places: Vec<String> = {
                            let mut v: Vec<String> = Vec::new();
                            for e in w.wild(&m.id) {
                                if e.setting.is_empty() && !v.contains(&e.area) {
                                    v.push(e.area.clone());
                                }
                            }
                            v
                        };
                        let can_map = map_ok && !wild_places.is_empty();
                        if ui
                            .add_enabled(can_map, secondary("Show on map"))
                            .on_hover_text("Every place it lives in the wild, flashing on the Map")
                            .on_disabled_hover_text(if map_ok { "Not found in the wild" } else { "The Map needs Platinum Together installed on this PC" })
                            .clicked()
                        {
                            show_map = Some(wild_places);
                        }
                        if ui.add(secondary("Open on the wiki  ↗")).on_hover_text(w.page_url(m)).clicked() {
                            open_wiki = true;
                        }
                    });
                    // On rare grass today: hunt it there.
                    if !rare_spots.is_empty() {
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!("✨ On rare grass today: {}", super::rare::places_text(&rare_spots)))
                                    .size(kit::fs(12.5))
                                    .family(kit::strong_family(kit::fs(12.5))).strong()
                                    .color(GOLD),
                            );
                            let b = egui::Button::new(RichText::new("Hunt on rare grass").family(kit::strong_family(13.5)).strong().color(Color32::from_rgb(40, 30, 6)))
                                .fill(GOLD)
                                .rounding(kit::btn_round(6.0));
                            if ui
                                .add(b)
                                .on_hover_text("Make it your target and hunt it with Sweet Scent on today's rare grass")
                                .clicked()
                            {
                                rare_hunt = true;
                            }
                            if ui.link(RichText::new("Show on map").size(kit::fs(12.0))).clicked() {
                                rare_show = true;
                            }
                        });
                    }
                });
            });
        });
        if rare_hunt {
            if let Some(sp) = sp {
                self.hunt_on_rare_grass(&sp.slug, sp.name, &rare_spots);
            }
        } else if rare_show {
            if let Some(s) = rare_spots.first() {
                self.go_to_rare(s);
            }
        }
        if toggle {
            self.setup.pokedex.shiny = !shiny;
        }
        if let Some(places) = show_map {
            self.show_on_map(&places, m.name.clone(), None);
        }
        if let Some(t) = show_type {
            let st = &mut self.setup.pokedex;
            st.type_filter = Some(t);
            st.search.clear();
            st.open = None;
            st.grid_gen += 1;
        }
        if open_wiki {
            ui.ctx().open_url(egui::OpenUrl::new_tab(w.page_url(m)));
        }
        if hunt {
            if let Some(sp) = sp {
                self.set_target(sp.slug.clone());
                self.setup.tab = super::Tab::Shiny;
                self.setup.nav_collapsed.retain(|t| *t != super::Tab::Shiny);
            }
        }
    }

    /// Overview tab: Pokédex data, training, breeding, base stats and the
    /// type chart.
    fn entry_overview(
        &mut self,
        ui: &mut egui::Ui,
        m: &crate::wiki::Mon,
        sp: Option<&crate::species::Species>,
        extra: &crate::dexextra::State,
    ) {
        use crate::dexextra::State;
        let e = match extra {
            State::Ready(e) => Some(e.as_ref()),
            _ => None,
        };
        match extra {
            State::Loading => {
                kit::pill(ui, "Getting the details from pokemondb.net…", PillKind::Info);
                ui.add_space(8.0);
            }
            State::Failed(err) => {
                let mut retry = false;
                kit::card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Couldn't get the details from pokemondb.net. Check your internet connection.")
                                .size(kit::fs(12.5))
                                .color(MUTED()),
                        )
                        .on_hover_text(err);
                        retry = ui.button("Try again").clicked();
                    });
                });
                if retry {
                    if let Some(sp) = sp {
                        self.dexdb.retry(&sp.slug);
                    }
                }
                ui.add_space(8.0);
            }
            State::Ready(_) => {}
        }
        let dash = |s: Option<&str>| match s {
            Some(v) if !v.is_empty() => v.to_string(),
            Some(_) => "—".to_string(),
            None => "…".to_string(),
        };
        let mut abilities: Vec<String> = Vec::new();
        if let Some(e) = e {
            for (i, a) in e.abilities.iter().enumerate() {
                abilities.push(format!("{}. {a}", i + 1));
            }
        }
        let dex_rows: Vec<(&str, String)> = vec![
            ("National №", format!("#{:04}", m.dex)),
            ("Type", m.types.join(" / ")),
            ("Species", dash(e.map(|e| e.category.as_str()))),
            ("Height", dash(e.map(|e| e.height.as_str()))),
            ("Weight", dash(e.map(|e| e.weight.as_str()))),
            ("Abilities", if e.is_some() { if abilities.is_empty() { "—".into() } else { abilities.join("\n") } } else { "…".into() }),
            (
                "Hidden ability",
                match e {
                    Some(e) if !e.hidden_ability.is_empty() => format!("{} (newer games)", e.hidden_ability),
                    Some(_) => "—".into(),
                    None => "…".into(),
                },
            ),
        ];
        let train_rows: Vec<(&str, String)> = vec![
            ("EV yield", dash(e.map(|e| e.ev_yield.as_str()))),
            ("Catch rate", dash(e.map(|e| e.catch_rate.as_str()))),
            ("Base Friendship", dash(e.map(|e| e.base_friendship.as_str()))),
            ("Base Exp.", dash(e.map(|e| e.base_exp.as_str()))),
            ("Growth Rate", dash(e.map(|e| e.growth_rate.as_str()))),
        ];
        let egg = e.map(|e| e.egg_groups.join(", "));
        let breed_rows: Vec<(&str, String)> = vec![
            ("Egg Groups", dash(egg.as_deref())),
            ("Gender", dash(e.map(|e| e.gender.as_str()))),
            ("Egg cycles", dash(e.map(|e| e.egg_cycles.as_str()))),
        ];

        let left = |ui: &mut egui::Ui| {
            fact_card(ui, "Pokédex data", &dex_rows);
            ui.add_space(10.0);
            fact_card(ui, "Training", &train_rows);
            ui.add_space(10.0);
            fact_card(ui, "Breeding", &breed_rows);
        };
        let right = |ui: &mut egui::Ui| {
            stats_card(ui, m, e);
            ui.add_space(10.0);
            defenses_card(ui, m);
        };
        if ui.available_width() >= 640.0 {
            ui.columns(2, |cols| {
                left(&mut cols[0]);
                right(&mut cols[1]);
            });
        } else {
            left(ui);
            ui.add_space(10.0);
            right(ui);
        }

        // Where it came from.
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let small = |t: &str| RichText::new(t).size(kit::fs(11.0)).color(MUTED());
            ui.label(small("Name, number and types from the Platinum Together Wiki. Everything else from"));
            if let Some(sp) = sp {
                let url = crate::dexextra::Extra::page_url(&sp.slug);
                if ui
                    .add(egui::Button::new(RichText::new("pokemondb.net ↗").size(kit::fs(11.0)).color(theme::LINK())).frame(false))
                    .on_hover_text(&url)
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                }
            }
            if let Some(e) = e.filter(|e| !e.platinum_notes.is_empty()) {
                ui.label(small(&format!("(Platinum values used for: {}.)", e.platinum_notes.join(", ").to_lowercase())));
            }
        });
    }

    /// Evolution tab: the family as a chart, then how each step happens.
    fn entry_evolution(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, m: &crate::wiki::Mon) {
        let family = w.family(&m.id);
        if family.len() <= 1 {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                card_title(ui, "Evolution");
                ui.label(RichText::new(format!("{} doesn't evolve.", m.name)).size(kit::fs(12.5)).color(MUTED()));
            });
            return;
        }
        let caught = self.dex_caught();
        let shiny = self.setup.pokedex.shiny;
        let mut open: Option<String> = None;

        // ── chart: stages left to right, branches stacked (4 to a row when there are lots)
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            card_title(ui, "Evolution chart");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 10.0);
                for (si, stage) in family.iter().enumerate() {
                    if si > 0 {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(26.0, EVO_CHIP + TILE_H), Sense::hover());
                        ui.painter().text(
                            egui::pos2(r.center().x, r.min.y + EVO_CHIP + TILE_H / 2.0 - 10.0),
                            Align2::CENTER_CENTER,
                            "›",
                            kit::font_bold(26.0),
                            MUTED(),
                        );
                    }
                    let per_row = if stage.len() > 3 { 4 } else { 1 };
                    ui.vertical(|ui| {
                        for row in stage.chunks(per_row) {
                            ui.horizontal(|ui| {
                                for id in row {
                                    let how = if si > 0 { w.evolves_from(id).map(|e| e.method.clone()) } else { None };
                                    if self.evo_tile(ui, w, id, how.as_deref(), id == &m.id, shiny, &caught) {
                                        open = Some(id.clone());
                                    }
                                }
                            });
                        }
                    });
                }
            });
            ui.add_space(2.0);
            ui.label(RichText::new("Click one to open it.").size(kit::fs(11.0)).color(MUTED()));
        });
        ui.add_space(10.0);

        // ── the full wording from the wiki, for every step in the family
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            card_title(ui, "How they evolve");
            let mut first = true;
            for stage in family.iter().skip(1) {
                for id in stage {
                    let Some(e) = w.evolves_from(id) else { continue };
                    let name = |x: &str| w.mon(x).map(|m| m.name.clone()).unwrap_or_else(|| x.to_string());
                    let mine = e.species == m.id || e.from == m.id;
                    if !first {
                        ui.add_space(4.0);
                        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
                        ui.painter().rect_filled(r, 0.0, LINE());
                        ui.add_space(4.0);
                    }
                    first = false;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{}  →  {}", name(&e.from), name(&e.species)))
                                .size(kit::fs(12.5))
                                .family(kit::strong_family(kit::fs(12.5))).strong()
                                .color(if mine { TEXT() } else { MUTED() }),
                        );
                        ui.label(RichText::new(short_method(&e.method)).size(kit::fs(11.0)).color(theme::LINK()));
                    });
                    ui.add(
                        egui::Label::new(RichText::new(&e.method).size(kit::fs(12.0)).color(if mine { TEXT() } else { MUTED() }))
                            .wrap(),
                    );
                }
            }
        });
        if let Some(id) = open {
            if id != m.id {
                self.setup.pokedex.open = Some(id);
            }
        }
    }

    /// One Pokémon in the evolution chart, with how it evolves above it.
    /// Returns true when clicked.
    #[allow(clippy::too_many_arguments)]
    fn evo_tile(
        &mut self,
        ui: &mut egui::Ui,
        w: &Arc<Wiki>,
        id: &str,
        how: Option<&str>,
        current: bool,
        shiny: bool,
        caught: &std::collections::BTreeMap<String, super::extras::DexSource>,
    ) -> bool {
        let (r, resp) = ui.allocate_exact_size(egui::vec2(EVO_TILE_W, EVO_CHIP + TILE_H), Sense::click());
        if !ui.is_rect_visible(r) {
            return false;
        }
        let Some(mon) = w.mon(id) else { return false };
        let tile = egui::Rect::from_min_size(r.min + egui::vec2(0.0, EVO_CHIP), egui::vec2(EVO_TILE_W, TILE_H));
        let p = ui.painter_at(r.expand(2.0));
        // how it evolves
        if let Some(how) = how {
            let label = short_method(how);
            let galley = p.layout_no_wrap(label, kit::font_small(10.0), TEXT());
            let wdt = (galley.size().x + 14.0).min(EVO_TILE_W);
            let chip = egui::Rect::from_center_size(egui::pos2(r.center().x, r.min.y + 9.0), egui::vec2(wdt, 18.0));
            p.rect_filled(chip, 9.0, CARD_HOVER());
            p.with_clip_rect(chip.shrink2(egui::vec2(6.0, 0.0))).galley(
                egui::pos2(chip.center().x - galley.size().x.min(wdt - 12.0) / 2.0, chip.center().y - galley.size().y / 2.0),
                galley,
                TEXT(),
            );
        }
        let has_sp = self.species.get((mon.dex as usize).wrapping_sub(1)).cloned();
        let hovered = resp.hovered();
        if let Some(sp) = has_sp {
            let has = caught.contains_key(&sp.slug);
            let look = TileLook {
                fill: if hovered || current { CARD_HOVER() } else { CARD() },
                stroke: Stroke::new(
                    if current { 2.0f32 } else { 1.0 },
                    if current || hovered { theme::LINK() } else if has { GOLD.gamma_multiply(0.7) } else { LINE() },
                ),
                name: TEXT(),
                num: MUTED(),
                tint: if w.unobtainable(id).is_some() { Color32::from_white_alpha(110) } else { Color32::WHITE },
                shiny,
            };
            self.grid_tile(&p, tile, &sp, &look);
            for (k, t) in mon.types.iter().enumerate() {
                p.circle_filled(tile.min + egui::vec2(10.0 + k as f32 * 11.0, 10.0), 4.0, type_color(t));
            }
            if has {
                theme::sparkle(&p, tile.right_top() + egui::vec2(-9.0, 9.0), 4.5, GOLD);
            }
            if mon.modern {
                p.text(tile.right_bottom() + egui::vec2(-7.0, -24.0), Align2::RIGHT_CENTER, "NEW", kit::font_bold(8.5), MUTED());
            }
        }
        let mut tip = format!("#{:04} {}", mon.dex, mon.name);
        if let Some(how) = how {
            tip.push_str("\n");
            tip.push_str(how);
        }
        if !current {
            tip.push_str("\nClick to open");
        }
        resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !current
    }
}

const EVO_TILE_W: f32 = 120.0;
/// Room above each tile for the "Lv. 16" chip.
const EVO_CHIP: f32 = 24.0;

pub(super) fn card_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(kit::fs(13.5)).family(kit::strong_family(kit::fs(13.5))).strong().color(TEXT()));
    ui.add_space(6.0);
}

/// A titled card of label → value rows.
fn fact_card(ui: &mut egui::Ui, title: &str, rows: &[(&str, String)]) {
    kit::card(ui, |ui| {
        ui.set_width(ui.available_width());
        card_title(ui, title);
        egui::Grid::new(("pdx-facts", title)).num_columns(2).min_col_width(104.0).spacing([12.0, 7.0]).show(ui, |ui| {
            for (label, value) in rows {
                ui.label(RichText::new(*label).size(kit::fs(12.0)).color(MUTED()));
                ui.add(egui::Label::new(RichText::new(value).size(kit::fs(12.5)).color(TEXT())).wrap());
                ui.end_row();
            }
        });
    });
}

fn stat_color(v: u32) -> Color32 {
    match v {
        0..=49 => Color32::from_rgb(232, 98, 82),
        50..=79 => Color32::from_rgb(240, 160, 64),
        80..=99 => Color32::from_rgb(232, 204, 72),
        100..=119 => Color32::from_rgb(132, 204, 92),
        _ => Color32::from_rgb(72, 196, 168),
    }
}

/// Base stats as bars, with the total (BST).
fn stats_card(ui: &mut egui::Ui, m: &crate::wiki::Mon, e: Option<&crate::dexextra::Extra>) {
    kit::card(ui, |ui| {
        ui.set_width(ui.available_width());
        card_title(ui, "Base stats");
        let Some(e) = e.filter(|e| e.stats.len() == 6) else {
            ui.label(RichText::new("…").color(MUTED()));
            return;
        };
        let names = ["HP", "Attack", "Defense", "Sp. Atk", "Sp. Def", "Speed"];
        for (i, (n, v)) in names.iter().zip(&e.stats).enumerate() {
            let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), Sense::hover());
            let p = ui.painter_at(r);
            p.text(r.left_center(), Align2::LEFT_CENTER, *n, kit::font(12.0), MUTED());
            p.text(r.left_center() + egui::vec2(96.0, 0.0), Align2::RIGHT_CENTER, v.to_string(), kit::font_bold(12.5), TEXT());
            let bar_x = r.left() + 106.0;
            let full = (r.right() - bar_x).max(10.0);
            let track = egui::Rect::from_min_max(egui::pos2(bar_x, r.center().y - 4.0), egui::pos2(r.right(), r.center().y + 4.0));
            p.rect_filled(track, 4.0, CARD_HOVER());
            let w = full * (*v as f32 / 200.0).clamp(0.02, 1.0);
            p.rect_filled(egui::Rect::from_min_size(track.min, egui::vec2(w, 8.0)), 4.0, stat_color(*v));
            if let Some(old) = e.modern_stats.get(i).filter(|o| **o != *v) {
                resp.on_hover_text(format!("{old} in newer games"));
            }
            ui.add_space(2.0);
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Total").size(kit::fs(12.0)).color(MUTED()));
            ui.label(RichText::new(e.total.to_string()).size(kit::fs(14.0)).family(kit::strong_family(kit::fs(14.0))).strong().color(TEXT()));
        });
        let note = if !e.modern_stats.is_empty() {
            let old: u32 = e.modern_stats.iter().sum();
            format!("As in Platinum. Newer games: {old} total.")
        } else if m.modern {
            "As in the latest games.".to_string()
        } else {
            "Same as in Platinum.".to_string()
        };
        ui.label(RichText::new(note).size(kit::fs(11.0)).color(MUTED()));
    });
}

/// What hits it hard and what it shrugs off, from the wiki's types.
fn defenses_card(ui: &mut egui::Ui, m: &crate::wiki::Mon) {
    kit::card(ui, |ui| {
        ui.set_width(ui.available_width());
        card_title(ui, "Type defenses");
        let fx = crate::dexextra::defenses(&m.types);
        let group = |ui: &mut egui::Ui, label: &str, keep: &dyn Fn(u32) -> bool| {
            let list: Vec<&(&str, u32)> = fx.iter().filter(|(_, x)| keep(*x)).collect();
            if list.is_empty() {
                return;
            }
            ui.label(RichText::new(label).size(kit::fs(12.0)).color(MUTED()));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                for (t, x) in list {
                    let mult = match x {
                        400 => "×4",
                        200 => "×2",
                        50 => "×½",
                        25 => "×¼",
                        _ => "×0",
                    };
                    type_chip(ui, t, mult);
                }
            });
            ui.add_space(6.0);
        };
        group(ui, "Weak to", &|x| x > 100);
        group(ui, "Resists", &|x| x > 0 && x < 100);
        group(ui, "Immune to", &|x| x == 0);
        ui.label(
            RichText::new("Damage taken from each attacking type. Uses Platinum Together's types for this Pokémon.")
                .size(kit::fs(11.0))
                .color(MUTED()),
        );
    });
}

/// A few words for an evolution method, for the chart ("Lv. 16", "Moon Stone").
/// The full wiki sentence is shown under the chart.
pub(super) fn short_method(method: &str) -> String {
    let m = method.trim();
    let first = m.split(". ").next().unwrap_or(m).trim_end_matches('.');
    let num_after = |s: &str, key: &str| -> Option<u32> {
        let i = s.find(key)? + key.len();
        s[i..].trim_start().split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
    };
    let time = if m.contains("during the day") {
        " (day)"
    } else if m.contains("at night") {
        " (night)"
    } else {
        ""
    };
    // "the Ice Rock" / "the Moss Rock"
    if let Some(i) = first.find("(the ") {
        let rest = &first[i + 5..];
        if let Some(j) = rest.find(')') {
            if rest[..j].ends_with("Rock") {
                return rest[..j].to_string();
            }
        }
    }
    if first.starts_with("No way yet") {
        return "Can't yet".into();
    }
    if m.contains("Mt. Coronet") {
        return "Mt. Coronet".into();
    }
    if let Some(rest) = first.strip_prefix("Use a ").or_else(|| first.strip_prefix("Use an ")) {
        let item = rest.split(" on ").next().unwrap_or(rest);
        let who = if rest.contains("male one") && !rest.contains("female one") {
            " ♂"
        } else if rest.contains("female one") {
            " ♀"
        } else {
            ""
        };
        return format!("{item}{who}");
    }
    if let Some(rest) = first.strip_prefix("Trade it while it holds ") {
        let item = rest.trim_start_matches("a ").trim_start_matches("an ");
        return format!("Hold {item}");
    }
    if first == "Trade it" {
        return match num_after(m, "at Lv.") {
            Some(n) => format!("Trade or Lv. {n}"),
            None => "Trade".into(),
        };
    }
    if first.starts_with("Shows up when") {
        return match num_after(first, "Level") {
            Some(n) => format!("Lv. {n}, spare slot"),
            None => "Spare slot".into(),
        };
    }
    if first.contains("friendship") {
        let extra = if m.contains("Fairy-type move") { " + Fairy" } else { "" };
        return format!("Friendship{extra}{time}");
    }
    if m.contains("Beauty") {
        return "Beauty".into();
    }
    if let Some(rest) = first.strip_prefix("Level up while it knows ") {
        return format!("Knows {rest}");
    }
    if let Some(rest) = first.strip_prefix("Level up while holding ") {
        let item = rest.trim_start_matches("a ").trim_start_matches("an ");
        let item = item.split(" during").next().unwrap_or(item).split(" at night").next().unwrap_or(item);
        return format!("Hold {item}{time}");
    }
    if let Some(rest) = first.strip_prefix("Level up with a ").or_else(|| first.strip_prefix("Level up with an ")) {
        return format!("{}", rest.replace(" in the party", " in party"));
    }
    if first.starts_with("Level") {
        if let Some(n) = num_after(first, "Level") {
            let mut s = format!("Lv. {n}");
            if first.contains("female only") {
                s.push_str(" ♀");
            } else if first.contains("male only") {
                s.push_str(" ♂");
            } else if first.contains("higher than Defense") {
                s.push_str(", Atk > Def");
            } else if first.contains("lower than Defense") {
                s.push_str(", Atk < Def");
            } else if first.contains("equal to Defense") {
                s.push_str(", Atk = Def");
            } else if first.contains("hidden value") {
                s.push_str(" (random)");
            } else if first.contains("raining") {
                s.push_str(" (rain)");
            }
            s.push_str(time);
            return s;
        }
    }
    let mut s: String = first.chars().take(22).collect();
    if first.chars().count() > 22 {
        s.push('…');
    }
    s
}
