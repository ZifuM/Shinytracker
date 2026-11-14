//! Dashboard widgets for Rare Grass.

use super::rare_app::{countdown, day_of, reset_now, secs_to_reset, when, OutToday, GREEN};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::w_kit::{head, note};
use super::{App, Tab};
use eframe::egui::{self, vec2, RichText, Sense};

impl App {
    fn w_rare_head(&mut self, ui: &mut egui::Ui, icon: &str, title: &str) -> bool {
        head(ui, icon, GREEN, title, Some("Open ›"))
    }

    /// A row: sprite, name, and a line under it.
    fn w_rare_row(&mut self, ui: &mut egui::Ui, slug: &str, name: &str, sub: &str) {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
            let p = ui.painter().clone();
            self.rare_sprite(&p, r.center(), 13.0, slug, false);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(name).font(crate::fonts::sans_bold_font(12.5)).color(TEXT()));
                ui.add(egui::Label::new(RichText::new(sub).size(11.0).color(MUTED())).truncate());
            });
        });
    }

    fn w_rare_where(o: &OutToday) -> String {
        let more = o.spots.len().saturating_sub(1);
        if more > 0 {
            format!("{} {} · +{more} place{}", o.spots[0].place, o.spots[0].tiles, if more == 1 { "" } else { "s" })
        } else {
            format!("{} · {} tile{}", o.spots[0].place, o.tiles(), if o.tiles() == 1 { "" } else { "s" })
        }
    }

    /// The rarest Pokémon out today.
    pub(super) fn w_rare_out(&mut self, ui: &mut egui::Ui) {
        let today = self.rare_out_today();
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_rare_head(ui, "✨", "Rare grass today");
            ui.add_space(6.0);
            if today.is_empty() {
                note(ui, "Nothing on rare grass yet today.");
                return;
            }
            let room = kit::take_card_fill().map(|h| ((h - 70.0) / 38.0).floor() as usize).unwrap_or(5).clamp(2, 12);
            for o in today.iter().take(room) {
                let sp = o.spots[0].species.clone();
                self.w_rare_row(ui, &sp.slug, sp.name, &Self::w_rare_where(o));
            }
            if today.len() > room {
                note(ui, format!("+{} more · rarest first", today.len() - room));
            }
        });
        if go {
            self.go_to(Tab::RareGrass);
        }
    }

    /// Pokémon on rare grass for the first time.
    pub(super) fn w_rare_new(&mut self, ui: &mut egui::Ui) {
        let today = self.rare_out_today();
        let firsts: Vec<&OutToday> = today.iter().filter(|o| o.days == 1).collect();
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_rare_head(ui, "🆕", "New on rare grass");
            ui.add_space(6.0);
            if firsts.is_empty() {
                note(ui, "Nothing new today: every Pokémon out has been seen before.");
                return;
            }
            for o in firsts.iter().take(5) {
                let sp = o.spots[0].species.clone();
                self.w_rare_row(ui, &sp.slug, sp.name, &Self::w_rare_where(o));
            }
            if firsts.len() > 5 {
                note(ui, format!("+{} more first-timers", firsts.len() - 5));
            }
        });
        if go {
            self.go_to(Tab::RareGrass);
        }
    }

    /// Today in numbers.
    pub(super) fn w_rare_stats(&mut self, ui: &mut egui::Ui) {
        let today = self.rare_out_today();
        let tiles: usize = today.iter().map(|o| o.tiles()).sum();
        let seen = self.rare_glossary().len();
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_rare_head(ui, "🌿", "Rare grass in numbers");
            ui.add_space(8.0);
            ui.columns(3, |c| {
                for (ui, (n, l)) in c.iter_mut().zip([(today.len(), "out today"), (tiles, "tiles today"), (seen, "seen ever")]) {
                    ui.label(RichText::new(n.to_string()).font(crate::fonts::sans_bold_font(24.0)).color(TEXT()));
                    ui.label(RichText::new(l).size(11.5).color(MUTED()));
                }
            });
            ui.add_space(6.0);
            note(ui, format!("New list at the reset in {}", countdown(secs_to_reset())));
        });
        if go {
            self.go_to(Tab::RareGrass);
        }
    }

    /// The places with the most Pokémon today.
    pub(super) fn w_rare_places(&mut self, ui: &mut egui::Ui) {
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
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_rare_head(ui, "📍", "Busiest places today");
            ui.add_space(6.0);
            if places.is_empty() {
                note(ui, "Nothing on rare grass yet today.");
                return;
            }
            for (place, mons, tiles) in places.iter().take(5) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(place).font(crate::fonts::sans_bold_font(12.5)).color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{tiles} tiles")).size(11.0).color(GREEN));
                    });
                });
                ui.add(egui::Label::new(RichText::new(mons.join(", ")).size(11.5).color(MUTED())).truncate());
                ui.add_space(3.0);
            }
        });
        if go {
            self.go_to(Tab::RareGrass);
        }
    }

    /// Seen before, but not in the last few days.
    pub(super) fn w_rare_missing(&mut self, ui: &mut egui::Ui) {
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
        let mut go = false;
        kit::card(ui, |ui| {
            go = self.w_rare_head(ui, "⏳", "Not seen lately");
            ui.add_space(6.0);
            if gone.is_empty() {
                note(ui, "Everything seen before has been out in the last 3 days.");
                return;
            }
            for (slug, last) in gone.iter().take(5) {
                let name = self.species.iter().find(|s| &s.slug == slug).map(|s| s.name).unwrap_or("?");
                self.w_rare_row(ui, slug, name, &format!("Last on rare grass: {}", when(*last)));
            }
            if gone.len() > 5 {
                note(ui, format!("+{} more", gone.len() - 5));
            }
        });
        if go {
            self.go_to(Tab::RareGrass);
        }
    }
}
