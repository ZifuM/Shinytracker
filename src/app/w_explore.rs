//! Dashboard widgets for the Pokédex, the Map and Game Info.

use super::info_home::Page;
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::w_build::{Head, Lead, Row};
use super::w_fit::{self as fit, wbox, Shape, WBox};
use super::w_kit as wk;
use super::{App, Tab};
use crate::apps::WidgetCfg;
use crate::battle::World;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Rect, Sense, Stroke};

/// Starter-ish examples for a widget's preview in the gallery.
const SAMPLE_MONS: [&str; 3] = ["TURTWIG", "CHIMCHAR", "PIPLUP"];

impl App {
    /// A search box across the top of `r`; returns the room under it.
    fn w_search(&mut self, ui: &mut egui::Ui, b: &WBox, r: Rect, key: egui::Id, q: &mut String, hint: &str) -> (Rect, egui::Response) {
        let h = b.px(32.0).clamp(26.0, 40.0);
        let (top, rest) = fit::split_top(r, h, b.px(8.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(top).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let resp = kit::search_box_keyed(&mut child, Some(key), q, hint, top.width());
        (rest, resp)
    }

    /// Run `add` in a child that fills `r` (for pickers that lay themselves
    /// out top to bottom).
    fn w_in<R>(&mut self, ui: &mut egui::Ui, r: Rect, add: impl FnOnce(&mut Self, &mut egui::Ui) -> R) -> R {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(r.intersect(ui.clip_rect()));
        add(self, &mut child)
    }

    // ───────────────────────────── Pokédex ─────────────────────────────

    /// A different Pokémon each day: its picture, as big as the box allows,
    /// with its number, name, types and where to find it.
    pub(super) fn w_motd(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "motd");
        let mini = b.shape() == Shape::Mini || b.flat();
        let (body, mut open) = if mini { (b.inner(), false) } else { fit::head(ui, &b, "📖", kit::C_RED, "Pokémon of the day", Some("Open ›")) };
        let p = ui.painter_at(b.r);
        let Some(w) = self.wiki.clone() else { return loading(&p, &b, body, ui) };
        let day = crate::stats::local_day(crate::config::now_unix());
        let all: Vec<&crate::wiki::Mon> = w.all().filter(|m| w.unobtainable(&m.id).is_none()).collect();
        if all.is_empty() {
            return;
        }
        // the same one all day, a different one tomorrow
        let k = (day.unsigned_abs().wrapping_mul(2_654_435_761) >> 7) as usize % all.len();
        let m = all[k].clone();
        let wild = w.wild(&m.id);
        let place = wild.iter().find(|x| x.setting.is_empty()).or(wild.first()).and_then(|x| w.place(&x.area)).map(|p| p.name.clone());
        let how = place.map(|p| format!("Found on {p}")).or_else(|| w.other(&m.id).first().map(|o| format!("{} ({})", o.how, o.place))).unwrap_or_else(|| "Not in the wild".into());
        let resp = ui.interact(body, ui.id().with(("motd", b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
        let col = m.types.first().map(|t| super::pokedex::type_color(t)).unwrap_or(kit::C_RED);
        p.rect_filled(body, b.px(14.0), theme::blend(CARD(), col, if resp.hovered() { 0.18 } else { 0.12 }));
        let ir = body.shrink(b.px(8.0));
        let stacked = b.shape() == Shape::Column || b.shape() == Shape::Mini;
        let (pic, words) = if stacked { fit::split_top(ir, ir.height() * 0.6, b.px(4.0)) } else { fit::split_left(ir, ir.height().min(ir.width() * 0.45), b.px(10.0)) };
        let side = pic.width().min(pic.height());
        self.team_sprite(&p, &m.id, false, pic.center(), vec2(side, side));
        let al = if stacked { Align2::CENTER_CENTER } else { Align2::LEFT_CENTER };
        let mut want = vec![(b.px(17.0), b.px(24.0))];
        let room = words.height();
        if room > b.px(40.0) {
            want.insert(0, (b.px(13.0), b.px(15.0)));
        }
        if room > b.px(62.0) {
            want.push((b.px(20.0), b.px(22.0)));
        }
        if room > b.px(84.0) {
            want.push((b.px(13.0), b.px(30.0)));
        }
        let fl = fit::vflow(words, &want, b.px(3.0));
        let mut i = 0;
        if want.len() > 1 && room > b.px(40.0) {
            fit::label(&p, fl[0], &format!("#{:03}", m.dex), b.font(11.5), MUTED(), al);
            i = 1;
        }
        fit::label(&p, fl[i], &m.name, b.bold(17.0), TEXT(), al);
        if room > b.px(62.0) {
            let tr = fl[i + 1];
            let tags: Vec<Rect> = m.types.iter().map(|t| wk::type_tag(&p.with_clip_rect(Rect::NOTHING), pos2(0.0, 0.0), t, "")).collect();
            let total: f32 = tags.iter().map(|t| t.width() + 5.0).sum::<f32>() - 5.0;
            let mut x = if stacked { tr.center().x - total / 2.0 } else { tr.min.x };
            for t in &m.types {
                x = wk::type_tag(&p, pos2(x, tr.center().y), t, "").max.x + 5.0;
            }
        }
        if room > b.px(84.0) {
            let last = *fl.last().unwrap_or(&words);
            fit::label(&p, last, &how, b.font(11.5), MUTED(), al);
        }
        if resp.on_hover_text(format!("{}: open its page", m.name)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            open = true;
        }
        if open {
            self.open_dex_entry(&m.id);
        }
    }

    /// Look a Pokémon up right here: the results fill the box.
    pub(super) fn w_dexsearch(&mut self, ui: &mut egui::Ui, w: &WidgetCfg) {
        let b = wbox(ui, "dexsearch");
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, _) = if small { (b.inner(), false) } else { fit::head(ui, &b, "🔍", kit::C_RED, "Pokédex search", None) };
        let Some(wiki) = self.wiki.clone() else {
            let p = ui.painter_at(b.r);
            return loading(&p, &b, body, ui);
        };
        let mut q = self.wtext(w.id, "q").clone();
        let (rest, _) = self.w_search(ui, &b, body, egui::Id::new(("w-dexsearch", w.id)), &mut q, "🔍  A name or a dex number");
        *self.wtext(w.id, "q") = q.clone();
        let ql = q.trim().to_lowercase();
        let num: Option<u32> = ql.trim_start_matches('#').parse().ok();
        // as many as there's room for
        let room = fit::tiles(rest, 64, 0.85, b.px(70.0), b.px(70.0), b.px(8.0)).len();
        let hits: Vec<(String, String, u32, Vec<String>)> = if ql.is_empty() {
            wiki.all().take(room).map(|m| (m.id.clone(), m.name.clone(), m.dex, m.types.clone())).collect()
        } else {
            let mut v: Vec<_> = wiki.all().filter(|m| num.map_or_else(|| m.name.to_lowercase().contains(&ql), |d| m.dex == d)).map(|m| (m.id.clone(), m.name.clone(), m.dex, m.types.clone())).collect();
            v.sort_by_key(|h| (!h.1.to_lowercase().starts_with(&ql), h.1.len()));
            v.truncate(room.max(1));
            v
        };
        let p = ui.painter_at(b.r);
        if hits.is_empty() {
            fit::empty(&p, &b, rest, "🔍", "No Pokémon by that name.");
            return;
        }
        let mut open: Option<String> = None;
        for (k, r) in fit::tiles(rest, hits.len(), 0.85, b.px(70.0), b.px(70.0), b.px(8.0)).iter().enumerate() {
            let (id, name, dex, types) = &hits[k];
            let col = types.first().map(|t| super::pokedex::type_color(t)).unwrap_or(kit::C_RED);
            let resp = ui.interact(*r, ui.id().with(("dex-hit", k, w.id)), Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let tp = ui.painter_at(*r);
            tp.rect_filled(*r, b.px(12.0), theme::blend(CARD(), col, 0.07 + 0.08 * ht));
            tp.rect_stroke(*r, b.px(12.0), Stroke::new(1.0f32, theme::blend(LINE(), col, 0.2 + 0.4 * ht)));
            let text_h = if r.height() > b.px(100.0) { b.px(32.0) } else if r.height() > b.px(80.0) { b.px(16.0) } else { 0.0 };
            let (pic, words) = fit::split_bottom(r.shrink(b.px(6.0)), text_h, 0.0);
            let side = pic.width().min(pic.height());
            self.team_sprite(&tp, id, false, pic.center(), vec2(side, side));
            if text_h > 0.0 {
                let fl = fit::stack(words, if text_h > b.px(20.0) { 2 } else { 1 }, 0.0);
                fit::label(&tp, fl[0], name, b.bold(12.0), TEXT(), Align2::CENTER_CENTER);
                if fl.len() > 1 {
                    fit::label(&tp, fl[1], &format!("#{dex:03}"), b.font(10.5), MUTED(), Align2::CENTER_CENTER);
                }
            }
            if resp.on_hover_text(format!("#{dex:03} {name}")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                open = Some(id.clone());
            }
        }
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
    }

    /// Pokémon you keep here, as big as the box allows; a click from their
    /// page.
    pub(super) fn w_favs(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let b = wbox(ui, "favs");
        let list: Vec<String> = if preview { SAMPLE_MONS.iter().map(|s| s.to_string()).collect() } else { w.data.clone() };
        let adding = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new(("w-favs-add", w.id)))).unwrap_or(false);
        let (body, toggle) = fit::head(ui, &b, "❤", kit::C_PINK, "Favourite Pokémon", Some(if adding { "Done" } else { "+ Add" }));
        let mut open: Option<String> = None;
        let mut remove: Option<usize> = None;
        let mut picked: Option<String> = None;
        let p = ui.painter_at(b.r);
        if adding {
            picked = self.w_in(ui, body, |app, ui| app.mon_picker(ui, w.id, "fav", "🔍  Find a Pokémon to add"));
        } else if list.is_empty() {
            fit::empty(&p, &b, body, "❤", "Keep the Pokémon you look at most here. Press “+ Add” to pick some.");
        } else {
            for (i, r) in fit::tiles(body, list.len(), 0.85, b.px(60.0), b.px(60.0), b.px(8.0)).iter().enumerate() {
                let id = &list[i];
                let resp = ui.interact(*r, ui.id().with(("fav", i, w.id)), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let tp = ui.painter_at(*r);
                tp.rect_filled(*r, b.px(12.0), theme::blend(CARD(), kit::C_PINK, 0.06 + 0.08 * ht));
                let with_name = r.height() > b.px(80.0);
                let (pic, name_r) = if with_name { fit::split_bottom(r.shrink(b.px(6.0)), b.px(16.0), 0.0) } else { (r.shrink(b.px(6.0)), Rect::NOTHING) };
                let side = pic.width().min(pic.height());
                self.team_sprite(&tp, id, false, pic.center(), vec2(side, side));
                if with_name {
                    let name = self.mon_name(id);
                    fit::label(&tp, name_r, &name, b.bold(11.5), TEXT(), Align2::CENTER_CENTER);
                }
                let resp = resp.on_hover_text("Open its page · right-click to remove").on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    open = Some(id.clone());
                }
                resp.context_menu(|ui| {
                    if ui.button("Remove from favourites").clicked() {
                        remove = Some(i);
                        ui.close_menu();
                    }
                });
            }
        }
        if toggle && !preview {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("w-favs-add", w.id)), !adding));
        }
        if let Some(id) = picked {
            if !list.contains(&id) {
                let mut v = list.clone();
                v.push(id);
                *out = Some(v);
            }
        }
        if let Some(i) = remove {
            let mut v = list.clone();
            v.remove(i);
            *out = Some(v);
        }
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
    }

    /// What lives in a place you pick, as many as fit.
    pub(super) fn w_catchable(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let b = wbox(ui, "catchable");
        let place = if preview { Some("Route 201".to_string()) } else { w.data.first().cloned() };
        let Some(wiki) = self.wiki.clone() else {
            let (body, _) = fit::head(ui, &b, "🌿", kit::C_GREEN, "Catchable here", None);
            let p = ui.painter_at(b.r);
            return loading(&p, &b, body, ui);
        };
        let Some(place) = place else {
            let (body, _) = fit::head(ui, &b, "🌿", kit::C_GREEN, "Catchable here", None);
            let (note_r, rest) = fit::split_top(body, b.px(16.0), b.px(6.0));
            let p = ui.painter_at(b.r);
            fit::label(&p, note_r, "Pick a place and the Pokémon there show up.", b.font(11.5), MUTED(), Align2::LEFT_CENTER);
            if let Some(pl) = self.w_in(ui, rest, |app, ui| app.place_picker(ui, w.id, "place", "🔍  Route 210, Mt. Coronet…")) {
                *out = Some(vec![pl]);
            }
            return;
        };
        let keys: Vec<String> = super::info_places::groups(&wiki).into_iter().find(|g| g.name == place).map(|g| g.keys).unwrap_or_default();
        // each Pokémon once, with how you meet it
        let mut seen: Vec<(String, String, String)> = Vec::new();
        for x in wiki.data.wild.iter().filter(|x| keys.contains(&x.area) && x.setting.is_empty()) {
            if !seen.iter().any(|s| s.0 == x.species) {
                seen.push((x.species.clone(), method_word(&x.method).to_string(), x.levels.clone()));
            }
        }
        let rows: Vec<Row> = seen.iter().map(|(id, how, lv)| Row::new(self.mon_name(id)).lead(Lead::Wiki(id.clone(), false)).right(format!("{how} · Lv {lv}"), Some(MUTED()))).collect();
        let c = self.f_list(ui, &b, Head { icon: "🌿", col: kit::C_GREEN, title: &place, link: Some("Change") }, &rows, "No wild Pokémon here.");
        if c.link && !preview {
            *out = Some(Vec::new());
        }
        if let Some(k) = c.item {
            let id = seen[k].0.clone();
            self.open_dex_entry(&id);
        } else if c.more {
            self.show_on_map(&keys, place.clone(), None);
        }
    }

    /// What a type hits hard, and what hits it hard; pick the type from the
    /// dots.
    pub(super) fn w_typematch(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let b = wbox(ui, "typematch");
        let world = World::Platinum;
        let types: Vec<&str> = crate::calc::types_in(world).to_vec();
        let cur = if preview { "Fire".to_string() } else { w.data.first().cloned().unwrap_or_else(|| "Water".into()) };
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, _) = if small { (b.inner(), false) } else { fit::head(ui, &b, "⚔", kit::C_ORANGE, "Type matchup", None) };
        let p = ui.painter_at(b.r);
        // the dots to pick a type: a strip across the top (two rows when narrow)
        let per_row = ((body.width() + b.px(4.0)) / (b.px(20.0) + b.px(4.0))).floor().max(1.0) as usize;
        let dot_rows = types.len().div_ceil(per_row.min(types.len()).max(1));
        let dot = ((body.width() - b.px(4.0) * (types.len().min(per_row) as f32 - 1.0)) / types.len().min(per_row) as f32).min(b.px(26.0));
        let show_dots = body.height() > b.px(90.0) || !small;
        let (dots_r, rest) = if show_dots { fit::split_top(body, dot_rows as f32 * (dot + b.px(4.0)), b.px(6.0)) } else { (Rect::NOTHING, body) };
        let mut pick: Option<String> = None;
        if show_dots {
            let n_row = types.len().min(per_row);
            for (i, t) in types.iter().enumerate() {
                let (cx, cy) = (i % n_row, i / n_row);
                let r = Rect::from_min_size(pos2(dots_r.min.x + cx as f32 * (dot + b.px(4.0)), dots_r.min.y + cy as f32 * (dot + b.px(4.0))), vec2(dot, dot));
                let on = *t == cur;
                let resp = ui.interact(r, ui.id().with(("tm-dot", i, w.id)), Sense::click());
                p.circle_filled(r.center(), dot * if on { 0.46 } else { 0.32 }, super::pokedex::type_color(t));
                if on {
                    p.circle_stroke(r.center(), dot * 0.48, Stroke::new(2.0f32, TEXT()));
                }
                if resp.on_hover_text(*t).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some(t.to_string());
                }
            }
        }
        let me = [cur.clone()];
        let hits: Vec<String> = types.iter().filter(|d| crate::calc::eff(&cur, &[d.to_string()], world) > 1.0).map(|d| d.to_string()).collect();
        let weak: Vec<String> = types.iter().filter(|a| crate::calc::eff(a, &me, world) > 1.0).map(|a| a.to_string()).collect();
        let resist: Vec<String> = types.iter().filter(|a| crate::calc::eff(a, &me, world) < 1.0).map(|a| a.to_string()).collect();
        // the type, then three groups: side by side when it's wide
        let (cur_r, groups_r) = fit::split_top(rest, b.px(22.0), b.px(6.0));
        let tag = wk::type_tag(&p, pos2(cur_r.min.x, cur_r.center().y), &cur, "");
        fit::label(&p, Rect::from_min_max(pos2(tag.max.x + b.px(8.0), cur_r.min.y), cur_r.max), "attacking and defending", b.font(11.5), MUTED(), Align2::LEFT_CENTER);
        let wide = groups_r.width() > groups_r.height() * 2.4;
        let parts = if wide { fit::cols(groups_r, 3, b.px(12.0)) } else { fit::stack(groups_r, 3, b.px(6.0)) };
        for ((label, list), r) in [("Hits hard", hits), ("Weak to", weak), ("Takes little from", resist)].into_iter().zip(parts.iter()) {
            let (lab, tags_r) = fit::split_top(*r, b.px(15.0), b.px(3.0));
            fit::label(&p, lab, label, b.bold(11.5), TEXT(), Align2::LEFT_CENTER);
            let clip = p.with_clip_rect(tags_r);
            if list.is_empty() {
                fit::label(&clip, tags_r, "Nothing", b.font(11.0), MUTED(), Align2::LEFT_TOP);
                continue;
            }
            // tags that wrap in their room
            let mut x = tags_r.min.x;
            let mut y = tags_r.min.y + b.px(9.0);
            for t in &list {
                let probe = wk::type_tag(&p.with_clip_rect(Rect::NOTHING), pos2(0.0, 0.0), t, "");
                if x + probe.width() > tags_r.max.x && x > tags_r.min.x {
                    x = tags_r.min.x;
                    y += probe.height() + b.px(4.0);
                }
                x = wk::type_tag(&clip, pos2(x, y), t, "").max.x + b.px(4.0);
            }
        }
        if let Some(t) = pick {
            if !preview {
                *out = Some(vec![t]);
            }
        }
    }

    // ───────────────────────────── Map ─────────────────────────────

    /// Sinnoh at a glance, today's rare grass marked; zoom and look around.
    pub(super) fn w_minimap(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "minimap");
        let _ = self.map_data();
        let map = self.game_map();
        let spots = self.rare_spots_today();
        let link = if spots.is_empty() { "Open ›".to_string() } else { format!("{} rare grass today ›", spots.len()) };
        let small = b.cols <= 3;
        let (body, mut go) = if small { (b.inner(), false) } else { fit::head(ui, &b, "🗺", kit::C_GREEN, "Mini map", Some(&link)) };
        let p = ui.painter_at(b.r);
        let Some(map) = map else {
            fit::empty(&p, &b, body, "🗺", "The map is drawn from your game's map files. It shows here once they're read.");
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
            return;
        };
        let Some((tex, size)) = self.mapui.overview_loaded(ui.ctx(), &map) else {
            fit::empty(&p, &b, body, "🗺", "Open the Map once and the mini map shows here.");
            return;
        };
        let r = body;
        let resp = ui.interact(r, ui.id().with(("w-minimap", b.r.min.x as i32, b.r.min.y as i32)), Sense::click_and_drag());
        p.rect_filled(r, b.px(12.0), crate::gamemap::OCEAN);
        // zoom (1 = the whole map) and the map point in the middle
        let vid = egui::Id::new(("w-minimap-view", b.r.min.x as i32, b.r.min.y as i32));
        let (mut zoom, mut mid) = ui.ctx().data(|d| d.get_temp::<(f32, egui::Vec2)>(vid)).unwrap_or((1.0, size / 2.0));
        let fitk = (r.width() / size.x).min(r.height() / size.y);
        if resp.hovered() {
            let (scroll, pp) = ui.input(|i| (i.smooth_scroll_delta.y, i.pointer.hover_pos()));
            if scroll != 0.0 {
                let old = fitk * zoom;
                zoom = (zoom * (scroll * 0.004).exp()).clamp(1.0, 10.0);
                if let Some(pp) = pp {
                    let at = mid + (pp - r.center()) / old;
                    mid = at - (pp - r.center()) / (fitk * zoom);
                }
                ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
            }
        }
        let s = fitk * zoom;
        if resp.dragged() {
            mid -= resp.drag_delta() / s;
        }
        if resp.double_clicked() {
            zoom = 1.0;
            mid = size / 2.0;
        }
        let half = r.size() / 2.0 / s;
        mid.x = if half.x * 2.0 >= size.x { size.x / 2.0 } else { mid.x.clamp(half.x, size.x - half.x) };
        mid.y = if half.y * 2.0 >= size.y { size.y / 2.0 } else { mid.y.clamp(half.y, size.y - half.y) };
        ui.ctx().data_mut(|d| d.insert_temp(vid, (zoom, mid)));
        let img = Rect::from_min_size(r.center() - mid * s, size * s);
        let clip = p.with_clip_rect(r.intersect(ui.clip_rect()));
        clip.image(tex, img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), egui::Color32::WHITE);
        let t = ui.input(|i| i.time) as f32;
        for sp in spots.iter().filter(|s| s.matrix == 0) {
            let c = img.min + vec2(sp.x as f32 + 0.5, sp.y as f32 + 0.5) * s;
            let pulse = 0.5 + 0.5 * (t * 3.0).sin();
            let rad = (3.0 * zoom.sqrt() * b.s()).min(8.0);
            clip.circle_filled(c, rad + 0.5 + 2.0 * pulse, theme::GOLD.gamma_multiply(0.35));
            clip.circle_filled(c, rad, theme::GOLD);
        }
        if !spots.is_empty() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
        }
        p.rect_stroke(r, b.px(12.0), Stroke::new(1.0f32, LINE()));
        // zoom buttons (and, in a small one, the way to the Map)
        let bs = b.px(26.0).clamp(18.0, 30.0);
        let mut bump = 0.0f32;
        for (k, (label, by)) in [("+", 1.5f32), ("−", 1.0 / 1.5)].iter().enumerate() {
            let br = Rect::from_min_size(pos2(r.max.x - bs - b.px(6.0), r.min.y + b.px(6.0) + k as f32 * (bs + b.px(4.0))), vec2(bs, bs));
            let bresp = ui.interact(br, ui.id().with(("w-minimap-zoom", k, b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
            p.rect_filled(br, b.px(7.0), if bresp.hovered() { kit::CARD_HOVER() } else { kit::CARD().gamma_multiply(0.92) });
            p.rect_stroke(br, b.px(7.0), Stroke::new(1.0f32, LINE()));
            p.text(br.center(), Align2::CENTER_CENTER, *label, b.bold(15.0), TEXT());
            if bresp.on_hover_text(if k == 0 { "Zoom in" } else { "Zoom out" }).clicked() {
                bump = *by;
            }
        }
        if small {
            let br = Rect::from_min_size(pos2(r.max.x - bs - b.px(6.0), r.max.y - bs - b.px(6.0)), vec2(bs, bs));
            let bresp = ui.interact(br, ui.id().with(("w-minimap-open", b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
            p.rect_filled(br, b.px(7.0), if bresp.hovered() { kit::CARD_HOVER() } else { kit::CARD().gamma_multiply(0.92) });
            p.text(br.center(), Align2::CENTER_CENTER, "↗", b.bold(13.0), TEXT());
            if bresp.on_hover_text("Open the Map").clicked() {
                go = true;
            }
        }
        if bump != 0.0 {
            let z = (zoom * bump).clamp(1.0, 10.0);
            ui.ctx().data_mut(|d| d.insert_temp(vid, (z, mid)));
        }
        if zoom > 1.01 {
            p.text(pos2(r.min.x + b.px(10.0), r.max.y - b.px(10.0)), Align2::LEFT_BOTTOM, format!("{:.1}×", zoom), b.bold(11.0), egui::Color32::from_white_alpha(200));
        }
        let resp = resp.on_hover_text("Scroll to zoom · drag to look around · double-click to see it all");
        if resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if go {
            self.go_to(Tab::Map);
        }
    }

    /// Places you pick, with what lives there.
    pub(super) fn w_mapfavs(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let b = wbox(ui, "mapfavs");
        let list: Vec<String> = if preview { vec!["Route 201".into(), "Lake Verity".into(), "Eterna Forest".into()] } else { w.data.clone() };
        let adding = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new(("w-mapfavs-add", w.id)))).unwrap_or(false);
        let Some(wiki) = self.wiki.clone() else {
            let (body, _) = fit::head(ui, &b, "📍", kit::C_GREEN, "Favourite places", None);
            let p = ui.painter_at(b.r);
            return loading(&p, &b, body, ui);
        };
        let groups = super::info_places::groups(&wiki);
        let mut picked: Option<String> = None;
        let mut show: Option<(Vec<String>, String)> = None;
        let toggle;
        if adding {
            let (body, t) = fit::head(ui, &b, "📍", kit::C_GREEN, "Favourite places", Some("Done"));
            toggle = t;
            picked = self.w_in(ui, body, |app, ui| app.place_picker(ui, w.id, "place", "🔍  Find a place to add"));
        } else {
            let mut keys_of: Vec<Vec<String>> = Vec::new();
            let rows: Vec<Row> = list
                .iter()
                .map(|name| {
                    let keys = groups.iter().find(|g| &g.name == name).map(|g| g.keys.clone()).unwrap_or_default();
                    let mut mons: Vec<String> = Vec::new();
                    for x in wiki.data.wild.iter().filter(|x| keys.contains(&x.area) && x.setting.is_empty()) {
                        if !mons.contains(&x.species) {
                            mons.push(x.species.clone());
                        }
                    }
                    keys_of.push(keys);
                    let names: Vec<String> = mons.iter().take(4).map(|m| self.mon_name(m)).collect();
                    Row::new(name.clone()).lead(mons.first().map_or(Lead::Icon("📍".into(), kit::C_GREEN), |m| Lead::Wiki(m.clone(), false))).sub(names.join(", ")).right(format!("{}", mons.len()), Some(MUTED())).tip("Show it on the Map · right-click to remove").menu("Remove")
                })
                .collect();
            let c = self.f_list(ui, &b, Head { icon: "📍", col: kit::C_GREEN, title: "Favourite places", link: Some("+ Add") }, &rows, "Keep the places you go to most here. Press “+ Add” to pick some.");
            toggle = c.link;
            if let Some(k) = c.item {
                show = Some((keys_of[k].clone(), list[k].clone()));
            }
            if let (Some(k), false) = (c.menu, preview) {
                let mut v = list.clone();
                v.remove(k);
                *out = Some(v);
            }
        }
        if toggle && !preview {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("w-mapfavs-add", w.id)), !adding));
        }
        if let Some(pl) = picked {
            if !list.contains(&pl) {
                let mut v = list.clone();
                v.push(pl);
                *out = Some(v);
            }
        }
        if let Some((keys, name)) = show {
            self.show_on_map(&keys, name, None);
        }
    }

    /// Where your current target can be found, best spots first.
    pub(super) fn w_huntwhere(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "huntwhere");
        let Some(wiki) = self.wiki.clone() else {
            let (body, _) = fit::head(ui, &b, "🔭", kit::C_TEAL, "Where's my hunt", None);
            let p = ui.painter_at(b.r);
            return loading(&p, &b, body, ui);
        };
        let idx = self.shown_target_idx(ui.ctx());
        let target = idx.and_then(|i| wiki.by_dex(self.species[i].dex)).cloned();
        let mut rows: Vec<Row> = Vec::new();
        let mut keys_of: Vec<Vec<String>> = Vec::new();
        let mut title = "Where's my hunt".to_string();
        let mut empty = "Pick a target in the Shiny Hunter and where to find it shows here.".to_string();
        if let Some(m) = &target {
            title = format!("Where's {}", m.name);
            let mut places: Vec<(String, String, String, u32)> = Vec::new();
            for x in wiki.wild(&m.id).iter().filter(|x| x.setting.is_empty() || self.world_has(&x.setting)) {
                let name = wiki.place(&x.area).map(|p| if p.group.is_empty() { p.name.clone() } else { p.group.clone() }).unwrap_or_else(|| x.area.clone());
                let best = x.chance.morning.max(x.chance.day).max(x.chance.night);
                if !places.iter().any(|p| p.0 == name && p.1 == x.method) {
                    places.push((name, x.method.clone(), x.area.clone(), best));
                }
            }
            places.sort_by(|a, b| b.3.cmp(&a.3));
            empty = match wiki.other(&m.id).first() {
                Some(o) => format!("Not in the wild: {} ({}).", o.how, o.place),
                None => "Not in the wild.".to_string(),
            };
            for (name, method, _, best) in &places {
                rows.push(Row::new(name.clone()).lead(Lead::Wiki(m.id.clone(), true)).right(format!("{} · {best}%", method_word(method)), Some(MUTED())).tip("Show it on the Map"));
                keys_of.push(places.iter().filter(|p| &p.0 == name).map(|p| p.2.clone()).collect());
            }
        }
        let c = self.f_list(ui, &b, Head { icon: "🔭", col: kit::C_TEAL, title: &title, link: None }, &rows, &empty);
        if let (Some(k), Some(m)) = (c.item, &target) {
            self.show_on_map(&keys_of[k], m.name.clone(), None);
        } else if let (true, Some(m)) = (c.more, &target) {
            let all: Vec<String> = keys_of.concat();
            self.show_on_map(&all, m.name.clone(), None);
        }
    }

    /// The places you looked at last on the Map.
    pub(super) fn w_maprecent(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "maprecent");
        let recent = self.cfg.home.map_recent.clone();
        let map = self.game_map();
        let rows: Vec<Row> = match &map {
            Some(map) => recent
                .iter()
                .map(|z| {
                    let (kind, col, _) = super::map_panel::zone_kind(map, *z);
                    Row::new(self.zone_label(map, *z)).lead(Lead::Icon("📍".into(), col)).right(kind, Some(MUTED()))
                })
                .collect(),
            None => Vec::new(),
        };
        let c = self.f_list(ui, &b, Head { icon: "🕘", col: kit::C_GREEN, title: "Recent places", link: None }, &rows, "The places you open on the Map show up here.");
        if let (Some(k), Some(map)) = (c.item, map) {
            self.goto_zones(&map, &[recent[k]]);
        }
    }

    // ───────────────────────────── Game Info ─────────────────────────────

    /// The newest news from the game's wiki: as many notices as fit, each
    /// with as much of its text as there's room for.
    pub(super) fn w_infonews(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "infonews");
        let (body, mut open) = fit::head(ui, &b, "📰", kit::C_VIOLET, "Latest game news", Some("Game Info ›"));
        let p = ui.painter_at(b.r);
        let Some(w) = self.wiki.clone() else { return loading(&p, &b, body, ui) };
        let notices = &w.data.home.notices;
        if notices.is_empty() {
            fit::empty(&p, &b, body, "📰", "No news right now.");
        }
        let mut read: Option<String> = None;
        // side by side when it's wide, else one under another
        let n = notices.len().min(4);
        let per = b.px(70.0);
        let wide = body.width() > body.height() * 2.2;
        let k = if wide { (((body.width() + b.px(12.0)) / (b.px(220.0) + b.px(12.0))).floor() as usize).clamp(1, n.max(1)) } else { (((body.height() + b.px(10.0)) / (per + b.px(10.0))).floor() as usize).clamp(1, n.max(1)) };
        let parts = if wide { fit::cols(body, k.min(n).max(1), b.px(12.0)) } else { fit::stack(body, k.min(n).max(1), b.px(10.0)) };
        for (i, (nt, r)) in notices.iter().zip(parts.iter()).enumerate() {
            fit::tile(&p, *r, kit::C_VIOLET, b.px(10.0));
            let ir = r.shrink(b.px(8.0));
            let has_link = nt.guide().is_some();
            let (main, link_r) = if has_link && ir.height() > b.px(52.0) { fit::split_bottom(ir, b.px(15.0), b.px(4.0)) } else { (ir, Rect::NOTHING) };
            let (title_r, text_r) = fit::split_top(main, b.px(17.0), b.px(3.0));
            fit::label(&p, title_r, nt.title.trim_end_matches(['.', ':']), b.bold(13.0), TEXT(), Align2::LEFT_CENTER);
            fit::note(&p, text_r, &nt.text, b.font(11.5));
            if link_r.is_positive() {
                let label = if nt.label.is_empty() { "Read more".to_string() } else { nt.label.clone() };
                let lr = fit::label(&p, link_r, &format!("{label}  ›"), b.bold(11.5), kit::primary(), Align2::LEFT_CENTER);
                if ui.interact(lr.expand(3.0), ui.id().with(("news-read", i, b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    read = nt.guide().map(|g| g.to_string());
                }
            }
        }
        if notices.len() > parts.len() && !notices.is_empty() {
            let _ = &mut open;
        }
        if let Some(g) = read {
            self.go_to(Tab::Info);
            self.info_read(&g, None, None);
        } else if open {
            self.go_to(Tab::Info);
        }
    }

    /// A different how-to-play guide every day.
    pub(super) fn w_tip(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "tip");
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, _) = if small { (b.inner(), false) } else { fit::head(ui, &b, "💡", kit::C_GOLD, "Tip of the day", None) };
        let p = ui.painter_at(b.r);
        let Some(w) = self.wiki.clone() else { return loading(&p, &b, body, ui) };
        let guides = w.guides_sorted();
        if guides.is_empty() {
            return;
        }
        let day = crate::stats::local_day(crate::config::now_unix());
        let g = guides[day.rem_euclid(guides.len() as i64) as usize].clone();
        // the group, the title, the summary (as much as fits) and the button
        let with_btn = body.height() > b.px(96.0);
        let (main, btn) = if with_btn { fit::split_bottom(body, b.px(30.0), b.px(8.0)) } else { (body, Rect::NOTHING) };
        let with_group = !g.group.is_empty() && main.height() > b.px(56.0);
        let (group_r, main) = if with_group { fit::split_top(main, b.px(13.0), b.px(2.0)) } else { (Rect::NOTHING, main) };
        let (title_r, text_r) = fit::split_top(main, b.px(19.0), b.px(4.0));
        if with_group {
            fit::label(&p, group_r, &g.group.to_uppercase(), b.bold(10.0), theme::blend(MUTED(), kit::C_GOLD, 0.5), Align2::LEFT_CENTER);
        }
        fit::label(&p, title_r, &g.title, b.bold(15.0), TEXT(), Align2::LEFT_CENTER);
        fit::note(&p, text_r, &g.summary, b.font(12.0));
        let mut read = false;
        if with_btn {
            let br = Rect::from_min_size(btn.min, vec2(btn.width().min(b.px(190.0)), btn.height()));
            read = fit::button(ui, br, "Read the guide  ›", false, ("tip-read", b.r.min.x as i32, b.r.min.y as i32));
        } else if ui.interact(body, ui.id().with(("tip-open", b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).on_hover_text("Read the guide").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            read = true;
        }
        if read {
            self.go_to(Tab::Info);
            self.info_read(&g.id, None, None);
        }
    }

    /// Search the game: moves, items, places, guides (and Pokémon); the
    /// results fill the box.
    pub(super) fn w_infosearch(&mut self, ui: &mut egui::Ui, w: &WidgetCfg) {
        let b = wbox(ui, "infosearch");
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, _) = if small { (b.inner(), false) } else { fit::head(ui, &b, "🔎", kit::C_CYAN, "Quick lookup", None) };
        let Some(wiki) = self.wiki.clone() else {
            let p = ui.painter_at(b.r);
            return loading(&p, &b, body, ui);
        };
        let mut q = self.wtext(w.id, "q").clone();
        let (rest, resp) = self.w_search(ui, &b, body, egui::Id::new(("w-infosearch", w.id)), &mut q, "🔍  Surf, Leftovers, Route 210, saving…");
        *self.wtext(w.id, "q") = q.clone();
        let room = fit::rows(rest, 40, b.px(32.0), b.px(4.0), 1.0).len().max(1);
        let hits = if q.trim().is_empty() { Vec::new() } else { self.info_quick(&wiki, &q, room) };
        let mut open: Option<(super::info_home::Kind, String)> = None;
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            open = hits.first().map(|h| (h.0, h.1.clone()));
        }
        let p = ui.painter_at(b.r);
        if q.trim().is_empty() {
            fit::note(&p, rest, "Type a move, an item, a place or a guide. Enter opens the first result.", b.font(11.5));
        } else if hits.is_empty() {
            fit::note(&p, rest, "Nothing found.", b.font(11.5));
        }
        for (k, r) in fit::rows(rest, hits.len(), b.px(32.0), b.px(4.0), 1.4).iter().enumerate() {
            let (kind, key, icon, col, title, sub) = &hits[k];
            let resp = fit::hit(ui, *r, ("iq", k, w.id), b.px(8.0));
            let rp = ui.painter_at(*r);
            let ir = r.shrink2(vec2(b.px(4.0), b.px(2.0)));
            let (chip, words) = fit::split_left(ir, ir.height().min(b.px(28.0)), b.px(8.0));
            kit::icon_chip(&rp, Rect::from_center_size(chip.center(), vec2(chip.width(), chip.width())), icon, *col, false);
            let (t_r, s_r) = fit::split_right(words, (words.width() * 0.4).min(b.px(160.0)), b.px(8.0));
            fit::label(&rp, t_r, title, b.bold(12.5), TEXT(), Align2::LEFT_CENTER);
            fit::label(&rp, s_r, sub, b.font(11.0), MUTED(), Align2::RIGHT_CENTER);
            if resp.clicked() {
                open = Some((*kind, key.clone()));
            }
        }
        if let Some((k, key)) = open {
            self.info_quick_open(k, key);
        }
    }

    /// Where you are in the story (Game Info's own card, filling the box).
    pub(super) fn w_adventure(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "adventure");
        let Some(w) = self.wiki.clone() else {
            let p = ui.painter_at(b.r);
            let body = b.inner();
            return loading(&p, &b, body, ui);
        };
        let r = b.r;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(r.intersect(ui.clip_rect()));
        kit::set_card_fill(Some(r.height()));
        self.info_adventure(&mut child, &w);
        kit::set_card_fill(None);
    }

    /// Game Info pages you pinned, then the ones you looked at lately.
    pub(super) fn w_bookmarks(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "bookmarks");
        let pins: Vec<Page> = self.cfg.info_pins.iter().filter_map(|k| Page::from_key(k)).collect();
        let recent: Vec<Page> = self.setup.info.recent.iter().filter(|p| !pins.contains(p)).take(12).cloned().collect();
        let pages: Vec<(Page, bool)> = pins.iter().map(|p| (p.clone(), true)).chain(recent.iter().map(|p| (p.clone(), false))).collect();
        let rows: Vec<Row> = pages
            .iter()
            .map(|(pg, pinned)| {
                let (icon, col) = match pg {
                    Page::Move(_) => ("💥", kit::C_VIOLET),
                    Page::Item(_) => ("🎒", kit::C_GOLD),
                    Page::Place(_) => ("🗺", kit::C_GREEN),
                    Page::Guide(_) => ("📚", kit::C_CYAN),
                    _ => ("•", kit::C_SLATE),
                };
                Row::new(self.page_label(pg)).lead(Lead::Icon(icon.into(), col)).right(if *pinned { "📌" } else { "" }, Some(MUTED()))
            })
            .collect();
        let c = self.f_list(ui, &b, Head { icon: "🔖", col: kit::C_CYAN, title: "Pinned pages", link: None }, &rows, "Pin a move, item, place or guide in Game Info (📌 on its page) and it shows here.");
        if let Some(k) = c.item {
            let pg = pages[k].0.clone();
            if let Page::Guide(id) = &pg {
                let id = id.clone();
                self.go_to(Tab::Info);
                self.info_read(&id, None, None);
            } else {
                self.info_open_page(pg);
            }
        }
    }
}

/// "grass", "surf"… in words.
pub(super) fn method_word(m: &str) -> &str {
    match m {
        "grass" => "Grass",
        "radar" => "Poké Radar",
        "surf" => "Surfing",
        "old_rod" => "Old Rod",
        "good_rod" => "Good Rod",
        "super_rod" => "Super Rod",
        "cave" => "Cave",
        _ => m,
    }
}

/// A widget waiting for the game's data.
pub(super) fn loading(p: &egui::Painter, b: &WBox, r: Rect, ui: &egui::Ui) {
    fit::empty(p, b, r, "⏳", "Getting the game's data… (it needs the internet the first time)");
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
}
