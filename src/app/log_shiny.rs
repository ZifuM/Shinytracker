//! Log a shiny: one you caught without tracking it, with whatever you know
//! about it (nature, ability, IVs, EVs, moves, encounters…). It goes in the
//! Shiny Dex and your shiny lists; it never makes a run, and adds no
//! encounters, XP or achievements.

use super::pokedex::type_color;
use super::store::pill_button;
use super::team_build::{holdable, learnable, nature_label, best_ways};
use super::team_home::{caps, cat_tag, chip, small, type_tag, Picker, ACCENT};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::battle::{self, World};
use crate::config::{HuntMethod, LoggedShiny};
use crate::stats::{civil_from_days, days_from_civil, fmt_date, local_day, now_unix, utc_offset_secs};
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

#[derive(Default)]
pub(super) struct LogState {
    pub open: bool,
    pub draft: LoggedShiny,
    /// The draft is a shiny already in the log (else a new one).
    pub editing: bool,
    /// Picking a species, an item or a move.
    pub picker: Option<Picker>,
    pub query: String,
    /// Delete was pressed once (press again to confirm).
    pub del_armed: bool,
    /// Put the cursor in the picker's search box.
    pub focus: bool,
}

/// The world the Pokémon's abilities, stats and moves come from.
fn world_of(app: &App) -> World {
    if app.cfg.info_modern {
        World::Modern
    } else {
        World::Platinum
    }
}

/// Local midday of a day number, as unix time (dates only, so midday keeps
/// the day the same in any time zone change).
fn day_unix(day: i64) -> u64 {
    (day * 86_400 + 12 * 3600 - utc_offset_secs()).max(0) as u64
}

impl App {
    /// Log a new shiny (of `slug`, if known).
    pub(super) fn open_log(&mut self, slug: Option<&str>) {
        let mut d = LoggedShiny { when_unix: now_unix(), ..Default::default() };
        d.mon.nature.clear();
        d.mon.level = 50;
        if let Some(slug) = slug {
            self.log_set_species(&mut d, slug);
        }
        self.setup.log = LogState { open: true, draft: d, picker: if slug.is_none() { Some(Picker::Species) } else { None }, focus: true, ..Default::default() };
    }

    /// Edit a logged shiny.
    pub(super) fn edit_log(&mut self, id: u64) {
        if let Some(l) = self.cfg.logged.iter().find(|l| l.id == id) {
            self.setup.log = LogState { open: true, draft: l.clone(), editing: true, ..Default::default() };
        }
    }

    /// Point the draft at a species (by Pokédex slug), keeping what still fits.
    fn log_set_species(&mut self, d: &mut LoggedShiny, slug: &str) {
        let Some(sp) = self.species.iter().find(|s| s.slug == slug) else { return };
        d.slug = sp.slug.clone();
        d.mon.species = sp.name.to_string();
        d.mon.shiny = true;
        let world = world_of(self);
        if let Some(b) = battle::data().species(&d.mon.species) {
            let abs = b.abilities_in(world);
            if !abs.contains(&d.mon.ability) {
                d.mon.ability = if abs.len() == 1 { abs[0].clone() } else { String::new() };
            }
        }
    }

    /// The Log a shiny window, over everything.
    pub(super) fn log_window(&mut self, ctx: &egui::Context) {
        if !self.setup.log.open {
            return;
        }
        let screen = ctx.screen_rect();
        let size = vec2((screen.width() - 80.0).clamp(360.0, 1120.0), (screen.height() - 70.0).clamp(320.0, 860.0));
        let panel = Rect::from_center_size(screen.center(), size);
        let mut close = false;
        let mut save = false;
        let mut delete = false;
        egui::Area::new(egui::Id::new("log-shiny")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(150));
            if scrim.clicked() && self.setup.log.picker.is_none() {
                close = true;
            }
            ui.painter().add(kit::shadow_tier(3).as_shape(panel, 22.0));
            ui.painter().rect_filled(panel, 22.0, CARD());
            ui.painter().rect_stroke(panel, 22.0, Stroke::new(1.0f32, kit::CONTROL()));
            ui.interact(panel, egui::Id::new("log-shiny-panel"), Sense::click());
            let inner = panel.shrink2(vec2(26.0, 22.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                // header
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let title = if self.setup.log.editing { "Edit logged shiny" } else { "Log a shiny" };
                        ui.label(RichText::new(format!("✨  {title}")).font(crate::fonts::sans_bold_font(22.0)).color(TEXT()));
                        ui.label(
                            RichText::new("For one you caught without tracking it. Fill in what you know and leave the rest. It goes in your Shiny Dex; it doesn't make a run.")
                                .size(kit::fs(12.5))
                                .color(MUTED()),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                        if pill_button(ui, r, "×", false, "log-x").on_hover_text("Close without saving (Esc)").clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(14.0);
                // body, above the footer
                let foot_h = 46.0;
                let body_h = ui.available_height() - foot_h - 12.0;
                let mut d = self.setup.log.draft.clone();
                ui.allocate_ui_with_layout(vec2(ui.available_width(), body_h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_min_height(body_h);
                    match self.setup.log.picker {
                        Some(pk) => self.log_picker(ui, &mut d, pk),
                        None => self.log_form(ui, &mut d),
                    }
                });
                self.setup.log.draft = d;
                // footer
                ui.add_space(12.0);
                let (fr, _) = ui.allocate_exact_size(vec2(ui.available_width(), foot_h), Sense::hover());
                ui.painter().hline(fr.x_range(), fr.min.y - 6.0, Stroke::new(1.0f32, LINE()));
                let have_species = !self.setup.log.draft.slug.is_empty();
                let sb = Rect::from_min_size(pos2(fr.max.x - 170.0, fr.min.y + 4.0), vec2(170.0, 38.0));
                let label = if self.setup.log.editing { "Save changes" } else { "✨  Save shiny" };
                if have_species {
                    if pill_button(ui, sb, label, true, "log-save").clicked() {
                        save = true;
                    }
                } else {
                    ui.scope(|ui| {
                        ui.disable();
                        pill_button(ui, sb, label, false, "log-save").on_disabled_hover_text("Pick the Pokémon first");
                    });
                }
                let cb = Rect::from_min_size(pos2(sb.min.x - 106.0, fr.min.y + 4.0), vec2(96.0, 38.0));
                if pill_button(ui, cb, "Cancel", false, "log-cancel").clicked() {
                    close = true;
                }
                if self.setup.log.editing {
                    let armed = self.setup.log.del_armed;
                    let db = Rect::from_min_size(pos2(fr.min.x, fr.min.y + 4.0), vec2(if armed { 150.0 } else { 96.0 }, 38.0));
                    if pill_button(ui, db, if armed { "Sure? Delete" } else { "Delete" }, false, "log-del").on_hover_text("Take this shiny out of your log").clicked() {
                        if armed {
                            delete = true;
                        } else {
                            self.setup.log.del_armed = true;
                        }
                    }
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.setup.log.picker.is_some() && !self.setup.log.draft.slug.is_empty() {
                self.setup.log.picker = None;
            } else {
                close = true;
            }
        }
        if save {
            let mut d = self.setup.log.draft.clone();
            if !d.stats_known {
                d.mon.ivs = [31; 6];
                d.mon.evs = [0; 6];
            }
            let name = if d.mon.nickname.trim().is_empty() { d.mon.species.clone() } else { d.mon.nickname.trim().to_string() };
            if self.setup.log.editing {
                if let Some(x) = self.cfg.logged.iter_mut().find(|l| l.id == d.id) {
                    *x = d;
                }
                self.app_toast(format!("Saved {name}"));
            } else {
                d.id = self.cfg.logged.iter().map(|l| l.id).max().unwrap_or(0) + 1;
                d.logged_unix = now_unix();
                // it now has details: the bare hand mark isn't needed
                self.cfg.dex_manual.retain(|s| *s != d.slug);
                self.cfg.logged.push(d);
                self.app_toast(format!("Shiny {name} logged in your Shiny Dex"));
            }
            self.dirty = true;
            close = true;
        }
        if delete {
            let id = self.setup.log.draft.id;
            self.cfg.logged.retain(|l| l.id != id);
            self.dirty = true;
            self.app_toast("Logged shiny deleted".into());
            close = true;
        }
        if close {
            self.setup.log = LogState::default();
        }
    }

    /// The form: the Pokémon on the left, its moves and stats on the right.
    fn log_form(&mut self, ui: &mut egui::Ui, d: &mut LoggedShiny) {
        let world = world_of(self);
        let sp = battle::data().species(&d.mon.species);
        let avail = ui.available_width();
        let wide = avail >= 900.0;
        let gap = 16.0;
        let left_w = if wide { 392.0 } else { avail };
        egui::ScrollArea::vertical().id_salt("log-form").auto_shrink([false, false]).show(ui, |ui| {
            let left = |app: &mut App, ui: &mut egui::Ui, d: &mut LoggedShiny| {
                app.log_who(ui, d, world, left_w);
                ui.add_space(gap);
                app.log_catch(ui, d, left_w);
            };
            let right = |app: &mut App, ui: &mut egui::Ui, d: &mut LoggedShiny| {
                let Some(sp) = sp else {
                    kit::card(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(small("Pick the Pokémon to fill in its moves and stats."));
                    });
                    return;
                };
                let wiki = app.wiki.clone();
                let mut open: Option<Picker> = None;
                app.team_moves_card(ui, world, sp, &mut d.mon, wiki.as_deref(), &mut open);
                if open.is_some() {
                    app.setup.log.picker = open;
                    app.setup.log.query.clear();
                    app.setup.log.focus = true;
                }
                ui.add_space(gap);
                app.log_stats(ui, d, world, sp);
                ui.add_space(gap);
                kit::card(ui, |ui| {
                    ui.label(caps("NOTES"));
                    ui.add_space(4.0);
                    let nw = ui.available_width();
                    ui.add(egui::TextEdit::multiline(&mut d.mon.notes).hint_text("The story of the catch, the ball, who you were with…").desired_rows(2).desired_width(nw));
                });
            };
            if wide {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.allocate_ui_with_layout(vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(left_w);
                        left(self, ui, d);
                    });
                    let rw = ui.available_width() - 8.0;
                    ui.allocate_ui_with_layout(vec2(rw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_width(rw);
                        right(self, ui, d);
                    });
                });
            } else {
                left(self, ui, d);
                ui.add_space(gap);
                right(self, ui, d);
            }
        });
    }

    /// The Pokémon: sprite, nickname, level, gender, ability, item.
    fn log_who(&mut self, ui: &mut egui::Ui, d: &mut LoggedShiny, world: World, w: f32) {
        let sp = battle::data().species(&d.mon.species);
        let _ = w;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 120.0), Sense::click());
            let ty: Vec<String> = sp.map(|s| s.types_in(world).to_vec()).unwrap_or_default();
            let col = ty.first().map(|x| type_color(x)).unwrap_or(GOLD);
            let p = ui.painter().clone();
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            p.rect_filled(r, 12.0, theme::blend(kit::sunken(), col, 0.12 + 0.08 * ht));
            let c = pos2(r.min.x + 60.0, r.center().y);
            if d.slug.is_empty() || !self.team_sprite(&p, &d.mon.species, true, c, vec2(104.0, 104.0)) {
                p.circle_filled(c, 40.0, theme::blend(kit::sunken(), col, 0.25));
                p.text(c, Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(26.0), MUTED());
            }
            theme::sparkle(&p, pos2(r.min.x + 100.0, r.min.y + 20.0), 6.0, GOLD);
            let x = r.min.x + 124.0;
            let f = crate::fonts::sans_bold_font(17.0);
            let name = if d.slug.is_empty() { "Pick a Pokémon".to_string() } else { d.mon.species.clone() };
            p.text(pos2(x, r.min.y + 30.0), Align2::LEFT_CENTER, super::charts::clip(ui, &name, &f, r.max.x - x - 8.0), f, TEXT());
            if let Some(s) = sp {
                p.text(pos2(x, r.min.y + 50.0), Align2::LEFT_CENTER, format!("#{:03}", s.num), crate::fonts::sans_font(11.5), MUTED());
                let mut tx = x;
                for t in &ty {
                    tx += type_tag(&p, pos2(tx, r.min.y + 74.0), t, 9.5) + 4.0;
                }
            }
            p.text(pos2(x, r.min.y + 100.0), Align2::LEFT_CENTER, "Change ›", crate::fonts::sans_font(11.5), theme::blend(MUTED(), ACCENT, 0.4 + 0.6 * ht));
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Pick the Pokémon").clicked() {
                self.setup.log.picker = Some(Picker::Species);
                self.setup.log.query.clear();
                self.setup.log.focus = true;
            }
            ui.add_space(12.0);
            egui::Grid::new("log-who").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui| {
                ui.label(caps("NICKNAME"));
                ui.add(egui::TextEdit::singleline(&mut d.mon.nickname).desired_width(160.0).char_limit(12).hint_text("Optional"));
                ui.end_row();
                ui.label(caps("LEVEL"));
                ui.horizontal(|ui| {
                    if chip(ui, "Unknown", !d.level_known, ACCENT).clicked() {
                        d.level_known = false;
                    }
                    let resp = ui.scope(|ui| {
                        if !d.level_known {
                            ui.multiply_opacity(0.45);
                        }
                        ui.add(egui::DragValue::new(&mut d.mon.level).range(1..=100).speed(0.2).prefix("Lv "))
                    }).inner;
                    if resp.changed() || resp.gained_focus() || resp.clicked() {
                        d.level_known = true;
                    }
                });
                ui.end_row();
                ui.label(caps("GENDER"));
                ui.horizontal(|ui| {
                    for (g, l) in [(None, "Unknown"), (Some('M'), "♂"), (Some('F'), "♀")] {
                        if chip(ui, l, d.mon.gender == g, ACCENT).clicked() {
                            d.mon.gender = g;
                        }
                    }
                });
                ui.end_row();
                ui.label(caps("ABILITY"));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    if chip(ui, "Unknown", d.mon.ability.is_empty(), ACCENT).clicked() {
                        d.mon.ability.clear();
                    }
                    if let Some(s) = sp {
                        for a in s.abilities_in(world) {
                            if chip(ui, &a, d.mon.ability == a, ACCENT).clicked() {
                                d.mon.ability = a.clone();
                            }
                        }
                    }
                });
                ui.end_row();
                ui.label(caps("ITEM"));
                let item = if d.mon.item.is_empty() { "None  ›".to_string() } else { format!("{}  ›", d.mon.item) };
                if ui.add_enabled(sp.is_some(), egui::Button::new(item).fill(kit::btn_fill()).min_size(vec2(160.0, 28.0))).on_hover_text("Pick the item it holds").clicked() {
                    self.setup.log.picker = Some(Picker::Item);
                    self.setup.log.query.clear();
                    self.setup.log.focus = true;
                }
                ui.end_row();
            });
        });
    }

    /// When, how and where it was caught.
    fn log_catch(&mut self, ui: &mut egui::Ui, d: &mut LoggedShiny, w: f32) {
        let _ = w;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("The catch").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
            ui.add_space(8.0);
            egui::Grid::new("log-catch").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui| {
                ui.label(caps("CAUGHT ON"));
                ui.horizontal(|ui| {
                    let (y, m, dd) = civil_from_days(local_day(d.when_unix));
                    let (mut y, mut m, mut dd) = (y as i32, m, dd);
                    let mut changed = false;
                    changed |= ui.add(egui::DragValue::new(&mut dd).range(1..=31).speed(0.1)).changed();
                    changed |= ui.add(egui::DragValue::new(&mut m).range(1..=12).speed(0.05).custom_formatter(|v, _| ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][(v as usize).clamp(1, 12) - 1].to_string())).changed();
                    changed |= ui.add(egui::DragValue::new(&mut y).range(2000..=2100).speed(0.05)).changed();
                    if changed {
                        d.when_unix = day_unix(days_from_civil(y as i64, m, dd));
                    }
                    if chip(ui, "Today", local_day(d.when_unix) == local_day(now_unix()), ACCENT).clicked() {
                        d.when_unix = now_unix();
                    }
                });
                ui.end_row();
                ui.label(caps("ENCOUNTERS"));
                ui.horizontal(|ui| {
                    if chip(ui, "N/A", d.encounters.is_none(), ACCENT).on_hover_text("Not known, or it wasn't a hunt").clicked() {
                        d.encounters = None;
                    }
                    let known = d.encounters.is_some();
                    let mut n = d.encounters.unwrap_or(0);
                    let resp = ui.scope(|ui| {
                        if !known {
                            ui.multiply_opacity(0.45);
                        }
                        ui.add(egui::DragValue::new(&mut n).range(0..=10_000_000).speed(5.0).custom_formatter(move |v, _| if !known && v == 0.0 { "type a number".into() } else { theme::thousands(v as u64) }))
                    }).inner;
                    if resp.changed() || (known && n > 0) {
                        d.encounters = if n > 0 { Some(n) } else { None };
                    }
                });
                ui.end_row();
                ui.label(caps("METHOD"));
                let cur = if d.method.is_empty() { "Not sure".to_string() } else { d.method.clone() };
                kit::Dropdown::from_id_salt("log-method").selected_text(cur).width(226.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.method, String::new(), "Not sure");
                    for m in HuntMethod::ALL {
                        if matches!(m, HuntMethod::FromGameLog | HuntMethod::Custom) {
                            continue;
                        }
                        ui.selectable_value(&mut d.method, m.label().to_string(), m.label());
                    }
                    ui.selectable_value(&mut d.method, "Gift / egg / other".to_string(), "Gift / egg / other");
                });
                ui.end_row();
                ui.label(caps("WHERE"));
                ui.add(egui::TextEdit::singleline(&mut d.place).desired_width(226.0).hint_text("Route 210, Mt. Coronet…"));
                ui.end_row();
            });
        });
    }

    /// Nature, and IVs/EVs when you know them.
    fn log_stats(&mut self, ui: &mut egui::Ui, d: &mut LoggedShiny, world: World, sp: &battle::Species) {
        if d.stats_known {
            d.nature_known = true;
            if d.mon.nature.is_empty() {
                d.mon.nature = "Hardy".into();
            }
            self.team_stats_card(ui, world, sp, &mut d.mon);
            ui.add_space(6.0);
            if ui.link(RichText::new("I don't know its IVs and EVs").size(12.0)).clicked() {
                d.stats_known = false;
            }
            return;
        }
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Nature & stats").font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cur = if d.nature_known && !d.mon.nature.is_empty() { nature_label(&d.mon.nature) } else { "Unknown".into() };
                    let mut pick: Option<Option<String>> = None;
                    kit::Dropdown::from_id_salt("log-nature").selected_text(cur).width(190.0).height(360.0).show_ui(ui, |ui| {
                        if ui.selectable_label(!d.nature_known, "Unknown").clicked() {
                            pick = Some(None);
                        }
                        for n in &battle::data().natures {
                            if ui.selectable_label(d.nature_known && d.mon.nature == n.name, nature_label(&n.name)).clicked() {
                                pick = Some(Some(n.name.clone()));
                            }
                        }
                    });
                    match pick {
                        Some(Some(n)) => {
                            d.mon.nature = n;
                            d.nature_known = true;
                        }
                        Some(None) => {
                            d.mon.nature.clear();
                            d.nature_known = false;
                        }
                        None => {}
                    }
                    ui.label(caps("NATURE"));
                });
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(small("IVs and EVs not filled in."));
                if ui.button("I know its IVs and EVs").clicked() {
                    d.stats_known = true;
                }
            });
        });
    }

    /// Picking a species, an item or a move, in place of the form.
    fn log_picker(&mut self, ui: &mut egui::Ui, d: &mut LoggedShiny, pk: Picker) {
        let world = world_of(self);
        ui.horizontal(|ui| {
            if !d.slug.is_empty() && ui.button("‹  Back").clicked() {
                self.setup.log.picker = None;
            }
            let what = match pk {
                Picker::Species => "Which Pokémon did you catch?".to_string(),
                Picker::Item => "What is it holding?".to_string(),
                Picker::Move(i) => format!("Move {}", i + 1),
            };
            ui.label(RichText::new(what).font(crate::fonts::sans_bold_font(16.0)).color(TEXT()));
        });
        ui.add_space(8.0);
        let mut q = std::mem::take(&mut self.setup.log.query);
        let hint = match pk {
            Picker::Species => "Name or number",
            Picker::Item => "Item name",
            Picker::Move(_) => "Move name or type",
        };
        let resp = kit::search_box_keyed(ui, Some(egui::Id::new(("log-q", format!("{pk:?}")))), &mut q, hint, 320.0);
        if std::mem::take(&mut self.setup.log.focus) {
            resp.request_focus();
        }
        self.setup.log.query = q.clone();
        ui.add_space(8.0);
        let q = q.trim().to_lowercase();
        let full = ui.available_width();
        match pk {
            Picker::Species => {
                let list: Vec<usize> = (0..self.species.len())
                    .filter(|&i| {
                        let sp = &self.species[i];
                        q.is_empty() || sp.name.to_lowercase().contains(&q) || sp.dex.to_string() == q.trim_start_matches('#').trim_start_matches('0')
                    })
                    .collect();
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let mut chosen: Option<String> = if enter && !q.is_empty() { list.first().map(|&i| self.species[i].slug.clone()) } else { None };
                if list.is_empty() {
                    ui.add_space(20.0);
                    ui.label(small("No Pokémon match. Try another name or number."));
                }
                // a grid of tiles, like the Shiny Dex
                let gap = 8.0;
                let tw = 128.0;
                let th = 118.0;
                let cols = ((full - 12.0 + gap) / (tw + gap)).floor().max(1.0) as usize;
                let tw = ((full - 12.0 + gap) / cols as f32 - gap).floor();
                let rows = list.len().div_ceil(cols);
                let cur = d.slug.clone();
                egui::ScrollArea::vertical().id_salt("log-pick-sp").auto_shrink([false, false]).show_rows(ui, th + gap, rows, |ui, range| {
                    for row in range {
                        let (rr, _) = ui.allocate_exact_size(vec2(full - 12.0, th + gap), Sense::hover());
                        for c in 0..cols {
                            let Some(&i) = list.get(row * cols + c) else { break };
                            let sp = self.species[i].clone();
                            let r = Rect::from_min_size(pos2(rr.min.x + c as f32 * (tw + gap), rr.min.y), vec2(tw, th));
                            let resp = ui.interact(r, ui.id().with(("log-sp", i)), Sense::click());
                            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                            let p = ui.painter().clone();
                            let on = sp.slug == cur;
                            p.rect_filled(r, 12.0, theme::blend(kit::sunken(), CARD_HOVER(), 0.5 + 0.5 * ht));
                            p.rect_stroke(r, 12.0, Stroke::new(if on { 2.0f32 } else { 1.0 }, if on { GOLD } else { theme::blend(LINE(), GOLD, 0.5 * ht) }));
                            p.text(r.min + vec2(9.0, 9.0), Align2::LEFT_TOP, format!("#{:04}", sp.dex), crate::fonts::sans_font(10.5), MUTED());
                            let sr = Rect::from_min_max(r.min + vec2(8.0, 20.0), pos2(r.max.x - 8.0, r.max.y - 26.0));
                            if !self.draw_sprite_centered(&p, &sp, true, sr.center(), sr.size()) {
                                p.circle_filled(sr.center(), 22.0, theme::blend(kit::sunken(), GOLD, 0.12));
                                p.text(sr.center(), Align2::CENTER_CENTER, sp.name.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(14.0), MUTED());
                            }
                            let f = crate::fonts::sans_bold_font(12.5);
                            p.text(pos2(r.center().x, r.max.y - 14.0), Align2::CENTER_CENTER, super::charts::clip(ui, sp.name, &f, tw - 12.0), f, TEXT());
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                chosen = Some(sp.slug.clone());
                            }
                        }
                    }
                });
                if let Some(slug) = chosen {
                    self.log_set_species(d, &slug);
                    // moves it can't learn go
                    if let Some(w) = self.wiki.clone() {
                        let learn = learnable(&w, &d.mon.species, world);
                        d.mon.moves.retain(|m| super::team_build::mkey(&w, m).is_some_and(|k| learn.iter().any(|l| l.key == k)));
                    }
                    self.setup.log.picker = None;
                    self.setup.log.query.clear();
                }
            }
            Picker::Item => {
                let w = self.wiki.clone();
                let mut rows: Vec<String> = holdable(w.as_deref(), world).iter().map(|i| i.n.clone()).filter(|n| q.is_empty() || n.to_lowercase().contains(&q)).collect();
                if q.is_empty() {
                    rows.insert(0, "None".into());
                }
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let mut chosen: Option<String> = if enter && !q.is_empty() { rows.first().cloned() } else { None };
                egui::ScrollArea::vertical().id_salt("log-pick-item").auto_shrink([false, false]).show_rows(ui, 40.0, rows.len(), |ui, range| {
                    for n in &rows[range] {
                        let (r, resp) = ui.allocate_exact_size(vec2(full - 12.0, 40.0), Sense::click());
                        let r = r.shrink2(vec2(0.0, 2.0));
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter().clone();
                        p.rect_filled(r, 8.0, theme::blend(kit::sunken(), CARD_HOVER(), 0.4 + 0.6 * ht));
                        p.text(pos2(r.min.x + 14.0, r.center().y), Align2::LEFT_CENTER, n, crate::fonts::sans_bold_font(13.0), TEXT());
                        let desc = if n == "None" { "Not holding anything".to_string() } else { super::team_build::item_text(w.as_deref(), n).map(|i| i.description.clone()).unwrap_or_default() };
                        if !desc.is_empty() {
                            let f = crate::fonts::sans_font(11.5);
                            p.text(pos2(r.min.x + 220.0, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, &desc, &f, r.width() - 232.0), f, MUTED());
                        }
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            chosen = Some(n.clone());
                        }
                    }
                });
                if let Some(n) = chosen {
                    d.mon.item = if n == "None" { String::new() } else { n };
                    self.setup.log.picker = None;
                }
            }
            Picker::Move(idx) => {
                let Some(w) = self.wiki.clone() else {
                    ui.label(small("Getting the game's data…"));
                    return;
                };
                let learn = learnable(&w, &d.mon.species, world);
                let ways = best_ways(&learn);
                let taken: Vec<String> = d.mon.moves.iter().enumerate().filter(|(i, _)| *i != idx).map(|(_, n)| n.clone()).collect();
                let mut rows: Vec<(String, String, String, String, String, String, String)> = ways
                    .iter()
                    .filter_map(|l| w.move_info(&l.key).map(|i| (i.n.clone(), i.t.clone(), i.c.clone(), i.p.clone(), i.a.clone(), l.how.label(), i.d.clone())))
                    .filter(|r| q.is_empty() || r.0.to_lowercase().contains(&q) || r.1.to_lowercase() == q || r.2.to_lowercase() == q)
                    .filter(|r| !taken.contains(&r.0))
                    .collect();
                rows.sort();
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let mut chosen: Option<String> = if enter && !q.is_empty() { rows.first().map(|r| r.0.clone()) } else { None };
                // column header
                let (hr, _) = ui.allocate_exact_size(vec2(full - 12.0, 18.0), Sense::hover());
                {
                    let p = ui.painter();
                    let f = crate::fonts::sans_bold_font(10.5);
                    for (x, t) in [(16.0, format!("{} MOVES", rows.len())), (210.0, "TYPE".into()), (290.0, "CAT".into()), (350.0, "POW".into()), (400.0, "ACC".into()), (456.0, "HOW".into())] {
                        p.text(pos2(hr.min.x + x, hr.center().y), Align2::LEFT_CENTER, t, f.clone(), MUTED());
                    }
                }
                let desc_x = 560.0;
                let show_desc = full > desc_x + 200.0;
                egui::ScrollArea::vertical().id_salt("log-pick-move").auto_shrink([false, false]).show_rows(ui, 40.0, rows.len(), |ui, range| {
                    for (n, t, c, pw, ac, how, desc) in &rows[range] {
                        let (r, resp) = ui.allocate_exact_size(vec2(full - 12.0, 40.0), Sense::click());
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        let p = ui.painter().clone();
                        let r = r.shrink2(vec2(0.0, 2.0));
                        p.rect_filled(r, 8.0, theme::blend(kit::sunken(), CARD_HOVER(), 0.4 + 0.6 * ht));
                        p.rect_filled(Rect::from_min_size(r.min, vec2(5.0, r.height())), egui::Rounding { nw: 8.0, sw: 8.0, ne: 0.0, se: 0.0 }, type_color(t));
                        p.text(pos2(r.min.x + 16.0, r.center().y), Align2::LEFT_CENTER, n, crate::fonts::sans_bold_font(13.0), TEXT());
                        type_tag(&p, pos2(r.min.x + 210.0, r.center().y), t, 9.0);
                        cat_tag(&p, pos2(r.min.x + 290.0, r.center().y), c);
                        let dash = |v: &str| if v.is_empty() || v == "0" { "—".to_string() } else { v.to_string() };
                        p.text(pos2(r.min.x + 350.0, r.center().y), Align2::LEFT_CENTER, dash(pw), crate::fonts::sans_font(12.0), TEXT());
                        let acc = if ac.is_empty() || ac == "0" { "—".to_string() } else { format!("{ac}%") };
                        p.text(pos2(r.min.x + 400.0, r.center().y), Align2::LEFT_CENTER, acc, crate::fonts::sans_font(12.0), TEXT());
                        p.text(pos2(r.min.x + 456.0, r.center().y), Align2::LEFT_CENTER, how, crate::fonts::sans_bold_font(11.0), super::charts::ink(ACCENT));
                        if show_desc && !desc.is_empty() {
                            let f = crate::fonts::sans_font(11.5);
                            p.text(pos2(r.min.x + desc_x, r.center().y), Align2::LEFT_CENTER, super::charts::clip(ui, desc, &f, r.width() - desc_x - 12.0), f, MUTED());
                        }
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            chosen = Some(n.clone());
                        }
                    }
                });
                if let Some(n) = chosen {
                    while d.mon.moves.len() <= idx {
                        d.mon.moves.push(String::new());
                    }
                    d.mon.moves[idx] = n;
                    d.mon.moves.retain(|m| !m.is_empty());
                    self.setup.log.picker = None;
                }
            }
        }
    }

    /// The logged shinies of one species, as cards (for its Shiny Dex page).
    pub(super) fn logged_cards(&mut self, ui: &mut egui::Ui, slug: &str) {
        let list: Vec<LoggedShiny> = self.cfg.logged.iter().filter(|l| l.slug == slug).cloned().collect();
        if list.is_empty() {
            return;
        }
        kit::section(ui, "Logged by hand");
        let Some(sp) = self.species.iter().find(|s| s.slug == slug).cloned() else { return };
        let wiki = self.wiki.clone();
        let mut edit: Option<u64> = None;
        let mut to_journal: Option<u64> = None;
        let have_journal = self.has_app(crate::apps::AppId::Journal);
        for l in &list {
            let width = ui.available_width().min(920.0);
            ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                kit::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_top(|ui| {
                        // the shiny, as on a trophy
                        let (r, _) = ui.allocate_exact_size(vec2(150.0, 150.0), Sense::hover());
                        let p = ui.painter_at(r.expand(4.0));
                        p.rect_filled(r, kit::r(12.0), CARD_HOVER());
                        theme::platform(&p, r.center_bottom() - vec2(0.0, 18.0), r.width() * 0.32, 9.0);
                        self.draw_sprite(&p, &sp, true, r.center_bottom() - vec2(0.0, 16.0), vec2(130.0, 120.0));
                        theme::sparkle(&p, r.right_top() + vec2(-14.0, 14.0), 6.0, GOLD);
                        ui.add_space(14.0);
                        ui.vertical(|ui| {
                            // name, LOGGED tag, Edit
                            ui.horizontal(|ui| {
                                let name = if l.mon.nickname.trim().is_empty() { l.mon.species.clone() } else { l.mon.nickname.trim().to_string() };
                                ui.label(RichText::new(name).font(crate::fonts::sans_bold_font(17.0)).color(TEXT()));
                                if !l.mon.nickname.trim().is_empty() {
                                    ui.label(small(l.mon.species.clone()));
                                }
                                match l.mon.gender {
                                    Some('M') => ui.label(RichText::new("♂").size(14.0).color(kit::C_BLUE)),
                                    Some('F') => ui.label(RichText::new("♀").size(14.0).color(kit::C_PINK)),
                                    _ => ui.label(""),
                                };
                                pill(ui, "LOGGED", GOLD);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add(egui::Button::new("✏  Edit").fill(kit::btn_fill())).clicked() {
                                        edit = Some(l.id);
                                    }
                                    if super::journal_add::add_button(ui, have_journal) {
                                        to_journal = Some(l.id);
                                    }
                                });
                            });
                            // the catch
                            let mut facts: Vec<String> = vec![format!("Caught {}", fmt_date(l.when_unix))];
                            facts.push(l.encounters.map_or("Encounters N/A".into(), |n| format!("{} encounters", theme::thousands(n))));
                            if !l.method.is_empty() {
                                facts.push(l.method.clone());
                            }
                            if !l.place.trim().is_empty() {
                                facts.push(l.place.trim().to_string());
                            }
                            ui.label(RichText::new(facts.join("  ·  ")).size(kit::fs(12.5)).color(MUTED()));
                            ui.add_space(8.0);
                            // what it is
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                                let lv = if l.level_known { format!("Lv {}", l.mon.level) } else { "Lv ?".into() };
                                pill(ui, &lv, kit::C_SLATE);
                                let nat = if l.nature_known && !l.mon.nature.is_empty() { nature_label(&l.mon.nature) } else { "Nature ?".into() };
                                pill(ui, &nat, kit::C_INDIGO);
                                pill(ui, if l.mon.ability.is_empty() { "Ability ?" } else { l.mon.ability.as_str() }, kit::C_TEAL);
                                if !l.mon.item.is_empty() {
                                    pill(ui, &format!("@ {}", l.mon.item), kit::C_ORANGE);
                                }
                            });
                            // moves, by type
                            if !l.mon.moves.is_empty() {
                                ui.add_space(6.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                                    for m in &l.mon.moves {
                                        let t = wiki.as_deref().and_then(|w| super::team_build::mkey(w, m).and_then(|k| w.move_info(&k).map(|i| i.t.clone()))).unwrap_or_default();
                                        pill(ui, m, if t.is_empty() { kit::C_SLATE } else { type_color(&t) });
                                    }
                                });
                            }
                            // IVs (and EVs), as small bars
                            if l.stats_known {
                                ui.add_space(10.0);
                                stat_bars(ui, "IVs", l.mon.ivs, 31);
                                if l.mon.evs.iter().any(|v| *v > 0) {
                                    ui.add_space(4.0);
                                    stat_bars(ui, "EVs", l.mon.evs, 252);
                                }
                            }
                            if !l.mon.notes.trim().is_empty() {
                                ui.add_space(8.0);
                                ui.label(RichText::new(l.mon.notes.trim()).size(kit::fs(12.5)).italics().color(MUTED()));
                            }
                        });
                    });
                });
            });
            ui.add_space(10.0);
        }
        if let Some(id) = edit {
            self.edit_log(id);
        }
        if let Some(id) = to_journal {
            if let Some(c) = self.logged_clip(id) {
                self.journal_add(c);
            }
        }
    }
}

/// A small coloured tag.
fn pill(ui: &mut egui::Ui, text: &str, col: Color32) {
    egui::Frame::none()
        .fill(theme::blend(CARD(), col, 0.18))
        .stroke(Stroke::new(1.0f32, theme::blend(CARD(), col, 0.45)))
        .rounding(10.0)
        .inner_margin(egui::Margin::symmetric(9.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(kit::fs(12.0)).strong().color(super::charts::ink(col)));
        });
}

/// Six stats as labelled mini bars (gold at the top value).
fn stat_bars(ui: &mut egui::Ui, title: &str, v: [u16; 6], top: u16) {
    const ST: [&str; 6] = ["HP", "Atk", "Def", "SpA", "SpD", "Spe"];
    ui.horizontal(|ui| {
        ui.add_sized(vec2(34.0, 30.0), egui::Label::new(caps(title)));
        for (i, s) in ST.iter().enumerate() {
            let (r, _) = ui.allocate_exact_size(vec2(62.0, 30.0), Sense::hover());
            let p = ui.painter();
            p.text(pos2(r.min.x, r.min.y + 7.0), Align2::LEFT_CENTER, *s, crate::fonts::sans_font(10.5), MUTED());
            let maxed = v[i] >= top;
            p.text(pos2(r.max.x - 6.0, r.min.y + 7.0), Align2::RIGHT_CENTER, v[i].to_string(), crate::fonts::sans_bold_font(12.0), if maxed { GOLD } else { TEXT() });
            let bar = Rect::from_min_size(pos2(r.min.x, r.max.y - 9.0), vec2(r.width() - 6.0, 5.0));
            p.rect_filled(bar, 2.5, LINE());
            let f = (v[i] as f32 / top as f32).clamp(0.0, 1.0);
            if f > 0.0 {
                p.rect_filled(Rect::from_min_size(bar.min, vec2((bar.width() * f).max(4.0), bar.height())), 2.5, if maxed { GOLD } else { ACCENT });
            }
        }
    });
}
