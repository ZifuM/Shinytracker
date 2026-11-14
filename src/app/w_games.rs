//! Dashboard widgets for the Card Collector.

use super::cards_app::{xp_bar, xp_rules, CARD_ASPECT};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_build::{Head, Lead, Row};
use super::w_fit::{self as fit, wbox, Shape};
use super::{App, Tab};
use crate::cards::{self, Rarity, XP_PER_PACK};
use crate::theme::GOLD;
use eframe::egui::{self, pos2, vec2, Align2, Rect, Sense};

impl App {
    fn open_cards(&mut self, view: super::cards_app::View) {
        self.setup.cards.view = view;
        self.go_to(Tab::Cards);
    }

    /// XP towards the next booster pack, as a bar that takes the width.
    pub(super) fn w_cards_xp(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "cards_xp");
        let f = self.setup.cards.file().clone();
        let pack_at = self.setup.cards.pack_at;
        let gain = self.setup.cards.gains.back().map(|g| g.0.clone());
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, mut go) = if small { (b.inner(), false) } else { fit::head(ui, &b, "⭐", GOLD, "Booster progress", Some("Open ›")) };
        let p = ui.painter_at(b.r);
        let mut want = vec![(b.px(22.0), b.px(64.0)), (b.px(10.0), b.px(18.0))];
        let with_note = body.height() > b.px(70.0);
        let with_packs = f.packs > 0 && body.height() > b.px(96.0);
        if with_note {
            want.push((b.px(13.0), b.px(15.0)));
        }
        if with_packs {
            want.push((b.px(14.0), b.px(16.0)));
        }
        let fl = fit::vflow(body, &want, b.px(6.0));
        let at = fit::hero(&p, Rect::from_min_max(fl[0].min, pos2(fl[0].max.x - b.px(80.0), fl[0].max.y)), &f.xp.to_string(), fl[0].height().min(b.px(56.0)), TEXT(), Align2::LEFT_CENTER);
        p.text(pos2(at.max.x + b.px(6.0), at.max.y - at.height() * 0.18), Align2::LEFT_BOTTOM, format!("/ {XP_PER_PACK} XP"), b.font(12.0), MUTED());
        xp_bar(&p, fl[1], f.xp as f32 / XP_PER_PACK as f32, ui.input(|i| i.time) as f32, pack_at);
        let mut k = 2;
        if with_note {
            let t = match &gain {
                Some(g) => format!("Last: +{} XP · {}", g.xp, g.why),
                None => format!("{} XP to the next pack", XP_PER_PACK - f.xp),
            };
            fit::label(&p, fl[k], &t, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
            k += 1;
        }
        if with_packs {
            fit::label(&p, fl[k], &format!("🎁  {} pack{} to open", f.packs, if f.packs == 1 { "" } else { "s" }), b.font(12.0), GOLD, Align2::LEFT_CENTER);
        }
        let resp = ui.interact(b.r, ui.id().with(("cards-xp", b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
        if resp.on_hover_text(xp_rules()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && small {
            go = true;
        }
        if go {
            self.open_cards(super::cards_app::View::Binder);
        }
    }

    /// The booster packs, as big as the box allows.
    pub(super) fn w_cards_packs(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "cards_packs");
        let f = self.setup.cards.file().clone();
        let mini = b.shape() == Shape::Mini;
        let (body, go) = if mini { (b.inner(), false) } else { fit::head(ui, &b, "🎁", kit::C_PINK, "Packs to open", Some("All ›")) };
        let p = ui.painter_at(b.r);
        let words = if f.packs > 0 { format!("{} waiting", f.packs) } else { format!("None yet · {} XP to the next", XP_PER_PACK - f.xp) };
        let (top, room) = if body.height() > b.px(90.0) { fit::split_top(body, b.px(18.0), b.px(8.0)) } else { (Rect::NOTHING, body) };
        if top.is_positive() {
            fit::label(&p, top, &words, b.bold(14.0), if f.packs > 0 { TEXT() } else { MUTED() }, Align2::LEFT_CENTER);
        }
        let mut open: Option<&'static str> = None;
        let n = cards::SETS.len();
        // packs keep their shape (1 : 1.45), side by side
        let slots = fit::cols(room, n, b.px(12.0));
        for (k, (s, slot)) in cards::SETS.iter().zip(slots.iter()).enumerate() {
            let w = slot.width().min(slot.height() / 1.45);
            let r = Rect::from_center_size(slot.center(), vec2(w, w * 1.45));
            let resp = ui.interact(r, ui.id().with(("w-pack", k, b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && f.packs > 0, kit::QUICK);
            self.draw_booster(ui, r.translate(vec2(0.0, -4.0 * ht)), s, f.packs > 0);
            if resp.on_hover_text(if f.packs > 0 { format!("Open a {} pack", s.name) } else { "No packs yet".to_string() }).clicked() && f.packs > 0 {
                open = Some(s.id);
            }
        }
        if !top.is_positive() && f.packs > 0 {
            let badge = Rect::from_min_size(b.r.min + vec2(b.px(8.0), b.px(8.0)), vec2(b.px(26.0), b.px(20.0)));
            p.rect_filled(badge, badge.height() / 2.0, GOLD);
            p.text(badge.center(), Align2::CENTER_CENTER, f.packs.to_string(), b.bold(11.0), egui::Color32::BLACK);
        }
        if let Some(s) = open {
            self.go_to(Tab::Cards);
            self.setup.cards.choose = cards::set(s).map(|x| x.id);
        } else if go {
            self.open_cards(super::cards_app::View::Packs);
        }
    }

    /// Your best recent cards, as many as fit, each as big as it can be.
    pub(super) fn w_cards_pulls(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "cards_pulls");
        let f = self.setup.cards.file().clone();
        let (body, go) = fit::head(ui, &b, "🃏", kit::C_GOLD, "Latest pulls", Some("Binder ›"));
        let mut best: Vec<&cards::Card> = f.pulls.iter().take(4).flat_map(|p| p.cards.iter()).filter_map(|id| cards::by_id(id)).filter(|c| c.rarity >= Rarity::Uncommon).collect();
        best.sort_by_key(|c| std::cmp::Reverse(c.rarity));
        best.dedup_by(|a, b| a.id == b.id);
        let mut look: Option<String> = None;
        if best.is_empty() {
            let p = ui.painter_at(b.r);
            fit::empty(&p, &b, body, "🃏", "Open a pack and your best cards show here.");
        } else {
            // cards keep their shape: a grid of slots that shape, as many as fit
            let cells = fit::tiles(body, best.len(), 1.0 / CARD_ASPECT, b.px(40.0), b.px(40.0) * CARD_ASPECT, b.px(8.0));
            for (k, slot) in cells.iter().enumerate() {
                let c = best[k];
                let w = slot.width().min(slot.height() / CARD_ASPECT);
                let r = Rect::from_center_size(slot.center(), vec2(w, w * CARD_ASPECT));
                self.draw_card(ui, r, c, false, true);
                if ui.interact(r, ui.id().with(("w-pull", k, b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).on_hover_text(&c.name).clicked() {
                    look = Some(c.id.clone());
                }
            }
        }
        if look.is_some() {
            self.setup.cards.open = look;
            self.open_cards(super::cards_app::View::Binder);
        } else if go {
            self.open_cards(super::cards_app::View::Binder);
        }
    }

    /// How much of each set you have, with its holo rares.
    pub(super) fn w_cards_sets(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "cards_sets");
        let f = self.setup.cards.file().clone();
        let rows: Vec<Row> = cards::SETS
            .iter()
            .map(|s| {
                let n = cards::of_set(s.id).len();
                let h = f.unique(Some(s.id));
                let holos = cards::of_set(s.id).into_iter().filter(|c| c.rarity == Rarity::Holo).collect::<Vec<_>>();
                let got = holos.iter().filter(|c| f.count(&c.id) > 0).count();
                Row::new(s.name).sub(format!("★ {got} of {} holo rares", holos.len())).right(format!("{h} / {n}"), Some(MUTED())).bar(h as f32 / n.max(1) as f32, kit::C_CYAN)
            })
            .collect();
        let c = self.f_list(ui, &b, Head { icon: "📚", col: kit::C_CYAN, title: "Set completion", link: Some("Binder ›") }, &rows, "No sets yet.");
        if c.link || c.item.is_some() || c.more {
            self.open_cards(super::cards_app::View::Binder);
        }
    }

    /// Today's missions (and the week's when there's room).
    pub(super) fn w_cards_missions(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "cards_missions");
        let day = crate::stats::local_day(crate::config::now_unix());
        self.setup.cards.file().roll_over(day);
        let f = self.setup.cards.file().clone();
        let mut rows: Vec<Row> = Vec::new();
        for (m, have) in f.day.missions.iter().map(|m| (m, f.day.progress(m))).chain(f.week.missions.iter().map(|m| (m, f.week.progress(m)))) {
            let done = m.done;
            rows.push(
                Row::new(m.text())
                    .lead(Lead::Icon(if done { "✔".into() } else { m.icon().into() }, if done { crate::theme::GREEN } else { kit::C_CYAN }))
                    .sub(format!("{}/{}", have.min(m.goal), m.goal))
                    .right(if done { "Done".to_string() } else { format!("+{} XP", m.xp) }, Some(if done { crate::theme::GREEN } else { GOLD }))
                    .bar(have as f32 / m.goal.max(1) as f32, if done { crate::theme::GREEN } else { kit::C_CYAN }),
            );
        }
        let c = self.f_list(ui, &b, Head { icon: "🎯", col: kit::C_CYAN, title: "Card missions", link: Some("All ›") }, &rows, "No missions right now.");
        if c.link || c.item.is_some() || c.more {
            self.open_cards(super::cards_app::View::Missions);
        }
    }
}
