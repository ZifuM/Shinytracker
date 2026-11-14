//! Dashboard widgets for Rare Grass.

use super::rare_app::{countdown, day_of, reset_now, secs_to_reset, when, OutToday, GREEN};
use super::ui_kit as kit;
use super::w_build::{Head, Lead, Row, Stat};
use super::w_fit::wbox;
use super::{App, Tab};
use crate::theme;
use eframe::egui;

impl App {
    fn w_rare_where(o: &OutToday) -> String {
        let more = o.spots.len().saturating_sub(1);
        if more > 0 {
            format!("{} {} · +{more} place{}", o.spots[0].place, o.spots[0].tiles, if more == 1 { "" } else { "s" })
        } else {
            format!("{} · {} tile{}", o.spots[0].place, o.tiles(), if o.tiles() == 1 { "" } else { "s" })
        }
    }

    fn rare_head(icon: &'static str, title: &'static str) -> Head<'static> {
        Head { icon, col: GREEN, title, link: Some("Open ›") }
    }

    /// The rarest Pokémon out today.
    pub(super) fn w_rare_out(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "rare_out");
        let today = self.rare_out_today();
        let rows: Vec<Row> = today
            .iter()
            .map(|o| {
                let sp = &o.spots[0].species;
                Row::new(sp.name).lead(Lead::Slug(sp.slug.clone())).sub(Self::w_rare_where(o)).right(format!("{}", o.tiles()), Some(GREEN)).tip("Open Rare Grass")
            })
            .collect();
        let c = self.f_list(ui, &b, Self::rare_head("✨", "Rare grass today"), &rows, "Nothing on rare grass yet today.");
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::RareGrass);
        }
    }

    /// Pokémon on rare grass for the first time.
    pub(super) fn w_rare_new(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "rare_new");
        let today = self.rare_out_today();
        let rows: Vec<Row> = today
            .iter()
            .filter(|o| o.days == 1)
            .map(|o| {
                let sp = &o.spots[0].species;
                Row::new(sp.name).lead(Lead::Slug(sp.slug.clone())).sub(Self::w_rare_where(o)).right("New", Some(theme::GOLD))
            })
            .collect();
        let c = self.f_list(ui, &b, Self::rare_head("🆕", "New on rare grass"), &rows, "Nothing new today: every Pokémon out has been seen before.");
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::RareGrass);
        }
    }

    /// Today in numbers.
    pub(super) fn w_rare_stats(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "rare_stats");
        let today = self.rare_out_today();
        let tiles: usize = today.iter().map(|o| o.tiles()).sum();
        let seen = self.rare_glossary().len();
        let stats = [Stat::new(today.len().to_string(), "out today", GREEN), Stat::new(tiles.to_string(), "tiles today", GREEN), Stat::new(seen.to_string(), "seen ever", kit::C_GOLD)];
        let note = format!("New list at the reset in {}", countdown(secs_to_reset()));
        if self.f_stats(ui, &b, Self::rare_head("🌿", "Rare grass in numbers"), &stats, Some(&note)) {
            self.go_to(Tab::RareGrass);
        }
    }

    /// The places with the most Pokémon today.
    pub(super) fn w_rare_places(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "rare_places");
        let today = self.rare_out_today();
        let mut places: Vec<(String, Vec<&'static str>, usize)> = Vec::new();
        for o in &today {
            for s in &o.spots {
                match places.iter_mut().find(|p| p.0 == s.place) {
                    Some(p) => {
                        p.1.push(s.species.name);
                        p.2 += s.tiles;
                    }
                    None => places.push((s.place.clone(), vec![s.species.name], s.tiles)),
                }
            }
        }
        places.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
        let rows: Vec<Row> = places.iter().map(|(place, mons, tiles)| Row::new(place.clone()).lead(Lead::Icon("📍".into(), GREEN)).sub(mons.join(", ")).right(format!("{tiles} tiles"), Some(GREEN))).collect();
        let c = self.f_list(ui, &b, Self::rare_head("📍", "Busiest places today"), &rows, "Nothing on rare grass yet today.");
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::RareGrass);
        }
    }

    /// Seen before, but not in the last few days.
    pub(super) fn w_rare_missing(&mut self, ui: &mut egui::Ui) {
        let b = wbox(ui, "rare_missing");
        let today = day_of(reset_now());
        let mut gone: Vec<(String, i64)> = self
            .rare_glossary()
            .into_iter()
            .filter_map(|g| {
                let last = *g.days.last()?;
                (today - day_of(last) >= 3).then_some((g.slug, last))
            })
            .collect();
        gone.sort_by(|a, b| a.1.cmp(&b.1));
        let rows: Vec<Row> = gone
            .iter()
            .map(|(slug, last)| {
                let name = self.species.iter().find(|s| &s.slug == slug).map(|s| s.name).unwrap_or("?");
                Row::new(name).lead(Lead::Slug(slug.clone())).sub(format!("Last on rare grass: {}", when(*last)))
            })
            .collect();
        let c = self.f_list(ui, &b, Self::rare_head("⏳", "Not seen lately"), &rows, "Everything seen before has been out in the last 3 days.");
        if c.link || c.item.is_some() || c.more {
            self.go_to(Tab::RareGrass);
        }
    }
}
