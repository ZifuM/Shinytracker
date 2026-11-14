//! Dashboard widgets for the Shiny Hunter, and the Dashboard's own notes
//! (the clock, current hunt and week are in `w_core`), laid out by the box
//! they fill.

use super::home::ago;
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::w_fit::{self as fit, wbox, Shape};
use super::w_kit::n;
use super::{App, Tab};
use crate::apps::WidgetCfg;
use crate::stats::found_by;
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Rect, Sense, Stroke};

impl App {
    /// A notepad that keeps what you write, filling the box.
    pub(super) fn w_notes(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool, out: &mut Option<Vec<String>>) {
        let b = wbox(ui, "notes");
        let mut text = if preview { "Hunt Gible on Route 206 after the gym.\nAsk about the next event.".to_string() } else { w.data.first().cloned().unwrap_or_default() };
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, _) = if small { (b.inner(), false) } else { fit::head(ui, &b, "📝", kit::C_GOLD, "Notes", None) };
        let font = egui::FontId::proportional((13.0 * b.s()).max(10.0));
        let rows = ((body.height() - 8.0) / (font.size * 1.3)).floor().max(1.0) as usize;
        let edit = egui::TextEdit::multiline(&mut text)
            .id(egui::Id::new(("w-notes", w.id)))
            .hint_text("Write anything here…")
            .desired_width(body.width())
            .desired_rows(rows)
            .font(font);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(body.intersect(ui.clip_rect()));
        if child.add_sized(body.size(), edit).changed() && !preview {
            *out = Some(vec![text.clone()]);
        }
    }

    /// Your latest shinies, big, taking turns.
    pub(super) fn w_showcase(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "showcase");
        let recent: Vec<(Option<String>, String, u64, u64)> = self.cfg.history.iter().rev().take(5).map(|h| (h.slug.clone(), h.target.clone(), h.encounters, h.when_unix)).collect();
        let mini = b.shape() == Shape::Mini || b.flat();
        let (body, link) = if mini { (b.inner(), false) } else { fit::head(ui, &b, "🌟", GOLD, "Shiny showcase", Some("Shiny Dex ›")) };
        let mut trophy: Option<Option<String>> = if link { Some(None) } else { None };
        let p = ui.painter_at(b.r);
        if recent.is_empty() {
            let side = body.width().min(body.height() * 0.6);
            let c = pos2(body.center().x, body.min.y + body.height() * 0.4);
            p.circle_stroke(c, side * 0.3, Stroke::new(1.5f32, theme::blend(LINE(), GOLD, 0.3)));
            p.text(c, Align2::CENTER_CENTER, "?", b.bold(22.0), MUTED());
            fit::label(&p, Rect::from_min_max(pos2(body.min.x, body.max.y - b.px(16.0)), body.max), "Your first shiny shows here", b.font(12.0), MUTED(), Align2::CENTER_CENTER);
        } else {
            // a new one every 5 seconds
            let t = ui.input(|i| i.time);
            let k = (t / 5.0) as usize % recent.len();
            let fade = ((t % 5.0) / 0.4).min(1.0) as f32;
            let (slug, name, enc, when) = recent[k].clone();
            let resp = ui.interact(body, ui.id().with(("showcase", b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
            p.rect_filled(body, b.px(14.0), theme::blend(CARD(), GOLD, 0.08));
            // the sprite beside the words when it's flat, above them otherwise
            let side_by = body.width() > body.height() * 1.9;
            let text_h = if body.height() > b.px(110.0) || side_by { b.px(36.0) } else { 0.0 };
            let (pic, words) = if side_by { fit::split_left(body.shrink(b.px(6.0)), body.height(), b.px(10.0)) } else { fit::split_bottom(body.shrink(b.px(8.0)), text_h, b.px(4.0)) };
            let side = pic.width().min(pic.height());
            let c = pic.center();
            for (rad, a) in [(0.56, 0.06), (0.42, 0.09)] {
                p.circle_filled(c, side * rad, GOLD.gamma_multiply(a * fade));
            }
            if let Some((_, sp)) = self.species_of(slug.as_deref()) {
                let s = side * (0.9 + 0.1 * fade);
                self.draw_sprite_centered(&p, &sp, true, c, vec2(s, s));
            }
            for (dx, dy, sz) in [(-0.46, -0.3, 6.0), (0.44, -0.18, 4.5), (0.36, 0.3, 3.5)] {
                theme::sparkle(&p, c + vec2(dx, dy) * side, b.px(sz) * fade, GOLD);
            }
            if words.is_positive() && words.height() > b.px(20.0) {
                let fl = fit::vflow(words, &[(b.px(17.0), b.px(19.0)), (b.px(13.0), b.px(15.0))], b.px(2.0));
                let al = if side_by { Align2::LEFT_CENTER } else { Align2::CENTER_CENTER };
                fit::label(&p, fl[0], &name, b.bold(15.0), TEXT(), al);
                fit::label(&p, fl[1], &format!("{} encounters · {}", n(enc), ago(when)), b.font(11.5), MUTED(), al);
            }
            // which one this is
            let dots = recent.len() as f32;
            for i in 0..recent.len() {
                let dc = pos2(body.center().x + (i as f32 - (dots - 1.0) / 2.0) * b.px(12.0), body.min.y + b.px(9.0));
                p.circle_filled(dc, if i == k { b.px(3.5) } else { b.px(2.5) }, if i == k { GOLD } else { MUTED().gamma_multiply(0.5) });
            }
            if resp.on_hover_text("Open its trophy").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                trophy = Some(slug);
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(if fade < 1.0 { 30 } else { 500 }));
        }
        if let Some(t) = trophy {
            self.setup.tab = Tab::Pokedex;
            self.setup.pokedex.shiny = true;
            self.setup.pokedex.open = None;
            self.setup.trophy_back = t.as_ref().map(|_| Tab::Home);
            self.setup.dex.open = t;
            self.setup.scroll_top = true;
        }
    }

    /// How full your Shiny Dex is (a ring), and the next ones to get, as
    /// many as there's room for.
    pub(super) fn w_shinydex(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "shinydex");
        let caught = self.dex_caught();
        let entries: Vec<crate::moddex::Entry> = crate::moddex::entries().into_iter().filter(|e| !e.unobtainable).collect();
        let total = entries.len();
        let have = entries.iter().filter(|e| self.species.get(e.dex as usize - 1).is_some_and(|s| caught.contains_key(&s.slug))).count();
        let next: Vec<usize> = entries.iter().map(|e| e.dex as usize - 1).filter(|i| self.species.get(*i).is_some_and(|s| !caught.contains_key(&s.slug))).take(24).collect();
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, mut go) = if small { (b.inner(), false) } else { fit::head(ui, &b, "💎", kit::C_PINK, "Shiny Dex", Some("Open ›")) };
        let p = ui.painter_at(b.r);
        let f = if total > 0 { have as f32 / total as f32 } else { 0.0 };
        // the ring and the count; the rest of the room for the next ones
        let wide = body.width() > body.height() * 1.6;
        let (top, rest) = if b.shape() == Shape::Mini {
            (body, Rect::NOTHING)
        } else if wide {
            fit::split_left(body, (body.height() * 2.2).min(body.width() * 0.5), b.px(12.0))
        } else {
            fit::split_top(body, (body.height() * 0.48).clamp(b.px(60.0), b.px(160.0)), b.px(10.0))
        };
        let ring_side = top.height().min(top.width() * 0.45);
        let (ring_r, words) = if b.shape() == Shape::Mini { fit::split_top(top, top.height() * 0.66, b.px(4.0)) } else { fit::split_left(top, ring_side, b.px(12.0)) };
        let rad = (ring_r.width().min(ring_r.height()) / 2.0 - b.px(4.0)).max(8.0);
        let c = ring_r.center();
        fit::ring(&p, c, rad, (rad * 0.22).max(3.0), kit::grow(ui.ctx(), egui::Id::new(("w-dexring", b.r.min.x as i32, b.r.min.y as i32)), f), kit::C_PINK);
        p.text(c, Align2::CENTER_CENTER, format!("{:.0}%", f * 100.0), b.bold((rad / b.s() * 0.36).clamp(10.0, 24.0)), TEXT());
        let fl = fit::vflow(words, &[(b.px(18.0), b.px(36.0)), (b.px(13.0), b.px(15.0))], b.px(2.0));
        let al = if b.shape() == Shape::Mini { Align2::CENTER_CENTER } else { Align2::LEFT_CENTER };
        fit::hero(&p, fl[0], &format!("{} / {}", n(have as u64), n(total as u64)), fl[0].height(), TEXT(), al);
        fit::label(&p, fl[1], "shinies caught", b.font(11.5), MUTED(), al);
        // still to get, as many as fit
        if rest.is_positive() && !next.is_empty() {
            let (lab, grid) = if rest.height() > b.px(70.0) { fit::split_top(rest, b.px(15.0), b.px(4.0)) } else { (Rect::NOTHING, rest) };
            if lab.is_positive() {
                fit::label(&p, lab, "Still to get:", b.font(11.5), MUTED(), Align2::LEFT_CENTER);
            }
            for (k, r) in fit::tiles(grid, next.len(), 1.0, b.px(36.0), b.px(36.0), b.px(6.0)).iter().enumerate() {
                let sp = self.species[next[k]].clone();
                let resp = ui.interact(*r, ui.id().with(("dex-next", k, b.r.min.x as i32, b.r.min.y as i32)), Sense::hover());
                p.rect_filled(*r, b.px(10.0), theme::blend(CARD(), kit::C_PINK, 0.08));
                let side = r.width().min(r.height()) * 0.92;
                self.draw_sprite_centered(&p, &sp, true, r.center(), vec2(side, side));
                resp.on_hover_text(sp.name);
            }
        }
        if small && ui.interact(b.r, ui.id().with(("dex-open", b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).on_hover_text("Open your Shiny Dex").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            go = true;
        }
        if go {
            self.setup.tab = Tab::Pokedex;
            self.setup.pokedex.shiny = true;
            self.setup.pokedex.open = None;
            self.setup.dex.open = None;
            self.setup.scroll_top = true;
        }
    }

    /// How your hunt's luck compares: a dial of the share of hunters who'd
    /// have found it by now, as big as the box allows.
    pub(super) fn w_luck(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "luck");
        let (count, _) = self.odds_count();
        let odds = self.cfg.odds_one_in();
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, _) = if small { (b.inner(), false) } else { fit::head(ui, &b, "🍀", kit::C_GREEN, "Luck meter", None) };
        let p = ui.painter_at(b.r);
        let Some(o) = odds.filter(|o| *o > 1.0) else {
            fit::empty(&p, &b, body, "🍀", "Pick a hunting method with known odds and your luck shows here.");
            return;
        };
        let f = found_by(count, o);
        let col = super::charts::luck_col(f);
        let line = if count == 0 {
            "Start hunting and the needle moves.".to_string()
        } else if f < 0.5 {
            format!("of hunters would have found it by now. Right on track at {}.", n(count))
        } else {
            "of hunters would have found it by now. Unlucky so far; it can come any time.".to_string()
        };
        // the dial beside the words when it's flat, above them otherwise
        let side_by = body.width() > body.height() * 2.6;
        let (dial_r, words) = if side_by {
            fit::split_left(body, body.height() * 2.0, b.px(12.0))
        } else {
            let text_h = if body.height() > b.px(120.0) { b.px(44.0) } else { b.px(24.0) };
            fit::split_bottom(body, text_h, b.px(6.0))
        };
        // a half-dial (twice as wide as it is tall)
        let rad = (dial_r.width() / 2.0).min(dial_r.height() - b.px(4.0)).max(10.0);
        let c = pos2(dial_r.center().x, dial_r.center().y + rad / 2.0);
        let thick = (rad * 0.16).clamp(4.0, 18.0);
        let steps = 48;
        for i in 0..steps {
            let a0 = std::f32::consts::PI * (1.0 - i as f32 / steps as f32);
            let a1 = std::f32::consts::PI * (1.0 - (i + 1) as f32 / steps as f32);
            let seg = super::charts::luck_col(i as f64 / steps as f64);
            p.line_segment([c + vec2(a0.cos(), -a0.sin()) * rad, c + vec2(a1.cos(), -a1.sin()) * rad], Stroke::new(thick, seg.gamma_multiply(0.75)));
        }
        let g = kit::grow(ui.ctx(), egui::Id::new(("w-luck", b.r.min.x as i32, b.r.min.y as i32)), f as f32);
        let a = std::f32::consts::PI * (1.0 - g);
        p.line_segment([c, c + vec2(a.cos(), -a.sin()) * (rad - thick * 1.4)], Stroke::new((rad * 0.035).max(2.0), TEXT()));
        p.circle_filled(c, (rad * 0.06).max(3.0), TEXT());
        let pct = format!("{:.0}%", f * 100.0);
        if words.height() > b.px(36.0) {
            let fl = fit::vflow(words, &[(b.px(20.0), b.px(30.0)), (b.px(13.0), b.px(30.0))], b.px(2.0));
            let al = if side_by { Align2::LEFT_CENTER } else { Align2::CENTER_CENTER };
            fit::hero(&p, fl[0], &pct, fl[0].height(), super::charts::ink(col), al);
            let g = p.layout(line.clone(), b.font(11.5), MUTED(), fl[1].width());
            let at = if side_by { fl[1].min } else { pos2(fl[1].center().x - g.size().x / 2.0, fl[1].min.y) };
            p.with_clip_rect(fl[1]).galley(at, g, MUTED());
        } else {
            fit::hero(&p, words, &pct, words.height(), super::charts::ink(col), Align2::CENTER_CENTER);
        }
    }
}
