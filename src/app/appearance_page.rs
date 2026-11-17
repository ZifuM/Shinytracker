//! The Appearance page: style, light or dark, colours and palettes, with a
//! live preview that follows you down the page and tries a choice while you
//! hover it. Looks can be shared as a short code.

use super::charts::{self as ch, CYAN, GOLD, GREEN, ORANGE, PINK, PURPLE};
use super::hunt_pages::{mode_tile, swatch_row};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::App;
use crate::config::{Appearance, PlusTheme, UiMode};
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
        self.look_plus_card(ui);
        if self.cfg.appearance.plus {
            ui.add_space(18.0);
            self.look_theme_card(ui, hover);
        }
        // a theme brings its own mode and colours: those cards rest meanwhile
        let themed = themed(&self.cfg.appearance);
        ui.add_space(18.0);
        ui.add_enabled_ui(!themed, |ui| self.look_style_card(ui, &mut *hover));
        ui.add_space(18.0);
        // (with Use my colours on, the Colours card colours the theme)
        let own = themed && (self.cfg.appearance.theme_colours || theme::theme_def(self.cfg.appearance.plus_theme).is_some_and(|d| d.own_accent));
        ui.add_enabled_ui(!themed || own, |ui| self.look_colours_card(ui, &mut *hover));
        ui.add_space(18.0);
        self.look_comfort_card(ui);
        ui.add_space(18.0);
        self.look_share_card(ui);
    }

    // ───────────────────────────── look ─────────────────────────────

    /// Modern or Modern Plus. Modern stays exactly as it was.
    fn look_plus_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "🎨", PURPLE, "Look");
            ui.add_space(10.0);
            let mut plus = self.cfg.appearance.plus;
            if kit::segmented(ui, &mut plus, &[(true, "Modern Plus (recommended)"), (false, "Modern (not recommended)")]) {
                self.cfg.appearance.plus = plus;
                // Modern Plus lives on the bottom dock: move the pins there,
                // and put them back where they were when it's turned off
                use crate::apps::PinsPlace;
                if plus {
                    if self.cfg.apps.pins_place != PinsPlace::Dock {
                        self.cfg.apps.place_before_plus = Some(self.cfg.apps.pins_place);
                        self.cfg.apps.pins_place = PinsPlace::Dock;
                    }
                } else if let Some(p) = self.cfg.apps.place_before_plus.take() {
                    self.cfg.apps.pins_place = p;
                }
                self.dirty = true;
            }
            ui.add_space(8.0);
            let text = if plus {
                "Modern Plus: your apps live on a dock at the bottom, the pages become tabs in the title bar and your hunt rides on the dock, which tucks away until you need it. Every main button is one solid colour (yours, from Colours below), and the Library, Pokédex, Rare Grass and Card Collector get tidier layouts. Switch back to Modern any time; nothing else changes."
            } else {
                "Modern: the older look, with a sidebar. Not recommended: new things are made for Modern Plus first. You can switch back any time."
            };
            ui.label(egui::RichText::new(text).size(kit::fs(12.0)).color(MUTED()));
        });
    }

    // ───────────────────────────── theme ─────────────────────────────

    /// Modern Plus's themes: "Your colours" (your own mode and colours) or a
    /// ready-made look from the UI Lab. Hover one to try it in the preview.
    fn look_theme_card(&mut self, ui: &mut egui::Ui, hover: &mut Hover) {
        kit::card(ui, |ui| {
            card_head(ui, "🖌", ORANGE, "Theme");
            ui.add_space(14.0);
            let a = self.cfg.appearance.clone();
            let avail = ui.available_width();
            let cols = ((avail + 10.0) / (150.0 + 10.0)).floor().clamp(2.0, 4.0);
            let tw = ((avail - 10.0 * (cols - 1.0)) / cols).floor();
            let mut pick: Option<PlusTheme> = None;
            // ready-made looks first, then the ones that match an overlay theme
            let ready: Vec<PlusTheme> = theme::PLUS_THEMES.into_iter().filter(|t| !theme::OVERLAY_MATCH.contains(t)).collect();
            for (group, list) in [(None, ready), (Some("MATCH YOUR OVERLAY"), theme::OVERLAY_MATCH.to_vec())] {
            if let Some(g) = group {
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    label(ui, g);
                    kit::info(ui, "App themes made to match the overlay themes: Sticker, Glass HUD, Retro DS, Field journal, Broadcast and Terminal.");
                });
                ui.add_space(8.0);
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                for t in list {
                    let on = a.plus_theme == t;
                    let mut c = a.clone();
                    c.plus_theme = t;
                    let lk = theme::look_of(&c);
                    let name = theme_name(t);
                    let r = theme_tile(ui, tw, name, &lk, theme::theme_def(t), on);
                    // ▶ on a tile: watch that theme's startup animation
                    tile_play(ui, &r, &c);
                    if r.hovered() && !on {
                        *hover = Some((c, format!("{name} theme")));
                    }
                    let tip = match theme::theme_def(t) {
                        Some(d) => d.blurb.to_string(),
                        None => "Your own mode and colours, from the cards below.".to_string(),
                    };
                    if r.on_hover_text(tip).clicked() {
                        pick = Some(t);
                    }
                }
            });
            }
            // the one that matches your overlay, a click away
            if let Some(m) = theme::theme_for_overlay(self.cfg.overlay.theme) {
                if a.plus_theme != m && self.cfg.overlay.style != crate::config::OverlayStyle::Legacy {
                    ui.add_space(10.0);
                    let text = format!("Match my overlay ({})", theme_name(m));
                    if ui.add(egui::Button::new(text).fill(kit::btn_fill())).on_hover_text("Use the app theme made for your overlay theme").clicked() {
                        pick = Some(m);
                    }
                }
            }
            if let Some(t) = pick {
                self.cfg.appearance.plus_theme = t;
            }
            self.theme_options(ui, false);
            ui.add_space(8.0);
            let note = if themed(&self.cfg.appearance) {
                if self.cfg.appearance.theme_colours { "This theme takes your colours from Colours below, in its Light or Dark above. Mode below rests while a theme is on." } else { "This theme brings its own colours (in its Light or Dark above), so Mode and Colours below rest until you pick Your colours again, or turn on Use my colours." }
            } else {
                "Your colours: your own mode and colours, set below. Or pick a theme for a ready-made look."
            };
            ui.label(RichText::new(note).size(kit::fs(12.0)).color(MUTED()));
        });
    }

    /// The theme's options: its light or dark, your colours through it, its
    /// background, transitions, startup animation and sounds (with previews).
    /// On the Theme card, and in the welcome tour's Look step.
    pub(super) fn theme_options(&mut self, ui: &mut egui::Ui, with_sounds: bool) {
        // every theme comes light and dark: its own mode, or flipped
        if let Some(d) = theme::theme_def(self.cfg.appearance.plus_theme) {
            ui.add_space(10.0);
            let own = if d.light { "Light" } else { "Dark" };
            let other = if d.light { "Dark" } else { "Light" };
            let mut flip = self.cfg.appearance.theme_flip;
            setting_row(ui, "Mode", "Each theme comes in its own mode, or the other one (made from its colours).", |ui| {
                kit::segmented(ui, &mut flip, &[(false, own), (true, other)]);
            });
            self.cfg.appearance.theme_flip = flip;
        }
        // any theme in your own colours
        if let Some(d) = theme::theme_def(self.cfg.appearance.plus_theme).filter(|d| !d.own_accent) {
            ui.add_space(10.0);
            let help = format!("{} in the colours you pick in Colours below: it keeps its style, light and dark, and takes your colour. Off: the theme's own colours.", d.name);
            if kit::toggle_row(ui, "Use my colours", &help, &mut self.cfg.appearance.theme_colours) {
                self.dirty = true;
            }
        }
        // Winter's snow, Spooky's embers and the other moving backgrounds can be turned off
        if theme::theme_def(self.cfg.appearance.plus_theme).is_some_and(|d| d.particles.is_some() || theme::has_ambient(d.skin)) {
            ui.add_space(10.0);
            kit::toggle_row(ui, "Background effects", "The theme's moving background: snow, embers, glints, drifting glows, bubbles or a rolling grid. Off for a still one.", &mut self.cfg.appearance.particles);
        }
        // each theme's own way of bringing a page in
        if theme::theme_def(self.cfg.appearance.plus_theme).is_some() {
            ui.add_space(10.0);
            kit::toggle_row(ui, "Page transitions", "Pages come in the theme's own way: a page ripped out (Field journal), pixels breaking up (pixel themes), a news wipe (Broadcast), a ripple (Liquid), bubbles popping (Companion)... Off (the default): a plain fade.", &mut self.cfg.appearance.theme_transitions);
        }
        // every theme (Your colours too) opens the app with its own little show
        {
            ui.add_space(10.0);
            kit::toggle_row(ui, "Startup animation", "The theme's own intro as the app opens (a Pokédex opening, a Game Boy boot, a sunrise over a neon grid...). It plays its own sound even with Theme sounds off. A click skips it.", &mut self.cfg.appearance.theme_intro);
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                if ui.button("▶  Play it now").on_hover_text("Or the ▶ on any theme's tile, to watch that one's").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    theme::replay_intro();
                }
            });
        }
        // every theme has its own sounds (also in Settings → Sounds)
        if with_sounds {
            self.theme_sounds_block(ui);
        } else if self.cfg.appearance.plus {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(if self.cfg.appearance.theme_sounds { "Theme sounds: on" } else { "Theme sounds: off" }).size(kit::fs(13.0)).color(TEXT()));
                if ui.button("🔊  Sounds settings").on_hover_text("The theme's sounds, their volume and previews, with every other sound").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.open_settings(super::Tab::Sounds);
                }
            });
        }
    }

    /// The theme's sounds: on or off, their volume, and a ▶ for each.
    pub(super) fn theme_sounds_block(&mut self, ui: &mut egui::Ui) {
        {
            ui.add_space(10.0);
            let was = self.cfg.appearance.theme_sounds;
            kit::toggle_row(ui, "Theme sounds", "Little sounds for clicks, hovers and pages, made to match the theme (8-bit blips, pencil ticks, coins, ice chimes...). Off unless you turn them on; they play on the output picked in Sounds.", &mut self.cfg.appearance.theme_sounds);
            if self.cfg.appearance.theme_sounds && !was {
                // (a taste of the pack as it's turned on)
                crate::sound::play_ui(theme::sound_pack(), crate::sound::UiSnd::Page, self.cfg.appearance.theme_volume, &self.cfg.sounds.device);
            }
            // (the volume shows either way: the startup animation's sound uses it too)
            {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Volume").size(kit::fs(13.0)).color(TEXT()));
                    ui.spacing_mut().slider_width = (ui.available_width() - kit::SLIDER_VALUE_W - 16.0).max(80.0);
                    let text = format!("{:.0}%", self.cfg.appearance.theme_volume * 100.0);
                    let r = kit::slider_f32(ui, &mut self.cfg.appearance.theme_volume, 0.0..=1.0, &text);
                    // (a sample when you let go, to hear the new level)
                    if r.drag_stopped() || (r.changed() && !r.dragged()) {
                        crate::sound::play_ui(theme::sound_pack(), crate::sound::UiSnd::Click, self.cfg.appearance.theme_volume, &self.cfg.sounds.device);
                    }
                });
                ui.add_space(6.0);
                // hear each of the pack's sounds
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Hear it").size(kit::fs(13.0)).color(TEXT()));
                    for (k, label) in [(crate::sound::UiSnd::Click, "▶ Click"), (crate::sound::UiSnd::Hover, "▶ Hover"), (crate::sound::UiSnd::Page, "▶ New page")] {
                        if ui.button(label).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            crate::sound::play_ui(theme::sound_pack(), k, self.cfg.appearance.theme_volume, &self.cfg.sounds.device);
                        }
                    }
                });
            }
        }
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
            // (Modern Plus has no sidebar: tabs in the title bar and the dock)
            if !a.plus {
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
            ui.add_space(10.0);
            self.dirty |= kit::toggle_row(
                ui,
                "Your party in the title bar",
                "Pictures of your party next to the overlay's status: your party when the game opened (the game writes it then), kept up to date from the game's log. A \"?\" is one the log couldn't name; click a slot to set it.",
                &mut self.cfg.show_party,
            );            ui.add_space(10.0);
            divider(ui);
            // (Modern Plus keeps pins on the dock: nothing to pick there)
            if !self.cfg.appearance.plus {
                ui.add_space(12.0);
                self.pins_place_picker(ui);
                ui.add_space(10.0);
                divider(ui);
            }
            ui.add_space(10.0);
            let mut show = !self.cfg.apps.tours_off;
            if kit::toggle_row(
                ui,
                "Show tours",
                "A short spotlight tour the first time you open the Companion, each app and each of its pages. Off: no tours at all (\"Don't show any more tours\" on a tour turns this off).",
                &mut show,
            ) {
                self.cfg.apps.tours_off = !show;
                self.dirty = true;
            }
            ui.add_space(6.0);
            let mut again = false;
            setting_row(ui, "App tours", "Show every tour again, as if the Companion and every app were new. This turns tours back on too.", |ui| {
                again = ui.add(egui::Button::new("Show app tours again").fill(kit::btn_fill())).on_hover_text("Each app's tour shows the next time you open it").clicked();
            });
            if again {
                self.tours_reset();
            }
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
                // Modern Plus draws its own layout, in the theme's colours
                let def = if themed(&a) { theme::theme_def(a.plus_theme) } else { None };
                let mock = |p: &egui::Painter, r: Rect| {
                    if a.plus {
                        draw_mock_plus(p, r, &lk, def);
                    } else {
                        draw_mock_window(p, r, &lk, a.nav_compact);
                    }
                };
                let inner = ui.available_width();
                // over the settings (narrow windows) it's kept short
                let tall = inner < 560.0;
                let mh = if tall { (inner * 0.70).clamp(240.0, 320.0) } else { 232.0 };
                if tall {
                    let (r, _) = ui.allocate_exact_size(vec2(inner, mh), Sense::hover());
                    mock(ui.painter(), r);
                    ui.add_space(10.0);
                    // the overlay's header follows the main colour too
                    let (r2, _) = ui.allocate_exact_size(vec2(inner, 64.0), Sense::hover());
                    draw_mock_overlay(ui.painter(), r2, &lk);
                } else {
                    let ow = (inner * 0.30).min(220.0);
                    let (row, _) = ui.allocate_exact_size(vec2(inner, mh), Sense::hover());
                    let win = Rect::from_min_size(row.min, vec2(inner - ow - 12.0, mh));
                    mock(ui.painter(), win);
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
                if themed(&a) {
                    cap = vec![format!("{} theme", theme_name(a.plus_theme))];
                }
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

/// Is a Modern Plus theme setting the mode and colours?
pub(super) fn themed(a: &Appearance) -> bool {
    a.plus && a.plus_theme != PlusTheme::Yours && theme::theme_ready(a.plus_theme)
}

pub(super) fn theme_name(t: PlusTheme) -> &'static str {
    theme::theme_def(t).map(|d| d.name).unwrap_or("Your colours")
}

/// A theme's number in look codes.
fn theme_num(t: PlusTheme) -> u8 {
    match t {
        PlusTheme::Yours => 0,
        PlusTheme::Studio => 1,
        PlusTheme::Pokedex => 2,
        PlusTheme::Aurora => 3,
        PlusTheme::Field => 4,
        PlusTheme::Synthwave => 5,
        PlusTheme::Gold => 6,
        PlusTheme::Companion => 7,
        PlusTheme::Pocket => 8,
        PlusTheme::Candy => 9,
        PlusTheme::Brutal => 10,
        PlusTheme::Liquid => 11,
        PlusTheme::Winter => 12,
        PlusTheme::Spooky => 13,
        PlusTheme::Sticker => 14,
        PlusTheme::GlassHud => 15,
        PlusTheme::RetroDs => 16,
        PlusTheme::Journal => 17,
        PlusTheme::Broadcast => 18,
        PlusTheme::Terminal => 19,
    }
}

fn theme_of_num(n: u8) -> Option<PlusTheme> {
    theme::PLUS_THEMES.into_iter().find(|t| theme_num(*t) == n)
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
    let code = format!("PTC1-{mode}{style}{flags:02X}-{}-{}-{}-{}", hex(a.accent), hex(a.accent2), hex(a.button), hex(a.button2));
    // (a Modern Plus theme rides on the end: "-T1")
    if a.plus_theme == PlusTheme::Yours {
        code
    } else {
        // (an F: in the other mode)
        format!("{code}-T{}{}{}", theme_num(a.plus_theme), if a.theme_flip { "F" } else { "" }, if a.theme_colours { "C" } else { "" })
    }
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
    if !(parts.len() == 5 || parts.len() == 6) || parts[0].len() != 4 {
        return None;
    }
    let mut colours = false;
    let (th, flip) = match parts.get(5) {
        Some(t) => {
            let t = t.strip_prefix('T')?;
            let (t, mine) = match t.strip_suffix('C') {
                Some(n) => (n, true),
                None => (t, false),
            };
            colours = mine;
            let (num, flip) = match t.strip_suffix('F') {
                Some(n) => (n, true),
                None => (t, false),
            };
            (theme_of_num(num.parse().ok()?)?, flip)
        }
        None => (PlusTheme::Yours, false),
    };
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
    a.plus_theme = th;
    a.theme_flip = flip;
    a.theme_colours = colours;
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

/// A small picture of the Companion in Modern Plus: tabs in the title bar
/// (with the back button), cards, and the dock with your hunt, all in the
/// theme's colours (its painted title bar, glows, sky and glass too).
fn draw_mock_plus(p: &egui::Painter, r: Rect, lk: &Look, def: Option<theme::ThemeDef>) {
    let s = &lk.s;
    let rad = 8.0;
    p.rect_filled(r, rad, s.page);
    let inner = p.with_clip_rect(r.shrink(2.5));
    if let Some((top, bottom)) = def.and_then(|d| d.backdrop) {
        theme::paint_backdrop_colours(&inner, r, top, bottom);
    }
    if let Some((a, b)) = def.and_then(|d| d.glow) {
        let big = r.width().max(r.height()) * 0.62;
        theme::glow_spot(&inner, r.min + vec2(r.width() * 0.15, r.height() * 0.10), big, a, 0.20);
        theme::glow_spot(&inner, r.min + vec2(r.width() * 0.85, r.height() * 0.30), big, b, 0.24);
    }
    let glass = def.is_some_and(|d| d.glass);
    let (gf, gr) = theme::glass_colours(lk.light);
    let card_fill = if glass { gf } else { s.card };
    let card_line = if glass { gr } else { s.line };
    let hard = def.and_then(|d| d.hard_shadow);
    let pixel = def.is_some_and(|d| d.pixel);
    let cr = 10.0 * def.map_or(1.0, |d| d.round);
    // title bar
    let bar = Rect::from_min_size(r.min, vec2(r.width(), 28.0));
    let chrome = def.and_then(|d| d.chrome);
    let (hf, hi) = match chrome {
        Some((c, ink)) => (c, ink),
        None if def.is_some() => (s.sidebar, s.text),
        None => (s.page, s.text),
    };
    p.rect_filled(bar, egui::Rounding { nw: rad, ne: rad, sw: 0.0, se: 0.0 }, hf);
    if chrome.is_none() {
        p.rect_filled(Rect::from_min_size(bar.min + vec2(rad, 0.0), vec2(bar.width() - rad * 2.0, 1.5)), 0.0, lk.accent);
    }
    p.line_segment([bar.left_bottom(), bar.right_bottom()], Stroke::new(1.0f32, s.line));
    p.circle_filled(pos2(bar.min.x + 14.0, bar.center().y), 5.5, lk.accent);
    // the back button and the tabs
    let bc = pos2(bar.min.x + 34.0, bar.center().y);
    p.add(egui::Shape::line(vec![bc + vec2(2.0, -4.0), bc + vec2(-2.0, 0.0), bc + vec2(2.0, 4.0)], Stroke::new(1.5f32, theme::blend(hf, hi, 0.7))));
    let mut x = bar.min.x + 46.0;
    for (k, w) in [26.0, 30.0, 24.0, 22.0].into_iter().enumerate() {
        if x + w > bar.max.x - 70.0 {
            break;
        }
        let ink = if k == 0 { hi } else { theme::blend(hf, hi, 0.6) };
        p.rect_filled(Rect::from_min_size(pos2(x, bar.center().y - 2.0), vec2(w, 4.0)), 2.0, ink);
        if k == 0 {
            let under = match (chrome, def) {
                (Some(_), Some(d)) => d.accent2,
                _ => lk.button,
            };
            p.rect_filled(Rect::from_min_size(pos2(x, bar.max.y - 3.0), vec2(w, 2.0)), 1.0, under);
        }
        x += w + 10.0;
    }
    // search
    let sr = Rect::from_min_size(pos2(bar.max.x - 64.0, bar.center().y - 7.0), vec2(54.0, 14.0));
    p.rect_filled(sr, 7.0, theme::blend(hf, hi, 0.10));
    // the hunt card: a big number, a bar and Caught!
    let body = Rect::from_min_max(pos2(r.min.x + 12.0, bar.max.y + 10.0), pos2(r.max.x - 12.0, r.max.y - 40.0));
    let hero = Rect::from_min_size(body.min, vec2(body.width(), (body.height() * 0.58).max(60.0)));
    if let Some(sh) = hard {
        p.rect_filled(hero.translate(vec2(4.0, 4.0)), cr, sh);
    } else if !glass {
        p.rect_filled(hero.translate(vec2(0.0, 3.0)), cr, Color32::from_black_alpha(if lk.light { 10 } else { 40 }));
    }
    p.rect_filled(hero, cr, card_fill);
    p.rect_stroke(hero, cr, Stroke::new(1.0f32, card_line));
    p.text(hero.min + vec2(12.0, 14.0), Align2::LEFT_CENTER, "SHINY HUNT", ch::bold(9.0), s.muted);
    p.text(hero.min + vec2(12.0, 36.0), Align2::LEFT_CENTER, "1,234", ch::heavy(22.0), s.text);
    if hero.height() > 64.0 {
        let pb = Rect::from_min_size(pos2(hero.min.x + 12.0, hero.max.y - 14.0), vec2(hero.width() * 0.55, 5.0));
        p.rect_filled(pb, 2.5, theme::blend(s.card, s.text, 0.12));
        p.rect_filled(Rect::from_min_size(pb.min, vec2(pb.width() * 0.62, 5.0)), 2.5, lk.accent);
    }
    let cb = Rect::from_min_size(pos2(hero.max.x - 76.0, hero.center().y - 11.0), vec2(64.0, 22.0));
    if pixel {
        p.add(theme::pixel_key(cb, lk.button, 0.0, 0.0));
    } else {
        p.rect_filled(cb, 11.0 * def.map_or(1.0, |d| d.round), lk.button);
    }
    p.text(cb.center(), Align2::CENTER_CENTER, "Caught!", ch::bold(10.0), ink_on(lk.button));
    // two small cards
    let row = Rect::from_min_max(pos2(body.min.x, hero.max.y + 8.0), body.max);
    if row.height() > 16.0 {
        let half = (row.width() - 8.0) / 2.0;
        for k in 0..2 {
            let c = Rect::from_min_size(pos2(row.min.x + k as f32 * (half + 8.0), row.min.y), vec2(half, row.height()));
            if let Some(sh) = hard {
                p.rect_filled(c.translate(vec2(3.0, 3.0)), cr, sh);
            }
            p.rect_filled(c, cr, card_fill);
            p.rect_stroke(c, cr, Stroke::new(1.0f32, card_line));
            p.rect_filled(Rect::from_min_size(c.min + vec2(10.0, 10.0), vec2(half * 0.4, 5.0)), 2.5, s.text);
            if c.height() > 26.0 {
                p.rect_filled(Rect::from_min_size(c.min + vec2(10.0, 20.0), vec2(half * 0.6, 4.0)), 2.0, s.muted);
            }
        }
    }
    // the dock: your hunt with Start overlay, then your apps
    let dw = (r.width() * 0.62).clamp(150.0, 300.0);
    let dock = Rect::from_center_size(pos2(r.center().x, r.max.y - 18.0), vec2(dw, 26.0));
    p.rect_filled(dock, 13.0 * def.map_or(1.0, |d| d.round), if glass { gf } else { theme::blend(s.card, s.text, 0.06) });
    p.rect_stroke(dock, 13.0, Stroke::new(1.0f32, card_line));
    let cap = Rect::from_min_size(dock.min + vec2(4.0, 4.0), vec2(dw * 0.38, 18.0));
    p.rect_filled(cap, 9.0, theme::blend(s.card, s.text, 0.10));
    let go = Rect::from_min_size(pos2(cap.max.x - 30.0, cap.min.y + 2.0), vec2(28.0, 14.0));
    p.rect_filled(go, 7.0, lk.button);
    let cols = [lk.accent, PINK, GOLD, CYAN, GREEN];
    let step = (dock.max.x - cap.max.x - 8.0) / cols.len() as f32;
    for (k, c) in cols.into_iter().enumerate() {
        let cx = cap.max.x + 4.0 + step * (k as f32 + 0.5);
        let ir = Rect::from_center_size(pos2(cx, dock.center().y), vec2(16.0, 16.0));
        if pixel {
            // the pixel icons (Shiny Hunter, Pokédex, Card Collector, Stats, Boxes)
            let icon = ["✨", "📖", "🃏", "📊", "📦"][k];
            if let Some(bits) = theme::pixel_icon(icon) {
                theme::pixel_chip(p, ir, bits, c, Color32::WHITE);
            }
        } else {
            p.rect_filled(ir.shrink(1.0), 4.0, c);
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

/// A theme as a tile: a little window in its colours, and its name.
/// The ▶ on a theme tile (shown on hover): plays that theme's startup
/// animation, in the light or dark and colours `a` gives it.
pub(super) fn tile_play(ui: &mut egui::Ui, r: &egui::Response, a: &Appearance) {
    let pr = Rect::from_center_size(pos2(r.rect.max.x - 20.0, r.rect.min.y + 20.0), vec2(24.0, 24.0));
    let pb = ui.interact(pr, r.id.with("intro"), Sense::click());
    if r.hovered() || pb.hovered() {
        let hot = pb.hovered();
        let p = ui.painter();
        p.circle_filled(pr.center(), 12.0, if hot { kit::primary() } else { Color32::from_black_alpha(150) });
        let c = pr.center() + vec2(1.5, 0.0);
        p.add(egui::Shape::convex_polygon(vec![c + vec2(-4.0, -5.5), c + vec2(5.5, 0.0), c + vec2(-4.0, 5.5)], Color32::WHITE, Stroke::NONE));
    }
    if pb.on_hover_text("Play this theme's startup animation").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        theme::preview_intro(a);
    }
}

pub(super) fn theme_tile(ui: &mut egui::Ui, w: f32, name: &str, lk: &Look, def: Option<theme::ThemeDef>, on: bool) -> egui::Response {
    let chrome = def.and_then(|d| d.chrome);
    let glow = def.and_then(|d| d.glow);
    let glass = def.is_some_and(|d| d.glass);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 100.0), Sense::click());
    let p = ui.painter_at(r.expand(4.0));
    let t = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() || on, kit::QUICK);
    let fill = if on { kit::SELECTED_BG() } else { theme::blend(kit::CARD(), kit::CARD_HOVER(), t) };
    p.rect_filled(r, 12.0, fill);
    let s = &lk.s;
    let win = Rect::from_min_size(r.min + vec2(8.0, 8.0), vec2(w - 16.0, 58.0));
    p.rect_filled(win, 8.0, s.page);
    // its sky (Synthwave)
    if let Some((top, bottom)) = def.and_then(|d| d.backdrop) {
        // (rounded ends in the end colours, the wash in between)
        let k = 8.0;
        p.rect_filled(Rect::from_min_size(win.min, vec2(win.width(), k + 1.0)), egui::Rounding { nw: k, ne: k, sw: 0.0, se: 0.0 }, top);
        p.rect_filled(Rect::from_min_max(pos2(win.min.x, win.max.y - k - 1.0), win.max), egui::Rounding { nw: 0.0, ne: 0.0, sw: k, se: k }, bottom);
        theme::paint_backdrop_colours(&p, win.shrink2(vec2(0.0, k)), top, bottom);
    }
    // its glows (Aurora glass)
    if let Some((a, b)) = glow {
        let gp = p.with_clip_rect(win.shrink(1.0));
        theme::glow_spot(&gp, win.min + vec2(win.width() * 0.15, 6.0), win.width() * 0.7, a, 0.30);
        theme::glow_spot(&gp, win.min + vec2(win.width() * 0.85, 18.0), win.width() * 0.7, b, 0.34);
    }
    // title bar: the theme's own colour, or the page's with a hairline
    let bar = Rect::from_min_size(win.min, vec2(win.width(), 13.0));
    match chrome {
        Some((c, ink)) => {
            p.rect_filled(bar, egui::Rounding { nw: 8.0, ne: 8.0, sw: 0.0, se: 0.0 }, c);
            p.rect_filled(Rect::from_min_size(bar.min + vec2(8.0, 5.0), vec2(22.0, 3.0)), 1.5, ink);
        }
        None => {
            p.rect_filled(Rect::from_min_size(bar.min + vec2(5.0, 0.0), vec2(bar.width() - 10.0, 1.5)), 0.0, lk.accent);
            p.rect_filled(Rect::from_min_size(bar.min + vec2(8.0, 5.0), vec2(22.0, 3.0)), 1.5, s.text);
        }
    }
    // a card with a number, a line of text and a button
    let card = Rect::from_min_max(win.min + vec2(7.0, 18.0), win.max - vec2(7.0, 8.0));
    let cr = 5.0 * def.map_or(1.0, |d| d.round);
    if let Some(sh) = def.and_then(|d| d.hard_shadow) {
        p.rect_filled(card.translate(vec2(2.5, 2.5)), cr, sh);
    }
    if glass {
        let (gf, gr) = theme::glass_colours(lk.light);
        p.rect_filled(card, cr, gf);
        p.rect_stroke(card, cr, Stroke::new(1.0f32, gr));
    } else {
        p.rect_filled(card, cr, s.card);
        p.rect_stroke(card, cr, Stroke::new(1.0f32, s.line));
    }
    p.rect_filled(Rect::from_min_size(card.min + vec2(6.0, 7.0), vec2(card.width() * 0.32, 6.0)), 3.0, s.text);
    p.rect_filled(Rect::from_min_size(card.min + vec2(6.0, 18.0), vec2(card.width() * 0.48, 4.0)), 2.0, s.muted);
    let btn = Rect::from_min_size(pos2(card.max.x - 34.0, card.center().y - 6.0), vec2(28.0, 12.0));
    if def.is_some_and(|d| d.pixel) {
        // (a tiny pixel key: outline, fill, bevel)
        p.rect_filled(btn.expand(1.5), 0.0, theme::blend(lk.button, Color32::BLACK, 0.82));
        p.rect_filled(btn, 0.0, lk.button);
        p.rect_filled(Rect::from_min_size(btn.min, vec2(btn.width(), 2.0)), 0.0, theme::blend(lk.button, Color32::WHITE, 0.55));
        p.rect_filled(Rect::from_min_max(pos2(btn.min.x, btn.max.y - 2.0), btn.max), 0.0, theme::blend(lk.button, Color32::BLACK, 0.28));
    } else {
        p.rect_filled(btn, 6.0 * def.map_or(1.0, |d| d.round), lk.button);
    }
    if on {
        let c = pos2(win.max.x - 10.0, win.max.y - 10.0);
        p.circle_filled(c, 8.0, Color32::WHITE);
        p.add(egui::Shape::line(vec![c + vec2(-3.5, 0.0), c + vec2(-1.0, 2.5), c + vec2(4.0, -3.0)], Stroke::new(2.0f32, lk.accent)));
    }
    p.rect_stroke(win, 8.0, Stroke::new(1.0f32, theme::blend(s.line, s.text, 0.10)));
    p.text(pos2(r.min.x + 10.0, win.max.y + 17.0), Align2::LEFT_CENTER, name, ch::bold(12.0), TEXT());
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
        let mut b = Appearance::default();
        b.plus_theme = PlusTheme::Studio;
        let code = look_code(&b);
        assert!(code.ends_with("-T1"));
        assert_eq!(parse_look(&code, &Appearance::default()), Some(b));
    }
}
