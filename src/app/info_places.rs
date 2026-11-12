//! Game Info › Places: Sinnoh in story order (a timeline from the first
//! badge to after the League), and a page per place: its wild Pokémon by
//! method and time of day (MORE and MODERN marked, and switchable), its
//! items, today's rare grass there, and a jump to the Map.

use super::info_home::Page;
use super::info_items::{how_color, HOWS};
use super::pokedex::type_color;
use super::profile_page::card_head;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::theme;
use crate::wiki::{Item, Wiki, Wild};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Encounter methods, in the order they're shown.
const METHODS: [&str; 7] = ["grass", "cave", "surf", "old_rod", "good_rod", "super_rod", "radar"];

fn method_name(m: &str) -> &'static str {
    super::dex_tabs::method_label(m)
}

fn method_icon(m: &str) -> &'static str {
    match m {
        "grass" => "🌿",
        "cave" => "🗻",
        "surf" => "🌊",
        "old_rod" | "good_rod" | "super_rod" => "🎣",
        "radar" => "📡",
        _ => "✨",
    }
}

#[derive(Default)]
pub(super) struct PlacesState {
    pub query: String,
    /// Places list: only those with this method.
    pub method: Option<String>,
    /// Place page: open method, time of day (0 morning, 1 day, 2 night), area.
    pub tab: Option<String>,
    pub time: Option<usize>,
    pub area: Option<String>,
}

/// A place: its name, its areas (place keys), when you can first get there.
#[derive(Clone)]
pub(super) struct PlaceGroup {
    pub name: String,
    pub keys: Vec<String>,
    pub stage: usize,
    pub cave: bool,
}

pub(super) fn groups(w: &Wiki) -> Vec<PlaceGroup> {
    let mut g: BTreeMap<String, PlaceGroup> = BTreeMap::new();
    for (k, p) in &w.data.places {
        let name = if p.group.is_empty() { p.name.clone() } else { p.group.clone() };
        let e = g.entry(name.clone()).or_insert(PlaceGroup { name, keys: Vec::new(), stage: usize::MAX, cave: false });
        e.keys.push(k.clone());
        e.stage = e.stage.min(w.stage_index(&p.stage));
        e.cave |= p.cave;
    }
    let mut v: Vec<PlaceGroup> = g.into_values().collect();
    v.sort_by(|a, b| a.stage.cmp(&b.stage).then(natural(&a.name).cmp(&natural(&b.name))));
    v
}

/// "Route 9" before "Route 10".
fn natural(s: &str) -> (String, u32) {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    let head: String = s.chars().take_while(|c| !c.is_ascii_digit()).collect();
    (head, digits.parse().unwrap_or(0))
}

/// The group a place name (from an item spot, a tutor…) belongs to, if any.
pub(super) fn group_of(w: &Wiki, place: &str) -> Option<String> {
    let pl = place.to_lowercase();
    let mut best: Option<(usize, String)> = None;
    for p in w.data.places.values() {
        for cand in [&p.name, &p.group] {
            if cand.is_empty() {
                continue;
            }
            let c = cand.to_lowercase();
            let hit = pl == c || pl.starts_with(&format!("{c} ")) || pl.starts_with(&format!("{c},")) || pl.starts_with(&format!("{c} ("));
            if hit && best.as_ref().map_or(true, |b| c.len() > b.0) {
                let g = if p.group.is_empty() { p.name.clone() } else { p.group.clone() };
                best = Some((c.len(), g));
            }
        }
    }
    best.map(|b| b.1)
}

/// Items found at a place (by its names).
fn items_at<'a>(w: &'a Wiki, g: &PlaceGroup) -> Vec<(&'a Item, Vec<&'a crate::wiki::ItemSpot>)> {
    let mut names: Vec<String> = g.keys.iter().filter_map(|k| w.place(k)).map(|p| p.name.to_lowercase()).collect();
    names.push(g.name.to_lowercase());
    let hit = |place: &str| {
        let pl = place.to_lowercase();
        names.iter().any(|n| pl == *n || pl.starts_with(&format!("{n} ")) || pl.starts_with(&format!("{n},")))
    };
    w.data
        .items
        .iter()
        .filter_map(|it| {
            let spots: Vec<&crate::wiki::ItemSpot> = it.found.iter().filter(|s| s.how != "Held" && hit(&s.place)).collect();
            (!spots.is_empty()).then_some((it, spots))
        })
        .collect()
}

fn period_now() -> usize {
    match crate::stats::local_hour(crate::stats::now_unix()) {
        4..=9 => 0,
        10..=19 => 1,
        _ => 2,
    }
}

fn chance_at(e: &Wild, p: usize) -> u32 {
    [e.chance.morning, e.chance.day, e.chance.night][p]
}

fn small(t: impl Into<String>) -> RichText {
    RichText::new(t).font(crate::fonts::sans_font(12.0)).color(MUTED())
}

fn tag(ui: &mut egui::Ui, text: &str, col: Color32) -> egui::Response {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(10.5), col);
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(14.0, 6.0), Sense::hover());
    ui.painter().rect_filled(r, r.height() / 2.0, kit::tint(col, 0.16));
    ui.painter().galley(r.min + egui::vec2(7.0, 3.0), g, col);
    resp
}

/// A toggle chip. True when clicked.
fn chip(ui: &mut egui::Ui, text: &str, on: bool, col: Color32) -> bool {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(12.0), TEXT());
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(20.0, 12.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let fill = if on { theme::blend(CARD(), col, 0.24) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) };
    ui.painter().rect_filled(r, r.height() / 2.0, fill);
    ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, if on { theme::blend(LINE(), col, 0.8) } else { theme::blend(LINE(), col, 0.3 * ht) }));
    ui.painter().galley(r.min + egui::vec2(10.0, 6.0), g, TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// A short name for a badge stage: "Before Roark" → "Badge 1", "After the League" → "Postgame".
fn stage_badge(i: usize, name: &str) -> String {
    if name.to_lowercase().starts_with("after") {
        "Postgame".into()
    } else if name.to_lowercase().contains("league") {
        "8 badges".into()
    } else if i == 0 {
        "Start".into()
    } else {
        format!("{i} badge{}", if i == 1 { "" } else { "s" })
    }
}

impl App {
    pub(super) fn page_places(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        self.info_back_bar(ui, "Places");
        let all = groups(w);
        let pl = theme::pal();
        let full = ui.available_width();
        let slot = ui.painter().add(egui::Shape::Noop);
        let caves = all.iter().filter(|g| g.cave).count();
        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 20.0)).show(ui, |ui| {
            ui.set_width(full - 56.0);
            ui.label(RichText::new("PLACES").font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(MUTED(), kit::C_GREEN, 0.6)));
            ui.add_space(4.0);
            ui.label(RichText::new("Sinnoh, in story order").font(crate::fonts::sans_bold_font(24.0)).color(TEXT()));
            ui.add_space(2.0);
            ui.label(RichText::new(format!("{} places with wild Pokémon ({caves} caves), from your first steps to after the League. Each lists who lives there and what's lying around.", all.len())).size(13.0).color(MUTED()));
            ui.add_space(14.0);
            let width = ui.available_width();
            kit::search_box_keyed(ui, Some(egui::Id::new("places-search")), &mut self.setup.info.places.query, "Find a place, or a Pokémon to see where it lives", width);
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.label(small("With"));
                for m in ["grass", "cave", "surf", "super_rod", "radar"] {
                    let label = match m {
                        "super_rod" => "Fishing".to_string(),
                        other => method_name(other).to_string(),
                    };
                    let on = self.setup.info.places.method.as_deref() == Some(m);
                    if chip(ui, &format!("{}  {label}", method_icon(m)), on, kit::C_GREEN) {
                        self.setup.info.places.method = if on { None } else { Some(m.to_string()) };
                    }
                }
            });
        });
        let rect = frame.response.rect;
        ui.painter().set(
            slot,
            egui::Shape::Vec(vec![
                kit::card_shadow().as_shape(rect, 18.0).into(),
                theme::gradient_rect(rect, [18.0; 4], theme::blend(CARD(), kit::C_GREEN, if pl.light { 0.12 } else { 0.20 }), theme::blend(CARD(), pl.accent, if pl.light { 0.05 } else { 0.10 })),
                egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_GREEN, 0.35))),
            ]),
        );
        ui.add_space(20.0);

        // filter: a place's name, or a Pokémon that lives there
        let q = self.setup.info.places.query.trim().to_lowercase();
        let mon_ids: Vec<String> = if q.chars().count() >= 3 {
            w.data.species.iter().filter(|m| m.name.to_lowercase().starts_with(&q)).map(|m| m.id.clone()).collect()
        } else {
            Vec::new()
        };
        let method = self.setup.info.places.method.clone();
        let fishing = ["old_rod", "good_rod", "super_rod"];
        let shown: Vec<&PlaceGroup> = all
            .iter()
            .filter(|g| {
                q.is_empty()
                    || g.name.to_lowercase().contains(&q)
                    || w.data.wild.iter().any(|e| g.keys.contains(&e.area) && mon_ids.contains(&e.species))
            })
            .filter(|g| match &method {
                None => true,
                Some(m) if m == "super_rod" => w.data.wild.iter().any(|e| g.keys.contains(&e.area) && fishing.contains(&e.method.as_str())),
                Some(m) => w.data.wild.iter().any(|e| g.keys.contains(&e.area) && &e.method == m),
            })
            .collect();
        if !mon_ids.is_empty() && !shown.is_empty() {
            let names: Vec<String> = mon_ids.iter().filter_map(|i| w.mon(i)).map(|m| m.name.clone()).take(3).collect();
            ui.label(small(format!("Places where {} live, and places named like \"{q}\"", names.join(", "))));
            ui.add_space(8.0);
        }
        if shown.is_empty() {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(small("No places match. Try a shorter name or clear the filter."));
            });
            return;
        }
        // the timeline: one stretch per badge stage
        let mut open: Option<String> = None;
        let rail = 56.0;
        let at = self.story_stage();
        for (si, stage) in w.data.stages.iter().enumerate() {
            let here: Vec<&&PlaceGroup> = shown.iter().filter(|g| g.stage == si).collect();
            if here.is_empty() {
                continue;
            }
            let locked = at.is_some_and(|a| si > a);
            let next = at.is_some_and(|a| si == a + 1);
            let top = ui.cursor().min.y;
            ui.horizontal_top(|ui| {
                ui.add_space(rail);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(stage).font(crate::fonts::sans_bold_font(17.0)).color(if locked { MUTED() } else { TEXT() }));
                        ui.label(small(format!("{} · {}", stage_badge(si, stage), here.len())));
                        if next {
                            tag(ui, "YOUR NEXT BADGE", kit::C_TEAL);
                        } else if at == Some(si) {
                            tag(ui, "YOU'RE HERE", kit::C_GREEN);
                        } else if locked {
                            tag(ui, "LATER", kit::C_SLATE);
                        }
                    });
                    ui.add_space(10.0);
                    let avail = ui.available_width();
                    let cols = if avail >= 1100.0 { 4 } else if avail >= 800.0 { 3 } else if avail >= 520.0 { 2 } else { 1 };
                    let gap = 12.0;
                    let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
                    for row in here.chunks(cols) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for g in row {
                                if self.place_card(ui, w, g, cw, locked) {
                                    open = Some(g.name.clone());
                                }
                            }
                        });
                        ui.add_space(gap);
                    }
                });
            });
            // the rail: a numbered stop and a line down to the next
            let bottom = ui.cursor().min.y;
            let x = ui.min_rect().min.x + 20.0;
            let c = egui::pos2(x, top + 12.0);
            let p = ui.painter();
            p.line_segment([egui::pos2(x, top + 26.0), egui::pos2(x, bottom + 4.0)], Stroke::new(2.0f32, kit::tint(kit::C_GREEN, 0.35)));
            let rc = if locked { kit::C_SLATE } else { kit::C_GREEN };
            p.circle_filled(c, 13.0, theme::blend(CARD(), rc, 0.35));
            p.circle_stroke(c, 13.0, Stroke::new(1.5f32, rc));
            let label = if stage.to_lowercase().starts_with("after") { "★".to_string() } else { (si + 1).to_string() };
            p.text(c, Align2::CENTER_CENTER, label, crate::fonts::sans_bold_font(12.0), TEXT());
            ui.add_space(14.0);
        }
        if let Some(g) = open {
            self.info_push(Page::Place(g));
        }
    }

    /// A place in the list: name, cave or not, how many Pokémon and items, methods.
    fn place_card(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, g: &PlaceGroup, cw: f32, locked: bool) -> bool {
        let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 86.0), Sense::click());
        if !ui.is_rect_visible(r) {
            return false;
        }
        let wild: Vec<&Wild> = w.data.wild.iter().filter(|e| g.keys.contains(&e.area)).collect();
        let mut kinds: Vec<&str> = wild.iter().map(|e| e.species.as_str()).collect();
        kinds.sort_unstable();
        kinds.dedup();
        let mut methods: Vec<&str> = wild.iter().map(|e| e.method.as_str()).collect();
        methods.sort_by_key(|m| METHODS.iter().position(|x| x == m).unwrap_or(9));
        methods.dedup();
        // fishing is one icon
        let mut icons: Vec<&str> = methods.iter().map(|m| method_icon(m)).collect();
        icons.dedup();
        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
        let col = if locked {
            kit::C_SLATE
        } else if g.cave {
            Color32::from_rgb(170, 130, 90)
        } else {
            kit::C_GREEN
        };
        let p = ui.painter_at(r.expand(4.0));
        p.add(kit::card_shadow().as_shape(r, 14.0));
        p.rect_filled(r, 14.0, theme::blend(CARD(), CARD_HOVER(), ht));
        p.rect_stroke(r, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.2 + 0.5 * ht)));
        p.rect_filled(Rect::from_min_size(r.min + egui::vec2(0.0, 14.0), egui::vec2(3.0, r.height() - 28.0)), 1.5, col);
        let f = crate::fonts::sans_bold_font(14.5);
        p.text(egui::pos2(r.min.x + 16.0, r.min.y + 22.0), Align2::LEFT_CENTER, super::charts::clip(ui, &g.name, &f, cw - 32.0), f, if locked { MUTED() } else { TEXT() });
        let sub = format!("{} Pokémon{}", kinds.len(), if g.keys.len() > 1 { format!(" · {} areas", g.keys.len()) } else { String::new() });
        p.text(egui::pos2(r.min.x + 16.0, r.min.y + 42.0), Align2::LEFT_CENTER, sub, crate::fonts::sans_font(11.5), MUTED());
        // how you meet them: "Grass · Surfing · Fishing"
        let mut names: Vec<&str> = methods.iter().map(|m| if m.ends_with("rod") { "Fishing" } else { method_name(m) }).collect();
        names.dedup();
        let f3 = crate::fonts::sans_font(11.0);
        let line = super::charts::clip(ui, &names.join(" · "), &f3, cw - 80.0);
        p.text(egui::pos2(r.min.x + 16.0, r.max.y - 18.0), Align2::LEFT_CENTER, line, f3, theme::blend(MUTED(), col, 0.5));
        let _ = &icons;
        if g.cave {
            p.text(egui::pos2(r.max.x - 14.0, r.max.y - 18.0), Align2::RIGHT_CENTER, "CAVE", crate::fonts::sans_bold_font(9.5), col);
        }
        let tip: Vec<&str> = methods.iter().map(|m| method_name(m)).collect();
        resp.on_hover_text(tip.join(" · ")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    }

    // ───────────────────────────── one place ─────────────────────────────

    pub(super) fn page_place(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, name: &str) {
        let all = groups(w);
        let Some(g) = all.iter().find(|g| g.name == name).cloned() else {
            self.info_back_bar(ui, "Place");
            ui.label(small("This place isn't in the game's data any more."));
            return;
        };
        self.info_back_bar(ui, &g.name);
        let stage = w.data.stages.get(g.stage).cloned().unwrap_or_default();
        let wild_all: Vec<&Wild> = w.data.wild.iter().filter(|e| g.keys.contains(&e.area)).collect();
        let items = items_at(w, &g);
        let mut kinds: Vec<&str> = wild_all.iter().map(|e| e.species.as_str()).collect();
        kinds.sort_unstable();
        kinds.dedup();
        let pl = theme::pal();
        let col = if g.cave { Color32::from_rgb(170, 130, 90) } else { kit::C_GREEN };

        // ── hero
        let full = ui.available_width();
        let slot = ui.painter().add(egui::Shape::Noop);
        let mut to_map = false;
        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 22.0)).show(ui, |ui| {
            ui.set_width(full - 56.0);
            ui.label(RichText::new(format!("PLACE · {}", stage.to_uppercase())).font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(MUTED(), col, 0.6)));
            ui.add_space(4.0);
            ui.label(RichText::new(&g.name).font(crate::fonts::sans_bold_font(30.0)).color(TEXT()));
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                tag(ui, &stage_badge(g.stage, &stage).to_uppercase(), col).on_hover_text(format!("You can first get here {}", stage.to_lowercase()));
                tag(ui, if g.cave { "CAVE" } else { "OUTDOORS" }, kit::C_SLATE);
                tag(ui, &format!("{} POKéMON", kinds.len()), kit::C_RED);
                tag(ui, &format!("{} ITEMS", items.len()), kit::C_GOLD);
                if g.keys.len() > 1 {
                    tag(ui, &format!("{} AREAS", g.keys.len()), kit::C_SLATE);
                }
            });
            ui.add_space(14.0);
            to_map = ui.add(egui::Button::new(RichText::new("🗺  Show on the Map").size(13.0)).fill(kit::btn_fill())).on_hover_text("Open the Map on this place").clicked();
        });
        let rect = frame.response.rect;
        ui.painter().set(
            slot,
            egui::Shape::Vec(vec![
                kit::card_shadow().as_shape(rect, 18.0).into(),
                theme::gradient_rect(rect, [18.0; 4], theme::blend(CARD(), col, if pl.light { 0.16 } else { 0.26 }), theme::blend(CARD(), col, if pl.light { 0.04 } else { 0.07 })),
                egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.4))),
            ]),
        );
        if to_map {
            let title = g.name.clone();
            self.show_on_map(&g.keys, title, None);
            return;
        }
        ui.add_space(18.0);
        if let Some(at) = self.story_stage().filter(|a| g.stage > *a) {
            let need = super::info_story::stage_words(g.stage, &stage);
            let have = w.data.stages.get(at).map(|n| super::info_story::stage_words(at, n)).unwrap_or_default();
            egui::Frame::none()
                .fill(theme::blend(CARD(), kit::C_SLATE, 0.14))
                .stroke(Stroke::new(1.0f32, theme::blend(LINE(), kit::C_SLATE, 0.5)))
                .rounding(12.0)
                .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(format!("Not yet: you can get here at {}", need.to_lowercase())).font(crate::fonts::sans_bold_font(13.5)).color(TEXT()));
                    ui.label(small(format!("You said you're at {}. Change it in Game Info › Your adventure.", have.to_lowercase())));
                });
            ui.add_space(14.0);
        }
        let avail = ui.available_width();
        let gap = 18.0;
        if avail >= 1000.0 {
            let left_w = ((avail - gap) * 0.62).floor();
            let right_w = avail - gap - left_w;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(left_w);
                    self.place_wild(ui, w, &g, &wild_all);
                });
                ui.allocate_ui_with_layout(egui::vec2(right_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(right_w);
                    self.place_side(ui, w, &g, &items, &all);
                });
            });
        } else {
            self.place_wild(ui, w, &g, &wild_all);
            ui.add_space(gap);
            self.place_side(ui, w, &g, &items, &all);
        }
    }

    /// Who lives here: by method, at a time of day, in one area or all.
    fn place_wild(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, g: &PlaceGroup, wild_all: &[&Wild]) {
        let now = period_now();
        let st = &self.setup.info.places;
        // forget an area or method from another place
        let area = st.area.clone().filter(|a| g.keys.contains(a));
        let time = st.time.unwrap_or(now);
        let (hide_more, hide_modern) = (!self.cfg.info_more, !self.cfg.info_modern);
        let at = self.story_stage();
        let shown_world = |e: &&&Wild| !(hide_more && e.setting == "more") && !(hide_modern && e.setting == "modern");
        let in_area: Vec<&&Wild> = wild_all.iter().filter(|e| area.as_ref().map_or(true, |a| &e.area == a)).filter(shown_world).collect();
        let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
        for e in &in_area {
            *counts.entry(METHODS.iter().position(|m| *m == e.method).unwrap_or(9)).or_default() += 1;
        }
        let tab = st
            .tab
            .clone()
            .filter(|t| in_area.iter().any(|e| &e.method == t))
            .or_else(|| counts.keys().next().and_then(|i| METHODS.get(*i)).map(|s| s.to_string()));
        let mut set_tab: Option<String> = None;
        let mut set_time: Option<usize> = None;
        let mut set_area: Option<Option<String>> = None;
        let mut flip_more = false;
        let mut flip_modern = false;
        let mut open: Option<String> = None;
        kit::card(ui, |ui| {
            card_head(ui, "🐾", kit::C_RED, "Wild Pokémon", None);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let mut t = time;
                let labels = [
                    if now == 0 { "Morning (now)" } else { "Morning" },
                    if now == 1 { "Day (now)" } else { "Day" },
                    if now == 2 { "Night (now)" } else { "Night" },
                ];
                if kit::segmented(ui, &mut t, &[(0usize, labels[0]), (1, labels[1]), (2, labels[2])]) {
                    set_time = Some(t);
                }
            });
            if wild_all.is_empty() {
                ui.add_space(6.0);
                ui.label(small("No wild Pokémon listed here."));
                return;
            }
            ui.add_space(10.0);
            // areas
            if g.keys.len() > 1 {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    if chip(ui, "All areas", area.is_none(), kit::C_GREEN) {
                        set_area = Some(None);
                    }
                    for k in &g.keys {
                        let n = w.place(k).map(|p| p.name.clone()).unwrap_or_else(|| k.clone());
                        let short = n.strip_prefix(&g.name).map(|s| s.trim().trim_matches(['(', ')']).trim().to_string()).filter(|s| !s.is_empty()).unwrap_or(n);
                        if chip(ui, &short, area.as_deref() == Some(k.as_str()), kit::C_GREEN) {
                            set_area = Some(Some(k.clone()));
                        }
                    }
                });
                ui.add_space(8.0);
            }
            // methods
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for (i, n) in &counts {
                    let Some(m) = METHODS.get(*i) else { continue };
                    let on = tab.as_deref() == Some(*m);
                    if chip(ui, &format!("{}  {}  {n}", method_icon(m), method_name(m)), on, kit::C_RED) {
                        set_tab = Some(m.to_string());
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(small("Your world has:"));
                if chip(ui, "MORE POKéMON", !hide_more, kit::C_INDIGO) {
                    flip_more = true;
                }
                if chip(ui, "MODERN POKéMON", !hide_modern, kit::C_INDIGO) {
                    flip_modern = true;
                }
            });
            ui.add_space(10.0);
            let Some(tab) = tab.clone() else {
                ui.label(small("Nothing here with these options."));
                return;
            };
            let mut rows: Vec<&&&Wild> = in_area.iter().filter(|e| e.method == tab).collect();
            rows.sort_by_key(|e| (std::cmp::Reverse(chance_at(e, time)), e.species.clone()));
            if tab == "radar" {
                if let Some(t) = w.data.text.get("radar") {
                    ui.add(egui::Label::new(small(t)).wrap());
                    ui.add_space(6.0);
                }
            }
            let most = rows.iter().map(|e| chance_at(e, time)).max().unwrap_or(1).max(1) as f32;
            for e in rows {
                let c = chance_at(e, time);
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), Sense::click());
                if !ui.is_rect_visible(r) {
                    continue;
                }
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter_at(r.expand(2.0));
                if ht > 0.0 {
                    p.rect_filled(r, 10.0, theme::blend(CARD(), CARD_HOVER(), ht));
                }
                let later = at.is_some_and(|a| w.stage_index(&e.stage) > a);
                let dim = c == 0 || later;
                // a whole place that's later says so once, at the top
                let place_later = at.is_some_and(|a| g.stage > a);
                let mon = w.mon(&e.species).cloned();
                let name = mon.as_ref().map(|m| m.name.clone()).unwrap_or_else(|| e.species.clone());
                let tc = mon.as_ref().and_then(|m| m.types.first()).map(|t| type_color(t)).unwrap_or(MUTED());
                let sc = egui::pos2(r.min.x + 26.0, r.center().y);
                p.circle_filled(sc, 21.0, theme::blend(CARD(), tc, if dim { 0.08 } else { 0.2 }));
                if let Some(sp) = self.info_species(&name) {
                    if self.sprites.get(&sp, false).is_some() {
                        self.draw_sprite_centered(&p, &sp, false, sc, egui::vec2(42.0, 42.0));
                    }
                }
                let ink = if dim { MUTED() } else { TEXT() };
                let x = r.min.x + 56.0;
                let mut title = name.clone();
                match e.setting.as_str() {
                    "more" => title.push_str("  · MORE"),
                    "modern" => title.push_str("  · MODERN"),
                    _ => {}
                }
                if later && !place_later {
                    title.push_str(&format!("  · LATER, {}", e.stage));
                }
                p.text(egui::pos2(x, r.center().y - 9.0), Align2::LEFT_CENTER, title, crate::fonts::sans_bold_font(13.5), ink);
                let area_name = if area.is_none() && g.keys.len() > 1 { w.place(&e.area).map(|p| format!(" · {}", p.name)).unwrap_or_default() } else { String::new() };
                let types = mon.as_ref().map(|m| m.types.join(" / ")).unwrap_or_default();
                let note = if e.note.is_empty() { String::new() } else { format!(" · {}", e.note) };
                let f2 = crate::fonts::sans_font(11.5);
                let bar_w = 150.0;
                p.text(egui::pos2(x, r.center().y + 10.0), Align2::LEFT_CENTER, super::charts::clip(ui, &format!("Lv {} · {types}{area_name}{note}", e.levels), &f2, r.width() - 56.0 - bar_w - 30.0), f2, MUTED());
                // the chance at this time, as a bar
                let bar = Rect::from_min_size(egui::pos2(r.max.x - bar_w - 54.0, r.center().y - 4.0), egui::vec2(bar_w, 8.0));
                p.rect_filled(bar, 4.0, kit::tint(LINE(), 0.9));
                let f = c as f32 / most;
                if f > 0.0 {
                    let bc = if later { kit::tint(MUTED(), 0.6) } else { tc };
                    p.rect_filled(Rect::from_min_size(bar.min, egui::vec2((bar.width() * f).max(6.0), 8.0)), 4.0, bc);
                }
                p.text(egui::pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, if c == 0 { "—".to_string() } else { format!("{c}%") }, crate::fonts::sans_bold_font(13.5), ink);
                let others = format!("Morning {}% · Day {}% · Night {}%", e.chance.morning, e.chance.day, e.chance.night);
                if resp.on_hover_text(format!("{others}\nOpen in the Pokédex")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(e.species.clone());
                }
            }
            ui.add_space(6.0);
            if let Some(t) = w.data.text.get("chances") {
                ui.add(egui::Label::new(small(t)).wrap());
            }
        });
        let s = &mut self.setup.info.places;
        if let Some(t) = set_tab {
            s.tab = Some(t);
        }
        if let Some(t) = set_time {
            s.time = Some(t);
        }
        if let Some(a) = set_area {
            s.area = a;
        }
        if flip_more {
            self.cfg.info_more = !self.cfg.info_more;
        }
        if flip_modern {
            self.cfg.info_modern = !self.cfg.info_modern;
        }
        if flip_more || flip_modern {
            self.cfg.save();
        }
        if let Some(id) = open {
            self.info_open_dex(Some(id));
        }
    }

    /// Items here, today's rare grass here, and other places at the same point in the story.
    fn place_side(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, g: &PlaceGroup, items: &[(&Item, Vec<&crate::wiki::ItemSpot>)], all: &[PlaceGroup]) {
        let mut open_item: Option<String> = None;
        let mut open_place: Option<String> = None;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                card_head(ui, "🎒", kit::C_GOLD, "Items here", None);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(small(format!("{}", items.len())));
                });
            });
            ui.add_space(8.0);
            if items.is_empty() {
                ui.label(small("None listed."));
                return;
            }
            let mut by_how: BTreeMap<usize, Vec<(&Item, &crate::wiki::ItemSpot)>> = BTreeMap::new();
            for (it, spots) in items {
                for s in spots {
                    by_how.entry(HOWS.iter().position(|h| *h == s.how).unwrap_or(HOWS.len())).or_default().push((it, s));
                }
            }
            for (hi, list) in by_how {
                let how = HOWS.get(hi).copied().unwrap_or("Other");
                let col = how_color(how);
                ui.label(RichText::new(how.to_uppercase()).font(crate::fonts::sans_bold_font(10.5)).color(col));
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    let mut seen: Vec<&str> = Vec::new();
                    for (it, s) in list {
                        if seen.contains(&it.id.as_str()) {
                            continue;
                        }
                        seen.push(&it.id);
                        let gal = ui.painter().layout_no_wrap(it.name.clone(), crate::fonts::sans_font(12.0), TEXT());
                        let (r, resp) = ui.allocate_exact_size(gal.size() + egui::vec2(18.0, 10.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        ui.painter().rect_filled(r, r.height() / 2.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                        ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.3 + 0.4 * ht)));
                        ui.painter().galley(r.min + egui::vec2(9.0, 5.0), gal, TEXT());
                        let tip = if s.detail.is_empty() { s.place.clone() } else { format!("{}: {}", s.place, s.detail) };
                        if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open_item = Some(it.id.clone());
                        }
                    }
                });
                ui.add_space(10.0);
            }
        });
        // today's rare grass here (from the game's map files)
        let rare: Vec<super::rare::RareSpot> = {
            let lower = g.name.to_lowercase();
            self.rare_spots_today().into_iter().filter(|s| s.place.to_lowercase().starts_with(&lower)).collect()
        };
        if !rare.is_empty() {
            ui.add_space(18.0);
            let mut show: Option<usize> = None;
            kit::card(ui, |ui| {
                card_head(ui, "🌿", kit::C_GREEN, "Rare grass here today", None);
                ui.add_space(8.0);
                for (i, s) in rare.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let p = ui.painter_at(r.expand(2.0));
                    p.rect_filled(r, 10.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                    let sc = egui::pos2(r.min.x + 22.0, r.center().y);
                    if self.sprites.get(&s.species, false).is_some() {
                        self.draw_sprite_centered(&p, &s.species, false, sc, egui::vec2(36.0, 36.0));
                    }
                    p.text(egui::pos2(r.min.x + 48.0, r.center().y - 8.0), Align2::LEFT_CENTER, s.species.name, crate::fonts::sans_bold_font(13.0), TEXT());
                    p.text(egui::pos2(r.min.x + 48.0, r.center().y + 9.0), Align2::LEFT_CENTER, &s.place, crate::fonts::sans_font(11.0), MUTED());
                    p.text(egui::pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, "Map ›", crate::fonts::sans_font(12.0), if ht > 0.5 { TEXT() } else { MUTED() });
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        show = Some(i);
                    }
                }
            });
            if let Some(i) = show {
                if let (Some(map), Some(s)) = (self.game_map(), rare.get(i)) {
                    let title = format!("{} on rare grass", s.species.name);
                    self.show_rare_spot(&map, s.matrix, s.x, s.y, title);
                }
            }
        }
        // the same point in the story
        let same: Vec<&PlaceGroup> = all.iter().filter(|x| x.stage == g.stage && x.name != g.name).collect();
        if !same.is_empty() {
            ui.add_space(18.0);
            kit::card(ui, |ui| {
                let stage = w.data.stages.get(g.stage).cloned().unwrap_or_default();
                card_head(ui, "🗺", kit::C_TEAL, "Also reachable then", None);
                ui.label(small(format!("Other places you can get to {}", stage.to_lowercase())));
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for x in same {
                        if chip(ui, &x.name, false, kit::C_TEAL) {
                            open_place = Some(x.name.clone());
                        }
                    }
                });
            });
        }
        if let Some(id) = open_item {
            self.info_push(Page::Item(id));
        }
        if let Some(p) = open_place {
            self.info_push(Page::Place(p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_data() {
        let Ok(bytes) = std::fs::read("assets/wiki.json") else { return };
        let w = Wiki::parse(&bytes).expect("parses");
        let g = groups(&w);
        assert!(g.len() >= 60);
        // story order: the first place is reachable before Roark
        assert_eq!(g[0].stage, 0);
        assert!(g.windows(2).all(|p| p[0].stage <= p[1].stage));
        let r210 = g.iter().find(|x| x.name == "Route 210").expect("Route 210");
        assert_eq!(r210.keys.len(), 2);
        assert_eq!(group_of(&w, "Route 210 (south), Café Cabin").as_deref(), Some("Route 210"));
        assert_eq!(group_of(&w, "Iron Island B2F (left room)").as_deref(), Some("Iron Island"));
        assert!(group_of(&w, "Pickup (Ability)").is_none());
        assert!(!items_at(&w, r210).is_empty());
    }

    #[test]
    fn natural_order() {
        assert!(natural("Route 9") < natural("Route 10"));
    }
}
