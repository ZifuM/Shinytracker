//! Pokémon Boxes: your party and boxes, read from the game's save file.
//! READ ONLY and local to this PC: the save is only ever read (never written,
//! changed or patched; the game's memory and Platinum.exe are never touched),
//! and the app keeps nothing but your own notes and the names you give newer
//! Pokémon. It updates each time the game saves.

use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::App;
use crate::apps::AppId;
use crate::gamesave::{self, GameText, Mon, Save};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};

pub(super) const BOX_COL: Color32 = Color32::from_rgb(96, 165, 250);
const GREEN: Color32 = Color32::from_rgb(52, 211, 153);

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Sel {
    Party(usize),
    Slot(usize, usize),
}

#[derive(Default)]
pub(super) struct BoxesState {
    pub save: Option<Save>,
    pub text: Option<GameText>,
    pub error: Option<String>,
    /// When the save we have was written (unix ms).
    pub save_ms: u64,
    last_check: Option<Instant>,
    pub cur: usize,
    pub sel: Option<Sel>,
    /// Naming a newer Pokémon: the search.
    pub naming: Option<String>,
    /// Eggs the log says hatched since the save was read (the save doesn't
    /// know yet): the nearest ones are left off.
    pub hatched: usize,
    /// Poké Balls the log saw thrown: (unix ms, item number).
    pub throws: Vec<(u64, u16)>,
}

/// HP, Atk, Def, SpA, SpD, Spe (the save keeps Speed fourth).
fn stat_order(v: [u8; 6]) -> [u8; 6] {
    [v[0], v[1], v[2], v[4], v[5], v[3]]
}

impl App {
    fn boxes_on(&self) -> bool {
        crate::apps::live(AppId::Boxes) || crate::apps::live(AppId::Nuzlocke) || crate::apps::live(AppId::Breed) || crate::apps::live(AppId::Stickers)
    }

    pub(super) fn game_root(&self) -> std::path::PathBuf {
        crate::gamemap::game_dir(self.cfg.game_dir_setting().as_deref())
    }

    /// Every frame: look at the save every few seconds, read it again when
    /// the game has saved.
    /// (Also feeds the party in the title bar, with the app installed or not.)
    pub(super) fn boxes_tick(&mut self) {
        let eggs = self.cfg.overlay.eggs || self.cfg.hunt_method == crate::config::HuntMethod::Masuda;
        let dex = self.setup.pokedex.game || self.cfg.overlay.balls;
        // the Team Planner's checklist ticks itself off from the save
        let team = crate::apps::live(AppId::Team);
        if !self.boxes_on() && !self.cfg.show_party && !eggs && !dex && !self.setup.quick.open && !team {
            return;
        }
        let st = &self.setup.boxes;
        if st.last_check.is_some_and(|t| t.elapsed() < Duration::from_secs(4)) {
            return;
        }
        self.setup.boxes.last_check = Some(Instant::now());
        let root = self.game_root();
        if self.setup.boxes.text.is_none() {
            self.setup.boxes.text = gamesave::load_text(&root);
        }
        // the newer of save.bin and the game's own few-minutely snapshot
        let Some((path, ms)) = gamesave::newest_save(&root) else {
            self.setup.boxes.error = Some("Your save file wasn't found. Check the game folder in Settings > Advanced.".into());
            return;
        };
        if ms == self.setup.boxes.save_ms {
            return;
        }
        match gamesave::read(&path) {
            Ok(s) => {
                self.setup.boxes.save = Some(s);
                self.setup.boxes.hatched = 0;
                // throws the save now has are in its Bag count
                self.setup.boxes.throws.retain(|(t, _)| *t > ms);
                self.party_save_read(ms);
                self.setup.boxes.save_ms = ms;
                self.setup.boxes.error = None;
                self.breed_sync(self.wiki.clone());
            }
            Err(e) => self.setup.boxes.error = Some(e),
        }
    }

    /// The app's species for the game's number (None: a newer one not named yet).
    pub(super) fn box_species(&self, id: u16) -> Option<crate::species::Species> {
        let slug = super::party::resolve(&self.cfg, &self.species, id as u32)?;
        self.species.iter().find(|s| s.slug == slug).cloned()
    }

    fn box_level(&self, m: &Mon) -> Option<u8> {
        if m.level.is_some() {
            return m.level;
        }
        let dex = self.box_species(m.species)?.dex;
        let g = crate::sim::data().growth(dex)?;
        Some(gamesave::level_from_exp(g, m.exp))
    }

    fn sel_mon(&self) -> Option<Mon> {
        let s = self.setup.boxes.save.as_ref()?;
        match self.setup.boxes.sel? {
            Sel::Party(i) => s.party.get(i).cloned(),
            Sel::Slot(b, k) => s.boxes.get(b)?.slots.get(k)?.clone(),
        }
    }

    pub(super) fn page_boxes(&mut self, ui: &mut egui::Ui) {

        super::app_tour::here(ui.ctx(), "boxes");
        kit::page_header(ui, "POKéMON BOXES", "Your party and boxes, read from the game's save.");
        // the read-only note
        egui::Frame::none()
            .fill(theme::blend(kit::CARD(), GREEN, 0.10))
            .stroke(Stroke::new(1.0f32, theme::blend(kit::CARD(), GREEN, 0.45)))
            .rounding(10.0)
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("🔒  Read only, and only on this PC").strong().size(12.5).color(GREEN));
                ui.label(
                    RichText::new(
                        "Everything here comes from your save file, which is only ever read: nothing is written to the game, and nothing is uploaded. It updates each time the game saves (every few minutes). What you add yourself (your notes, and a name for any brand-new Pokémon a game update brings) stays in the app.",
                    )
                    .size(12.0)
                    .color(TEXT()),
                );
            });
        ui.add_space(12.0);
        if let Some(e) = self.setup.boxes.error.clone() {
            kit::pill(ui, &e, kit::PillKind::Warn);
            ui.add_space(8.0);
        }
        if self.setup.boxes.save.is_none() {
            ui.label(RichText::new("Reading your save…").color(MUTED()));
            return;
        }
        // Three panels on one line: party | box | details. Same top, same
        // bottom, the same card look, and edge to edge with the note above.
        let w = ui.available_width();
        let gap = 14.0;
        let pad = 12.0;
        let party_w = 96.0;
        let cg = 6.0; // between cells
        let cell = ((w - party_w - 320.0 - 2.0 * gap - 2.0 * pad - 5.0 * cg) / 6.0).clamp(54.0, 88.0).floor();
        let box_w = 6.0 * cell + 5.0 * cg + 2.0 * pad;
        let side_w = w - party_w - box_w - 2.0 * gap;
        let head_h = 32.0;
        let h = pad + head_h + 10.0 + 5.0 * cell + 4.0 * cg + pad;
        let (all, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let party_r = Rect::from_min_size(all.min, vec2(party_w, h));
        let box_r = Rect::from_min_size(pos2(party_r.max.x + gap, all.min.y), vec2(box_w, h));
        let side_r = Rect::from_min_max(pos2(box_r.max.x + gap, all.min.y), pos2(all.max.x, all.max.y));
        let _ = side_w;
        super::app_tour::mark(ui.ctx(), "boxes.party", party_r);
        super::app_tour::mark(ui.ctx(), "boxes.box", box_r);
        super::app_tour::mark(ui.ctx(), "boxes.side", side_r);
        for r in [party_r, box_r, side_r] {
            ui.painter().rect_filled(r, 14.0, kit::CARD());
            ui.painter().rect_stroke(r, 14.0, Stroke::new(1.0f32, LINE()));
        }
        // (children placed on the panels, so the page carries on under them)
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(party_r.shrink(pad)).layout(egui::Layout::top_down(egui::Align::Center)));
        self.boxes_party_col(&mut c, head_h, cell.min(party_w - 2.0 * pad), cg);
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(box_r.shrink(pad)).layout(egui::Layout::top_down(egui::Align::Min)));
        self.boxes_grid(&mut c, box_w - 2.0 * pad, head_h, cell, cg);
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(side_r.shrink(pad)).layout(egui::Layout::top_down(egui::Align::Min)));
        c.set_width(side_r.width() - 2.0 * pad);
        self.boxes_stage(&mut c, h - 2.0 * pad);
        ui.add_space(10.0);
        let s = self.setup.boxes.save.as_ref().expect("checked");
        let n: usize = s.boxes.iter().map(|b| b.slots.iter().flatten().count()).sum();
        let when = if self.setup.boxes.save_ms > 0 { crate::stats::fmt_clock(self.setup.boxes.save_ms / 1000) } else { String::new() };
        let line = format!("{} · {n} Pokémon in {} boxes · game last saved {when}", s.trainer, s.boxes.len());
        let has_party = s.party.iter().any(|m| !m.egg);
        let mut party_team = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(line).size(11.5).color(MUTED()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !super::app_modal::have(AppId::Team) {
                    super::app_modal::lock_btn(ui, AppId::Team, "Party to the Team Planner");
                } else if ui
                    .add_enabled(has_party, egui::Button::new("👥  Party to the Team Planner").fill(kit::btn_fill()))
                    .on_hover_text("Your party as a new team, with every Pokémon's real build, to plan with. It's a copy: your game isn't changed, and nothing goes back into it.")
                    .clicked()
                {
                    party_team = true;
                }
            });
        });
        if party_team {
            self.boxes_party_to_team();
        }
        ui.add_space(16.0);
        self.boxes_overview(ui);
    }

    /// One Pokémon in a cell: picture (or "?" / an Egg), shiny star.
    fn boxes_cell(&mut self, ui: &mut egui::Ui, r: Rect, m: Option<&Mon>, selected: bool, id: egui::Id) -> egui::Response {
        let resp = ui.interact(r, id, Sense::click());
        let p = ui.painter().clone();
        let hov = resp.hovered();
        p.rect_filled(r, 10.0, if hov { kit::CARD_HOVER() } else { kit::CARD() });
        p.rect_stroke(r, 10.0, if selected { Stroke::new(2.0f32, kit::ACCENT()) } else { Stroke::new(1.0f32, LINE()) });
        let Some(m) = m else { return resp };
        let c = r.center();
        let size = vec2(r.width() - 8.0, r.height() - 8.0);
        if m.egg {
            p.circle_filled(c, size.x * 0.28, Color32::from_rgb(240, 232, 205));
            p.circle_stroke(c, size.x * 0.28, Stroke::new(1.5f32, Color32::from_rgb(160, 150, 120)));
            p.text(c, Align2::CENTER_CENTER, "Egg", crate::fonts::sans_font(10.0), Color32::from_rgb(90, 80, 60));
            return resp;
        }
        match self.box_species(m.species) {
            Some(sp) if self.sprites.get(&sp, m.shiny()).is_some() => {
                self.draw_sprite_centered(&p, &sp, m.shiny(), c, size);
            }
            Some(sp) => {
                let short: String = sp.name.chars().take(2).collect();
                p.text(c, Align2::CENTER_CENTER, short, crate::fonts::sans_font(13.0), TEXT());
            }
            None => {
                p.text(c, Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(18.0), MUTED());
                p.text(pos2(c.x, r.max.y - 9.0), Align2::CENTER_CENTER, format!("#{}", m.species), crate::fonts::sans_font(9.0), MUTED());
            }
        }
        if m.shiny() {
            p.text(pos2(r.max.x - 9.0, r.min.y + 9.0), Align2::CENTER_CENTER, "★", crate::fonts::sans_bold_font(12.0), theme::GOLD);
        }
        resp
    }

    fn boxes_party_col(&mut self, ui: &mut egui::Ui, head_h: f32, cell: f32, cg: f32) {
        let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), head_h), Sense::hover());
        ui.painter().text(hr.center(), Align2::CENTER_CENTER, "PARTY", crate::fonts::sans_bold_font(11.0), MUTED());
        ui.add_space(10.0);
        let party = self.setup.boxes.save.as_ref().map(|s| s.party.clone()).unwrap_or_default();
        // six places, filled or not, spread over exactly the box's height so
        // the column starts and ends with the box
        let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), ui.available_height()), Sense::hover());
        let side = ((area.height() - 5.0 * cg) / 6.0).min(cell).floor();
        let step = (area.height() - side) / 5.0;
        for i in 0..6 {
            let r = Rect::from_min_size(pos2(area.center().x - side / 2.0, area.min.y + i as f32 * step), vec2(side, side));
            let sel = self.setup.boxes.sel == Some(Sel::Party(i));
            let m = party.get(i);
            let resp = self.boxes_cell(ui, r, m, sel, ui.id().with(("bx-party", i)));
            if m.is_some() && resp.on_hover_text("From your save (read only)").clicked() {
                self.setup.boxes.sel = Some(Sel::Party(i));
            }
        }
    }

    fn boxes_grid(&mut self, ui: &mut egui::Ui, w: f32, head_h: f32, cell: f32, gap: f32) {
        let Some(save) = self.setup.boxes.save.as_ref() else { return };
        let nboxes = save.boxes.len();
        if nboxes == 0 {
            return;
        }
        let cur = self.setup.boxes.cur.min(nboxes - 1);
        let names: Vec<(String, usize)> = save.boxes.iter().map(|b| (b.name.clone(), b.slots.iter().flatten().count())).collect();
        let slots = save.boxes[cur].slots.clone();
        // head: ‹ name ›, and a list to jump to any box
        ui.horizontal(|ui| {
            ui.set_height(head_h);
            if ui.add(egui::Button::new("‹").fill(kit::btn_fill()).min_size(vec2(36.0, head_h))).clicked() {
                self.setup.boxes.cur = (cur + nboxes - 1) % nboxes;
                self.setup.boxes.sel = None;
            }
            let mut pick = cur;
            egui::ComboBox::from_id_salt("bx-pick")
                .selected_text(RichText::new(format!("{}  ·  {}/30", names[cur].0, names[cur].1)).strong())
                .width(w - 2.0 * 36.0 - 2.0 * ui.spacing().item_spacing.x - 8.0)
                .height(420.0)
                .show_ui(ui, |ui| {
                    for (i, (n, c)) in names.iter().enumerate() {
                        let label = if *c == 0 { format!("{n}  (empty)") } else { format!("{n}  ·  {c}") };
                        ui.selectable_value(&mut pick, i, label);
                    }
                });
            if pick != cur {
                self.setup.boxes.cur = pick;
                self.setup.boxes.sel = None;
            }
            if ui.add(egui::Button::new("›").fill(kit::btn_fill()).min_size(vec2(36.0, head_h))).clicked() {
                self.setup.boxes.cur = (cur + 1) % nboxes;
                self.setup.boxes.sel = None;
            }
        });
        ui.add_space(10.0);
        let (area, _) = ui.allocate_exact_size(vec2(w, 5.0 * cell + 4.0 * gap), Sense::hover());
        for k in 0..gamesave::SLOTS {
            let (row, col) = (k / 6, k % 6);
            let r = Rect::from_min_size(pos2(area.min.x + col as f32 * (cell + gap), area.min.y + row as f32 * (cell + gap)), vec2(cell, cell));
            let m = slots.get(k).cloned().flatten();
            let sel = self.setup.boxes.sel == Some(Sel::Slot(cur, k));
            let resp = self.boxes_cell(ui, r, m.as_ref(), sel, ui.id().with(("bx-slot", cur, k)));
            if m.is_some() && resp.clicked() {
                self.setup.boxes.sel = Some(Sel::Slot(cur, k));
            }
        }
    }

    /// Where the picked Pokémon sits, in words.
    fn boxes_where(&self) -> String {
        match self.setup.boxes.sel {
            Some(Sel::Party(i)) => format!("In your party · place {}", i + 1),
            Some(Sel::Slot(b, k)) => {
                let name = self.setup.boxes.save.as_ref().and_then(|s| s.boxes.get(b)).map(|x| x.name.clone()).unwrap_or_default();
                format!("{name} · place {}", k + 1)
            }
            None => String::new(),
        }
    }

    /// Its ability by name. The game's text only names the first 123; past
    /// that it's worked out from the abilities this Pokémon can have that
    /// aren't one of those 123 (sometimes that leaves two).
    fn boxes_ability(&self, m: &Mon, text: &GameText) -> (String, Option<String>) {
        if (m.ability as usize) < text.abilities.len() && !text.abilities[m.ability as usize].is_empty() {
            return (text.ability_name(m.ability), None);
        }
        let Some(sp) = self.box_species(m.species) else { return (text.ability_name(m.ability), None) };
        let Some(b) = crate::battle::data().species.iter().find(|s| s.num == sp.dex && s.base.is_empty()) else {
            return (text.ability_name(m.ability), None);
        };
        let old = |a: &str| text.abilities.iter().any(|x| x.eq_ignore_ascii_case(a));
        let new: Vec<&String> = b.abilities.iter().filter(|a| !old(a)).collect();
        let why = "The game doesn't name its newer abilities. This is worked out from the ones this Pokémon can have.".to_string();
        match new.as_slice() {
            [one] => (one.to_string(), Some(why)),
            [a, b] => (format!("{a} or {b}"), Some(why)),
            _ => (text.ability_name(m.ability), None),
        }
    }

    /// Its stats as the game shows them: the party's straight from the save,
    /// box ones worked out (the game only keeps stats for the party).
    /// HP, Atk, Def, SpA, SpD, Spe, and its species' base stats.
    fn boxes_stats(&self, m: &Mon, level: Option<u8>) -> Option<([u16; 6], [u16; 6], bool)> {
        let sp = self.box_species(m.species)?;
        let b = crate::battle::data().species.iter().find(|s| s.num == sp.dex && s.base.is_empty())?;
        let base = b.base_stats(crate::battle::World::Platinum);
        if let Some(st) = m.stats {
            return Some((st, base, true));
        }
        let nature = crate::battle::data().nature(gamesave::NATURES[m.nature() as usize]);
        let ivs = stat_order(m.ivs).map(|v| v as u16);
        let evs = stat_order(m.evs).map(|v| v as u16);
        Some((crate::battle::stats(base, ivs, evs, level?, nature, sp.dex == 292), base, false))
    }

    /// The right-hand panel: the Pokémon on its type colours, like the top
    /// of its Pokédex page, with its name, types and where it is.
    fn boxes_stage(&mut self, ui: &mut egui::Ui, h: f32) {
        use super::charts as ch;
        if self.setup.boxes.sel.is_none() {
            let has_party = self.setup.boxes.save.as_ref().is_some_and(|s| !s.party.is_empty());
            if has_party {
                self.setup.boxes.sel = Some(Sel::Party(0));
            }
        }
        let Some(m) = self.sel_mon() else {
            ui.label(RichText::new("Pick a Pokémon to see everything the save knows about it.").color(MUTED()));
            return;
        };
        let text = self.setup.boxes.text.clone().unwrap_or_default();
        let sp = self.box_species(m.species);
        let wm = sp.as_ref().and_then(|s| self.wiki.as_ref().and_then(|w| w.by_dex(s.dex).cloned()));
        let level = self.box_level(&m);
        let types: Vec<String> = wm.as_ref().map(|w| w.types.clone()).unwrap_or_default();
        let w = ui.available_width();
        // ── the stage
        let stage_h = (h * 0.5).clamp(170.0, 250.0).floor();
        let (stage, _) = ui.allocate_exact_size(vec2(w, stage_h), Sense::hover());
        let p = ui.painter_at(stage.expand(2.0));
        let (c1, c2) = if m.egg {
            (Color32::from_rgb(214, 196, 150), Color32::from_rgb(150, 130, 90))
        } else {
            let c1 = types.first().map(|t| super::pokedex::type_color(t)).unwrap_or(BOX_COL);
            (c1, types.get(1).map(|t| super::pokedex::type_color(t)).unwrap_or(theme::blend(c1, Color32::BLACK, 0.35)))
        };
        p.add(theme::gradient_rect(stage, [12.0; 4], theme::blend(c1, Color32::BLACK, 0.15), theme::blend(c2, Color32::BLACK, 0.35)));
        let sc = stage.center();
        let big = (stage_h * 0.36).floor();
        p.circle_filled(sc, big, Color32::from_white_alpha(10));
        p.circle_filled(sc, big * 0.72, Color32::from_white_alpha(12));
        if m.egg {
            p.circle_filled(sc, big * 0.55, Color32::from_rgb(244, 238, 216));
            p.circle_stroke(sc, big * 0.55, Stroke::new(2.0f32, Color32::from_rgb(160, 150, 120)));
        } else if let Some(s) = &sp {
            self.draw_sprite_centered(&p, s, m.shiny(), sc, vec2(stage_h * 0.66, stage_h * 0.66));
        } else {
            p.text(sc, Align2::CENTER_CENTER, "?", ch::heavy(44.0), Color32::from_white_alpha(200));
        }
        if m.shiny() && !m.egg {
            theme::sparkle(&p, sc + vec2(big * 0.95, -big * 0.8), 7.0, theme::GOLD);
            theme::sparkle(&p, sc + vec2(-big, big * 0.55), 4.5, theme::GOLD);
        }
        if let Some(s) = sp.as_ref().filter(|_| !m.egg) {
            p.text(stage.min + vec2(12.0, 12.0), Align2::LEFT_TOP, format!("#{:04}", s.dex), ch::bold(12.0), Color32::from_white_alpha(190));
        }
        let lw = super::charts::bold(11.5);
        let wtxt = self.boxes_where();
        p.text(pos2(stage.max.x - 12.0, stage.min.y + 12.0), Align2::RIGHT_TOP, super::charts::clip(ui, &wtxt, &lw, w * 0.6), lw, Color32::from_white_alpha(190));
        ui.add_space(10.0);

        // ── name, types, chips
        let species_name = sp.as_ref().map_or_else(|| format!("Pokémon #{}", m.species), |s| s.name.to_string());
        let title = if m.egg { "Egg".to_string() } else if m.nicknamed && !m.nickname.is_empty() { m.nickname.clone() } else { species_name.clone() };
        ui.label(RichText::new(&title).font(ch::heavy(26.0)).color(TEXT()));
        if m.egg {
            ui.label(RichText::new(format!("A {species_name} Egg")).size(13.0).color(MUTED()));
        } else if title != species_name {
            ui.label(RichText::new(&species_name).size(13.0).color(MUTED()));
        }
        ui.add_space(6.0);
        if !m.egg {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                for t in &types {
                    super::pokedex::type_pill(ui, t);
                }
                if wm.as_ref().is_some_and(|w| w.modern) {
                    kit::pill(ui, "Modern", kit::PillKind::Info);
                }
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                if let Some(l) = level {
                    super::pokedex::info_chip(ui, "▲", BOX_COL, &format!("Lv {l}"));
                }
                if !m.genderless {
                    let (g, c) = if m.female { ("♀  Female", Color32::from_rgb(240, 120, 170)) } else { ("♂  Male", Color32::from_rgb(86, 150, 240)) };
                    super::pokedex::info_chip(ui, g.split_whitespace().next().unwrap_or(""), c, g.split("  ").nth(1).unwrap_or(""));
                }
                if m.shiny() {
                    super::pokedex::info_chip(ui, "★", theme::GOLD, "Shiny");
                }
            });
        }
        // a number past the list (a newer game update): name it once
        if sp.is_none() && !m.egg {
            ui.add_space(8.0);
            ui.label(RichText::new(format!("The game numbers this one #{}, which isn't on the app's list yet. Which Pokémon is it? You only name each number once.", m.species)).size(11.5).color(MUTED()));
            let mut q = self.setup.boxes.naming.clone().unwrap_or_default();
            ui.add(egui::TextEdit::singleline(&mut q).hint_text("Search a Pokémon").desired_width(f32::INFINITY));
            let ql = q.trim().to_lowercase();
            if !ql.is_empty() {
                let hits: Vec<(String, &'static str)> = self.species.iter().filter(|s| s.dex > 493 && s.name.to_lowercase().starts_with(&ql)).take(6).map(|s| (s.slug.clone(), s.name)).collect();
                ui.horizontal_wrapped(|ui| {
                    for (slug, name) in hits {
                        if ui.button(name).clicked() {
                            self.cfg.game_dex.insert(m.species as u32, slug);
                            self.dirty = true;
                            q.clear();
                        }
                    }
                });
            }
            self.setup.boxes.naming = Some(q);
        }
        if m.egg {
            ui.add_space(4.0);
            super::pokedex::info_chip(ui, "🥚", BOX_COL, &format!("About {} steps to hatch", theme::thousands(m.friendship as u64 * 255)));
            ui.label(RichText::new("As of the game's last save.").size(11.5).color(MUTED()));
            return;
        }
        // ── actions: this Pokémon in the other apps (copies; the game is
        // never changed)
        ui.add_space(12.0);
        let mut open_dex = None;
        let mut log_it = false;
        let mut to_team: Option<Option<u64>> = None;
        let (mut to_calc, mut to_breed, mut to_hunt) = (false, false, false);
        let teams: Vec<(u64, String, usize)> = if super::app_modal::have(AppId::Team) { self.team_choices().into_iter().filter(|t| t.2 < 6).collect() } else { Vec::new() };
        let known = crate::breed_save::species_id(m.species).is_some();
        let btn = |label: &str| egui::Button::new(label).fill(kit::btn_fill());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            if let Some(wm) = &wm {
                if ui.add(btn("📖  Pokédex")).on_hover_text(format!("Open {} in the Pokédex", wm.name)).clicked() {
                    open_dex = Some(wm.id.clone());
                }
            }
            if known {
                if !super::app_modal::have(AppId::Team) {
                    super::app_modal::lock_btn(ui, AppId::Team, "Add to a team");
                } else {
                    ui.menu_button("👥  Add to a team", |ui| {
                        ui.label(RichText::new("With its nature, IVs, EVs, Ability, item and moves").size(11.0).color(MUTED()));
                        if ui.button("➕  A new team").clicked() {
                            to_team = Some(None);
                            ui.close_menu();
                        }
                        for (id, name, n) in &teams {
                            let name = if name.trim().is_empty() { "Untitled team" } else { name.trim() };
                            if ui.button(format!("{name}  ·  {n}/6")).clicked() {
                                to_team = Some(Some(*id));
                                ui.close_menu();
                            }
                        }
                    })
                    .response
                    .on_hover_text("A copy in the Team Planner to plan with: your game isn't changed");
                }
                if !super::app_modal::have(AppId::Calc) {
                    super::app_modal::lock_btn(ui, AppId::Calc, "Damage calc");
                } else if ui.add(btn("⚔  Damage calc")).on_hover_text("On your side of the Damage Calculator, with its real stats and moves").clicked() {
                    to_calc = true;
                }
                if !super::app_modal::have(AppId::Breed) {
                    super::app_modal::lock_btn(ui, AppId::Breed, "Breed one");
                } else if ui.add(btn("Breed one")).on_hover_text("Sets the Breeding Planner's goal to its species, nature and Ability").clicked() {
                    to_breed = true;
                }
            }
            if !m.shiny() && sp.is_some() && ui.add(btn("🎯  Hunt a shiny")).on_hover_text("Opens the New hunt guide with it picked. Nothing changes until you start the hunt there.").clicked() {
                to_hunt = true;
            }
            if m.shiny() && sp.is_some() && ui.add(btn("✨  Log shiny")).on_hover_text("Opens Log a shiny with its details filled in from your save").clicked() {
                log_it = true;
            }
        });
        if let Some(id) = open_dex {
            self.open_dex_entry(&id);
        }
        if let Some(t) = to_team {
            self.boxes_to_team(&m, t);
        }
        if to_calc {
            self.boxes_to_calc(&m);
        }
        if to_breed {
            self.boxes_to_breed(&m);
        }
        if to_hunt {
            if let Some(s) = &sp {
                self.boxes_to_hunt(&s.slug.clone());
            }
        }
        if log_it {
            if let Some(s) = sp {
                self.boxes_log_shiny(&m, &s, level, &text);
            }
        }
    }

    /// Under the three panels: the picked Pokémon's own overview (like its
    /// Pokédex page's Overview, but this one's facts and real stats) and
    /// its evolutions.
    fn boxes_overview(&mut self, ui: &mut egui::Ui) {
        let Some(m) = self.sel_mon() else { return };
        if m.egg {
            return;
        }
        let Some(sp) = self.box_species(m.species) else { return };
        let text = self.setup.boxes.text.clone().unwrap_or_default();
        let level = self.box_level(&m);
        let wiki = self.wiki.clone();
        let wm = wiki.as_ref().and_then(|w| w.by_dex(sp.dex).cloned());
        let key = format!("{:08x}", m.pid);
        let stats = self.boxes_stats(&m, level);
        let (ability, ability_why) = self.boxes_ability(&m, &text);
        let nature = gamesave::NATURES[m.nature() as usize];
        let nat = crate::battle::data().nature(nature).cloned();
        let short = ["HP", "Atk", "Def", "SpA", "SpD", "Spe"];
        let nature_txt = match &nat {
            Some(n) if !n.neutral() => format!("{nature}  (+{} −{})", short[n.plus], short[n.minus]),
            _ => format!("{nature}  (no change)"),
        };
        let mut met = String::new();
        if let Some(place) = text.place_name(m.met_location) {
            met = place;
        }
        let mut met_on = String::new();
        if let Some((y, mo, d)) = m.met_date {
            met_on = format!("{d} {} 20{y:02}", ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][(mo as usize).clamp(1, 12) - 1]);
        }
        if m.met_level > 0 {
            met_on = if met_on.is_empty() { format!("Lv {}", m.met_level) } else { format!("{met_on} · Lv {}", m.met_level) };
        }
        let ivs = stat_order(m.ivs);
        let evs = stat_order(m.evs);
        let (hp_type, hp_pow) = crate::battle::hidden_power(ivs.map(|v| v as u16));
        let moves: Vec<(String, Option<gamesave::MoveInfo>)> = m.moves.iter().filter(|x| **x != 0).map(|x| (text.move_name(*x), text.move_info(*x))).collect();
        let dash = |s: String| if s.is_empty() { "—".to_string() } else { s };
        let facts: [(&str, String); 10] = [
            ("Level", level.map_or("—".into(), |l| l.to_string())),
            ("Nature", nature_txt),
            ("Ability", ability.clone()),
            ("Held item", if m.item == 0 { "None".into() } else { text.item_name(m.item) }),
            ("Poké Ball", if m.ball == 0 { "—".into() } else { text.item_name(m.ball as u16) }),
            ("Hidden Power", format!("{hp_type} · {hp_pow}")),
            ("Met at", dash(met)),
            ("Met", dash(met_on)),
            ("Trainer", format!("{} · ID {:05}", m.ot, m.tid)),
            ("Friendship", format!("{} / 255", m.friendship)),
        ];
        let mut note = self.cfg.box_notes.get(&key).cloned().unwrap_or_default();
        let mut note_changed = false;

        // Both columns end at the same line: last frame's natural heights say
        // which is short, and by how much.
        let cols_id = egui::Id::new("bx-overview-cols");
        let (nat_l, nat_r): (f32, f32) = ui.ctx().data(|d| d.get_temp(cols_id)).unwrap_or((0.0, 0.0));
        let grow_left = (nat_r - nat_l).max(0.0);
        let grow_right = (nat_l - nat_r).max(0.0);
        let mut left = |ui: &mut egui::Ui| {
            kit::card(ui, |ui| {
                ui.set_width(ui.available_width());
                super::pokedex::card_title(ui, "At a glance");
                let cw = ((ui.available_width() - 16.0) / 2.0).floor();
                for row in facts.chunks(2) {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 16.0;
                        for (l, v) in row {
                            if *l == "Ability" && ability_why.is_some() {
                                ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                    ui.set_width(cw);
                                    super::pokedex::fact(ui, cw, l, v);
                                })
                                .response
                                .on_hover_text(ability_why.clone().unwrap_or_default());
                            } else {
                                super::pokedex::fact(ui, cw, l, v);
                            }
                        }
                    });
                    ui.add_space(10.0);
                }
                ui.label(RichText::new("Your notes").size(11.5).color(MUTED()));
                ui.add_space(4.0);
                // the notes grow to fill, so both columns end together
                let te = egui::TextEdit::multiline(&mut note).desired_rows(2).desired_width(f32::INFINITY).min_size(vec2(0.0, 44.0 + grow_left)).hint_text("Anything you want to remember about it. Only kept in the app.");
                if ui.add(te).changed() {
                    note_changed = true;
                }
            });
            ui.add_space(16.0);
            boxes_moves_card(ui, &moves);
        };
        let right = |ui: &mut egui::Ui| {
            boxes_stats_card(ui, &sp, stats, level, nat.as_ref(), ivs, evs, grow_right);
            if let Some(wm) = &wm {
                ui.add_space(16.0);
                super::pokedex::defenses_card(ui, wm);
            }
        };
        let w = ui.available_width();
        if w >= 820.0 {
            let gap = 16.0;
            let lw = ((w - gap) / 2.0).floor();
            let (mut lh, mut rh) = (0.0, 0.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                lh = ui
                    .allocate_ui_with_layout(vec2(lw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(lw);
                        left(ui);
                    })
                    .response
                    .rect
                    .height();
                rh = ui
                    .allocate_ui_with_layout(vec2(lw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(lw);
                        right(ui);
                    })
                    .response
                    .rect
                    .height();
            });
            let natural = (lh - grow_left, rh - grow_right);
            if (natural.0 - nat_l).abs() > 0.5 || (natural.1 - nat_r).abs() > 0.5 {
                ui.ctx().data_mut(|d| d.insert_temp(cols_id, natural));
                ui.ctx().request_repaint();
            }
        } else {
            left(ui);
            ui.add_space(16.0);
            right(ui);
        }
        if note_changed {
            if note.trim().is_empty() {
                self.cfg.box_notes.remove(&key);
            } else {
                self.cfg.box_notes.insert(key.clone(), note);
            }
            self.dirty = true;
        }
        // ── evolutions
        if let (Some(w), Some(wm)) = (wiki.as_ref(), wm.as_ref()) {
            ui.add_space(16.0);
            self.boxes_evolution(ui, w, wm);
        }
    }

    /// The family as a chart, like the Pokédex's Evolution tab. Click one to
    /// open its Pokédex page.
    fn boxes_evolution(&mut self, ui: &mut egui::Ui, w: &std::sync::Arc<crate::wiki::Wiki>, m: &crate::wiki::Mon) {
        let family = w.family(&m.id);
        let caught = self.dex_caught();
        let mut open: Option<String> = None;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            super::pokedex::card_title(ui, "Evolution");
            if family.len() <= 1 {
                ui.label(RichText::new(format!("{} doesn't evolve.", m.name)).size(12.5).color(MUTED()));
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 10.0);
                for (si, stage) in family.iter().enumerate() {
                    if si > 0 {
                        let (r, _) = ui.allocate_exact_size(vec2(26.0, super::pokedex::EVO_CHIP + super::pokedex::EVO_TILE_H), Sense::hover());
                        ui.painter().text(
                            pos2(r.center().x, r.min.y + super::pokedex::EVO_CHIP + super::pokedex::EVO_TILE_H / 2.0 - 10.0),
                            Align2::CENTER_CENTER,
                            "›",
                            kit::font_bold(26.0),
                            MUTED(),
                        );
                    }
                    let per_row = if stage.len() > 3 { 4 } else { 1 };
                    ui.vertical(|ui| {
                        for row in stage.chunks(per_row) {
                            ui.horizontal(|ui| {
                                for id in row {
                                    let how = if si > 0 { w.evolves_from(id).map(|e| e.method.clone()) } else { None };
                                    if self.evo_tile(ui, w, id, how.as_deref(), id == &m.id, false, &caught) {
                                        open = Some(id.clone());
                                    }
                                }
                            });
                        }
                    });
                }
            });
            ui.add_space(4.0);
            // how this one evolves, in the wiki's words
            let next = w.evolves_into(&m.id);
            for e in next {
                let name = w.mon(&e.species).map(|x| x.name.clone()).unwrap_or_else(|| e.species.clone());
                ui.add(egui::Label::new(RichText::new(format!("Into {name}: {}", e.method)).size(12.0).color(TEXT())).wrap());
            }
            ui.label(RichText::new("Click one to open its Pokédex page.").size(11.0).color(MUTED()));
        });
        if let Some(id) = open {
            self.open_dex_entry(&id);
        }
    }

    /// A Pokémon from the save as a Team Planner member: its species,
    /// level, nature, IVs, EVs, Ability, item, moves, nickname, gender and
    /// shininess. (A copy: nothing goes back to the game.)
    pub(super) fn boxes_member(&self, m: &Mon, text: &GameText, world: crate::battle::World) -> Option<crate::teams::Member> {
        let id = crate::breed_save::species_id(m.species)?;
        let name = crate::battle::data().species(&id)?.name.clone();
        let mut t = crate::teams::Member::new(&name, world);
        if let Some(l) = self.box_level(m) {
            t.level = l.max(1);
        }
        t.nature = gamesave::NATURES[m.nature() as usize].to_string();
        t.ivs = stat_order(m.ivs).map(|v| v as u16);
        t.evs = stat_order(m.evs).map(|v| v as u16);
        let moves: Vec<String> = m.moves.iter().filter(|x| **x != 0 && (**x as usize) < text.moves.len()).map(|x| text.move_name(*x)).collect();
        if !moves.is_empty() {
            t.moves = moves;
        }
        if (m.ability as usize) < text.abilities.len() {
            t.ability = text.ability_name(m.ability);
        }
        t.item = if m.item != 0 && (m.item as usize) < text.items.len() { text.item_name(m.item) } else { String::new() };
        if m.nicknamed {
            t.nickname = m.nickname.clone();
        }
        t.gender = if m.genderless { None } else if m.female { Some('F') } else { Some('M') };
        t.shiny = m.shiny();
        t.happiness = m.friendship;
        // its checklist follows this very Pokémon
        t.game_pick = crate::teams::GamePick::Pid(m.pid);
        Some(t)
    }

    pub(super) fn team_world(&self) -> crate::battle::World {
        if self.cfg.info_modern {
            crate::battle::World::Modern
        } else {
            crate::battle::World::Platinum
        }
    }

    /// Your party as a new team in the Team Planner, to plan with (your
    /// game isn't changed, and the team can't go back into it).
    fn boxes_party_to_team(&mut self) {
        let Some(save) = self.setup.boxes.save.clone() else { return };
        let text = self.setup.boxes.text.clone().unwrap_or_default();
        let world = self.team_world();
        let members: Vec<crate::teams::Member> = save.party.iter().filter(|m| !m.egg).filter_map(|m| self.boxes_member(m, &text, world)).collect();
        if members.is_empty() {
            return;
        }
        let id = self.setup.team.new_id();
        let day = crate::stats::fmt_day_short(crate::stats::local_day(crate::stats::now_unix()));
        let who = if save.trainer.trim().is_empty() { "My".to_string() } else { format!("{}'s", save.trainer.trim()) };
        let mut t = crate::teams::Team::new(id, &format!("{who} party · {day}"), world);
        if members.iter().any(|m| crate::battle::data().species(&m.species).is_some_and(|s| s.modern_only())) {
            t.world = crate::battle::World::Modern;
        }
        t.members = members;
        t.notes = "Copied from your game's party (read only: changes here stay in the app).".into();
        self.setup.team.teams().teams.push(t);
        self.setup.team.touch();
        self.go_to(super::Tab::Team);
        self.setup.team.open = Some(id);
        self.setup.team.slot = 0;
    }

    /// This Pokémon into a team (`team`: one with room, else a new team).
    fn boxes_to_team(&mut self, m: &Mon, team: Option<u64>) {
        let text = self.setup.boxes.text.clone().unwrap_or_default();
        let world = self.team_world();
        let Some(member) = self.boxes_member(m, &text, world) else { return };
        self.team_add_from(&member.species, team);
        let Some(id) = self.setup.team.open else { return };
        let slot = self.setup.team.slot;
        if let Some(t) = self.setup.team.teams().teams.iter_mut().find(|t| t.id == id).and_then(|t| t.members.get_mut(slot)) {
            *t = member;
        }
        self.setup.team.touch();
    }

    /// This Pokémon on your side of the Damage Calculator.
    fn boxes_to_calc(&mut self, m: &Mon) {
        let text = self.setup.boxes.text.clone().unwrap_or_default();
        let world = self.team_world();
        let Some(member) = self.boxes_member(m, &text, world) else { return };
        let c = &mut self.setup.team.calc;
        c.own = None;
        c.own_set = true;
        c.world = world;
        // the no-team "team" (id 0): it keeps the sides it's given
        c.team = Some(0);
        c.left = Some(super::team_calc::Poke::new(member, None));
        c.right = None;
        self.go_to(super::Tab::Calc);
    }

    /// The Breeding Planner's goal: one like this (its species, nature and
    /// Ability), on the Plan page.
    fn boxes_to_breed(&mut self, m: &Mon) {
        let Some(id) = crate::breed_save::species_id(m.species) else { return };
        let text = self.setup.boxes.text.clone().unwrap_or_default();
        let ability = ((m.ability as usize) < text.abilities.len()).then(|| text.ability_name(m.ability));
        let f = self.setup.breed.file();
        f.target = crate::breed_plan::Target { species: id, nature: Some(gamesave::NATURES[m.nature() as usize].to_string()), ability, ..Default::default() };
        self.setup.breed.touch();
        self.setup.breed.view = super::breed_home::BreedView::Plan;
        self.go_to(super::Tab::Breed);
        self.setup.scroll_top = true;
    }

    /// The New hunt guide, with this Pokémon picked (nothing changes until
    /// you start the hunt there).
    fn boxes_to_hunt(&mut self, slug: &str) {
        self.open_hunt_wizard_for(slug);
    }

    fn boxes_log_shiny(&mut self, m: &Mon, sp: &crate::species::Species, level: Option<u8>, text: &GameText) {
        self.open_log(Some(&sp.slug));
        let d = &mut self.setup.log.draft;
        if let Some(l) = level {
            d.mon.level = l;
            d.level_known = true;
        }
        d.mon.nature = gamesave::NATURES[m.nature() as usize].to_string();
        d.nature_known = true;
        d.mon.ivs = stat_order(m.ivs).map(|v| v as u16);
        d.mon.evs = stat_order(m.evs).map(|v| v as u16);
        d.stats_known = true;
        d.mon.moves = m.moves.iter().filter(|x| **x != 0 && (**x as usize) < text.moves.len()).map(|x| text.move_name(*x)).collect();
        if m.nicknamed {
            d.mon.nickname = m.nickname.clone();
        }
        d.mon.gender = if m.genderless { None } else if m.female { Some('F') } else { Some('M') };
        if (m.ability as usize) < text.abilities.len() {
            d.mon.ability = text.ability_name(m.ability);
        }
        if m.item != 0 && (m.item as usize) < text.items.len() {
            d.mon.item = text.item_name(m.item);
        }
        if let Some((y, mo, dd)) = m.met_date {
            let day = crate::stats::days_from_civil(2000 + y as i64, mo as u32, dd as u32);
            d.when_unix = (day * 86_400 + 12 * 3600 - crate::stats::utc_offset_secs()).max(0) as u64;
        }
        if let Some(place) = text.place_name(m.met_location) {
            d.place = place;
        }
    }
}

/// Its four moves, each on its type's colour with its category and power
/// (from the game's own move table).
fn boxes_moves_card(ui: &mut egui::Ui, moves: &[(String, Option<gamesave::MoveInfo>)]) {
    kit::card(ui, |ui| {
        ui.set_width(ui.available_width());
        super::pokedex::card_title(ui, "Moves");
        let full = ui.available_width();
        let gap = 8.0;
        let cw = ((full - gap) / 2.0).floor();
        for row in 0..2 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for col in 0..2 {
                    let (r, _) = ui.allocate_exact_size(vec2(cw, 46.0), Sense::hover());
                    let p = ui.painter();
                    let Some((name, info)) = moves.get(row * 2 + col) else {
                        p.rect_filled(r, 10.0, kit::sunken());
                        p.text(r.center(), Align2::CENTER_CENTER, "—", crate::fonts::sans_font(12.5), MUTED());
                        continue;
                    };
                    let tc = info.map_or(MUTED(), |i| super::pokedex::type_color(i.kind));
                    p.rect_filled(r, 10.0, theme::blend(kit::CARD(), tc, 0.14));
                    p.rect_filled(Rect::from_min_size(r.min + vec2(8.0, 10.0), vec2(4.0, r.height() - 20.0)), 2.0, tc);
                    let f = super::charts::bold(13.0);
                    p.text(pos2(r.min.x + 20.0, r.min.y + 15.0), Align2::LEFT_CENTER, super::charts::clip(ui, name, &f, cw - 28.0), f, TEXT());
                    let sub = match info {
                        Some(i) if i.power > 1 => format!("{} · {} · {} power", i.kind, i.cat, i.power),
                        Some(i) => format!("{} · {}", i.kind, i.cat),
                        None => String::new(),
                    };
                    let sf = crate::fonts::sans_font(11.5);
                    p.text(pos2(r.min.x + 20.0, r.min.y + 32.0), Align2::LEFT_CENTER, super::charts::clip(ui, &sub, &sf, cw - 28.0), sf, MUTED());
                }
            });
            if row == 0 {
                ui.add_space(gap);
            }
        }
    });
}

/// Its real stats as bars (party: what the game keeps; box: worked out the
/// same way the game does), with the nature's boost and cut and its IVs and
/// EVs on each line.
fn boxes_stats_card(
    ui: &mut egui::Ui,
    sp: &crate::species::Species,
    stats: Option<([u16; 6], [u16; 6], bool)>,
    level: Option<u8>,
    nat: Option<&crate::battle::Nature>,
    ivs: [u8; 6],
    evs: [u8; 6],
    grow: f32,
) {
    kit::card(ui, |ui| {
        ui.set_width(ui.available_width());
        super::pokedex::card_title(ui, "Stats");
        let Some((st, base, from_save)) = stats else {
            ui.label(RichText::new("Not known for this one.").color(MUTED()));
            return;
        };
        let names = ["HP", "Attack", "Defense", "Sp. Atk", "Sp. Def", "Speed"];
        let lv = level.unwrap_or(50);
        // a bar is full at what a superb stat would be at this level
        let full: [f32; 6] = std::array::from_fn(|i| crate::battle::stat(i, 180, 31, 252, lv, None, false) as f32);
        let w = ui.available_width();
        let (hr, _) = ui.allocate_exact_size(vec2(w, 14.0), Sense::hover());
        let (ivx, evx) = (hr.min.x + w - 56.0, hr.min.x + w);
        for (x, t) in [(ivx, "IV"), (evx, "EV")] {
            ui.painter().text(pos2(x, hr.center().y), Align2::RIGHT_CENTER, t, crate::fonts::sans_font(10.5), MUTED());
        }
        let up = Color32::from_rgb(240, 110, 90);
        let down = Color32::from_rgb(96, 150, 240);
        for i in 0..6 {
            let (r, resp) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
            let p = ui.painter();
            let mark = match nat {
                Some(n) if !n.neutral() && n.plus == i => Some(("▲", up)),
                Some(n) if !n.neutral() && n.minus == i => Some(("▼", down)),
                _ => None,
            };
            let lc = mark.map_or(MUTED(), |m| m.1);
            p.text(r.left_center(), Align2::LEFT_CENTER, names[i], super::charts::reg(12.5), lc);
            if let Some((s, c)) = mark {
                p.text(r.left_center() + vec2(58.0, 0.0), Align2::LEFT_CENTER, s, crate::fonts::sans_font(9.5), c);
            }
            p.text(r.left_center() + vec2(104.0, 0.0), Align2::RIGHT_CENTER, st[i].to_string(), super::charts::bold(13.5), TEXT());
            let track = Rect::from_min_max(pos2(r.left() + 116.0, r.center().y - 5.0), pos2(ivx - 34.0, r.center().y + 5.0));
            p.rect_filled(track, 5.0, theme::blend(kit::CARD(), TEXT(), 0.08));
            let f = kit::grow(ui.ctx(), egui::Id::new(("box-stat", sp.dex, i)), (st[i] as f32 / full[i]).clamp(0.03, 1.0));
            p.rect_filled(Rect::from_min_size(track.min, vec2(track.width() * f, 10.0)), 5.0, super::pokedex::stat_color(base[i] as u32));
            p.text(pos2(ivx, r.center().y), Align2::RIGHT_CENTER, ivs[i].to_string(), super::charts::bold(12.5), if ivs[i] == 31 { theme::GOLD } else { TEXT() });
            p.text(pos2(evx, r.center().y), Align2::RIGHT_CENTER, evs[i].to_string(), crate::fonts::sans_font(12.5), MUTED());
            resp.on_hover_text(format!("{}: {}  ·  base {} · IV {} · EV {}", names[i], st[i], base[i], ivs[i], evs[i]));
        }
        ui.add_space(8.0);
        let total: u32 = st.iter().map(|v| *v as u32).sum();
        let best = (1..6).max_by_key(|&i| st[i]).unwrap_or(1);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Total").size(12.5).color(MUTED()));
            ui.label(RichText::new(total.to_string()).font(super::charts::heavy(18.0)).color(TEXT()));
            ui.label(RichText::new(format!("· best at {}", names[best])).size(12.5).color(MUTED()));
            let ev: u32 = evs.iter().map(|v| *v as u32).sum();
            ui.label(RichText::new(format!("· {ev} / 510 EVs")).size(12.5).color(MUTED()));
        });
        let note = if from_save {
            format!("Its stats at Lv {lv}, straight from your save. ▲ ▼ are its nature.")
        } else {
            format!("Worked out for Lv {lv} from its IVs, EVs, nature and Platinum's base stats (the game only keeps stats for your party). ▲ ▼ are its nature.")
        };
        ui.add(egui::Label::new(RichText::new(note).size(11.5).color(MUTED())).wrap());
        ui.add_space(grow);
    });
}
