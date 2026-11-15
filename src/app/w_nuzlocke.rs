//! Dashboard widgets for the Nuzlocke Tracker (read from the game's own
//! files, read only).

use super::nuzlocke_app::NUZ_COL;
use super::ui_kit as kit;
use super::w_build::{Head, Lead, Pic, Row, Stat};
use super::w_fit::wbox;
use super::{App, Tab};
use crate::nuzlocke::RouteMon;
use crate::theme;
use eframe::egui;

const IN_COL: egui::Color32 = egui::Color32::from_rgb(52, 211, 153);
const OUT_COL: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);

impl App {
    fn nuz_head(icon: &'static str, title: &'static str) -> Head<'static> {
        Head { icon, col: NUZ_COL, title, link: Some("Open ›") }
    }

    /// The species list's index for one of the game's species numbers.
    fn nuz_idx(&self, id: u16) -> Option<usize> {
        super::party::resolve(&self.cfg, &self.species, id as u32).and_then(|slug| self.species.iter().position(|s| s.slug == slug))
    }

    fn nuz_name(&self, r: &RouteMon) -> String {
        self.nuz_idx(r.species).map_or_else(|| format!("#{}", r.species), |i| self.species[i].name.to_string())
    }

    fn nuz_row(&self, r: &RouteMon) -> Row {
        let name = self.nuz_name(r);
        let row = Row::new(name);
        match self.nuz_idx(r.species) {
            Some(i) => row.lead(Lead::Species(i, false)),
            None => row,
        }
    }

    fn nuz_empty(&self) -> &'static str {
        "No Nuzlocke run yet: start one in the game."
    }

    fn nuz_open(&mut self, go: bool) {
        if go {
            self.go_to(Tab::Nuzlocke);
        }
    }

    /// Your run in numbers.
    pub(super) fn w_nuz_run(&mut self, ui: &mut egui::Ui) {
        self.nuz_refresh();
        let b = wbox(ui, "nuz_run");
        let w = self.nuz_world().cloned();
        let (total, ins) = w.as_ref().map_or((0, 0), |w| (w.routes.len(), w.routes.iter().filter(|r| r.alive).count()));
        let stats = [Stat::new(total.to_string(), "routes", NUZ_COL), Stat::new(ins.to_string(), "in", IN_COL), Stat::new((total - ins).to_string(), "out", OUT_COL)];
        let note = w.as_ref().map(|w| w.title()).unwrap_or_else(|| self.nuz_empty().to_string());
        let go = self.f_stats(ui, &b, Self::nuz_head("💀", "Your run in numbers"), &stats, Some(&note));
        self.nuz_open(go);
    }

    /// The newest routes, newest first.
    pub(super) fn w_nuz_routes(&mut self, ui: &mut egui::Ui) {
        self.nuz_refresh();
        let b = wbox(ui, "nuz_routes");
        let routes = self.nuz_world().map(|w| w.routes.clone()).unwrap_or_default();
        let rows: Vec<Row> = routes
            .iter()
            .rev()
            .map(|r| {
                let (t, c) = if r.alive { ("In", IN_COL) } else { ("Out", OUT_COL) };
                self.nuz_row(r).sub(format!("{} · Lv {}", r.route, r.level)).right(t, Some(c))
            })
            .collect();
        let c = self.f_list(ui, &b, Self::nuz_head("📍", "Latest routes"), &rows, self.nuz_empty());
        self.nuz_open(c.link || c.item.is_some() || c.more);
    }

    /// Every Pokémon out of the run.
    pub(super) fn w_nuz_lost(&mut self, ui: &mut egui::Ui) {
        self.nuz_refresh();
        let b = wbox(ui, "nuz_lost");
        let w = self.nuz_world().cloned();
        let rows: Vec<Row> = w.as_ref().map_or_else(Vec::new, |w| w.routes.iter().filter(|r| !r.alive).rev().map(|r| self.nuz_row(r).sub(r.route.clone()).right(format!("Lv {}", r.level), Some(kit::C_SLATE))).collect());
        let empty = if w.is_some() { "Nobody's out of the run yet." } else { self.nuz_empty() };
        let c = self.f_list(ui, &b, Self::nuz_head("🕯", "Out of the run"), &rows, empty);
        self.nuz_open(c.link || c.item.is_some() || c.more);
    }

    /// Your team, in the world you're playing.
    pub(super) fn w_nuz_team(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "nuz_team");
        let party = self.setup.boxes.save.as_ref().map(|s| s.party.clone()).unwrap_or_default();
        let pics: Vec<Pic> = party
            .iter()
            .filter(|m| !m.egg)
            .map(|m| {
                let idx = self.nuz_idx(m.species);
                let name = idx.map_or_else(|| format!("#{}", m.species), |i| self.species[i].name.to_string());
                let lv = m.level.map(|l| format!("Lv {l}")).unwrap_or_default();
                Pic { lead: idx.map_or(Lead::None, |i| Lead::Species(i, m.shiny())), name: name.clone(), sub: lv, tip: name, dim: false }
            })
            .collect();
        let empty = if self.setup.boxes.save.is_some() { "No team yet." } else { "Your save hasn't been read yet: open the Nuzlocke Tracker." };
        let c = self.f_pics(ui, &b, Self::nuz_head("👥", "Nuzlocke team"), &pics, empty, 52.0);
        self.nuz_open(c.link || c.item.is_some());
    }

    /// The run's rules.
    pub(super) fn w_nuz_rules(&mut self, ui: &mut egui::Ui) {
        self.nuz_refresh();
        let b = wbox(ui, "nuz_rules");
        let w = self.nuz_world().cloned();
        let mut rows: Vec<Row> = Vec::new();
        if let Some(w) = &w {
            rows.push(Row::new("Type").right(w.kind(), Some(NUZ_COL)));
            if let Some(o) = &w.opts {
                if let Some(v) = o.level_caps() {
                    rows.push(Row::new("Level caps").right(v, None));
                }
                if let Some(v) = o.word("lives") {
                    rows.push(Row::new("Lives").right(if v == "0" { "None".to_string() } else { v.to_string() }, None));
                }
                if let Some(v) = o.word("party").filter(|v| *v != "1") {
                    rows.push(Row::new("Players").right(v.to_string(), Some(theme::GOLD)));
                }
            }
        }
        let c = self.f_list(ui, &b, Self::nuz_head("📜", "Your run's rules"), &rows, self.nuz_empty());
        self.nuz_open(c.link || c.item.is_some() || c.more);
    }
}
