//! Game Info › Items: the Bag (every item by pocket, filtered by how you get
//! it), every shop's stock and prices (the Poké Mart by badges), the
//! evolution items with what they evolve, and a page per item (all the ways
//! to get it, the first chance in the story, prices, what it's for).

use super::info_home::Page;
use super::pokedex::type_color;
use super::profile_page::card_head;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::theme;
use crate::wiki::{self, Item, ItemSpot, Wiki};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum ItemsView {
    #[default]
    Bag,
    Shops,
    Evolution,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum ItemSort {
    #[default]
    Name,
    Price,
    Ways,
}

#[derive(Default)]
pub(super) struct ItemsState {
    pub view: ItemsView,
    pub query: String,
    pub pocket: Option<String>,
    pub how: Option<String>,
    pub sort: ItemSort,
    /// Shops: the Poké Mart's stock up to this many badges.
    pub badges: Option<u8>,
}

/// The ways to get things, in the order they're shown.
pub(super) const HOWS: [&str; 9] = ["Ground", "Hidden", "Shop", "Gift", "Held", "Prize", "Berry tree", "Underground", "Other"];

pub(super) fn how_color(how: &str) -> Color32 {
    match how {
        "Ground" => kit::C_GREEN,
        "Hidden" => kit::C_VIOLET,
        "Shop" => kit::C_GOLD,
        "Gift" => kit::C_PINK,
        "Held" => kit::C_CYAN,
        "Prize" => kit::C_ORANGE,
        "Berry tree" => kit::C_TEAL,
        "Underground" => Color32::from_rgb(170, 130, 90),
        _ => kit::C_SLATE,
    }
}

fn how_icon(how: &str) -> &'static str {
    match how {
        "Ground" => "📍",
        "Hidden" => "🔍",
        "Shop" => "💰",
        "Gift" => "🎁",
        "Held" => "🐾",
        "Prize" => "🏆",
        "Berry tree" => "🌳",
        "Underground" => "⛏",
        _ => "✨",
    }
}

fn how_label(how: &str) -> &'static str {
    match how {
        "Ground" => "On the ground",
        "Hidden" => "Hidden",
        "Shop" => "In shops",
        "Gift" => "Gifts",
        "Held" => "Held by wild Pokémon",
        "Prize" => "Prizes",
        "Berry tree" => "Berry trees",
        "Underground" => "The Underground",
        _ => "Other ways",
    }
}

pub(super) fn pocket_meta(p: &str) -> (&'static str, Color32) {
    match p {
        "Items" => ("🎒", kit::C_GOLD),
        "Medicine" => ("💊", kit::C_PINK),
        "Poké Balls" => ("🔴", kit::C_RED),
        "TMs & HMs" => ("💿", kit::C_VIOLET),
        "Berries" => ("🍓", kit::C_GREEN),
        "Key Items" => ("🔑", kit::C_CYAN),
        "Battle Items" => ("💥", kit::C_ORANGE),
        "Mail" => ("✉", kit::C_SLATE),
        _ => ("🎒", kit::C_SLATE),
    }
}

const POCKETS: [&str; 8] = ["Items", "Medicine", "Poké Balls", "TMs & HMs", "Berries", "Key Items", "Battle Items", "Mail"];

fn price(it: &Item) -> u32 {
    it.price.trim().parse().unwrap_or(0)
}

/// "… for ₽1,200 …" → 1200.
fn yen(s: &str) -> Option<u32> {
    let at = s.find('₽')?;
    let digits: String = s[at + '₽'.len_utf8()..].chars().take_while(|c| c.is_ascii_digit() || *c == ',').filter(|c| *c != ',').collect();
    digits.parse().ok()
}

/// "after 3 badges" → 3, "from the start" → 0, "after all 8 badges" → 8.
fn badges_needed(s: &str) -> Option<u8> {
    let l = s.to_lowercase();
    if l.contains("from the start") {
        return Some(0);
    }
    let at = l.find("after ")?;
    let rest = l[at + 6..].trim_start_matches("all ");
    let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if rest[n.len()..].trim_start().starts_with("badge") {
        n.parse().ok()
    } else {
        None
    }
}

fn fmt_n(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Gold for prices, darker on light pages so it reads.
fn gold_ink() -> Color32 {
    if theme::pal().light {
        theme::blend(kit::C_GOLD, Color32::from_rgb(90, 60, 0), 0.55)
    } else {
        kit::C_GOLD
    }
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

/// Items that evolve something: item → (from, into, how) for each.
fn evolution_items(w: &Wiki) -> BTreeMap<String, Vec<(String, String, String)>> {
    let mut out: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    let names: Vec<&Item> = w.data.items.iter().filter(|i| i.name.chars().count() > 3 && i.pocket != "Key Items").collect();
    for e in &w.data.evolutions {
        for it in &names {
            if has_word(&e.method, &it.name) {
                out.entry(it.id.clone()).or_default().push((e.from.clone(), e.species.clone(), e.method.clone()));
            }
        }
    }
    out
}

/// `needle` in `hay` as a whole word (not inside a longer word).
fn has_word(hay: &str, needle: &str) -> bool {
    let mut from = 0;
    while let Some(i) = hay[from..].find(needle) {
        let s = from + i;
        let e = s + needle.len();
        let before = hay[..s].chars().next_back();
        let after = hay[e..].chars().next();
        let edge = |c: Option<char>| c.map_or(true, |c| !c.is_alphanumeric());
        if edge(before) && edge(after) {
            return true;
        }
        from = e;
    }
    false
}

/// Where a spot is in the story: its place's badge stage, if the place is a known one.
fn spot_stage(w: &Wiki, place: &str) -> Option<usize> {
    let pl = place.to_lowercase();
    w.data
        .places
        .values()
        .filter(|p| {
            let n = p.name.to_lowercase();
            let g = p.group.to_lowercase();
            pl == n || pl.starts_with(&format!("{n} ")) || pl.starts_with(&format!("{n},")) || (!g.is_empty() && (pl == g || pl.starts_with(&format!("{g} ")) || pl.starts_with(&format!("{g},"))))
        })
        .map(|p| w.stage_index(&p.stage))
        .min()
}

impl App {
    // ───────────────────────────── the list ─────────────────────────────

    pub(super) fn page_items(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        self.info_back_bar(ui, "Items");
        let pl = theme::pal();
        let full = ui.available_width();
        let slot = ui.painter().add(egui::Shape::Noop);
        let shops = {
            let mut s: Vec<&str> = w.data.items.iter().flat_map(|i| i.found.iter()).filter(|s| s.how == "Shop").map(|s| s.place.as_str()).collect();
            s.sort_unstable();
            s.dedup();
            s.len()
        };
        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 20.0)).show(ui, |ui| {
            ui.set_width(full - 56.0);
            ui.label(RichText::new("ITEMS").font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(MUTED(), kit::C_GOLD, 0.6)));
            ui.add_space(4.0);
            ui.label(RichText::new("Every item, and where to find it").font(crate::fonts::sans_bold_font(24.0)).color(TEXT()));
            ui.add_space(2.0);
            ui.label(RichText::new(format!("{} items in 8 Bag pockets, from {} shops, gifts, hidden spots and more.", w.data.items.len(), shops)).size(13.0).color(MUTED()));
            ui.add_space(14.0);
            let mut v = self.setup.info.items.view;
            kit::segmented(ui, &mut v, &[(ItemsView::Bag, "The Bag"), (ItemsView::Shops, "Shops"), (ItemsView::Evolution, "Evolution items")]);
            self.setup.info.items.view = v;
            ui.add_space(12.0);
            let width = ui.available_width();
            kit::search_box_keyed(ui, Some(egui::Id::new("items-search")), &mut self.setup.info.items.query, "Filter by name, e.g. potion, stone, berry", width);
        });
        let rect = frame.response.rect;
        ui.painter().set(
            slot,
            egui::Shape::Vec(vec![
                kit::card_shadow().as_shape(rect, 18.0).into(),
                theme::gradient_rect(rect, [18.0; 4], theme::blend(CARD(), kit::C_GOLD, if pl.light { 0.14 } else { 0.20 }), theme::blend(CARD(), pl.accent, if pl.light { 0.05 } else { 0.10 })),
                egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), kit::C_GOLD, 0.35))),
            ]),
        );
        ui.add_space(16.0);
        match self.setup.info.items.view {
            ItemsView::Bag => self.items_bag(ui, w),
            ItemsView::Shops => self.items_shops(ui, w),
            ItemsView::Evolution => self.items_evolution(ui, w),
        }
    }

    fn items_bag(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let s = &mut self.setup.info.items;
        // pockets
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            let all = w.data.items.len();
            if chip(ui, "🎒", &format!("All  {all}"), s.pocket.is_none(), kit::C_SLATE) {
                s.pocket = None;
            }
            for p in POCKETS {
                let n = w.data.items.iter().filter(|i| i.pocket == p).count();
                if n == 0 {
                    continue;
                }
                let (icon, col) = pocket_meta(p);
                let on = s.pocket.as_deref() == Some(p);
                if chip(ui, icon, &format!("{p}  {n}"), on, col) {
                    s.pocket = if on { None } else { Some(p.to_string()) };
                }
            }
        });
        ui.add_space(10.0);
        // how you get it, and order
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            ui.label(small("Get it by"));
            for h in HOWS {
                let on = s.how.as_deref() == Some(h);
                if chip(ui, how_icon(h), h, on, how_color(h)) {
                    s.how = if on { None } else { Some(h.to_string()) };
                }
            }
        });
        ui.add_space(10.0);
        let q = s.query.trim().to_lowercase();
        let mut list: Vec<&Item> = w
            .data
            .items
            .iter()
            .filter(|i| q.is_empty() || i.name.to_lowercase().contains(&q) || i.description.to_lowercase().contains(&q))
            .filter(|i| s.pocket.as_ref().map_or(true, |p| &i.pocket == p))
            .filter(|i| s.how.as_ref().map_or(true, |h| i.found.iter().any(|f| &f.how == h)))
            .collect();
        match s.sort {
            ItemSort::Name => list.sort_by(|a, b| a.name.cmp(&b.name)),
            ItemSort::Price => list.sort_by(|a, b| price(b).cmp(&price(a)).then(a.name.cmp(&b.name))),
            ItemSort::Ways => list.sort_by(|a, b| b.found.len().cmp(&a.found.len()).then(a.name.cmp(&b.name))),
        }
        let filtered = s.pocket.is_some() || s.how.is_some() || !q.is_empty();
        let mut clear = false;
        ui.horizontal(|ui| {
            let mut so = s.sort;
            kit::segmented(ui, &mut so, &[(ItemSort::Name, "A–Z"), (ItemSort::Price, "Dearest"), (ItemSort::Ways, "Most ways to get")]);
            s.sort = so;
            ui.add_space(10.0);
            ui.label(RichText::new(format!("{} of {} items", list.len(), w.data.items.len())).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
            if filtered {
                ui.add_space(6.0);
                clear = ui.link(RichText::new("Clear filters").size(12.5)).clicked();
            }
        });
        if clear {
            s.pocket = None;
            s.how = None;
            s.query.clear();
        }
        ui.add_space(10.0);
        if list.is_empty() {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(small("No items match. Try clearing a filter."));
            });
            return;
        }
        let avail = ui.available_width();
        let cols = if avail >= 1200.0 { 4 } else if avail >= 860.0 { 3 } else if avail >= 540.0 { 2 } else { 1 };
        let gap = 12.0;
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut open: Option<String> = None;
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for it in row {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 110.0), Sense::click());
                    if !ui.is_rect_visible(r) {
                        continue;
                    }
                    let (icon, col) = pocket_meta(&it.pocket);
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let p = ui.painter_at(r.expand(4.0));
                    p.add(kit::card_shadow().as_shape(r, 14.0));
                    p.rect_filled(r, 14.0, theme::blend(CARD(), CARD_HOVER(), ht));
                    p.rect_stroke(r, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.2 + 0.5 * ht)));
                    let chip = Rect::from_min_size(r.min + egui::vec2(12.0, 12.0), egui::vec2(30.0, 30.0));
                    kit::icon_chip(&p, chip, icon, col, false);
                    let x = chip.max.x + 10.0;
                    let maxw = r.max.x - x - 10.0;
                    let f = crate::fonts::sans_bold_font(14.0);
                    p.text(egui::pos2(x, chip.center().y - 7.0), Align2::LEFT_CENTER, super::charts::clip(ui, &it.name, &f, maxw), f, TEXT());
                    p.text(egui::pos2(x, chip.center().y + 9.0), Align2::LEFT_CENTER, &it.pocket, crate::fonts::sans_font(11.0), col);
                    let mut job = egui::text::LayoutJob::simple(it.description.clone(), crate::fonts::sans_font(11.5), MUTED(), r.width() - 24.0);
                    job.wrap.max_rows = 2;
                    let gal = ui.fonts(|fo| fo.layout_job(job));
                    p.galley(egui::pos2(r.min.x + 12.0, chip.max.y + 8.0), gal, MUTED());
                    // footer: how you get it, as coloured dots, and the price
                    let fy = r.max.y - 13.0;
                    let mut hows: Vec<&str> = it.found.iter().map(|f| f.how.as_str()).collect();
                    hows.sort_by_key(|h| HOWS.iter().position(|x| x == h).unwrap_or(99));
                    hows.dedup();
                    // how you get it, in the hows' colours: "Ground · Shop · +2"
                    let pr = price(it);
                    let price_w = if pr > 0 { 70.0 } else { 0.0 };
                    let mut fx = r.min.x + 12.0;
                    let limit = r.max.x - 12.0 - price_w;
                    let fsm = crate::fonts::sans_bold_font(10.5);
                    let mut shown = 0;
                    for h in &hows {
                        let g = p.layout_no_wrap(h.to_uppercase(), fsm.clone(), how_color(h));
                        if fx + g.size().x + 14.0 > limit {
                            break;
                        }
                        let tr = Rect::from_min_size(egui::pos2(fx, fy - 8.0), egui::vec2(g.size().x + 10.0, 16.0));
                        p.rect_filled(tr, 8.0, kit::tint(how_color(h), 0.14));
                        p.galley(egui::pos2(tr.min.x + 5.0, fy - g.size().y / 2.0), g, how_color(h));
                        fx = tr.max.x + 4.0;
                        shown += 1;
                    }
                    if shown < hows.len() {
                        p.text(egui::pos2(fx + 2.0, fy), Align2::LEFT_CENTER, format!("+{}", hows.len() - shown), crate::fonts::sans_bold_font(10.5), MUTED());
                    }
                    if hows.is_empty() {
                        p.text(egui::pos2(fx, fy), Align2::LEFT_CENTER, "not listed", crate::fonts::sans_font(10.5), MUTED());
                    }
                    if pr > 0 && it.found.iter().any(|f| f.how == "Shop") {
                        p.text(egui::pos2(r.max.x - 12.0, fy), Align2::RIGHT_CENTER, format!("₽{}", fmt_n(pr)), crate::fonts::sans_bold_font(11.5), TEXT());
                    }
                    let tip = if hows.is_empty() { "The game's data doesn't say where".to_string() } else { hows.iter().map(|h| how_label(h)).collect::<Vec<_>>().join(" · ") };
                    if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some(it.id.clone());
                    }
                }
            });
            ui.add_space(gap);
        }
        if let Some(id) = open {
            self.info_push(Page::Item(id));
        }
    }

    /// Every shop and what it sells. The Poké Mart grows with your badges.
    fn items_shops(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let q = self.setup.info.items.query.trim().to_lowercase();
        // shop → (item, price, badges, detail)
        let mut shops: BTreeMap<String, Vec<(&Item, Option<u32>, Option<u8>, String)>> = BTreeMap::new();
        for it in &w.data.items {
            if !q.is_empty() && !it.name.to_lowercase().contains(&q) {
                continue;
            }
            for s in it.found.iter().filter(|s| s.how == "Shop") {
                shops.entry(s.place.clone()).or_default().push((it, yen(&s.detail).or(Some(price(it)).filter(|p| *p > 0)), badges_needed(&s.detail), s.detail.clone()));
            }
        }
        let mut open: Option<String> = None;
        // the Poké Mart first, with its badge ladder
        if let Some(mart) = shops.remove("Poké Mart (any)") {
            let badges = self.setup.info.items.badges.unwrap_or(8);
            let mut set: Option<u8> = None;
            kit::card(ui, |ui| {
                ui.horizontal(|ui| {
                    card_head(ui, "🏪", kit::C_GOLD, "Every Poké Mart", None);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(small("Same stock in every town, more with each badge"));
                    });
                });
                ui.add_space(10.0);
                // the badge slider: 0 to 8 dots
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Badges").font(crate::fonts::sans_bold_font(12.5)).color(TEXT()));
                    ui.add_space(6.0);
                    for b in 0..=8u8 {
                        let on = b <= badges;
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let c = r.center();
                        if b == 0 {
                            ui.painter().circle_stroke(c, 10.5, Stroke::new(1.2f32, if badges == 0 { kit::C_GOLD } else { MUTED() }));
                            ui.painter().text(c, Align2::CENTER_CENTER, "0", crate::fonts::sans_bold_font(11.0), if badges == 0 { TEXT() } else { MUTED() });
                        } else {
                            let fill = if on { kit::C_GOLD } else { theme::blend(kit::sunken(), kit::C_GOLD, 0.15 * ht) };
                            ui.painter().circle_filled(c, 11.0, fill);
                            ui.painter().text(c, Align2::CENTER_CENTER, b.to_string(), crate::fonts::sans_bold_font(11.0), if on { Color32::from_rgb(40, 30, 10) } else { MUTED() });
                        }
                        if resp.on_hover_text(format!("What the Mart sells with {b} badge{}", if b == 1 { "" } else { "s" })).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            set = Some(b);
                        }
                    }
                });
                ui.add_space(12.0);
                let mut stock: Vec<&(&Item, Option<u32>, Option<u8>, String)> = mart.iter().collect();
                stock.sort_by_key(|x| (x.2.unwrap_or(0), x.1.unwrap_or(0)));
                let avail = ui.available_width();
                let cols = if avail >= 900.0 { 3 } else if avail >= 560.0 { 2 } else { 1 };
                let cw = ((avail - 10.0 * (cols as f32 - 1.0)) / cols as f32).floor();
                for row in stock.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        for (it, pr, need, _) in row {
                            let locked = need.unwrap_or(0) > badges;
                            let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 40.0), Sense::click());
                            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                            let p = ui.painter_at(r.expand(2.0));
                            p.rect_filled(r, 10.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
                            let ink = if locked { kit::tint(MUTED(), 0.6) } else { TEXT() };
                            let (icon, col) = pocket_meta(&it.pocket);
                            p.text(egui::pos2(r.min.x + 16.0, r.center().y), Align2::CENTER_CENTER, icon, egui::FontId::proportional(13.0), if locked { kit::tint(col, 0.4) } else { col });
                            p.text(egui::pos2(r.min.x + 32.0, r.center().y), Align2::LEFT_CENTER, &it.name, crate::fonts::sans_bold_font(13.0), ink);
                            let right = match (locked, need) {
                                (true, Some(n)) => format!("🔒 {n} badge{}", if *n == 1 { "" } else { "s" }),
                                _ => pr.map(|p| format!("₽{}", fmt_n(p))).unwrap_or_default(),
                            };
                            p.text(egui::pos2(r.max.x - 12.0, r.center().y), Align2::RIGHT_CENTER, right, crate::fonts::sans_bold_font(12.0), if locked { MUTED() } else { gold_ink() });
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                open = Some(it.id.clone());
                            }
                        }
                    });
                    ui.add_space(6.0);
                }
            });
            if let Some(b) = set {
                self.setup.info.items.badges = Some(b);
            }
            ui.add_space(18.0);
        }
        // the rest, by town
        let mut list: Vec<(String, Vec<(&Item, Option<u32>, Option<u8>, String)>)> = shops.into_iter().collect();
        list.sort_by_key(|(s, v)| (std::cmp::Reverse(v.len()), s.clone()));
        let avail = ui.available_width();
        let cols = if avail >= 1100.0 { 3 } else if avail >= 700.0 { 2 } else { 1 };
        let gap = 16.0;
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        for row in list.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (shop, stock) in row {
                    ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        kit::card(ui, |ui| {
                            let (town, counter) = match shop.split_once(", ") {
                                Some((a, b)) => (a.to_string(), b.to_string()),
                                None => (shop.clone(), String::new()),
                            };
                            card_head(ui, "🏪", kit::C_GOLD, &town, None);
                            if !counter.is_empty() {
                                ui.label(small(counter));
                            }
                            ui.add_space(8.0);
                            let mut stock = stock.clone();
                            stock.sort_by_key(|x| (x.1.unwrap_or(0), x.0.name.clone()));
                            for (it, pr, _, detail) in stock {
                                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), Sense::click());
                                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                                let p = ui.painter_at(r.expand(2.0));
                                if ht > 0.0 {
                                    p.rect_filled(r, 8.0, theme::blend(CARD(), CARD_HOVER(), ht));
                                }
                                let f = crate::fonts::sans_font(13.0);
                                p.text(egui::pos2(r.min.x + 6.0, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &it.name, &f, r.width() - 110.0), f, TEXT());
                                let label = match pr {
                                    Some(p) => format!("₽{}", fmt_n(p)),
                                    None => "—".into(),
                                };
                                p.text(egui::pos2(r.max.x - 6.0, r.center().y), Align2::RIGHT_CENTER, label, crate::fonts::sans_bold_font(12.0), gold_ink());
                                if resp.on_hover_text(detail).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    open = Some(it.id.clone());
                                }
                            }
                        });
                    });
                }
            });
            ui.add_space(gap);
        }
        if let Some(id) = open {
            self.info_push(Page::Item(id));
        }
    }

    /// Stones and trade items: what each one evolves, and where to get it.
    fn items_evolution(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>) {
        let q = self.setup.info.items.query.trim().to_lowercase();
        let evo = evolution_items(w);
        let mut list: Vec<(&Item, &Vec<(String, String, String)>)> = evo
            .iter()
            .filter_map(|(id, e)| w.data.items.iter().find(|i| &i.id == id).map(|i| (i, e)))
            .filter(|(i, _)| q.is_empty() || i.name.to_lowercase().contains(&q))
            .collect();
        list.sort_by_key(|(i, e)| (std::cmp::Reverse(e.len()), i.name.clone()));
        let mut open_item: Option<String> = None;
        let mut open_mon: Option<String> = None;
        let avail = ui.available_width();
        let cols = if avail >= 1000.0 { 2 } else { 1 };
        let gap = 16.0;
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        for row in list.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (it, evos) in row {
                    ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        kit::card(ui, |ui| {
                            let (icon, col) = pocket_meta(&it.pocket);
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
                                kit::icon_chip(ui.painter(), r, icon, col, false);
                                ui.add_space(4.0);
                                if ui.add(egui::Label::new(RichText::new(&it.name).font(crate::fonts::sans_bold_font(16.0)).color(TEXT())).sense(Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    open_item = Some(it.id.clone());
                                }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.link(RichText::new("Where to get it ›").size(12.5)).clicked() {
                                        open_item = Some(it.id.clone());
                                    }
                                });
                            });
                            ui.add_space(8.0);
                            for (from, into, how) in evos.iter() {
                                let fm = w.mon(from).map(|m| m.name.clone()).unwrap_or_else(|| from.clone());
                                let im = w.mon(into).map(|m| m.name.clone()).unwrap_or_else(|| into.clone());
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    if self.evo_mon(ui, &fm).clicked() {
                                        open_mon = Some(from.clone());
                                    }
                                    ui.label(RichText::new("→").color(MUTED()));
                                    if self.evo_mon(ui, &im).clicked() {
                                        open_mon = Some(into.clone());
                                    }
                                    let short = if how.to_lowercase().contains("trade") { "trade holding it" } else if how.to_lowercase().contains("hold") { "level up holding it" } else { "use it" };
                                    ui.label(small(short)).on_hover_text(how);
                                });
                                ui.add_space(2.0);
                            }
                            ui.add_space(6.0);
                            let spots: Vec<&ItemSpot> = it.found.iter().filter(|s| s.how != "Other").take(2).collect();
                            if spots.is_empty() {
                                ui.label(small("Where to get it: not listed"));
                            } else {
                                for s in spots {
                                    ui.horizontal(|ui| {
                                        tag(ui, &s.how.to_uppercase(), how_color(&s.how));
                                        ui.label(RichText::new(&s.place).size(12.0).color(TEXT()));
                                    });
                                }
                                let more = it.found.len().saturating_sub(2);
                                if more > 0 {
                                    ui.label(small(format!("+ {more} more ways")));
                                }
                            }
                        });
                    });
                }
            });
            ui.add_space(gap);
        }
        if let Some(id) = open_item {
            self.info_push(Page::Item(id));
        }
        if let Some(id) = open_mon {
            self.info_open_dex(Some(id));
        }
    }

    /// A Pokémon as a little sprite and name.
    fn evo_mon(&mut self, ui: &mut egui::Ui, name: &str) -> egui::Response {
        let g = ui.painter().layout_no_wrap(name.to_string(), crate::fonts::sans_bold_font(12.5), TEXT());
        let (r, resp) = ui.allocate_exact_size(egui::vec2(g.size().x + 40.0, 32.0), Sense::click());
        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
        let p = ui.painter().clone();
        p.rect_filled(r, 16.0, theme::blend(kit::sunken(), CARD_HOVER(), ht));
        let c = egui::pos2(r.min.x + 17.0, r.center().y);
        let drew = match self.info_species(name) {
            Some(sp) if self.sprites.get(&sp, false).is_some() => self.draw_sprite_centered(&p, &sp, false, c, egui::vec2(30.0, 30.0)),
            _ => false,
        };
        if !drew {
            p.circle_filled(c, 8.0, kit::tint(kit::C_RED, 0.2));
        }
        p.galley(egui::pos2(r.min.x + 34.0, r.center().y - g.size().y / 2.0), g, TEXT());
        resp.on_hover_text("Open in the Pokédex").on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    // ───────────────────────────── one item ─────────────────────────────

    pub(super) fn page_item(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, id: &str) {
        let Some(it) = w.data.items.iter().find(|i| i.id == id).cloned() else {
            self.info_back_bar(ui, "Item");
            ui.label(small("This item isn't in the game's data any more."));
            return;
        };
        self.info_back_bar(ui, &it.name);
        let (icon, col) = pocket_meta(&it.pocket);
        let pl = theme::pal();
        let pr = price(&it);
        let sold = it.found.iter().any(|s| s.how == "Shop");
        let teaches = if it.teaches.is_empty() { None } else { w.move_key(&it.teaches) };

        // ── hero
        let full = ui.available_width();
        let slot = ui.painter().add(egui::Shape::Noop);
        let mut open_move: Option<String> = None;
        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 22.0)).show(ui, |ui| {
            ui.set_width(full - 56.0);
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), Sense::hover());
                kit::icon_chip(ui.painter(), r, icon, col, true);
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("ITEM · {}", it.pocket.to_uppercase())).font(crate::fonts::sans_bold_font(11.0)).color(theme::blend(MUTED(), col, 0.6)));
                    ui.add_space(2.0);
                    ui.label(RichText::new(&it.name).font(crate::fonts::sans_bold_font(30.0)).color(TEXT()));
                    ui.add_space(8.0);
                    ui.add(egui::Label::new(RichText::new(&it.description).size(14.5).color(TEXT())).wrap());
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                        if pr > 0 && sold {
                            tag(ui, &format!("BUY ₽{}", fmt_n(pr)), kit::C_GOLD);
                        } else if it.pocket != "Key Items" {
                            tag(ui, "CAN'T BE BOUGHT", kit::C_SLATE);
                        }
                        if pr > 0 && it.pocket != "Key Items" {
                            tag(ui, &format!("SELLS FOR ₽{}", fmt_n(pr / 2)), kit::C_GREEN).on_hover_text("Shops buy items back for half their price");
                        }
                        if let Some(k) = &teaches {
                            if let Some(m) = w.move_info(k) {
                                let r = tag(ui, &format!("TEACHES {}", m.n.to_uppercase()), type_color(&m.t)).interact(Sense::click());
                                if r.on_hover_text("Open the move").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    open_move = Some(k.clone());
                                }
                            }
                        }
                    });
                });
            });
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
        if let Some(k) = open_move {
            self.info_push(Page::Move(k));
            return;
        }
        ui.add_space(18.0);

        let avail = ui.available_width();
        let gap = 18.0;
        if avail >= 1000.0 {
            let left_w = ((avail - gap) * 0.6).floor();
            let right_w = avail - gap - left_w;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(left_w);
                    self.item_where(ui, w, &it);
                });
                ui.allocate_ui_with_layout(egui::vec2(right_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(right_w);
                    self.item_facts(ui, w, &it);
                });
            });
        } else {
            self.item_facts(ui, w, &it);
            ui.add_space(gap);
            self.item_where(ui, w, &it);
        }
    }

    /// Every way to get it, grouped by how.
    fn item_where(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, it: &Item) {
        let mut open_mon: Option<String> = None;
        let mut open_place: Option<String> = None;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                card_head(ui, "📍", kit::C_GREEN, "Where to get it", None);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(small(format!("{} way{}", it.found.len(), if it.found.len() == 1 { "" } else { "s" })));
                });
            });
            ui.add_space(8.0);
            if it.found.is_empty() {
                ui.label(small("The game's data doesn't list anywhere to get it."));
                return;
            }
            let mut groups: BTreeMap<usize, Vec<&ItemSpot>> = BTreeMap::new();
            for s in &it.found {
                groups.entry(HOWS.iter().position(|h| *h == s.how).unwrap_or(HOWS.len())).or_default().push(s);
            }
            for (gi, spots) in groups {
                let how = HOWS.get(gi).copied().unwrap_or("Other");
                let col = how_color(how);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
                    ui.painter().rect_filled(r, 7.0, kit::tint(col, 0.2));
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, how_icon(how), egui::FontId::proportional(12.0), col);
                    ui.label(RichText::new(how_label(how).to_uppercase()).font(crate::fonts::sans_bold_font(11.0)).color(col));
                    ui.label(small(format!("{}", spots.len())));
                });
                ui.add_space(4.0);
                if how == "Held" {
                    // wild Pokémon that carry it: sprites
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for s in &spots {
                            let id = if s.species.is_empty() { None } else { Some(s.species.clone()) };
                            let name = id.as_ref().and_then(|i| w.mon(i)).map(|m| m.name.clone()).unwrap_or_else(|| s.place.trim_start_matches("Wild ").to_string());
                            let resp = self.evo_mon(ui, &name).on_hover_text(&s.detail);
                            if resp.clicked() {
                                open_mon = id.or_else(|| w.data.species.iter().find(|m| m.name == name).map(|m| m.id.clone()));
                            }
                            ui.label(small(short_chance(&s.detail)));
                        }
                    });
                    ui.add_space(6.0);
                    continue;
                }
                for s in spots.iter().take(if spots.len() > 8 { 6 } else { 8 }) {
                    ui.horizontal_top(|ui| {
                        ui.add_space(30.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.horizontal(|ui| {
                                match super::info_places::group_of(w, &s.place) {
                                    Some(g) => {
                                        if ui.link(RichText::new(&s.place).size(13.0).strong()).on_hover_text(format!("Open {g}")).clicked() {
                                            open_place = Some(g);
                                        }
                                    }
                                    None => {
                                        ui.label(RichText::new(&s.place).size(13.0).strong().color(TEXT()));
                                    }
                                }
                                if let Some(p) = yen(&s.detail).filter(|_| how == "Shop") {
                                    ui.label(RichText::new(format!("₽{}", fmt_n(p))).size(12.0).strong().color(gold_ink()));
                                }
                            });
                            if !s.detail.is_empty() {
                                ui.add(egui::Label::new(small(&s.detail)).wrap());
                            }
                        });
                    });
                    ui.add_space(6.0);
                }
                if spots.len() > 8 {
                    let key = egui::Id::new(("item-more", &it.id, gi));
                    let open = ui.ctx().data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
                    if open {
                        for s in spots.iter().skip(6) {
                            ui.horizontal_top(|ui| {
                                ui.add_space(30.0);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    ui.label(RichText::new(&s.place).size(13.0).strong().color(TEXT()));
                                    if !s.detail.is_empty() {
                                        ui.add(egui::Label::new(small(&s.detail)).wrap());
                                    }
                                });
                            });
                            ui.add_space(6.0);
                        }
                    }
                    ui.horizontal(|ui| {
                        ui.add_space(30.0);
                        if ui.link(RichText::new(if open { "Show fewer".to_string() } else { format!("Show {} more", spots.len() - 6) }).size(12.5)).clicked() {
                            ui.ctx().data_mut(|d| d.insert_temp(key, !open));
                        }
                    });
                }
            }
        });
        if let Some(id) = open_mon {
            self.info_open_dex(Some(id));
        }
        if let Some(p) = open_place {
            self.info_push(Page::Place(p));
        }
    }

    /// The short version: first chance in the story, prices, what it's for.
    fn item_facts(&mut self, ui: &mut egui::Ui, w: &Arc<Wiki>, it: &Item) {
        let pr = price(it);
        // earliest spot you can reach (ground, hidden, gift, shop, berry)
        let first = it
            .found
            .iter()
            .filter(|s| matches!(s.how.as_str(), "Ground" | "Hidden" | "Gift" | "Shop" | "Berry tree"))
            .filter_map(|s| spot_stage(w, &s.place).map(|st| (st, s)))
            .min_by_key(|x| x.0);
        let mart = it.found.iter().find(|s| s.place == "Poké Mart (any)").and_then(|s| badges_needed(&s.detail));
        let mut open_mon: Option<String> = None;
        kit::card(ui, |ui| {
            card_head(ui, "💡", kit::C_GOLD, "Good to know", None);
            ui.add_space(10.0);
            let row = |ui: &mut egui::Ui, label: &str, value: String, note: Option<String>| {
                ui.horizontal_top(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(110.0, 18.0), Sense::hover());
                    ui.painter().text(r.left_center(), Align2::LEFT_CENTER, label.to_uppercase(), crate::fonts::sans_bold_font(10.0), MUTED());
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.add(egui::Label::new(RichText::new(value).size(13.0).strong().color(TEXT())).wrap());
                        if let Some(n) = note {
                            ui.add(egui::Label::new(small(n)).wrap());
                        }
                    });
                });
                ui.add_space(8.0);
            };
            if let Some((st, s)) = first {
                let stage = w.data.stages.get(st).cloned().unwrap_or_default();
                let mark = match self.story_stage() {
                    Some(a) if st <= a => " · ✔ you can get it now",
                    Some(_) => " · later in your story",
                    None => "",
                };
                row(ui, "First chance", s.place.clone(), Some(format!("{} · {}{mark}", stage, s.how.to_lowercase())));
            }
            if let Some(b) = mart {
                row(ui, "Poké Mart", if b == 0 { "From the start".into() } else { format!("After {b} badge{}", if b == 1 { "" } else { "s" }) }, Some("Every Poké Mart sells it".into()));
            }
            if pr > 0 && it.pocket != "Key Items" {
                let buy = if it.found.iter().any(|s| s.how == "Shop") { format!("₽{} to buy", fmt_n(pr)) } else { "Not sold".to_string() };
                row(ui, "Price", buy, Some(format!("sells back for ₽{}", fmt_n(pr / 2))));
            }
            row(ui, "Pocket", it.pocket.clone(), None);
            // evolutions it's used for
            let evo = evolution_items(w);
            if let Some(list) = evo.get(&it.id) {
                ui.add_space(4.0);
                ui.label(RichText::new("EVOLVES").font(crate::fonts::sans_bold_font(10.0)).color(MUTED()));
                ui.add_space(4.0);
                for (from, into, how) in list {
                    let fm = w.mon(from).map(|m| m.name.clone()).unwrap_or_else(|| from.clone());
                    let im = w.mon(into).map(|m| m.name.clone()).unwrap_or_else(|| into.clone());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if self.evo_mon(ui, &fm).clicked() {
                            open_mon = Some(from.clone());
                        }
                        ui.label(RichText::new("→").color(MUTED()));
                        if self.evo_mon(ui, &im).clicked() {
                            open_mon = Some(into.clone());
                        }
                    });
                    ui.add(egui::Label::new(small(how)).wrap());
                    ui.add_space(6.0);
                }
            }
        });
        if let Some(id) = open_mon {
            self.info_open_dex(Some(id));
        }
        // more from the same pocket, nearby alphabetically
        ui.add_space(18.0);
        let same: Vec<&Item> = {
            let mut v: Vec<&Item> = w.data.items.iter().filter(|i| i.pocket == it.pocket && i.id != it.id).collect();
            v.sort_by(|a, b| a.name.cmp(&b.name));
            let at = v.iter().position(|i| i.name > it.name).unwrap_or(v.len());
            let start = at.saturating_sub(3);
            v.into_iter().skip(start).take(6).collect()
        };
        if same.is_empty() {
            return;
        }
        let mut open: Option<String> = None;
        kit::card(ui, |ui| {
            let (icon, col) = pocket_meta(&it.pocket);
            card_head(ui, icon, col, &format!("More {}", it.pocket.to_lowercase()), None);
            ui.add_space(6.0);
            for i in same {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let p = ui.painter_at(r.expand(2.0));
                if ht > 0.0 {
                    p.rect_filled(r, 8.0, theme::blend(CARD(), CARD_HOVER(), ht));
                }
                p.text(egui::pos2(r.min.x + 8.0, r.center().y), Align2::LEFT_CENTER, &i.name, crate::fonts::sans_bold_font(13.0), TEXT());
                let pr = price(i);
                if pr > 0 && i.found.iter().any(|s| s.how == "Shop") {
                    p.text(egui::pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, format!("₽{}", fmt_n(pr)), crate::fonts::sans_font(12.0), MUTED());
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(i.id.clone());
                }
            }
        });
        if let Some(id) = open {
            self.info_push(Page::Item(id));
        }
    }

    /// From a guide's link or anywhere: open an item by its wiki slug.
    pub(super) fn info_open_item_slug(&mut self, w: &Arc<Wiki>, slug: &str) -> bool {
        match w.data.items.iter().find(|i| wiki::slug(&i.name) == slug) {
            Some(it) => {
                let id = it.id.clone();
                self.info_push(Page::Item(id));
                true
            }
            None => false,
        }
    }
}

/// "Rare, 5%" / "Always holds it (100%)" → "5%" / "100%".
fn short_chance(detail: &str) -> String {
    if let Some(open) = detail.rfind('(') {
        if let Some(close) = detail[open..].find(')') {
            return detail[open + 1..open + close].to_string();
        }
    }
    detail.rsplit(", ").next().unwrap_or(detail).to_string()
}

/// A filter chip with an icon. True when clicked.
fn chip(ui: &mut egui::Ui, icon: &str, text: &str, on: bool, col: Color32) -> bool {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(12.0), TEXT());
    let (r, resp) = ui.allocate_exact_size(g.size() + egui::vec2(40.0, 12.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    let fill = if on { theme::blend(CARD(), col, 0.26) } else { theme::blend(kit::sunken(), CARD_HOVER(), ht) };
    ui.painter().rect_filled(r, r.height() / 2.0, fill);
    ui.painter().rect_stroke(r, r.height() / 2.0, Stroke::new(1.0f32, if on { theme::blend(LINE(), col, 0.8) } else { theme::blend(LINE(), col, 0.3 * ht) }));
    ui.painter().text(egui::pos2(r.min.x + 16.0, r.center().y), Align2::CENTER_CENTER, icon, egui::FontId::proportional(12.0), col);
    ui.painter().galley(egui::pos2(r.min.x + 28.0, r.min.y + 6.0), g, TEXT());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_and_badges() {
        assert_eq!(yen("from the Poké Mart clerk after 5 badges for ₽1,200"), Some(1200));
        assert_eq!(yen("no price"), None);
        assert_eq!(badges_needed("from the Poké Mart clerk after 5 badges for ₽1,200"), Some(5));
        assert_eq!(badges_needed("after 1 badge for ₽250"), Some(1));
        assert_eq!(badges_needed("after all 8 badges for ₽3,000"), Some(8));
        assert_eq!(badges_needed("from the start for ₽200"), Some(0));
        assert_eq!(badges_needed("after you beat him"), None);
        assert_eq!(short_chance("Always holds it (100%)"), "100%");
        assert_eq!(short_chance("Rare, 5%"), "5%");
    }

    #[test]
    fn words() {
        assert!(has_word("Use a Moon Stone on it.", "Moon Stone"));
        assert!(!has_word("Use a Moon Stones on it.", "Moon Stone"));
    }

    #[test]
    fn real_data() {
        let Ok(bytes) = std::fs::read("assets/wiki.json") else { return };
        let w = Wiki::parse(&bytes).expect("parses");
        let evo = evolution_items(&w);
        assert!(evo.contains_key("ITEM_MOON_STONE"));
        assert!(evo.get("ITEM_WATER_STONE").is_some_and(|v| v.iter().any(|e| e.0 == "EEVEE")));
        // held items name their Pokémon
        let lo = w.data.items.iter().find(|i| i.name == "Leftovers").expect("leftovers");
        assert!(lo.found.iter().any(|s| s.how == "Held" && s.species == "MUNCHLAX"));
        // some known places get a story stage
        assert!(spot_stage(&w, "Route 213").is_some());
    }
}
