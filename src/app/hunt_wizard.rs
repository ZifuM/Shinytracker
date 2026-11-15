//! New hunt: a step-by-step pop-up on the Hunt page that walks through
//! setting up a hunt (the Pokémon, how you're hunting, battles, bonuses,
//! counting, where the count starts, extras) with Previous, Skip and Next,
//! then a review and Start hunt. Nothing changes until Start hunt; every
//! setting also stays on the Hunt page to change later.

use super::hunt_page::method_tile;
use super::settings::{method_badge, method_experimental, method_offered, EXPERIMENTAL_NOTE};
use super::ui_kit::{self as kit, PillKind, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::config::{BattleType, CountSource, HuntMethod};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

const VIOLET: Color32 = Color32::from_rgb(139, 92, 246);

#[derive(Clone, Copy, PartialEq, Debug)]
enum Step {
    Target,
    Method,
    Battles,
    Bonuses,
    Counting,
    Start,
    Extras,
    Review,
}

impl Step {
    fn meta(self) -> (&'static str, &'static str) {
        match self {
            Step::Target => ("🎯", "Pokémon"),
            Step::Method => ("⚔", "Method"),
            Step::Battles => ("👥", "Battles"),
            Step::Bonuses => ("✨", "Bonuses"),
            Step::Counting => ("👁", "Counting"),
            Step::Start => ("🔢", "Count"),
            Step::Extras => ("⚙", "Extras"),
            Step::Review => ("🏁", "Review"),
        }
    }
}

/// Where the count starts.
#[derive(Clone, Copy, PartialEq, Debug)]
enum From {
    /// A fresh hunt at 0.
    Zero,
    /// Pick up the saved hunt for this Pokémon (or keep the current one).
    Resume,
    /// Start at a number (hunting it somewhere else before).
    Number,
}

pub(super) struct HuntWizard {
    step: Step,
    target: Option<String>,
    method: HuntMethod,
    battle: BattleType,
    charms: u32,
    lure: bool,
    odds: u32,
    source: CountSource,
    from: From,
    number: u64,
    auto_method: bool,
    target_only: bool,
    overlay: bool,
    /// The target picker's list was used this step (it shares the search).
    search_cleared: bool,
}

impl App {
    /// Open the New hunt guide on the Hunt page, filled in with the hunt's
    /// settings as they are now.
    pub(super) fn open_hunt_wizard(&mut self) {
        let c = &self.cfg;
        self.setup.hunt_wizard = Some(HuntWizard {
            step: Step::Target,
            target: c.target.clone(),
            method: c.hunt_method,
            battle: c.battle_type,
            charms: c.shiny_charms,
            lure: c.shiny_lure,
            odds: c.odds,
            source: c.count_source,
            // a hunt under way carries on unless you say otherwise
            from: if c.count > 0 { From::Resume } else { From::Zero },
            number: 0,
            auto_method: c.auto_method,
            target_only: c.odds_target_only,
            overlay: !self.overlay_on,
            search_cleared: false,
        });
        self.setup.tab = Tab::Hunt;
        self.setup.picking_target = false;
        self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
        self.setup.scroll_top = true;
    }

    /// The New hunt guide with `slug` picked as the target (a fresh hunt),
    /// on its next step. Nothing changes until it's started there.
    pub(super) fn open_hunt_wizard_for(&mut self, slug: &str) {
        self.open_hunt_wizard();
        let same = self.cfg.target.as_deref() == Some(slug);
        if let Some(w) = self.setup.hunt_wizard.as_mut() {
            w.target = Some(slug.to_string());
            if !same {
                w.from = From::Zero;
            }
            w.step = Step::Method;
        }
    }

    /// The steps that apply to the choices so far.
    fn wizard_steps(w: &HuntWizard) -> Vec<Step> {
        let mut v = vec![Step::Target, Step::Method];
        if w.method.is_wild_battles() {
            v.push(Step::Battles);
        }
        if w.method.uses_charms() || w.method == HuntMethod::Custom {
            v.push(Step::Bonuses);
        }
        v.extend([Step::Counting, Step::Start, Step::Extras, Step::Review]);
        v
    }

    /// The odds the choices give, as "1 in N".
    fn wizard_odds(&self, w: &HuntWizard) -> Option<f64> {
        let mut c = self.cfg.clone();
        c.hunt_method = w.method;
        c.shiny_charms = w.charms;
        c.shiny_lure = w.lure;
        c.odds = w.odds;
        c.battle_type = w.battle;
        c.odds_one_in()
    }

    /// The hunt saved for this Pokémon, if there is one: (encounters, saved when).
    fn wizard_saved(&self, slug: Option<&str>) -> Option<(u64, u64)> {
        let slug = slug?;
        if self.cfg.target.as_deref() == Some(slug) {
            return Some((self.cfg.count, 0));
        }
        self.cfg.saved_runs.iter().filter(|r| r.target.as_deref() == Some(slug)).max_by_key(|r| r.saved_unix).map(|r| (r.count, r.saved_unix))
    }

    pub(super) fn hunt_wizard(&mut self, ctx: &egui::Context) {
        let Some(mut w) = self.setup.hunt_wizard.take() else { return };
        if self.setup.tab != Tab::Hunt {
            // left the page: put it away (nothing was changed)
            return;
        }
        let screen = ctx.screen_rect();
        let steps = Self::wizard_steps(&w);
        if !steps.contains(&w.step) {
            w.step = Step::Review;
        }
        let at = steps.iter().position(|s| *s == w.step).unwrap_or(0);
        let width = 680.0f32.min(screen.width() - 40.0);
        let height = 600.0f32.min(screen.height() - 80.0);
        let pos = pos2(screen.center().x - width / 2.0, screen.center().y - height / 2.0 + 10.0);
        let mut go: Option<isize> = None;
        let mut close = false;
        let mut start = false;
        let mut quick_pick = false;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        egui::Area::new(egui::Id::new("hunt-wizard")).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
            ui.painter().with_clip_rect(screen).rect_filled(screen, 0.0, Color32::from_black_alpha(120));
            egui::Frame::none()
                .fill(kit::CARD())
                .stroke(Stroke::new(1.0f32, theme::blend(kit::CARD(), TEXT(), 0.16)))
                .rounding(16.0)
                .shadow(egui::epaint::Shadow { offset: vec2(0.0, 18.0), blur: 50.0, spread: 0.0, color: Color32::from_black_alpha(140) })
                .inner_margin(18.0)
                .show(ui, |ui| {
                    ui.set_width(width - 36.0);
                    ui.set_height(height - 36.0);
                    // title and the close button
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("New hunt").font(super::charts::bold(19.0)).color(TEXT()));
                        ui.label(RichText::new(format!("Step {} of {}", at + 1, steps.len())).size(12.0).color(MUTED()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (xr, xresp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
                            let hov = xresp.hovered();
                            if hov {
                                ui.painter().rect_filled(xr, 7.0, theme::blend(kit::CARD(), TEXT(), 0.08));
                            }
                            let c = xr.center();
                            let st = Stroke::new(1.5f32, if hov { TEXT() } else { MUTED() });
                            ui.painter().line_segment([c - vec2(5.0, 5.0), c + vec2(5.0, 5.0)], st);
                            ui.painter().line_segment([c + vec2(-5.0, 5.0), c + vec2(5.0, -5.0)], st);
                            if xresp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Close: nothing changes (Esc)").clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.add_space(10.0);
                    // the steps, as a rail
                    let (rr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
                    let n = steps.len() as f32;
                    let sw = rr.width() / n;
                    for (i, s) in steps.iter().enumerate() {
                        let (icon, name) = s.meta();
                        let c = pos2(rr.min.x + sw * (i as f32 + 0.5), rr.min.y + 11.0);
                        if i + 1 < steps.len() {
                            ui.painter().line_segment([c + vec2(13.0, 0.0), c + vec2(sw - 13.0, 0.0)], Stroke::new(2.0f32, if i < at { VIOLET } else { LINE() }));
                        }
                        let done = i < at;
                        let here = i == at;
                        let fill = if here { VIOLET } else if done { theme::blend(kit::CARD(), VIOLET, 0.45) } else { theme::blend(kit::CARD(), TEXT(), 0.08) };
                        ui.painter().circle_filled(c, 11.0, fill);
                        ui.painter().text(c, Align2::CENTER_CENTER, if done { "✔" } else { icon }, egui::FontId::proportional(11.0), if here || done { Color32::WHITE } else { MUTED() });
                        ui.painter().text(pos2(c.x, rr.max.y - 6.0), Align2::CENTER_CENTER, name, crate::fonts::sans_font(10.5), if here { TEXT() } else { MUTED() });
                        let hit = ui.interact(Rect::from_center_size(pos2(c.x, rr.center().y), vec2(sw, 40.0)), ui.id().with(("wiz-step", i)), Sense::click());
                        if hit.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("Go to {name}")).clicked() {
                            go = Some(i as isize - at as isize);
                        }
                    }
                    ui.add_space(12.0);
                    let (lr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().rect_filled(lr, 0.0, LINE());
                    ui.add_space(12.0);
                    // the step
                    let foot = 52.0;
                    let body_h = (ui.available_height() - foot).max(120.0);
                    egui::ScrollArea::vertical().id_salt(("wiz-body", w.step as u8)).max_height(body_h).auto_shrink([false, false]).show(ui, |ui| {
                        ui.set_min_height(body_h - 4.0);
                        match w.step {
                            Step::Target => self.wiz_target(ui, &mut w),
                            Step::Method => self.wiz_method(ui, &mut w),
                            Step::Battles => wiz_battles(ui, &mut w),
                            Step::Bonuses => self.wiz_bonuses(ui, &mut w),
                            Step::Counting => wiz_counting(ui, &mut w),
                            Step::Start => self.wiz_start(ui, &mut w),
                            Step::Extras => self.wiz_extras(ui, &mut w),
                            Step::Review => {
                                if self.wiz_review(ui, &mut w) {
                                    go = Some(-(at as isize));
                                }
                            }
                        }
                    });
                    // Previous · Skip · Next / Start hunt
                    let (fr, _) = ui.allocate_exact_size(vec2(ui.available_width(), foot), Sense::hover());
                    ui.painter().line_segment([fr.left_top(), fr.right_top()], Stroke::new(1.0f32, LINE()));
                    let by = fr.min.y + 12.0;
                    let first = at == 0;
                    let last = at + 1 == steps.len();
                    let prev = Rect::from_min_size(pos2(fr.min.x, by), vec2(110.0, 36.0));
                    if ui.add_enabled_ui(!first, |ui| ui.put(prev, egui::Button::new(RichText::new("‹  Previous").size(13.0)).fill(kit::btn_fill()).rounding(9.0))).inner.clicked() {
                        go = Some(-1);
                    }
                    // skip the guide, now or from now on
                    let cb = Rect::from_min_size(pos2(prev.max.x + 16.0, by + 1.0), vec2(200.0, 17.0));
                    let mut off = self.cfg.hunt_guide_off;
                    if ui
                        .put(cb, egui::Checkbox::new(&mut off, RichText::new("Skip this guide next time").size(12.0).color(MUTED())))
                        .on_hover_text("New hunt opens the Pokémon picker on the Hunt page instead, the way it used to. The + New hunt button on the Hunt page always opens this guide.")
                        .changed()
                    {
                        self.cfg.hunt_guide_off = off;
                        self.dirty = true;
                    }
                    let quick = Rect::from_min_size(pos2(prev.max.x + 16.0, by + 19.0), vec2(200.0, 17.0));
                    if ui.put(quick, egui::Button::new(RichText::new("Just pick a Pokémon  ›").size(12.0).color(VIOLET)).frame(false)).on_hover_text("Close the guide and pick a Pokémon on the Hunt page, keeping everything else as it is").clicked() {
                        quick_pick = true;
                    }
                    let next = Rect::from_min_size(pos2(fr.max.x - 150.0, by), vec2(150.0, 36.0));
                    let label = if last { "▶  Start hunt" } else { "Next  ›" };
                    let next_resp = ui.put(next, egui::Button::new(RichText::new(label).font(super::charts::bold(13.5)).color(kit::on_primary())).fill(VIOLET).rounding(9.0));
                    let need_target = w.step == Step::Target && w.target.is_none();
                    if next_resp.on_hover_text(if need_target { "Pick a Pokémon first, or Skip to hunt without one" } else { "" }).clicked() {
                        if last {
                            start = true;
                        } else if !need_target {
                            go = Some(1);
                        }
                    }
                    if !last {
                        let skip = Rect::from_min_size(pos2(next.min.x - 98.0, by), vec2(88.0, 36.0));
                        if ui.put(skip, egui::Button::new(RichText::new("Skip").size(13.0).color(MUTED())).frame(false)).on_hover_text("Leave this as it is and go on").clicked() {
                            go = Some(1);
                        }
                    }
                });
        });
        if let Some(d) = go {
            let i = (at as isize + d).clamp(0, steps.len() as isize - 1) as usize;
            w.step = steps[i];
        }
        if start {
            self.wizard_apply(&w);
            return;
        }
        if quick_pick {
            self.open_quick_picker();
            return;
        }
        if !close {
            self.setup.hunt_wizard = Some(w);
        }
    }

    // ───────────────────────────── the steps ─────────────────────────────

    fn wiz_target(&mut self, ui: &mut egui::Ui, w: &mut HuntWizard) {
        step_head(ui, "Which Pokémon are you hunting?", "Search by name or number, or pick one you've hunted before. You can add more targets later on the Hunt page.");
        if !w.search_cleared {
            self.setup.search.clear();
            w.search_cleared = true;
        }
        if let Some(slug) = w.target.clone() {
            let name = self.target_name(Some(&slug));
            ui.horizontal(|ui| {
                kit::pill(ui, &format!("✔ {name}"), PillKind::Ok);
                if let Some((n, when)) = self.wizard_saved(Some(&slug)).filter(|(n, _)| *n > 0) {
                    let what = if when == 0 { format!("your current hunt: {} so far", theme::thousands(n)) } else { format!("you have a saved hunt: {} so far", theme::thousands(n)) };
                    ui.label(RichText::new(what).size(12.0).color(MUTED()));
                }
            });
            ui.add_space(8.0);
        }
        let h = (ui.available_height() - 8.0).max(200.0);
        if let Some(slug) = self.target_picker(ui, h) {
            if w.target.as_deref() != Some(&slug) {
                w.from = if self.wizard_saved(Some(&slug)).is_some_and(|(n, _)| n > 0) { From::Resume } else { From::Zero };
            }
            w.target = Some(slug);
        }
    }

    fn wiz_method(&mut self, ui: &mut egui::Ui, w: &mut HuntWizard) {
        step_head(ui, "How are you hunting it?", "Each way has its own shiny odds. Pick the one you'll use: you can switch any time on the Hunt page.");
        // on rare grass today: say so
        if let Some(slug) = w.target.clone() {
            let spots = self.rare_spots_for(&slug);
            if !spots.is_empty() {
                let places = super::rare::places_text(&spots);
                kit::pill(ui, &format!("🌿 {} is on rare grass today: {places}. Rare grass has double odds.", self.target_name(Some(&slug))), PillKind::Ok);
                ui.add_space(8.0);
            }
        }
        let mut c = self.cfg.clone();
        c.shiny_charms = w.charms;
        c.shiny_lure = w.lure;
        let methods: Vec<HuntMethod> = HuntMethod::ALL.into_iter().filter(|&m| method_offered(m) || m == w.method).collect();
        let full = ui.available_width();
        let per = if full >= 500.0 { 3 } else { 2 };
        let gap = 10.0;
        let tw = ((full - gap * (per as f32 - 1.0)) / per as f32).floor();
        for chunk in methods.chunks(per) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &m in chunk {
                    if method_tile(ui, tw, m, w.method == m, &method_badge(&c, m)) {
                        w.method = m;
                    }
                }
            });
            ui.add_space(gap);
        }
        ui.add(egui::Label::new(RichText::new(w.method.note()).size(12.0).color(MUTED())).wrap());
        if method_experimental(w.method) {
            ui.add_space(6.0);
            kit::pill(ui, &format!("⚠ {EXPERIMENTAL_NOTE}"), PillKind::Warn);
        }
        if w.method == HuntMethod::Masuda {
            ui.add_space(6.0);
            ui.label(RichText::new("Your Day-Care and Eggs show on the Hunt page, from your save.").size(12.0).color(MUTED()));
        }
    }

    fn wiz_bonuses(&mut self, ui: &mut egui::Ui, w: &mut HuntWizard) {
        if w.method == HuntMethod::Custom {
            step_head(ui, "What are your odds?", "Type the chance of a shiny, as 1 in this many.");
            ui.add(egui::DragValue::new(&mut w.odds).range(1..=65536).prefix("1 in "));
            return;
        }
        step_head(ui, "Anything boosting your odds?", "These add rolls out of 8192 (from the game's dev). Wild: +1 per Shiny Charm, +3 with a Shiny Lure lit. Eggs: +2 for the Charm, +2 for the Lure. Ten rolls at most.");
        row(ui, "Shiny Charms in your Bag", if w.method == HuntMethod::Masuda { "For Eggs one Charm counts (more don't add more)." } else { "Each one adds a roll, and they stack." }, |ui| {
            kit::stepper(ui, &mut w.charms, 0, 20);
        });
        if w.method != HuntMethod::PokeRadar {
            ui.add_space(6.0);
            switch(ui, "Shiny Lure lit", "On while it's lit. The app follows the game's log too: on when you use one, off when it wears off.", &mut w.lure);
        }
        ui.add_space(12.0);
        if let Some(o) = self.wizard_odds(w) {
            kit::pill(ui, &format!("Your odds: 1 in {}", super::fmt_one_in(o)), PillKind::Info);
        }
    }

    fn wiz_start(&mut self, ui: &mut egui::Ui, w: &mut HuntWizard) {
        step_head(ui, "Where does the count start?", "Hunting it before somewhere else? Start from that number so your odds stay right.");
        let saved = self.wizard_saved(w.target.as_deref()).filter(|(n, _)| *n > 0);
        if let Some((n, when)) = saved {
            let label = if when == 0 { format!("Keep going from {}", theme::thousands(n)) } else { format!("Pick up your saved hunt ({} so far)", theme::thousands(n)) };
            choice(ui, w.from == From::Resume, &label, "Carries on where you left off: the count, time and stats.", || w.from = From::Resume);
        }
        if choice(ui, w.from == From::Zero, "Start at 0", if saved.is_some() { "A fresh hunt. The old one is kept under Runs." } else { "A fresh hunt." }, || {}) {
            w.from = From::Zero;
        }
        if choice(ui, w.from == From::Number, "Start from a number", "You've already done some encounters (on another tracker, say).", || {}) {
            w.from = From::Number;
        }
        if w.from == From::Number {
            ui.horizontal(|ui| {
                ui.add_space(30.0);
                ui.add(egui::DragValue::new(&mut w.number).range(0..=9_999_999).speed(10.0).suffix(" encounters"));
            });
        }
    }

    fn wiz_extras(&mut self, ui: &mut egui::Ui, w: &mut HuntWizard) {
        step_head(ui, "A few extras", "Change these any time on the Hunt page.");
        switch(ui, "Switch method automatically", "Changes to Rare grass when the game log shows Sweet Scent on rare grass, and to Soft resets for starters and static Pokémon. A short pop-up says when.", &mut w.auto_method);
        if w.method.is_wild_battles() && w.target.is_some() {
            ui.add_space(8.0);
            let name = self.target_name(w.target.as_deref());
            switch(ui, &format!("Odds count only {name}"), &format!("Only {name} encounters can give you a shiny {name}: the odds bar uses those instead of every encounter."), &mut w.target_only);
        }
        ui.add_space(8.0);
        switch(ui, "Start the overlay when I start", "The counter on top of the game. You can start it later from the top bar too.", &mut w.overlay);
    }

    /// True: "Change" was pressed (back to the first step).
    fn wiz_review(&mut self, ui: &mut egui::Ui, w: &mut HuntWizard) -> bool {
        step_head(ui, "All set?", "Here's your hunt. Start hunt saves it; any step can be changed first.");
        let name = w.target.as_deref().map_or("No Pokémon picked".to_string(), |s| self.target_name(Some(s)));
        let odds = self.wizard_odds(w).map_or("—".to_string(), |o| format!("1 in {}", super::fmt_one_in(o)));
        let mut rows: Vec<(&str, String)> = vec![("Pokémon", name), ("Method", w.method.label().to_string())];
        if w.method.is_wild_battles() {
            rows.push(("Battles", match w.battle {
                BattleType::Doubles => "Doubles".into(),
                BattleType::Singles => "Singles".into(),
                BattleType::Auto => "Not sure".into(),
            }));
        }
        if w.method.uses_charms() {
            rows.push(("Shiny Charms", w.charms.to_string()));
            if w.method != HuntMethod::PokeRadar {
                rows.push(("Shiny Lure", if w.lure { "Lit".into() } else { "Not lit".into() }));
            }
        }
        rows.push(("Odds", odds));
        rows.push(("Counting", match w.source {
            CountSource::GameLog => "From the game's log".into(),
            CountSource::Screen => "By reading the screen".into(),
        }));
        rows.push(("Count starts", match w.from {
            From::Zero => "At 0".into(),
            From::Resume => "Where you left off".into(),
            From::Number => format!("At {}", theme::thousands(w.number)),
        }));
        rows.push(("Overlay", if w.overlay { "Starts now".into() } else { "Not now".into() }));
        for (k, v) in rows {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(130.0, 26.0), Sense::hover());
                ui.painter().text(r.left_center(), Align2::LEFT_CENTER, k, crate::fonts::sans_font(13.0), MUTED());
                ui.label(RichText::new(v).size(13.5).strong().color(TEXT()));
            });
            let (lr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().rect_filled(lr, 0.0, LINE());
        }
        ui.add_space(10.0);
        let mut back = false;
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new("Change something").fill(kit::btn_fill())).on_hover_text("Back to the first step (or click a step above)").clicked() {
                back = true;
            }
            if self.cfg.target.is_some() && self.cfg.count > 0 && w.target != self.cfg.target {
                ui.label(RichText::new("Your current hunt is saved under Runs.").size(12.0).color(MUTED()));
            }
        });
        back
    }

    /// Start hunt: everything chosen goes into the hunt at once.
    fn wizard_apply(&mut self, w: &HuntWizard) {
        // the Pokémon and the count
        let same = w.target == self.cfg.target;
        match (same, w.from) {
            (true, From::Resume) => {}
            (true, _) => {
                self.park_current();
                self.start_fresh(w.target.clone());
            }
            (false, From::Resume) => {
                if let Some(slug) = w.target.clone() {
                    self.set_target(slug);
                }
            }
            (false, _) => {
                self.park_current();
                self.cfg.target2 = None;
                self.cfg.more_targets.clear();
                self.start_fresh(w.target.clone());
            }
        }
        if w.from == From::Number {
            self.cfg.count = w.number;
        }
        // the settings
        self.cfg.hunt_method = w.method;
        self.cfg.battle_type = w.battle;
        self.cfg.shiny_charms = w.charms;
        self.cfg.shiny_lure = w.lure;
        self.cfg.odds = w.odds;
        self.cfg.count_source = w.source;
        self.cfg.auto_method = w.auto_method;
        self.cfg.odds_target_only = w.target_only;
        self.dirty = true;
        self.push_settings(true);
        self.setup.hunt_wizard = None;
        self.setup.tab = Tab::Hunt;
        self.setup.scroll_top = true;
        if w.overlay && !self.overlay_on {
            self.start_overlay();
        }
    }
}

fn wiz_battles(ui: &mut egui::Ui, w: &mut HuntWizard) {
    step_head(ui, "Single or double battles?", "In a double battle each Pokémon gets its own shiny chance, so the odds per battle are better. Sweet Scent on rare grass usually gives doubles.");
    if choice(ui, w.battle == BattleType::Doubles, "Doubles", "Two Pokémon per battle (Sweet Scent, doubles grass).", || {}) {
        w.battle = BattleType::Doubles;
    }
    if choice(ui, w.battle == BattleType::Singles, "Singles", "One Pokémon per battle.", || {}) {
        w.battle = BattleType::Singles;
    }
    if choice(ui, w.battle == BattleType::Auto, "Not sure", "Counts however many Pokémon each battle has.", || {}) {
        w.battle = BattleType::Auto;
    }
}

fn wiz_counting(ui: &mut egui::Ui, w: &mut HuntWizard) {
    step_head(ui, "How should encounters be counted?", "Both count by themselves while you play. The game's log is the most accurate.");
    if choice(ui, w.source == CountSource::GameLog, "From the game's log (best)", "Reads the game's own session log: every encounter, egg and the exact odds. Nothing to set up.", || {}) {
        w.source = CountSource::GameLog;
    }
    if choice(ui, w.source == CountSource::Screen, "By reading the screen", "Reads the Pokémon's names on screen. Needs the name boxes set up on the Counting page.", || {}) {
        w.source = CountSource::Screen;
    }
}

/// A step's title and a line under it.
fn step_head(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.label(RichText::new(title).font(super::charts::bold(17.0)).color(TEXT()));
    ui.add_space(2.0);
    ui.add(egui::Label::new(RichText::new(sub).size(12.5).color(MUTED())).wrap());
    ui.add_space(12.0);
}

/// A setting: its name and help on the left, the control on the right.
fn row(ui: &mut egui::Ui, label: &str, help: &str, control: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 160.0).max(120.0));
            ui.label(RichText::new(label).size(13.5).strong().color(TEXT()));
            ui.add(egui::Label::new(RichText::new(help).size(12.0).color(MUTED())).wrap());
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control);
    });
}

/// One choice of several: a card with a ring, its name and a line about it.
/// True when clicked.
fn choice(ui: &mut egui::Ui, on: bool, title: &str, sub: &str, then: impl FnOnce()) -> bool {
    let w = ui.available_width();
    let g = ui.painter().layout(sub.to_string(), crate::fonts::sans_font(12.0), MUTED(), w - 60.0);
    let h = 30.0 + g.size().y + 10.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let hov = resp.hovered();
    let p = ui.painter();
    p.rect_filled(r, 10.0, if on { theme::blend(kit::CARD(), VIOLET, 0.14) } else { theme::blend(kit::CARD(), TEXT(), if hov { 0.06 } else { 0.03 }) });
    p.rect_stroke(r, 10.0, Stroke::new(1.0f32, if on { VIOLET } else { LINE() }));
    let c = pos2(r.min.x + 22.0, r.min.y + 20.0);
    p.circle_stroke(c, 8.0, Stroke::new(1.5f32, if on { VIOLET } else { MUTED() }));
    if on {
        p.circle_filled(c, 4.5, VIOLET);
    }
    p.text(pos2(r.min.x + 42.0, r.min.y + 20.0), Align2::LEFT_CENTER, title, super::charts::bold(13.5), TEXT());
    p.galley(pos2(r.min.x + 42.0, r.min.y + 32.0), g, MUTED());
    ui.add_space(8.0);
    let clicked = resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    if clicked {
        then();
    }
    clicked
}

/// A switch with its name and a short line about it, both showing.
fn switch(ui: &mut egui::Ui, label: &str, help: &str, on: &mut bool) {
    row(ui, label, help, |ui| {
        kit::toggle(ui, on);
    });
    ui.add_space(4.0);
}
