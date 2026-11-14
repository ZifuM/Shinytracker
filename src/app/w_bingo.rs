//! Dashboard widgets for Shiny Bingo.

use super::bingo_app::{countdown, secs_to_next_card, GOLD, VIOLET};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_build::{Head, Lead, Pic, Row};
use super::w_fit::{self as fit, wbox, Shape};
use super::{App, Tab};
use crate::bingo::{self, Goal};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense};

impl App {
    fn bingo_head(icon: &'static str, title: &'static str) -> Head<'static> {
        Head { icon, col: VIOLET, title, link: Some("Open ›") }
    }

    /// The card itself, as big as the box allows (it stays square).
    pub(super) fn w_bingo_card(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "bingo_card");
        let card = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let lines = f.lines_done(&card).len();
        let (body, go) = fit::head(ui, &b, "🎯", VIOLET, "Shiny Bingo", Some("Open ›"));
        // the note under the board when there's a strip of room for it
        let side = body.width().min(body.height());
        let (board_room, note_r) = if body.height() - side > b.px(14.0) || body.height() > b.px(200.0) { fit::split_bottom(body, b.px(15.0), b.px(6.0)) } else { (body, Rect::NOTHING) };
        let side = board_room.width().min(board_room.height());
        let board = Rect::from_center_size(board_room.center(), vec2(side, side));
        // the board draws itself in a small child sized to it
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(board).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(board.expand(2.0).intersect(ui.clip_rect()));
        kit::set_card_fill(Some(side + 64.0));
        self.bingo_grid(&mut child, true, None);
        kit::set_card_fill(None);
        if note_r.is_positive() {
            let p = ui.painter_at(b.r);
            let l = if f.rewards_waiting > 0 { "daily reward ready!".to_string() } else if lines == 0 { "no lines yet".to_string() } else { format!("{lines} line{}", if lines == 1 { "" } else { "s" }) };
            fit::label(&p, note_r, &format!("{} / 25 · {l} · new card in {}", f.count(&card), countdown(secs_to_next_card())), b.font(11.0), MUTED(), Align2::CENTER_CENTER);
        }
        if go {
            self.go_to(Tab::Bingo);
        }
    }

    /// Squares, lines and the countdown.
    pub(super) fn w_bingo_week(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "bingo_week");
        let card = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let lines = f.lines_done(&card).len();
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, mut go) = if small { (b.inner(), false) } else { fit::head(ui, &b, "📅", VIOLET, "Bingo today", Some("Open ›")) };
        let p = ui.painter_at(b.r);
        let with_note = body.height() > b.px(70.0);
        let mut want = vec![(b.px(22.0), b.px(64.0)), (b.px(6.0), b.px(12.0))];
        if with_note {
            want.push((b.px(13.0), b.px(15.0)));
        }
        let fl = fit::vflow(body, &want, b.px(6.0));
        let at = fit::hero(&p, Rect::from_min_max(fl[0].min, pos2(fl[0].max.x - b.px(90.0), fl[0].max.y)), &f.count(&card).to_string(), fl[0].height().min(b.px(56.0)), TEXT(), Align2::LEFT_CENTER);
        p.text(pos2(at.max.x + b.px(6.0), at.max.y - at.height() * 0.18), Align2::LEFT_BOTTOM, "/ 25 squares", b.font(12.0), MUTED());
        let seg = (fl[1].width() - 11.0 * b.px(4.0)) / 12.0;
        for k in 0..12 {
            let sr = Rect::from_min_size(pos2(fl[1].min.x + k as f32 * (seg + b.px(4.0)), fl[1].min.y), vec2(seg, fl[1].height()));
            p.rect_filled(sr, fl[1].height() / 2.0, if k < lines { GOLD } else { Color32::from_rgb(38, 38, 54) });
        }
        if with_note {
            let l = if f.blackout { "Blackout!".to_string() } else { format!("{lines} of 12 lines") };
            fit::label(&p, fl[2], &format!("{l} · new card in {}", countdown(secs_to_next_card())), b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
        if small && ui.interact(b.r, ui.id().with(("bingo-week", b.r.min.x as i32, b.r.min.y as i32)), Sense::click()).on_hover_text("Open Shiny Bingo").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            go = true;
        }
        if go {
            self.go_to(Tab::Bingo);
        }
    }

    /// The lines nearest to done, and what each still needs.
    pub(super) fn w_bingo_next(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "bingo_next");
        let card = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let close = if f.blackout { Vec::new() } else { f.closest(&card) };
        let rows: Vec<Row> = close
            .iter()
            .map(|(l, left)| {
                let what: Vec<String> = left.iter().filter_map(|s| card[*s].map(|g| g.text())).collect();
                Row::new(bingo::line_name(*l)).sub(what.join(" · ")).right(format!("{} to go", left.len()), Some(GOLD))
            })
            .collect();
        let c = self.f_list(ui, &b, Self::bingo_head("📌", "Closest to a line"), &rows, "Every square is done today. A new card comes at the daily reset!");
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::Bingo);
        }
    }

    /// Today's Pokémon to meet, with a tick on the ones you have.
    pub(super) fn w_bingo_meet(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "bingo_meet");
        let card = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let pics: Vec<Pic> = card
            .iter()
            .enumerate()
            .filter_map(|(i, g)| if let Some(Goal::Meet(d)) = g { Some((i, *d)) } else { None })
            .map(|(i, d)| {
                let done = f.done(&card, i);
                let lead = match self.species.iter().position(|s| s.dex == d) {
                    Some(si) if self.sprites.get(&self.species[si], false).is_some() => Lead::Species(si, false),
                    _ => Lead::Badge(bingo::meet_name(d).chars().take(3).collect::<String>().to_uppercase(), VIOLET),
                };
                Pic { lead, name: bingo::meet_name(d).to_string(), sub: if done { "Met ✔".into() } else { "To meet".into() }, tip: format!("{}{}", bingo::meet_name(d), if done { ": met today" } else { ": not met yet (the Map shows where it lives)" }), dim: !done }
            })
            .collect();
        let c = self.f_pics(ui, &b, Self::bingo_head("🔍", "Bingo Pokémon"), &pics, "No Pokémon to meet on today's card.", 52.0);
        if c.link || c.item.is_some() {
            self.go_to(Tab::Bingo);
        }
    }

    /// The latest squares ticked.
    pub(super) fn w_bingo_recent(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "bingo_recent");
        let _ = self.bingo_card();
        let f = self.setup.bingo.file().clone();
        let now = crate::stats::now_unix();
        let rows: Vec<Row> = f
            .recent
            .iter()
            .map(|(t, what)| {
                let s = now.saturating_sub(*t);
                let ago = if s < 3600 { format!("{}m", (s / 60).max(1)) } else if s < 86_400 { format!("{}h", s / 3600) } else { format!("{}d", s / 86_400) };
                Row::new(what.clone()).lead(Lead::Icon("✔".into(), GOLD)).right(ago, Some(MUTED()))
            })
            .collect();
        let c = self.f_list(ui, &b, Self::bingo_head("✅", "Just ticked"), &rows, "Nothing ticked on today's card yet. Go meet some Pokémon!");
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::Bingo);
        }
    }
}
