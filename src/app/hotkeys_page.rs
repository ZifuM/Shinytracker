//! The Hotkeys page: every shortcut in groups, a master switch, suggested
//! keys, swapping a key that's already used, and rows that light up when
//! their hotkey is pressed (so you can test them from the game).

use super::charts::{self as ch, CYAN, GOLD, GREEN, ORANGE, PINK, PURPLE};
use super::ui_kit::{self as kit, PillKind, MUTED, TEXT};
use super::{App, Tab};
use crate::hotkeys::{code_for, HotAction, HotGroup, KeyBind, KeyBinds};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Color32, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};
use super::notices::Notice;

impl App {
    /// A short note on the overlay and this window after a hotkey that
    /// otherwise shows nothing (sounds, all hotkeys off).
    pub(super) fn hotkey_notice(&mut self, icon: &'static str, title: &str, body: &str) {
        self.notify(Notice {
            id: "hotkey",
            icon,
            title: title.to_string(),
            body: body.to_string(),
            progress: None,
            action: None,
            expires: Some(Instant::now() + Duration::from_secs(4)),
            accent: CYAN,
        });
    }

    /// Stop recording a hotkey if its page was left (the others are switched
    /// off while recording).
    pub(super) fn hotkeys_guard(&mut self) {
        if self.setup.recording.is_some() && self.setup.tab != Tab::Hotkeys {
            self.setup.recording = None;
            self.apply_keybinds();
        }
    }

    pub(super) fn page_hotkeys(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "HOTKEYS", "Shortcuts that work even while the game is focused. Press one any time: its row lights up here.");
        self.hk_capture(ui);
        self.hk_status_card(ui);
        ui.add_space(18.0);
        let w = ui.available_width();
        if w >= 1150.0 {
            let gap = 18.0;
            let cw = ((w - gap) / 2.0).floor();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(cw);
                    self.hk_group_card(ui, HotGroup::Counting);
                    ui.add_space(18.0);
                    self.hk_group_card(ui, HotGroup::Hunt);
                });
                ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(cw);
                    self.hk_group_card(ui, HotGroup::Overlay);
                    ui.add_space(18.0);
                    self.hk_group_card(ui, HotGroup::App);
                });
            });
        } else {
            for g in HotGroup::ALL {
                self.hk_group_card(ui, g);
                ui.add_space(18.0);
            }
        }
        ui.add_space(18.0);
        hk_tips_card(ui);
        ui.add_space(30.0);
        // keep the "pressed" glow fading
        if self.hot_fired.iter().any(|(_, t)| t.elapsed() < Duration::from_secs(4)) {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
    }

    /// While recording: take the next key press as the new combination.
    fn hk_capture(&mut self, ui: &mut egui::Ui) {
        let Some(action) = self.setup.recording else { return };
        ui.ctx().request_repaint();
        let pressed = ui.ctx().input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            })
        });
        let Some((key, m)) = pressed else { return };
        let name = key.name();
        if key == egui::Key::Escape {
            self.setup.recording = None;
            self.setup.bind_msg = None;
            self.apply_keybinds();
            return;
        }
        if code_for(name).is_none() {
            self.setup.bind_msg = Some((false, format!("\"{name}\" can't be used. Try a letter, number, F1–F24, an arrow, Home / End, Page Up / Down, Insert / Delete, Space or a symbol key like [ or ;.")));
            return;
        }
        let bind = KeyBind { ctrl: m.ctrl || m.command, shift: m.shift, alt: m.alt, key: name.to_string() };
        let is_fkey = name.starts_with('F') && name.len() > 1;
        if !bind.has_modifier() && !is_fkey {
            self.setup.bind_msg = Some((false, format!("Add Ctrl, Shift or Alt to {} so it doesn't fire while you type. (F-keys work on their own.)", bind.text())));
            return;
        }
        self.setup.recording = None;
        if let Some(other) = self.cfg.keybinds.used_by(&bind, Some(action)) {
            self.setup.bind_msg = None;
            self.setup.hk_swap = Some((action, bind, other));
        } else {
            self.setup.bind_msg = Some((true, format!("{} → {}", self.hk_label(action), bind.text())));
            self.cfg.keybinds.set(action, Some(bind));
        }
        self.apply_keybinds();
    }

    fn hk_label(&self, a: HotAction) -> String {
        if a == HotAction::AddMany {
            format!("Add {}", self.cfg.keybinds.add_many_n)
        } else {
            a.label().to_string()
        }
    }

    // ───────────────────────────── status ─────────────────────────────

    fn hk_status_card(&mut self, ui: &mut egui::Ui) {
        let kb = self.cfg.keybinds.clone();
        let set = HotAction::ALL.iter().filter(|a| kb.get(**a).is_some()).count();
        let free: Vec<HotAction> = HotAction::ALL.into_iter().filter(|a| kb.get(*a).is_none() && kb.used_by(&a.suggested(), None).is_none()).collect();
        let available = self.hotkeys.available();
        let mut on = !kb.paused;
        let mut fill = false;
        let mut reset = false;
        let mut swap: Option<bool> = None;
        kit::card(ui, |ui| {
            let col = if !available { theme::HP_RED } else if on { GREEN } else { ORANGE };
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::hover());
                kit::icon_chip(ui.painter(), r, "⌨", col, true);
                ui.add_space(8.0);
                let tw = (ui.available_width() - 70.0).max(200.0);
                ui.allocate_ui_with_layout(vec2(tw, 46.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(tw);
                    let title = if !available {
                        "Hotkeys can't be used"
                    } else if on {
                        "Hotkeys are on"
                    } else {
                        "Hotkeys are off"
                    };
                    ui.label(RichText::new(title).font(ch::bold(17.0)).color(TEXT()));
                    let sub = if !available {
                        "Windows didn't let the Companion listen for keys. Restarting it usually helps.".to_string()
                    } else if on {
                        format!("{set} of {} set · they work even while the game is focused", HotAction::ALL.len())
                    } else {
                        match kb.get(HotAction::HotkeysToggle) {
                            Some(b) => format!("Only {} works, to turn them back on.", b.text()),
                            None => "None of them work until you turn them back on.".to_string(),
                        }
                    };
                    ui.label(RichText::new(sub).size(12.5).color(MUTED()));
                });
                ui.add_space((ui.available_width() - 48.0).max(4.0));
                kit::toggle(ui, &mut on).on_hover_text("Turn every hotkey off for a while (say, when you're typing in chat)");
            });
            if !free.is_empty() || set != HotAction::ALL.len() || kb != KeyBinds::default() {
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    if !free.is_empty() {
                        let names: Vec<String> = free.iter().map(|a| format!("{}: {}", a.label(), a.suggested().text())).collect();
                        if kit::accent_button(ui, egui::Button::new(RichText::new(format!("Set up {} suggested", free.len())).font(ch::bold(13.0)).color(kit::on_primary())), 6.0)
                            .on_hover_text(names.join("\n"))
                            .clicked()
                        {
                            fill = true;
                        }
                    }
                    if kb != KeyBinds::default() && ui.add(egui::Button::new("Reset to defaults").fill(kit::btn_fill())).clicked() {
                        reset = true;
                    }
                });
            }
            // "already used": move it here?
            if let Some((a, b, other)) = &self.setup.hk_swap {
                ui.add_space(12.0);
                egui::Frame::none().fill(ch::faint(GOLD, 0.14)).rounding(12.0).inner_margin(egui::Margin::symmetric(12.0, 10.0)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                        ui.label(RichText::new(format!("{} is already “{}”.", b.text(), other.label())).size(13.0).color(TEXT()));
                        if kit::accent_button(ui, egui::Button::new(RichText::new(format!("Use it for “{}”", a.label())).font(ch::bold(12.5)).color(kit::on_primary())), 6.0).clicked() {
                            swap = Some(true);
                        }
                        if ui.add(egui::Button::new("Keep it").fill(kit::btn_fill())).clicked() {
                            swap = Some(false);
                        }
                    });
                    ui.label(RichText::new(format!("Moving it turns “{}” off until you give it another key.", other.label())).size(11.5).color(MUTED()));
                });
            }
            if let Some((ok, msg)) = &self.setup.bind_msg {
                ui.add_space(10.0);
                kit::pill(ui, msg, if *ok { PillKind::Ok } else { PillKind::Bad });
            }
            for e in self.hotkeys.errors.clone() {
                ui.add_space(8.0);
                kit::pill(ui, &e, PillKind::Bad);
            }
        });
        if on == kb.paused {
            self.cfg.keybinds.paused = !on;
            self.apply_keybinds();
        }
        if fill {
            let n = self.cfg.keybinds.fill_suggested();
            self.setup.bind_msg = Some((true, format!("Set up {n} hotkeys. Change any of them below.")));
            self.apply_keybinds();
        }
        if reset {
            self.cfg.keybinds = KeyBinds::default();
            self.setup.recording = None;
            self.setup.hk_swap = None;
            self.setup.bind_msg = Some((true, "Hotkeys reset to defaults".into()));
            self.apply_keybinds();
        }
        match swap {
            Some(true) => {
                if let Some((a, b, other)) = self.setup.hk_swap.take() {
                    self.cfg.keybinds.set(other, None);
                    self.cfg.keybinds.set(a, Some(b.clone()));
                    self.setup.bind_msg = Some((true, format!("{} → {} (“{}” is off now)", self.hk_label(a), b.text(), other.label())));
                    self.apply_keybinds();
                }
            }
            Some(false) => self.setup.hk_swap = None,
            None => {}
        }
    }

    // ───────────────────────────── groups ─────────────────────────────

    fn hk_group_card(&mut self, ui: &mut egui::Ui, g: HotGroup) {
        let (icon, col, sub) = match g {
            HotGroup::Counting => ("🎯", PINK, "Fix the count without leaving the game."),
            HotGroup::Overlay => ("🖵", CYAN, "Show, hide and move between layouts."),
            HotGroup::Hunt => ("✨", GOLD, "For the moment the shiny shows up."),
            HotGroup::App => ("⚙", PURPLE, "Sounds, this window and the hotkeys themselves."),
        };
        let actions: Vec<HotAction> = HotAction::ALL.into_iter().filter(|a| a.group() == g).collect();
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
                kit::icon_chip(ui.painter(), r, icon, col, false);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(g.label()).font(ch::bold(16.0)).color(TEXT()));
                    ui.label(RichText::new(sub).size(11.5).color(MUTED()));
                });
            });
            ui.add_space(10.0);
            for (i, a) in actions.iter().enumerate() {
                if i > 0 {
                    ui.add_space(4.0);
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, kit::LINE()));
                    ui.add_space(4.0);
                }
                self.hk_row(ui, *a, col);
            }
        });
    }

    fn hk_row(&mut self, ui: &mut egui::Ui, a: HotAction, col: Color32) {
        let bind = self.cfg.keybinds.get(a).cloned();
        let recording = self.setup.recording == Some(a);
        let fired = self.hot_fired.iter().rev().find(|(x, _)| *x == a).map(|(_, t)| t.elapsed().as_secs_f32());
        let failed = self.hotkeys.failed.contains(&a);
        let paused = self.cfg.keybinds.paused && a != HotAction::HotkeysToggle;
        let bg = ui.painter().add(egui::Shape::Noop);
        let mut start = false;
        let mut clear = false;
        let mut cancel = false;
        // The control's width and the text's height, both measured the frame
        // before: the row is as tall as its text, with everything centred.
        let cid = ui.id().with(("hk-ctrl-w", a.label()));
        let hid = ui.id().with(("hk-text-h", a.label()));
        let cw = ui.ctx().data(|d| d.get_temp::<f32>(cid)).unwrap_or(220.0);
        let th = ui.ctx().data(|d| d.get_temp::<f32>(hid)).unwrap_or(44.0);
        let row = ui
            .horizontal(|ui| {
                ui.set_min_height(th);
                ui.spacing_mut().item_spacing.x = 10.0;
                let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                kit::icon_chip(ui.painter(), r, a.icon(), col, false);
                let tw = (ui.available_width() - cw - 16.0).max(150.0);
                let text = ui.allocate_ui_with_layout(vec2(tw, th), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(tw);
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                        ui.label(RichText::new(self.hk_label(a)).font(ch::bold(13.5)).color(TEXT()));
                        if let Some(s) = fired.filter(|s| *s < 4.0) {
                            let g = 1.0 - (s / 4.0);
                            mini_pill(ui, "✔ Pressed", GREEN, g.max(0.35));
                        } else if failed {
                            mini_pill(ui, "Not working", theme::HP_RED, 1.0).on_hover_text("Windows wouldn't give the Companion this key: another app probably uses it. Pick another.");
                        } else if a.is_new() && bind.is_none() {
                            mini_pill(ui, "New", PURPLE, 1.0);
                        }
                    });
                    ui.add(egui::Label::new(RichText::new(a.about()).size(11.5).color(MUTED())).wrap());
                    if a == HotAction::AddMany {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Adds").size(12.0).color(MUTED()));
                            let mut n = self.cfg.keybinds.add_many_n.clamp(1, 99) as u32;
                            if kit::stepper(ui, &mut n, 2, 99) {
                                self.cfg.keybinds.add_many_n = n as u64;
                                self.dirty = true;
                            }
                        });
                    }
                    ui.add_space(6.0);
                });
                let h = text.response.rect.height();
                if (h - th).abs() > 0.5 {
                    ui.ctx().data_mut(|d| d.insert_temp(hid, h));
                    ui.ctx().request_repaint();
                }
                ui.add_space((ui.available_width() - cw).max(4.0));
                let x0 = ui.cursor().min.x;
                ui.spacing_mut().item_spacing.x = 4.0;
                if recording {
                    let held = ui.input(|i| i.modifiers);
                    let pulse = (ui.input(|i| i.time) as f32 * 4.0).sin() * 0.5 + 0.5;
                    let mut parts: Vec<&str> = Vec::new();
                    if held.ctrl || held.command {
                        parts.push("Ctrl");
                    }
                    if held.alt {
                        parts.push("Alt");
                    }
                    if held.shift {
                        parts.push("Shift");
                    }
                    if parts.is_empty() {
                        ui.label(RichText::new("Press the keys…").font(ch::bold(13.0)).color(theme::blend(ch::ink(GOLD), TEXT(), pulse * 0.4)));
                    } else {
                        for p in parts {
                            cap(ui, p, true);
                        }
                        ui.label(RichText::new("+ …").font(ch::bold(13.0)).color(ch::ink(GOLD)));
                    }
                    ui.add_space(6.0);
                    cancel = ui.add(egui::Button::new("Cancel").fill(kit::btn_fill())).on_hover_text("Or press Esc").clicked();
                } else {
                    match &bind {
                        Some(b) => {
                            for p in b.parts() {
                                cap(ui, p, !paused);
                            }
                            cap(ui, crate::hotkeys::key_label(&b.key), !paused);
                        }
                        None => {
                            ui.label(RichText::new("Off").size(12.5).color(MUTED())).on_hover_text(format!("Suggested: {}", a.suggested().text()));
                        }
                    }
                    ui.add_space(6.0);
                    let label = if bind.is_some() { "Change" } else { "Set" };
                    start = ui.add(egui::Button::new(label).fill(kit::btn_fill())).on_hover_text("Then press the keys you want").clicked();
                    if bind.is_some() {
                        clear = ui.add(egui::Button::new(RichText::new("×").size(15.0)).fill(kit::btn_fill()).min_size(vec2(30.0, 0.0))).on_hover_text("Turn this hotkey off").clicked();
                    }
                }
                let used = ui.min_rect().max.x - x0;
                if (used - cw).abs() > 0.5 {
                    ui.ctx().data_mut(|d| d.insert_temp(cid, used));
                    ui.ctx().request_repaint();
                }
            })
            .response
            .rect;
        // glow behind a row that was just pressed, or the one recording
        let glow = if recording {
            Some((GOLD, 0.12))
        } else {
            fired.filter(|s| *s < 1.6).map(|s| (GREEN, 0.22 * (1.0 - s / 1.6)))
        };
        if let Some((c, k)) = glow {
            let r = Rect::from_min_max(pos2(row.min.x - 8.0, row.min.y), pos2(row.max.x + 8.0, row.max.y));
            ui.painter().set(bg, egui::Shape::rect_filled(r, 10.0, ch::faint(c, k)));
            if recording {
                ui.painter().rect_stroke(r, 10.0, Stroke::new(1.0f32, ch::faint(GOLD, 0.6)));
            }
        }
        if start {
            self.setup.recording = Some(a);
            self.setup.bind_msg = None;
            self.setup.hk_swap = None;
            self.hotkeys.suspend();
        }
        if cancel {
            self.setup.recording = None;
            self.apply_keybinds();
        }
        if clear {
            self.cfg.keybinds.set(a, None);
            self.setup.bind_msg = Some((true, format!("{} turned off", self.hk_label(a))));
            self.apply_keybinds();
        }
    }
}

/// A key cap (dimmed while hotkeys are off).
fn cap(ui: &mut egui::Ui, text: &str, live: bool) {
    let g = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::monospace(12.5), TEXT());
    let w = (g.size().x + 16.0).max(28.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
    let p = ui.painter();
    let face = Rect::from_min_size(r.min, vec2(r.width(), 23.0));
    p.rect_filled(face.translate(vec2(0.0, 3.0)), 6.0, theme::blend(kit::LINE(), Color32::BLACK, 0.15));
    p.rect_filled(face, 6.0, kit::CARD_HOVER());
    p.rect_stroke(face, 6.0, Stroke::new(1.0f32, kit::LINE()));
    let ink = if live { TEXT() } else { MUTED() };
    p.galley(face.center() - g.size() / 2.0, g, ink);
}

/// A small coloured label.
fn mini_pill(ui: &mut egui::Ui, text: &str, c: Color32, alpha: f32) -> egui::Response {
    let g = ui.painter().layout_no_wrap(text.to_string(), ch::bold(10.5), ch::ink(c));
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 14.0, 18.0), Sense::hover());
    ui.painter().rect_filled(r, 9.0, ch::faint(c, 0.18 * alpha));
    ui.painter().galley(r.center() - g.size() / 2.0, g, theme::blend(kit::CARD(), ch::ink(c), alpha));
    resp
}

fn hk_tips_card(ui: &mut egui::Ui) {
    kit::card(ui, |ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
            kit::icon_chip(ui.painter(), r, "💡", GOLD, false);
            ui.add_space(4.0);
            ui.label(RichText::new("Good to know").font(ch::bold(16.0)).color(TEXT()));
        });
        ui.add_space(10.0);
        let tips = [
            ("Ctrl, Shift or Alt", "Every hotkey needs one (except F-keys), so typing never sets one off."),
            ("Macro pads & stream decks", "F13–F24 work too: map a pad button to one and nothing else will clash."),
            ("Typing in chat?", "Flip the switch at the top, or give \"All hotkeys off / on\" a key."),
            ("Not working?", "Another app may already use that key. Its row says so: pick a different one."),
        ];
        let avail = ui.available_width();
        let cols = if avail >= 760.0 { 2 } else { 1 };
        let cw = ((avail - 18.0 * (cols as f32 - 1.0)) / cols as f32).floor();
        for row in tips.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 18.0;
                for (t, b) in row {
                    ui.allocate_ui_with_layout(vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(cw);
                        ui.label(RichText::new(*t).font(ch::bold(13.0)).color(TEXT()));
                        ui.add(egui::Label::new(RichText::new(*b).size(12.0).color(MUTED())).wrap());
                    });
                }
            });
            ui.add_space(10.0);
        }
    });
}
