//! Eggs from the save: each Egg you carry and how close it is to hatching
//! (on the overlay, in every theme and size), and the Masuda helper on the
//! Hunt page (your Day-Care pair, whether it counts for the Masuda Method).
//! READ ONLY: the save is only ever read.

use super::ov_common::{look, Look};
use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
use super::App;
use crate::config::OverlayTheme;
use crate::gamesave::Mon;
use crate::species::Species;
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke};

/// One Egg you carry.
#[derive(Clone)]
pub(super) struct EggNow {
    pub sp: Option<Species>,
    /// 0..1, how far along.
    pub frac: f32,
    /// About how many steps are left.
    pub steps: u32,
}

/// The Eggs you carry, as of the game's last save.
#[derive(Clone)]
pub(super) struct EggsNow {
    pub eggs: Vec<EggNow>,
    /// 1, 2 (Flame Body or the Egg Warmer) or 4 (both).
    pub speed: u32,
    pub flame_body: bool,
    pub warmer: bool,
    /// How long ago the game saved.
    pub ago_secs: u64,
}

impl EggsNow {
    /// The nearest to hatching.
    pub fn next_steps(&self) -> u32 {
        self.eggs.iter().map(|e| e.steps).min().unwrap_or(0)
    }

    pub fn tip(&self) -> String {
        let mut t = String::new();
        for e in &self.eggs {
            let name = e.sp.as_ref().map_or("An", |s| s.name);
            t.push_str(&format!("{name} Egg: {}% · {}\n", (e.frac * 100.0).round(), steps_text(e.steps)));
        }
        let mut why = Vec::new();
        if self.flame_body {
            why.push("Flame Body");
        }
        if self.warmer {
            why.push("the Egg Warmer");
        }
        if !why.is_empty() {
            t.push_str(&format!("{} times as fast with {}.\n", self.speed, why.join(" and ")));
        }
        t.push_str(&format!("As of the game's save {}. The game saves itself every few minutes.", ago_text(self.ago_secs)));
        t
    }
}

pub(super) fn steps_text(steps: u32) -> String {
    if steps <= crate::breed::CYCLE_STEPS {
        "hatching any step now".to_string()
    } else {
        format!("about {} steps", theme::thousands(steps as u64))
    }
}

fn short_steps(steps: u32) -> String {
    if steps <= crate::breed::CYCLE_STEPS {
        "Soon!".to_string()
    } else if steps >= 10_000 {
        format!("{:.1}k", steps as f32 / 1000.0)
    } else {
        theme::thousands(steps as u64)
    }
}

fn ago_text(secs: u64) -> String {
    match secs {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", secs / 60),
        _ => format!("{}h ago", secs / 3600),
    }
}

/// The language a Pokémon came from.
pub(super) fn language(l: u8) -> &'static str {
    match l {
        1 => "Japanese",
        2 => "English",
        3 => "French",
        4 => "Italian",
        5 => "German",
        7 => "Spanish",
        8 => "Korean",
        _ => "Unknown",
    }
}

// Abilities that hatch Eggs twice as fast (the game's own numbers).
const MAGMA_ARMOR: u8 = 40;
const FLAME_BODY: u8 = 49;

/// Full / Compact strip heights.
pub(super) const STRIP_FULL_H: f32 = 80.0;
pub(super) const STRIP_COMPACT_H: f32 = 56.0;
pub(super) const STRIP_GAP: f32 = 6.0;

impl App {
    /// How far along an Egg is (0..1) and about how many steps are left
    /// (Flame Body / Magma Armor in the party and the Egg Warmer counted).
    pub(super) fn egg_progress(&self, m: &Mon) -> (f32, u32) {
        let st = &self.setup.boxes;
        let (flame_body, warmer) = match st.save.as_ref() {
            Some(s) => (
                s.party.iter().any(|x| !x.egg && (x.ability == FLAME_BODY || x.ability == MAGMA_ARMOR)),
                s.bag.iter().any(|(_, item, _)| st.text.as_ref().is_some_and(|t| t.item_name(*item) == "Egg Warmer")),
            ),
            None => (false, false),
        };
        let speed = if flame_body { 2 } else { 1 } * if warmer { 2 } else { 1 };
        let total = self
            .box_species(m.species)
            .and_then(|s| self.wiki.as_ref()?.by_dex(s.dex).map(|w| w.id.clone()))
            .and_then(|id| crate::breed::info(&id).map(|i| i.cycles))
            .unwrap_or(20)
            .max(m.friendship as u16)
            .max(1);
        let left = m.friendship as u32;
        (1.0 - left as f32 / total as f32, (left * crate::breed::CYCLE_STEPS).div_ceil(speed))
    }

    /// The Eggs you carry (None: none, or no save read yet).
    pub(super) fn eggs_now(&self) -> Option<EggsNow> {
        let st = &self.setup.boxes;
        let s = st.save.as_ref()?;
        let text = st.text.as_ref();
        let flame_body = s.party.iter().any(|m| !m.egg && (m.ability == FLAME_BODY || m.ability == MAGMA_ARMOR));
        let warmer = s.bag.iter().any(|(_, item, _)| text.is_some_and(|t| t.item_name(*item) == "Egg Warmer"));
        let speed = if flame_body { 2 } else { 1 } * if warmer { 2 } else { 1 };
        let mut eggs: Vec<EggNow> = s
            .party
            .iter()
            .filter(|m| m.egg)
            .map(|m| {
                let (frac, steps) = self.egg_progress(m);
                EggNow { sp: self.box_species(m.species), frac, steps }
            })
            .collect();
        // the log saw some hatch since the save: leave off the nearest ones
        for _ in 0..st.hatched.min(eggs.len()) {
            if let Some(i) = eggs.iter().enumerate().min_by_key(|(_, e)| e.steps).map(|(i, _)| i) {
                eggs.remove(i);
            }
        }
        if eggs.is_empty() {
            return None;
        }
        let now = crate::config::now_unix() * 1000;
        Some(EggsNow { eggs, speed, flame_body, warmer, ago_secs: now.saturating_sub(st.save_ms) / 1000 })
    }

    /// The Eggs for the overlay (its Egg section on, and Eggs to show).
    pub(super) fn ov_eggs(&self) -> Option<EggsNow> {
        // Eggs are for egg hunts: only with the Masuda Method
        if !self.cfg.overlay.eggs || self.cfg.hunt_method != crate::config::HuntMethod::Masuda {
            return None;
        }
        self.eggs_now()
    }

    /// The strip under the Full / Compact overlay, in its theme. `r` is
    /// the strip's whole area.
    pub(super) fn egg_strip(&mut self, ui: &mut egui::Ui, p: &Painter, r: Rect, theme: OverlayTheme, compact: bool, eggs: &EggsNow) {
        let classic = theme == OverlayTheme::Classic;
        let lk: Look = look(theme);
        let (text, muted, fill) = if classic { (Color32::WHITE, theme::MUTED(), GOLD) } else { (lk.text, lk.muted, (lk.tone)(lk.accent)) };
        if classic {
            p.rect_filled(r, 12.0, theme::BG().gamma_multiply(self.cfg.opacity));
            p.rect_stroke(r, 12.0, Stroke::new(1.5f32, theme::OUTLINE));
        } else {
            (lk.panel)(p, r, self.cfg.opacity, false);
        }
        let label = |s: f32| if classic { crate::fonts::pixel_font(s * 0.75) } else { (lk.label)(s) };
        let body = |s: f32| if classic { crate::fonts::sans_bold_font(s) } else { (lk.body)(s) };
        let pad = 12.0;
        let n = eggs.eggs.len();
        let mut y = r.min.y;
        if !compact {
            p.text(pos2(r.min.x + pad, y + 15.0), Align2::LEFT_CENTER, if n == 1 { "1 EGG".to_string() } else { format!("{n} EGGS") }, label(10.0), text);
            let right = if eggs.speed > 1 { format!("steps left · ×{} · {}", eggs.speed, ago_text(eggs.ago_secs)) } else { format!("steps left · {}", ago_text(eggs.ago_secs)) };
            p.text(pos2(r.max.x - pad, y + 15.0), Align2::RIGHT_CENTER, right, label(9.0), muted);
            y += 22.0;
        }
        let egg_h = if compact { 24.0 } else { 30.0 };
        let cw = ((r.width() - 2.0 * pad) / n as f32).min(if compact { 40.0 } else { 54.0 });
        let x0 = r.center().x - cw * n as f32 / 2.0;
        let shell = if classic { Color32::from_rgb(70, 66, 84) } else { super::ov_common::fade(muted, 0.3) };
        for (i, e) in eggs.eggs.iter().enumerate() {
            let cx = x0 + cw * (i as f32 + 0.5);
            let c = pos2(cx, y + 6.0 + egg_h / 2.0);
            paint_egg(p, c, egg_h, e.frac, shell, fill, if classic { theme::OUTLINE } else { muted });
            let soon = e.steps <= crate::breed::CYCLE_STEPS;
            p.text(pos2(cx, c.y + egg_h / 2.0 + 10.0), Align2::CENTER_CENTER, short_steps(e.steps), body(if compact { 10.0 } else { 11.0 }), if soon { if classic { GOLD } else { (lk.tone)(GOLD) } } else { text });
        }
        super::ov_common::tip(ui, r, "egg-strip", &eggs.tip());
    }

    /// The Eggs cell in the Bar: a label over a little egg for each one.
    /// Returns how wide it was.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bar_eggs(&self, ui: &mut egui::Ui, p: &Painter, x: f32, top: f32, bar_h: f32, eggs: &EggsNow, label: FontId, text: Color32, muted: Color32, fill: Color32, shell: Color32) -> f32 {
        let lab = format!("EGG {}", short_steps(eggs.next_steps()).to_uppercase());
        let a = p.text(pos2(x, top + 16.0), Align2::LEFT_CENTER, lab, label, text);
        let step = 15.0;
        for (i, e) in eggs.eggs.iter().enumerate() {
            paint_egg(p, pos2(x + 6.0 + i as f32 * step, top + 32.0), 13.0, e.frac, shell, fill, muted);
        }
        let w = a.width().max(eggs.eggs.len() as f32 * step);
        super::ov_common::tip(ui, Rect::from_min_size(pos2(x, top + 6.0), vec2(w, bar_h - 12.0)), "bar-eggs", &eggs.tip());
        w
    }

    // ───────────────────────────── Masuda helper ─────────────────────────────

    /// On the Hunt page with the Masuda Method: what your save says about
    /// your Day-Care pair, the Eggs you carry and your Shiny Charm.
    pub(super) fn masuda_helper(&mut self, ui: &mut egui::Ui) {
        let Some(s) = self.setup.boxes.save.clone() else {
            let why = self.setup.boxes.error.clone().unwrap_or_else(|| "Reading your save…".into());
            ui.label(RichText::new(why).size(12.0).color(MUTED()));
            return;
        };
        let text = self.setup.boxes.text.clone().unwrap_or_default();
        egui::Frame::none().fill(kit::sunken()).rounding(12.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("YOUR DAY-CARE").size(10.5).strong().color(MUTED()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("🔒 From your save, read only").size(10.5).color(MUTED()));
                });
            });
            ui.add_space(8.0);
            // the pair
            let pair: Vec<Mon> = s.daycare.clone();
            if pair.is_empty() {
                ui.label(RichText::new("Nobody's at the Day-Care. Leave two Pokémon there (Solaceon Town) to get Eggs.").size(12.5).color(TEXT()));
            } else {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for (k, m) in pair.iter().enumerate() {
                        if k == 1 {
                            ui.label(RichText::new("+").size(16.0).color(MUTED()));
                        }
                        self.masuda_parent(ui, m);
                    }
                });
            }
            ui.add_space(10.0);
            let charms = s.bag.iter().filter(|(_, i, _)| text.item_name(*i) == "Shiny Charm").map(|(_, _, q)| *q as u32).sum::<u32>();
            if pair.len() == 2 {
                let (a, b) = (&pair[0], &pair[1]);
                // can they have Eggs at all, and how keen
                let par = |m: &Mon, other: &Mon| -> Option<crate::breed::Parent> {
                    let sp = self.box_species(m.species)?;
                    let id = self.wiki.as_ref()?.by_dex(sp.dex)?.id.clone();
                    let g = if m.genderless { crate::breed::Gender::Genderless } else if m.female { crate::breed::Gender::Female } else { crate::breed::Gender::Male };
                    let mut p = crate::breed::Parent::new(&id, g);
                    p.other_trainer = (m.tid, m.sid) != (other.tid, other.sid);
                    Some(p)
                };
                if let (Some(pa), Some(pb)) = (par(a, b), par(b, a)) {
                    let pb = crate::breed::Parent { other_trainer: false, ..pb };
                    match crate::breed::compat(&pa, &pb, self.wiki.as_deref()) {
                        Ok(c) => {
                            ui.label(RichText::new(format!("💬 {}", c.line())).size(12.0).color(TEXT()));
                        }
                        Err(why) => {
                            kit::pill(ui, &format!("✖ No Eggs from these two: {}", why.text()), PillKind::Warn);
                        }
                    }
                    ui.add_space(6.0);
                }
                let masuda = a.language != b.language;
                let one_in = crate::breed::shiny_one_in(crate::breed::shiny_rolls(charms > 0 || self.cfg.shiny_charms > 0, masuda, self.cfg.shiny_lure));
                if masuda {
                    kit::pill(
                        ui,
                        &format!("✔ Masuda Method: {} + {} · each Egg 1 in {}", language(a.language), language(b.language), super::fmt_one_in(one_in)),
                        PillKind::Ok,
                    );
                } else {
                    kit::pill(ui, &format!("✖ Not Masuda: both are {} · each Egg 1 in {}", language(a.language), super::fmt_one_in(one_in)), PillKind::Warn);
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new("The Masuda Method needs a parent from a game in another language: a trade or the GTS. A foreign Ditto works with anything.")
                                .size(11.5)
                                .color(MUTED()),
                        )
                        .wrap(),
                    );
                }
            }
            if s.egg_waiting {
                ui.add_space(6.0);
                kit::pill(ui, "🔔 An Egg is waiting at the Day-Care", PillKind::Info);
            }
            // the Shiny Charm: the save knows, the setting may not
            if charms > 0 && self.cfg.shiny_charms == 0 {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Your Bag has a Shiny Charm, but your odds don't count it yet.").size(12.0).color(TEXT()));
                    if ui.add(egui::Button::new("Count it").fill(kit::btn_fill())).on_hover_text("Sets Shiny Charms in your Bag to 1 (only in this app)").clicked() {
                        self.cfg.shiny_charms = 1;
                        self.dirty = true;
                    }
                });
            }
            // the Eggs you carry
            ui.add_space(12.0);
            ui.label(RichText::new("EGGS YOU CARRY").size(10.5).strong().color(MUTED()));
            ui.add_space(6.0);
            match self.eggs_now() {
                None => {
                    ui.label(RichText::new("None right now. Pick up Eggs from the Day-Care Man and they'll show here and on the overlay.").size(12.0).color(MUTED()));
                }
                Some(e) => {
                    let n = e.eggs.len();
                    let w = ui.available_width();
                    let cw = ((w - 8.0 * (n as f32 - 1.0)) / n.max(3) as f32).min(120.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for egg in &e.eggs {
                            let (r, resp) = ui.allocate_exact_size(vec2(cw, 86.0), Sense::hover());
                            let p = ui.painter();
                            p.rect_filled(r, 10.0, kit::CARD());
                            paint_egg(p, pos2(r.center().x, r.min.y + 26.0), 32.0, egg.frac, theme::blend(kit::CARD(), TEXT(), 0.12), GOLD, MUTED());
                            p.text(pos2(r.center().x, r.min.y + 54.0), Align2::CENTER_CENTER, format!("{}%", (egg.frac * 100.0).round()), crate::fonts::sans_bold_font(13.0), TEXT());
                            p.text(pos2(r.center().x, r.min.y + 71.0), Align2::CENTER_CENTER, short_steps(egg.steps) + if egg.steps > crate::breed::CYCLE_STEPS { " steps" } else { "" }, crate::fonts::sans_font(11.0), MUTED());
                            let name = egg.sp.as_ref().map_or("An", |s| s.name);
                            resp.on_hover_text(format!("{name} Egg: {}", steps_text(egg.steps)));
                        }
                    });
                    ui.add_space(6.0);
                    let mut why = Vec::new();
                    why.push(if e.flame_body { "Flame Body in your party ✔" } else { "No Flame Body or Magma Armor in your party" });
                    why.push(if e.warmer { "Egg Warmer ✔" } else { "No Egg Warmer" });
                    ui.label(RichText::new(format!("{} · {} · as of the game's save {}", why.join(" · "), if e.speed > 1 { format!("×{} speed", e.speed) } else { "normal speed".into() }, ago_text(e.ago_secs))).size(11.5).color(MUTED()));
                }
            }
        });
    }

    fn masuda_parent(&mut self, ui: &mut egui::Ui, m: &Mon) {
        let sp = self.box_species(m.species);
        let (r, _) = ui.allocate_exact_size(vec2(210.0, 52.0), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(r, 10.0, kit::CARD());
        if let Some(s) = &sp {
            self.draw_sprite_centered(&p, s, m.shiny(), pos2(r.min.x + 28.0, r.center().y), vec2(44.0, 44.0));
        }
        let name = if m.nicknamed && !m.nickname.is_empty() { m.nickname.clone() } else { sp.as_ref().map_or_else(|| format!("#{}", m.species), |s| s.name.to_string()) };
        p.text(pos2(r.min.x + 58.0, r.min.y + 18.0), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(13.0), TEXT());
        let g = if m.genderless { "" } else if m.female { " ♀" } else { " ♂" };
        p.text(pos2(r.min.x + 58.0, r.min.y + 36.0), Align2::LEFT_CENTER, format!("{}{g}", language(m.language)), crate::fonts::sans_font(11.5), MUTED());
    }
}

/// An egg shape (narrower at the top) around `c`, `h` tall.
fn egg_points(c: Pos2, h: f32) -> Vec<Pos2> {
    let (rx, ry) = (h * 0.39, h / 2.0);
    (0..36)
        .map(|i| {
            let a = i as f32 / 36.0 * std::f32::consts::TAU;
            let (s, co) = a.sin_cos();
            pos2(c.x + rx * s * (1.0 - 0.14 * co), c.y - ry * co)
        })
        .collect()
}

/// An egg filling up from the bottom as it gets closer to hatching.
pub(super) fn paint_egg(p: &Painter, c: Pos2, h: f32, frac: f32, shell: Color32, fill: Color32, ink: Color32) {
    let pts = egg_points(c, h);
    p.add(Shape::convex_polygon(pts.clone(), shell, Stroke::NONE));
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let clip = Rect::from_min_max(pos2(c.x - h, c.y + h / 2.0 - h * f), pos2(c.x + h, c.y + h / 2.0 + 1.0));
        p.with_clip_rect(clip.intersect(p.clip_rect())).add(Shape::convex_polygon(pts.clone(), fill, Stroke::NONE));
    }
    p.add(Shape::closed_line(pts, Stroke::new(1.2f32, ink)));
}
