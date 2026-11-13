//! The Profile page: your trainer at a glance (badge, name, level and XP),
//! what's nearly unlocked, recent unlocks, where XP comes from and the road
//! to Champion; then goals and an achievement browser.

use super::charts::{self as ch, BLUE, CYAN, GOLD, GREEN, ORANGE, PINK, PURPLE};
use super::trainer::{AchFilter, ProfileSection, ToastKind};
use super::ui_kit::{self as kit, MUTED, TEXT};
use super::App;
use crate::progress::{self, Achievement, Category, Lifetime, Tier, ACHIEVEMENTS};
use crate::stats::{fmt_date, fmt_duration};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

/// Levels where the title changes, with the title.
const TITLES: [(u32, &str); 11] = [
    (1, "Rookie Trainer"),
    (5, "Shiny Seeker"),
    (10, "Sparkle Scout"),
    (15, "Shiny Spotter"),
    (20, "Shiny Hunter"),
    (30, "Veteran Hunter"),
    (40, "Shiny Expert"),
    (50, "Shiny Master"),
    (65, "Shiny Ace"),
    (80, "Shiny Legend"),
    (100, "Shiny Champion"),
];

const SORTS: [&str; 4] = ["Ladder order", "Closest first", "Recently unlocked", "Rarest tier"];

fn tier_col(t: Tier) -> Color32 {
    let c = t.rgb();
    Color32::from_rgb(c[0], c[1], c[2])
}

fn cat_icon(c: Category) -> &'static str {
    match c {
        Category::Encounters => "⚔",
        Category::Time => "⏱",
        Category::Shinies => "✨",
        Category::Luck => "🍀",
        Category::Dedication => "🔥",
        Category::Goals => "🎯",
        Category::Fun => "🎉",
    }
}

impl App {
    fn ach_when(&self, id: &str) -> Option<u64> {
        self.cfg.achievements.iter().find(|u| u.id == id).map(|u| u.unix)
    }

    pub(super) fn page_profile(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "PROFILE", "Your trainer: level, XP, goals and achievements. Everything you hunt earns XP.");
        let l = self.lifetime_full();
        self.profile_hero(ui, &l);
        ui.add_space(18.0);

        let unlocked = self.cfg.achievements.iter().filter(|u| progress::find(&u.id).is_some()).count();
        let tier = crate::pass::tier(self.pass_xp());
        let today = crate::stats::local_day(crate::stats::now_unix());
        let goals = if crate::pass::current(today).active(today) { format!("Battle Pass  ·  Tier {tier}") } else { "Battle Pass".to_string() };
        let achs = format!("Achievements  {unlocked}/{}", ACHIEVEMENTS.len());
        let mut sec = self.setup.profile_section;
        if kit::tab_bar(ui, &mut sec, &[(ProfileSection::Overview, "Overview"), (ProfileSection::Goals, &goals), (ProfileSection::Achievements, &achs)]) {
            self.setup.profile_section = sec;
        }
        ui.add_space(16.0);
        match self.setup.profile_section {
            ProfileSection::Overview => self.profile_overview(ui, &l),
            ProfileSection::Goals => self.pass_section(ui),
            ProfileSection::Achievements => self.profile_achievements(ui, &l),
        }
        ui.add_space(30.0);
    }

    // ───────────────────────────── trainer card ─────────────────────────────

    fn profile_hero(&mut self, ui: &mut egui::Ui, l: &Lifetime) {
        let xp = self.total_xp(l);
        let total = xp.total();
        let level = progress::level_for(total);
        let (into, need) = progress::level_progress(total);
        let (metal, main, light) = progress::badge_metal(level);
        let mc = Color32::from_rgb(main[0], main[1], main[2]);
        let lc = Color32::from_rgb(light[0], light[1], light[2]);
        let w = ui.available_width();
        let wide = w >= 900.0;
        let h = if wide { 232.0 } else { 330.0 };
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter_at(rect.expand(16.0));
        p.add(kit::card_shadow().as_shape(rect, 28.0));
        p.rect_filled(rect, 28.0, kit::CARD());
        // a soft glow of the badge's metal behind the badge
        let pc = p.with_clip_rect(rect.shrink(1.0));
        let bc = pos2(rect.min.x + 112.0, rect.min.y + 104.0);
        for (r, a) in [(170.0, 0.05), (120.0, 0.07), (84.0, 0.10)] {
            pc.circle_filled(bc, r, ch::faint(mc, a));
        }
        p.rect_stroke(rect, 28.0, Stroke::new(1.0f32, theme::blend(kit::LINE(), mc, 0.35)));
        // the profile picture (or the level badge), click to change it
        let ctx = ui.ctx().clone();
        let has_pic = self.draw_avatar(&ctx, &p, bc, 66.0, level, mc);
        let pic = ui.interact(Rect::from_center_size(bc, vec2(136.0, 136.0)), ui.id().with("avatar-pic"), Sense::click());
        if pic.hovered() {
            if has_pic {
                p.circle_filled(bc, 66.0, Color32::from_black_alpha(120));
                p.text(bc - vec2(0.0, 8.0), Align2::CENTER_CENTER, "📷", egui::FontId::proportional(22.0), Color32::WHITE);
                p.text(bc + vec2(0.0, 16.0), Align2::CENTER_CENTER, "Change", ch::bold(12.5), Color32::WHITE);
            } else {
                let chip = Rect::from_center_size(bc + vec2(0.0, 60.0), vec2(104.0, 24.0));
                p.rect_filled(chip, 12.0, kit::primary());
                p.text(chip.center(), Align2::CENTER_CENTER, "📷  Add a photo", ch::bold(11.5), kit::on_primary());
            }
        }
        if pic
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(if has_pic { "Change or remove your profile picture" } else { "Add a profile picture" })
            .clicked()
        {
            self.open_avatar_editor(&ctx);
        }
        p.text(pos2(bc.x, bc.y + 88.0), Align2::CENTER_CENTER, format!("{} badge", metal), ch::bold(11.5), ch::ink(lc));

        let x = rect.min.x + 226.0;
        let stats_w = if wide { 300.0 } else { 0.0 };
        let room = rect.max.x - 28.0 - stats_w - x;
        // name, editable in place
        // the name plate from the battle pass sits behind the name
        let shown = if self.cfg.trainer_name.is_empty() { "Your trainer name".to_string() } else { self.cfg.trainer_name.clone() };
        let plate = self.cfg.plate.clone();
        // on a plate the name is set smaller, so the plate frames it
        let name_font = ch::heavy(if plate.is_empty() { 26.0 } else { 19.0 });
        let plate_h = 38.0;
        // the text starts on the column (x); on a plate it moves in by the
        // plate's padding so the plate's left edge is on the column instead
        let text_x = if plate.is_empty() { x } else { x + plate_h * crate::cosmetics::PLATE_PAD };
        let nr = Rect::from_min_size(pos2(text_x, rect.min.y + 22.0), vec2(room.min(380.0) - (text_x - x), 40.0));
        let _ = shown;
        let ink = (!plate.is_empty()).then(|| super::pass_page::plate_ink(&plate));
        // the plate goes under the name, but where depends on where the text box
        // puts the text: keep its place in the paint order and fill it in after
        let plate_slot = p.add(egui::Shape::Noop);
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(nr).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let mut te = egui::TextEdit::singleline(&mut self.cfg.trainer_name)
            .hint_text(RichText::new("Your trainer name").color(MUTED().gamma_multiply(0.8)))
            .font(name_font.clone())
            .frame(false)
            .margin(egui::Margin::ZERO)
            .char_limit(16)
            .desired_width(nr.width() - 30.0);
        if let Some(ink) = ink {
            te = te.text_color(ink);
        }
        let out = te.show(&mut c);
        let resp = out.response.clone();
        if ink.is_some() {
            // centre the plate on the name's capitals
            let size = name_font.size;
            let base = out.galley.rows.first().and_then(|r| r.glyphs.first()).map(|g| g.pos.y).unwrap_or(size);
            let cy = out.galley_pos.y + base - size * 0.36;
            let tw = out.galley.size().x.max(48.0);
            let ctx = ui.ctx().clone();
            let shape = self.plate_shape(&ctx, &plate, App::plate_rect(out.galley_pos.x, cy, tw, plate_h));
            p.set(plate_slot, shape);
        }
        let hov = resp.hovered() || resp.has_focus();
        if hov {
            p.rect_stroke(resp.rect.expand2(vec2(6.0, 2.0)), 10.0, Stroke::new(1.0f32, if resp.has_focus() { kit::primary() } else { kit::LINE() }));
        }
        if !resp.has_focus() {
            let tw = p.layout_no_wrap(if self.cfg.trainer_name.is_empty() { "Your trainer name".into() } else { self.cfg.trainer_name.clone() }, name_font.clone(), TEXT()).size().x;
            let tw = if ink.is_some() { tw.max(48.0) + plate_h * crate::cosmetics::PLATE_PAD } else { tw };
            p.text(pos2(resp.rect.min.x + tw + 12.0, resp.rect.center().y), Align2::LEFT_CENTER, "✏", egui::FontId::proportional(14.0), if hov { TEXT() } else { MUTED() });
        }
        if resp.on_hover_text("Click to change your trainer name").changed() {
            self.dirty = true;
        }
        // title and level
        // the title: pick any you've earned
        let tr = Rect::from_min_size(pos2(x - 2.0, rect.min.y + 64.0), vec2(room.min(560.0), 28.0));
        let mut tc = ui.new_child(egui::UiBuilder::new().max_rect(tr).layout(egui::Layout::left_to_right(egui::Align::Center)));
        tc.spacing_mut().item_spacing.x = 10.0;
        self.title_picker(&mut tc, level, lc);
        tc.label(RichText::new(format!("Level {level}")).font(ch::reg(14.0)).color(MUTED()));
        // XP bar
        let frac = if level >= progress::MAX_LEVEL { 1.0 } else { into as f32 / need.max(1) as f32 };
        let bar = Rect::from_min_size(pos2(x, rect.min.y + 104.0), vec2(room.min(560.0), 14.0));
        p.rect_filled(bar, 7.0, theme::blend(kit::CARD(), TEXT(), 0.09));
        let f = kit::grow(ui.ctx(), egui::Id::new("profile-xp"), frac);
        if f > 0.0 {
            p.add(kit::primary_shape(Rect::from_min_size(bar.min, vec2((bar.width() * f).max(14.0), bar.height())), false));
        }
        let left = if level >= progress::MAX_LEVEL {
            "Max level reached!".to_string()
        } else {
            format!("{} / {} XP to level {}", theme::thousands(into), theme::thousands(need), level + 1)
        };
        p.text(pos2(x, bar.max.y + 12.0), Align2::LEFT_TOP, left, ch::bold(12.5), TEXT());
        p.text(pos2(bar.max.x, bar.max.y + 12.0), Align2::RIGHT_TOP, format!("{} XP total", theme::thousands(total)), ch::reg(12.5), MUTED());

        // buttons
        let br = if wide { Rect::from_min_size(pos2(x, rect.max.y - 56.0), vec2(room, 34.0)) } else { Rect::from_min_size(pos2(x, rect.min.y + 150.0), vec2(room, 34.0)) };
        let mut bc_ui = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::left_to_right(egui::Align::Center)));
        bc_ui.spacing_mut().item_spacing.x = 8.0;
        if kit::accent_button(&mut bc_ui, egui::Button::new(RichText::new("🖼  Trainer card").font(ch::bold(13.0)).color(kit::on_primary())), 4.0)
            .on_hover_text("A shareable picture of your level, stats, shinies and achievements")
            .clicked()
        {
            self.setup.profile_card = Some(super::trainer::ProfileCardView::default());
        }
        if bc_ui.add(egui::Button::new("📊  Lifetime stats").fill(kit::btn_fill())).clicked() {
            self.setup.stats_tab = super::stats_life::StatsTab::Lifetime;
            self.go_to(super::Tab::Stats);
        }
        let pic_label = if has_pic { "📷  Profile picture" } else { "📷  Add a photo" };
        if bc_ui.add(egui::Button::new(pic_label).fill(kit::btn_fill())).on_hover_text("Pick a picture, then crop and resize it").clicked() {
            self.open_avatar_editor(&ctx);
        }

        // key numbers
        let unlocked = l.achievements;
        let facts = [
            ("✨", GOLD, "Shinies", if l.phases > 0 { format!("{} (+{} phase)", l.shinies - l.phases, l.phases) } else { l.shinies.to_string() }),
            ("⚔", PURPLE, "Encounters", theme::thousands(l.encounters)),
            ("⏱", CYAN, "Hunted", fmt_duration(l.hunt_secs)),
            ("🏆", ORANGE, "Achievements", format!("{unlocked} / {}", ACHIEVEMENTS.len())),
        ];
        let (fx, fy, fw, fh) = if wide {
            (rect.max.x - 28.0 - stats_w, rect.min.y + 34.0, stats_w / 2.0, 78.0)
        } else {
            (x, rect.min.y + 206.0, (rect.max.x - 28.0 - x) / 2.0, 56.0)
        };
        if wide {
            p.line_segment([pos2(fx - 20.0, rect.min.y + 26.0), pos2(fx - 20.0, rect.max.y - 26.0)], Stroke::new(1.0f32, kit::LINE()));
        }
        for (k, (icon, col, label, value)) in facts.iter().enumerate() {
            let cx = fx + fw * (k % 2) as f32;
            let cy = fy + fh * (k / 2) as f32;
            // chip on the left, label and number beside it
            let chip = Rect::from_min_size(pos2(cx, cy + 4.0), vec2(36.0, 36.0));
            kit::icon_chip(&p, chip, icon, *col, false);
            let tx = chip.max.x + 10.0;
            p.text(pos2(tx, cy + 2.0), Align2::LEFT_TOP, *label, ch::reg(12.0), MUTED());
            let vf = ch::fit_font(ui, value, ch::heavy, 19.0, 11.0, fw - 56.0);
            p.text(pos2(tx, cy + 18.0), Align2::LEFT_TOP, value, vf, TEXT());
        }
    }

    // ───────────────────────────── overview ─────────────────────────────

    fn profile_overview(&mut self, ui: &mut egui::Ui, l: &Lifetime) {
        let w = ui.available_width();
        if w >= 900.0 {
            let gap = 18.0;
            let rw = (w * 0.42).clamp(340.0, 460.0);
            let lw = w - rw - gap;
            let h = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(vec2(lw, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(lw);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    self.profile_almost(ui, l);
                    ui.add_space(18.0);
                    self.profile_recent(ui);
                });
                ui.allocate_ui_with_layout(vec2(rw, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(rw);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    self.profile_xp_card(ui, l);
                    ui.add_space(18.0);
                    self.profile_road(ui, l);
                    ui.add_space(18.0);
                    self.profile_options(ui);
                });
            });
        } else {
            self.profile_almost(ui, l);
            ui.add_space(18.0);
            self.profile_xp_card(ui, l);
            ui.add_space(18.0);
            self.profile_recent(ui);
            ui.add_space(18.0);
            self.profile_road(ui, l);
            ui.add_space(18.0);
            self.profile_options(ui);
        }
    }

    /// The locked achievements you're closest to (one per ladder, furthest
    /// along first).
    pub(super) fn near_achievements(&self, l: &Lifetime) -> Vec<(&'static Achievement, f64)> {
        let mut near: Vec<(&'static Achievement, f64)> = ACHIEVEMENTS
            .iter()
            .filter(|a| self.ach_when(a.id).is_none() && !progress::is_binary(a) && !a.secret)
            .map(|a| {
                let (c, t) = progress::progress(a, l);
                (a, (c / t).clamp(0.0, 1.0))
            })
            .collect();
        // furthest along first; with nothing started, the easiest first steps
        let step = |a: &Achievement| progress::series(progress::series_of(a.id)).iter().position(|x| x.id == a.id).unwrap_or(0);
        near.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.tier.cmp(&b.0.tier)).then(step(a.0).cmp(&step(b.0))));
        // one per ladder, so it isn't four steps of the same thing
        let mut seen: Vec<&str> = vec![];
        near.retain(|(a, _)| {
            let s = progress::series_of(a.id);
            if seen.contains(&s) {
                false
            } else {
                seen.push(s);
                true
            }
        });
        near
    }

    /// The locked achievements you're closest to.
    fn profile_almost(&mut self, ui: &mut egui::Ui, l: &Lifetime) {
        let near = self.near_achievements(l);
        let mut go = false;
        kit::card(ui, |ui| {
            if card_head(ui, "🎯", PINK, "Almost there", Some("See all ›")) {
                go = true;
            }
            ui.add_space(10.0);
            if near.is_empty() {
                ui.label(RichText::new("Start hunting and your next achievements show up here.").size(12.5).color(MUTED()));
                return;
            }
            for (k, (a, f)) in near.iter().take(5).enumerate() {
                if k > 0 {
                    divider(ui);
                }
                self.near_row(ui, a, *f, l);
            }
        });
        if go {
            self.setup.profile_section = ProfileSection::Achievements;
            self.setup.ach_filter = AchFilter::Locked;
            self.setup.ach_sort = 1;
            self.setup.ach_tab = 0;
        }
    }

    pub(super) fn near_row(&mut self, ui: &mut egui::Ui, a: &'static Achievement, f: f64, l: &Lifetime) {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 62.0), Sense::hover());
        let p = ui.painter().clone();
        let tc = tier_col(a.tier);
        let sc = pos2(r.min.x + 26.0, r.center().y);
        p.circle_filled(sc, 23.0, ch::faint(tc, 0.14));
        if let Some(sp) = self.species.iter().find(|s| s.slug == progress::mascot(a.id)).cloned() {
            self.draw_sprite_tinted(&p, &sp, true, sc + vec2(0.0, 18.0), vec2(40.0, 40.0), Color32::from_rgba_unmultiplied(10, 8, 18, 200));
        }
        let x = r.min.x + 62.0;
        let (c, t) = progress::progress(a, l);
        let fmt = |v: f64| if matches!(a.goal, progress::Goal::HuntHours(_)) { format!("{v:.1}h") } else { theme::thousands(v as u64) };
        let nr = p.text(pos2(x, r.min.y + 14.0), Align2::LEFT_CENTER, a.name, ch::bold(13.5), TEXT());
        tier_dot(&p, pos2(nr.max.x + 10.0, nr.center().y), a.tier);
        p.text(pos2(r.max.x, r.min.y + 14.0), Align2::RIGHT_CENTER, format!("{:.0}%", f * 100.0), ch::bold(12.5), ch::ink(tc));
        p.text(pos2(x, r.min.y + 32.0), Align2::LEFT_CENTER, ch::clip(ui, a.desc, &ch::reg(12.0), r.max.x - x - 110.0), ch::reg(12.0), MUTED());
        p.text(pos2(r.max.x, r.min.y + 32.0), Align2::RIGHT_CENTER, format!("{} / {}", fmt(c.min(t)), fmt(t)), ch::reg(11.5), MUTED());
        let bar = Rect::from_min_size(pos2(x, r.min.y + 46.0), vec2(r.max.x - x, 5.0));
        p.rect_filled(bar, 2.5, theme::blend(kit::CARD(), TEXT(), 0.09));
        let g = kit::grow(ui.ctx(), egui::Id::new(("near", a.id)), f as f32);
        if g > 0.0 {
            p.rect_filled(Rect::from_min_size(bar.min, vec2((bar.width() * g).max(5.0), 5.0)), 2.5, tc);
        }
    }

    /// The last few unlocks.
    fn profile_recent(&mut self, ui: &mut egui::Ui) {
        let mut rec: Vec<(&'static Achievement, u64)> = self.cfg.achievements.iter().filter_map(|u| progress::find(&u.id).map(|a| (a, u.unix))).collect();
        rec.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.tier.cmp(&a.0.tier)));
        kit::card(ui, |ui| {
            card_head(ui, "🏆", GOLD, "Recent unlocks", None);
            ui.add_space(12.0);
            if rec.is_empty() {
                ui.label(RichText::new("Nothing yet. Your first unlocks appear here.").size(12.5).color(MUTED()));
                return;
            }
            let full = ui.available_width();
            let per = ((full + 10.0) / 150.0).floor().clamp(2.0, 4.0) as usize;
            let tw = ((full - 10.0 * (per as f32 - 1.0)) / per as f32).floor();
            for row in rec.iter().take(per * 2).collect::<Vec<_>>().chunks(per) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for (a, when) in row {
                        let (r, resp) = ui.allocate_exact_size(vec2(tw, 128.0), Sense::hover());
                        let p = ui.painter().clone();
                        let tc = tier_col(a.tier);
                        p.rect_filled(r, 18.0, theme::blend(kit::CARD(), tc, 0.08));
                        p.rect_stroke(r, 18.0, Stroke::new(1.0f32, theme::blend(kit::LINE(), tc, 0.4)));
                        let sc = pos2(r.center().x, r.min.y + 42.0);
                        p.circle_filled(sc, 28.0, ch::faint(tc, 0.18));
                        if let Some(sp) = self.species.iter().find(|s| s.slug == progress::mascot(a.id)).cloned() {
                            self.draw_sprite_centered(&p, &sp, true, sc, vec2(52.0, 52.0));
                        }
                        theme::sparkle(&p, sc + vec2(24.0, -22.0), 5.0, GOLD);
                        p.text(pos2(r.center().x, r.min.y + 86.0), Align2::CENTER_CENTER, ch::clip(ui, a.name, &ch::bold(12.5), tw - 16.0), ch::bold(12.5), TEXT());
                        p.text(pos2(r.center().x, r.min.y + 106.0), Align2::CENTER_CENTER, fmt_date(*when), ch::reg(11.0), MUTED());
                        resp.on_hover_text(format!("{}\n{} · +{} XP", a.desc, a.tier.label(), a.tier.xp()));
                    }
                });
                ui.add_space(10.0);
            }
        });
    }

    /// Where XP comes from, as one stacked bar.
    fn profile_xp_card(&mut self, ui: &mut egui::Ui, l: &Lifetime) {
        let xp = self.total_xp(l);
        let total = xp.total().max(1) as f64;
        let parts = [
            ("Encounters", "1 per encounter", xp.encounters, PURPLE),
            ("Time", "1 per minute hunted", xp.time, CYAN),
            ("Shinies", "1,000 each", xp.shinies, GOLD),
            ("Achievements", "100 to 1,000 each", xp.achievements, ORANGE),
            ("Goals", "daily goals and challenges", xp.goals, GREEN),
        ];
        kit::card(ui, |ui| {
            card_head(ui, "⚡", PURPLE, "Where your XP comes from", None);
            ui.add_space(14.0);
            let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 14.0), Sense::hover());
            let p = ui.painter();
            p.rect_filled(bar, 7.0, theme::blend(kit::CARD(), TEXT(), 0.09));
            let g = kit::grow(ui.ctx(), egui::Id::new("xp-split"), 1.0);
            let mut x = bar.min.x;
            let shown: Vec<_> = parts.iter().filter(|p| p.2 > 0).collect();
            for (k, (_, _, v, c)) in shown.iter().enumerate() {
                let wv = bar.width() * (*v as f64 / total) as f32 * g;
                let r = Rect::from_min_size(pos2(x, bar.min.y), vec2(wv, bar.height()));
                let rounding = egui::Rounding { nw: if k == 0 { 7.0 } else { 0.0 }, sw: if k == 0 { 7.0 } else { 0.0 }, ne: if k + 1 == shown.len() { 7.0 } else { 0.0 }, se: if k + 1 == shown.len() { 7.0 } else { 0.0 } };
                p.rect_filled(r, rounding, *c);
                x += wv;
            }
            ui.add_space(12.0);
            for (name, how, v, c) in parts {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
                let p = ui.painter();
                p.circle_filled(pos2(r.min.x + 6.0, r.center().y), 5.0, c);
                let nr = p.text(pos2(r.min.x + 20.0, r.center().y), Align2::LEFT_CENTER, name, ch::bold(13.0), TEXT());
                p.text(pos2(nr.max.x + 8.0, r.center().y), Align2::LEFT_CENTER, how, ch::reg(11.5), MUTED());
                p.text(pos2(r.max.x - 52.0, r.center().y), Align2::RIGHT_CENTER, theme::thousands(v), ch::bold(13.0), TEXT());
                p.text(pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, format!("{:.0}%", v as f64 / total * 100.0), ch::reg(12.0), MUTED());
            }
        });
    }

    /// The titles on the way to level 100, and where you are.
    fn profile_road(&mut self, ui: &mut egui::Ui, l: &Lifetime) {
        let level = l.level.max(1);
        kit::card(ui, |ui| {
            card_head(ui, "⬆", BLUE, "Road to Champion", None);
            ui.add_space(8.0);
            if let Some((lv, name)) = TITLES.iter().find(|(lv, _)| *lv > level) {
                ui.label(RichText::new(format!("Next title: {name} at level {lv} · {} to go", lv - level)).size(12.5).color(MUTED()));
            } else {
                ui.label(RichText::new("You've reached the top title. Champion!").size(12.5).color(ch::ink(GOLD)));
            }
            ui.add_space(10.0);
            let row_h = 30.0;
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), row_h * TITLES.len() as f32), Sense::hover());
            let p = ui.painter();
            let lx = r.min.x + 12.0;
            p.line_segment([pos2(lx, r.min.y + row_h / 2.0), pos2(lx, r.max.y - row_h / 2.0)], Stroke::new(2.0f32, kit::LINE()));
            let cur_i = TITLES.iter().rposition(|(lv, _)| *lv <= level).unwrap_or(0);
            // the part already travelled
            p.line_segment([pos2(lx, r.min.y + row_h / 2.0), pos2(lx, r.min.y + row_h / 2.0 + row_h * cur_i as f32)], Stroke::new(2.0f32, kit::primary()));
            for (i, (lv, name)) in TITLES.iter().enumerate() {
                let y = r.min.y + row_h / 2.0 + row_h * i as f32;
                let reached = i <= cur_i;
                if i == cur_i {
                    p.circle_filled(pos2(lx, y), 9.0, ch::faint(kit::primary(), 0.25));
                    p.circle_filled(pos2(lx, y), 6.0, kit::primary());
                } else if reached {
                    p.circle_filled(pos2(lx, y), 5.0, kit::primary());
                } else {
                    p.circle_filled(pos2(lx, y), 5.0, kit::CARD());
                    p.circle_stroke(pos2(lx, y), 5.0, Stroke::new(1.5f32, kit::LINE()));
                }
                let col = if i == cur_i { TEXT() } else if reached { theme::blend(MUTED(), TEXT(), 0.5) } else { MUTED() };
                p.text(pos2(lx + 20.0, y), Align2::LEFT_CENTER, *name, if i == cur_i { ch::bold(13.0) } else { ch::reg(12.5) }, col);
                p.text(pos2(r.max.x, y), Align2::RIGHT_CENTER, format!("Lv {lv}"), ch::reg(12.0), MUTED());
                if i == cur_i {
                    p.text(pos2(r.max.x - 52.0, y), Align2::RIGHT_CENTER, "you are here", ch::bold(11.0), ch::ink(kit::primary()));
                }
            }
        });
    }

    fn profile_options(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            card_head(ui, "🔔", ORANGE, "Pop-ups", None);
            ui.add_space(12.0);
            self.dirty |= kit::toggle_row(ui, "Achievement pop-ups", "A little pop-up on the overlay when you unlock an achievement or level up.", &mut self.cfg.toasts);
            ui.add_space(10.0);
            divider(ui);
            ui.add_space(10.0);
            self.dirty |= kit::toggle_row(ui, "Show my level on the overlay", "A small level badge next to the overlay title.", &mut self.cfg.level_on_overlay);
            ui.add_space(10.0);
            if ui.add(egui::Button::new("Preview a pop-up").fill(kit::btn_fill())).clicked() {
                let was = self.cfg.toasts;
                self.cfg.toasts = true;
                self.push_toast(ToastKind::Achievement { name: "Show Off".into(), desc: "This is what a pop-up looks like".into(), tier: Tier::Gold });
                self.cfg.toasts = was;
                self.cfg.mini = false;
                self.play(crate::sound::Sfx::Achievement);
            }
        });
    }

    // ───────────────────────────── achievements ─────────────────────────────

    fn profile_achievements(&mut self, ui: &mut egui::Ui, l: &Lifetime) {
        let got = |a: &Achievement| self.cfg.achievements.iter().any(|u| u.id == a.id);
        // summary: a ring and the tiers
        let total = ACHIEVEMENTS.len();
        let done = ACHIEVEMENTS.iter().filter(|a| got(a)).count();
        let ach_xp: u64 = ACHIEVEMENTS.iter().filter(|a| got(a)).map(|a| a.tier.xp()).sum();
        kit::card(ui, |ui| {
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
            let p = ui.painter();
            let c = pos2(r.min.x + 48.0, r.center().y);
            let frac = done as f32 / total.max(1) as f32;
            let g = kit::grow(ui.ctx(), egui::Id::new("ach-ring"), frac);
            p.circle_stroke(c, 40.0, Stroke::new(9.0f32, theme::blend(kit::CARD(), TEXT(), 0.09)));
            arc(p, c, 40.0, g, Stroke::new(9.0f32, kit::primary()));
            p.text(c - vec2(0.0, 6.0), Align2::CENTER_CENTER, format!("{:.0}%", frac * 100.0), ch::heavy(18.0), TEXT());
            p.text(c + vec2(0.0, 13.0), Align2::CENTER_CENTER, format!("{done}/{total}"), ch::reg(11.0), MUTED());
            let x = r.min.x + 112.0;
            p.text(pos2(x, r.min.y + 18.0), Align2::LEFT_CENTER, format!("{done} of {total} unlocked"), ch::bold(16.0), TEXT());
            p.text(pos2(x, r.min.y + 40.0), Align2::LEFT_CENTER, format!("{} XP from achievements", theme::thousands(ach_xp)), ch::reg(12.5), MUTED());
            // tiers
            let tw = (r.max.x - x) / 4.0;
            for (k, t) in [Tier::Bronze, Tier::Silver, Tier::Gold, Tier::Platinum].into_iter().enumerate() {
                let all = ACHIEVEMENTS.iter().filter(|a| a.tier == t).count();
                let n = ACHIEVEMENTS.iter().filter(|a| a.tier == t && got(a)).count();
                let tx = x + tw * k as f32;
                let tc = tier_col(t);
                theme::medal(p, pos2(tx + 9.0, r.min.y + 70.0), 8.0, t.rgb(), n == 0);
                p.text(pos2(tx + 24.0, r.min.y + 70.0), Align2::LEFT_CENTER, t.label(), ch::bold(12.0), ch::ink(tc));
                p.text(pos2(tx + tw - 18.0, r.min.y + 70.0), Align2::RIGHT_CENTER, format!("{n} / {all}"), ch::reg(11.5), MUTED());
                let bar = Rect::from_min_size(pos2(tx, r.min.y + 86.0), vec2(tw - 18.0, 5.0));
                p.rect_filled(bar, 2.5, theme::blend(kit::CARD(), TEXT(), 0.09));
                let f = kit::grow(ui.ctx(), egui::Id::new(("tier-bar", k)), n as f32 / all.max(1) as f32);
                if f > 0.0 {
                    p.rect_filled(Rect::from_min_size(bar.min, vec2((bar.width() * f).max(5.0), 5.0)), 2.5, tc);
                }
            }
        });
        ui.add_space(14.0);

        // categories
        let mut tab = self.setup.ach_tab.min(Category::ALL.len());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            if cat_chip(ui, "★", "All", done, total, tab == 0).clicked() {
                tab = 0;
            }
            for (i, c) in Category::ALL.iter().enumerate() {
                let all = ACHIEVEMENTS.iter().filter(|a| a.cat == *c).count();
                let n = ACHIEVEMENTS.iter().filter(|a| a.cat == *c && got(a)).count();
                if cat_chip(ui, cat_icon(*c), c.label(), n, all, tab == i + 1).clicked() {
                    tab = i + 1;
                }
            }
        });
        self.setup.ach_tab = tab;
        ui.add_space(12.0);

        // search, filter, sort
        let rest_id = egui::Id::new("ach-tools-w");
        let rest_w = ui.ctx().data(|d| d.get_temp::<f32>(rest_id)).unwrap_or(380.0);
        let mut query = std::mem::take(&mut self.setup.ach_query);
        let mut filter = self.setup.ach_filter;
        let mut sort = self.setup.ach_sort.min(3);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let sw = (ui.available_width() - rest_w - 70.0).clamp(140.0, 320.0);
            kit::search_box(ui, &mut query, "Search achievements", sw);
            ui.add_space((ui.available_width() - rest_w).max(8.0));
            let start = ui.cursor().min.x;
            kit::segmented(ui, &mut filter, &[(AchFilter::All, "All"), (AchFilter::Unlocked, "Unlocked"), (AchFilter::Locked, "Locked")]);
            kit::Dropdown::from_id_salt("ach-sort").selected_text(SORTS[sort as usize]).width(150.0).show_ui(ui, |ui| {
                for (i, s) in SORTS.iter().enumerate() {
                    ui.selectable_value(&mut sort, i as u8, *s);
                }
            });
            let used = ui.min_rect().max.x - start;
            if (used - rest_w).abs() > 0.5 {
                ui.ctx().data_mut(|d| d.insert_temp(rest_id, used));
                ui.ctx().request_repaint();
            }
        });
        self.setup.ach_query = query;
        self.setup.ach_filter = filter;
        self.setup.ach_sort = sort;
        ui.add_space(14.0);

        // the list
        let q = self.setup.ach_query.trim().to_lowercase();
        let mut list: Vec<&'static Achievement> = ACHIEVEMENTS
            .iter()
            .filter(|a| tab == 0 || a.cat == Category::ALL[tab - 1])
            .filter(|a| match filter {
                AchFilter::All => true,
                AchFilter::Unlocked => got(a),
                AchFilter::Locked => !got(a),
            })
            .filter(|a| q.is_empty() || ((!a.secret || got(a)) && (a.name.to_lowercase().contains(&q) || a.desc.to_lowercase().contains(&q))))
            .collect();
        let frac = |a: &Achievement| {
            let (c, t) = progress::progress(a, l);
            (c / t).clamp(0.0, 1.0)
        };
        match sort {
            1 => list.sort_by(|a, b| got(a).cmp(&got(b)).then(frac(b).total_cmp(&frac(a)))),
            2 => list.sort_by(|a, b| self.ach_when(b.id).unwrap_or(0).cmp(&self.ach_when(a.id).unwrap_or(0))),
            3 => list.sort_by(|a, b| b.tier.cmp(&a.tier)),
            _ => {}
        }
        if list.is_empty() {
            kit::card(ui, |ui| {
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    let msg = if !q.is_empty() {
                        "No achievements match that search."
                    } else if filter == AchFilter::Unlocked {
                        "None unlocked here yet. Keep hunting!"
                    } else {
                        "All done here!"
                    };
                    ui.label(RichText::new(msg).size(13.0).color(MUTED()));
                });
                ui.add_space(8.0);
            });
            return;
        }
        let gap = 12.0;
        let full = ui.available_width();
        let cols = ((full + gap) / (330.0 + gap)).floor().clamp(1.0, 4.0) as usize;
        let tw = ((full - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for a in row {
                    self.ach_tile(ui, a, l, tw);
                }
            });
            ui.add_space(gap);
        }
    }

    /// One achievement: its Pokémon (shiny when unlocked, a shadow while
    /// locked), name, tier, what to do and progress or when you got it.
    fn ach_tile(&mut self, ui: &mut egui::Ui, a: &'static Achievement, l: &Lifetime, w: f32) {
        let when = self.ach_when(a.id);
        let done = when.is_some();
        let tc = tier_col(a.tier);
        let (r, resp) = ui.allocate_exact_size(vec2(w, 112.0), Sense::hover());
        let lift = kit::anim(ui.ctx(), resp.id.with("lift"), resp.hovered(), kit::QUICK);
        let r = r.translate(vec2(0.0, -2.0 * lift));
        let p = ui.painter_at(r.expand(10.0));
        if lift > 0.0 {
            p.add(kit::shadow_tier(2).as_shape(r, 20.0));
        }
        p.rect_filled(r, 20.0, if done { theme::blend(kit::CARD(), tc, 0.07) } else { kit::CARD() });
        p.rect_stroke(r, 20.0, Stroke::new(1.0f32, if done { theme::blend(kit::LINE(), tc, 0.55) } else { kit::LINE() }));
        // the Pokémon
        let sc = pos2(r.min.x + 50.0, r.center().y);
        p.circle_filled(sc, 36.0, if done { ch::faint(tc, 0.2) } else { theme::blend(kit::CARD(), TEXT(), 0.05) });
        if let Some(sp) = self.species.iter().find(|s| s.slug == progress::mascot(a.id)).cloned() {
            let tint = if done { Color32::WHITE } else { Color32::from_rgba_unmultiplied(10, 8, 18, 230) };
            self.draw_sprite_tinted(&p, &sp, true, sc + vec2(0.0, 28.0), vec2(62.0, 58.0), tint);
        }
        if done {
            theme::sparkle(&p, sc + vec2(28.0, -26.0), 6.0, GOLD);
        }
        theme::medal(&p, sc + vec2(-26.0, 26.0), 8.5, a.tier.rgb(), !done);
        // text
        let x = r.min.x + 100.0;
        let room = r.max.x - 14.0 - x;
        let hidden = a.secret && !done;
        let name = if hidden { "???" } else { a.name };
        p.text(pos2(x, r.min.y + 22.0), Align2::LEFT_CENTER, ch::clip(ui, name, &ch::bold(14.0), room), ch::bold(14.0), if done { TEXT() } else { theme::blend(MUTED(), TEXT(), 0.6) });
        let chip = format!("{} · +{} XP", a.tier.label().to_uppercase(), a.tier.xp());
        let g = p.layout_no_wrap(chip, ch::bold(9.5), ch::ink(tc));
        let cr = Rect::from_min_size(pos2(x, r.min.y + 34.0), vec2(g.size().x + 12.0, 17.0));
        p.rect_filled(cr, 8.5, ch::faint(tc, if done { 0.25 } else { 0.12 }));
        p.galley(pos2(cr.min.x + 6.0, cr.center().y - g.size().y / 2.0), g, ch::ink(tc));
        // the ladder this belongs to
        let series = progress::series(progress::series_of(a.id));
        if series.len() > 1 {
            let i = series.iter().position(|s| s.id == a.id).unwrap_or(0);
            p.text(pos2(cr.max.x + 8.0, cr.center().y), Align2::LEFT_CENTER, format!("step {} of {}", i + 1, series.len()), ch::reg(10.5), MUTED());
        }
        // the title it earns
        if let Some((_, t)) = progress::ACH_TITLES.iter().find(|(id, _)| *id == a.id) {
            if !hidden {
                let label = format!("Title: {t}");
                let f = ch::bold(10.5);
                let g = p.layout_no_wrap(label, f, ch::ink(GOLD));
                let tr = Rect::from_min_size(pos2(r.max.x - 14.0 - g.size().x - 12.0, cr.min.y), vec2(g.size().x + 12.0, 17.0));
                p.rect_filled(tr, 8.5, ch::faint(GOLD, if done { 0.25 } else { 0.1 }));
                p.galley(pos2(tr.min.x + 6.0, tr.center().y - g.size().y / 2.0), g, ch::ink(GOLD));
            }
        }
        let desc = if hidden { "A secret. Keep hunting!" } else { a.desc };
        let job = egui::text::LayoutJob {
            wrap: egui::text::TextWrapping { max_width: room, max_rows: 2, break_anywhere: false, overflow_character: Some('…') },
            ..egui::text::LayoutJob::single_section(desc.to_string(), egui::TextFormat::simple(ch::reg(11.5), MUTED()))
        };
        let gd = ui.fonts(|f| f.layout_job(job));
        p.galley(pos2(x, r.min.y + 58.0), gd, MUTED());
        let by = r.max.y - 14.0;
        if let Some(t) = when {
            p.text(pos2(x, by), Align2::LEFT_CENTER, format!("✔ Unlocked {}", fmt_date(t)), ch::bold(11.0), ch::ink(GREEN));
        } else if !progress::is_binary(a) {
            let (c, t) = progress::progress(a, l);
            let f = (c / t).clamp(0.0, 1.0) as f32;
            let fmt = |v: f64| if matches!(a.goal, progress::Goal::HuntHours(_)) { format!("{v:.1}h") } else { theme::thousands(v as u64) };
            let label = format!("{} / {}", fmt(c.min(t)), fmt(t));
            let lw = p.layout_no_wrap(label.clone(), ch::reg(10.5), MUTED()).size().x;
            let bar = Rect::from_min_size(pos2(x, by - 2.5), vec2((room - lw - 10.0).max(30.0), 5.0));
            p.rect_filled(bar, 2.5, theme::blend(kit::CARD(), TEXT(), 0.09));
            if f > 0.0 {
                p.rect_filled(Rect::from_min_size(bar.min, vec2((bar.width() * f).max(5.0), 5.0)), 2.5, tc);
            }
            p.text(pos2(r.max.x - 14.0, by), Align2::RIGHT_CENTER, label, ch::reg(10.5), MUTED());
        } else {
            p.text(pos2(x, by), Align2::LEFT_CENTER, "Not yet", ch::reg(11.0), MUTED());
        }
    }
}

/// A card's title row; with `link`, a link on the right (true when clicked).
pub(super) fn card_head(ui: &mut egui::Ui, icon: &str, col: Color32, title: &str, link: Option<&str>) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        kit::icon_chip(ui.painter(), r, icon, col, false);
        ui.add_space(4.0);
        ui.label(RichText::new(title).font(ch::bold(16.0)).color(TEXT()));
        if let Some(t) = link {
            let w = ui.painter().layout_no_wrap(t.to_string(), ch::reg(12.5), TEXT()).size().x;
            ui.add_space((ui.available_width() - w - 2.0).max(8.0));
            clicked = ui.link(RichText::new(t).size(12.5)).clicked();
        }
    });
    clicked
}

fn divider(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, kit::LINE()));
}

fn tier_dot(p: &egui::Painter, at: egui::Pos2, t: Tier) {
    let tc = tier_col(t);
    let g = p.layout_no_wrap(t.label().to_uppercase(), ch::bold(9.5), ch::ink(tc));
    let r = Rect::from_min_size(pos2(at.x, at.y - 8.5), vec2(g.size().x + 12.0, 17.0));
    p.rect_filled(r, 8.5, ch::faint(tc, 0.18));
    p.galley(pos2(r.min.x + 6.0, r.center().y - g.size().y / 2.0), g, ch::ink(tc));
}

/// Part of a circle from the top, clockwise, `f` of the way round.
fn arc(p: &egui::Painter, c: egui::Pos2, r: f32, f: f32, stroke: Stroke) {
    if f <= 0.0 {
        return;
    }
    let n = (64.0 * f).ceil().max(2.0) as usize;
    let pts: Vec<egui::Pos2> = (0..=n)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * f * i as f32 / n as f32;
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    p.add(egui::Shape::line(pts, stroke));
}

/// A category pill: icon, name and how many are done.
fn cat_chip(ui: &mut egui::Ui, icon: &str, label: &str, n: usize, all: usize, on: bool) -> egui::Response {
    let count = format!("{n}/{all}");
    let g = ui.painter().layout_no_wrap(label.to_string(), ch::bold(12.5), TEXT());
    let gc = ui.painter().layout_no_wrap(count.clone(), ch::bold(10.5), TEXT());
    let w = 34.0 + g.size().x + 8.0 + gc.size().x + 16.0 + 12.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id, resp.hovered(), kit::QUICK);
    let p = ui.painter();
    let full = n == all && all > 0;
    if on {
        p.add(kit::primary_shape(r, false));
    } else {
        p.rect_filled(r, 17.0, theme::blend(kit::CARD(), TEXT(), 0.03 + 0.04 * ht));
        p.rect_stroke(r, 17.0, Stroke::new(1.0f32, kit::LINE()));
    }
    let ink = if on { kit::on_primary() } else { theme::blend(MUTED(), TEXT(), 0.5 + 0.5 * ht) };
    p.text(pos2(r.min.x + 20.0, r.center().y), Align2::CENTER_CENTER, icon, egui::FontId::proportional(14.0), ink);
    p.galley(pos2(r.min.x + 34.0, r.center().y - g.size().y / 2.0), g, ink);
    let cr = Rect::from_min_size(pos2(r.max.x - 12.0 - gc.size().x - 12.0, r.center().y - 9.0), vec2(gc.size().x + 12.0, 18.0));
    let (cf, ci) = if full { (GOLD, Color32::from_rgb(40, 30, 0)) } else if on { (kit::on_primary().gamma_multiply(0.22), kit::on_primary()) } else { (theme::blend(kit::CARD(), TEXT(), 0.1), MUTED()) };
    p.rect_filled(cr, 9.0, cf);
    p.text(cr.center(), Align2::CENTER_CENTER, count, ch::bold(10.5), ci);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

impl App {
    /// The title shown with your name: the one you picked, or your level's.
    pub(super) fn player_title(&self, level: u32) -> String {
        let ids: Vec<String> = self.cfg.achievements.iter().map(|u| u.id.clone()).collect();
        progress::shown_title(&self.cfg.title_choice, level, &ids, &self.pass_titles())
    }

    /// The title as a drop-down of every title you've earned.
    fn title_picker(&mut self, ui: &mut egui::Ui, level: u32, col: Color32) {
        let ids: Vec<String> = self.cfg.achievements.iter().map(|u| u.id.clone()).collect();
        let earned = progress::earned_titles(level, &ids, &self.pass_titles());
        let shown = self.player_title(level);
        let total = progress::LEVEL_TITLES.len() + progress::ACH_TITLES.len() + crate::pass::all_rewards().filter(|r| r.kind == crate::pass::Kind2::Title).count();
        let mut pick: Option<String> = None;
        kit::Dropdown::from_id_salt("profile-title")
            .selected_text(RichText::new(&shown).font(ch::bold(15.0)).color(super::pass_page::title_col(&shown, ch::ink(col))))
            .height(420.0)
            .width(0.0)
            .show_ui(ui, |ui| {
                ui.set_min_width(340.0);
                let auto = self.cfg.title_choice.is_empty();
                ui.horizontal(|ui| {
                    if ui.selectable_label(auto, RichText::new("Follow my level").strong()).on_hover_text("Changes by itself as you level up").clicked() {
                        pick = Some(String::new());
                    }
                    ui.label(RichText::new(progress::title(level)).size(11.5).color(MUTED()));
                });
                ui.separator();
                let mut last_level = true;
                let mut shown_pass = false;
                for (t, how) in &earned {
                    let is_level = how.starts_with("Level ");
                    if last_level && !is_level {
                        ui.separator();
                        ui.label(RichText::new("FROM ACHIEVEMENTS").size(10.5).strong().color(MUTED()));
                    }
                    if how == "Battle pass" && !shown_pass {
                        shown_pass = true;
                        ui.separator();
                        ui.label(RichText::new("FROM THE BATTLE PASS").size(10.5).strong().color(MUTED()));
                    }
                    last_level = is_level;
                    ui.horizontal(|ui| {
                        let label = match crate::pass::title_theme(t) {
                            Some(_) => RichText::new(*t).strong().color(super::pass_page::title_col(t, TEXT())),
                            None => RichText::new(*t),
                        };
                        if ui.selectable_label(!auto && shown == *t, label).clicked() {
                            pick = Some(t.to_string());
                        }
                        ui.label(RichText::new(how).size(11.5).color(MUTED()));
                    });
                }
                ui.separator();
                ui.label(RichText::new(format!("{} of {total} titles earned · unlock achievements for more", earned.len())).size(11.5).color(MUTED()));
            })
            .response
            .on_hover_text("Pick the title shown with your name");
        if let Some(t) = pick {
            self.cfg.title_choice = t;
            self.dirty = true;
        }
    }
}
