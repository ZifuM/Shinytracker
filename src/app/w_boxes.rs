//! Dashboard widgets for Pokémon Boxes (all read from the save, read only).

use super::boxes_app::BOX_COL;
use super::ui_kit as kit;
use super::w_build::{Head, Lead, Row, Stat};
use super::w_fit::wbox;
use super::{App, Tab};
use crate::gamesave::Mon;
use crate::theme;
use eframe::egui;

impl App {
    fn boxes_head(icon: &'static str, title: &'static str) -> Head<'static> {
        Head { icon, col: BOX_COL, title, link: Some("Open ›") }
    }

    /// A Pokémon as a widget row's picture and name.
    fn boxes_row(&self, m: &Mon) -> Row {
        let idx = super::party::resolve(&self.cfg, &self.species, m.species as u32).and_then(|slug| self.species.iter().position(|s| s.slug == slug));
        let name = if m.egg {
            "Egg".to_string()
        } else if m.nicknamed && !m.nickname.is_empty() {
            m.nickname.clone()
        } else {
            idx.map_or_else(|| format!("#{}", m.species), |i| self.species[i].name.to_string())
        };
        let r = Row::new(name);
        match idx {
            Some(i) if !m.egg => r.lead(Lead::Species(i, m.shiny())),
            _ => r,
        }
    }

    fn boxes_all(&self) -> Vec<(String, Mon)> {
        let Some(s) = self.setup.boxes.save.as_ref() else { return Vec::new() };
        s.boxes.iter().flat_map(|b| b.slots.iter().flatten().map(move |m| (b.name.clone(), m.clone()))).collect()
    }

    fn boxes_empty_note(&self) -> &'static str {
        if self.setup.boxes.save.is_some() {
            "Nothing here yet."
        } else {
            "Your save hasn't been read yet: open Pokémon Boxes."
        }
    }

    /// Your party, from the last save.
    pub(super) fn w_boxes_party(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "boxes_party");
        let party = self.setup.boxes.save.as_ref().map(|s| s.party.clone()).unwrap_or_default();
        let rows: Vec<Row> = party
            .iter()
            .map(|m| {
                let lv = m.level.map(|l| format!("Lv {l}")).unwrap_or_default();
                let r = self.boxes_row(m).sub(lv);
                if m.shiny() { r.right("★", Some(theme::GOLD)) } else { r }
            })
            .collect();
        let empty = self.boxes_empty_note();
        let c = self.f_list(ui, &b, Self::boxes_head("👥", "Your party"), &rows, empty);
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::Boxes);
        }
    }

    /// Boxes in numbers.
    pub(super) fn w_boxes_stats(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "boxes_stats");
        let all = self.boxes_all();
        let shinies = all.iter().filter(|(_, m)| m.shiny()).count();
        let used = self.setup.boxes.save.as_ref().map_or(0, |s| s.boxes.iter().filter(|b| b.slots.iter().any(|x| x.is_some())).count());
        let stats = [Stat::new(all.len().to_string(), "in boxes", BOX_COL), Stat::new(shinies.to_string(), "shiny", theme::GOLD), Stat::new(used.to_string(), "boxes used", kit::C_SLATE)];
        let note = "Read only, from your save";
        if self.f_stats(ui, &b, Self::boxes_head("📦", "Boxes in numbers"), &stats, Some(note)) {
            self.go_to(Tab::Boxes);
        }
    }

    /// Every shiny in your boxes.
    pub(super) fn w_boxes_shinies(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "boxes_shinies");
        let all = self.boxes_all();
        let rows: Vec<Row> = all.iter().filter(|(_, m)| m.shiny()).map(|(bx, m)| self.boxes_row(m).sub(bx.clone()).right("★", Some(theme::GOLD))).collect();
        let empty = if self.setup.boxes.save.is_some() { "No shinies in your boxes yet." } else { self.boxes_empty_note() };
        let c = self.f_list(ui, &b, Self::boxes_head("★", "Shinies in your boxes"), &rows, empty);
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::Boxes);
        }
    }

    /// The best IVs in your boxes and party.
    pub(super) fn w_boxes_ivs(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "boxes_ivs");
        let mut all = self.boxes_all();
        if let Some(s) = self.setup.boxes.save.as_ref() {
            all.extend(s.party.iter().map(|m| ("Party".to_string(), m.clone())));
        }
        all.retain(|(_, m)| !m.egg);
        all.sort_by_key(|(_, m)| std::cmp::Reverse(m.ivs.iter().map(|v| *v as u32).sum::<u32>()));
        let rows: Vec<Row> = all
            .iter()
            .take(12)
            .map(|(bx, m)| {
                let total: u32 = m.ivs.iter().map(|v| *v as u32).sum();
                let perfect = m.ivs.iter().filter(|v| **v == 31).count();
                self.boxes_row(m).sub(format!("{bx} · {perfect}×31")).right(format!("{total}/186"), Some(if total >= 160 { theme::GOLD } else { BOX_COL }))
            })
            .collect();
        let empty = self.boxes_empty_note();
        let c = self.f_list(ui, &b, Self::boxes_head("💎", "Best IVs"), &rows, empty);
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::Boxes);
        }
    }

    /// The Pokédex, from the save.
    pub(super) fn w_boxes_dex(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "boxes_dex");
        let (seen, caught) = self.setup.boxes.save.as_ref().map_or((0, 0), |s| (s.dex_seen.len(), s.dex_caught.len()));
        let stats = [Stat::new(caught.to_string(), "caught", kit::C_GREEN), Stat::new(seen.to_string(), "seen", BOX_COL), Stat::new((493 - seen.min(493)).to_string(), "not seen", kit::C_SLATE)];
        let note = "The first 493 Pokémon, from your save";
        if self.f_stats(ui, &b, Self::boxes_head("📕", "Your Pokédex"), &stats, Some(note)) {
            self.go_to(Tab::Boxes);
        }
    }
}
