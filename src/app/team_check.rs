//! Team Planner → Checklist: everything you need to do in Platinum Together
//! to have this team for real, worked out from the game's data, with
//! ticks, a shopping list and links to the Map and Game Info.

use super::info_home::Page;
use super::team_build::{best_ways, item_text, learnable, mkey, wiki_id, How};
use super::team_home::{caps, chip, small, soft, ACCENT};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{self, World, STATS};
use crate::teams::{Member, Team};
use crate::theme;
use crate::wiki::Wiki;
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

/// Where a step's link goes.
#[derive(Clone, Debug)]
pub(super) enum Go {
    Page(Page),
    Map(Vec<String>, String),
    Guide(String),
}

#[derive(Clone, Debug)]
pub(super) struct Step {
    /// Saved in the member's `done` list when ticked.
    pub key: String,
    pub icon: &'static str,
    pub title: String,
    pub detail: String,
    pub links: Vec<(String, Go)>,
    /// Something's wrong (can't be learned, can't be caught).
    pub warn: bool,
    /// ₽ and Rift Shards it costs, when bought.
    pub money: u32,
    pub shards: u32,
    /// Only a suggestion: doesn't count toward "done".
    pub optional: bool,
    /// ₽ it could cost (vitamins): shown, not added to what you need.
    pub maybe_money: u32,
}

impl Step {
    fn new(key: impl Into<String>, icon: &'static str, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Step { key: key.into(), icon, title: title.into(), detail: detail.into(), links: Vec::new(), warn: false, money: 0, shards: 0, optional: false, maybe_money: 0 }
    }
    fn link(mut self, label: &str, go: Go) -> Self {
        self.links.push((label.to_string(), go));
        self
    }
}

fn price(s: &str) -> u32 {
    s.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)
}

fn item_id(w: &Wiki, name: &str) -> Option<String> {
    item_text(Some(w), name).map(|i| i.id.clone())
}

/// Where an item is found, briefly ("Route 210 (an item ball)").
fn item_where(w: &Wiki, name: &str, n: usize) -> (String, u32) {
    let Some(it) = item_text(Some(w), name) else { return (String::new(), 0) };
    let mut shop = 0;
    let spots: Vec<String> = it
        .found
        .iter()
        .take(n)
        .map(|s| {
            if s.how == "Shop" && shop == 0 {
                shop = price(&it.price);
            }
            if s.detail.is_empty() {
                format!("{}: {}", s.how, s.place)
            } else {
                format!("{}: {} ({})", s.how, s.place, s.detail)
            }
        })
        .collect();
    (spots.join("  ·  "), shop)
}

/// Every step for one member.
pub(super) fn steps(w: &Wiki, world: World, m: &Member, story: Option<usize>) -> Vec<Step> {
    let mut out = Vec::new();
    let d = battle::data();
    let Some(sp) = d.species(&m.species) else { return out };
    let Some(id) = wiki_id(&m.species) else { return out };
    let name = m.species.clone();
    // ── catch: the earliest in the family you can find
    let mut chain: Vec<String> = vec![id.clone()];
    while let Some(e) = w.evolves_from(chain.last().unwrap()) {
        if e.from.is_empty() || chain.contains(&e.from) || chain.len() > 4 {
            break;
        }
        chain.push(e.from.clone());
    }
    chain.reverse(); // base first
    let ok_setting = |s: &str| match s {
        "modern" => world == World::Modern,
        _ => true,
    };
    let mut catch_at = None;
    for (i, c) in chain.iter().enumerate() {
        let wild: Vec<_> = w.wild(c).into_iter().filter(|e| ok_setting(&e.setting)).collect();
        let other: Vec<_> = w.other(c).into_iter().filter(|o| o.works != Some(false)).collect();
        if wild.is_empty() && other.is_empty() {
            continue;
        }
        let cname = w.mon(c).map(|x| x.name.clone()).unwrap_or_else(|| c.clone());
        // places, earliest in the story first
        let mut places: Vec<(usize, String, String)> = Vec::new();
        for e in &wild {
            let group = w.place(&e.area).map(|p| p.group.clone()).filter(|g| !g.is_empty()).unwrap_or_else(|| e.area.clone());
            if places.iter().any(|p| p.1 == group) {
                continue;
            }
            let how = match e.method.as_str() {
                "grass" => "grass",
                "cave" => "walking",
                "surf" => "Surf",
                "old_rod" => "Old Rod",
                "good_rod" => "Good Rod",
                "super_rod" => "Super Rod",
                "radar" => "Poké Radar",
                x => x,
            };
            places.push((w.stage_index(&e.stage), group, format!("{how}, Lv {}", e.levels)));
        }
        places.sort();
        let mut detail: Vec<String> = places.iter().take(3).map(|p| format!("{} ({})", p.1, p.2)).collect();
        if places.len() > 3 {
            detail.push(format!("{} more", places.len() - 3));
        }
        for o in other.iter().take(2) {
            let short = o.detail.chars().count() <= 60 && !o.detail.is_empty();
            detail.push(format!("{}: {}{}", o.how, o.place, if short { format!(" ({})", o.detail) } else { String::new() }));
        }
        let later = story.is_some_and(|s| places.first().is_some_and(|p| p.0 != usize::MAX && p.0 > s)) && other.is_empty();
        let mut st = Step::new("catch", "🎯", format!("Catch {cname}"), detail.join("  ·  "));
        if later {
            st.detail = format!("Later in the story. {}", st.detail);
        }
        if let Some(p) = places.first() {
            st = st.link(&p.1, Go::Page(Page::Place(p.1.clone())));
        }
        if !places.is_empty() {
            let areas: Vec<String> = wild.iter().map(|e| e.area.clone()).collect();
            st = st.link("Show on map", Go::Map(areas, cname.clone()));
        }
        out.push(st);
        catch_at = Some(i);
        break;
    }
    if catch_at.is_none() {
        let why = w.unobtainable(&id).map(|s| s.to_string()).unwrap_or_else(|| "The wiki doesn't list a way to get it.".into());
        let mut st = Step::new("catch", "🎯", format!("Get {name}"), why);
        st.warn = true;
        out.push(st);
        catch_at = Some(chain.len() - 1);
    }
    // ── evolve
    for c in chain.iter().skip(catch_at.unwrap_or(0) + 1) {
        if let Some(e) = w.evolves_from(c) {
            let to = w.mon(c).map(|x| x.name.clone()).unwrap_or_else(|| c.clone());
            out.push(Step::new(format!("evolve:{c}"), "✨", format!("Evolve into {to}"), e.method.trim_end_matches('.').to_string()));
        }
    }
    // forms
    if !sp.base.is_empty() && !sp.is_mega() {
        out.push(Step::new(format!("form:{}", sp.id), "🔑", format!("Change it to {}", sp.name), "A form change: Game Info has how it works in Platinum Together."));
    }
    if sp.is_mega() || !d.item(&m.item).is_none_or(|i| i.mega.is_empty()) {
        out.push(Step::new("keystone", "🔑", "Get the Key Stone", "Mega Evolution needs it, in MODERN POKéMON worlds.").link("Mega Evolution guide", Go::Guide("megas".into())));
    }
    // ── moves
    let ways = learnable(w, &m.species, world);
    let best = best_ways(&ways);
    for mv in &m.moves {
        let key = mkey(w, mv);
        let l = key.as_ref().and_then(|k| best.iter().find(|l| &l.key == k).copied()).or_else(|| if mv.starts_with("Hidden Power") { best.iter().find(|l| l.key == "hidden_power").copied() } else { None });
        let mvname = key.as_ref().and_then(|k| w.move_info(k)).map(|i| i.n.clone()).unwrap_or_else(|| mv.clone());
        let mut st = Step::new(format!("move:{}", battle::to_id(mv)), "💿", format!("Teach {mvname}"), String::new());
        match l {
            None => {
                st.detail = format!("{name} can't learn it in this world.");
                st.warn = true;
            }
            Some(l) => {
                let who = l.via.clone().unwrap_or_else(|| name.clone());
                match &l.how {
                    How::Level(lv) if lv.chars().all(|c| c.is_ascii_digit()) => {
                        st.detail = if l.via.is_some() { format!("{who} learns it at Lv {lv}: teach it before evolving (or ask the Move Reminder).") } else { format!("Learns it at Lv {lv}.") };
                    }
                    How::Level(lv) => st.detail = format!("{who} learns it ({lv})."),
                    How::Tm(t) => {
                        let (wh, cost) = item_where(w, t, 2);
                        st.detail = format!("{t}{}{}", if l.via.is_some() { format!(" (teach it to {who})") } else { String::new() }, if wh.is_empty() { String::new() } else { format!(": {wh}") });
                        st.money = cost;
                        if let Some(id) = item_id(w, t) {
                            st = st.link(t, Go::Page(Page::Item(id)));
                        }
                    }
                    How::Tutor => {
                        let tu = key.as_ref().and_then(|k| w.data.tutors.get(k));
                        match tu {
                            Some(tu) => {
                                let cost: Vec<String> = tu.cost.iter().map(|(c, n)| format!("{n} {c}")).collect();
                                st.detail = format!("Move Tutor at {}{}", tu.place, if cost.is_empty() { String::new() } else { format!(" for {} shards", cost.join(", ")) });
                                st.shards = tu.cost.values().sum();
                                let group = w.place(&tu.place).map(|p| p.group.clone()).filter(|g| !g.is_empty()).unwrap_or_else(|| tu.place.clone());
                                st = st.link(&group, Go::Page(Page::Place(group.clone())));
                            }
                            None => st.detail = "From a Move Tutor.".into(),
                        }
                    }
                    How::Egg => st.detail = format!("Egg move: breed a {who} whose father knows it."),
                }
                if l.modern {
                    st.detail.push_str(" (MODERN worlds)");
                }
                if let Some(k) = &key {
                    st = st.link("Move", Go::Page(Page::Move(k.clone())));
                }
            }
        }
        out.push(st);
    }
    // ── item
    if !m.item.is_empty() {
        let (wh, cost) = item_where(w, &m.item, 2);
        let mut st = Step::new("item", "🎒", format!("Get {}", m.item), if wh.is_empty() { "Not in the wiki's item list.".into() } else { wh });
        st.money = cost;
        if let Some(id) = item_id(w, &m.item) {
            st = st.link("Item", Go::Page(Page::Item(id)));
        }
        out.push(st);
    }
    // ── ability
    let abs = sp.abilities_in(world);
    if abs.len() > 1 && !m.ability.is_empty() {
        let i = abs.iter().position(|a| *a == m.ability);
        let mut st = Step::new(format!("ability:{}", battle::to_id(&m.ability)), "🏅", format!("Ability: {}", m.ability), String::new());
        match i {
            Some(_) if world == World::Modern && m.ability == sp.hidden => {
                st.detail = "A hidden Ability: use an Ability Patch (one a week at the Rift Shop, 2,000 Rift Shards).".into();
                st.shards = 2000;
            }
            Some(_) => {
                let other = abs.iter().filter(|a| **a != m.ability && !(world == World::Modern && **a == sp.hidden)).cloned().collect::<Vec<_>>().join(" or ");
                st.detail = format!("About half have it (the rest have {other}). Or swap with an Ability Capsule: 400 Rift Shards at the Rift Shop.");
            }
            None => {
                st.detail = format!("It can't have that here: pick {}.", abs.join(" or "));
                st.warn = true;
            }
        }
        out.push(st.link("Rift Shop", Go::Guide("rift-shop".into())));
    }
    // ── nature
    if let Some(n) = d.nature(&m.nature) {
        let mut st = Step::new(format!("nature:{}", n.name.to_lowercase()), "🍓", format!("{} nature", n.name), String::new());
        st.detail = if n.neutral() {
            "Any nature that doesn't change a stat works.".into()
        } else {
            format!("+{} −{}. Catch one with it (Synchronize helps), or use a Mint: ₽20,000 at any Poké Mart after 3 badges.", STATS[n.plus], STATS[n.minus])
        };
        out.push(st);
    }
    // ── EVs
    let spent: Vec<String> = (0..6).filter(|&i| m.evs[i] > 0).map(|i| format!("{} {}", m.evs[i], STATS[i])).collect();
    if !spent.is_empty() {
        let vit = ["HP Up", "Protein", "Iron", "Calcium", "Zinc", "Carbos"];
        let mut v: Vec<String> = Vec::new();
        let mut cost = 0;
        for i in 0..6 {
            let n = (m.evs[i].min(100) / 10) as u32;
            if n > 0 {
                v.push(format!("{n} {}", vit[i]));
                cost += n * 9800;
            }
        }
        let mut st = Step::new("evs", "💊", format!("EV training: {}", spent.join(" / ")), format!("Vitamins take a stat to 100 ({}), battles do the rest. A Power item doubles up: Veilstone Store 2F.", v.join(", ")));
        st.maybe_money = cost;
        if let Some(id) = item_id(w, "Protein") {
            st = st.link("Vitamins", Go::Page(Page::Item(id)));
        }
        out.push(st);
    }
    // ── level
    if m.level > 1 {
        out.push(Step::new(format!("level:{}", m.level), "📍", format!("Reach Lv {}", m.level), "Battles, Exp. Share or Rare Candies."));
    }
    out
}

/// (done, total) steps for a team's checklist.
pub(super) fn team_progress(w: Option<&Wiki>, t: &Team) -> (usize, usize) {
    let Some(w) = w else { return (0, 0) };
    let mut done = 0;
    let mut total = 0;
    for m in &t.members {
        for s in steps(w, t.world, m, None) {
            if s.optional {
                continue;
            }
            total += 1;
            if m.done.contains(&s.key) {
                done += 1;
            }
        }
    }
    (done, total)
}

fn fmt_money(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("₽{out}")
}

impl App {
    pub(super) fn team_checklist(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.setup.team.team().cloned() else { return };
        let Some(w) = self.wiki.clone() else {
            kit::card(ui, |ui| {
                ui.label(RichText::new("Getting the game's data…").size(14.0).color(TEXT()));
            });
            return;
        };
        if t.members.is_empty() {
            kit::card(ui, |ui| {
                ui.label(RichText::new("Add a Pokémon in Build first").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            });
            return;
        }
        let story = self.story_stage();
        let all: Vec<Vec<Step>> = t.members.iter().map(|m| steps(&w, t.world, m, story)).collect();
        // ── the summary
        let mut done = 0;
        let mut total = 0;
        let mut money = 0;
        let mut maybe = 0;
        let mut shards = 0;
        let mut warn = 0;
        for (m, ss) in t.members.iter().zip(&all) {
            for s in ss {
                total += 1;
                let d = m.done.contains(&s.key);
                if d {
                    done += 1;
                } else {
                    money += s.money;
                    maybe += s.maybe_money;
                    shards += s.shards;
                }
                if s.warn {
                    warn += 1;
                }
            }
        }
        let hide = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("team-hide-done")).unwrap_or(false));
        let mut set_hide = hide;
        let mut tick_all: Option<bool> = None;
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                // progress ring
                let (r, _) = ui.allocate_exact_size(egui::vec2(84.0, 84.0), Sense::hover());
                let p = ui.painter();
                let frac = if total == 0 { 0.0 } else { done as f32 / total as f32 };
                let c = r.center();
                p.circle_stroke(c, 34.0, Stroke::new(8.0f32, kit::sunken()));
                let n = 64;
                let pts: Vec<egui::Pos2> = (0..=((n as f32 * frac) as usize)).map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * (i as f32 / n as f32);
                    c + egui::vec2(a.cos(), a.sin()) * 34.0
                }).collect();
                if pts.len() > 1 {
                    p.add(egui::Shape::line(pts, Stroke::new(8.0f32, if done == total { kit::C_GREEN } else { ACCENT })));
                }
                p.text(c, Align2::CENTER_CENTER, format!("{}%", (frac * 100.0).round()), crate::fonts::sans_bold_font(17.0), TEXT());
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    let head = if total > 0 && done == total { "Team ready!".to_string() } else { format!("{done} of {total} steps done") };
                    ui.label(RichText::new(head).font(crate::fonts::sans_bold_font(20.0)).color(TEXT()));
                    ui.label(small("Everything it takes to have this team in your game, from the game's own data. Tick things off as you go."));
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                        if money > 0 {
                            pill(ui, &format!("💰  {} to spend", fmt_money(money)), kit::C_GOLD);
                        }
                        if maybe > 0 {
                            pill(ui, &format!("+ {} if you use vitamins", fmt_money(maybe)), kit::C_SLATE);
                        }
                        if shards > 0 {
                            pill(ui, &format!("{shards} Rift Shards"), kit::C_VIOLET);
                        }
                        if warn > 0 {
                            pill(ui, &format!("{warn} problem{}", if warn == 1 { "" } else { "s" }), kit::C_RED);
                        }
                        if story.is_none() {
                            ui.label(small("Set where you are in the story on Game Info's Places page to see what's still ahead."));
                        }
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if soft(ui, "Untick all").clicked() {
                        tick_all = Some(false);
                    }
                    if chip(ui, if hide { "Showing what's left" } else { "Hide done" }, hide, ACCENT).clicked() {
                        set_hide = !hide;
                    }
                });
            });
        });
        if set_hide != hide {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("team-hide-done"), set_hide));
        }
        if let Some(on) = tick_all {
            if let Some(tm) = self.setup.team.team() {
                for (m, ss) in tm.members.iter_mut().zip(&all) {
                    m.done = if on { ss.iter().map(|s| s.key.clone()).collect() } else { Vec::new() };
                }
            }
            self.setup.team.touch();
        }
        // ── one card per member, two columns when there's room
        let avail = ui.available_width();
        let gap = 16.0;
        let cols = if avail >= 1100.0 { 2 } else { 1 };
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut toggles: Vec<(usize, String)> = Vec::new();
        let mut go: Option<Go> = None;
        let mut hunt: Option<String> = None;
        let idx: Vec<usize> = (0..t.members.len()).collect();
        for row in idx.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &i in row {
                    ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(cw);
                        self.check_card(ui, &t, i, &all[i], set_hide, &mut toggles, &mut go, &mut hunt);
                    });
                }
            });
        }
        if !toggles.is_empty() {
            if let Some(tm) = self.setup.team.team() {
                for (i, k) in toggles {
                    if let Some(m) = tm.members.get_mut(i) {
                        if let Some(p) = m.done.iter().position(|x| *x == k) {
                            m.done.remove(p);
                        } else {
                            m.done.push(k);
                        }
                    }
                }
            }
            self.setup.team.touch();
        }
        if let Some(sp) = hunt {
            if let Some(s) = self.team_species(&sp) {
                self.set_target(s.slug.clone());
                self.setup.tab = super::Tab::Shiny;
                self.setup.nav_collapsed.retain(|t| *t != super::Tab::Shiny);
            }
        }
        match go {
            Some(Go::Page(p)) => self.info_open_page(p),
            Some(Go::Guide(g)) => self.info_open_page(Page::Guide(g)),
            Some(Go::Map(areas, title)) => self.show_on_map(&areas, title, None),
            None => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_card(&mut self, ui: &mut egui::Ui, t: &Team, i: usize, ss: &[Step], hide: bool, toggles: &mut Vec<(usize, String)>, go: &mut Option<Go>, hunt: &mut Option<String>) {
        let m = &t.members[i];
        let done = ss.iter().filter(|s| m.done.contains(&s.key)).count();
        kit::card(ui, |ui| {
            // head
            let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), Sense::hover());
            let p = ui.painter().clone();
            let c = egui::pos2(r.min.x + 24.0, r.center().y);
            p.circle_filled(c, 24.0, kit::sunken());
            if !self.team_sprite(&p, &m.species, m.shiny, c, egui::vec2(48.0, 48.0)) {
                p.text(c, Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(12.0), MUTED());
            }
            p.text(egui::pos2(r.min.x + 60.0, r.min.y + 16.0), Align2::LEFT_CENTER, m.display_name(), crate::fonts::sans_bold_font(15.0), TEXT());
            let sub = if m.nickname.trim().is_empty() { format!("Lv {}", m.level) } else { format!("{}  ·  Lv {}", m.species, m.level) };
            p.text(egui::pos2(r.min.x + 60.0, r.min.y + 36.0), Align2::LEFT_CENTER, sub, crate::fonts::sans_font(11.5), MUTED());
            let bar = Rect::from_min_size(egui::pos2(r.max.x - 150.0, r.min.y + 14.0), egui::vec2(110.0, 6.0));
            p.rect_filled(bar, 3.0, kit::sunken());
            let frac = if ss.is_empty() { 0.0 } else { done as f32 / ss.len() as f32 };
            p.rect_filled(Rect::from_min_size(bar.min, egui::vec2(bar.width() * frac, 6.0)), 3.0, if done == ss.len() { kit::C_GREEN } else { ACCENT });
            p.text(egui::pos2(r.max.x, bar.center().y), Align2::RIGHT_CENTER, format!("{done}/{}", ss.len()), crate::fonts::sans_bold_font(11.5), MUTED());
            let hr = Rect::from_min_size(egui::pos2(r.max.x - 150.0, r.min.y + 28.0), egui::vec2(150.0, 22.0));
            let hresp = ui.interact(hr, ui.id().with(("hunt", i)), Sense::click());
            let hc = if hresp.hovered() { kit::C_VIOLET } else { theme::blend(MUTED(), kit::C_VIOLET, 0.5) };
            p.text(egui::pos2(hr.max.x, hr.center().y), Align2::RIGHT_CENTER, "✨ Hunt it shiny ›", crate::fonts::sans_bold_font(11.5), hc);
            if hresp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Make it your Shiny Hunter target").clicked() {
                *hunt = Some(m.species.clone());
            }
            ui.add_space(8.0);
            for s in ss {
                let on = m.done.contains(&s.key);
                if hide && on {
                    continue;
                }
                self.check_row(ui, s, on, i, toggles, go);
            }
            if hide && done == ss.len() {
                ui.label(RichText::new("✔ All done").size(13.0).color(kit::C_GREEN));
            }
        });
    }

    fn check_row(&mut self, ui: &mut egui::Ui, s: &Step, on: bool, i: usize, toggles: &mut Vec<(usize, String)>, go: &mut Option<Go>) {
        let full = ui.available_width();
        let tx = 40.0;
        let dg = ui.painter().layout(s.detail.clone(), crate::fonts::sans_font(11.5), MUTED(), full - tx - 8.0);
        let h = 24.0 + dg.size().y + if s.links.is_empty() { 8.0 } else { 30.0 };
        let (r, resp) = ui.allocate_exact_size(egui::vec2(full, h), Sense::click());
        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
        let p = ui.painter().clone();
        if ht > 0.0 {
            p.rect_filled(r, 8.0, theme::blend(CARD(), CARD_HOVER(), ht * 0.7));
        }
        // the tick
        let b = Rect::from_center_size(egui::pos2(r.min.x + 18.0, r.min.y + 14.0), egui::vec2(20.0, 20.0));
        let col = if s.warn { kit::C_RED } else { ACCENT };
        if on {
            p.rect_filled(b, 6.0, col);
            p.add(egui::Shape::line(vec![b.min + egui::vec2(5.0, 10.5), b.min + egui::vec2(8.5, 14.0), b.min + egui::vec2(15.0, 6.5)], Stroke::new(2.2f32, Color32::WHITE)));
        } else {
            p.rect_stroke(b, 6.0, Stroke::new(1.5f32, theme::blend(LINE(), col, 0.4 + 0.5 * ht)));
        }
        let ink = if on { theme::blend(TEXT(), CARD(), 0.45) } else if s.warn { super::charts::ink(kit::C_RED) } else { TEXT() };
        let title = format!("{}  {}", s.icon, s.title);
        let g = p.layout_no_wrap(title, crate::fonts::sans_bold_font(13.0), ink);
        let tw = g.size().x;
        p.galley(egui::pos2(r.min.x + tx, r.min.y + 14.0 - g.size().y / 2.0), g, ink);
        if on {
            p.line_segment([egui::pos2(r.min.x + tx + 22.0, r.min.y + 14.0), egui::pos2(r.min.x + tx + tw, r.min.y + 14.0)], Stroke::new(1.0f32, ink));
        }
        if s.money > 0 && !on {
            p.text(egui::pos2(r.max.x - 8.0, r.min.y + 14.0), Align2::RIGHT_CENTER, fmt_money(s.money), crate::fonts::sans_bold_font(11.5), super::charts::ink(kit::C_GOLD));
        } else if s.maybe_money > 0 && !on {
            p.text(egui::pos2(r.max.x - 8.0, r.min.y + 14.0), Align2::RIGHT_CENTER, format!("up to {}", fmt_money(s.maybe_money)), crate::fonts::sans_font(11.5), MUTED());
        } else if s.shards > 0 && !on {
            p.text(egui::pos2(r.max.x - 8.0, r.min.y + 14.0), Align2::RIGHT_CENTER, format!("{} shards", s.shards), crate::fonts::sans_bold_font(11.5), super::charts::ink(kit::C_VIOLET));
        }
        let dy = r.min.y + 26.0;
        p.galley(egui::pos2(r.min.x + tx, dy), dg.clone(), if on { theme::blend(MUTED(), CARD(), 0.4) } else { MUTED() });
        // links
        let mut x = r.min.x + tx;
        let ly = dy + dg.size().y + 6.0;
        let mut link_hit = false;
        for (k, (label, target)) in s.links.iter().enumerate() {
            let lg = p.layout_no_wrap(format!("{label} ›"), crate::fonts::sans_font(11.5), TEXT());
            let lr = Rect::from_min_size(egui::pos2(x, ly), lg.size() + egui::vec2(16.0, 8.0));
            let lresp = ui.interact(lr, resp.id.with(("link", k)), Sense::click());
            let lh = kit::anim(ui.ctx(), lresp.id.with("h"), lresp.hovered(), kit::QUICK);
            p.rect_filled(lr, lr.height() / 2.0, theme::blend(kit::sunken(), theme::blend(CARD_HOVER(), ACCENT, 0.2), lh));
            p.galley(lr.min + egui::vec2(8.0, 4.0), lg, TEXT());
            if lresp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *go = Some(target.clone());
                link_hit = true;
            }
            x = lr.max.x + 6.0;
        }
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(if on { "Mark as not done" } else { "Mark as done" }).clicked() && !link_hit {
            toggles.push((i, s.key.clone()));
        }
        let _ = caps;
    }
}

fn pill(ui: &mut egui::Ui, text: &str, col: Color32) {
    let g = ui.painter().layout_no_wrap(text.to_string(), crate::fonts::sans_bold_font(12.0), super::charts::ink(col));
    let (r, _) = ui.allocate_exact_size(g.size() + egui::vec2(20.0, 10.0), Sense::hover());
    ui.painter().rect_filled(r, r.height() / 2.0, super::charts::faint(col, 0.18));
    ui.painter().galley(r.min + egui::vec2(10.0, 5.0), g, Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garchomp_steps() {
        let w = crate::wiki::Wiki::parse(include_bytes!("../../assets/wiki.json")).expect("wiki");
        let ms = crate::teams::import("Garchomp @ Choice Scarf\nAbility: Sand Veil\nEVs: 252 Atk / 4 SpD / 252 Spe\nJolly Nature\n- Outrage\n- Earthquake\n- Stone Edge\n- Fire Fang\n");
        let ss = steps(&w, World::Platinum, &ms[0], None);
        let keys: Vec<&str> = ss.iter().map(|s| s.key.as_str()).collect();
        assert!(keys.contains(&"catch"), "{keys:?}");
        assert!(ss.iter().any(|s| s.title.contains("Gible")), "catches Gible");
        assert!(ss.iter().filter(|s| s.key.starts_with("evolve:")).count() == 2);
        assert!(ss.iter().any(|s| s.key == "move:earthquake" && !s.warn && s.detail.contains("TM26")));
        assert!(ss.iter().any(|s| s.key == "evs" && s.maybe_money > 0));
    }
}
