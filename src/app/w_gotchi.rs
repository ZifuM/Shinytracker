//! Dashboard widgets for Pokégotchi.

use super::gotchi_app::{act_icon, meter, mood_col, need_col, need_tint, paint_device, Act};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_fit::{self as fit, wbox, Shape, WBox};
use super::{App, Tab};
use crate::gotchi::{self, Need};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Rect, Sense, Stroke};
use std::time::Instant;

const PINK: egui::Color32 = egui::Color32::from_rgb(255, 156, 199);

impl App {
    /// The box, and the room under the heading; None (and a placeholder
    /// drawn) when there's no pet yet.
    fn gotchi_wbox(&mut self, ui: &mut egui::Ui, kind: &str, icon: &str, col: egui::Color32, title: &str, heading: bool) -> (WBox, Option<Rect>, bool) {
        let b = wbox(ui, kind);
        let (body, go) = if heading { fit::head(ui, &b, icon, col, title, Some("Open ›")) } else { (b.inner(), false) };
        if self.setup.gotchi.file().partner().is_none() {
            let p = ui.painter_at(b.r);
            fit::empty(&p, &b, body, "🐾", "Adopt a shiny you caught in Pokégotchi and it shows here.");
            return (b, None, go);
        }
        (b, Some(body), go)
    }

    /// Your pet on its device, with its name, friendship and mood beside
    /// (or under) it.
    pub(super) fn w_gotchi_pet(&mut self, ui: &mut egui::Ui) {
        let mini = matches!(super::w_fit::peek_shape(), Some(Shape::Mini));
        let (b, body, mut go) = self.gotchi_wbox(ui, "gotchi_pet", "🐾", PINK, "Your Pokégotchi", !mini);
        let Some(body) = body else {
            if go {
                self.go_to(Tab::Gotchi);
            }
            return;
        };
        let f = self.setup.gotchi.file().clone();
        let Some(pet) = f.partner().cloned() else { return };
        let mood = pet.mood(f.asleep);
        let name = self.gotchi_name(&pet);
        let t = ui.input(|i| i.time) as f32;
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
        let resp = ui.interact(b.r, ui.id().with(("gotchi-pet", b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
        let p = ui.painter_at(b.r);
        // the device keeps its shape (236 × 290)
        let tall = b.shape() == Shape::Column || mini || body.width() < body.height() * 1.3;
        let (dev_room, text) = if mini {
            (body, Rect::NOTHING)
        } else if tall {
            fit::split_top(body, body.height() * 0.62, b.px(8.0))
        } else {
            fit::split_left(body, (body.height() * 236.0 / 290.0).min(body.width() * 0.5), b.px(14.0))
        };
        let k = (dev_room.width() / 236.0).min(dev_room.height() / 290.0);
        let dev = Rect::from_center_size(dev_room.center(), vec2(236.0 * k, 290.0 * k));
        let parts = paint_device(&p, dev, f.frame, t, None, None, false);
        self.gotchi_screen(&p, parts.screen, f.frame, t, true);
        if text.is_positive() {
            let mut want = vec![(b.px(18.0), b.px(22.0)), (b.px(14.0), b.px(16.0)), (b.px(16.0), b.px(18.0))];
            let with_hint = text.height() > b.px(90.0);
            if with_hint {
                want.push((b.px(28.0), b.px(44.0)));
            }
            let fl = fit::vflow(text, &want, b.px(3.0));
            fit::label(&p, fl[0], &name, b.bold(16.0), TEXT(), if tall { Align2::CENTER_CENTER } else { Align2::LEFT_CENTER });
            fit::label(&p, fl[1], &format!("Friendship Lv {}", pet.level()), b.font(12.0), kit::C_GOLD, if tall { Align2::CENTER_CENTER } else { Align2::LEFT_CENTER });
            let mr = fl[2];
            let g = fit::fit(&p, mood.name(), b.bold(13.0), 0.8, mr.width() - b.px(16.0), TEXT());
            let x0 = if tall { mr.center().x - (g.size().x + b.px(14.0)) / 2.0 } else { mr.min.x };
            p.circle_filled(pos2(x0 + b.px(5.0), mr.center().y), b.px(4.5), mood_col(mood));
            p.galley(pos2(x0 + b.px(14.0), mr.center().y - g.size().y / 2.0), g, TEXT());
            if with_hint {
                fit::note(&p, fl[3], mood.hint(), b.font(11.5));
            }
        }
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            go = true;
        }
        if go {
            self.go_to(Tab::Gotchi);
        }
    }

    /// The four needs as bars filling the box.
    pub(super) fn w_gotchi_needs(&mut self, ui: &mut egui::Ui) {
        let mini = matches!(super::w_fit::peek_shape(), Some(Shape::Mini));
        let (b, body, go) = self.gotchi_wbox(ui, "gotchi_needs", "💗", PINK, "Pet needs", !mini);
        let Some(body) = body else {
            if go {
                self.go_to(Tab::Gotchi);
            }
            return;
        };
        let f = self.setup.gotchi.file().clone();
        let Some(pet) = f.partner().cloned() else { return };
        let p = ui.painter_at(b.r);
        // side by side when it's flat, else one under another
        let parts = if b.flat() || body.width() > body.height() * 3.0 { fit::cols(body, 4, b.px(10.0)) } else { fit::stack(body, 4, b.px(6.0)) };
        let side = b.flat() || body.width() > body.height() * 3.0;
        for (n, r) in Need::ALL.iter().zip(parts.iter()) {
            let v = pet.need(*n);
            if side {
                let fl = fit::vflow(*r, &[(b.px(14.0), b.px(16.0)), (b.px(6.0), b.px(8.0))], b.px(4.0));
                fit::label(&p, fl[0], n.name(), b.bold(11.5), TEXT(), Align2::LEFT_CENTER);
                fit::label(&p, fl[0], &format!("{v:.0}"), b.font(11.0), MUTED(), Align2::RIGHT_CENTER);
                meter(&p, fl[1], v / 100.0, need_col(*n), kit::sunken());
            } else {
                let (lab, rest) = fit::split_left(*r, (r.width() * 0.3).clamp(b.px(46.0), b.px(80.0)), b.px(8.0));
                let (bar, val) = fit::split_right(rest, b.px(28.0), b.px(6.0));
                fit::label(&p, lab, n.name(), b.bold(12.0), TEXT(), Align2::LEFT_CENTER);
                let bh = (r.height() * 0.32).clamp(b.px(6.0), b.px(14.0));
                meter(&p, Rect::from_center_size(bar.center(), vec2(bar.width(), bh)), v / 100.0, need_col(*n), kit::sunken());
                fit::label(&p, val, &format!("{v:.0}"), b.font(12.0), MUTED(), Align2::RIGHT_CENTER);
            }
        }
        if go {
            self.go_to(Tab::Gotchi);
        }
    }

    /// Feed, treat, clean and pat, as buttons filling the box.
    pub(super) fn w_gotchi_care(&mut self, ui: &mut egui::Ui) {
        let (b, body, go) = self.gotchi_wbox(ui, "gotchi_care", "🍓", PINK, "Quick care", !matches!(super::w_fit::peek_shape(), Some(Shape::Mini)));
        let Some(body) = body else {
            if go {
                self.go_to(Tab::Gotchi);
            }
            return;
        };
        let f = self.setup.gotchi.file().clone();
        let said = self.setup.gotchi.said.as_ref().filter(|s| s.1.elapsed().as_secs_f32() < 2.6).map(|s| s.0.clone());
        let (body, note_r) = if body.height() > b.px(110.0) { fit::split_bottom(body, b.px(15.0), b.px(6.0)) } else { (body, Rect::NOTHING) };
        let acts = [Act::Feed, Act::Treat, Act::Clean, Act::Pat];
        let cells = fit::tiles(body, 4, 1.1, b.px(46.0), b.px(40.0), b.px(8.0));
        let mut act = None;
        for (i, (a, br)) in acts.iter().zip(cells.iter()).enumerate() {
            let resp = ui.interact(*br, ui.id().with(("w-gotchi-care", i, b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let p = ui.painter_at(*br);
            p.rect_filled(*br, b.px(10.0), theme::blend(kit::sunken(), need_tint(*a), 0.08 + 0.14 * ht));
            p.rect_stroke(*br, b.px(10.0), Stroke::new(1.0f32, theme::blend(kit::LINE(), need_tint(*a), 0.3 * ht)));
            let label = match a {
                Act::Feed => format!("Feed {}", f.berries),
                Act::Treat => format!("Treat {}", f.treats),
                _ => a.name().to_string(),
            };
            let with_label = br.height() > b.px(52.0);
            let (ic, lab) = if with_label { fit::split_bottom(br.shrink(b.px(4.0)), b.px(15.0), b.px(2.0)) } else { (br.shrink(b.px(4.0)), Rect::NOTHING) };
            act_icon(&p, ic.center(), (ic.height().min(ic.width()) * 0.42).clamp(8.0, 30.0), *a, need_tint(*a));
            if with_label {
                fit::label(&p, lab, &label, b.bold(11.0), TEXT(), Align2::CENTER_CENTER);
            }
            let tip = match a {
                Act::Feed => "Feed a berry",
                Act::Treat => "Give a treat",
                Act::Clean => "Clean up",
                _ => "Pat",
            };
            if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                act = Some(*a);
            }
        }
        if note_r.is_positive() {
            let p = ui.painter_at(b.r);
            fit::label(&p, note_r, &said.unwrap_or_else(|| format!("{} berries · {} treats", f.berries, f.treats)), b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
        if let Some(a) = act {
            self.gotchi_act(a);
        } else if go {
            self.go_to(Tab::Gotchi);
        }
    }

    /// Berries and treats, and how far to the next berry.
    pub(super) fn w_gotchi_pantry(&mut self, ui: &mut egui::Ui) {
        let (b, body, go) = self.gotchi_wbox(ui, "gotchi_pantry", "🎒", kit::C_GREEN, "Pantry", !matches!(super::w_fit::peek_shape(), Some(Shape::Mini)));
        let Some(body) = body else {
            if go {
                self.go_to(Tab::Gotchi);
            }
            return;
        };
        let f = self.setup.gotchi.file().clone();
        let today = crate::stats::local_day(crate::config::now_unix());
        let td = if f.today.0 == today { f.today } else { (0, 0, 0, 0) };
        let p = ui.painter_at(b.r);
        let left = gotchi::ENC_PER_BERRY - f.enc_part;
        // the two counts take what the berry bar and today's line leave
        let with_today = body.height() > b.px(120.0);
        let mut want = vec![(b.px(36.0), body.height()), (b.px(26.0), b.px(30.0))];
        if with_today {
            want.push((b.px(14.0), b.px(16.0)));
        }
        let fl = fit::vflow(body, &want, b.px(8.0));
        for ((a, n, what), tr) in [(Act::Feed, f.berries, "berries"), (Act::Treat, f.treats, "treats")].iter().zip(fit::cols(fl[0], 2, b.px(10.0)).iter()) {
            p.rect_filled(*tr, b.px(10.0), kit::sunken());
            let ic = (tr.height() * 0.3).clamp(b.px(10.0), b.px(26.0));
            let (icr, txt) = fit::split_left(tr.shrink(b.px(8.0)), ic * 2.2, b.px(6.0));
            act_icon(&p, icr.center(), ic, *a, need_tint(*a));
            let tf = fit::vflow(txt, &[(b.px(16.0), txt.height() * 0.6), (b.px(12.0), b.px(14.0))], b.px(1.0));
            fit::hero(&p, tf[0], &n.to_string(), tf[0].height(), TEXT(), Align2::LEFT_CENTER);
            fit::label(&p, tf[1], what, b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
        let (lab, bar) = fit::split_top(fl[1], fl[1].height() - b.px(8.0), b.px(2.0));
        fit::label(&p, lab, "Next berry", b.font(12.0), TEXT(), Align2::LEFT_CENTER);
        fit::label(&p, lab, &format!("{left} to go"), b.font(12.0), MUTED(), Align2::RIGHT_CENTER);
        fit::meter(&p, bar, f.enc_part as f32 / gotchi::ENC_PER_BERRY as f32, kit::C_GREEN);
        if with_today {
            fit::label(&p, fl[2], &format!("Today: +{} berr{} · +{} treat{}", td.1, if td.1 == 1 { "y" } else { "ies" }, td.2, if td.2 == 1 { "" } else { "s" }), b.font(11.0), MUTED(), Align2::LEFT_CENTER);
        }
        if go {
            self.go_to(Tab::Gotchi);
        }
    }

    /// The pets in the Box, as big as the box allows; click one to bring it
    /// out.
    pub(super) fn w_gotchi_box(&mut self, ui: &mut egui::Ui) {
        let (b, body, go) = self.gotchi_wbox(ui, "gotchi_box", "📦", PINK, "The Box", true);
        let Some(body) = body else {
            if go {
                self.go_to(Tab::Gotchi);
            }
            return;
        };
        let f = self.setup.gotchi.file().clone();
        let mut swap = None;
        let cells = fit::tiles(body, gotchi::MAX_BOX, 0.8, b.px(56.0), b.px(56.0), b.px(8.0));
        for (i, r) in cells.iter().enumerate() {
            let p = ui.painter_at(*r);
            let Some(pet) = f.pets.get(i) else {
                p.rect_stroke(r.shrink(0.5), b.px(12.0), Stroke::new(1.0f32, theme::blend(kit::LINE(), PINK, 0.12)));
                continue;
            };
            let out = i == f.partner;
            let resp = ui.interact(*r, ui.id().with(("w-gotchi-box", i, b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            p.rect_filled(*r, b.px(12.0), theme::blend(kit::sunken(), PINK, if out { 0.16 } else { 0.04 * ht }));
            if out {
                p.rect_stroke(r.shrink(0.5), b.px(12.0), Stroke::new(1.0f32, PINK.gamma_multiply(0.6)));
            }
            let with_text = r.height() > b.px(84.0);
            let (pic, txt) = if with_text { fit::split_bottom(r.shrink(b.px(6.0)), b.px(30.0), 0.0) } else { (r.shrink(b.px(6.0)), Rect::NOTHING) };
            if let Some(si) = self.gotchi_species(&pet.slug) {
                let sp = self.species[si].clone();
                let side = pic.width().min(pic.height());
                self.draw_sprite_centered(&p, &sp, true, pic.center(), vec2(side, side));
            }
            if with_text {
                let name = self.gotchi_name(pet);
                let (n_r, s_r) = fit::split_top(txt, b.px(16.0), 0.0);
                fit::label(&p, n_r, &name, b.bold(11.5), TEXT(), Align2::CENTER_CENTER);
                fit::label(&p, s_r, &if out { "Out".to_string() } else { format!("Lv {}", pet.level()) }, b.font(10.5), if out { PINK } else { MUTED() }, Align2::CENTER_CENTER);
            }
            if resp.on_hover_text(if out { "Out with you now" } else { "Bring it out" }).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !out {
                swap = Some(i);
            }
        }
        if let Some(i) = swap {
            self.setup.gotchi.file().swap(i);
            self.setup.gotchi.game = None;
            self.setup.gotchi.say("Hi again!");
            self.setup.gotchi.hop = Some(Instant::now());
            self.setup.gotchi.touch();
        } else if go {
            self.go_to(Tab::Gotchi);
        }
    }
}
