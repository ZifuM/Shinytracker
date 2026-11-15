//! The Poké Radar chain, counted from the game's log. Platinum Together
//! writes no chain line, so the app works it out:
//!
//! - the Radar is used (item 431): the chain is on (using it again never
//!   resets it);
//! - a wild battle with the chain's Pokémon that ends with it knocked out or
//!   caught: +1;
//! - you run, a different Pokémon shows up, or it's a double battle: the
//!   chain is broken, back to 0 until the Radar is used again.
//!
//! From chain 40 the odds stop improving, so each Radar battle from then on
//! counts as one "check at max chain".

use super::notices::Notice;
use super::App;
use crate::config::{Config, HuntMethod, RADAR_MAX};
use crate::gamelog::{BattleEnd, RadarEvent};
use crate::theme;
use std::time::{Duration, Instant};

/// How long a broken chain shows in red on the overlay.
pub(super) const BROKEN_FLASH: Duration = Duration::from_secs(5);

/// What the chain did, worked out from one log event (kept apart from the
/// app so the rules can be tested on their own).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Step {
    Nothing,
    /// The Radar went on.
    On,
    /// The chain went up to this.
    Up(u32),
    /// The chain broke at this length, and why.
    Broke(u32, String),
}

/// Apply one event to the chain in `c`. `name` gives a Pokémon's name.
pub(crate) fn step(c: &mut Config, ev: &RadarEvent, name: &dyn Fn(u32) -> String) -> Step {
    match ev {
        RadarEvent::Used => {
            if c.radar_on {
                Step::Nothing
            } else {
                c.radar_on = true;
                Step::On
            }
        }
        RadarEvent::Battle { species, multi, end } => {
            if !c.radar_on {
                return Step::Nothing;
            }
            let before = c.radar_chain;
            if before >= RADAR_MAX {
                c.radar_checks += 1;
            }
            let why = if *multi {
                Some("it was a double battle".to_string())
            } else if before > 0 && c.radar_species != Some(*species) {
                Some(format!("a {} showed up", name(*species)))
            } else if *end == BattleEnd::Ran {
                Some("you ran".to_string())
            } else {
                None
            };
            match why {
                Some(why) => {
                    c.radar_chain = 0;
                    c.radar_species = None;
                    c.radar_on = false;
                    if before > 0 {
                        c.radar_breaks += 1;
                        Step::Broke(before, why)
                    } else {
                        Step::Nothing
                    }
                }
                None => {
                    c.radar_chain = before + 1;
                    c.radar_species = Some(*species);
                    c.radar_best = c.radar_best.max(c.radar_chain);
                    Step::Up(c.radar_chain)
                }
            }
        }
    }
}

/// The overlay's chain section: label above a 40-step meter.
pub(super) struct RadarBar {
    pub label: String,
    pub filled: u32,
    pub look: BarLook,
    pub tip: String,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum BarLook {
    Off,
    Building,
    Max,
    Broken,
}

impl RadarBar {
    /// Label colour, and the meter's fill colour.
    pub fn colors(&self, accent: eframe::egui::Color32, muted: eframe::egui::Color32) -> (eframe::egui::Color32, eframe::egui::Color32) {
        match self.look {
            // green while building, so it never looks like a broken (red) chain
            BarLook::Off => (muted, accent),
            BarLook::Building => (theme::GREEN, theme::GREEN),
            BarLook::Max => (theme::GOLD, theme::GOLD),
            BarLook::Broken => (theme::HP_RED, theme::HP_RED),
        }
    }
}

/// The chain section's width in the bar (fixed, so nothing shifts).
pub(super) const BAR_CHAIN_W: f32 = 180.0;
/// The name + count section's least width while hunting with the Radar.
pub(super) const BAR_COUNT_W: f32 = 116.0;

/// A 40-step meter, `filled` of them lit (`on` hides the lit steps, to flash).
pub(super) fn meter(p: &eframe::egui::Painter, r: eframe::egui::Rect, filled: u32, fill: eframe::egui::Color32, empty: eframe::egui::Color32, on: bool) {
    let n = RADAR_MAX as f32;
    let gap = 1.0;
    let w = (r.width() - gap * (n - 1.0)) / n;
    for i in 0..RADAR_MAX {
        let x = r.min.x + i as f32 * (w + gap);
        let seg = eframe::egui::Rect::from_min_size(eframe::egui::pos2(x, r.min.y), eframe::egui::vec2(w, r.height()));
        let lit = i < filled && on;
        p.rect_filled(seg, 1.0, if lit { fill } else { empty });
    }
}

/// "you ran" → "YOU RAN"; "a Bibarel showed up" → "BIBAREL"; "it was a double battle" → "DOUBLE BATTLE".
fn why_short(why: &str) -> String {
    if let Some(name) = why.strip_prefix("a ").and_then(|w| w.strip_suffix(" showed up")) {
        name.to_uppercase()
    } else if why.contains("double") {
        "DOUBLE BATTLE".to_string()
    } else {
        why.to_uppercase()
    }
}

/// The chain's numbers back to the start (a new hunt, or Reset).
pub(crate) fn clear(c: &mut Config, whole_hunt: bool) {
    c.radar_chain = 0;
    c.radar_species = None;
    c.radar_on = false;
    if whole_hunt {
        c.radar_best = 0;
        c.radar_checks = 0;
        c.radar_breaks = 0;
    }
}

impl App {
    pub(super) fn radar_event(&mut self, ev: RadarEvent) {
        if self.cfg.hunt_method != HuntMethod::PokeRadar {
            // Using the Radar switches to it (like rare grass), when allowed.
            if ev != RadarEvent::Used || !self.cfg.auto_method {
                return;
            }
            self.cfg.hunt_method = HuntMethod::PokeRadar;
            clear(&mut self.cfg, false);
            self.notify(Notice {
                id: "auto-method",
                icon: "🔁",
                title: "Switched to Poké Radar".into(),
                body: "You used the Poké Radar, so the chain is counted from now. Turn this off in Settings > Hunt.".into(),
                progress: None,
                action: None,
                expires: Some(Instant::now() + Duration::from_secs(8)),
                accent: theme::GREEN,
            });
        }
        let species = self.species.clone();
        // the log's number is the game's own: the same as the Pokédex up to #493
        let name = move |id: u32| crate::species::national_from_game(id).and_then(|d| species.get(d as usize - 1)).map_or_else(|| format!("#{id}"), |s| s.name.to_string());
        let st = step(&mut self.cfg, &ev, &name);
        self.dirty = true;
        match st {
            Step::Up(n) if n == RADAR_MAX => self.radar_notice("Max chain!", format!("Odds are now 1/{} for every patch. From here each Radar battle counts as a check.", super::fmt_one_in(Config::radar_one_in(n))), theme::GOLD),
            Step::Up(n) if n % 10 == 0 && n < RADAR_MAX => {
                let who = self.cfg.radar_species.map(|d| name(d)).unwrap_or_default();
                self.radar_notice(&format!("Chain {n}"), format!("{who} · odds 1/{}", super::fmt_one_in(Config::radar_one_in(n))), theme::GREEN)
            }
            Step::Broke(at, why) => {
                self.radar_flash = Some((at, why.clone(), Instant::now()));
                self.radar_notice(&format!("Chain broken at {at}"), format!("Because {why}. Use the Radar again to start a new one."), theme::HP_RED);
            }
            _ => {}
        }
    }

    fn radar_notice(&mut self, title: &str, body: String, accent: eframe::egui::Color32) {
        self.notify(Notice {
            id: "radar",
            icon: "📡",
            title: title.to_string(),
            body,
            progress: None,
            action: None,
            expires: Some(Instant::now() + Duration::from_secs(6)),
            accent,
        });
    }

    /// The chain set back to 0 by hand (left the area, or a miscount).
    pub(super) fn radar_reset(&mut self) {
        clear(&mut self.cfg, false);
        self.radar_flash = None;
        self.dirty = true;
    }

    /// A chain that just broke, for the red flash: (length, why).
    pub(super) fn radar_broken_now(&self) -> Option<(u32, &str)> {
        self.radar_flash.as_ref().filter(|f| f.2.elapsed() < BROKEN_FLASH).map(|f| (f.0, f.1.as_str()))
    }

    /// At max chain on a Radar hunt: the count shows checks from here.
    pub(super) fn radar_checking(&self) -> bool {
        self.cfg.hunt_method == HuntMethod::PokeRadar && self.cfg.radar_maxed()
    }

    /// The overlay's chain section, during Poké Radar hunts.
    pub(super) fn radar_bar(&self) -> Option<RadarBar> {
        if self.cfg.hunt_method != HuntMethod::PokeRadar {
            return None;
        }
        let odds = |n: u32| super::fmt_one_in(Config::radar_one_in(n));
        let c = self.cfg.radar_chain;
        Some(if let Some((at, why)) = self.radar_broken_now() {
            RadarBar { label: format!("BROKEN AT {at} · {}", why_short(why)), filled: at.min(RADAR_MAX), look: BarLook::Broken, tip: format!("The chain broke at {at} because {why}. Use the Radar again to start a new one.") }
        } else if c >= RADAR_MAX {
            RadarBar { label: format!("★ MAX {c} · 1/{}", odds(c)), filled: RADAR_MAX, look: BarLook::Max, tip: format!("Max chain: odds are 1/{} for every patch. {} checks at max chain, best chain {}.", odds(c), self.cfg.radar_checks, self.cfg.radar_best) }
        } else if self.cfg.radar_on || c > 0 {
            RadarBar { label: format!("CHAIN {c}/{RADAR_MAX} · 1/{}", odds(c)), filled: c, look: BarLook::Building, tip: format!("Chain {c}: odds 1/{} now. They start improving after 20 and reach 1/{} at {RADAR_MAX}.", odds(c), odds(RADAR_MAX)) }
        } else {
            RadarBar { label: format!("RADAR OFF · 1/{}", odds(0)), filled: 0, look: BarLook::Off, tip: "Use the Poké Radar in game to start a chain.".to_string() }
        })
    }

    /// The chain controls under the Poké Radar method (Hunt page and
    /// Settings): the chain with − / +, Reset, and how the hunt has gone.
    pub(super) fn radar_controls(&mut self, ui: &mut eframe::egui::Ui) {
        use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
        use eframe::egui::RichText;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Current chain").size(kit::fs(13.0)).color(TEXT()));
            let before = self.cfg.radar_chain;
            if kit::stepper(ui, &mut self.cfg.radar_chain, 0, 999) {
                self.dirty = true;
                // fixing it by hand: the chain is going, with the same Pokémon
                if self.cfg.radar_chain > before {
                    self.cfg.radar_on = true;
                }
                self.cfg.radar_best = self.cfg.radar_best.max(self.cfg.radar_chain);
            }
            if ui.add(eframe::egui::Button::new("Reset").fill(kit::btn_fill())).on_hover_text("Chain back to 0 (left the area, or it miscounted).").clicked() {
                self.radar_reset();
            }
            kit::info(ui, "Counted from the game's log: use the Radar, then every battle with the same Pokémon that you knock out or catch is +1. Running, a different Pokémon or a double battle breaks it. Odds stop improving at 40.");
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let (txt, kind) = if self.cfg.radar_chain >= RADAR_MAX {
                (format!("★ Max chain · 1/{} · {} checks", super::fmt_one_in(Config::radar_one_in(self.cfg.radar_chain)), self.cfg.radar_checks), PillKind::Ok)
            } else if self.cfg.radar_on {
                (format!("Radar on · odds 1/{}", super::fmt_one_in(Config::radar_one_in(self.cfg.radar_chain))), PillKind::Ok)
            } else {
                ("Radar off · use it in game to start a chain".to_string(), PillKind::Warn)
            };
            kit::pill(ui, &txt, kind);
            ui.label(RichText::new(format!("Best {} · broken {}", self.cfg.radar_best, self.cfg.radar_breaks)).size(kit::fs(11.5)).color(MUTED()));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamelog::RadarWatch;

    fn run(log: &str) -> (Config, Vec<Step>) {
        let mut c = Config::default();
        c.hunt_method = HuntMethod::PokeRadar;
        let mut w = RadarWatch::default();
        let name = |d: u32| format!("#{d}");
        let steps = log.lines().filter_map(|l| w.push(l)).map(|ev| step(&mut c, &ev, &name)).filter(|s| *s != Step::Nothing).collect();
        (c, steps)
    }

    fn battle(t: &str, species: u32, end: &str) -> String {
        let end_line = match end {
            "ko" => format!("{t} INFO  FAINT    battler 1 state 0\n"),
            "catch" => format!("{t} INFO  CATCH    caught battler 1 slot 0 species {species}\n"),
            _ => String::new(),
        };
        format!("{t} INFO  WILD     shiny chance 2 in 8192 for species {species} at level 17\n{t} INFO  BATTLE   encounter effect took 169 ms\n{end_line}{t} INFO  BATTLE   HUD fade 16 towards 0000\n")
    }

    #[test]
    fn your_real_chain_counts_to_seven_then_breaks() {
        let mut log = String::from("08:20:03 INFO  UI       bag page: use item 431 (request 20431)\n");
        log += &battle("08:20:05", 315, "ran");
        log += "08:20:33 INFO  UI       bag page: use item 431 (request 20431)\n";
        for _ in 0..7 {
            log += &battle("08:21:00", 280, "ko");
        }
        log += &battle("08:22:37", 400, "ko");
        let (c, steps) = run(&log);
        assert_eq!(steps[0], Step::On);
        // the Roselia it ran from was chain 0: nothing to break
        assert_eq!(steps[1], Step::On);
        assert_eq!(&steps[2..9], &(1..=7).map(Step::Up).collect::<Vec<_>>()[..]);
        assert_eq!(steps[9], Step::Broke(7, "a #400 showed up".into()));
        assert_eq!((c.radar_chain, c.radar_best, c.radar_breaks, c.radar_on), (0, 7, 1, false));
    }

    #[test]
    fn catches_keep_it_running_away_breaks_it() {
        let mut log = String::from("08:00:00 INFO  UI       Y wheel: used item 431 (1)\n");
        log += &battle("08:00:01", 280, "ko");
        log += &battle("08:00:02", 280, "catch");
        log += &battle("08:00:03", 280, "ran");
        let (c, steps) = run(&log);
        assert_eq!(steps, vec![Step::On, Step::Up(1), Step::Up(2), Step::Broke(2, "you ran".into())]);
        assert_eq!(c.radar_chain, 0);
    }

    #[test]
    fn no_radar_no_chain_and_checks_from_forty() {
        // battles before the Radar is used don't count
        let (c, steps) = run(&battle("08:00:00", 280, "ko"));
        assert!(steps.is_empty());
        assert_eq!(c.radar_chain, 0);
        // at max chain every Radar battle is a check, even one that breaks it
        let mut log = String::from("08:00:00 INFO  UI       bag page: use item 431 (request 20431)\n");
        for _ in 0..42 {
            log += &battle("08:00:01", 280, "ko");
        }
        log += &battle("08:00:02", 280, "ran");
        let (c, _) = run(&log);
        assert_eq!((c.radar_best, c.radar_checks, c.radar_chain), (42, 3, 0));
    }

    #[test]
    fn odds_by_chain() {
        assert_eq!(Config::radar_one_in(0), 4096.0);
        assert_eq!(Config::radar_one_in(20), 4096.0);
        assert!((Config::radar_one_in(23) - 3600.0).abs() < 1.0);
        assert!((Config::radar_one_in(40) - 200.0).abs() < 1.0);
        assert!((Config::radar_one_in(99) - 200.0).abs() < 1.0);
    }
}

#[cfg(test)]
mod real_log {
    use super::*;
    use crate::gamelog::RadarWatch;

    /// Your Poké Radar test on 15 Nov: replayed line by line.
    #[test]
    #[ignore]
    fn replay_real_session() {
        let path = "/mnt/user-data/uploads/PlatinumTogether/Game/logs/session-20261115-080313.log";
        let Ok(log) = std::fs::read_to_string(path) else { return };
        let mut c = Config::default();
        c.hunt_method = HuntMethod::PokeRadar;
        let mut w = RadarWatch::default();
        let name = |d: u32| format!("#{d}");
        for l in log.lines() {
            if let Some(ev) = w.push(l) {
                let st = step(&mut c, &ev, &name);
                println!("{} {:?} -> {:?} (chain {})", &l[..8.min(l.len())], ev, st, c.radar_chain);
            }
        }
        println!("best {} breaks {} checks {}", c.radar_best, c.radar_breaks, c.radar_checks);
    }
}
