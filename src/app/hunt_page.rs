//! The Hunt page: what you're hunting and how. Your target (shiny next
//! to normal, where it lives, rare grass today), the method and the
//! things that change the odds, then the odds themselves with a chance
//! curve and how long each milestone should take at your pace, and the
//! session settings.

use super::charts::{self as ch, CYAN, GOLD, PINK, PURPLE};
use super::settings::{method_badge, method_experimental, method_icon, method_offered, method_short, EXPERIMENTAL_NOTE};
use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
use super::{fmt_one_in, App};
use crate::config::{BattleType, HuntMethod};
use crate::stats::fmt_duration;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

/// Most extra Pokémon in one hunt.
const MAX_ALSO: usize = 5;

/// Where a target lives, worked out from the Pokédex data.
struct Habitat {
    wiki_id: Option<String>,
    types: Vec<String>,
    /// "Route 203", "Grass · Lv 4–6", best chance %
    places: Vec<(String, String, u32)>,
    more: usize,
    note: Option<String>,
}

impl App {
    pub(super) fn page_hunt(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "HUNT", "What you're hunting and how. Your odds update as you change anything.");
        let w = ui.available_width();
        if w >= 980.0 {
            let gap = 18.0;
            let side_w = (w * 0.38).clamp(340.0, 440.0);
            let main_w = w - side_w - gap;
            let h = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(main_w, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(main_w);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    self.hunt_target(ui);
                    ui.add_space(18.0);
                    self.hunt_method(ui);
                });
                ui.allocate_ui_with_layout(vec2(side_w, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(side_w);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    self.hunt_odds(ui);
                    ui.add_space(18.0);
                    self.hunt_session(ui);
                });
            });
        } else {
            self.hunt_target(ui);
            ui.add_space(18.0);
            self.hunt_odds(ui);
            ui.add_space(18.0);
            self.hunt_method(ui);
            ui.add_space(18.0);
            self.hunt_session(ui);
        }
        ui.add_space(30.0);
    }

    fn habitat(&self, dex: u32) -> Option<Habitat> {
        let w = self.wiki.as_ref()?;
        let m = w.by_dex(dex)?;
        let wild = w.wild(&m.id);
        // one line per place: methods, levels and the best chance there
        let mut places: Vec<(String, Vec<String>, String, u32)> = Vec::new();
        for e in &wild {
            let place = w.place(&e.area).map(|p| p.name.clone()).unwrap_or_else(|| e.area.clone());
            let how = match e.method.as_str() {
                "grass" => "Grass",
                "radar" => "Poké Radar",
                "surf" => "Surfing",
                "old_rod" => "Old Rod",
                "good_rod" => "Good Rod",
                "super_rod" => "Super Rod",
                "cave" => "Cave",
                x => x,
            }
            .to_string();
            let chance = e.chance.morning.max(e.chance.day).max(e.chance.night);
            match places.iter_mut().find(|p| p.0 == place) {
                Some(p) => {
                    if !p.1.contains(&how) {
                        p.1.push(how);
                    }
                    p.3 = p.3.max(chance);
                }
                None => places.push((place, vec![how], e.levels.clone(), chance)),
            }
        }
        places.sort_by(|a, b| b.3.cmp(&a.3));
        let more = places.len().saturating_sub(3);
        let note = if places.is_empty() {
            let other = w.other(&m.id);
            if let Some(why) = w.unobtainable(&m.id) {
                Some(format!("Not obtainable: {why}"))
            } else if !other.is_empty() {
                Some("Not in the wild: a gift, egg or trade".into())
            } else if w.evolves_from(&m.id).is_some() {
                Some("Not in the wild: evolve it".into())
            } else {
                Some("Not found in the wild".into())
            }
        } else {
            None
        };
        Some(Habitat {
            wiki_id: Some(m.id.clone()),
            types: m.types.clone(),
            places: places
                .into_iter()
                .take(3)
                .map(|(p, how, lv, c)| {
                    let lv = if lv.is_empty() { String::new() } else { format!(" · Lv {}", lv.replace('-', "–")) };
                    (p, format!("{}{lv}", how.join(", ")), c)
                })
                .collect(),
            more,
            note,
        })
    }

    // ───────────────────────────── target ─────────────────────────────

    fn hunt_target(&mut self, ui: &mut egui::Ui) {
        let mut open_dex: Option<String> = None;
        let mut show_rare = false;
        let mut picked: Option<String> = None;
        kit::card(ui, |ui| {
            card_head(ui, "🎯", PINK, "Your target", None);
            ui.add_space(10.0);
            match self.target_idx() {
                None => {
                    ui.label(RichText::new("Pick the Pokémon you're hunting").font(ch::bold(16.0)).color(TEXT()));
                    ui.label(RichText::new("Search by name or Pokédex number. Your encounters are counted towards it from then on.").size(12.5).color(MUTED()));
                    ui.add_space(10.0);
                    picked = self.target_picker(ui, 360.0);
                }
                Some(i) => {
                    let sp = self.species[i].clone();
                    let hab = self.habitat(sp.dex);
                    let rare = self.rare_spots_for(&sp.slug);
                    let full = ui.available_width();
                    let art_w = if full >= 520.0 { 236.0 } else { 0.0 };
                    let (rect, _) = ui.allocate_exact_size(vec2(full, 150.0), Sense::hover());
                    let p = ui.painter_at(rect.expand(8.0));
                    if art_w > 0.0 {
                        // normal → shiny, so you know what to look for
                        let nc = pos2(rect.min.x + 46.0, rect.min.y + 66.0);
                        let sc = pos2(rect.min.x + 160.0, rect.min.y + 62.0);
                        p.circle_filled(nc, 38.0, theme::blend(kit::CARD(), TEXT(), 0.06));
                        self.draw_sprite_centered(&p, &sp, false, nc, vec2(64.0, 64.0));
                        p.circle_filled(sc, 62.0, ch::faint(GOLD, 0.07));
                        p.circle_filled(sc, 48.0, ch::faint(GOLD, 0.14));
                        self.draw_sprite_centered(&p, &sp, true, sc, vec2(92.0, 92.0));
                        theme::sparkle(&p, sc + vec2(40.0, -38.0), 8.0, GOLD);
                        theme::sparkle(&p, sc + vec2(-44.0, 30.0), 5.0, GOLD);
                        p.text(pos2(nc.x, rect.min.y + 132.0), Align2::CENTER_CENTER, "Normal", ch::reg(11.5), MUTED());
                        p.text(pos2(sc.x, rect.min.y + 132.0), Align2::CENTER_CENTER, "✨ Shiny", ch::bold(11.5), ch::ink(GOLD));
                        p.text(pos2((nc.x + sc.x) / 2.0 - 4.0, nc.y), Align2::CENTER_CENTER, "›", ch::bold(18.0), MUTED());
                    }
                    let x = rect.min.x + art_w + 8.0;
                    let room = rect.max.x - x;
                    let nr = p.text(pos2(x, rect.min.y + 4.0), Align2::LEFT_TOP, sp.name, ch::fit_font(ui, sp.name, ch::heavy, 28.0, 16.0, room - 70.0), TEXT());
                    p.text(pos2(nr.max.x + 10.0, nr.center().y + 2.0), Align2::LEFT_CENTER, format!("#{:04}", sp.dex), ch::reg(13.0), MUTED());
                    let mut cx = x;
                    let ty = rect.min.y + 46.0;
                    if let Some(h) = &hab {
                        for t in &h.types {
                            let c = super::pokedex::type_color(t);
                            let g = p.layout_no_wrap(t.to_uppercase(), ch::bold(10.5), Color32::WHITE);
                            let r = Rect::from_min_size(pos2(cx, ty), vec2(g.size().x + 16.0, 20.0));
                            p.rect_filled(r, 10.0, c);
                            p.galley(pos2(r.min.x + 8.0, r.center().y - g.size().y / 2.0), g, Color32::WHITE);
                            cx = r.max.x + 6.0;
                        }
                    }
                    // where it lives
                    let mut y = rect.min.y + 92.0;
                    match &hab {
                        Some(h) if !h.places.is_empty() => {
                            p.text(pos2(x, y - 8.0), Align2::LEFT_CENTER, "WHERE TO FIND IT", ch::bold(10.5), MUTED());
                            y += 12.0;
                            for (place, how, chance) in &h.places {
                                let pr = p.text(pos2(x, y), Align2::LEFT_CENTER, ch::clip(ui, place, &ch::bold(12.5), room * 0.5), ch::bold(12.5), TEXT());
                                let rest = if *chance > 0 { format!("{how} · {chance}%") } else { how.clone() };
                                p.text(pos2(pr.max.x + 8.0, y), Align2::LEFT_CENTER, ch::clip(ui, &rest, &ch::reg(12.0), rect.max.x - pr.max.x - 10.0), ch::reg(12.0), MUTED());
                                y += 19.0;
                            }
                            if h.more > 0 {
                                p.text(pos2(x, y), Align2::LEFT_CENTER, format!("+ {} more in the Pokédex", h.more), ch::reg(11.5), MUTED());
                            }
                        }
                        Some(h) => {
                            if let Some(n) = &h.note {
                                p.text(pos2(x, y + 4.0), Align2::LEFT_CENTER, ch::clip(ui, n, &ch::reg(12.5), room), ch::reg(12.5), MUTED());
                            }
                        }
                        None => {
                            p.text(pos2(x, y + 4.0), Align2::LEFT_CENTER, "Where to find it shows once the Pokédex has loaded.", ch::reg(12.0), MUTED());
                        }
                    }
                    if !rare.is_empty() {
                        ui.add_space(4.0);
                        let places = super::rare::places_text(&rare);
                        egui::Frame::none()
                            .fill(ch::faint(super::charts::GREEN, 0.12))
                            .rounding(14.0)
                            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("🌿").size(15.0));
                                    ui.label(RichText::new(format!("On rare grass today: {places}")).size(12.5).color(ch::ink(super::charts::GREEN)));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.link(RichText::new("Show on map ›").size(12.5)).clicked() {
                                            show_rare = true;
                                        }
                                    });
                                });
                            });
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let picking = self.setup.picking_target;
                        if picking {
                            if ui.add(egui::Button::new("Cancel").fill(kit::btn_fill())).clicked() {
                                self.setup.picking_target = false;
                            }
                        } else if kit::accent_button(ui, egui::Button::new(RichText::new("🔄  Change target").font(ch::bold(13.0)).color(kit::on_primary())), 4.0).clicked() {
                            self.setup.picking_target = true;
                            self.setup.picking_also = None;
                            self.setup.search.clear();
                        }
                        if let Some(id) = hab.as_ref().and_then(|h| h.wiki_id.clone()) {
                            if ui.add(egui::Button::new("📖  Pokédex entry").fill(kit::btn_fill())).clicked() {
                                open_dex = Some(id);
                            }
                        }
                    });
                    if self.setup.picking_target {
                        ui.add_space(12.0);
                        picked = self.target_picker(ui, 330.0);
                        ui.add_space(4.0);
                        ui.label(RichText::new("Switching target saves your current run. Find it again under Runs.").size(11.5).color(MUTED()));
                    }
                    ui.add_space(14.0);
                    divider(ui);
                    ui.add_space(12.0);
                    self.also_row(ui);
                }
            }
        });
        if let Some(slug) = picked {
            self.set_target(slug);
            self.setup.picking_target = false;
        }
        if let Some(id) = open_dex {
            self.open_dex_entry(&id);
        }
        if show_rare {
            if let Some(slug) = self.cfg.target.clone() {
                if let Some(s) = self.rare_spots_for(&slug).first().cloned() {
                    self.go_to_rare(&s);
                }
            }
        }
    }

    /// Other Pokémon hunted alongside the target, as chips.
    fn also_row(&mut self, ui: &mut egui::Ui) {
        let also = self.also_slugs();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Also hunting").font(ch::bold(13.5)).color(TEXT()));
            kit::info(ui, "Hunting more than one Pokémon on the same route? They all count as sightings, and Caught! asks which one it was.");
        });
        ui.add_space(6.0);
        let mut remove = None;
        let mut toggle_pick: Option<usize> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for (k, slug) in also.iter().enumerate() {
                let Some(sp) = self.species.iter().find(|s| &s.slug == slug).cloned() else { continue };
                let g = ui.painter().layout_no_wrap(sp.name.to_string(), ch::bold(12.5), TEXT());
                let size = vec2(34.0 + g.size().x + 34.0, 36.0);
                let (r, resp) = ui.allocate_exact_size(size, Sense::click());
                let on = self.setup.picking_also == Some(k);
                let ht = kit::anim(ui.ctx(), resp.id, resp.hovered() || on, kit::QUICK);
                let p = ui.painter();
                p.rect_filled(r, 18.0, theme::blend(kit::sunken(), TEXT(), 0.05 * ht));
                p.rect_stroke(r, 18.0, Stroke::new(1.0f32, if on { kit::primary() } else { kit::LINE() }));
                self.draw_sprite_centered(p, &sp, false, pos2(r.min.x + 19.0, r.center().y), vec2(30.0, 30.0));
                p.galley(pos2(r.min.x + 36.0, r.center().y - g.size().y / 2.0), g, TEXT());
                let xr = Rect::from_center_size(pos2(r.max.x - 17.0, r.center().y), vec2(22.0, 22.0));
                let xresp = ui.interact(xr, resp.id.with("x"), Sense::click()).on_hover_text("Stop hunting this one");
                let xh = kit::anim(ui.ctx(), xresp.id, xresp.hovered(), kit::QUICK);
                ui.painter().circle_filled(xr.center(), 10.0, theme::blend(kit::sunken(), theme::HP_RED, 0.25 * xh));
                ui.painter().text(xr.center(), Align2::CENTER_CENTER, "×", ch::bold(15.0), theme::blend(MUTED(), theme::HP_RED, xh));
                if xresp.clicked() {
                    remove = Some(k);
                } else if resp.on_hover_text("Change").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    toggle_pick = Some(k);
                }
            }
            if also.len() < MAX_ALSO {
                let adding = self.setup.picking_also == Some(also.len());
                let label = if adding { "Done" } else { "+ Add a Pokémon" };
                let g = ui.painter().layout_no_wrap(label.to_string(), ch::bold(12.5), TEXT());
                let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 28.0, 36.0), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id, resp.hovered(), kit::QUICK);
                let p = ui.painter();
                p.rect_filled(r, 18.0, theme::blend(kit::CARD(), TEXT(), 0.04 + 0.04 * ht));
                dashed_pill(p, r, theme::blend(kit::LINE(), TEXT(), 0.2 + 0.3 * ht));
                p.galley(pos2(r.center().x - g.size().x / 2.0, r.center().y - g.size().y / 2.0), g, if adding { kit::primary() } else { TEXT() });
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    toggle_pick = Some(also.len());
                }
            }
        });
        if also.is_empty() && self.setup.picking_also.is_none() {
            ui.add_space(2.0);
            ui.label(RichText::new("Optional. They all count as sightings, and Caught! asks which one.").size(11.5).color(MUTED()));
        }
        if let Some(k) = toggle_pick {
            self.setup.picking_also = if self.setup.picking_also == Some(k) { None } else { Some(k) };
            self.setup.picking_target = false;
            self.setup.search.clear();
        }
        if let Some(k) = remove {
            let mut list = also.clone();
            list.remove(k);
            self.set_also(list);
            self.setup.picking_also = None;
        }
        if let Some(k) = self.setup.picking_also {
            ui.add_space(10.0);
            if let Some(slug) = self.target_picker(ui, 260.0) {
                let mut list = self.also_slugs();
                if self.cfg.target.as_deref() != Some(slug.as_str()) && !list.contains(&slug) {
                    if k < list.len() {
                        list[k] = slug;
                    } else {
                        list.push(slug);
                    }
                    self.set_also(list);
                }
                self.setup.picking_also = None;
            }
        }
    }

    // ───────────────────────────── method ─────────────────────────────

    fn hunt_method(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "⚔", PURPLE, "How you're hunting", None);
            ui.add_space(12.0);
            let methods: Vec<HuntMethod> = HuntMethod::ALL.into_iter().filter(|&m| method_offered(m) || m == self.cfg.hunt_method).collect();
            let full = ui.available_width();
            let per = if full >= 500.0 { 3 } else { 2 };
            let gap = 10.0;
            let tw = ((full - gap * (per as f32 - 1.0)) / per as f32).floor();
            for chunk in methods.chunks(per) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &m in chunk {
                        if method_tile(ui, tw, m, self.cfg.hunt_method == m, &method_badge(&self.cfg, m)) {
                            self.cfg.hunt_method = m;
                            self.dirty = true;
                        }
                    }
                });
                ui.add_space(gap);
            }
            // what the chosen method means
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(self.cfg.hunt_method.note()).size(12.0).color(MUTED()));
            });
            if method_experimental(self.cfg.hunt_method) {
                ui.add_space(6.0);
                kit::pill(ui, &format!("⚠ {EXPERIMENTAL_NOTE}"), PillKind::Warn);
            }
            match self.cfg.hunt_method {
                HuntMethod::PokeRadar => {
                    ui.add_space(10.0);
                    setting_row(ui, "Current chain", "Odds stop improving at a chain of 40.", |ui| {
                        self.dirty |= kit::stepper(ui, &mut self.cfg.radar_chain, 0, 999);
                    });
                }
                HuntMethod::Custom => {
                    ui.add_space(10.0);
                    setting_row(ui, "Your odds", "The chance of a shiny, as 1 in this many.", |ui| {
                        self.dirty |= ui.add(egui::DragValue::new(&mut self.cfg.odds).range(1..=65536).prefix("1 in ")).changed();
                    });
                }
                HuntMethod::FromGameLog => {
                    ui.add_space(10.0);
                    match self.cfg.last_log_odds {
                        Some((n, d)) => kit::pill(ui, &format!("✔ Game says {n} in {d} (1 in {})", fmt_one_in(d as f64 / n.max(1) as f64)), PillKind::Ok),
                        None => kit::pill(ui, "Waiting for your first battle", PillKind::Warn),
                    };
                }
                _ => {}
            }
            ui.add_space(14.0);
            divider(ui);
            ui.add_space(12.0);
            if self.cfg.hunt_method.is_wild_battles() {
                let mut changed = false;
                setting_row(ui, "Battles", "Doubles: 2 Pokémon per battle, each gets its own shiny chance.\nNot sure: counts however many enemy names are on screen.", |ui| {
                    changed = kit::segmented(ui, &mut self.cfg.battle_type, &[(BattleType::Doubles, "Doubles"), (BattleType::Singles, "Singles"), (BattleType::Auto, "Not sure")]);
                });
                if changed {
                    self.dirty = true;
                    self.push_settings(true);
                }
                ui.add_space(10.0);
            }
            if self.cfg.hunt_method.uses_charms() {
                let charm_help = if self.cfg.hunt_method == HuntMethod::Masuda {
                    "For Eggs the Shiny Charm gives 2 more tries (8 with the Masuda Method). More than one isn't known to add more."
                } else {
                    "Each Shiny Charm gives wild Pokémon one more chance to be shiny."
                };
                setting_row(ui, "Shiny Charms in your Bag", charm_help, |ui| {
                    self.dirty |= kit::stepper(ui, &mut self.cfg.shiny_charms, 0, 20);
                });
                ui.add_space(12.0);
            }
            self.dirty |= kit::toggle_row(
                ui,
                "Switch method automatically",
                "Changes to Rare grass when the game log shows Sweet Scent on rare grass, and to Soft resets when you pick a starter or reset into a static Pokémon. A short pop-up says when it switches. Your count isn't touched.",
                &mut self.cfg.auto_method,
            );
            if self.cfg.hunt_method.is_wild_battles() && self.cfg.target.is_some() {
                ui.add_space(10.0);
                let name = self.target_name(self.cfg.target.as_deref());
                self.dirty |= kit::toggle_row(
                    ui,
                    &format!("Odds count only {name}"),
                    &format!(
                        "Only {name} encounters can give you a shiny {name}. On, the odds bar and share images use the {name} you've seen ({} so far) instead of every encounter.",
                        theme::thousands(self.cfg.target_seen)
                    ),
                    &mut self.cfg.odds_target_only,
                );
            }
        });
    }

    // ───────────────────────────── odds ─────────────────────────────

    fn hunt_odds(&mut self, ui: &mut egui::Ui) {
        let (count, what) = self.odds_count();
        let one_in = self.cfg.odds_one_in().filter(|o| *o > 1.0);
        let per_hour = self.cfg.run_stats.per_hour();
        kit::card(ui, |ui| {
            card_head(ui, "🍀", GOLD, "Your odds", None);
            ui.add_space(12.0);
            let Some(o) = one_in else {
                ui.label(RichText::new("Your odds show after your first battle.").size(13.0).color(MUTED()));
                return;
            };
            // headline
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new("1 in").font(ch::bold(16.0)).color(MUTED()));
                ui.label(RichText::new(fmt_one_in(o)).font(ch::heavy(36.0)).color(TEXT()));
            });
            let per = if !self.cfg.hunt_method.is_wild_battles() {
                if self.cfg.hunt_method == HuntMethod::Masuda { "per egg".to_string() } else { "per reset".to_string() }
            } else {
                match (self.cfg.per_battle_one_in(), self.cfg.pokemon_per_battle()) {
                    (Some(b), Some(n)) if n > 1.0 => format!("per Pokémon · 1 in {} per {} battle", fmt_one_in(b), if n == 2.0 { "double" } else { "" }).replace("per  battle", "per battle"),
                    _ => "per Pokémon".to_string(),
                }
            };
            ui.label(RichText::new(per).size(12.5).color(MUTED()));
            ui.add_space(14.0);
            chance_curve(ui, o, count, &what);
            ui.add_space(14.0);
            // milestones
            let need = |pr: f64| ((1.0 - pr).ln() / (1.0 - 1.0 / o).ln()).ceil() as u64;
            for (pr, label) in [(0.5, "Even chance"), (0.9, "90% chance"), (0.99, "99% chance")] {
                let n = need(pr);
                let left = n.saturating_sub(count);
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
                let p = ui.painter();
                let done = count >= n;
                let dot = pos2(r.min.x + 10.0, r.center().y);
                p.circle_filled(dot, 5.0, if done { ch::faint(GOLD, 0.9) } else { theme::blend(kit::CARD(), TEXT(), 0.18) });
                p.text(pos2(r.min.x + 26.0, r.center().y - 9.0), Align2::LEFT_CENTER, label, ch::bold(13.0), TEXT());
                p.text(pos2(r.min.x + 26.0, r.center().y + 9.0), Align2::LEFT_CENTER, format!("by {} {}", theme::thousands(n), what), ch::reg(12.0), MUTED());
                let right = if done {
                    "Passed".to_string()
                } else {
                    match per_hour.filter(|r| *r > 0.0) {
                        Some(rate) => format!("{} to go · ~{}", theme::thousands(left), fmt_duration((left as f64 / rate * 3600.0) as u64)),
                        None => format!("{} to go", theme::thousands(left)),
                    }
                };
                p.text(pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, right, if done { ch::bold(12.5) } else { ch::reg(12.5) }, if done { ch::ink(GOLD) } else { MUTED() });
            }
            if per_hour.is_some() {
                ui.add_space(4.0);
                ui.label(RichText::new("Times are at your pace this hunt.").size(11.5).color(MUTED()));
            }
        });
    }

    // ───────────────────────────── session ─────────────────────────────

    fn hunt_session(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "⏱", CYAN, "Sessions & breaks", None);
            ui.add_space(12.0);
            self.dirty |= kit::toggle_row(ui, "Break reminder", "A gentle nudge on the overlay after a long stretch of hunting in one session.", &mut self.cfg.break_reminder);
            if self.cfg.break_reminder {
                ui.add_space(6.0);
                setting_row(ui, "Remind me after", "Minutes of hunting in one session.", |ui| {
                    self.dirty |= ui.add(egui::DragValue::new(&mut self.cfg.break_minutes).range(15..=600).speed(5.0).suffix(" min")).changed();
                });
            }
            ui.add_space(12.0);
            divider(ui);
            ui.add_space(12.0);
            setting_row(ui, "Session goal", "A pat on the back when a session reaches this many encounters. 0 = off.", |ui| {
                self.dirty |= ui
                    .add(
                        egui::DragValue::new(&mut self.cfg.session_goal)
                            .range(0..=100_000)
                            .speed(10.0)
                            .custom_formatter(|v, _| if v < 1.0 { "Off".into() } else { format!("{} enc.", theme::thousands(v as u64)) })
                            .custom_parser(|t| {
                                let t = t.trim().to_lowercase();
                                if t == "off" { Some(0.0) } else { t.trim_end_matches("enc.").trim().replace(',', "").parse().ok() }
                            }),
                    )
                    .changed();
            });
            ui.add_space(12.0);
            divider(ui);
            ui.add_space(12.0);
            self.dirty |= kit::toggle_row(
                ui,
                "Pause the timer when you stop",
                "Hunt time stops when no encounter comes for a while (a break, a menu, the phone) and carries on with the next one. Counting keeps going.",
                &mut self.cfg.idle_pause,
            );
            if self.cfg.idle_pause {
                ui.add_space(8.0);
                let mut m = self.cfg.idle_minutes;
                ui.label(RichText::new("Pause after this long with no encounter").size(12.5).color(MUTED()));
                ui.add_space(6.0);
                kit::segmented(ui, &mut m, &[(2, "2 min"), (3, "3 min"), (5, "5 min"), (10, "10 min"), (15, "15 min")]);
                if m != self.cfg.idle_minutes {
                    self.cfg.idle_minutes = m;
                    self.dirty = true;
                }
            }
        });
    }
}

/// A card's title row: icon chip, title, optional muted note on the right.
fn card_head(ui: &mut egui::Ui, icon: &str, col: Color32, title: &str, right: Option<&str>) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        kit::icon_chip(ui.painter(), r, icon, col, false);
        ui.add_space(4.0);
        ui.label(RichText::new(title).font(ch::bold(16.0)).color(TEXT()));
        if let Some(t) = right {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(t).size(12.0).color(MUTED()));
            });
        }
    });
}

fn divider(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, kit::LINE()));
}

/// A label (with a help ⓘ) on the left and a control on the right. Laid
/// out left to right (switches and steppers draw wrong right to left), with
/// the control's width measured the frame before so it sits flush right.
fn setting_row(ui: &mut egui::Ui, label: &str, help: &str, control: impl FnOnce(&mut egui::Ui)) {
    let id = ui.id().with(("setting-row-w", label));
    let cw = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(120.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(TEXT()));
        kit::info(ui, help);
        ui.add_space((ui.available_width() - cw).max(8.0));
        let start = ui.cursor().min.x;
        control(ui);
        let used = ui.min_rect().max.x - start;
        if (used - cw).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(id, used));
            ui.ctx().request_repaint();
        }
    });
}

/// A pill outline drawn in dashes ("add" chips).
fn dashed_pill(p: &egui::Painter, r: Rect, c: Color32) {
    let rad = r.height() / 2.0;
    let mut pts = vec![];
    let n = 48;
    for k in 0..=n {
        let t = k as f32 / n as f32 * std::f32::consts::TAU;
        let (s, co) = t.sin_cos();
        let cx = if co >= 0.0 { r.max.x - rad } else { r.min.x + rad };
        pts.push(pos2(cx + co * rad, r.center().y + s * rad));
    }
    // stretch the round ends into a pill: straight runs join the halves
    let path: Vec<egui::Pos2> = pts;
    p.extend(egui::Shape::dashed_line(&path, Stroke::new(1.2f32, c), 4.0, 3.0));
}

/// A method choice: icon, name, what it is and its odds.
fn method_tile(ui: &mut egui::Ui, w: f32, m: HuntMethod, sel: bool, badge: &str) -> bool {
    let h = 84.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let hov = resp.hovered();
    let t = kit::anim(ui.ctx(), resp.id.with("sel"), sel, kit::BASE);
    let ht = kit::anim(ui.ctx(), resp.id.with("hov"), hov && !sel, kit::QUICK);
    let p = ui.painter_at(rect.expand(4.0));
    let fill = theme::blend(theme::blend(kit::CARD(), TEXT(), 0.035 * ht), kit::primary(), 0.10 * t);
    p.rect_filled(rect, 18.0, fill);
    p.rect_stroke(rect, 18.0, Stroke::new(1.0 + t, theme::blend(theme::blend(kit::LINE(), TEXT(), 0.18 * ht), kit::primary(), t)));
    let chip = Rect::from_min_size(rect.min + vec2(14.0, 14.0), vec2(32.0, 32.0));
    kit::icon_chip(&p, chip, method_icon(m), if sel { kit::primary() } else { MUTED() }, false);
    let (title, sub) = method_short(m);
    let x = chip.max.x + 10.0;
    let room = rect.max.x - 12.0 - x;
    p.text(pos2(x, chip.min.y + 1.0), Align2::LEFT_TOP, ch::clip(ui, title, &ch::bold(13.5), room - 18.0), ch::bold(13.5), TEXT());
    p.text(pos2(x, chip.min.y + 19.0), Align2::LEFT_TOP, ch::clip(ui, sub, &ch::reg(11.5), room), ch::reg(11.5), MUTED());
    let off = super::settings::method_unavailable(m);
    let bc = if sel { kit::primary() } else { MUTED() };
    let g = p.layout_no_wrap(badge.to_string(), ch::bold(11.0), ch::ink(bc));
    let br = Rect::from_min_size(pos2(rect.min.x + 14.0, rect.max.y - 30.0), vec2(g.size().x + 14.0, 20.0));
    if !off {
        p.rect_filled(br, 10.0, ch::faint(bc, 0.14));
        p.galley(pos2(br.min.x + 7.0, br.center().y - g.size().y / 2.0), g, ch::ink(bc));
    }
    if method_experimental(m) && !off {
        let wr = Rect::from_min_size(pos2(br.max.x + 6.0, br.min.y), vec2(20.0, 20.0));
        p.circle_filled(wr.center(), 9.0, ch::faint(theme::HP_YELLOW, 0.2));
        p.text(wr.center(), Align2::CENTER_CENTER, "!", ch::bold(12.0), ch::ink(theme::HP_YELLOW));
    }
    if t > 0.0 {
        let c = pos2(rect.max.x - 18.0, rect.min.y + 18.0);
        p.circle_filled(c, 9.0 * t, kit::primary());
        p.text(c, Align2::CENTER_CENTER, "✔", egui::FontId::proportional(10.0 * t.max(0.01)), kit::on_primary());
    }
    if super::settings::method_unavailable(m) {
        // shown, but greyed out and not pickable
        super::settings::grey_tile(ui, rect, 18.0);
        let tip = format!("{}\n\n{}", super::settings::UNAVAILABLE_NOTE, m.note());
        resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::NotAllowed);
        return false;
    }
    let tip = if method_experimental(m) { format!("{}\n\n⚠ {EXPERIMENTAL_NOTE}", m.note()) } else { m.note().to_string() };
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !sel
}

/// Chance of having found the shiny by each encounter count, from 0 to 3×
/// the odds, with where you are now. Hover to read any point.
fn chance_curve(ui: &mut egui::Ui, o: f64, count: u64, what: &str) {
    let w = ui.available_width();
    let h = 150.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter_at(rect.expand(6.0));
    let plot = Rect::from_min_max(pos2(rect.min.x + 34.0, rect.min.y + 8.0), pos2(rect.max.x - 6.0, rect.max.y - 22.0));
    let max_n = (o * 3.0).max(count as f64 * 1.08);
    let f = |n: f64| 1.0 - (1.0 - 1.0 / o).powf(n);
    let to_x = |n: f64| plot.min.x + (n / max_n) as f32 * plot.width();
    let to_y = |pr: f64| plot.max.y - pr as f32 * plot.height();
    let grid = theme::blend(kit::CARD(), TEXT(), 0.08);
    for (pr, lab) in [(0.0, "0%"), (0.5, "50%"), (0.9, "90%"), (1.0, "")] {
        let y = to_y(pr);
        if pr == 0.5 || pr == 0.9 {
            p.extend(egui::Shape::dashed_line(&[pos2(plot.min.x, y), pos2(plot.max.x, y)], Stroke::new(1.0f32, grid), 4.0, 4.0));
        } else {
            p.line_segment([pos2(plot.min.x, y), pos2(plot.max.x, y)], Stroke::new(1.0f32, grid));
        }
        if !lab.is_empty() {
            p.text(pos2(plot.min.x - 6.0, y), Align2::RIGHT_CENTER, lab, ch::reg(10.5), MUTED());
        }
    }
    for k in 1..=3 {
        let n = o * k as f64;
        if n > max_n {
            break;
        }
        let x = to_x(n);
        p.line_segment([pos2(x, plot.min.y), pos2(x, plot.max.y)], Stroke::new(1.0f32, grid));
        let al = if k == 3 { Align2::RIGHT_TOP } else { Align2::CENTER_TOP };
        p.text(pos2(x, plot.max.y + 6.0), al, ch::short_num(n), ch::reg(10.5), MUTED());
    }
    p.text(pos2(plot.min.x, plot.max.y + 6.0), Align2::LEFT_TOP, "0", ch::reg(10.5), MUTED());
    // the curve, grown in when the page opens
    let grow = kit::grow(ui.ctx(), egui::Id::new("hunt-chance-curve"), 1.0) as f64;
    let steps = 90;
    let col = kit::primary();
    let mut line = Vec::with_capacity(steps + 1);
    for k in 0..=steps {
        let n = max_n * k as f64 / steps as f64;
        let pt = pos2(to_x(n), to_y(f(n) * grow));
        line.push(pt);
    }
    // area under the curve as thin vertical strips (the shape isn't convex)
    for win in line.windows(2) {
        let (a, b) = (win[0], win[1]);
        let mesh_rect = [a, b, pos2(b.x, plot.max.y), pos2(a.x, plot.max.y)];
        p.add(egui::Shape::convex_polygon(mesh_rect.to_vec(), ch::faint(col, 0.10), Stroke::NONE));
    }
    p.add(egui::Shape::line(line, Stroke::new(2.2f32, col)));
    // you are here
    let cn = (count as f64).min(max_n);
    let you = pos2(to_x(cn), to_y(f(cn) * grow));
    p.line_segment([pos2(you.x, you.y), pos2(you.x, plot.max.y)], Stroke::new(1.0f32, ch::faint(GOLD, 0.6)));
    p.circle_filled(you, 6.0, kit::CARD());
    p.circle_filled(you, 4.5, GOLD);
    let tag = format!("You · {:.0}%", f(count as f64) * 100.0);
    let left = you.x > plot.center().x;
    let g = p.layout_no_wrap(tag, ch::bold(11.5), ch::ink(GOLD));
    let tw = g.size().x + 14.0;
    let tx = if left { you.x - 10.0 - tw } else { you.x + 10.0 };
    let tr = Rect::from_min_size(pos2(tx, (you.y - 32.0).max(plot.min.y)), vec2(tw, 20.0));
    p.rect_filled(tr, 10.0, kit::CARD());
    p.rect_stroke(tr, 10.0, Stroke::new(1.0f32, ch::faint(GOLD, 0.5)));
    p.galley(pos2(tr.min.x + 7.0, tr.center().y - g.size().y / 2.0), g, ch::ink(GOLD));
    // hover readout
    if let Some(pos) = resp.hover_pos().filter(|q| plot.expand2(vec2(0.0, 10.0)).contains(*q)) {
        let n = ((pos.x - plot.min.x) / plot.width()).clamp(0.0, 1.0) as f64 * max_n;
        let hp = pos2(to_x(n), to_y(f(n)));
        p.line_segment([pos2(hp.x, plot.min.y), pos2(hp.x, plot.max.y)], Stroke::new(1.0f32, ch::faint(TEXT(), 0.35)));
        p.circle_filled(hp, 4.0, col);
        egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), egui::Id::new("hunt-curve-tip"), |ui| {
            ui.label(RichText::new(format!("{:.0}% by {} {}", f(n) * 100.0, theme::thousands(n.round() as u64), what)).size(12.5));
        });
    }
}
