//! The Pokédex app: every Pokémon in Platinum Together from the wiki data,
//! in a sprite grid like the Shiny Dex. Clicking one opens its details.


use super::ui_kit::{self as kit, PillKind, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::theme::{self, GOLD};
use crate::wiki::Wiki;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};
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
    /// Only this region.
    pub region: Option<crate::moddex::Region>,
    /// Grid order: 0 number, 1 name, 2 most met.
    pub sort: u8,
    /// Shiny Dex mode: also list Pokémon that can't be got yet.
    pub show_unobtainable: bool,
}

/// What a grid tile shows.
struct TileMon {
    dex: u32,
    name: String,
    types: Vec<String>,
    modern: bool,
    cant_get: bool,
}

/// The tabs on a Pokémon's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum EntryTab {
    #[default]
    Overview,
    Evolution,
    Where,
    Moves,
    /// Its trading cards.
    Tcg,
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
        // Shiny Dex mode: a caught shiny opens its trophy page
        if self.setup.pokedex.shiny {
            if let Some(slug) = self.setup.dex.open.clone() {
                self.page_dex_entry(ui, &slug);
                return;
            }
        }
        self.pokedex_list(ui, &w);
    }

    fn pokedex_list(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        // (Re)build the summary when new data arrives.
        let key = Arc::as_ptr(w) as usize;
        if self.setup.pokedex.index.as_ref().map(|(k, _)| *k) != Some(key) {
            self.setup.pokedex.index = Some((key, build_index(w)));
        }
        if self.setup.pokedex.shiny {
            kit::page_header(ui, "POKÉDEX", "Your Shiny Dex: every shiny you've caught. Click a caught one for its trophy; right-click to mark one you caught before.");
        } else {
            kit::page_header(ui, "POKÉDEX", &format!("Every Pokémon in Platinum Together (Platinum Together Wiki v{}). Click one for everything about it.", w.data.version));
        }
        let Some((ikey, index)) = self.setup.pokedex.index.take() else { return };
        let caught = self.dex_caught();
        let met = self.species_met();
        let rare: std::collections::BTreeSet<String> = self.rare_spots_today().into_iter().map(|s| s.species.slug).collect();
        let mut targets: Vec<String> = self.cfg.target.iter().cloned().collect();
        targets.extend(self.also_slugs());

        self.dex_progress_card(ui, &index, &caught);
        ui.add_space(16.0);
        self.dex_toolbar(ui, w, &index);
        ui.add_space(14.0);

        // ── filter and sort
        let st = &self.setup.pokedex;
        let q = st.search.trim().to_lowercase();
        let qnum = q.trim_start_matches('#').trim_start_matches('0').to_string();
        let slug_of = |m: &MonInfo| self.species.get((m.dex as usize).wrapping_sub(1)).map(|s| s.slug.clone()).unwrap_or_default();
        let mut list: Vec<&MonInfo> = index
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
            .filter(|m| st.region.map_or(true, |r| crate::moddex::Region::of(m.dex as u16, m.modern) == r))
            .filter(|m| !st.shiny || !m.cant_get || st.show_unobtainable || st.how == HowFilter::CantGet)
            .filter(|m| {
                let has = caught.contains_key(&slug_of(m));
                match st.caught {
                    CaughtFilter::All => true,
                    CaughtFilter::Caught => has,
                    CaughtFilter::NotYet => !has,
                }
            })
            .collect();
        match st.sort {
            1 => list.sort_by(|a, b| a.name.cmp(&b.name)),
            2 => list.sort_by(|a, b| met.get(&b.dex).copied().unwrap_or(0).cmp(&met.get(&a.dex).copied().unwrap_or(0)).then(a.dex.cmp(&b.dex))),
            _ => {}
        }
        let shiny = st.shiny;
        self.setup.pokedex.list_ids = list.iter().map(|m| m.id.clone()).collect();

        // ── count and legend
        let mut show_unob = self.setup.pokedex.show_unobtainable;
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} Pokémon", list.len())).font(super::charts::bold(14.0)).color(TEXT()));
            ui.add_space(10.0);
            for (icon, c, t) in [("✨", GOLD, "shiny caught"), ("🎯", theme::HP_RED, "you're hunting it"), ("🌿", theme::GREEN, "on rare grass today")] {
                ui.label(RichText::new(icon).size(12.0).color(c));
                ui.label(RichText::new(t).size(12.0).color(MUTED()));
                ui.add_space(6.0);
            }
            if shiny {
                ui.add_space(6.0);
                kit::checkbox(ui, &mut show_unob, "Unobtainable").on_hover_text("Also list Pokémon that can't be got in the game yet");
                // your hunts at a glance, as the Shiny Dex showed them
                let hunts: Vec<&crate::config::HuntRecord> = self.cfg.history.iter().filter(|h| !h.phase).collect();
                if !hunts.is_empty() {
                    let n = hunts.len();
                    let phases = self.cfg.history.len() - n;
                    let total: u64 = hunts.iter().map(|h| h.encounters).sum();
                    let mut txt = format!("{n} caught on hunts · avg {} encounters", theme::thousands(total / n as u64));
                    if phases > 0 {
                        txt.push_str(&format!(" · {phases} phase{}", if phases == 1 { "" } else { "s" }));
                    }
                    if let Some(h) = hunts.iter().min_by_key(|h| h.encounters) {
                        txt.push_str(&format!(" · luckiest {} at {}", h.target, theme::thousands(h.encounters)));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(txt).size(12.0).color(MUTED()));
                    });
                }
            }
        });
        if show_unob != self.setup.pokedex.show_unobtainable {
            self.setup.pokedex.show_unobtainable = show_unob;
        }
        ui.add_space(10.0);
        if list.is_empty() {
            let mut clear = false;
            kit::card(ui, |ui| {
                ui.add_space(8.0);
                ui.add_space(14.0);
                ui.vertical_centered(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                    let c = r.center() - vec2(4.0, 4.0);
                    ui.painter().circle_filled(r.center(), 22.0, theme::blend(CARD(), TEXT(), 0.06));
                    ui.painter().circle_stroke(c, 8.5, Stroke::new(2.2f32, MUTED()));
                    ui.painter().line_segment([c + vec2(6.2, 6.2), c + vec2(12.0, 12.0)], Stroke::new(2.6f32, MUTED()));
                    ui.add_space(6.0);
                    let q = self.setup.pokedex.search.trim();
                    let title = if q.is_empty() { "No Pokémon match these filters".to_string() } else { format!("No Pokémon match \"{q}\"") };
                    ui.label(RichText::new(title).font(super::charts::bold(15.0)).color(TEXT()));
                    ui.add_space(4.0);
                    ui.label(RichText::new("Try fewer filters, or search by name or dex number (e.g. 25 or #0025).").size(12.5).color(MUTED()));
                    ui.add_space(12.0);
                    clear = kit::accent_button_w(ui, egui::Button::new("Clear all filters"), 10.0, 150.0).clicked();
                });
                ui.add_space(14.0);
                ui.add_space(8.0);
            });
            if clear {
                self.dex_clear_filters();
            }
            self.setup.pokedex.index = Some((ikey, index));
            return;
        }

        // ── grid
        let gap = 10.0;
        let full = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 4.0);
        let cols = ((full + gap) / (124.0 + gap)).floor().max(2.0) as usize;
        let tw = ((full - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let th = 150.0;
        let mut open: Option<String> = None;
        let mut trophy: Option<String> = None;
        let mut mark: Option<(String, bool)> = None;
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for m in row {
                    let (r, resp) = ui.allocate_exact_size(vec2(tw, th), Sense::click());
                    if !ui.is_rect_visible(r) {
                        continue;
                    }
                    let Some(sp) = self.species.get((m.dex as usize).wrapping_sub(1)).cloned() else { continue };
                    let src = caught.get(&sp.slug).cloned();
                    let has = src.is_some();
                    let hunting = targets.contains(&sp.slug);
                    let on_rare = rare.contains(&sp.slug);
                    let n_met = met.get(&m.dex).copied().unwrap_or(0);
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let tm = TileMon { dex: m.dex, name: m.name.clone(), types: m.types.clone(), modern: m.modern, cant_get: m.cant_get };
                    self.dex_tile(ui, r, &sp, &tm, shiny, has, hunting, on_rare, ht);
                    let mut tip = format!("#{:04} {}  ·  {}", m.dex, m.name, m.types.join(" / "));
                    match (&src, shiny) {
                        (Some(super::extras::DexSource::Hunt(t, n)), true) => tip.push_str(&format!("\n✨ Shiny caught {} after {} encounters", crate::stats::fmt_date(*t), theme::thousands(*n))),
                        (Some(super::extras::DexSource::Manual), true) => tip.push_str("\n✨ Shiny marked by hand"),
                        (Some(_), false) => tip.push_str("\n✨ Shiny caught"),
                        (None, true) if !m.cant_get => tip.push_str("\nNot caught yet"),
                        _ => {}
                    }
                    if hunting {
                        tip.push_str("\n🎯 You're hunting it");
                    }
                    if on_rare {
                        tip.push_str("\n🌿 On rare grass today");
                    }
                    if n_met > 0 {
                        tip.push_str(&format!("\nMet {} times on your hunts", theme::thousands(n_met)));
                    }
                    if m.cant_get {
                        tip.push_str("\nCan't be got in the game yet");
                    }
                    if shiny {
                        tip.push_str(if has { "\nClick for its trophy" } else { "\nClick for its Pokédex page" });
                        tip.push_str(if matches!(src, Some(super::extras::DexSource::Hunt(..))) { "" } else { "\nRight-click to mark one you caught before" });
                    }
                    let resp = resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
                    if resp.clicked() {
                        if shiny && has {
                            trophy = Some(sp.slug.clone());
                        } else {
                            open = Some(m.id.clone());
                        }
                    }
                    if shiny {
                        // right-click: mark a shiny you caught before (or take the mark off)
                        let slug = sp.slug.clone();
                        let manual = matches!(src, Some(super::extras::DexSource::Manual));
                        resp.context_menu(|ui| {
                            if !has {
                                if ui.button("✨ Mark as caught (shiny)").clicked() {
                                    mark = Some((slug.clone(), true));
                                    ui.close_menu();
                                }
                            } else if manual {
                                if ui.button("Remove the mark").clicked() {
                                    mark = Some((slug.clone(), false));
                                    ui.close_menu();
                                }
                            } else {
                                ui.label("Caught on a hunt: click it to see the trophy");
                            }
                        });
                    }
                }
            });
            ui.add_space(gap);
        }
        self.setup.pokedex.index = Some((ikey, index));
        if let Some(id) = open {
            self.setup.pokedex.open = Some(id);
            self.setup.pokedex.tab = EntryTab::Overview;
        }
        if let Some(slug) = trophy {
            self.setup.dex.open = Some(slug);
            self.setup.note_draft.clear();
            self.setup.trophy_back = None;
        }
        if let Some((slug, on)) = mark {
            if on {
                if !self.cfg.dex_manual.contains(&slug) {
                    self.cfg.dex_manual.push(slug);
                }
            } else {
                self.cfg.dex_manual.retain(|s| *s != slug);
            }
            self.dirty = true;
            self.check_progress();
        }
    }

    /// How many of each species you've met (national dex → encounters), over
    /// every run.
    pub(super) fn species_met(&self) -> std::collections::BTreeMap<u32, u64> {
        let mut out = std::collections::BTreeMap::new();
        let mut add = |st: &crate::stats::RunStats| {
            for (d, n) in &st.species {
                *out.entry(*d).or_insert(0) += n;
            }
        };
        add(&self.cfg.run_stats);
        for r in self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id) {
            add(&r.stats);
        }
        for h in self.cfg.history.iter().filter(|h| !h.phase) {
            if let Some(st) = &h.stats {
                add(st);
            }
        }
        out
    }

    fn dex_clear_filters(&mut self) {
        let st = &mut self.setup.pokedex;
        st.search.clear();
        st.type_filter = None;
        st.how = HowFilter::Any;
        st.stage = None;
        st.era = EraFilter::All;
        st.caught = CaughtFilter::All;
        st.region = None;
    }

    /// Shiny Dex progress: overall and by region (click a region to show it).
    fn dex_progress_card(&mut self, ui: &mut egui::Ui, index: &[MonInfo], caught: &std::collections::BTreeMap<String, super::extras::DexSource>) {
        use crate::moddex::Region;
        let mut per: Vec<(Region, usize, usize)> = Region::ALL.iter().map(|r| (*r, 0, 0)).collect();
        for m in index.iter().filter(|m| !m.cant_get) {
            let r = Region::of(m.dex as u16, m.modern);
            let has = self.species.get((m.dex as usize).wrapping_sub(1)).is_some_and(|s| caught.contains_key(&s.slug));
            if let Some(e) = per.iter_mut().find(|e| e.0 == r) {
                e.2 += 1;
                e.1 += has as usize;
            }
        }
        let got: usize = per.iter().map(|e| e.1).sum();
        let all: usize = per.iter().map(|e| e.2).sum::<usize>().max(1);
        let sel = self.setup.pokedex.region;
        let mut pick: Option<Option<Region>> = None;
        kit::card(ui, |ui| {
            let wide = ui.available_width() >= 760.0;
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), if wide { 92.0 } else { 200.0 }), Sense::hover());
            let p = ui.painter().clone();
            let c = pos2(rect.min.x + 46.0, rect.min.y + 46.0);
            let frac = got as f32 / all as f32;
            let g = kit::grow(ui.ctx(), egui::Id::new("dex-ring"), frac);
            p.circle_stroke(c, 38.0, Stroke::new(8.0f32, theme::blend(CARD(), TEXT(), 0.09)));
            ring_arc(&p, c, 38.0, g, Stroke::new(8.0f32, GOLD));
            p.text(c - vec2(0.0, 6.0), Align2::CENTER_CENTER, format!("{:.0}%", frac * 100.0), super::charts::heavy(17.0), TEXT());
            p.text(c + vec2(0.0, 12.0), Align2::CENTER_CENTER, "shiny", super::charts::reg(10.5), MUTED());
            let x = rect.min.x + 104.0;
            p.text(pos2(x, rect.min.y + 22.0), Align2::LEFT_CENTER, "Your Shiny Dex", super::charts::bold(16.0), TEXT());
            p.text(pos2(x, rect.min.y + 44.0), Align2::LEFT_CENTER, format!("{got} of {all} Pokémon you can get · click a region to show it"), super::charts::reg(12.5), MUTED());
            let (rx, ry, rw) = if wide { (x, rect.min.y + 62.0, (rect.max.x - x) / 5.0) } else { (rect.min.x, rect.min.y + 100.0, rect.width() / 2.0) };
            for (k, (reg, n, tot)) in per.iter().enumerate() {
                let (cx, cy) = if wide { (rx + rw * k as f32, ry) } else { (rx + rw * (k % 2) as f32, ry + 34.0 * (k / 2) as f32) };
                let cell = Rect::from_min_size(pos2(cx - 6.0, cy - 6.0), vec2(rw - 10.0, 32.0));
                let resp = ui.interact(cell, ui.id().with(("dex-region", k)), Sense::click());
                let on = sel == Some(*reg);
                let ht = kit::anim(ui.ctx(), resp.id, resp.hovered() || on, kit::QUICK);
                if ht > 0.0 {
                    p.rect_filled(cell, 10.0, theme::blend(CARD(), kit::primary(), 0.10 * ht));
                }
                if on {
                    p.rect_stroke(cell, 10.0, Stroke::new(1.0f32, kit::primary()));
                }
                p.text(pos2(cx, cy + 4.0), Align2::LEFT_CENTER, reg.label(), super::charts::bold(12.0), TEXT());
                p.text(pos2(cx + rw - 22.0, cy + 4.0), Align2::RIGHT_CENTER, format!("{n}/{tot}"), super::charts::reg(11.5), MUTED());
                let bar = Rect::from_min_size(pos2(cx, cy + 14.0), vec2(rw - 22.0, 5.0));
                p.rect_filled(bar, 2.5, theme::blend(CARD(), TEXT(), 0.09));
                let f = kit::grow(ui.ctx(), egui::Id::new(("dex-reg-bar", k)), *n as f32 / (*tot).max(1) as f32);
                if f > 0.0 {
                    p.rect_filled(Rect::from_min_size(bar.min, vec2((bar.width() * f).max(5.0), 5.0)), 2.5, GOLD);
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(if on { "Show every region" } else { "Show only this region" }).clicked() {
                    pick = Some(if on { None } else { Some(*reg) });
                }
            }
        });
        if let Some(r) = pick {
            self.setup.pokedex.region = r;
            self.setup.pokedex.grid_gen += 1;
        }
    }

    /// Search, sprite style, sort, type chips and the other filters.
    fn dex_toolbar(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, index: &[MonInfo]) {
        let stages = w.data.stages.clone();
        let mut types: Vec<String> = Vec::new();
        for m in index {
            for t in &m.types {
                if !types.contains(t) {
                    types.push(t.clone());
                }
            }
        }
        let st = &mut self.setup.pokedex;
        // row 1: search, normal/shiny, sort
        let rest_id = egui::Id::new("dex-tools-w");
        let rest_w = ui.ctx().data(|d| d.get_temp::<f32>(rest_id)).unwrap_or(330.0);
        // The search box draws wider than the width it's given (padding and
        // the icon), so that extra is measured too, or the row runs off the
        // page and drags everything under it wider.
        let extra_id = egui::Id::new("dex-search-extra");
        let extra = ui.ctx().data(|d| d.get_temp::<f32>(extra_id)).unwrap_or(48.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let row_start = ui.cursor().min.x;
            let sw = (ui.available_width() - rest_w - extra - 24.0).max(140.0);
            kit::search_box(ui, &mut st.search, "Search by name or number", sw);
            let real = ui.cursor().min.x - row_start - 8.0 - sw;
            if (real - extra).abs() > 0.5 {
                ui.ctx().data_mut(|d| d.insert_temp(extra_id, real));
                ui.ctx().request_repaint();
            }
            ui.add_space((ui.available_width() - rest_w - 2.0).max(8.0));
            let start = ui.cursor().min.x;
            kit::segmented(ui, &mut st.shiny, &[(false, "Normal"), (true, "✨ Shiny Dex")]);
            kit::Dropdown::from_id_salt("pdx-sort")
                .selected_text(["Number", "Name", "Most met"][st.sort.min(2) as usize])
                .width(110.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut st.sort, 0, "Number");
                    ui.selectable_value(&mut st.sort, 1, "Name");
                    ui.selectable_value(&mut st.sort, 2, "Most met").on_hover_text("The Pokémon you've run into most on your hunts");
                });
            let used = ui.min_rect().max.x - start;
            if (used - rest_w).abs() > 0.5 {
                ui.ctx().data_mut(|d| d.insert_temp(rest_id, used));
                ui.ctx().request_repaint();
            }
        });
        ui.add_space(10.0);
        // row 2: types as chips
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for t in &types {
                let on = st.type_filter.as_deref() == Some(t.as_str());
                if type_toggle(ui, t, on).on_hover_text(if on { "Show every type".to_string() } else { format!("Only {t}-type") }).clicked() {
                    st.type_filter = if on { None } else { Some(t.clone()) };
                }
            }
        });
        ui.add_space(10.0);
        // row 3: caught, how, when, era, clear
        let active = !st.search.is_empty() || st.type_filter.is_some() || st.how != HowFilter::Any || st.stage.is_some() || st.era != EraFilter::All || st.caught != CaughtFilter::All || st.region.is_some();
        let mut clear = false;
        // The switches are nested layouts, which a wrapped row can't move to
        // the next line, so narrow pages get the row in two halves.
        let split = ui.available_width() < 980.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            kit::segmented(ui, &mut st.caught, &[(CaughtFilter::All, "All"), (CaughtFilter::Caught, "Shiny caught"), (CaughtFilter::NotYet, "Not caught")]);
            kit::Dropdown::from_id_salt("pdx-how").selected_text(st.how.label()).width(170.0).show_ui(ui, |ui| {
                for h in HowFilter::ALL {
                    ui.selectable_value(&mut st.how, h, h.label());
                }
            });
            if split {
                return;
            }
            dex_when_era(ui, st, &stages, active, &mut clear);
        });
        if split {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                dex_when_era(ui, st, &stages, active, &mut clear);
            });
        }
        if clear {
            self.dex_clear_filters();
        }
    }
}

/// The toolbar's "wild by" stage picker, era switch and clear link.
fn dex_when_era(ui: &mut egui::Ui, st: &mut DexState, stages: &[String], active: bool, clear: &mut bool) {
    kit::Dropdown::from_id_salt("pdx-stage")
        .selected_text(match st.stage.and_then(|i| stages.get(i)) {
            Some(s) => format!("Wild by: {s}"),
            None => "Any point in the game".to_string(),
        })
        .width(210.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut st.stage, None, "Any point in the game");
            for (i, s) in stages.iter().enumerate() {
                ui.selectable_value(&mut st.stage, Some(i), format!("Wild by: {s}"));
            }
        })
        .response
        .on_hover_text("Only Pokémon found in the wild by this point (badge stage)");
    kit::segmented(ui, &mut st.era, &[(EraFilter::All, "All"), (EraFilter::Platinum, "Platinum"), (EraFilter::Modern, "Modern")]);
    if active && ui.link(RichText::new("Clear filters").size(12.5)).clicked() {
        *clear = true;
    }
}

impl App {

    /// One Pokémon in the grid.
    #[allow(clippy::too_many_arguments)]
    fn dex_tile(&mut self, ui: &egui::Ui, r: Rect, sp: &crate::species::Species, m: &TileMon, shiny: bool, has: bool, hunting: bool, on_rare: bool, ht: f32) {
        let p = ui.painter_at(r.expand(4.0));
        let tc = m.types.first().map(|t| type_color(t)).unwrap_or(MUTED());
        let r = r.translate(vec2(0.0, -2.0 * ht));
        if ht > 0.0 {
            p.add(kit::shadow_tier(2).as_shape(r, 18.0));
        }
        // Shiny Dex mode: caught ones glow gold, the rest are shadows to fill in
        let fill = if shiny && has { theme::blend(CARD(), GOLD, 0.10 + 0.04 * ht) } else { theme::blend(CARD(), tc, 0.05 + 0.04 * ht) };
        p.rect_filled(r, 18.0, fill);
        let border = if has { GOLD.gamma_multiply(0.75) } else { theme::blend(LINE(), tc, 0.5 * ht) };
        p.rect_stroke(r, 18.0, Stroke::new(if has { 1.5f32 } else { 1.0 }, border));
        // sprite on a soft circle of its type colour
        let c = pos2(r.center().x, r.min.y + 54.0);
        p.circle_filled(c, 36.0, theme::blend(CARD(), tc, 0.18));
        let tint = if shiny && !has {
            // a silhouette, like a dex you haven't filled in yet
            if m.cant_get { Color32::from_rgba_unmultiplied(10, 8, 18, 110) } else { Color32::from_rgba_unmultiplied(10, 8, 18, 200) }
        } else if m.cant_get {
            Color32::from_white_alpha(110)
        } else {
            Color32::WHITE
        };
        self.draw_sprite_tinted(&p, sp, shiny, c + vec2(0.0, 32.0), vec2(68.0, 64.0), tint);
        p.text(r.min + vec2(10.0, 12.0), Align2::LEFT_CENTER, format!("#{:04}", m.dex), super::charts::reg(10.5), MUTED());
        // corner marks
        let mut mx = r.max.x - 12.0;
        if has {
            theme::sparkle(&p, pos2(mx, r.min.y + 12.0), 5.0, GOLD);
            mx -= 16.0;
        }
        if hunting {
            p.text(pos2(mx, r.min.y + 12.0), Align2::CENTER_CENTER, "🎯", egui::FontId::proportional(11.0), theme::HP_RED);
            mx -= 16.0;
        }
        if on_rare {
            p.text(pos2(mx, r.min.y + 12.0), Align2::CENTER_CENTER, "🌿", egui::FontId::proportional(11.0), theme::GREEN);
            mx -= 16.0;
        }
        if shiny && m.cant_get {
            // padlock: can't be got yet
            let c = pos2(mx, r.min.y + 13.0);
            p.rect_filled(Rect::from_center_size(c + vec2(0.0, 2.0), vec2(9.0, 7.0)), 1.5, MUTED());
            p.circle_stroke(c - vec2(0.0, 1.5), 3.0, Stroke::new(1.3f32, MUTED()));
        }
        // name and types
        let name = super::charts::clip(ui, &m.name, &super::charts::bold(13.0), r.width() - 12.0);
        p.text(pos2(r.center().x, r.min.y + 104.0), Align2::CENTER_CENTER, name, super::charts::bold(13.0), if m.cant_get || (shiny && !has) { MUTED() } else { TEXT() });
        let font = super::charts::bold(8.5);
        let gs: Vec<_> = m.types.iter().map(|t| (type_color(t), p.layout_no_wrap(t.to_uppercase(), font.clone(), Color32::WHITE))).collect();
        let total: f32 = gs.iter().map(|(_, g)| g.size().x + 12.0).sum::<f32>() + 4.0 * (gs.len().saturating_sub(1)) as f32;
        let mut x = r.center().x - total / 2.0;
        for (c, g) in gs {
            let pr = Rect::from_min_size(pos2(x, r.min.y + 118.0), vec2(g.size().x + 12.0, 16.0));
            p.rect_filled(pr, 8.0, c);
            let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
            let ink = if lum > 150.0 { Color32::from_rgb(34, 34, 40) } else { Color32::WHITE };
            let gw = g.size();
            p.galley_with_override_text_color(pos2(pr.min.x + 6.0, pr.center().y - gw.y / 2.0), g, ink);
            x = pr.max.x + 4.0;
        }
        if m.modern {
            p.text(pos2(r.min.x + 10.0, r.min.y + 26.0), Align2::LEFT_CENTER, "NEW", super::charts::bold(8.5), theme::LINK());
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
                ui.spacing_mut().item_spacing.x = 8.0;
                if let Some(n) = &next {
                    if self.nav_chip(ui, n, false).on_hover_text("Next (→ key)").clicked() {
                        go = Some(n.id.clone());
                    }
                }
                if let Some(p) = &prev {
                    if self.nav_chip(ui, p, true).on_hover_text("Previous (← key)").clicked() {
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
        let tcg = match self.tcg_count(&m) {
            0 => "TCG".to_string(),
            n => format!("TCG ({n})"),
        };
        kit::tab_bar(
            ui,
            &mut self.setup.pokedex.tab,
            &[
                (EntryTab::Overview, "Overview"),
                (EntryTab::Evolution, "Evolution"),
                (EntryTab::Where, "Where to find"),
                (EntryTab::Moves, "Moves"),
                (EntryTab::Tcg, tcg.as_str()),
            ],
        );
        ui.add_space(10.0);
        let tab = self.setup.pokedex.tab;
        match tab {
            EntryTab::Overview => self.entry_overview(ui, &m, sp.as_ref(), &extra),
            EntryTab::Evolution => self.entry_evolution(ui, w, &m),
            EntryTab::Where => self.entry_where(ui, w, &m),
            EntryTab::Moves => self.entry_moves(ui, w, &m),
            EntryTab::Tcg => self.entry_tcg(ui, w, &m),
        }
    }

    /// Big sprite on its type colours, name, types, your history with it
    /// and the actions.
    fn entry_header(
        &mut self,
        ui: &mut egui::Ui,
        w: &Arc<Wiki>,
        m: &crate::wiki::Mon,
        sp: Option<&crate::species::Species>,
        extra: Option<&crate::dexextra::Extra>,
    ) {
        use super::charts as ch;
        let caught = sp.and_then(|sp| self.dex_caught().get(&sp.slug).cloned());
        let cant_get = w.unobtainable(&m.id).map(|s| s.to_string());
        let hunting = sp.is_some_and(|sp| self.cfg.target.as_deref() == Some(sp.slug.as_str()) || self.also_slugs().contains(&sp.slug));
        let met = self.species_met().get(&m.dex).copied().unwrap_or(0);
        let rare_spots = sp.map(|s| self.rare_spots_for(&s.slug)).unwrap_or_default();
        let team_list = self.team_choices();
        let mut add_team: Option<Option<u64>> = None;
        let mut test_catch = false;
        let map_ok = self.gamemap.as_ref().is_some_and(|r| r.is_ok());
        let wild_places: Vec<String> = {
            let mut v: Vec<String> = Vec::new();
            for e in w.wild(&m.id) {
                if e.setting.is_empty() && !v.contains(&e.area) {
                    v.push(e.area.clone());
                }
            }
            v
        };
        let mut hunt = false;
        let mut show_map = false;
        let mut rare_hunt = false;
        let mut rare_show = false;
        let mut show_type: Option<String> = None;
        let mut open_wiki = false;
        let mut open_mon: Option<String> = None;
        let mut shiny = self.setup.pokedex.shiny;

        let full = ui.available_width();
        let wide = full >= 760.0;
        let stage_w = if wide { 260.0 } else { full };
        let c1 = m.types.first().map(|t| type_color(t)).unwrap_or(MUTED());
        let c2 = m.types.get(1).map(|t| type_color(t)).unwrap_or(theme::blend(c1, Color32::BLACK, 0.35));
        kit::card(ui, |ui| {
            let inner = ui.available_width();
            let layout = if wide { egui::Layout::left_to_right(egui::Align::Min) } else { egui::Layout::top_down(egui::Align::Min) };
            ui.with_layout(layout, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                // ── stage
                let sw = if wide { stage_w } else { inner };
                let (stage, resp) = ui.allocate_exact_size(vec2(sw, 260.0), Sense::click());
                let p = ui.painter_at(stage.expand(2.0));
                p.add(theme::gradient_rect(stage, [20.0; 4], theme::blend(c1, Color32::BLACK, 0.15), theme::blend(c2, Color32::BLACK, 0.35)));
                let sc = pos2(stage.center().x, stage.min.y + 112.0);
                p.circle_filled(sc, 86.0, Color32::from_white_alpha(10));
                p.circle_filled(sc, 62.0, Color32::from_white_alpha(12));
                if let Some(sp) = sp {
                    self.draw_sprite_centered(&p, sp, shiny, sc, vec2(150.0, 150.0));
                }
                if shiny {
                    theme::sparkle(&p, sc + vec2(66.0, -60.0), 8.0, GOLD);
                    theme::sparkle(&p, sc + vec2(-70.0, 40.0), 5.0, GOLD);
                }
                let tr = Rect::from_center_size(pos2(stage.center().x, stage.max.y - 32.0), vec2(170.0, 34.0));
                let mut c = ui.new_child(egui::UiBuilder::new().max_rect(tr).layout(egui::Layout::left_to_right(egui::Align::Center)));
                kit::segmented(&mut c, &mut shiny, &[(false, "Normal"), (true, "✨ Shiny")]);
                if resp.on_hover_text("Click to switch normal / shiny").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    shiny = !shiny;
                }
                ui.add_space(22.0);
                let facts_w = if wide && inner >= 1050.0 { 290.0 } else { 0.0 };
                let details_w = if wide { inner - sw - 22.0 - if facts_w > 0.0 { facts_w + 44.0 } else { 0.0 } } else { inner };
                // ── details
                ui.allocate_ui_with_layout(vec2(details_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(details_w);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("#{:04}", m.dex)).size(13.0).color(MUTED()));
                    ui.label(RichText::new(&m.name).font(ch::heavy(32.0)).color(TEXT()));
                    if let Some(e) = extra.filter(|e| !e.category.is_empty()) {
                        ui.label(RichText::new(&e.category).size(13.5).color(MUTED()));
                    }
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                        for t in &m.types {
                            if type_pill(ui, t).on_hover_text(format!("Show every {t}-type Pokémon")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                show_type = Some(t.clone());
                            }
                        }
                        if m.modern {
                            kit::pill(ui, "Modern", PillKind::Info).on_hover_text("Added by the MODERN POKéMON option");
                        }
                        if let Some(why) = &cant_get {
                            let why = if why.trim().is_empty() { "Can't be got in the game yet" } else { why.as_str() };
                            kit::pill(ui, "Can't get yet", PillKind::Warn).on_hover_text(why);
                        }
                    });
                    ui.add_space(12.0);
                    // your history with it
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        match &caught {
                            Some(super::extras::DexSource::Hunt(t, n)) => {
                                info_chip(ui, "✨", GOLD, &format!("Shiny caught {} · {} encounters", crate::stats::fmt_date(*t), theme::thousands(*n)));
                            }
                            Some(super::extras::DexSource::Manual) => {
                                info_chip(ui, "✨", GOLD, "In your Shiny Dex");
                            }
                            None => {
                                info_chip(ui, "○", MUTED(), "No shiny yet");
                            }
                        }
                        if hunting {
                            info_chip(ui, "🎯", theme::HP_RED, "You're hunting it");
                        }
                        if met > 0 {
                            info_chip(ui, "⚔", super::charts::PURPLE, &format!("Met {} times on your hunts", theme::thousands(met)));
                        }
                    });
                    ui.add_space(14.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        if let Some(sp) = sp {
                            let label = if hunting { "🎯  Go to my hunt" } else { "✨  Hunt this shiny" };
                            let tip = if hunting {
                                format!("You're hunting {} now", m.name)
                            } else {
                                let mut t = format!("Make {} your Shiny Hunter target. Your current hunt is saved, so you can switch back to it from Runs.", sp.name);
                                if cant_get.is_some() {
                                    t.push_str("\n\nHeads up: the wiki says you can't get this one in the game yet.");
                                }
                                t
                            };
                            if kit::accent_button(ui, egui::Button::new(RichText::new(label).font(ch::bold(13.0)).color(kit::on_primary())), 4.0).on_hover_text(tip).clicked() {
                                hunt = true;
                            }
                        }
                        let can_map = map_ok && !wild_places.is_empty();
                        if !super::app_modal::have(crate::apps::AppId::Map) {
                            super::app_modal::lock_btn(ui, crate::apps::AppId::Map, "Show on map");
                        } else if ui
                            .add_enabled(can_map, egui::Button::new("🗺  Show on map").fill(kit::btn_fill()))
                            .on_hover_text("Every place it lives in the wild, flashing on the Map")
                            .on_disabled_hover_text(if map_ok { "Not found in the wild" } else { "The Map needs Platinum Together installed on this PC" })
                            .clicked()
                        {
                            show_map = true;
                        }
                        if ui.add(egui::Button::new("Wiki  ↗").fill(kit::btn_fill())).on_hover_text(w.page_url(m)).clicked() {
                            open_wiki = true;
                        }
                        if super::app_modal::have(crate::apps::AppId::Team) {
                            super::team_home::add_to_team_menu(ui, &team_list, &mut add_team);
                        } else {
                            super::app_modal::lock_btn(ui, crate::apps::AppId::Team, "Add to team");
                        }
                        if ui.add(egui::Button::new("🎯  Catch odds").fill(kit::btn_fill())).on_hover_text("Every Poké Ball's chance, in the Battle Sim's Catch Lab").clicked() {
                            test_catch = true;
                        }
                    });
                    if !rare_spots.is_empty() {
                        ui.add_space(12.0);
                        egui::Frame::none().fill(ch::faint(super::charts::GREEN, 0.12)).rounding(14.0).inner_margin(egui::Margin::symmetric(12.0, 8.0)).show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                ui.label(RichText::new(format!("🌿 On rare grass today: {}", super::rare::places_text(&rare_spots))).size(12.5).color(ch::ink(super::charts::GREEN)));
                                if ui.add(egui::Button::new(RichText::new("Hunt it there").size(12.5)).fill(kit::btn_fill())).on_hover_text("Make it your target and hunt it with Sweet Scent on today's rare grass").clicked() {
                                    rare_hunt = true;
                                }
                                if ui.link(RichText::new("Show on map").size(12.5)).clicked() {
                                    rare_show = true;
                                }
                            });
                        });
                    }
                });
                if facts_w > 0.0 {
                    ui.add_space(22.0);
                    let (line, _) = ui.allocate_exact_size(vec2(1.0, 236.0), Sense::hover());
                    ui.painter().line_segment([line.center_top(), line.center_bottom()], Stroke::new(1.0f32, LINE()));
                    ui.add_space(20.0);
                    ui.allocate_ui_with_layout(vec2(facts_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(facts_w);
                        ui.add_space(8.0);
                        ui.label(RichText::new("QUICK FACTS").font(ch::bold(10.5)).color(MUTED()));
                        ui.add_space(8.0);
                        let row = |ui: &mut egui::Ui, label: &str| {
                            ui.label(RichText::new(label).size(11.5).color(MUTED()));
                        };
                        let name_of = |id: &str| w.mon(id).map(|x| x.name.clone()).unwrap_or_else(|| id.to_string());
                        if let Some(e) = w.evolves_from(&m.id) {
                            row(ui, "Evolves from");
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                if ui.link(RichText::new(name_of(&e.from)).size(13.5)).clicked() {
                                    open_mon = Some(e.from.clone());
                                }
                                ui.label(RichText::new(short_method(&e.method)).size(12.0).color(MUTED()));
                            });
                            ui.add_space(6.0);
                        }
                        let into = w.evolves_into(&m.id);
                        if !into.is_empty() {
                            row(ui, "Evolves into");
                            for e in into.iter().take(3) {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    if ui.link(RichText::new(name_of(&e.species)).size(13.5)).clicked() {
                                        open_mon = Some(e.species.clone());
                                    }
                                    ui.label(RichText::new(short_method(&e.method)).size(12.0).color(MUTED()));
                                });
                            }
                            if into.len() > 3 {
                                ui.label(RichText::new(format!("+ {} more (see Evolution)", into.len() - 3)).size(11.5).color(MUTED()));
                            }
                            ui.add_space(6.0);
                        }
                        if w.evolves_from(&m.id).is_none() && into.is_empty() {
                            row(ui, "Evolution");
                            ui.label(RichText::new("Doesn't evolve").size(13.5).color(TEXT()));
                            ui.add_space(6.0);
                        }
                        let best = w.wild(&m.id).into_iter().filter(|e| e.setting.is_empty()).max_by_key(|e| e.chance.morning.max(e.chance.day).max(e.chance.night));
                        row(ui, "Most common at");
                        match best {
                            Some(b) => {
                                let place = w.place(&b.area).map(|p| p.name.clone()).unwrap_or_else(|| b.area.clone());
                                let ch_ = b.chance.morning.max(b.chance.day).max(b.chance.night);
                                ui.label(RichText::new(format!("{place} · {ch_}%")).size(13.5).color(TEXT()));
                            }
                            None => {
                                let other = w.other(&m.id);
                                let t = match other.first() {
                                    Some(o) if !o.how.is_empty() => format!("Not in the wild · {}", o.how),
                                    _ if !w.wild(&m.id).is_empty() => "Only with MORE / MODERN on".to_string(),
                                    _ => "Not in the wild".to_string(),
                                };
                                ui.label(RichText::new(t).size(13.5).color(TEXT()));
                            }
                        }
                        ui.add_space(6.0);
                        if let Some(e) = extra.filter(|e| e.total > 0) {
                            row(ui, "Base stat total");
                            ui.label(RichText::new(e.total.to_string()).size(13.5).color(TEXT()));
                        }
                    });
                }
            });
        });
        if let Some(id) = open_mon {
            self.setup.pokedex.open = Some(id);
            return;
        }
        if rare_hunt {
            if let Some(sp) = sp {
                self.hunt_on_rare_grass(&sp.slug, sp.name, &rare_spots);
            }
        } else if rare_show {
            if let Some(s) = rare_spots.first() {
                self.go_to_rare(s);
            }
        }
        self.setup.pokedex.shiny = shiny;
        if show_map {
            self.show_on_map(&wild_places, m.name.clone(), None);
        }
        if let Some(t) = add_team {
            self.team_add_from(&m.id, t);
        }
        if test_catch {
            self.sim_test_catch(&m.id);
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

    /// Overview tab: the facts at a glance, abilities, gender, base stats
    /// and every type matchup.
    fn entry_overview(&mut self, ui: &mut egui::Ui, m: &crate::wiki::Mon, sp: Option<&crate::species::Species>, extra: &crate::dexextra::State) {
        use crate::dexextra::State;
        let e = match extra {
            State::Ready(e) => Some(e.as_ref()),
            _ => None,
        };
        match extra {
            State::Loading => {
                kit::pill(ui, "Getting the details from pokemondb.net…", PillKind::Info);
                ui.add_space(10.0);
            }
            State::Failed(err) => {
                let mut retry = false;
                kit::card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Couldn't get the details from pokemondb.net. Check your internet connection.").size(12.5).color(MUTED())).on_hover_text(err);
                        retry = ui.button("Try again").clicked();
                    });
                });
                if retry {
                    if let Some(sp) = sp {
                        self.dexdb.retry(&sp.slug);
                    }
                }
                ui.add_space(10.0);
            }
            State::Ready(_) => {}
        }
        let mut breed_go: Option<BreedGo> = None;
        let mut left = |ui: &mut egui::Ui| {
            glance_card(ui, e);
            ui.add_space(16.0);
            if let Some(g) = breeding_card(ui, m, e) {
                breed_go = Some(g);
            }
        };
        let right = |ui: &mut egui::Ui| {
            stats_card(ui, m, e);
            ui.add_space(16.0);
            defenses_card(ui, m);
        };
        let w = ui.available_width();
        if w >= 820.0 {
            let gap = 16.0;
            let lw = ((w - gap) / 2.0).floor();
            let h = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(lw, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(lw);
                    left(ui);
                });
                ui.allocate_ui_with_layout(vec2(lw, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(lw);
                    right(ui);
                });
            });
        } else {
            left(ui);
            ui.add_space(16.0);
            right(ui);
        }
        match breed_go {
            Some(BreedGo::Plan) => self.open_breed_plan(crate::breed_plan::Target { species: m.id.clone(), ..Default::default() }, None),
            Some(BreedGo::Partners) => self.open_breed_partners(&m.id),
            None => {}
        }
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let small = |t: &str| RichText::new(t).size(11.0).color(MUTED());
            ui.label(small("Name, number and types from the Platinum Together Wiki. Everything else from"));
            if let Some(sp) = sp {
                let url = crate::dexextra::Extra::page_url(&sp.slug);
                if ui.add(egui::Button::new(RichText::new("pokemondb.net ↗").size(11.0).color(theme::LINK())).frame(false)).on_hover_text(&url).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                }
            }
            if let Some(e) = e.filter(|e| !e.platinum_notes.is_empty()) {
                ui.label(small(&format!("(Platinum values used for: {}.)", e.platinum_notes.join(", ").to_lowercase())));
            }
        });
    }

    /// "‹ [sprite] #0003 Venusaur" for going to a neighbour.
    fn nav_chip(&mut self, ui: &mut egui::Ui, m: &crate::wiki::Mon, prev: bool) -> egui::Response {
        let label = format!("#{:04} {}", m.dex, m.name);
        let g = ui.painter().layout_no_wrap(label, super::charts::bold(12.5), TEXT());
        let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 70.0, 40.0), Sense::click());
        let ht = kit::anim(ui.ctx(), resp.id, resp.hovered(), kit::QUICK);
        let p = ui.painter().clone();
        p.rect_filled(r, 20.0, theme::blend(CARD(), TEXT(), 0.03 + 0.05 * ht));
        p.rect_stroke(r, 20.0, Stroke::new(1.0f32, LINE()));
        let (arrow_x, sprite_x, text_x) = if prev { (r.min.x + 14.0, r.min.x + 38.0, r.min.x + 56.0) } else { (r.max.x - 14.0, r.max.x - 38.0, r.min.x + 14.0) };
        p.text(pos2(arrow_x, r.center().y), Align2::CENTER_CENTER, if prev { "‹" } else { "›" }, super::charts::bold(16.0), MUTED());
        if let Some(sp) = self.species.get((m.dex as usize).wrapping_sub(1)).cloned() {
            self.draw_sprite_centered(&p, &sp, false, pos2(sprite_x, r.center().y), vec2(32.0, 32.0));
        }
        p.galley(pos2(text_x, r.center().y - g.size().y / 2.0), g, TEXT());
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
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
                        let (r, _) = ui.allocate_exact_size(egui::vec2(26.0, EVO_CHIP + EVO_TILE_H), Sense::hover());
                        ui.painter().text(
                            egui::pos2(r.center().x, r.min.y + EVO_CHIP + EVO_TILE_H / 2.0 - 10.0),
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
        let (r, resp) = ui.allocate_exact_size(egui::vec2(EVO_TILE_W, EVO_CHIP + EVO_TILE_H), Sense::click());
        if !ui.is_rect_visible(r) {
            return false;
        }
        let Some(mon) = w.mon(id) else { return false };
        let tile = egui::Rect::from_min_size(r.min + egui::vec2(0.0, EVO_CHIP), egui::vec2(EVO_TILE_W, EVO_TILE_H));
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
        if let Some(sp) = has_sp {
            let has = caught.contains_key(&sp.slug);
            let hunting = self.cfg.target.as_deref() == Some(sp.slug.as_str());
            let tm = TileMon { dex: mon.dex, name: mon.name.clone(), types: mon.types.clone(), modern: mon.modern, cant_get: w.unobtainable(id).is_some() };
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() || current, kit::QUICK);
            self.dex_tile(ui, tile, &sp, &tm, shiny, has, hunting, false, if current { 0.0 } else { ht });
            if current {
                ui.painter().rect_stroke(tile, 18.0, Stroke::new(2.0f32, kit::primary()));
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

const EVO_TILE_W: f32 = 132.0;
const EVO_TILE_H: f32 = 150.0;
/// Room above each tile for the "Lv. 16" chip.
const EVO_CHIP: f32 = 24.0;

pub(super) fn card_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).font(super::charts::bold(16.0)).color(TEXT()));
    ui.add_space(10.0);
}

/// A small rounded note: an icon and some words.
fn info_chip(ui: &mut egui::Ui, icon: &str, c: Color32, text: &str) {
    egui::Frame::none()
        .fill(theme::blend(CARD(), c, 0.12))
        .rounding(14.0)
        .inner_margin(egui::Margin { left: 10.0, right: 12.0, top: 5.0, bottom: 5.0 })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new(icon).size(12.5).color(c));
                ui.label(RichText::new(text).size(12.5).color(TEXT()));
            });
        });
}

/// A type you can click to show only that type.
fn type_toggle(ui: &mut egui::Ui, t: &str, on: bool) -> egui::Response {
    let c = type_color(t);
    let g = ui.painter().layout_no_wrap(t.to_string(), super::charts::bold(11.5), TEXT());
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 30.0, 28.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id, resp.hovered(), kit::QUICK);
    let p = ui.painter();
    if on {
        p.rect_filled(r, 14.0, c);
    } else {
        p.rect_filled(r, 14.0, theme::blend(CARD(), c, 0.10 + 0.12 * ht));
        p.rect_stroke(r, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), c, 0.5)));
    }
    let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    let ink = if on { if lum > 150.0 { Color32::from_rgb(34, 34, 40) } else { Color32::WHITE } } else { TEXT() };
    p.circle_filled(pos2(r.min.x + 13.0, r.center().y), 4.5, if on { ink } else { c });
    p.galley_with_override_text_color(pos2(r.min.x + 22.0, r.center().y - g.size().y / 2.0), g, ink);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Part of a circle from the top, clockwise.
fn ring_arc(p: &egui::Painter, c: egui::Pos2, r: f32, f: f32, stroke: Stroke) {
    if f <= 0.0 {
        return;
    }
    let n = (64.0 * f).ceil().max(2.0) as usize;
    let pts: Vec<egui::Pos2> = (0..=n)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * f * i as f32 / n as f32;
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    p.add(egui::Shape::line(pts, stroke));
}

/// Label above value, for the fact grids.
fn fact(ui: &mut egui::Ui, w: f32, label: &str, value: &str) {
    ui.allocate_ui_with_layout(vec2(w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_width(w);
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(RichText::new(label).size(11.5).color(MUTED()));
        ui.add(egui::Label::new(RichText::new(value).size(13.5).color(TEXT())).wrap());
    });
}

fn val(e: Option<&crate::dexextra::Extra>, f: impl Fn(&crate::dexextra::Extra) -> String) -> String {
    match e {
        Some(e) => {
            let v = f(e);
            if v.is_empty() { "—".into() } else { v }
        }
        None => "…".into(),
    }
}

/// Size, catch rate, training and abilities.
fn glance_card(ui: &mut egui::Ui, e: Option<&crate::dexextra::Extra>) {
    kit::card(ui, |ui| {
        ui.set_width(ui.available_width());
        card_title(ui, "At a glance");
        let cw = ((ui.available_width() - 16.0) / 2.0).floor();
        let rows = [
            [("Height", val(e, |e| e.height.clone())), ("Weight", val(e, |e| e.weight.clone()))],
            [("Catch rate", val(e, |e| e.catch_rate.clone())), ("Growth rate", val(e, |e| e.growth_rate.clone()))],
            [("EV yield", val(e, |e| e.ev_yield.clone())), ("Base exp.", val(e, |e| e.base_exp.clone()))],
            [("Base friendship", val(e, |e| e.base_friendship.clone())), ("Species", val(e, |e| e.category.clone()))],
        ];
        for row in rows {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                for (l, v) in row {
                    fact(ui, cw, l, &v);
                }
            });
            ui.add_space(10.0);
        }
        ui.label(RichText::new("Abilities").size(11.5).color(MUTED()));
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            match e {
                Some(e) => {
                    for a in &e.abilities {
                        info_chip(ui, "◆", kit::primary(), a);
                    }
                    if !e.hidden_ability.is_empty() {
                        info_chip(ui, "◇", MUTED(), &format!("{} (hidden, newer games)", e.hidden_ability));
                    }
                    if e.abilities.is_empty() && e.hidden_ability.is_empty() {
                        ui.label(RichText::new("—").color(MUTED()));
                    }
                }
                None => {
                    ui.label(RichText::new("…").color(MUTED()));
                }
            }
        });
    });
}

/// What the Breeding card's buttons ask for.
enum BreedGo {
    Plan,
    Partners,
}

/// Egg groups, egg cycles, the gender split as a bar, and the way into the
/// Breeding Planner. Uses the app's own breeding data, so it works offline.
fn breeding_card(ui: &mut egui::Ui, m: &crate::wiki::Mon, e: Option<&crate::dexextra::Extra>) -> Option<BreedGo> {
    let info = crate::breed::info(&m.id);
    let mut go = None;
    kit::card(ui, |ui| {
        ui.set_width(ui.available_width());
        card_title(ui, "Breeding");
        let cw = ((ui.available_width() - 16.0) / 2.0).floor();
        let groups = match info {
            Some(i) => i.groups.join(", "),
            None => val(e, |e| e.egg_groups.join(", ")),
        };
        let cycles = match info {
            Some(i) => format!("{} (about {} steps)", i.cycles, super::sim_home::thousands(crate::breed::hatch_steps(i.cycles, false, false))),
            None => val(e, |e| e.egg_cycles.clone()),
        };
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            fact(ui, cw, "Egg groups", &groups);
            fact(ui, cw, "Egg cycles", &cycles);
        });
        ui.add_space(12.0);
        ui.label(RichText::new("Gender").size(11.5).color(MUTED()));
        ui.add_space(4.0);
        // male share 0..=1, None when genderless
        let male: Option<f32> = match info {
            Some(i) => i.female_chance().map(|f| 1.0 - f as f32),
            None => {
                let g = e.map(|e| e.gender.clone()).unwrap_or_default();
                g.split('%').next().and_then(|s| s.trim().parse::<f32>().ok()).filter(|_| g.contains("male")).map(|v| v / 100.0)
            }
        };
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 10.0), Sense::hover());
        let p = ui.painter();
        let blue = Color32::from_rgb(86, 150, 240);
        let pink = Color32::from_rgb(240, 120, 170);
        match male {
            Some(mp) => {
                let mw = r.width() * mp.clamp(0.0, 1.0);
                p.rect_filled(r, 5.0, pink);
                if mw > 0.0 {
                    let rounding = if mp >= 1.0 { egui::Rounding::same(5.0) } else { egui::Rounding { nw: 5.0, sw: 5.0, ne: 0.0, se: 0.0 } };
                    p.rect_filled(Rect::from_min_size(r.min, vec2(mw, r.height())), rounding, blue);
                }
            }
            None => {
                p.rect_filled(r, 5.0, theme::blend(CARD(), TEXT(), 0.12));
            }
        }
        ui.add_space(4.0);
        let label = match info {
            Some(i) => i.gender_label().to_string(),
            None => e.map(|e| e.gender.replace("male,", "male ·")).unwrap_or_else(|| "…".into()),
        };
        ui.label(RichText::new(label).size(12.5).color(TEXT()));
        // Into the Breeding Planner.
        if let Some(i) = info {
            ui.add_space(10.0);
            if i.no_eggs() {
                ui.label(RichText::new("Can't make Eggs itself.").size(12.5).color(MUTED()));
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                if !super::app_modal::have(crate::apps::AppId::Breed) {
                    super::app_modal::lock_btn(ui, crate::apps::AppId::Breed, "Plan breeding it");
                } else if kit::accent_button_w(ui, egui::Button::new(RichText::new("🐣  Plan breeding it").size(12.5)), 8.0, 0.0)
                    .on_hover_text("Open the Breeding Planner with this as the goal")
                    .clicked()
                {
                    go = Some(BreedGo::Plan);
                }
                if !i.no_eggs() && super::team_home::soft(ui, "Who it breeds with").on_hover_text("Open the Day Care with it, to see every partner").clicked() {
                    go = Some(BreedGo::Partners);
                }
            });
        }
    });
    go
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

/// Base stats as bars, with the total and what it's best at.
fn stats_card(ui: &mut egui::Ui, m: &crate::wiki::Mon, e: Option<&crate::dexextra::Extra>) {
    kit::card(ui, |ui| {
        ui.set_width(ui.available_width());
        card_title(ui, "Base stats");
        let Some(e) = e.filter(|e| e.stats.len() == 6) else {
            ui.label(RichText::new("…").color(MUTED()));
            return;
        };
        let names = ["HP", "Attack", "Defense", "Sp. Atk", "Sp. Def", "Speed"];
        let best = e.stats.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i).unwrap_or(0);
        for (i, (n, v)) in names.iter().zip(&e.stats).enumerate() {
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
            let p = ui.painter();
            p.text(r.left_center(), Align2::LEFT_CENTER, *n, super::charts::reg(12.5), if i == best { TEXT() } else { MUTED() });
            p.text(r.left_center() + vec2(100.0, 0.0), Align2::RIGHT_CENTER, v.to_string(), super::charts::bold(13.5), TEXT());
            let track = Rect::from_min_max(pos2(r.left() + 114.0, r.center().y - 5.0), pos2(r.right(), r.center().y + 5.0));
            p.rect_filled(track, 5.0, theme::blend(CARD(), TEXT(), 0.08));
            let f = kit::grow(ui.ctx(), egui::Id::new(("dex-stat", m.dex, i)), (*v as f32 / 180.0).clamp(0.03, 1.0));
            p.rect_filled(Rect::from_min_size(track.min, vec2(track.width() * f, 10.0)), 5.0, stat_color(*v));
            if let Some(old) = e.modern_stats.get(i).filter(|o| **o != *v) {
                resp.on_hover_text(format!("{old} in newer games"));
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Total").size(12.5).color(MUTED()));
            ui.label(RichText::new(e.total.to_string()).font(super::charts::heavy(18.0)).color(TEXT()));
            ui.label(RichText::new(format!("· best at {}", names[best])).size(12.5).color(MUTED()));
        });
        let note = if !e.modern_stats.is_empty() {
            let old: u32 = e.modern_stats.iter().sum();
            format!("As in Platinum. Newer games: {old} total.")
        } else if m.modern {
            "As in the latest games.".to_string()
        } else {
            "Same as in Platinum.".to_string()
        };
        ui.label(RichText::new(note).size(11.5).color(MUTED()));
    });
}

/// Every attacking type and how much it hurts, as a grid.
fn defenses_card(ui: &mut egui::Ui, m: &crate::wiki::Mon) {
    kit::card(ui, |ui| {
        ui.set_width(ui.available_width());
        card_title(ui, "Type matchups");
        ui.label(RichText::new("Damage it takes from each attacking type").size(12.0).color(MUTED()));
        ui.add_space(10.0);
        let fx = crate::dexextra::defenses(&m.types);
        let full = ui.available_width();
        let cols = if full >= 420.0 { 3 } else { 2 };
        let gap = 6.0;
        let cw = ((full - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        for row in fx.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (t, x) in row {
                    let (r, _) = ui.allocate_exact_size(vec2(cw, 28.0), Sense::hover());
                    let p = ui.painter();
                    let (bg, label, strong) = match x {
                        400 => (Color32::from_rgb(232, 72, 72), "×4", true),
                        200 => (Color32::from_rgb(240, 120, 80), "×2", true),
                        50 => (Color32::from_rgb(72, 180, 120), "×½", true),
                        25 => (Color32::from_rgb(52, 160, 110), "×¼", true),
                        0 => (Color32::from_rgb(120, 120, 140), "×0", true),
                        _ => (theme::blend(CARD(), TEXT(), 0.12), "×1", false),
                    };
                    p.rect_filled(r, 8.0, if strong { bg.gamma_multiply(0.22) } else { theme::blend(CARD(), TEXT(), 0.04) });
                    let tc = type_color(t);
                    p.rect_filled(Rect::from_min_size(r.min + vec2(6.0, 8.0), vec2(4.0, 12.0)), 2.0, tc);
                    p.text(pos2(r.min.x + 16.0, r.center().y), Align2::LEFT_CENTER, *t, super::charts::reg(12.0), if strong { TEXT() } else { MUTED() });
                    p.text(pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, label, super::charts::bold(12.5), if strong { theme::blend(bg, TEXT(), 0.2) } else { MUTED() });
                }
            });
            ui.add_space(gap);
        }
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
