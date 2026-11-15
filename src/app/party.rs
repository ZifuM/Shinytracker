//! Your party, in the title bar. The game doesn't write the party live, so
//! it's worked out:
//!
//! - `world-card.ini`, which the game writes when it opens, gives the party
//!   then (the game's species numbers, in slot order);
//! - the log's box moves ("BOXES move 881 -> 3": 0-5 are party slots,
//!   bigger numbers box places) swap Pokémon in and out, and catches join a
//!   party that has room;
//! - a Pokémon taken out of a box is named if the app saw it go in, else from
//!   the picture the box page loaded just before (a guess), else "?".
//!
//! Each time the game opens, the file puts it right again. Any slot can be
//! set by hand.

use super::App;
use crate::config::{BoxSpot, Config, PartySlot};
use crate::gamelog::PartyEvent;
use crate::species::Species;
use crate::theme;
use eframe::egui::{self, Rect, Sense, Stroke};


/// The game's species number → species slug, if known.
pub(crate) fn resolve(cfg: &Config, species: &[Species], id: u32) -> Option<String> {
    if id == 0 {
        return None;
    }
    if let Some(nat) = crate::species::national_from_game(id) {
        return species.iter().find(|s| s.dex == nat).map(|s| s.slug.clone());
    }
    // Past the end of the list (a newer game update): what the user named it.
    cfg.game_dex.get(&id).cloned()
}

fn slot_of(cfg: &Config, species: &[Species], id: u32) -> PartySlot {
    PartySlot { slug: resolve(cfg, species, id), game_id: id, ..Default::default() }
}

/// A Pokémon from the save as a party slot (or a box place).
fn slot_from_save(cfg: &Config, species: &[Species], m: &crate::gamesave::Mon) -> PartySlot {
    PartySlot { slug: if m.egg { None } else { resolve(cfg, species, m.species as u32) }, game_id: m.species as u32, guess: false, pid: m.pid, shiny: m.shiny(), level: m.level.unwrap_or(0), egg: m.egg }
}

/// The log's box places, as the save lays them out: Box 1's first place is
/// 100, each box 30 on (worked out from your box 27, places 880-885).
pub(crate) fn spot_of(b: usize, s: usize) -> u32 {
    100 + 30 * b as u32 + s as u32
}

/// The party and box places as a save has them. Its box places are marked as
/// a guess for the log's moves (the save can be a few minutes old).
pub(crate) fn set_from_save(cfg: &mut Config, species: &[Species], save: &crate::gamesave::Save) {
    cfg.party = save.party.iter().map(|m| slot_from_save(cfg, species, m)).collect();
    let mut spots = std::collections::HashMap::new();
    for (b, bx) in save.boxes.iter().enumerate() {
        for (s, m) in bx.slots.iter().enumerate() {
            let v = match m {
                Some(m) => BoxSpot::Mon(PartySlot { guess: true, ..slot_from_save(cfg, species, m) }),
                None => BoxSpot::Empty,
            };
            spots.insert(spot_of(b, s), v);
        }
    }
    cfg.box_spots = spots;
}

/// The party as `world-card.ini` gives it (the game opening). `ms` the
/// same as before: the app was restarted, so the box places go back to what
/// they were then (this session's moves are read again).
pub(crate) fn apply_card(cfg: &mut Config, species: &[Species], party: &[u32], ms: u64) {
    if ms == cfg.party_card_ms {
        cfg.box_spots = cfg.box_spots_base.clone();
    } else {
        cfg.party_card_ms = ms;
        cfg.box_spots_base = cfg.box_spots.clone();
    }
    cfg.party_card = party.to_vec();
    cfg.party = party.iter().map(|&id| slot_of(cfg, species, id)).collect();
}

/// Seconds into the (local) day of a unix-ms time.
fn day_secs(ms: u64) -> u32 {
    ((ms / 1000) as i64 + crate::stats::utc_offset_secs()).rem_euclid(86_400) as u32
}

/// One log event. Moves swap; a move to an empty party slot goes to the end
/// (the game keeps the party together).
pub(crate) fn apply_event(cfg: &mut Config, species: &[Species], ev: &PartyEvent) {
    match ev {
        PartyEvent::Named { species: id, name } => {
            if let Some(i) = crate::worker::species_by_log_name(species, name) {
                let slug = species[i].slug.clone();
                cfg.game_dex.insert(*id, slug.clone());
                for s in cfg.party.iter_mut().filter(|s| s.game_id == *id && s.slug.is_none()) {
                    s.slug = Some(slug.clone());
                }
            }
        }
        PartyEvent::Caught { species: id } => {
            if cfg.party.len() < 6 {
                let s = slot_of(cfg, species, *id);
                cfg.party.push(s);
            }
        }
        PartyEvent::Move { from, to, seen } => {
            let (a, b) = (*from, *to);
            let party = |i: u32| i < 6;
            match (party(a), party(b)) {
                (true, true) => {
                    let (a, b) = (a as usize, b as usize);
                    let n = cfg.party.len();
                    if a < n && b < n {
                        cfg.party.swap(a, b);
                    } else if a < n {
                        let m = cfg.party.remove(a);
                        cfg.party.push(m);
                    }
                }
                (false, true) => {
                    // out of a box: known if the app saw it go in, else the
                    // one new picture the box page loaded (not one already
                    // in the party), else "?"
                    let incoming = match cfg.box_spots.get(&a) {
                        Some(BoxSpot::Empty) => return,
                        Some(BoxSpot::Mon(m)) => m.clone(),
                        None => guess_from(cfg, species, seen),
                    };
                    let slot = b as usize;
                    if slot < cfg.party.len() {
                        let out = std::mem::replace(&mut cfg.party[slot], incoming);
                        cfg.box_spots.insert(a, BoxSpot::Mon(out));
                    } else {
                        cfg.party.push(incoming);
                        cfg.box_spots.insert(a, BoxSpot::Empty);
                    }
                }
                (true, false) => {
                    let slot = a as usize;
                    if slot >= cfg.party.len() {
                        return;
                    }
                    match cfg.box_spots.get(&b).cloned() {
                        // put away into an empty place: the party gets shorter
                        Some(BoxSpot::Empty) => {
                            let out = cfg.party.remove(slot);
                            cfg.box_spots.insert(b, BoxSpot::Mon(out));
                        }
                        // swapped with what was there (named if the app knows it)
                        known => {
                            let incoming = match known {
                                Some(BoxSpot::Mon(m)) => m,
                                _ => guess_from(cfg, species, seen),
                            };
                            let out = std::mem::replace(&mut cfg.party[slot], incoming);
                            cfg.box_spots.insert(b, BoxSpot::Mon(out));
                        }
                    }
                }
                (false, false) => {
                    let pa = cfg.box_spots.remove(&a);
                    let pb = cfg.box_spots.remove(&b);
                    if let Some(x) = pa {
                        cfg.box_spots.insert(b, x);
                    }
                    if let Some(x) = pb {
                        cfg.box_spots.insert(a, x);
                    }
                }
            }
        }
    }
}

/// The one picture the box page loaded that isn't already in the party.
fn guess_from(cfg: &Config, species: &[Species], seen: &[String]) -> PartySlot {
    let in_party: Vec<&str> = cfg.party.iter().filter_map(|s| s.slug.as_deref()).collect();
    let fresh: Vec<usize> = seen
        .iter()
        .filter_map(|n| crate::worker::species_by_log_name(species, n))
        .filter(|&i| !in_party.contains(&species[i].slug.as_str()))
        .collect();
    match fresh.as_slice() {
        [i] => PartySlot { slug: Some(species[*i].slug.clone()), guess: true, ..Default::default() },
        _ => PartySlot::default(),
    }
}

impl App {
    pub(super) fn party_card(&mut self, party: Vec<u32>, ms: u64) {
        let species = self.species.clone();
        apply_card(&mut self.cfg, &species, &party, ms);
        self.party_recompute();
    }

    pub(super) fn party_event(&mut self, ev: PartyEvent, at: u32) {
        if let PartyEvent::Named { .. } = ev {
            let species = self.species.clone();
            apply_event(&mut self.cfg, &species, &ev);
        } else {
            self.setup.party_events.push((at, ev));
        }
        self.party_recompute();
    }

    /// A save newer than the one the party came from.
    pub(super) fn party_save_read(&mut self, ms: u64) {
        if ms > self.cfg.party_save_ms {
            self.cfg.party_save_ms = ms;
        }
        self.party_recompute();
    }

    /// The party again from its newest starting point (the game opening, or a
    /// save written after it) and this session's moves since then.
    fn party_recompute(&mut self) {
        let species = self.species.clone();
        let save_base = self.cfg.party_save_ms >= self.cfg.party_card_ms && self.cfg.party_save_ms > 0;
        let base_ms = match (save_base, self.setup.boxes.save.as_ref()) {
            (true, Some(save)) if !save.party.is_empty() => {
                set_from_save(&mut self.cfg, &species, save);
                self.cfg.party_save_ms
            }
            _ if !self.cfg.party_card.is_empty() => {
                let card = self.cfg.party_card.clone();
                self.cfg.party = card.iter().map(|&id| slot_of(&self.cfg, &species, id)).collect();
                self.cfg.box_spots = self.cfg.box_spots_base.clone();
                self.cfg.party_card_ms
            }
            // nothing to start from yet: keep what was shown last time
            _ => return,
        };
        let from = day_secs(base_ms);
        let events = self.setup.party_events.clone();
        for (at, ev) in events.iter().filter(|(at, _)| *at >= from) {
            let _ = at;
            apply_event(&mut self.cfg, &species, ev);
        }
        self.dirty = true;
    }

    /// The party's pictures, for the title bar (right to left layout).
    pub(super) fn party_strip(&mut self, ui: &mut egui::Ui) {
        if !self.cfg.show_party || self.cfg.party.is_empty() {
            return;
        }
        let n = self.cfg.party.len();
        let cell = 30.0;
        let (r, _) = ui.allocate_exact_size(egui::vec2(cell * n as f32 + 4.0, 36.0), Sense::hover());
        let p = ui.painter().clone();
        let since = if self.cfg.party_card_ms > 0 { time_of(self.cfg.party_card_ms) } else { String::new() };
        let mut pick: Option<usize> = None;
        for i in 0..n {
            let slot = self.cfg.party[i].clone();
            let c = egui::pos2(r.min.x + 2.0 + cell * (i as f32 + 0.5), r.center().y);
            let cr = Rect::from_center_size(c, egui::vec2(cell - 2.0, cell));
            let resp = ui.interact(cr, ui.id().with(("party", i)), Sense::click());
            let hov = resp.hovered();
            p.circle_filled(c, 13.0, theme::blend(super::ui_kit::PAGE(), super::ui_kit::TEXT(), if hov { 0.14 } else { 0.07 }));
            let sp = slot.slug.as_deref().and_then(|s| self.species.iter().find(|x| x.slug == s)).cloned();
            let egg = slot.egg.then(|| self.setup.boxes.save.as_ref()?.party.iter().find(|m| m.pid == slot.pid).cloned()).flatten();
            let name = if let Some(m) = &egg {
                // an Egg, filling up as it gets closer to hatching
                let (frac, _) = self.egg_progress(m);
                let ink = super::ui_kit::MUTED();
                super::eggs::paint_egg(&p, c, 20.0, frac, theme::blend(super::ui_kit::PAGE(), ink, 0.25), theme::GOLD, ink);
                "Egg".to_string()
            } else {
            match &sp {
                Some(sp) => {
                    // (while its picture loads: its first letters, not "no sprite")
                    if self.sprites.get(sp, slot.shiny).is_some() {
                        self.draw_sprite_centered(&p, sp, slot.shiny, c, egui::vec2(28.0, 28.0));
                    } else {
                        let short: String = sp.name.chars().take(2).collect();
                        p.text(c, egui::Align2::CENTER_CENTER, short, crate::fonts::sans_font(11.0), super::ui_kit::TEXT());
                    }
                    sp.name.to_string()
                }
                None => {
                    p.text(c, egui::Align2::CENTER_CENTER, "?", crate::fonts::sans_font(13.0), super::ui_kit::MUTED());
                    if slot.game_id > 0 { format!("Not sure (the game's #{})", slot.game_id) } else { "Not sure".to_string() }
                }
            }
            };
            if slot.guess {
                p.circle_filled(egui::pos2(cr.max.x - 3.0, cr.min.y + 5.0), 3.0, theme::HP_YELLOW);
                p.circle_stroke(egui::pos2(cr.max.x - 3.0, cr.min.y + 5.0), 3.0, Stroke::new(1.0f32, super::ui_kit::PAGE()));
            }
            if slot.shiny {
                p.text(egui::pos2(cr.max.x - 3.0, cr.min.y + 5.0), egui::Align2::CENTER_CENTER, "★", crate::fonts::sans_bold_font(10.0), theme::GOLD);
            }
            let tip = self.party_tip(i, &slot, &name, &since);
            if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                pick = Some(i);
            }
        }
        if let Some(i) = pick {
            // from the save: its details in Pokémon Boxes (if installed)
            let from_save = self.cfg.party.get(i).is_some_and(|s| s.pid != 0);
            if from_save && crate::apps::live(crate::apps::AppId::Boxes) {
                self.setup.boxes.sel = Some(super::boxes_app::Sel::Party(i));
                self.go_to(super::Tab::Boxes);
            } else {
                self.setup.party_pick = Some((i, String::new()));
            }
        }
    }

    /// A slot's hover text: everything the save says about it, or where the
    /// slot came from.
    fn party_tip(&self, i: usize, slot: &PartySlot, name: &str, since: &str) -> String {
        let mon = (slot.pid != 0).then(|| self.setup.boxes.save.as_ref()?.party.iter().find(|m| m.pid == slot.pid)).flatten();
        let mut t = String::new();
        match mon {
            Some(m) if m.egg => {
                let (frac, steps) = self.egg_progress(m);
                let what = self.box_species(m.species).map_or(String::new(), |s| format!(" ({})", s.name));
                t.push_str(&format!("Slot {}: Egg{what} · {}% · {}", i + 1, (frac * 100.0).round(), super::eggs::steps_text(steps)));
                t.push_str("\nFrom the game's last save (read only).");
            }
            Some(m) => {
                let text = self.setup.boxes.text.clone().unwrap_or_default();
                let nick = if m.nicknamed && !m.nickname.is_empty() && m.nickname.to_lowercase() != name.to_lowercase() { format!(" \"{}\"", m.nickname) } else { String::new() };
                t.push_str(&format!("Slot {}: {name}{nick}{}", i + 1, if m.shiny() { " ★ shiny" } else { "" }));
                if let Some(l) = m.level {
                    t.push_str(&format!("\nLv {l} · {} · {}", crate::gamesave::NATURES[m.nature() as usize], text.ability_name(m.ability)));
                }
                if m.item != 0 {
                    t.push_str(&format!("\nHolding {}", text.item_name(m.item)));
                }
                let moves: Vec<String> = m.moves.iter().filter(|x| **x != 0).map(|x| text.move_name(*x)).collect();
                if !moves.is_empty() {
                    t.push_str(&format!("\n{}", moves.join(" · ")));
                }
                let iv = [m.ivs[0], m.ivs[1], m.ivs[2], m.ivs[4], m.ivs[5], m.ivs[3]];
                t.push_str(&format!("\nIVs {}", iv.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("/")));
                t.push_str("\nFrom your last save (read only).");
                if crate::apps::live(crate::apps::AppId::Boxes) {
                    t.push_str(" Click for everything in Pokémon Boxes.");
                } else {
                    t.push_str(" Click to set this slot by hand.");
                }
            }
            None => {
                t.push_str(&format!("Slot {}: {name}", i + 1));
                if slot.guess {
                    t.push_str(" (a guess)");
                }
                if !since.is_empty() {
                    t.push_str(&format!("\nYour party when the game opened ({since}), kept up to date from the game's log."));
                }
                t.push_str("\nClick to set this slot by hand.");
            }
        }
        t
    }

    /// The "set this slot" box, over the page.
    pub(super) fn party_pick_window(&mut self, ctx: &egui::Context) {
        let Some((slot, mut q)) = self.setup.party_pick.clone() else { return };
        if slot >= self.cfg.party.len() {
            self.setup.party_pick = None;
            return;
        }
        let mut open = true;
        let mut chosen: Option<Option<String>> = None;
        egui::Window::new(format!("Party slot {}", slot + 1))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-180.0, 60.0))
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Which Pokémon is in this slot?").size(12.5).color(super::ui_kit::MUTED()));
                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut q).hint_text("Search").desired_width(f32::INFINITY)).request_focus();
                ui.add_space(4.0);
                let ql = q.trim().to_lowercase();
                if !ql.is_empty() {
                    let hits: Vec<(String, &'static str)> = self
                        .species
                        .iter()
                        .filter(|s| s.name.to_lowercase().starts_with(&ql) || s.dex.to_string() == ql)
                        .take(8)
                        .map(|s| (s.slug.clone(), s.name))
                        .collect();
                    for (slug, name) in hits {
                        if ui.button(name).clicked() {
                            chosen = Some(Some(slug));
                        }
                    }
                }
                ui.add_space(6.0);
                if ui.button("Not sure (?)").clicked() {
                    chosen = Some(None);
                }
            });
        if let Some(c) = chosen {
            let s = &mut self.cfg.party[slot];
            // a number past #493 named by hand: remembered for next time
            if let (Some(slug), true) = (&c, s.game_id > 493) {
                self.cfg.game_dex.insert(s.game_id, slug.clone());
            }
            s.slug = c;
            s.guess = false;
            self.dirty = true;
            self.setup.party_pick = None;
        } else if !open {
            self.setup.party_pick = None;
        } else {
            self.setup.party_pick = Some((slot, q));
        }
    }
}

/// The time of day for a unix-ms time, in local time.
fn time_of(ms: u64) -> String {
    crate::stats::fmt_clock(ms / 1000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamelog::PartyWatch;

    fn species() -> Vec<Species> {
        crate::species::all()
    }

    fn names(cfg: &Config, sp: &[Species]) -> Vec<String> {
        cfg.party.iter().map(|s| s.slug.as_deref().and_then(|x| sp.iter().find(|y| y.slug == x)).map_or("?".to_string(), |y| y.name.to_string())).collect()
    }

    fn run(cfg: &mut Config, sp: &[Species], log: &str) {
        let mut w = PartyWatch::default();
        for l in log.lines() {
            if let Some(e) = w.push(l.trim()) {
                apply_event(cfg, sp, &e);
            }
        }
    }

    /// Your test this morning: Swampert, Sceptile, 3 Eevee, Vespiquen, then
    /// three out of box 27 and two swaps.
    #[test]
    fn your_party_test() {
        let sp = species();
        let mut c = Config::default();
        apply_card(&mut c, &sp, &[260, 254, 133, 133, 133, 416], 1);
        assert_eq!(names(&c, &sp), ["Swampert", "Sceptile", "Eevee", "Eevee", "Eevee", "Vespiquen"]);
        run(
            &mut c,
            &sp,
            "09:14:46 INFO  UI       box page: open on box 27
             09:14:46 INFO  GEN5     VULPIX/0 ready in 0.4 ms (36 frames, 16 pictures, 59x59)
             09:14:50 INFO  GEN5     VESPIQUEN/0 ready in 2.4 ms (112 frames, 62 pictures, 79x79)
             09:14:50 INFO  BOXES    move 880 -> 5: done
             09:14:51 INFO  UI       box page: closed on box 27
             09:15:06 INFO  UI       box page: open on box 27
             09:15:08 INFO  BOXES    move 5 -> 880: done
             09:15:09 INFO  UI       box page: closed on box 27",
        );
        assert_eq!(names(&c, &sp), ["Swampert", "Sceptile", "Eevee", "Eevee", "Eevee", "Vespiquen"]);
        run(
            &mut c,
            &sp,
            "09:17:26 INFO  UI       box page: open on box 27
             09:17:27 INFO  GEN5     TALONFLAME/0 ready in 0.6 ms (80 frames, 9 pictures, 128x128)
             09:17:28 INFO  BOXES    move 881 -> 3: done
             09:17:29 INFO  GEN5     GRENINJA/2 ready in 0.8 ms (24 frames, 24 pictures, 96x96)
             09:17:29 INFO  GEN5     INFERNAPE/0 ready in 1.1 ms (66 frames, 32 pictures, 80x80)
             09:17:29 INFO  BOXES    move 884 -> 2: done
             09:17:30 INFO  GEN5     GARCHOMP/0 ready in 1.2 ms (44 frames, 25 pictures, 82x82)
             09:17:30 INFO  BOXES    move 885 -> 4: done
             09:17:31 INFO  BOXES    move 5 -> 3: done
             09:17:32 INFO  BOXES    move 2 -> 5: done
             09:17:36 INFO  UI       box page: closed on box 27",
        );
        // Greninja vs Infernape can't be told apart: "?"
        assert_eq!(names(&c, &sp), ["Swampert", "Sceptile", "Talonflame", "Vespiquen", "Garchomp", "?"]);
        // the Eevees are remembered in their box places
        assert!(matches!(c.box_spots.get(&881), Some(BoxSpot::Mon(m)) if m.slug.as_deref() == Some("eevee")));
        // taking one back out names it for sure
        run(&mut c, &sp, "10:00:00 INFO  UI       box page: open on box 27\n10:00:01 INFO  BOXES    move 881 -> 0: done");
        assert_eq!(names(&c, &sp)[0], "Eevee");
        assert!(!c.party[0].guess);
    }

    #[test]
    fn restart_reads_the_session_again_from_the_same_place() {
        let sp = species();
        let mut c = Config::default();
        apply_card(&mut c, &sp, &[260, 133], 5);
        let log = "09:00:00 INFO  UI       box page: open on box 1\n09:00:01 INFO  GEN5     PIKACHU/0 ready in 1 ms\n09:00:02 INFO  BOXES    move 900 -> 1: done";
        run(&mut c, &sp, log);
        let once = c.party.clone();
        // app restarted: same file, the same log read again
        apply_card(&mut c, &sp, &[260, 133], 5);
        run(&mut c, &sp, log);
        assert_eq!(c.party, once);
        assert_eq!(names(&c, &sp), ["Swampert", "Pikachu"]);
    }

    #[test]
    fn catches_join_a_party_with_room_and_numbers_past_493() {
        let sp = species();
        let mut c = Config::default();
        apply_card(&mut c, &sp, &[544, 133, 133, 133], 1);
        assert_eq!(names(&c, &sp)[0], "Talonflame");
        run(&mut c, &sp, "09:06:37 INFO  CATCH    caught battler 3 slot 1 species 37");
        assert_eq!(names(&c, &sp).len(), 5);
        // a number the app doesn't know yet, named later by the log
        apply_card(&mut c, &sp, &[700], 2);
        assert_eq!(names(&c, &sp), ["?"]);
        run(&mut c, &sp, "10:00:00 INFO  WILD     shiny chance 2 in 8192 for species 700 at level 5\n10:00:01 INFO  GEN5     DEDENNE/0 ready in 1 ms");
        assert_eq!(names(&c, &sp), ["Dedenne"]);
    }

    /// Your save, if it's here: it fills the party with full details, and
    /// box 27's places line up with the log's 880-885.
    #[test]
    fn party_from_your_save() {
        let p = std::path::Path::new("/mnt/user-data/uploads/PlatinumTogether/Game/save.bin");
        let Ok(save) = crate::gamesave::read(p) else { return };
        let sp = species();
        let mut c = Config::default();
        set_from_save(&mut c, &sp, &save);
        assert_eq!(c.party.len(), 6);
        assert_eq!(names(&c, &sp)[0], "Swampert");
        assert!(c.party.iter().all(|s| s.pid != 0));
        assert!(c.party[5].shiny, "your shiny Greninja");
        // box 27 (the 27th box) place 1 = the log's 881
        assert_eq!(spot_of(26, 1), 881);
        assert!(matches!(c.box_spots.get(&spot_of(26, 1)), Some(BoxSpot::Mon(_))));
        // a move after the save: taking box 27's first Pokémon into slot 1
        let before = match c.box_spots.get(&880) { Some(BoxSpot::Mon(m)) => m.slug.clone(), _ => None };
        apply_event(&mut c, &sp, &PartyEvent::Move { from: 880, to: 0, seen: vec![] });
        assert_eq!(c.party[0].slug, before);
        assert!(c.party[0].guess, "from the save's box, which may be minutes old");
    }

    #[test]
    fn reads_world_card() {
        let d = std::env::temp_dir().join(format!("wc-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("world-card.ini");
        std::fs::write(&f, "Zifu\t8\t104:07\tRoute 209\t0\t260.0.0,254.0.0,133.0.1,133.0.1,133.0.1,416.0.0\t0\n").unwrap();
        assert_eq!(crate::gamelog::read_world_card(&f), Some(vec![260, 254, 133, 133, 133, 416]));
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[cfg(test)]
mod real_log {
    use super::*;
    use crate::gamelog::PartyWatch;

    /// Replay a real session from its opening party (PARTY_LOG, PARTY_CARD="544,133").
    #[test]
    #[ignore]
    fn replay_real_session() {
        let (Ok(path), Ok(card)) = (std::env::var("PARTY_LOG"), std::env::var("PARTY_CARD")) else { return };
        let sp = crate::species::all();
        let mut c = Config::default();
        let ids: Vec<u32> = card.split(',').filter_map(|x| x.parse().ok()).collect();
        apply_card(&mut c, &sp, &ids, 1);
        let mut w = PartyWatch::default();
        for l in std::fs::read_to_string(path).unwrap().lines() {
            if let Some(e) = w.push(l) {
                apply_event(&mut c, &sp, &e);
                if !matches!(e, PartyEvent::Named { .. }) {
                    let n: Vec<String> = c.party.iter().map(|s| format!("{}{}", s.slug.clone().unwrap_or("?".into()), if s.guess { "*" } else { "" })).collect();
                    println!("{} {:?} -> {}", &l[..8], e, n.join(", "));
                }
            }
        }
    }
}
