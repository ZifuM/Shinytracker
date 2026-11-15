//! The Pokédex's "Your game" view: what your save's Pokédex has seen and
//! caught (read only). The game keeps that for the first 493; the newer
//! Pokémon count as caught when one is in your party or boxes.

use super::ui_kit::{self as kit, CARD, MUTED, TEXT};
use super::App;
use crate::moddex::Region;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Rect, Sense, Stroke};
use std::collections::{BTreeMap, BTreeSet};

/// Where a Pokémon is in your game's Pokédex.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum InGame {
    Unseen,
    Seen,
    Caught,
}

/// Your save's Pokédex, by National Dex number.
pub(super) struct GameDex {
    pub seen: BTreeSet<u32>,
    pub caught: BTreeSet<u32>,
    /// How many you have in your party and boxes.
    pub owned: BTreeMap<u32, usize>,
}

impl GameDex {
    pub fn status(&self, dex: u32) -> InGame {
        if self.caught.contains(&dex) || self.owned.contains_key(&dex) {
            InGame::Caught
        } else if self.seen.contains(&dex) {
            InGame::Seen
        } else {
            InGame::Unseen
        }
    }
}

/// The colour for caught in this view.
pub(super) const CAUGHT_COL: egui::Color32 = egui::Color32::from_rgb(232, 72, 72);

impl App {
    /// Your game's Pokédex (None: the save hasn't been read).
    pub(super) fn game_dex(&self) -> Option<GameDex> {
        let s = self.setup.boxes.save.as_ref()?;
        let mut owned = BTreeMap::new();
        let all = s.party.iter().chain(s.boxes.iter().flat_map(|b| b.slots.iter().flatten()));
        for m in all.filter(|m| !m.egg) {
            if let Some(n) = crate::species::national_from_game(m.species as u32) {
                *owned.entry(n).or_insert(0) += 1;
            }
        }
        Some(GameDex {
            seen: s.dex_seen.iter().map(|n| *n as u32).collect(),
            caught: s.dex_caught.iter().map(|n| *n as u32).collect(),
            owned,
        })
    }

    /// The top card in "Your game": caught and seen, all and by region.
    /// `dexes`: every Pokémon listed (National Dex number, newer or not).
    pub(super) fn game_dex_card(&mut self, ui: &mut egui::Ui, dexes: &[(u32, bool)], g: Option<&GameDex>) {
        let Some(g) = g else {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                let why = self.setup.boxes.error.clone().unwrap_or_else(|| "Reading your save…".into());
                ui.label(egui::RichText::new(why).size(12.5).color(MUTED()));
            });
            return;
        };
        let mut per: Vec<(Region, usize, usize, usize)> = Region::ALL.iter().map(|r| (*r, 0, 0, 0)).collect();
        for &(dex, modern) in dexes {
            let r = Region::of(dex as u16, modern);
            let st = g.status(dex);
            if let Some(e) = per.iter_mut().find(|e| e.0 == r) {
                e.3 += 1;
                e.1 += (st == InGame::Caught) as usize;
                e.2 += (st != InGame::Unseen) as usize;
            }
        }
        // the game's own Pokédex: the first 493
        let caught = (1..=493).filter(|d| g.status(*d) == InGame::Caught).count();
        let seen = (1..=493).filter(|d| g.status(*d) != InGame::Unseen).count();
        let sel = self.setup.pokedex.region;
        let mut pick: Option<Option<Region>> = None;
        kit::card(ui, |ui| {
            let wide = ui.available_width() >= 760.0;
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), if wide { 92.0 } else { 200.0 }), Sense::hover());
            let p = ui.painter().clone();
            let c = pos2(rect.min.x + 46.0, rect.min.y + 46.0);
            // two rings: seen outside, caught inside
            let fs = kit::grow(ui.ctx(), egui::Id::new("gdex-seen"), seen as f32 / 493.0);
            let fc = kit::grow(ui.ctx(), egui::Id::new("gdex-caught"), caught as f32 / 493.0);
            p.circle_stroke(c, 40.0, Stroke::new(5.0f32, theme::blend(CARD(), TEXT(), 0.09)));
            super::pokedex::ring_arc(&p, c, 40.0, fs, Stroke::new(5.0f32, theme::blend(CARD(), CAUGHT_COL, 0.45)));
            p.circle_stroke(c, 31.0, Stroke::new(7.0f32, theme::blend(CARD(), TEXT(), 0.09)));
            super::pokedex::ring_arc(&p, c, 31.0, fc, Stroke::new(7.0f32, CAUGHT_COL));
            p.text(c - vec2(0.0, 5.0), Align2::CENTER_CENTER, caught.to_string(), super::charts::heavy(17.0), TEXT());
            p.text(c + vec2(0.0, 11.0), Align2::CENTER_CENTER, "caught", super::charts::reg(10.0), MUTED());
            let x = rect.min.x + 104.0;
            p.text(pos2(x, rect.min.y + 22.0), Align2::LEFT_CENTER, "Your game's Pokédex", super::charts::bold(16.0), TEXT());
            p.text(
                pos2(x, rect.min.y + 44.0),
                Align2::LEFT_CENTER,
                format!("{caught} caught · {seen} seen of 493 · from your save, read only · click a region to show it"),
                super::charts::reg(12.5),
                MUTED(),
            );
            let (rx, ry, rw) = if wide { (x, rect.min.y + 62.0, (rect.max.x - x) / 5.0) } else { (rect.min.x, rect.min.y + 100.0, rect.width() / 2.0) };
            for (k, (reg, n, s, tot)) in per.iter().enumerate() {
                let (cx, cy) = if wide { (rx + rw * k as f32, ry) } else { (rx + rw * (k % 2) as f32, ry + 34.0 * (k / 2) as f32) };
                let cell = Rect::from_min_size(pos2(cx - 6.0, cy - 6.0), vec2(rw - 10.0, 32.0));
                let resp = ui.interact(cell, ui.id().with(("gdex-region", k)), Sense::click());
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
                let tot = (*tot).max(1) as f32;
                let fseen = kit::grow(ui.ctx(), egui::Id::new(("gdex-reg-seen", k)), *s as f32 / tot);
                let fc = kit::grow(ui.ctx(), egui::Id::new(("gdex-reg", k)), *n as f32 / tot);
                if fseen > 0.0 {
                    p.rect_filled(Rect::from_min_size(bar.min, vec2((bar.width() * fseen).max(5.0), 5.0)), 2.5, theme::blend(CARD(), CAUGHT_COL, 0.4));
                }
                if fc > 0.0 {
                    p.rect_filled(Rect::from_min_size(bar.min, vec2((bar.width() * fc).max(5.0), 5.0)), 2.5, CAUGHT_COL);
                }
                let tip = if *reg == Region::Modern {
                    format!("{n} caught of {tot:.0}. The game's Pokédex only covers the first 493, so these count when one's in your party or boxes.")
                } else {
                    format!("{n} caught, {s} seen of {tot:.0}")
                };
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("{tip}\n{}", if on { "Click to show every region" } else { "Click to show only this region" })).clicked() {
                    pick = Some(if on { None } else { Some(*reg) });
                }
            }
        });
        if let Some(r) = pick {
            self.setup.pokedex.region = r;
            self.setup.pokedex.grid_gen += 1;
        }
    }
}
