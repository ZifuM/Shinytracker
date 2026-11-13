//! The Library: every installed app, to open, close or remove; as tiles or
//! as a list. Opening an app puts it in the sidebar. A bar along the top
//! shows what the companion keeps on this PC.

use super::store::{app_col, app_icon, card_bg, pill_button, AppModal};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::apps::{self, AppId};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use std::path::Path;
use std::time::{Duration, Instant};

fn dir_size(p: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(p) else { return std::fs::metadata(p).map(|m| m.len()).unwrap_or(0) };
    rd.flatten()
        .map(|e| {
            let path = e.path();
            if path.is_dir() {
                dir_size(&path)
            } else {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

/// What's kept on this PC, by what it's for (measured off the UI thread).
fn measure() -> Vec<(String, u64)> {
    let cache = crate::config::cache_dir();
    let data = crate::config::data_dir();
    let f = |p: &Path| dir_size(p);
    let mut v = vec![
        ("Sprites".to_string(), f(&cache.join("sprites"))),
        ("Game Info".to_string(), f(&cache.join("wiki.json")) + f(&cache.join("wiki-seen.json"))),
        ("Pokédex".to_string(), f(&cache.join("pokemondb"))),
        ("Store pictures".to_string(), f(&cache.join("store"))),
        ("Your saves".to_string(), f(&data.join("config.json")) + f(&data.join("teams.json")) + f(&data.join("battle-sim.json")) + f(&data.join("breeding.json"))),
        ("Backups".to_string(), f(&data.join("backups"))),
    ];
    v.retain(|x| x.1 > 0);
    v
}

fn mb(b: u64) -> String {
    let m = b as f64 / 1_048_576.0;
    if m < 0.1 {
        return format!("{} KB", (b / 1024).max(1));
    }
    if m >= 10.0 {
        format!("{m:.0} MB")
    } else {
        format!("{m:.1} MB")
    }
}

fn ago(t: Option<u64>) -> String {
    let Some(t) = t else { return "Not yet".into() };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let d = now.saturating_sub(t);
    match d {
        0..=119 => "Just now".into(),
        120..=3599 => format!("{} min ago", d / 60),
        3600..=86_399 => format!("{} h ago", d / 3600),
        86_400..=172_799 => "Yesterday".into(),
        _ => format!("{} days ago", d / 86_400),
    }
}

impl App {
    pub(super) fn page_library(&mut self, ui: &mut egui::Ui) {
        let installed = self.cfg.apps.installed();
        let open = self.cfg.apps.open_list();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                kit::page_header(ui, "Library", &format!("{} apps installed, {} open. Open one and it joins your sidebar.", installed.len(), open.len()));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                ui.add_space(4.0);
                if ui.add(egui::Button::new("App Store ›").min_size(vec2(0.0, 30.0))).clicked() {
                    self.go_to(super::Tab::Store);
                }
            });
        });
        self.library_storage(ui);
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let mut list = self.cfg.apps.list_view;
            if kit::segmented(ui, &mut list, &[(false, "Tiles"), (true, "List")]) {
                self.cfg.apps.list_view = list;
                self.dirty = true;
            }
        });
        ui.add_space(12.0);
        if self.cfg.apps.list_view {
            self.library_list(ui, &installed);
        } else {
            self.library_tiles(ui, &installed);
        }
        let missing = apps::ALL.iter().filter(|a| !self.has_app(**a)).count();
        if missing > 0 {
            ui.add_space(16.0);
            let w = ui.available_width();
            let (r, resp) = ui.allocate_exact_size(vec2(w, 64.0), Sense::click());
            let p = ui.painter();
            p.rect_stroke(r.shrink(1.0), 16.0, Stroke::new(1.0f32, if resp.hovered() { kit::CONTROL() } else { LINE() }));
            kit::icon_chip(p, Rect::from_center_size(pos2(r.min.x + 34.0, r.center().y), vec2(34.0, 34.0)), "🏪", super::shell::STORE_COL, false);
            p.text(pos2(r.min.x + 64.0, r.center().y - 9.0), Align2::LEFT_CENTER, format!("{missing} more app{} in the App Store", if missing == 1 { "" } else { "s" }), crate::fonts::sans_bold_font(14.0), TEXT());
            p.text(pos2(r.min.x + 64.0, r.center().y + 10.0), Align2::LEFT_CENTER, "Install only what you use. Your saved data is never touched.", crate::fonts::sans_font(12.0), MUTED());
            p.text(pos2(r.max.x - 20.0, r.center().y), Align2::RIGHT_CENTER, "Browse ›", crate::fonts::sans_bold_font(13.0), TEXT());
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.go_to(super::Tab::Store);
            }
        }
    }

    fn library_storage(&mut self, ui: &mut egui::Ui) {
        let st = &mut self.setup.store;
        let stale = st.sizes_got.as_ref().map_or(true, |(t, _)| t.elapsed() > Duration::from_secs(20));
        if stale && st.sizes.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            let c = ui.ctx().clone();
            std::thread::spawn(move || {
                let _ = tx.send(measure());
                c.request_repaint();
            });
            st.sizes = Some(rx);
        }
        if let Some(rx) = &st.sizes {
            if let Ok(v) = rx.try_recv() {
                st.sizes_got = Some((Instant::now(), v));
                st.sizes = None;
            }
        }
        let sizes = st.sizes_got.as_ref().map(|x| x.1.clone()).unwrap_or_default();
        let total: u64 = sizes.iter().map(|x| x.1).sum();
        let cols = [kit::C_RED, kit::C_ORANGE, kit::C_PINK, super::shell::STORE_COL, kit::C_VIOLET, kit::C_SLATE];
        egui::Frame::none().fill(CARD()).stroke(Stroke::new(1.0f32, LINE())).rounding(14.0).inner_margin(egui::Margin::symmetric(16.0, 12.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("On this PC · {}", if total > 0 { mb(total) } else { "measuring…".into() })).font(crate::fonts::sans_bold_font(13.0)).color(TEXT()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let sprites = sizes.iter().find(|x| x.0 == "Sprites").map_or(0, |x| x.1);
                    if sprites > 0 && ui.add(egui::Button::new(egui::RichText::new("Clear cached sprites").size(kit::fs(12.0)))).on_hover_text("Sprites download again when a page needs them").clicked() {
                        self.setup.store.modal = Some(AppModal::ClearSprites);
                    }
                    // the bar
                    let w = (ui.available_width() - 16.0).max(80.0);
                    let (r, _) = ui.allocate_exact_size(vec2(w, 10.0), Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(r, 5.0, kit::sunken());
                    if total > 0 {
                        let mut x = r.min.x;
                        for (i, (name, b)) in sizes.iter().enumerate() {
                            let seg = r.width() * (*b as f32 / total as f32);
                            let sr = Rect::from_min_size(pos2(x, r.min.y), vec2(seg.max(2.0), r.height()));
                            p.rect_filled(sr, 5.0, cols[i % cols.len()]);
                            ui.interact(sr, ui.id().with(("stor", i)), Sense::hover()).on_hover_text(format!("{name}: {}", mb(*b)));
                            x += seg;
                        }
                    }
                });
            });
            if !sizes.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for (i, (name, b)) in sizes.iter().enumerate() {
                        let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.0, cols[i % cols.len()]);
                        ui.label(egui::RichText::new(format!("{name} {}", mb(*b))).size(kit::fs(11.5)).color(MUTED()));
                        ui.add_space(8.0);
                    }
                });
            }
        });
    }

    fn library_tiles(&mut self, ui: &mut egui::Ui, installed: &[AppId]) {
        let w = ui.available_width();
        let cols = (((w + 12.0) / 262.0).floor() as usize).clamp(2, 5);
        let cw = (w - 12.0 * (cols as f32 - 1.0)) / cols as f32;
        let mut menu_at: Option<(AppId, Rect)> = None;
        for row in 0..installed.len().div_ceil(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for k in 0..cols {
                    let i = row * cols + k;
                    if i >= installed.len() {
                        break;
                    }
                    let a = installed[i];
                    let (rect, resp) = ui.allocate_exact_size(vec2(cw, 132.0), Sense::click());
                    let hov = resp.hovered();
                    let p = ui.painter().clone();
                    card_bg(&p, rect, hov);
                    let ic = Rect::from_min_size(rect.min + vec2(16.0, 16.0), vec2(52.0, 52.0));
                    kit::icon_chip(&p, ic, app_icon(a), app_col(a), false);
                    let is_open = self.cfg.apps.is_open(a);
                    p.text(pos2(ic.max.x + 12.0, ic.min.y + 6.0), Align2::LEFT_TOP, a.name(), crate::fonts::sans_bold_font(14.5), TEXT());
                    let (st, c) = if is_open { ("● Open", kit::C_CYAN) } else if a.meta().core { ("Core app", MUTED()) } else { ("Installed", MUTED()) };
                    p.text(pos2(ic.max.x + 12.0, ic.min.y + 28.0), Align2::LEFT_TOP, st, crate::fonts::sans_font(12.0), c);
                    // ⋯ menu
                    let mr = Rect::from_min_size(pos2(rect.max.x - 40.0, rect.min.y + 12.0), vec2(28.0, 28.0));
                    let m = pill_button(ui, mr, "•••", false, ("lib-menu", a));
                    if m.clicked() {
                        self.setup.store.menu = if self.setup.store.menu == Some(a) { None } else { Some(a) };
                    }
                    if self.setup.store.menu == Some(a) {
                        menu_at = Some((a, mr));
                    }
                    let by = rect.max.y - 44.0;
                    let mut x = rect.min.x + 16.0;
                    if is_open {
                        if pill_button(ui, Rect::from_min_size(pos2(x, by), vec2(76.0, 30.0)), "Go to", true, ("lib-go", a)).clicked() {
                            self.open_app(a);
                        }
                        x += 84.0;
                        if pill_button(ui, Rect::from_min_size(pos2(x, by), vec2(70.0, 30.0)), "Close", false, ("lib-close", a)).clicked() {
                            self.close_app(a);
                        }
                    } else if pill_button(ui, Rect::from_min_size(pos2(x, by), vec2(76.0, 30.0)), "Open", true, ("lib-open", a)).clicked() {
                        self.open_app(a);
                    }
                    if resp.double_clicked() {
                        self.open_app(a);
                    }
                }
            });
            ui.add_space(12.0);
        }
        if let Some((a, at)) = menu_at {
            self.library_menu(ui, a, at);
        }
    }

    /// A tile's ⋯ menu.
    fn library_menu(&mut self, ui: &mut egui::Ui, a: AppId, at: Rect) {
        let mut close = false;
        let area = egui::Area::new(egui::Id::new(("lib-menu-pop", a))).order(egui::Order::Foreground).fixed_pos(pos2(at.max.x - 200.0, at.max.y + 6.0)).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).rounding(12.0).inner_margin(egui::Margin::same(6.0)).show(ui, |ui| {
                ui.set_width(188.0);
                let item = |ui: &mut egui::Ui, t: &str, col: Color32| {
                    let (r, resp) = ui.allocate_exact_size(vec2(188.0, 32.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(r, 8.0, CARD_HOVER());
                    }
                    ui.painter().text(r.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, t, crate::fonts::sans_font(13.0), col);
                    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                };
                if item(ui, "Store page", TEXT()) {
                    self.show_store_page(a);
                    close = true;
                }
                let pinned = self.cfg.apps.is_pinned(a);
                let full = self.cfg.apps.pin_list().len() >= crate::apps::MAX_PINS;
                if pinned && item(ui, "Unpin", TEXT()) {
                    self.cfg.apps.unpin(a);
                    self.dirty = true;
                    close = true;
                } else if !pinned && !full && item(ui, "Pin", TEXT()) {
                    self.cfg.apps.pin(a);
                    self.dirty = true;
                    close = true;
                }
                if self.cfg.apps.is_open(a) && item(ui, "Close", TEXT()) {
                    self.close_app(a);
                    close = true;
                }
                if a.meta().core {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("  Core app: always installed").size(kit::fs(11.5)).color(MUTED()));
                    ui.add_space(4.0);
                } else if item(ui, "Uninstall…", Color32::from_rgb(255, 140, 150)) {
                    self.request_uninstall(a);
                    close = true;
                }
            });
        });
        if close || (area.response.clicked_elsewhere() && !ui.rect_contains_pointer(at)) {
            self.setup.store.menu = None;
        }
    }

    fn library_list(&mut self, ui: &mut egui::Ui, installed: &[AppId]) {
        let w = ui.available_width();
        egui::Frame::none().fill(CARD()).stroke(Stroke::new(1.0f32, LINE())).rounding(16.0).inner_margin(egui::Margin::symmetric(10.0, 6.0)).show(ui, |ui| {
            ui.set_width(w - 20.0);
            let cw = w - 20.0;
            let xs = [12.0, cw * 0.36, cw * 0.52, cw * 0.67];
            let (hr, _) = ui.allocate_exact_size(vec2(cw, 34.0), Sense::hover());
            for (x, t) in xs.iter().zip(["APP", "CATEGORY", "STATUS", "LAST OPENED"]) {
                ui.painter().text(pos2(hr.min.x + x, hr.center().y), Align2::LEFT_CENTER, t, crate::fonts::sans_bold_font(10.5), MUTED());
            }
            for a in installed {
                let a = *a;
                let (r, resp) = ui.allocate_exact_size(vec2(cw, 52.0), Sense::click());
                let p = ui.painter().clone();
                p.hline(r.x_range(), r.min.y, Stroke::new(1.0f32, LINE()));
                if resp.hovered() {
                    p.rect_filled(r.shrink(2.0), 10.0, theme::blend(CARD(), TEXT(), 0.03));
                }
                let cy = r.center().y;
                kit::icon_chip(&p, Rect::from_center_size(pos2(r.min.x + 26.0, cy), vec2(30.0, 30.0)), app_icon(a), app_col(a), false);
                let g = p.text(pos2(r.min.x + 50.0, cy), Align2::LEFT_CENTER, a.name(), crate::fonts::sans_bold_font(13.5), TEXT());
                if a.meta().beta {
                    super::store::tag(&p, pos2(g.max.x + 8.0, cy - 10.0), "BETA", kit::C_GOLD);
                }
                p.text(pos2(r.min.x + xs[1], cy), Align2::LEFT_CENTER, a.meta().cat.name(), crate::fonts::sans_font(12.5), MUTED());
                let is_open = self.cfg.apps.is_open(a);
                p.text(pos2(r.min.x + xs[2], cy), Align2::LEFT_CENTER, if is_open { "● Open" } else { "Closed" }, crate::fonts::sans_font(12.5), if is_open { kit::C_CYAN } else { MUTED() });
                p.text(pos2(r.min.x + xs[3], cy), Align2::LEFT_CENTER, ago(self.cfg.apps.last_opened(a)), crate::fonts::sans_font(12.5), MUTED());
                let mut x = r.max.x - 8.0;
                if !a.meta().core {
                    x -= 84.0;
                    if pill_button(ui, Rect::from_min_size(pos2(x, cy - 15.0), vec2(84.0, 30.0)), "Uninstall", false, ("ll-un", a)).clicked() {
                        self.request_uninstall(a);
                    }
                    x -= 8.0;
                }
                x -= 70.0;
                if pill_button(ui, Rect::from_min_size(pos2(x, cy - 15.0), vec2(70.0, 30.0)), "Details", false, ("ll-d", a)).clicked() {
                    self.show_store_page(a);
                }
                x -= 8.0;
                x -= 72.0;
                if is_open {
                    if pill_button(ui, Rect::from_min_size(pos2(x, cy - 15.0), vec2(72.0, 30.0)), "Close", false, ("ll-c", a)).clicked() {
                        self.close_app(a);
                    }
                } else if pill_button(ui, Rect::from_min_size(pos2(x, cy - 15.0), vec2(72.0, 30.0)), "Open", true, ("ll-o", a)).clicked() {
                    self.open_app(a);
                }
                if resp.double_clicked() {
                    self.open_app(a);
                }
            }
        });
    }
}
