//! Questions the App Store asks (drawn over everything), its short toasts,
//! and the lock on features that need another app: greyed with a 🔒, and
//! clicking says which app it needs and offers to install it.

use super::store::{app_col, app_icon, pill_button, AppModal};
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::App;
use crate::apps::AppId;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use std::time::Duration;

/// A lock clicked somewhere this frame: which app, for what.
static ASK: std::sync::Mutex<Option<(AppId, String)>> = std::sync::Mutex::new(None);

/// A greyed "🔒 feature" button for a feature that needs `need`, usable
/// anywhere (no borrow of the app): clicking asks to install the app.
pub(super) fn lock_btn(ui: &mut egui::Ui, need: AppId, label: &str) -> egui::Response {
    let resp = ui
        .add(
            egui::Button::new(egui::RichText::new(format!("🔒 {label}")).size(kit::fs(12.5)).color(MUTED()))
                .fill(kit::btn_fill().gamma_multiply(0.6))
                .stroke(Stroke::new(1.0f32, LINE()))
                .rounding(kit::btn_round(8.0)),
        )
        .on_hover_text(format!("Needs {}. Click to install it.", need.name()))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if resp.clicked() {
        if let Ok(mut a) = ASK.lock() {
            *a = Some((need, label.to_string()));
        }
    }
    resp
}

/// Is `a` installed? (Any page can ask.)
pub(super) fn have(a: AppId) -> bool {
    crate::apps::live(a)
}

impl App {
    /// Cover `rect` (a card) with a lock: "Needs the Map app". Clicking asks
    /// to install it.
    pub(super) fn lock_veil(&mut self, ui: &mut egui::Ui, rect: Rect, need: AppId, feature: &str) {
        let resp = ui.interact(rect, ui.id().with(("lock-veil", need, feature)), Sense::click());
        let p = ui.painter();
        p.rect_filled(rect, 16.0, theme::blend(kit::PAGE(), CARD(), 0.4).gamma_multiply(if resp.hovered() { 0.8 } else { 0.88 }));
        let c = rect.center();
        p.text(c + vec2(0.0, -22.0), Align2::CENTER_CENTER, "🔒", egui::FontId::proportional(20.0), TEXT());
        p.text(c + vec2(0.0, 4.0), Align2::CENTER_CENTER, format!("Needs the {} app", need.name()), crate::fonts::sans_bold_font(14.0), TEXT());
        p.text(c + vec2(0.0, 24.0), Align2::CENTER_CENTER, "Click to install it", crate::fonts::sans_font(12.0), MUTED());
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.ask_for_app(need, feature);
        }
    }

    pub(super) fn ask_for_app(&mut self, need: AppId, feature: &str) {
        self.setup.store.modal = Some(AppModal::Lock { need, feature: feature.to_string() });
    }

    /// The open question, if any, and the toast. Drawn last, over everything.
    pub(super) fn app_overlays(&mut self, ctx: &egui::Context) {
        if let Some((need, label)) = ASK.lock().ok().and_then(|mut a| a.take()) {
            self.ask_for_app(need, &label);
        }
        if let Some((t, msg)) = self.setup.store.toast.clone() {
            let age = t.elapsed();
            if age > Duration::from_millis(2600) {
                self.setup.store.toast = None;
            } else {
                let screen = ctx.screen_rect();
                let a = (1.0 - (age.as_secs_f32() - 2.2).max(0.0) / 0.4).clamp(0.0, 1.0);
                egui::Area::new(egui::Id::new("app-toast")).order(egui::Order::Tooltip).interactable(false).fixed_pos(pos2(screen.center().x, screen.max.y - 70.0)).pivot(Align2::CENTER_CENTER).show(ctx, |ui| {
                    let g = ui.painter().layout_no_wrap(format!("✔  {msg}"), crate::fonts::sans_bold_font(13.0), Color32::from_rgb(200, 247, 220));
                    let (r, _) = ui.allocate_exact_size(g.size() + vec2(32.0, 20.0), Sense::hover());
                    let p = ui.painter();
                    p.add(kit::shadow_tier(2).as_shape(r, 14.0));
                    p.rect_filled(r, 14.0, Color32::from_rgb(28, 46, 38).gamma_multiply(a));
                    p.rect_stroke(r, 14.0, Stroke::new(1.0f32, Color32::from_rgb(44, 122, 85).gamma_multiply(a)));
                    p.galley(r.min + vec2(16.0, 10.0), g, Color32::WHITE.gamma_multiply(a));
                });
                ctx.request_repaint_after(Duration::from_millis(50));
            }
        }
        let Some(m) = self.setup.store.modal.clone() else { return };
        let screen = ctx.screen_rect();
        // the scrim (also eats clicks meant for the page underneath)
        let scrim = egui::Area::new(egui::Id::new("app-modal-scrim")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(r, 0.0, Color32::from_black_alpha(150));
            resp
        });
        let mut act: Option<Act> = None;
        egui::Area::new(egui::Id::new("app-modal")).order(egui::Order::Tooltip).fixed_pos(screen.center()).pivot(Align2::CENTER_CENTER).show(ctx, |ui| {
            kit::settings_style(ui);
            egui::Frame::none().fill(CARD()).stroke(Stroke::new(1.0f32, kit::CONTROL())).rounding(22.0).inner_margin(egui::Margin::same(26.0)).shadow(kit::shadow_tier(3)).show(ui, |ui| {
                ui.set_width(520.0);
                act = self.modal_body(ui, &m);
            });
        });
        if scrim.inner.clicked() && !matches!(m, AppModal::Requires { .. }) && !self.setup.store.busy.values().next().is_some() {
            act = act.or(Some(Act::Cancel));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            act = Some(Act::Cancel);
        }
        match act {
            None => {}
            Some(Act::Cancel) => self.setup.store.modal = None,
            Some(Act::Install(ids)) => self.start_install(&ids),
            Some(Act::Store(a)) => {
                self.setup.store.modal = None;
                self.show_store_page(a);
            }
            Some(Act::Remove(ids)) => {
                self.setup.store.modal = None;
                self.do_uninstall(&ids);
            }
            Some(Act::ClearSprites) => {
                self.setup.store.modal = None;
                let _ = std::fs::remove_dir_all(crate::config::cache_dir().join("sprites"));
                self.setup.store.sizes_got = None;
                self.app_toast("Sprites cleared. They download again when needed".to_string());
            }
        }
    }

    fn modal_body(&mut self, ui: &mut egui::Ui, m: &AppModal) -> Option<Act> {
        let busy = |s: &Self, ids: &[AppId]| ids.iter().any(|a| s.setup.store.busy.contains_key(a));
        let head = |ui: &mut egui::Ui, icons: &[AppId], lock: bool| {
            ui.horizontal(|ui| {
                for a in icons {
                    let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                    kit::icon_chip(ui.painter(), r, app_icon(*a), app_col(*a), false);
                }
                if lock {
                    let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 15.0, theme::blend(CARD(), kit::C_GOLD, 0.22));
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, "🔒", egui::FontId::proportional(14.0), kit::C_GOLD);
                }
            });
            ui.add_space(14.0);
        };
        let title = |ui: &mut egui::Ui, t: &str| {
            ui.label(egui::RichText::new(t).font(crate::fonts::title_font(20.0)).color(TEXT()));
            ui.add_space(6.0);
        };
        let body = |ui: &mut egui::Ui, t: &str| {
            ui.label(egui::RichText::new(t).size(kit::fs(13.5)).color(theme::blend(CARD(), TEXT(), 0.8)));
            ui.add_space(18.0);
        };
        let mut out = None;
        let row = |ui: &mut egui::Ui, buttons: &[(&str, bool, Act)], out: &mut Option<Act>| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (k, (label, primary, act)) in buttons.iter().enumerate() {
                    let g = ui.painter().layout_no_wrap(label.to_string(), crate::fonts::sans_bold_font(14.0), TEXT());
                    let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 36.0, 42.0), Sense::hover());
                    if pill_button(ui, r, label, *primary, ("modal-btn", k)).clicked() {
                        *out = Some(act.clone());
                    }
                }
            });
        };
        let installing = |ui: &mut egui::Ui| {
            let (r, _) = ui.allocate_exact_size(vec2(180.0, 42.0), Sense::hover());
            let p = ui.painter();
            p.rect_filled(r, 21.0, kit::sunken());
            p.text(r.center(), Align2::CENTER_CENTER, "Installing…", crate::fonts::sans_bold_font(14.0), TEXT());
        };
        match m {
            AppModal::Lock { need, feature } => {
                head(ui, &[*need], true);
                title(ui, &format!("{} is required", need.name()));
                let what = if feature.is_empty() { "This".to_string() } else { format!("“{feature}”") };
                body(ui, &format!("{what} uses the {} app. Please install {} to use it. It only takes a moment, and nothing you've saved changes.", need.name(), need.name()));
                if busy(self, &[*need]) {
                    installing(ui);
                } else {
                    let need = *need;
                    row(ui, &[(&format!("Install {}", need.name()), true, Act::Install(crate::apps::with_requirements(need, &self.cfg.apps.installed()))), ("See in App Store", false, Act::Store(need)), ("Not now", false, Act::Cancel)], &mut out);
                }
            }
            AppModal::Requires { app, need } => {
                let mut icons = vec![*app];
                icons.extend(need.iter().copied());
                head(ui, &icons, false);
                let names: Vec<&str> = need.iter().map(|a| a.name()).collect();
                title(ui, &format!("{} needs {}", app.name(), names.join(", ")));
                body(ui, &format!("{} uses your {} teams, so it can't work without it. Install both together?", app.name(), names.join(", ")));
                let mut all = need.clone();
                all.push(*app);
                if busy(self, &all) {
                    installing(ui);
                } else {
                    row(ui, &[("Install both", true, Act::Install(all)), ("Cancel", false, Act::Cancel)], &mut out);
                }
            }
            AppModal::Dependents { app, deps } => {
                head(ui, &[*app], false);
                let names: Vec<&str> = deps.iter().map(|a| a.name()).collect();
                title(ui, &format!("{} needs {}", names.join(", "), app.name()));
                body(ui, &format!("If you remove {}, {} won't work. Remove {} too? Your teams and battles are kept either way, and come back if you reinstall.", app.name(), names.join(", "), if deps.len() == 1 { "it" } else { "them" }));
                let mut all = vec![*app];
                all.extend(deps.iter().copied());
                row(ui, &[("Remove both", false, Act::Remove(all)), (&format!("Keep {}", app.name()), true, Act::Cancel)], &mut out);
            }
            AppModal::Uninstall { app } => {
                head(ui, &[*app], false);
                title(ui, &format!("Uninstall {}?", app.name()));
                body(ui, &format!("{} leaves your sidebar and Library. Everything you saved in it stays on this PC, so reinstalling brings it all back.", app.name()));
                row(ui, &[("Uninstall", false, Act::Remove(vec![*app])), ("Cancel", true, Act::Cancel)], &mut out);
            }
            AppModal::ClearSprites => {
                title(ui, "Clear cached sprites?");
                body(ui, "The Pokémon pictures the companion has downloaded are deleted. They download again when a page needs them (that needs the internet). Your hunts and saves aren't touched.");
                row(ui, &[("Clear sprites", false, Act::ClearSprites), ("Cancel", true, Act::Cancel)], &mut out);
            }
        }
        out
    }
}

#[derive(Clone)]
enum Act {
    Cancel,
    Install(Vec<AppId>),
    Store(AppId),
    Remove(Vec<AppId>),
    ClearSprites,
}
