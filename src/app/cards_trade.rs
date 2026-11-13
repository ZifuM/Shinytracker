//! The Card Collector's Trade tab: your Trainer ID, sending a card as a
//! one-use code made for a friend's Trainer ID, claiming a code, and the
//! list of trades. The record behind it lives in `crate::trade`.

use super::cards_app::CARD_ASPECT;
use super::store::pill_button;
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_kit::{head, note};
use super::App;
use crate::cards::{self, Card};
use crate::trade::{self, Book, Entry, Gift};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::Instant;

/// The Trade tab's own state.
#[derive(Default)]
pub(super) struct TradeUi {
    /// The card picked to send.
    pub pick: Option<String>,
    /// The friend's Trainer ID as typed.
    pub to: String,
    /// "Send my only one?" asked and waiting.
    pub confirm_last: bool,
    /// The code just made: (code, card, to).
    pub made: Option<(String, String, u64)>,
    /// The code being typed to claim.
    pub code: String,
    /// What claiming said: Ok((card, from)) or the reason it didn't.
    pub claimed: Option<(Result<(String, u64), String>, Instant)>,
    /// The card picker is open, and its search.
    pub picker: bool,
    pub search: String,
}

/// The send and claim cards are the same height (side by side they line up;
/// each one's contents fit in it).
const CARD_H: f32 = 376.0;

fn now() -> u64 {
    crate::config::now_unix()
}

impl App {
    /// The trade record, read (and checked against the card save) the
    /// first time it's needed. See `crate::trade` for how it's protected.
    pub(super) fn cards_book(&mut self) -> &mut Book {
        if self.setup.cards.book.is_none() {
            let found = trade::load();
            let f = self.setup.cards.file();
            let mut book = found.book.unwrap_or_else(|| Book::new(if f.trainer != 0 { f.trainer } else { trade::new_trainer_id() }));
            if book.me == 0 {
                book.me = trade::new_trainer_id();
            }
            f.trainer = book.me;
            let n = book.log.len() as u32;
            let seen = f.trades_seen;
            if n > seen {
                // the save is older than the record (a backup was put
                // back): replay what happened since
                for e in book.log[seen as usize..].iter() {
                    match e {
                        Entry::Sent { card, .. } => take_card(f, card),
                        Entry::Got { card, .. } => give_card(f, card),
                    }
                }
            } else if n < seen {
                // the record went missing or was rolled back: cards that came
                // from trades go (sent ones stay gone)
                let lost: u32 = f.traded_in.values().sum();
                let traded: Vec<(String, u32)> = std::mem::take(&mut f.traded_in).into_iter().collect();
                for (id, k) in traded {
                    if let Some(c) = f.owned.get_mut(&id) {
                        *c = c.saturating_sub(k);
                    }
                }
                f.owned.retain(|_, c| *c > 0);
                self.setup.cards.load_error = Some(if lost > 0 {
                    format!("Your trade record was deleted or changed, so the {lost} card{} you got from trades {} removed.", if lost == 1 { "" } else { "s" }, if lost == 1 { "was" } else { "were" })
                } else {
                    "Your trade record was deleted or changed; it has been started again.".to_string()
                });
            }
            let f = self.setup.cards.file();
            f.trades_seen = n;
            self.setup.cards.touch();
            // every copy back in step (repairs a missing or damaged one)
            let _ = trade::save(&book);
            self.setup.cards.book = Some(book);
        }
        self.setup.cards.book.as_mut().expect("loaded above")
    }

    /// Send a card: a code for `to`, and the card leaves the binder.
    fn cards_send(&mut self, card: &str, to: u64) -> Result<String, String> {
        if self.setup.cards.file().count(card) == 0 {
            return Err("That card isn't in your binder any more.".into());
        }
        let book = self.cards_book();
        if to == book.me {
            return Err("That's your own Trainer ID. Ask your friend for theirs.".into());
        }
        let serial = book.serial + 1;
        let code = trade::make_code(&Gift { card: card.to_string(), from: book.me, to, serial }).ok_or("That card can't be traded.")?;
        let mut next = book.clone();
        next.serial = serial;
        next.log.push(Entry::Sent { card: card.to_string(), to, serial, when: now(), code: code.clone() });
        // the record first: if it can't be saved, nothing happens
        trade::save(&next)?;
        let n = next.log.len() as u32;
        self.setup.cards.book = Some(next);
        let f = self.setup.cards.file();
        take_card(f, card);
        f.trades_seen = n;
        self.setup.cards.touch();
        self.setup.cards.save_tick(true);
        Ok(code)
    }

    /// Claim a code: Ok((card, from)) or why not.
    fn cards_claim(&mut self, code: &str) -> Result<(String, u64), String> {
        let g = trade::read_code(code).map_err(|e| match e {
            trade::CodeError::Garbled => "That isn't a whole trade code. Check it was copied in full.".to_string(),
            trade::CodeError::Tampered => "That code doesn't check out. Copy it again from the message you were sent.".to_string(),
            trade::CodeError::UnknownCard => "That code is for a card this version doesn't have. Update the app and try again.".to_string(),
        })?;
        let book = self.cards_book();
        if g.from == book.me {
            return Err("That's a code you made. Send it to the friend it's for.".into());
        }
        if g.to != book.me {
            return Err(format!("This code isn't for you: it was made for {}.", trade::fmt_id(g.to)));
        }
        if book.has(g.from, g.serial) {
            return Err("This code has already been claimed.".into());
        }
        let mut next = book.clone();
        next.log.push(Entry::Got { card: g.card.clone(), from: g.from, serial: g.serial, when: now() });
        trade::save(&next)?;
        let n = next.log.len() as u32;
        self.setup.cards.book = Some(next);
        let f = self.setup.cards.file();
        give_card(f, &g.card);
        f.first.entry(g.card.clone()).or_insert(now());
        f.trades_seen = n;
        self.setup.cards.touch();
        self.setup.cards.save_tick(true);
        Ok((g.card, g.from))
    }

    pub(super) fn cards_trade(&mut self, ui: &mut egui::Ui) {
        let me = self.cards_book().me;
        let w = ui.available_width();
        let wide = w >= 900.0;
        // ── your Trainer ID
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("YOUR TRAINER ID").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
                    ui.label(RichText::new(trade::fmt_id(me)).font(crate::fonts::sans_bold_font(26.0)).color(TEXT()));
                });
                ui.add_space(16.0);
                let (r, _) = ui.allocate_exact_size(vec2(120.0, 34.0), Sense::hover());
                if pill_button(ui, r, "📋  Copy", true, "trade-copy-id").clicked() {
                    ui.ctx().copy_text(trade::fmt_id(me));
                    self.app_toast("Trainer ID copied".into());
                }
                ui.add_space(16.0);
                note(ui, "Give this to a friend who wants to send you a card. Their code will only work for this ID.");
            });
        });
        ui.add_space(14.0);
        // ── send and claim, side by side when there's room
        if wide {
            let gap = 14.0;
            let cw = ((w - gap) / 2.0).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(cw);
                    self.trade_send_card(ui);
                });
                ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(cw);
                    self.trade_claim_card(ui);
                });
            });
        } else {
            self.trade_send_card(ui);
            ui.add_space(14.0);
            self.trade_claim_card(ui);
        }
        ui.add_space(14.0);
        self.cards_trade_spares(ui);
        ui.add_space(14.0);
        self.trade_history(ui);
        self.trade_picker(ui.ctx());
    }

    fn trade_send_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(CARD_H);
            head(ui, "📤", kit::C_PINK, "Send a card", None);
            ui.add_space(4.0);
            // a code was just made: show it until they start another
            if let Some((code, card, to)) = self.setup.cards.trade.made.clone() {
                let name = cards::by_id(&card).map_or(card.clone(), |c| c.name.clone());
                note(ui, format!("{name} has left your binder. Send this code to {}. It works once, only for them.", trade::fmt_id(to)));
                ui.add_space(10.0);
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 54.0), Sense::hover());
                ui.painter().rect_filled(r, 12.0, kit::sunken());
                ui.painter().rect_stroke(r, 12.0, Stroke::new(1.0f32, kit::C_PINK.gamma_multiply(0.6)));
                ui.painter().text(r.center(), Align2::CENTER_CENTER, &code, egui::FontId::monospace(15.0), TEXT());
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let (r1, _) = ui.allocate_exact_size(vec2(150.0, 34.0), Sense::hover());
                    if pill_button(ui, r1, "📋  Copy code", true, "trade-copy-code").clicked() {
                        ui.ctx().copy_text(code.clone());
                        self.app_toast("Trade code copied".into());
                    }
                    let (r2, _) = ui.allocate_exact_size(vec2(170.0, 34.0), Sense::hover());
                    if pill_button(ui, r2, "Send another card", false, "trade-another").clicked() {
                        let t = &mut self.setup.cards.trade;
                        t.made = None;
                        t.pick = None;
                        t.confirm_last = false;
                    }
                });
                ui.add_space(6.0);
                note(ui, "Lost it? It stays in your trade history below, ready to copy again.");
                return;
            }
            note(ui, "Pick a card, type your friend's Trainer ID, and you'll get a code to send them. The card leaves your binder as soon as the code is made.");
            ui.add_space(12.0);
            // 1. the card
            let pick = self.setup.cards.trade.pick.clone().and_then(|id| cards::by_id(&id));
            ui.horizontal(|ui| {
                let cw = 74.0;
                let (r, resp) = ui.allocate_exact_size(vec2(cw, cw * CARD_ASPECT), Sense::click());
                match pick {
                    Some(c) => self.draw_card(ui, r, c, false, false),
                    None => {
                        ui.painter().rect_filled(r, 8.0, kit::sunken());
                        ui.painter().extend(egui::Shape::dashed_line(&[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()], Stroke::new(1.2f32, MUTED()), 5.0, 4.0));
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, "+", crate::fonts::sans_bold_font(26.0), MUTED());
                    }
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.setup.cards.trade.picker = true;
                }
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    match pick {
                        Some(c) => {
                            let n = self.setup.cards.file().count(&c.id);
                            ui.label(RichText::new(&c.name).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                            ui.label(RichText::new(format!("{} · #{} · {} {}", c.set_name(), c.num, c.rarity.symbol(), c.rarity.name())).size(kit::fs(12.0)).color(MUTED()));
                            ui.label(RichText::new(if n > 1 { format!("You have {n}") } else { "Your only one".to_string() }).size(kit::fs(12.0)).color(if n > 1 { MUTED() } else { Color32::from_rgb(250, 190, 90) }));
                        }
                        None => {
                            ui.label(RichText::new("No card picked").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                            ui.label(RichText::new("Click the box to choose one from your binder").size(kit::fs(12.0)).color(MUTED()));
                        }
                    }
                    ui.add_space(4.0);
                    if super::w_kit::link(ui, if pick.is_some() { "Choose another ›" } else { "Choose a card ›" }) {
                        self.setup.cards.trade.picker = true;
                    }
                });
            });
            ui.add_space(14.0);
            // 2. who it's for
            ui.label(RichText::new("YOUR FRIEND'S TRAINER ID").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
            ui.add_space(4.0);
            let typed = &mut self.setup.cards.trade.to;
            ui.add(egui::TextEdit::singleline(typed).hint_text("TR-XXXX-XXXX").desired_width(220.0).font(egui::FontId::monospace(15.0)));
            let to = trade::parse_id(typed);
            let bad_id = !typed.trim().is_empty() && to.is_none();
            if bad_id {
                ui.label(RichText::new("That doesn't look like a Trainer ID (TR- and 8 letters or numbers).").size(kit::fs(11.5)).color(Color32::from_rgb(250, 150, 140)));
            }
            ui.add_space(14.0);
            // 3. make the code
            let last = pick.is_some_and(|c| self.setup.cards.file().count(&c.id) == 1);
            let ready = pick.is_some() && to.is_some();
            let confirm = self.setup.cards.trade.confirm_last;
            let label = if confirm { "Yes, trade my only one" } else { "📤  Make trade code" };
            let (r, _) = ui.allocate_exact_size(vec2(220.0, 36.0), Sense::hover());
            if pill_button(ui, r, label, ready, "trade-make").clicked() && ready {
                if last && !confirm {
                    self.setup.cards.trade.confirm_last = true;
                } else if let (Some(c), Some(to)) = (pick, to) {
                    let id = c.id.clone();
                    match self.cards_send(&id, to) {
                        Ok(code) => {
                            ui.ctx().copy_text(code.clone());
                            self.app_toast("Trade code made and copied".into());
                            let t = &mut self.setup.cards.trade;
                            t.made = Some((code, id, to));
                            t.confirm_last = false;
                        }
                        Err(e) => self.app_toast(e),
                    }
                }
            }
            if confirm {
                ui.add_space(4.0);
                ui.label(RichText::new("It's your only one: once the code is made it leaves your binder.").size(kit::fs(12.0)).color(Color32::from_rgb(250, 190, 90)));
                if super::w_kit::link(ui, "Keep it") {
                    self.setup.cards.trade.confirm_last = false;
                }
            }
        });
    }

    fn trade_claim_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(CARD_H);
            head(ui, "📥", kit::C_GREEN, "Claim a code", None);
            ui.add_space(4.0);
            note(ui, "Paste a code a friend sent you. Codes work once, and only for the Trainer ID they were made for.");
            ui.add_space(12.0);
            let typed = &mut self.setup.cards.trade.code;
            ui.add(egui::TextEdit::singleline(typed).hint_text("PTC-XXXX-XXXX-…").desired_width(ui.available_width()).font(egui::FontId::monospace(14.0)));
            ui.add_space(10.0);
            let has = !typed.trim().is_empty();
            let (r, _) = ui.allocate_exact_size(vec2(160.0, 36.0), Sense::hover());
            if pill_button(ui, r, "📥  Claim", has, "trade-claim").clicked() && has {
                let code = self.setup.cards.trade.code.clone();
                let res = self.cards_claim(&code);
                if res.is_ok() {
                    self.setup.cards.trade.code.clear();
                }
                self.setup.cards.trade.claimed = Some((res, Instant::now()));
            }
            ui.add_space(12.0);
            match self.setup.cards.trade.claimed.clone() {
                Some((Ok((card, from)), at)) => {
                    let Some(c) = cards::by_id(&card) else { return };
                    // the card flips in
                    let t = at.elapsed().as_secs_f32();
                    let flip = (t / 0.5).min(1.0);
                    let ch = 150.0;
                    let cw = ch / CARD_ASPECT;
                    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), ch), Sense::hover());
                    let sx = (flip * std::f32::consts::PI).cos().abs().max(0.03);
                    let r = Rect::from_center_size(pos2(row.min.x + cw / 2.0, row.center().y), vec2(cw * sx, ch));
                    ui.painter().add(kit::shadow_tier(2).as_shape(r, 8.0));
                    if flip < 0.5 {
                        super::cards_app::card_back(ui.painter(), r);
                    } else {
                        self.draw_card(ui, r, c, true, true);
                    }
                    if flip < 1.0 {
                        ui.ctx().request_repaint();
                    }
                    let tx = row.min.x + cw + 18.0;
                    let p = ui.painter();
                    p.text(pos2(tx, row.min.y + 20.0), Align2::LEFT_CENTER, format!("{} is in your binder!", c.name), crate::fonts::sans_bold_font(16.0), TEXT());
                    p.text(pos2(tx, row.min.y + 44.0), Align2::LEFT_CENTER, format!("From {}", trade::fmt_id(from)), crate::fonts::sans_font(12.5), MUTED());
                    p.text(pos2(tx, row.min.y + 64.0), Align2::LEFT_CENTER, format!("{} · #{} · {} {}", c.set_name(), c.num, c.rarity.symbol(), c.rarity.name()), crate::fonts::sans_font(12.0), MUTED());
                }
                Some((Err(e), _)) => {
                    ui.label(RichText::new(e).size(kit::fs(12.5)).color(Color32::from_rgb(250, 150, 140)));
                }
                None => {}
            }
        });
    }

    fn trade_history(&mut self, ui: &mut egui::Ui) {
        let book = self.cards_book().clone();
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            head(ui, "🔄", kit::C_SLATE, "Your trades", None);
            ui.add_space(4.0);
            if book.log.is_empty() {
                note(ui, "No trades yet.");
            }
            for e in book.log.iter().rev().take(30) {
                let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
                let p = ui.painter();
                let (icon, col, what, who, when, code) = match e {
                    Entry::Sent { card, to, when, code, .. } => ("↗", kit::C_PINK, card, format!("Sent to {}", trade::fmt_id(*to)), *when, Some(code.clone())),
                    Entry::Got { card, from, when, .. } => ("↙", kit::C_GREEN, card, format!("From {}", trade::fmt_id(*from)), *when, None),
                };
                let name = cards::by_id(what).map_or(what.clone(), |c| c.name.clone());
                p.text(pos2(row.min.x + 8.0, row.center().y), Align2::LEFT_CENTER, icon, egui::FontId::proportional(15.0), col);
                p.text(pos2(row.min.x + 30.0, row.center().y), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(13.0), TEXT());
                p.text(pos2(row.min.x + 210.0, row.center().y), Align2::LEFT_CENTER, who, crate::fonts::sans_font(12.5), MUTED());
                p.text(pos2(row.max.x - if code.is_some() { 110.0 } else { 8.0 }, row.center().y), Align2::RIGHT_CENTER, super::home::ago(when), crate::fonts::sans_font(12.0), MUTED());
                if let Some(code) = code {
                    let br = Rect::from_min_size(pos2(row.max.x - 100.0, row.center().y - 13.0), vec2(100.0, 26.0));
                    if pill_button(ui, br, "📋  Code", false, ("trade-hist-copy", when, &code)).on_hover_text(&code).clicked() {
                        ui.ctx().copy_text(code.clone());
                        self.app_toast("Trade code copied".into());
                    }
                }
                ui.painter().line_segment([pos2(row.min.x, row.max.y), pos2(row.max.x, row.max.y)], Stroke::new(1.0f32, kit::LINE().gamma_multiply(0.5)));
            }
            ui.add_space(10.0);
            note(ui, "Your trades are kept in a protected record (three copies that repair each other). If every copy is deleted or changed, cards you got from trades are removed.");
        });
    }

    /// Choosing which card to send: your cards, with a search.
    fn trade_picker(&mut self, ctx: &egui::Context) {
        if !self.setup.cards.trade.picker {
            return;
        }
        let f = self.setup.cards.file().clone();
        let screen = ctx.screen_rect();
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut picked: Option<String> = None;
        egui::Area::new(egui::Id::new("trade-picker")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(170));
            let panel = Rect::from_center_size(sr.center(), vec2((sr.width() - 60.0).min(900.0), (sr.height() - 60.0).min(680.0)));
            let _ = ui.interact(panel, ui.id().with("trade-picker-panel"), Sense::click());
            ui.painter().add(kit::shadow_tier(4).as_shape(panel, 16.0));
            ui.painter().rect_filled(panel, 16.0, kit::CARD());
            ui.painter().rect_stroke(panel, 16.0, Stroke::new(1.0f32, kit::LINE()));
            let inner = panel.shrink2(vec2(22.0, 20.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Choose a card to send").font(crate::fonts::sans_bold_font(18.0)).color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("Close").size(13.0))).clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(8.0);
                kit::search_box_keyed(ui, Some(egui::Id::new("trade-picker-search")), &mut self.setup.cards.trade.search, "Search your cards", 320.0);
                ui.add_space(10.0);
                let q = self.setup.cards.trade.search.trim().to_lowercase();
                let list: Vec<&'static Card> = cards::all().iter().filter(|c| f.count(&c.id) > 0).filter(|c| q.is_empty() || c.name.to_lowercase().contains(&q)).collect();
                if list.is_empty() {
                    note(ui, if f.owned.is_empty() { "Your binder is empty. Open a pack first!" } else { "No cards match." });
                    return;
                }
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let w = ui.available_width();
                    let gap = 10.0;
                    let cw = 92.0;
                    let cols = ((w + gap) / (cw + gap)).floor().max(1.0) as usize;
                    let ch = (cw * CARD_ASPECT).round();
                    for row in list.chunks(cols) {
                        let (rr, _) = ui.allocate_exact_size(vec2(w, ch + 34.0), Sense::hover());
                        if !ui.is_rect_visible(rr) {
                            continue;
                        }
                        for (k, c) in row.iter().enumerate() {
                            let r = Rect::from_min_size(pos2(rr.min.x + k as f32 * (cw + gap), rr.min.y + 4.0), vec2(cw, ch));
                            let resp = ui.interact(r, ui.id().with(("trade-pick", &c.id)), Sense::click());
                            let lift = if resp.hovered() { -4.0 } else { 0.0 };
                            let r2 = r.translate(vec2(0.0, lift));
                            self.draw_card(ui, r2, c, false, false);
                            if resp.hovered() {
                                ui.painter().rect_stroke(r2, 6.0, Stroke::new(2.0f32, kit::primary()));
                            }
                            let n = f.count(&c.id);
                            let p = ui.painter();
                            let name = super::charts::clip(ui, &c.name, &crate::fonts::sans_bold_font(11.5), cw);
                            p.text(pos2(r.min.x, r.max.y + 12.0), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(11.5), TEXT());
                            p.text(pos2(r.min.x, r.max.y + 26.0), Align2::LEFT_CENTER, if n > 1 { format!("×{n}") } else { "only one".to_string() }, crate::fonts::sans_font(10.5), MUTED());
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                picked = Some(c.id.clone());
                            }
                        }
                    }
                });
            });
            if scrim.clicked() && !panel.contains(ui.ctx().pointer_interact_pos().unwrap_or_default()) {
                close = true;
            }
        });
        if let Some(id) = picked {
            let t = &mut self.setup.cards.trade;
            t.pick = Some(id);
            t.confirm_last = false;
            t.picker = false;
        } else if close {
            self.setup.cards.trade.picker = false;
        }
    }
}

/// One fewer of a card (and of the traded-in ones, if it was one of those).
fn take_card(f: &mut cards::CardFile, card: &str) {
    if let Some(c) = f.owned.get_mut(card) {
        *c = c.saturating_sub(1);
        let left = *c;
        if left == 0 {
            f.owned.remove(card);
        }
        if let Some(t) = f.traded_in.get_mut(card) {
            *t = (*t).min(left);
            if *t == 0 {
                f.traded_in.remove(card);
            }
        }
    }
}

/// One more of a card, marked as came from a trade.
fn give_card(f: &mut cards::CardFile, card: &str) {
    *f.owned.entry(card.to_string()).or_insert(0) += 1;
    *f.traded_in.entry(card.to_string()).or_insert(0) += 1;
}
