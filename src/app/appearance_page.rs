//! The Appearance page: style, light or dark, colours and palettes, with a
//! live preview that follows you down the page and tries a choice while you
//! hover it. Looks can be shared as a short code.

use super::charts::{self as ch, CYAN, GOLD, GREEN, ORANGE, PINK, PURPLE};
use super::hunt_pages::{mode_tile, swatch_row};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::App;
use crate::config::{Appearance, UiMode};
use crate::theme::{self, Look};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

/// A ready-made colour scheme.
struct Palette {
    name: &'static str,
    accent: [u8; 3],
    second: Option<[u8; 3]>,
}

const PALETTES: [Palette; 11] = [
    Palette { name: "Poké Red", accent: [214, 52, 58], second: None },
    Palette { name: "Twilight", accent: [128, 78, 208], second: Some([240, 120, 60]) },
    Palette { name: "Sunset", accent: [236, 72, 92], second: Some([250, 160, 60]) },
    Palette { name: "Shiny Gold", accent: [206, 150, 20], second: Some([236, 104, 40]) },
    Palette { name: "Ocean", accent: [36, 110, 220], second: Some([24, 178, 196]) },
    Palette { name: "Forest", accent: [40, 146, 80], second: Some([142, 190, 52]) },
    Palette { name: "Sakura", accent: [214, 86, 150], second: Some([242, 150, 186]) },
    Palette { name: "Glacier", accent: [58, 136, 214], second: Some([132, 206, 232]) },
    Palette { name: "Ember", accent: [196, 40, 40], second: Some([246, 120, 30]) },
    Palette { name: "Neon", accent: [0, 176, 152], second: Some([146, 80, 246]) },
    Palette { name: "Midnight", accent: [62, 72, 182], second: Some([146, 64, 204]) },
];

/// What the mouse is over: the look it would give, and its name.
type Hover = Option<(Appearance, String)>;

impl App {
    pub(super) fn page_appearance(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "APPEARANCE", "Light or dark, and your colours. Hover any choice to try it in the preview first.");
        // "Undo changes" goes back to the look the page was opened with.
        let visit = kit::page_visit(ui.ctx());
        if self.setup.look_start.as_ref().map(|(v, _)| *v) != Some(visit) {
            self.setup.look_start = Some((visit, self.cfg.appearance.clone()));
            self.setup.look_msg = None;
        }
        let before = self.cfg.appearance.clone();
        // the preview shows what the mouse was over last frame
        let hid = egui::Id::new("look-hover");
        let shown: Hover = ui.ctx().data(|d| d.get_temp::<Option<(Appearance, String)>>(hid)).flatten();
        let mut hover: Hover = None;

        let w = ui.available_width();
        if w >= 1000.0 {
            let gap = 20.0;
            let pw = (w * 0.40).clamp(360.0, 460.0);
            let lw = w - pw - gap;
            let x0 = ui.cursor().min.x;
            let top = ui.cursor().min.y;
            let h = ui.available_height();
            let left = ui
                .allocate_ui_with_layout(vec2(lw, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(lw);
                    self.look_settings(ui, &mut hover);
                })
                .response
                .rect;
            let col = Rect::from_min_max(pos2(x0 + lw + gap, top), pos2(x0 + w, left.max.y));
            self.look_sticky_preview(ui, col, &shown);
        } else {
            self.look_preview_card(ui, &shown);
            ui.add_space(18.0);
            self.look_settings(ui, &mut hover);
        }
        ui.add_space(30.0);
        ui.ctx().data_mut(|d| d.insert_temp(hid, hover));
        if self.cfg.appearance != before {
            self.dirty = true;
        }
    }

    fn look_settings(&mut self, ui: &mut egui::Ui, hover: &mut Hover) {
        self.look_style_card(ui, hover);
        ui.add_space(18.0);
        self.look_colours_card(ui, hover);
        ui.add_space(18.0);
        self.look_comfort_card(ui);
        ui.add_space(18.0);
        self.look_share_card(ui);
    }

    // ───────────────────────────── style ─────────────────────────────

    fn look_style_card(&mut self, ui: &mut egui::Ui, hover: &mut Hover) {
        kit::card(ui, |ui| {
            card_head(ui, "✨", PURPLE, "Mode");
            ui.add_space(14.0);
            let a = self.cfg.appearance.clone();
            let avail = ui.available_width();
            let w4 = ((avail - 30.0) / 4.0).floor();
            let mut pick: Option<Option<UiMode>> = None;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                for (mode, name) in [(UiMode::Light, "Light"), (UiMode::SemiDark, "Semi-dark"), (UiMode::Dark, "Dark")] {
                    // the tile shows its own mode as picked only when Windows isn't in charge
                    let mut shown = a.clone();
                    if a.follow_system {
                        shown.mode = if mode == UiMode::Light { UiMode::Dark } else { UiMode::Light };
                    }
                    let r = mode_tile(ui, w4, mode, name, &shown);
                    if r.hovered() && (a.mode != mode || a.follow_system) {
                        let mut c = a.clone();
                        c.mode = mode;
                        c.follow_system = false;
                        *hover = Some((c, format!("{name} mode")));
                    }
                    if r.clicked() {
                        pick = Some(Some(mode));
                    }
                }
                let r = system_tile(ui, w4, a.follow_system, &a);
                if r.hovered() && !a.follow_system {
                    let mut c = a.clone();
                    c.follow_system = true;
                    if let Some(t) = ui.ctx().system_theme() {
                        c.mode = if t == egui::Theme::Light { UiMode::Light } else { UiMode::Dark };
                    }
                    *hover = Some((c, "Match Windows".into()));
                }
                if r.on_hover_text("Light or dark, the same as Windows. Changes by itself when Windows does.").clicked() {
                    pick = Some(None);
                }
            });
            match pick {
                Some(Some(m)) => {
                    self.cfg.appearance.mode = m;
                    self.cfg.appearance.follow_system = false;
                }
                Some(None) => {
                    self.cfg.appearance.follow_system = true;
                    if let Some(t) = ui.ctx().system_theme() {
                        self.cfg.appearance.mode = if t == egui::Theme::Light { UiMode::Light } else { UiMode::Dark };
                    }
                }
                None => {}
            }
            if self.cfg.appearance.follow_system {
                ui.add_space(6.0);
                let now = match self.cfg.appearance.mode {
                    UiMode::Light => "light",
                    _ => "dark",
                };
                ui.label(RichText::new(format!("Following Windows: {now} right now.")).size(11.5).color(MUTED()));
            }
        });
    }

    // ───────────────────────────── colours ─────────────────────────────

    fn look_colours_card(&mut self, ui: &mut egui::Ui, hover: &mut Hover) {
        let hunt = self.hunt_palette();
        kit::card(ui, |ui| {
            card_head(ui, "🎨", PINK, "Colours");
            ui.add_space(14.0);
            label(ui, "PALETTES");
            ui.add_space(8.0);
            let a = self.cfg.appearance.clone();
            let avail = ui.available_width();
            let cols = ((avail + 10.0) / (122.0 + 10.0)).floor().clamp(2.0, 6.0);
            let tw = ((avail - 10.0 * (cols - 1.0)) / cols).floor();
            let mut list: Vec<(String, [u8; 3], Option<[u8; 3]>, bool)> = Vec::new();
            if let Some((name, acc, sec)) = &hunt {
                list.push((name.clone(), *acc, *sec, true));
            }
            for p in &PALETTES {
                list.push((p.name.to_string(), p.accent, p.second, false));
            }
            let mut pick: Option<([u8; 3], Option<[u8; 3]>)> = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                for (name, acc, sec, from_hunt) in &list {
                    let on = palette_on(&a, *acc, *sec);
                    let r = palette_tile(ui, tw, name, *acc, *sec, on, *from_hunt);
                    if r.hovered() && !on {
                        let mut c = a.clone();
                        apply_palette(&mut c, *acc, *sec);
                        *hover = Some((c, name.clone()));
                    }
                    let r = if *from_hunt { r.on_hover_text("The colours of the Pokémon you're hunting") } else { r };
                    if r.clicked() {
                        pick = Some((*acc, *sec));
                    }
                }
            });
            if let Some((acc, sec)) = pick {
                apply_palette(&mut self.cfg.appearance, acc, sec);
            }

            ui.add_space(16.0);
            divider(ui);
            ui.add_space(14.0);
            // header colour
            ui.horizontal(|ui| {
                label(ui, "MAIN COLOUR");
                kit::info(ui, "The header bar, selected things, progress bars and (unless you change it below) buttons.");
            });
            ui.add_space(8.0);
            {
                let a = &mut self.cfg.appearance;
                swatch_row(ui, &mut a.accent, "accent");
                ui.add_space(10.0);
                kit::toggle_row(ui, "Blend into a second colour", "A gradient from your main colour into another one.", &mut a.gradient);
                if a.gradient {
                    ui.add_space(8.0);
                    swatch_row(ui, &mut a.accent2, "accent2");
                }
            }

            ui.add_space(16.0);
            divider(ui);
            ui.add_space(14.0);
            // buttons
            let a = &mut self.cfg.appearance;
            let mut mode: u8 = if a.separate_buttons { 2 } else if a.solid_buttons { 1 } else { 0 };
            let before = mode;
            ui.horizontal(|ui| {
                label(ui, "BUTTONS");
                kit::info(ui, "Caught!, Share, Start overlay, switches and selected options.");
            });
            ui.add_space(8.0);
            kit::segmented(ui, &mut mode, &[(0u8, "Same as main"), (1, "Main, no blend"), (2, "Their own")]);
            if mode != before {
                a.separate_buttons = mode == 2;
                a.solid_buttons = mode == 1;
            }
            if a.separate_buttons {
                ui.add_space(10.0);
                swatch_row(ui, &mut a.button, "button");
                ui.add_space(10.0);
                kit::toggle_row(ui, "Blend into a second colour", "Each button goes from this colour into another one.", &mut a.button_gradient);
                if a.button_gradient {
                    ui.add_space(8.0);
                    swatch_row(ui, &mut a.button2, "button2");
                }
            }
        });
    }

    /// "Your hunt": colours from the target's types.
    fn hunt_palette(&self) -> Option<(String, [u8; 3], Option<[u8; 3]>)> {
        let w = self.wiki.as_ref()?;
        let sp = self.species.get(self.target_idx()?)?;
        let mon = w.by_dex(sp.dex)?;
        let rgb = |c: Color32| [c.r(), c.g(), c.b()];
        let t1 = super::pokedex::type_color(mon.types.first()?);
        let second = match mon.types.get(1) {
            Some(t) => super::pokedex::type_color(t),
            None => theme::blend(t1, Color32::WHITE, 0.35),
        };
        // a touch deeper, so white text on it stays readable
        let deep = theme::blend(t1, Color32::BLACK, 0.12);
        Some((format!("Your hunt: {}", sp.name), rgb(deep), Some(rgb(second))))
    }

    // ───────────────────────────── comfort ─────────────────────────────

    fn look_comfort_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "⚙", CYAN, "Layout & motion");
            ui.add_space(12.0);
            let a = &mut self.cfg.appearance;
            kit::toggle_row(
                ui,
                "Animations",
                "Pages fade in, tabs and the sidebar's highlight slide, buttons ease on hover and charts grow in. Off for a still window. (The overlay's own effects aren't affected.)",
                &mut a.animations,
            );
            {
                ui.add_space(10.0);
                divider(ui);
                ui.add_space(10.0);
                let mut compact = a.nav_compact;
                setting_row(ui, "Sidebar", "Icons only gives the pages more room. The « button at the bottom of the sidebar does the same.", |ui| {
                    kit::segmented(ui, &mut compact, &[(false, "Full"), (true, "Icons only")]);
                });
                a.nav_compact = compact;
            }
            ui.add_space(10.0);
            divider(ui);
            ui.add_space(12.0);
            self.pins_place_picker(ui);
        });
    }

    // ───────────────────────────── share ─────────────────────────────

    fn look_share_card(&mut self, ui: &mut egui::Ui) {
        let code = look_code(&self.cfg.appearance);
        let start = self.setup.look_start.as_ref().map(|(_, a)| a.clone());
        kit::card(ui, |ui| {
            card_head(ui, "📌", GOLD, "Share & reset");
            ui.add_space(12.0);
            ui.label(RichText::new("Your look as a code. Send it to a friend, or paste theirs to try it.").size(12.5).color(MUTED()));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                egui::Frame::none()
                    .fill(kit::CARD_HOVER())
                    .stroke(Stroke::new(1.0f32, kit::LINE()))
                    .rounding(8.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(&code).monospace().size(12.5).color(TEXT()));
                    });
                if ui.add(egui::Button::new("Copy").fill(kit::btn_fill())).clicked() {
                    ui.ctx().copy_text(code.clone());
                    self.setup.look_msg = Some((true, "Copied. Paste it anywhere to share your look.".into()));
                }
            });
            ui.add_space(10.0);
            let mut apply = false;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.setup.look_code)
                        .hint_text("Paste a look code")
                        .desired_width((ui.available_width() - 90.0).clamp(160.0, 320.0))
                        .min_size(vec2(0.0, 30.0))
                        .vertical_align(egui::Align::Center)
                        .font(egui::TextStyle::Monospace),
                );
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let can = !self.setup.look_code.trim().is_empty();
                apply = (ui.add_enabled(can, egui::Button::new("Try it").fill(kit::btn_fill())).clicked() || enter) && can;
            });
            if apply {
                match parse_look(&self.setup.look_code, &self.cfg.appearance) {
                    Some(a) => {
                        self.cfg.appearance = a;
                        self.setup.look_code.clear();
                        self.setup.look_msg = Some((true, "Look applied. \"Undo changes\" takes it back.".into()));
                    }
                    None => self.setup.look_msg = Some((false, "That isn't a look code. They start with PTC1.".into())),
                }
            }
            if let Some((ok, msg)) = &self.setup.look_msg {
                ui.add_space(8.0);
                kit::pill(ui, msg, if *ok { kit::PillKind::Ok } else { kit::PillKind::Bad });
            }
            ui.add_space(14.0);
            divider(ui);
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                let changed = start.as_ref().is_some_and(|s| *s != self.cfg.appearance);
                if ui
                    .add_enabled(changed, egui::Button::new("Undo changes").fill(kit::btn_fill()))
                    .on_hover_text("Back to how it looked when you opened this page")
                    .clicked()
                {
                    if let Some(s) = start.clone() {
                        self.cfg.appearance = s;
                        self.setup.look_msg = None;
                    }
                }
                let default = self.cfg.appearance == Appearance::default();
                if ui
                    .add_enabled(!default, egui::Button::new("Reset to default").fill(kit::btn_fill()))
                    .on_hover_text("Modern style, dark, Poké Red")
                    .clicked()
                {
                    self.cfg.appearance = Appearance::default();
                    self.setup.look_msg = None;
                }
            });
        });
    }

    // ───────────────────────────── preview ─────────────────────────────

    /// The preview card, floating in its column and following the page as it
    /// scrolls (it stops at the column's ends).
    fn look_sticky_preview(&mut self, ui: &mut egui::Ui, col: Rect, shown: &Hover) {
        let ctx = ui.ctx().clone();
        let view = ui.clip_rect();
        let hid = egui::Id::new("look-prev-card-h");
        let card_h = ctx.data(|d| d.get_temp::<f32>(hid)).unwrap_or(480.0);
        let style = ui.style().clone();
        let y = (view.min.y + 16.0).max(col.min.y).min((col.max.y - card_h).max(col.min.y));
        let resp = egui::Area::new(egui::Id::new("look-prev-card"))
            .order(egui::Order::Middle)
            .fixed_pos(pos2(col.min.x, y))
            .constrain(false)
            .show(&ctx, |aui| {
                aui.set_style(style);
                aui.set_clip_rect(view);
                aui.set_width(col.width());
                aui.set_max_width(col.width());
                self.look_preview_card(aui, shown);
            });
        let h = resp.response.rect.height();
        if (h - card_h).abs() > 0.5 {
            ctx.data_mut(|d| d.insert_temp(hid, h));
            ctx.request_repaint();
        }
        // the mouse wheel still scrolls the page over the card
        if resp.response.contains_pointer() {
            let d = ctx.input(|i| i.smooth_scroll_delta);
            if d != egui::Vec2::ZERO {
                ui.scroll_with_delta(d);
            }
        }
    }

    fn look_preview_card(&mut self, ui: &mut egui::Ui, shown: &Hover) {
        egui::Frame::none()
            .fill(kit::CARD())
            .stroke(Stroke::new(1.0f32, kit::LINE()))
            .rounding(24.0)
            .shadow(kit::card_shadow())
            .inner_margin(egui::Margin::same(18.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
                    kit::icon_chip(ui.painter(), r, "👁", CYAN, false);
                    ui.add_space(4.0);
                    ui.label(RichText::new("Preview").font(ch::bold(16.0)).color(TEXT()));
                    let (txt, c) = match shown {
                        Some((_, name)) => (format!("Trying: {name}"), ORANGE),
                        None => ("Your look".to_string(), GREEN),
                    };
                    let g = ui.painter().layout_no_wrap(txt, ch::bold(11.5), ch::ink(c));
                    ui.add_space((ui.available_width() - g.size().x - 26.0).max(4.0));
                    let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 26.0, 22.0), Sense::hover());
                    ui.painter().rect_filled(r, 11.0, ch::faint(c, 0.15));
                    ui.painter().circle_filled(pos2(r.min.x + 11.0, r.center().y), 3.5, c);
                    ui.painter().galley(pos2(r.min.x + 19.0, r.center().y - g.size().y / 2.0), g, ch::ink(c));
                });
                ui.add_space(12.0);
                let a = shown.as_ref().map(|(a, _)| a.clone()).unwrap_or_else(|| self.cfg.appearance.clone());
                let lk = theme::look_of(&a);
                let inner = ui.available_width();
                // over the settings (narrow windows) it's kept short
                let tall = inner < 560.0;
                let mh = if tall { (inner * 0.70).clamp(240.0, 320.0) } else { 232.0 };
                if tall {
                    let (r, _) = ui.allocate_exact_size(vec2(inner, mh), Sense::hover());
                    draw_mock_window(ui.painter(), r, &lk, a.nav_compact);
                    ui.add_space(10.0);
                    // the overlay's header follows the main colour too
                    let (r2, _) = ui.allocate_exact_size(vec2(inner, 64.0), Sense::hover());
                    draw_mock_overlay(ui.painter(), r2, &lk);
                } else {
                    let ow = (inner * 0.30).min(220.0);
                    let (row, _) = ui.allocate_exact_size(vec2(inner, mh), Sense::hover());
                    let win = Rect::from_min_size(row.min, vec2(inner - ow - 12.0, mh));
                    draw_mock_window(ui.painter(), win, &lk, a.nav_compact);
                    let ov = Rect::from_min_max(pos2(win.max.x + 12.0, row.min.y), row.max);
                    draw_mock_overlay(ui.painter(), ov, &lk);
                }
                ui.add_space(10.0);
                let mut cap = vec![
                    if a.follow_system {
                        "Match Windows".to_string()
                    } else {
                        match a.mode {
                            UiMode::Light => "Light",
                            UiMode::SemiDark => "Semi-dark",
                            UiMode::Dark => "Dark",
                        }
                        .to_string()
                    },
                ];
                cap.push(palette_name(&a).unwrap_or_else(|| "Custom colours".into()));
                ui.label(RichText::new(cap.join(" · ")).size(12.0).color(MUTED()));
                if shown.is_some() {
                    ui.label(RichText::new("Click to keep it.").size(12.0).color(ch::ink(ORANGE)));
                } else {
                    ui.label(RichText::new("Hover any mode or palette to try it here.").size(12.0).color(MUTED()));
                }
            });
    }
}


fn palette_on(a: &Appearance, acc: [u8; 3], sec: Option<[u8; 3]>) -> bool {
    !a.separate_buttons
        && a.accent == acc
        && match sec {
            Some(s) => a.gradient && a.accent2 == s,
            None => !a.gradient,
        }
}

fn apply_palette(a: &mut Appearance, acc: [u8; 3], sec: Option<[u8; 3]>) {
    a.accent = acc;
    a.gradient = sec.is_some();
    if let Some(s) = sec {
        a.accent2 = s;
    }
    a.separate_buttons = false;
}

fn palette_name(a: &Appearance) -> Option<String> {
    PALETTES.iter().find(|p| palette_on(a, p.accent, p.second)).map(|p| p.name.to_string())
}

// ───────────────────────────── look codes ─────────────────────────────

fn hex(c: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

/// "PTC1-…": everything about a look in one line.
pub(super) fn look_code(a: &Appearance) -> String {
    let mode = match a.mode {
        UiMode::Light => 0,
        UiMode::SemiDark => 1,
        UiMode::Dark => 2,
    };
    let style = 0; // Modern
    let flags = (a.gradient as u8) | (a.separate_buttons as u8) << 1 | (a.button_gradient as u8) << 2 | (a.solid_buttons as u8) << 3 | (a.follow_system as u8) << 4;
    format!("PTC1-{mode}{style}{flags:02X}-{}-{}-{}-{}", hex(a.accent), hex(a.accent2), hex(a.button), hex(a.button2))
}

/// Read a look code (keeps the parts a code doesn't carry, like animations).
pub(super) fn parse_look(code: &str, keep: &Appearance) -> Option<Appearance> {
    // (old codes can say Classic or Clean: only Modern exists now)
    parse_look_raw(code, keep).map(|mut a| {
        a.modern_ui = true;
        a.clean_settings = true;
        a
    })
}

fn parse_look_raw(code: &str, keep: &Appearance) -> Option<Appearance> {
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
    let rest = code.strip_prefix("PTC1-")?;
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() != 5 || parts[0].len() != 4 {
        return None;
    }
    let col = |s: &str| -> Option<[u8; 3]> {
        if s.len() != 6 {
            return None;
        }
        let v = u32::from_str_radix(s, 16).ok()?;
        Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
    };
    let head = parts[0].as_bytes();
    let mode = match head[0] {
        b'0' => UiMode::Light,
        b'1' => UiMode::SemiDark,
        b'2' => UiMode::Dark,
        _ => return None,
    };
    let style = head[1].wrapping_sub(b'0');
    if style > 2 {
        return None;
    }
    let flags = u8::from_str_radix(&parts[0][2..4], 16).ok()?;
    let mut a = keep.clone();
    a.mode = mode;
    let _ = style; // (Modern is the only look now)
    a.gradient = flags & 1 != 0;
    a.separate_buttons = flags & 2 != 0;
    a.button_gradient = flags & 4 != 0;
    a.solid_buttons = flags & 8 != 0;
    a.follow_system = flags & 16 != 0;
    a.accent = col(parts[1])?;
    a.accent2 = col(parts[2])?;
    a.button = col(parts[3])?;
    a.button2 = col(parts[4])?;
    Some(a)
}

// ───────────────────────────── drawing ─────────────────────────────

/// Dark or white text, whichever reads better on `c`.
fn ink_on(c: Color32) -> Color32 {
    let lin = |v: u8| {
        let c = v as f32 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    let l = 0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b());
    if (1.05 / (l + 0.05)) >= ((l + 0.05) / 0.06) { Color32::WHITE } else { Color32::from_rgb(24, 22, 34) }
}

/// A small picture of the Companion window in a look.
fn draw_mock_window(p: &egui::Painter, r: Rect, lk: &Look, compact: bool) {
    let s = &lk.s;
    let rad = if lk.modern { 14.0 } else if lk.clean { 8.0 } else { 10.0 };
    p.rect_filled(r, rad, s.page);
    // sidebar
    let sw = if compact { 44.0 } else { (r.width() * 0.26).clamp(84.0, 120.0) };
    let side = Rect::from_min_size(r.min, vec2(sw, r.height()));
    p.rect_filled(side, egui::Rounding { nw: rad, sw: rad, ne: 0.0, se: 0.0 }, s.sidebar);
    // logo
    let lc = pos2(side.min.x + 18.0, side.min.y + 20.0);
    p.circle_filled(lc, 7.0, lk.accent);
    p.circle_filled(lc, 2.5, Color32::WHITE);
    if !compact {
        p.rect_filled(Rect::from_min_size(pos2(lc.x + 13.0, lc.y - 3.0), vec2(sw - 46.0, 6.0)), 3.0, s.text);
    }
    // nav items, the second one picked
    for i in 0..6 {
        let y = side.min.y + 50.0 + i as f32 * 26.0;
        if y + 20.0 > side.max.y - 8.0 {
            break;
        }
        let item = Rect::from_min_size(pos2(side.min.x + 7.0, y), vec2(sw - 14.0, 20.0));
        let picked = i == 1;
        if picked {
            if lk.modern {
                p.rect_filled(item, 7.0, theme::blend(s.sidebar, lk.accent, if lk.light { 0.16 } else { 0.26 }));
            } else {
                p.rect_filled(item, 5.0, s.selected);
                p.rect_filled(Rect::from_min_size(item.min + vec2(0.0, 4.0), vec2(3.0, 12.0)), 1.5, lk.accent);
            }
        }
        let ic = pos2(item.min.x + 11.0, item.center().y);
        let chip_col = [lk.accent, PINK, GOLD, CYAN, GREEN, PURPLE][i];
        p.rect_filled(Rect::from_center_size(ic, vec2(11.0, 11.0)), 3.0, if lk.modern { chip_col } else { theme::blend(s.muted, s.sidebar, 0.3) });
        if !compact {
            p.rect_filled(
                Rect::from_min_size(pos2(ic.x + 11.0, ic.y - 2.5), vec2((item.width() - 36.0) * if picked { 0.8 } else { 0.6 }, 5.0)),
                2.5,
                if picked { s.text } else { s.muted },
            );
        }
    }
    // top bar: title and the Start overlay button
    let body = Rect::from_min_max(pos2(side.max.x, r.min.y), r.max);
    if !lk.clean {
        // classic: coloured header across the top
        let hb = Rect::from_min_size(body.min, vec2(body.width(), 30.0));
        p.add(theme::gradient_rect(hb, [0.0, rad, 0.0, 0.0], lk.accent, lk.accent2));
        p.rect_filled(Rect::from_min_size(hb.min + vec2(12.0, 12.0), vec2(60.0, 6.0)), 3.0, Color32::WHITE);
    } else {
        p.rect_filled(Rect::from_min_size(body.min + vec2(14.0, 12.0), vec2(56.0, 7.0)), 3.5, s.text);
    }
    let bw = 84.0;
    let btn = Rect::from_min_size(pos2(body.max.x - bw - 10.0, body.min.y + 6.0), vec2(bw, 20.0));
    p.add(theme::gradient_rect(btn, [10.0; 4], lk.button, lk.button2));
    p.text(btn.center(), Align2::CENTER_CENTER, "▶ Start overlay", ch::bold(9.5), ink_on(theme::blend(lk.button, lk.button2, 0.5)));
    // content
    let cx = body.min.x + 12.0;
    let cw = body.width() - 24.0;
    let card_r = if lk.modern { 12.0 } else if lk.clean { 7.0 } else { 9.0 };
    let card = Rect::from_min_size(pos2(cx, body.min.y + 38.0), vec2(cw, (body.height() * 0.50).max(126.0)));
    p.rect_filled(card, card_r, s.card);
    p.rect_stroke(card, card_r, Stroke::new(1.0f32, s.line));
    p.text(card.min + vec2(12.0, 16.0), Align2::LEFT_CENTER, "Shiny hunt", ch::bold(11.5), s.text);
    p.text(card.min + vec2(12.0, 42.0), Align2::LEFT_CENTER, "1,234", ch::heavy(22.0), s.text);
    p.text(card.min + vec2(12.0, 62.0), Align2::LEFT_CENTER, "encounters · 1 in 4,096", ch::reg(9.5), s.muted);
    // progress bar
    let bar = Rect::from_min_size(pos2(card.min.x + 12.0, card.min.y + 76.0), vec2(card.width() - 24.0, 6.0));
    p.rect_filled(bar, 3.0, theme::blend(s.card, s.text, 0.10));
    p.add(theme::gradient_rect(Rect::from_min_size(bar.min, vec2(bar.width() * 0.62, 6.0)), [3.0; 4], lk.accent, lk.accent2));
    // a switch and the Caught! button
    let row_y = card.max.y - 18.0;
    if row_y > bar.max.y + 14.0 {
        let tg = Rect::from_center_size(pos2(card.min.x + 28.0, row_y), vec2(30.0, 16.0));
        p.add(theme::gradient_rect(tg, [8.0; 4], lk.button, lk.button2));
        p.circle_filled(pos2(tg.max.x - 8.0, tg.center().y), 6.0, Color32::WHITE);
        p.text(pos2(tg.max.x + 8.0, row_y), Align2::LEFT_CENTER, "Auto count", ch::reg(10.0), s.text);
        let cb = Rect::from_min_size(pos2(card.max.x - 82.0, row_y - 11.0), vec2(70.0, 22.0));
        p.add(theme::gradient_rect(cb, [if lk.clean { 11.0 } else { 6.0 }; 4], lk.button, lk.button2));
        p.text(cb.center(), Align2::CENTER_CENTER, "Caught!", ch::bold(10.5), ink_on(theme::blend(lk.button, lk.button2, 0.5)));
    }
    // a segmented control under the card
    let seg_y = card.max.y + 12.0;
    if seg_y + 24.0 < r.max.y - 8.0 {
        let seg = Rect::from_min_size(pos2(cx, seg_y), vec2((cw * 0.7).min(220.0), 24.0));
        p.rect_filled(seg, 12.0, s.card);
        p.rect_stroke(seg, 12.0, Stroke::new(1.0f32, s.control));
        let third = seg.width() / 3.0;
        let pick = Rect::from_min_size(seg.min + vec2(2.0, 2.0), vec2(third - 2.0, 20.0));
        if lk.modern {
            p.add(theme::gradient_rect(pick, [10.0; 4], lk.button, lk.button2));
        } else {
            p.rect_filled(pick, 10.0, s.selected);
        }
        let ink = if lk.modern { ink_on(theme::blend(lk.button, lk.button2, 0.5)) } else { s.text };
        for (k, t) in ["Day", "Week", "All"].iter().enumerate() {
            let c = pos2(seg.min.x + third * (k as f32 + 0.5), seg.center().y);
            p.text(c, Align2::CENTER_CENTER, *t, ch::bold(9.5), if k == 0 { ink } else { s.muted });
        }
        // two quiet lines of text
        let ty = seg.max.y + 14.0;
        if ty + 16.0 < r.max.y {
            p.rect_filled(Rect::from_min_size(pos2(cx, ty), vec2(cw * 0.8, 6.0)), 3.0, theme::blend(s.page, s.muted, 0.6));
            p.rect_filled(Rect::from_min_size(pos2(cx, ty + 12.0), vec2(cw * 0.5, 6.0)), 3.0, theme::blend(s.page, s.muted, 0.45));
        }
    }
    p.rect_stroke(r, rad, Stroke::new(1.0f32, theme::blend(s.line, s.text, 0.08)));
}

/// The overlay's header strip in a look (sitting over a bit of "game").
fn draw_mock_overlay(p: &egui::Painter, r: Rect, lk: &Look) {
    p.add(theme::gradient_rect(r, [12.0; 4], Color32::from_rgb(84, 138, 96), Color32::from_rgb(118, 170, 112)));
    let ov = Rect::from_center_size(r.center(), vec2((r.width() * 0.62).min(260.0).max((r.width() - 24.0).min(150.0)), (r.height() - 16.0).min(56.0)));
    p.rect_filled(ov, 10.0, Color32::from_rgba_unmultiplied(20, 19, 30, 235));
    let hb = Rect::from_min_size(ov.min, vec2(ov.width(), 18.0));
    p.add(theme::gradient_rect(hb, [10.0, 10.0, 0.0, 0.0], lk.accent, lk.accent2));
    p.text(hb.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, "SHINY HUNT", ch::bold(9.0), ink_on(lk.accent));
    p.text(pos2(ov.center().x, ov.max.y - 13.0), Align2::CENTER_CENTER, "1,234", ch::heavy(15.0), Color32::WHITE);
    p.text(r.max - vec2(8.0, 8.0), Align2::RIGHT_BOTTOM, "overlay", ch::reg(9.5), Color32::from_white_alpha(200));
}

/// The "Match Windows" mode tile: half light, half dark.
fn system_tile(ui: &mut egui::Ui, w: f32, selected: bool, a: &Appearance) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(w, 96.0), Sense::click());
    let p = ui.painter_at(r.expand(2.0));
    let win = Rect::from_min_size(r.min + vec2(10.0, 10.0), vec2(r.width() - 20.0, 52.0));
    let half = win.width() / 2.0;
    let light = Rect::from_min_size(win.min, vec2(half, win.height()));
    let dark = Rect::from_min_max(pos2(win.min.x + half, win.min.y), win.max);
    p.rect_filled(light, egui::Rounding { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 }, Color32::from_rgb(246, 244, 251));
    p.rect_filled(dark, egui::Rounding { ne: 6.0, se: 6.0, nw: 0.0, sw: 0.0 }, Color32::from_rgb(20, 19, 30));
    let c3 = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
    let hb = Rect::from_min_size(win.min, vec2(win.width(), 12.0));
    p.add(theme::gradient_rect(hb, [6.0, 6.0, 0.0, 0.0], c3(a.accent), if a.gradient { c3(a.accent2) } else { c3(a.accent) }));
    p.rect_filled(Rect::from_min_size(win.min + vec2(8.0, 20.0), vec2(half - 14.0, 6.0)), 3.0, Color32::from_rgb(40, 38, 56));
    p.rect_filled(Rect::from_min_size(pos2(dark.min.x + 6.0, win.min.y + 20.0), vec2(half - 14.0, 6.0)), 3.0, Color32::from_rgb(236, 234, 245));
    p.rect_stroke(win, 6.0, Stroke::new(1.0f32, Color32::from_rgb(92, 89, 120)));
    p.text(pos2(r.center().x, r.max.y - 16.0), Align2::CENTER_CENTER, "Match Windows", kit::font(12.5), if selected { TEXT() } else { MUTED() });
    p.rect_stroke(r, kit::r(10.0), if selected { Stroke::new(2.5f32, kit::ACCENT()) } else { Stroke::new(1.0f32, kit::LINE()) });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A palette as a tile: its colours as a stripe, a button in them, its name.
fn palette_tile(ui: &mut egui::Ui, w: f32, name: &str, acc: [u8; 3], sec: Option<[u8; 3]>, on: bool, from_hunt: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(w, 74.0), Sense::click());
    let p = ui.painter_at(r.expand(4.0));
    let t = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() || on, kit::QUICK);
    let fill = if on { kit::SELECTED_BG() } else { theme::blend(kit::CARD(), kit::CARD_HOVER(), t) };
    p.rect_filled(r, 12.0, fill);
    let a = Color32::from_rgb(acc[0], acc[1], acc[2]);
    let b = sec.map(|s| Color32::from_rgb(s[0], s[1], s[2])).unwrap_or(a);
    let stripe = Rect::from_min_size(r.min + vec2(8.0, 8.0), vec2(w - 16.0, 30.0));
    p.add(theme::gradient_rect(stripe, [8.0; 4], a, b));
    if from_hunt {
        theme::sparkle(&p, stripe.right_top() + vec2(-11.0, 11.0), 5.0, Color32::WHITE);
    }
    if on {
        let c = stripe.center();
        p.circle_filled(c, 9.0, Color32::WHITE);
        p.add(egui::Shape::line(vec![c + vec2(-4.0, 0.0), c + vec2(-1.0, 3.0), c + vec2(4.5, -3.5)], Stroke::new(2.0f32, a)));
    }
    let g = p.layout(name.to_string(), ch::bold(11.5), TEXT(), w - 16.0);
    let mut galley_pos = pos2(r.min.x + 9.0, stripe.max.y + 8.0);
    if g.rows.len() > 1 {
        galley_pos.y -= 3.0;
    }
    p.galley(galley_pos, g, TEXT());
    let stroke = if on { Stroke::new(2.0f32, kit::ACCENT()) } else { Stroke::new(1.0f32, kit::LINE()) };
    p.rect_stroke(r, 12.0, stroke);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn card_head(ui: &mut egui::Ui, icon: &str, col: Color32, title: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        kit::icon_chip(ui.painter(), r, icon, col, false);
        ui.add_space(4.0);
        ui.label(RichText::new(title).font(ch::bold(16.0)).color(TEXT()));
    });
}

fn label(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t).font(ch::bold(10.5)).color(MUTED()));
}

fn divider(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, kit::LINE()));
}

/// Label and help on the left, the control flush right (laid out left to
/// right, with the control's width measured the frame before).
pub(super) fn setting_row(ui: &mut egui::Ui, label: &str, help: &str, control: impl FnOnce(&mut egui::Ui)) {
    let id = ui.id().with(("look-setting-row-w", label));
    let cw = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(160.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(TEXT()));
        if !help.is_empty() {
            kit::info(ui, help);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_code_round_trip() {
        let mut a = Appearance::default();
        a.gradient = true;
        a.accent2 = [1, 2, 3];
        a.separate_buttons = true;
        a.button = [250, 0, 128];
        a.mode = UiMode::SemiDark;
        let code = look_code(&a);
        assert_eq!(parse_look(&code, &Appearance::default()), Some(a.clone()));
        assert_eq!(parse_look(&code.to_lowercase(), &Appearance::default()), Some(a));
        assert!(parse_look("hello", &Appearance::default()).is_none());
    }
}
