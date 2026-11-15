//! Matching the app to your world: `world.ini` (the world's options) and
//! the save (what's in your Bag and party) set the app's own switches, so
//! you don't have to: Game Info's MORE / MODERN POKéMON (which new teams,
//! the Catch Lab and the Pokédex follow), Rare Grass's MODERN, the Breeding
//! Planner's setup and the catch rate. Read only: nothing is written to the
//! game. How your hunt counts battles (Doubles / Singles) is only ever
//! suggested, never changed by itself.

use super::notices::Notice;
use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
use super::App;
use crate::battle::World;
use crate::config::BattleType;
use crate::theme;
use crate::worldini::WorldOptions;
use eframe::egui::{self, vec2, RichText};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct WorldState {
    pub opts: Option<WorldOptions>,
    /// world.ini's time when last read (unix ms).
    mtime: u64,
    last_check: Option<Instant>,
    /// The save last matched from (its time).
    save_seen: u64,
}

fn mtime_ms(p: &std::path::Path) -> Option<u64> {
    std::fs::metadata(p).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
}

impl App {
    /// The world being played now (None: not known, e.g. no game folder).
    pub(super) fn world_now(&self) -> Option<u32> {
        self.setup.world.opts.as_ref()?.active
    }

    /// Count encounters (or take some back) for the world being played.
    pub(super) fn world_note(&mut self, n: i64, now: u64) {
        let Some(w) = self.world_now() else { return };
        let name = self.setup.world.opts.as_ref().map(|o| o.name.clone()).unwrap_or_default();
        let t = self.cfg.world_tally.entry(w).or_default();
        if !name.is_empty() {
            t.name = name;
        }
        let day = crate::stats::local_day(now);
        let d = t.days.entry(day).or_insert(0);
        if n >= 0 {
            t.encounters += n as u64;
            *d += n as u64;
            if t.first == 0 {
                t.first = now;
            }
            t.last = now;
        } else {
            t.encounters = t.encounters.saturating_sub((-n) as u64);
            *d = d.saturating_sub((-n) as u64);
        }
    }

    /// A shiny caught in the world being played.
    pub(super) fn world_shiny(&mut self) {
        if let Some(w) = self.world_now() {
            self.cfg.world_tally.entry(w).or_default().shinies += 1;
        }
    }

    /// Every frame: look at world.ini every few seconds; match the app to
    /// it when it changes (and to the save when the game saves).
    pub(super) fn world_tick(&mut self) {
        if self.setup.world.last_check.is_some_and(|t| t.elapsed() < Duration::from_secs(5)) {
            return;
        }
        self.setup.world.last_check = Some(Instant::now());
        let root = crate::gamemap::game_dir(self.cfg.game_dir_setting().as_deref());
        let ms = mtime_ms(&root.join("world.ini")).unwrap_or(0);
        if ms != self.setup.world.mtime {
            self.setup.world.mtime = ms;
            self.setup.world.opts = crate::worldini::read(&root);
        }
        let catch = match (&self.setup.world.opts, self.cfg.world_match) {
            (Some(w), true) => w.catch_rate().code(),
            _ => 0,
        };
        crate::sim::set_world_catch(catch);
        if !self.cfg.world_match {
            return;
        }
        let mut changed = Vec::new();
        if let Some(w) = self.setup.world.opts.clone() {
            let key = format!("{:?}", w.raw);
            if key != self.cfg.world_applied {
                changed.extend(self.world_apply(&w));
                self.cfg.world_applied = key;
                self.dirty = true;
            }
        }
        let save_ms = self.setup.boxes.save_ms;
        if save_ms != 0 && save_ms != self.setup.world.save_seen {
            self.setup.world.save_seen = save_ms;
            changed.extend(self.world_breed_from_save());
        }
        if !changed.is_empty() {
            self.notify(Notice {
                id: "world-match",
                icon: "🌍",
                title: "Matched to your world".into(),
                body: changed.join(" · "),
                progress: None,
                action: None,
                expires: Some(Instant::now() + Duration::from_secs(8)),
                accent: theme::GREEN,
            });
        }
    }

    /// Set the app's switches from the world's options. Returns what changed.
    fn world_apply(&mut self, w: &WorldOptions) -> Vec<String> {
        let mut out = Vec::new();
        let onoff = |b: bool| if b { "on" } else { "off" };
        if self.cfg.info_modern != w.modern() {
            self.cfg.info_modern = w.modern();
            out.push(format!("MODERN POKéMON {}", onoff(w.modern())));
        }
        if self.cfg.info_more != w.more() {
            self.cfg.info_more = w.more();
            out.push(format!("MORE POKéMON {}", onoff(w.more())));
        }
        if self.cfg.rare_modern != w.modern() {
            self.cfg.rare_modern = w.modern();
            crate::rare_calc::set_modern(w.modern());
            if !out.iter().any(|s| s.starts_with("MODERN")) {
                out.push(format!("Rare grass MODERN {}", onoff(w.modern())));
            }
        }
        let world = if w.modern() { World::Modern } else { World::Platinum };
        let b = self.setup.breed.file();
        if b.setup.world != world {
            b.setup.world = world;
            self.setup.breed.touch();
            out.push("Breeding world".into());
        }
        out
    }

    /// The Breeding Planner's Shiny Charm, Egg Warmer and Flame Body, from
    /// your Bag and party. Returns what changed.
    fn world_breed_from_save(&mut self) -> Vec<String> {
        let Some(s) = self.setup.boxes.save.as_ref() else { return Vec::new() };
        let Some(t) = self.setup.boxes.text.as_ref() else { return Vec::new() };
        let has = |name: &str| s.bag.iter().any(|(_, i, _)| t.item_name(*i) == name);
        let (charm, warmer) = (has("Shiny Charm"), has("Egg Warmer"));
        let flame = s.party.iter().any(|m| !m.egg && (m.ability == 49 || m.ability == 40));
        let b = self.setup.breed.file();
        let mut out = Vec::new();
        for (have, slot, what) in [(charm, &mut b.setup.shiny_charm, "Shiny Charm"), (warmer, &mut b.setup.egg_warmer, "Egg Warmer"), (flame, &mut b.setup.flame_body, "Flame Body")] {
            if *slot != have {
                *slot = have;
                out.push(format!("{what} {}", if have { "✔" } else { "✖" }));
            }
        }
        if !out.is_empty() {
            self.setup.breed.touch();
        }
        out
    }

    /// Settings > Advanced > Game folder: what your world is set to, and
    /// what the app matches to it.
    pub(super) fn world_card(&mut self, ui: &mut egui::Ui) {
        let w = self.setup.world.opts.clone();
        kit::card(ui, |ui| {
            let title = match w.as_ref().map(|w| w.name.clone()).filter(|n| !n.is_empty()) {
                Some(n) => format!("Your world · {n}"),
                None => "Your world".to_string(),
            };
            super::advanced_page::card_head(ui, "🌍", theme::GREEN, &title, "From world.ini in your game folder, read only. Change these in the game; the app picks them up.");
            ui.add_space(14.0);
            let Some(w) = w else {
                kit::pill(ui, "world.ini wasn't found in your game folder", PillKind::Warn);
                return;
            };
            // what the world is set to, two to a row
            let rows = w.summary();
            let cw = ((ui.available_width() - 16.0) / 2.0).floor();
            for pair in rows.chunks(2) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    for (k, v) in pair {
                        super::pokedex::fact(ui, cw, k, v);
                    }
                });
                ui.add_space(8.0);
            }
            ui.add_space(4.0);
            self.dirty |= kit::toggle_row(
                ui,
                "Match the app to my world",
                "Sets MORE / MODERN POKéMON in Game Info (new teams, the Catch Lab and the Pokédex follow them), Rare Grass's MODERN, the Breeding Planner's world, its Shiny Charm, Egg Warmer and Flame Body from your save, and the catch rate in the Catch Lab and Battle Sim. Each time something changes in the game. Your count is never touched.",
                &mut self.cfg.world_match,
            );
            if self.cfg.world_match && w.catch_rate() != crate::worldini::CatchRate::Normal {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "Catch rate {}: catch chances count it ({}). How the game does it isn't published, so they're close, not exact.",
                        w.catch_rate().label().to_lowercase(),
                        if w.catch_rate() == crate::worldini::CatchRate::Always { "every ball catches" } else { "the catch value multiplied" }
                    ))
                    .size(11.5)
                    .color(MUTED()),
                );
            }
            // how the hunt counts battles: only ever suggested
            if let Some(d) = w.doubles() {
                let want = if d { BattleType::Doubles } else { BattleType::Singles };
                if self.cfg.battle_type != want {
                    ui.add_space(10.0);
                    egui::Frame::none().fill(kit::sunken()).rounding(10.0).inner_margin(egui::Margin::symmetric(12.0, 8.0)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                            let now = match self.cfg.battle_type {
                                BattleType::Doubles => "Doubles",
                                BattleType::Singles => "Singles",
                                BattleType::Auto => "Not sure",
                            };
                            ui.label(RichText::new(format!("Your world has {} wild battles; your hunt is set to {now}.", if d { "double" } else { "single" })).size(12.0).color(TEXT()));
                            let label = if d { "Use Doubles" } else { "Use Singles" };
                            if ui.add(egui::Button::new(label).fill(kit::btn_fill())).on_hover_text("Changes how each battle's shiny chance is worked out. Your count stays as it is.").clicked() {
                                self.cfg.battle_type = want;
                                self.dirty = true;
                                self.push_settings(true);
                            }
                        });
                    });
                }
            }
        });
    }
}
