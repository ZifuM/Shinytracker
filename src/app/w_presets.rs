//! Starter layouts: a few hand-laid Dashboards (five or six widgets) for a
//! kind of player, offered on the empty Dashboard and in Add widgets. One
//! click puts the layout in place (adding any apps it needs); on a
//! Dashboard that already has widgets it asks first, replace or add below,
//! and the old layout can be put back.

use super::store::{app_col, card_bg, pill_button};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::widgets::{def, place_all, settle, Cell, COLS, GRID_VERSION};
use super::App;
use crate::apps::{self, AppId, WidgetCfg};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};

pub(super) struct Preset {
    pub slug: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub about: &'static str,
    pub col: Color32,
    /// (widget, column, row, width, height) on the 12-column grid.
    pub widgets: &'static [(&'static str, i32, i32, i32, i32)],
}

pub(super) const PRESETS: &[Preset] = &[
    Preset {
        slug: "starter",
        name: "Starter",
        icon: "⭐",
        about: "A bit of everything: today, your hunt, rare grass and shortcuts",
        col: kit::C_BLUE,
        widgets: &[("today", 0, 0, 8, 1), ("rare", 8, 0, 4, 5), ("hunt_ctrl", 0, 1, 4, 2), ("shinies", 4, 1, 4, 2), ("jump", 0, 3, 8, 2)],
    },
    Preset {
        slug: "hunter",
        name: "Shiny Hunter",
        icon: "✨",
        about: "Your hunt front and centre, your week, latest shinies and today's rare grass",
        col: kit::C_VIOLET,
        widgets: &[("today", 0, 0, 8, 1), ("hunt_ctrl", 8, 0, 4, 4), ("week", 0, 1, 4, 3), ("shinies", 4, 1, 4, 3), ("rare", 0, 4, 8, 3), ("hunts", 8, 4, 4, 3)],
    },
    Preset {
        slug: "explorer",
        name: "Explorer",
        icon: "🗺",
        about: "The map, today's rare grass and what lives where",
        col: kit::C_GREEN,
        widgets: &[("minimap", 0, 0, 4, 4), ("rare", 4, 0, 4, 2), ("huntwhere", 8, 0, 4, 2), ("catchable", 4, 2, 4, 2), ("mapfavs", 8, 2, 4, 2)],
    },
    Preset {
        slug: "info",
        name: "Researcher",
        icon: "📖",
        about: "Look anything up: Pokédex, types, moves, items and guides",
        col: kit::C_ORANGE,
        widgets: &[("dexsearch", 0, 0, 6, 3), ("typematch", 6, 0, 6, 3), ("infosearch", 0, 3, 4, 2), ("bookmarks", 4, 3, 4, 2), ("tip", 8, 3, 4, 2)],
    },
    Preset {
        slug: "games",
        name: "Minigames",
        icon: "🎮",
        about: "Shiny Bingo, your Pokégotchi and booster packs",
        col: kit::C_PINK,
        widgets: &[("bingo_card", 0, 0, 4, 5), ("gotchi_pet", 4, 0, 4, 3), ("gotchi_care", 4, 3, 4, 2), ("cards_packs", 8, 0, 4, 2), ("cards_missions", 8, 2, 4, 3)],
    },
    Preset {
        slug: "battler",
        name: "Battler",
        icon: "⚔",
        about: "Your team, its weak spots, damage, speed and a quick battle",
        col: kit::C_RED,
        widgets: &[("mainteam", 0, 0, 8, 2), ("teamweak", 8, 0, 4, 2), ("quickcalc", 0, 2, 4, 3), ("speedtiers", 4, 2, 4, 3), ("quickbattle", 8, 2, 4, 3)],
    },
    Preset {
        slug: "breeder",
        name: "Breeder",
        icon: "🐣",
        about: "The Day Care, your breeders, egg moves and the next step",
        col: kit::C_GOLD,
        widgets: &[("breeding", 0, 0, 8, 1), ("nextstep", 8, 0, 4, 3), ("breeders", 0, 1, 4, 3), ("eggmoves", 4, 1, 4, 2), ("hatches", 4, 3, 8, 1)],
    },
];

impl Preset {
    fn rows(&self) -> i32 {
        self.widgets.iter().map(|w| w.2 + w.4).max().unwrap_or(1)
    }

    /// The apps this layout's widgets come from.
    fn apps(&self) -> Vec<AppId> {
        let mut out = Vec::new();
        for (k, ..) in self.widgets {
            if let Some(a) = def(k).and_then(|d| d.app) {
                if !out.contains(&a) {
                    out.push(a);
                }
            }
        }
        out
    }
}

/// The layout's widgets, ids from `first_id`, moved down `dy` rows.
pub(super) fn preset_widgets(p: &Preset, first_id: u64, dy: i32) -> Vec<WidgetCfg> {
    p.widgets
        .iter()
        .enumerate()
        .filter_map(|(i, (k, x, y, w, h))| {
            let d = def(k)?;
            Some(WidgetCfg { id: first_id + i as u64, kind: d.kind.to_string(), span: d.span, data: Vec::new(), pos: Some(Cell { x: *x, y: *y + dy, w: *w, h: *h }.save()) })
        })
        .collect()
}

/// A small picture of the layout: each widget a block in its app's colour
/// with its icon, where it sits on the grid.
pub(super) fn preset_mini(p: &egui::Painter, r: Rect, pr: &Preset, glow: f32) {
    let rows = pr.rows() as f32;
    let gap = 3.0;
    let cell = ((r.width() + gap) / COLS as f32).min((r.height() + gap) / rows);
    let size = vec2(cell * COLS as f32 - gap, cell * rows - gap);
    let o = r.center() - size / 2.0;
    for (k, x, y, w, h) in pr.widgets {
        let Some(d) = def(k) else { continue };
        let col = d.app.map_or(kit::C_CYAN, app_col);
        let b = Rect::from_min_size(o + vec2(*x as f32 * cell, *y as f32 * cell), vec2(*w as f32 * cell - gap, *h as f32 * cell - gap));
        p.rect_filled(b, 4.0, theme::blend(CARD(), col, 0.22 + 0.12 * glow));
        p.rect_stroke(b, 4.0, Stroke::new(1.0f32, theme::blend(LINE(), col, 0.55 + 0.3 * glow)));
        let fs = (b.height() * 0.8).min(b.width() * 0.5).min(14.0);
        if fs >= 7.0 {
            p.text(b.center(), Align2::CENTER_CENTER, d.icon, crate::fonts::sans_font(fs), theme::blend(CARD(), TEXT(), 0.85));
        }
    }
}

/// How a layout goes onto a Dashboard that already has widgets.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Put {
    Replace,
    Below,
}

impl App {
    /// Put a layout on the Dashboard (and get the apps it needs).
    pub(super) fn apply_preset(&mut self, i: usize, put: Put) {
        let Some(pr) = PRESETS.get(i) else { return };
        let before = self.home_widgets();
        let new = match put {
            Put::Replace => preset_widgets(pr, 1, 0),
            Put::Below => {
                let mut w = before.clone();
                let mut cells = place_all(&w);
                settle(&mut cells, None);
                for x in w.iter_mut() {
                    if let Some((_, c)) = cells.iter().find(|(id, _)| *id == x.id) {
                        x.pos = Some(c.save());
                    }
                }
                let bottom = cells.iter().map(|(_, c)| c.y + c.h).max().unwrap_or(0);
                let first = w.iter().map(|x| x.id).max().unwrap_or(0) + 1;
                w.extend(preset_widgets(pr, first, bottom));
                w
            }
        };
        // the apps it needs (and theirs), installed as from the App Store
        let have = self.cfg.apps.installed();
        let mut get: Vec<AppId> = Vec::new();
        for a in pr.apps() {
            for x in apps::with_requirements(a, &have) {
                if !get.contains(&x) {
                    get.push(x);
                }
            }
        }
        if !get.is_empty() {
            self.start_install(&get);
        }
        self.setup.home.undo = Some((pr.name, before));
        self.setup.home.preset_pick = None;
        self.setup.home.gallery.open = false;
        self.cfg.home.grid = GRID_VERSION;
        self.set_home_widgets(new);
        self.app_toast(format!("{} layout {}", pr.name, if put == Put::Replace { "is on your Dashboard" } else { "added below" }));
    }

    /// Back to the Dashboard from before the last layout.
    pub(super) fn undo_preset(&mut self) {
        if let Some((name, w)) = self.setup.home.undo.take() {
            self.set_home_widgets(w);
            self.app_toast(format!("Took the {name} layout back off"));
        }
    }

    /// The layouts as a row of cards (wrapping), `id` keeping each place's
    /// buttons apart.
    /// `big`: roomier cards with each layout's widgets listed (the tab in
    /// Add widgets).
    pub(super) fn preset_cards(&mut self, ui: &mut egui::Ui, id: &str, big: bool) {
        let has_widgets = !self.home_widgets().is_empty();
        let installed = self.cfg.apps.installed();
        let w = ui.available_width();
        let gap = 12.0;
        // as many to a row as fit, the rows kept even (4 + 3, not 6 + 1)
        let n = PRESETS.len();
        let min_w = if big { 244.0 } else { 148.0 };
        let fit_n = (((w + gap) / (min_w + gap)).floor() as usize).clamp(1, n);
        let rows_n = n.div_ceil(fit_n);
        let cols = n.div_ceil(rows_n);
        let cw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor().min(if big { 320.0 } else { 230.0 });
        let (ch, pic_h) = if big { (348.0, 112.0) } else { (196.0, 72.0) };
        let row_w = cw * cols as f32 + gap * (cols as f32 - 1.0);
        let mut pick: Option<(usize, Option<Put>)> = None;
        for row in (0..PRESETS.len()).collect::<Vec<_>>().chunks(cols) {
            let (rr, _) = ui.allocate_exact_size(vec2(w, ch), Sense::hover());
            let x0 = rr.min.x + (w - row_w) / 2.0;
            for (c, &i) in row.iter().enumerate() {
                let pr = &PRESETS[i];
                let r = Rect::from_min_size(pos2(x0 + c as f32 * (cw + gap), rr.min.y), vec2(cw, ch));
                let resp = ui.interact(r, ui.id().with((id, "preset", i)), Sense::click());
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let armed = self.setup.home.preset_pick == Some(i);
                let p = ui.painter().clone();
                card_bg(&p, r, resp.hovered());
                if ht > 0.0 || armed {
                    p.rect_stroke(r, 16.0, Stroke::new(1.5f32, pr.col.gamma_multiply(if armed { 1.0 } else { ht })));
                }
                // the picture
                let mr = Rect::from_min_size(r.min + vec2(12.0, 12.0), vec2(cw - 24.0, pic_h));
                p.rect_filled(mr, 10.0, theme::blend(kit::PAGE(), pr.col, 0.06 + 0.04 * ht));
                preset_mini(&p, mr.shrink(8.0), pr, ht);
                // name and what it's for
                let ty = mr.max.y + 12.0;
                p.text(pos2(r.min.x + 14.0, ty), Align2::LEFT_TOP, format!("{}  {}", pr.icon, pr.name), crate::fonts::sans_bold_font(13.5), TEXT());
                let mut job = egui::text::LayoutJob::simple(pr.about.to_string(), crate::fonts::sans_font(11.5), MUTED(), cw - 28.0);
                job.wrap.max_rows = 2;
                job.wrap.overflow_character = Some('…');
                p.galley(pos2(r.min.x + 14.0, ty + 22.0), p.layout_job(job), MUTED());
                if big {
                    // what's in it
                    let mut y = ty + 64.0;
                    for (k, ..) in pr.widgets {
                        if let Some(d) = def(k) {
                            let col = d.app.map_or(kit::C_CYAN, app_col);
                            p.circle_filled(pos2(r.min.x + 19.0, y + 7.0), 3.0, col);
                            let g = p.layout_no_wrap(d.name.to_string(), crate::fonts::sans_font(12.0), TEXT());
                            p.with_clip_rect(Rect::from_min_max(pos2(r.min.x, y), pos2(r.max.x - 12.0, y + 16.0))).galley(pos2(r.min.x + 30.0, y), g, TEXT());
                            y += 18.0;
                        }
                    }
                }
                let fy = r.max.y - 36.0;
                if armed {
                    // replace, or add below what's there
                    let bw = (cw - 34.0) / 2.0;
                    let a = Rect::from_min_size(pos2(r.min.x + 12.0, fy), vec2(bw, 26.0));
                    let b = Rect::from_min_size(pos2(a.max.x + 10.0, fy), vec2(bw, 26.0));
                    if pill_button(ui, a, "Replace", true, (id, "rep", i)).on_hover_text("Swap your widgets for this layout (you can undo)").clicked() {
                        pick = Some((i, Some(Put::Replace)));
                    }
                    if pill_button(ui, b, "Add below", false, (id, "below", i)).on_hover_text("Keep your widgets and add these underneath").clicked() {
                        pick = Some((i, Some(Put::Below)));
                    }
                } else {
                    let missing: Vec<AppId> = pr.apps().into_iter().filter(|a| !installed.contains(a)).collect();
                    let (txt, col) = if missing.is_empty() {
                        ("✔  Ready to go".to_string(), kit::C_GREEN)
                    } else {
                        (format!("+ adds {}", missing.iter().map(|a| a.name()).collect::<Vec<_>>().join(", ")), pr.col)
                    };
                    let g = p.layout(txt, crate::fonts::sans_bold_font(11.0), col, cw - 28.0);
                    let gy = r.max.y - 12.0 - g.size().y.min(28.0);
                    p.with_clip_rect(Rect::from_min_max(pos2(r.min.x, gy), pos2(r.max.x, r.max.y - 8.0))).galley(pos2(r.min.x + 14.0, gy), g, col);
                    let tip = format!("{}\n\n{}", pr.about, pr.widgets.iter().filter_map(|w| def(w.0)).map(|d| format!("{}  {}", d.icon, d.name)).collect::<Vec<_>>().join("\n"));
                    if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        pick = Some((i, if has_widgets { None } else { Some(Put::Replace) }));
                    }
                }
            }
            ui.add_space(gap);
        }
        match pick {
            Some((i, Some(put))) => self.apply_preset(i, put),
            Some((i, None)) => self.setup.home.preset_pick = if self.setup.home.preset_pick == Some(i) { None } else { Some(i) },
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::widgets::{fit, lim};

    #[test]
    fn every_layout_fits_its_widgets_and_the_grid() {
        let mut slugs = Vec::new();
        for pr in PRESETS {
            assert!(!slugs.contains(&pr.slug), "{} twice", pr.slug);
            slugs.push(pr.slug);
            assert!((5..=6).contains(&pr.widgets.len()), "{} has five or six widgets", pr.name);
            let mut taken = std::collections::HashSet::new();
            for (k, x, y, w, h) in pr.widgets {
                let d = def(k).unwrap_or_else(|| panic!("{}: no widget {k}", pr.name));
                let c = Cell { x: *x, y: *y, w: *w, h: *h };
                assert_eq!(fit(c, lim(Some(d))), c, "{}: {k} is a size it can be", pr.name);
                assert!(x + w <= COLS, "{}: {k} inside the grid", pr.name);
                for i in *x..x + w {
                    for j in *y..y + h {
                        assert!(taken.insert((i, j)), "{}: {k} overlaps", pr.name);
                    }
                }
            }
            assert_eq!(taken.len() as i32, COLS * pr.rows(), "{} leaves no holes", pr.name);
        }
    }

    #[test]
    fn layouts_need_no_upgrade_and_go_below_cleanly() {
        let have = preset_widgets(&PRESETS[1], 1, 0);
        for pr in PRESETS {
            let w = preset_widgets(pr, 1, 0);
            assert!(!super::super::widgets::needs_upgrade(&w), "{}", pr.name);
            let mut both = have.clone();
            let bottom = have.iter().map(|x| Cell::of(x.pos.unwrap())).map(|c| c.y + c.h).max().unwrap();
            both.extend(preset_widgets(pr, 100, bottom));
            assert!(!super::super::widgets::needs_upgrade(&both));
            let cells: Vec<Cell> = both.iter().map(|x| Cell::of(x.pos.unwrap())).collect();
            for (i, a) in cells.iter().enumerate() {
                for b in &cells[..i] {
                    assert!(!a.overlaps(*b), "{} below overlaps", pr.name);
                }
            }
        }
    }
}
