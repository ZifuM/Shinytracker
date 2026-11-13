//! Home in the modern look: a dashboard. The current hunt up top as a hero
//! card (with the one action that matters most: Start overlay), today's
//! numbers as tiles, the other apps, recent shinies and today's rare grass.

use super::shell::tab_meta;
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::stats::fmt_duration;
use crate::theme::{self, GOLD};
use eframe::egui::{self, Align2, Color32, Rect, RichText, Sense, Stroke};

impl App {
    pub(super) fn page_home_modern(&mut self, ui: &mut egui::Ui) {
        kit::page_header(ui, "HOME", &self.greeting());
        self.home_notices(ui);
        // the hunt you're on: the card from before, front and centre
        self.hunt_hero(ui, true);
        ui.add_space(16.0);
        self.home_today(ui);
        ui.add_space(18.0);

        let avail = ui.available_width();
        let wide = avail >= 980.0;
        let gap = 18.0;
        if wide {
            let left_w = ((avail - gap) * 0.6).floor();
            let right_w = avail - gap - left_w;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(egui::vec2(left_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(left_w);
                    if self.home_runs(ui) {
                        ui.add_space(gap);
                    }
                    self.home_rare(ui);
                });
                ui.allocate_ui_with_layout(egui::vec2(right_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(right_w);
                    self.home_almost(ui);
                    ui.add_space(gap);
                    if self.home_breeding(ui) {
                        ui.add_space(gap);
                    }
                    self.recent_shinies(ui);
                });
            });
            ui.add_space(gap);
            self.home_jump(ui);
        } else {
            if self.home_runs(ui) {
                ui.add_space(gap);
            }
            self.home_almost(ui);
            ui.add_space(gap);
            if self.home_breeding(ui) {
                ui.add_space(gap);
            }
            self.recent_shinies(ui);
            ui.add_space(gap);
            self.home_rare(ui);
            ui.add_space(gap);
            self.home_jump(ui);
        }
    }

    /// Encounters per day over every run (day → encounters).
    fn home_days(&self) -> std::collections::BTreeMap<i64, u64> {
        let mut m = std::collections::BTreeMap::new();
        let mut add = |st: &crate::stats::RunStats| {
            for d in &st.days {
                *m.entry(d.day).or_insert(0) += d.encounters;
            }
        };
        add(&self.cfg.run_stats);
        for r in self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id) {
            add(&r.stats);
        }
        for h in self.cfg.history.iter().filter(|h| !h.phase) {
            if let Some(st) = &h.stats {
                add(st);
            }
        }
        m
    }

    /// Today in four tiles: what you've done, your goal, your streak and your
    /// trainer level, each with a little picture of where it stands.
    fn home_today(&mut self, ui: &mut egui::Ui) {
        let days = self.home_days();
        let today_n = crate::stats::local_day(crate::stats::now_unix());
        let (today, _) = self.day_totals();
        let week: Vec<u64> = (0..7).map(|k| days.get(&(today_n - 6 + k)).copied().unwrap_or(0)).collect();
        let goal = self.cfg.daily_goal;
        let streak = self.daily_streak();
        let l = self.lifetime_full();
        let xp = self.total_xp(&l).total();
        let level = crate::progress::level_for(xp);
        let (into, need) = crate::progress::level_progress(xp);
        let avail = ui.available_width();
        let cols = if avail >= 860.0 { 4 } else { 2 };
        let gap = 12.0;
        let tw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut open: Option<usize> = None;
        let idx: Vec<usize> = (0..4).collect();
        for row in idx.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &i in row {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(tw, 112.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("lift"), resp.hovered(), kit::QUICK);
                    let r = r.translate(egui::vec2(0.0, -2.0 * ht));
                    let p = ui.painter_at(r.expand(14.0));
                    let (icon, col, label) = [("⚔", kit::C_VIOLET, "Today"), ("🎯", kit::C_PINK, "Daily goal"), ("🔥", kit::C_ORANGE, "Streak"), ("⭐", kit::C_GOLD, "Trainer level")][i];
                    p.add(kit::shadow_tier(if ht > 0.0 { 2 } else { 1 }).as_shape(r, 16.0));
                    p.rect_filled(r, 16.0, CARD());
                    p.rect_stroke(r, 16.0, Stroke::new(1.0f32, if resp.hovered() { theme::blend(LINE(), col, 0.6) } else { LINE() }));
                    let chip = Rect::from_min_size(r.min + egui::vec2(14.0, 14.0), egui::vec2(28.0, 28.0));
                    kit::icon_chip(&p, chip, icon, col, false);
                    p.text(egui::pos2(chip.max.x + 10.0, chip.center().y), Align2::LEFT_CENTER, label, crate::fonts::sans_font(12.5), MUTED());
                    let vy = chip.max.y + 22.0;
                    let big = crate::fonts::sans_bold_font(22.0);
                    let small = crate::fonts::sans_font(11.5);
                    // the picture on the right of each tile
                    let art = Rect::from_min_max(egui::pos2(r.max.x - (tw * 0.42).min(150.0), r.min.y + 16.0), egui::pos2(r.max.x - 14.0, r.max.y - 14.0));
                    match i {
                        0 => {
                            p.text(egui::pos2(r.min.x + 16.0, vy), Align2::LEFT_CENTER, theme::thousands(today), big, TEXT());
                            p.text(egui::pos2(r.min.x + 16.0, r.max.y - 18.0), Align2::LEFT_CENTER, format!("encounters · {}", fmt_duration(self.today_hunt_secs())), small, MUTED());
                            // the last 7 days, today brightest
                            let max = week.iter().copied().max().unwrap_or(1).max(1) as f32;
                            let slot = art.width() / 7.0;
                            let bw = (slot * 0.56).min(12.0);
                            for (k, v) in week.iter().enumerate() {
                                let cx = art.min.x + slot * (k as f32 + 0.5);
                                let h = ((art.height() - 8.0) * (*v as f32 / max)).max(3.0);
                                let br = Rect::from_min_max(egui::pos2(cx - bw / 2.0, art.max.y - h), egui::pos2(cx + bw / 2.0, art.max.y));
                                p.rect_filled(br, bw / 2.0, if k == 6 { col } else { theme::blend(CARD(), col, if *v > 0 { 0.45 } else { 0.15 }) });
                            }
                        }
                        1 => {
                            let (v, note, f) = if goal > 0 {
                                let f = (today as f32 / goal as f32).min(1.0);
                                (format!("{} / {}", theme::thousands(today.min(goal)), theme::thousands(goal)), if f >= 1.0 { "Goal reached ✔".to_string() } else { format!("{} to go", theme::thousands(goal - today)) }, f)
                            } else {
                                ("Off".to_string(), "Set one on your Profile".to_string(), 0.0)
                            };
                            p.text(egui::pos2(r.min.x + 16.0, vy), Align2::LEFT_CENTER, v, big, TEXT());
                            p.text(egui::pos2(r.min.x + 16.0, r.max.y - 18.0), Align2::LEFT_CENTER, note, small, if f >= 1.0 { theme::GREEN } else { MUTED() });
                            // a ring
                            let c = egui::pos2(art.max.x - 30.0, art.center().y + 4.0);
                            p.circle_stroke(c, 24.0, Stroke::new(6.0f32, theme::blend(CARD(), TEXT(), 0.1)));
                            let g = kit::grow(ui.ctx(), resp.id.with("ring"), f);
                            if g > 0.0 {
                                let n = (g * 48.0).ceil().max(2.0) as usize;
                                let pts: Vec<egui::Pos2> = (0..=n).map(|k| {
                                    let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * g * k as f32 / n as f32;
                                    c + egui::vec2(a.cos(), a.sin()) * 24.0
                                }).collect();
                                p.add(egui::Shape::line(pts, Stroke::new(6.0f32, if f >= 1.0 { theme::GREEN } else { col })));
                            }
                            p.text(c, Align2::CENTER_CENTER, format!("{:.0}%", f * 100.0), crate::fonts::sans_bold_font(11.0), TEXT());
                        }
                        2 => {
                            p.text(egui::pos2(r.min.x + 16.0, vy), Align2::LEFT_CENTER, format!("{streak} day{}", if streak == 1 { "" } else { "s" }), big, TEXT());
                            let hunted_today = week[6] > 0;
                            let note = if streak == 0 { "Hunt today to start one".to_string() } else if hunted_today { "Kept going today".to_string() } else { "Hunt today to keep it".to_string() };
                            p.text(egui::pos2(r.min.x + 16.0, r.max.y - 18.0), Align2::LEFT_CENTER, note, small, if !hunted_today && streak > 0 { kit::C_ORANGE } else { MUTED() });
                            // a dot for each of the last 7 days
                            let slot = art.width() / 7.0;
                            for (k, v) in week.iter().enumerate() {
                                let c = egui::pos2(art.min.x + slot * (k as f32 + 0.5), art.center().y + 8.0);
                                if *v > 0 {
                                    p.circle_filled(c, 6.0, col);
                                } else {
                                    p.circle_stroke(c, 5.5, Stroke::new(1.5f32, theme::blend(CARD(), TEXT(), 0.18)));
                                }
                                let wd = crate::stats::WEEKDAYS[crate::stats::weekday(today_n - 6 + k as i64)];
                                p.text(c + egui::vec2(0.0, 15.0), Align2::CENTER_CENTER, &wd[..1], crate::fonts::sans_font(9.5), if k == 6 { TEXT() } else { MUTED() });
                            }
                        }
                        _ => {
                            p.text(egui::pos2(r.min.x + 16.0, vy), Align2::LEFT_CENTER, level.to_string(), big, TEXT());
                            p.text(egui::pos2(r.min.x + 16.0, r.max.y - 18.0), Align2::LEFT_CENTER, crate::progress::title(level), small, MUTED());
                            // XP towards the next level
                            let f = (into as f32 / need.max(1) as f32).clamp(0.0, 1.0);
                            let bar = Rect::from_min_max(egui::pos2(art.min.x, art.center().y + 2.0), egui::pos2(art.max.x, art.center().y + 10.0));
                            kit::paint_bar(&p, bar, kit::grow(ui.ctx(), resp.id.with("xp"), f));
                            p.text(egui::pos2(art.max.x, bar.min.y - 10.0), Align2::RIGHT_CENTER, format!("{} / {} XP", theme::thousands(into), theme::thousands(need)), crate::fonts::sans_font(10.5), MUTED());
                        }
                    }
                    let tip = match i {
                        0 => "Your current hunt's stats",
                        1 => "Your goals",
                        2 => "Your goals and streaks",
                        _ => "Your Trainer Profile",
                    };
                    if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some(i);
                    }
                }
            });
            ui.add_space(gap);
        }
        if let Some(i) = open {
            self.setup.scroll_top = true;
            match i {
                0 => {
                    self.go_to(Tab::Runs);
                    self.setup.view_run = Some(super::RunRef::Current);
                    self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
                }
                1 | 2 => {
                    self.go_to(Tab::Profile);
                    self.setup.profile_section = super::trainer::ProfileSection::Goals;
                }
                _ => {
                    self.go_to(Tab::Profile);
                    self.setup.profile_section = super::trainer::ProfileSection::Overview;
                }
            }
        }
    }

    /// Your other hunts, most recent first, each a click from resuming.
    /// Returns false (and draws nothing) when there are none.
    fn home_runs(&mut self, ui: &mut egui::Ui) -> bool {
        let mut runs: Vec<(u64, u64)> = self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id).map(|r| (r.id, r.stats.last_unix.max(r.saved_unix))).collect();
        if runs.is_empty() {
            return false;
        }
        runs.sort_by(|a, b| b.1.cmp(&a.1));
        let total = runs.len();
        let mut resume: Option<u64> = None;
        let mut all = false;
        kit::card(ui, |ui| {
            all = super::profile_page::card_head(ui, "📜", kit::C_GREEN, "Pick up a hunt", Some(&format!("All runs ({total}) ›")));
            ui.add_space(8.0);
            for (k, (id, last)) in runs.iter().take(3).enumerate() {
                let Some(info) = self.run_info(super::RunRef::Saved(*id)) else { continue };
                if k > 0 {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0f32, LINE()));
                }
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 64.0), Sense::hover());
                let p = ui.painter_at(r.expand(2.0));
                if resp.hovered() {
                    p.rect_filled(r.expand2(egui::vec2(6.0, 0.0)), 12.0, kit::CARD_HOVER());
                }
                let mult = info.odds.filter(|o| *o > 1.0).map(|o| info.count as f64 / o);
                let col = match info.odds.filter(|o| *o > 1.0) {
                    Some(o) => super::charts::luck_col(crate::stats::found_by(info.count, o)),
                    None => MUTED(),
                };
                let sc = egui::pos2(r.min.x + 28.0, r.center().y);
                p.circle_filled(sc, 26.0, theme::blend(CARD(), col, 0.16));
                if let Some((_, sp)) = self.species_of(info.slug.as_deref()) {
                    self.draw_sprite_centered(&p, &sp, false, sc, egui::vec2(48.0, 48.0));
                }
                let x = r.min.x + 66.0;
                p.text(egui::pos2(x, r.center().y - 10.0), Align2::LEFT_CENTER, &info.name, crate::fonts::sans_bold_font(14.5), TEXT());
                p.text(egui::pos2(x, r.center().y + 11.0), Align2::LEFT_CENTER, format!("{} · {}", info.method, ago(*last)), crate::fonts::sans_font(11.5), MUTED());
                // the button, then the count and luck to its left
                let bw = 96.0;
                let br = Rect::from_min_size(egui::pos2(r.max.x - bw, r.center().y - 16.0), egui::vec2(bw, 32.0));
                let b = ui.put(br, egui::Button::new(RichText::new("▶  Resume").font(crate::fonts::sans_bold_font(12.5)).color(kit::on_primary())).fill(kit::primary()).rounding(16.0));
                if b.on_hover_text("Switch to this hunt (the one you're on is saved)").clicked() {
                    resume = Some(*id);
                }
                let nx = br.min.x - 16.0;
                p.text(egui::pos2(nx, r.center().y - 9.0), Align2::RIGHT_CENTER, theme::thousands(info.count), crate::fonts::sans_bold_font(16.0), TEXT());
                if let Some(m) = mult {
                    p.text(egui::pos2(nx, r.center().y + 11.0), Align2::RIGHT_CENTER, format!("{m:.2}× the odds"), crate::fonts::sans_font(11.5), super::charts::ink(col));
                } else {
                    p.text(egui::pos2(nx, r.center().y + 11.0), Align2::RIGHT_CENTER, "encounters", crate::fonts::sans_font(11.5), MUTED());
                }
            }
        });
        if let Some(id) = resume {
            self.resume_run(id);
        }
        if all {
            self.go_to(Tab::Runs);
            self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
        }
        true
    }

    /// Today's rare grass, with a link to the Map.
    fn home_rare(&mut self, ui: &mut egui::Ui) {
        let head = |ui: &mut egui::Ui, n: usize| -> bool {
            let link = if n > 0 { format!("{n} spot{} on the Map ›", if n == 1 { "" } else { "s" }) } else { "Open the Map ›".to_string() };
            super::profile_page::card_head(ui, "🌿", kit::C_GREEN, "Rare grass today", Some(&link))
        };
        if self.rare_home_titled(ui, Some(&head)) {
            self.go_to(Tab::Map);
        }
    }

    /// What's going on in the Breeding Planner, when something is. Returns
    /// whether it drew anything.
    fn home_breeding(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(w) = self.wiki.clone() else { return false };
        let f = self.setup.breed.file().clone();
        let pair = !f.daycare.a.species.is_empty() && !f.daycare.b.species.is_empty();
        if !pair && f.target.species.is_empty() {
            return false;
        }
        let mut go = false;
        kit::card(ui, |ui| {
            go = super::profile_page::card_head(ui, "🐣", kit::C_PINK, "Breeding", Some("Open ›"));
            ui.add_space(6.0);
            let name = |id: &str| w.mon(id).map(|m| m.name.clone()).unwrap_or_else(|| id.to_string());
            if !f.target.species.is_empty() {
                let t = &f.target;
                let stats = t.ivs.iter().filter(|x| **x).count();
                let mut what = vec![];
                if let Some(n) = &t.nature {
                    what.push(n.clone());
                }
                if stats > 0 {
                    what.push(format!("{stats} at 31"));
                }
                if t.shiny {
                    what.push("shiny".into());
                }
                let extra = if what.is_empty() { String::new() } else { format!(" ({})", what.join(", ")) };
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::hover());
                    let p = ui.painter().clone();
                    self.team_sprite(&p, &t.species, t.shiny, r.center(), egui::vec2(38.0, 38.0));
                    ui.label(RichText::new(format!("Planning a {}{extra}", name(&t.species))).size(13.0).color(TEXT()));
                });
            }
            if pair {
                let fam = crate::breed::family_pair(&w, &f.daycare.a, &f.daycare.b);
                let mine: Vec<&crate::breed::Hatch> = f.log.iter().filter(|h| h.pair == fam).collect();
                let goal = if f.daycare.goal_set { f.daycare.goal.clone() } else { crate::breed::Goal::from_parents(&f.daycare.a, &f.daycare.b) };
                let hits = mine.iter().filter(|h| h.hits(&goal)).count();
                let shinies = mine.iter().filter(|h| h.shiny).count();
                let mut line = format!("In the Day Care: {} × {}", name(&f.daycare.a.species), name(&f.daycare.b.species));
                if !mine.is_empty() {
                    line.push_str(&format!(" · {} Egg{} logged", mine.len(), if mine.len() == 1 { "" } else { "s" }));
                    if hits > 0 {
                        line.push_str(&format!(", {hits} the one you want"));
                    }
                    if shinies > 0 {
                        line.push_str(&format!(", {shinies} shiny"));
                    }
                }
                ui.label(RichText::new(line).size(12.5).color(MUTED()));
            }
        });
        if go {
            self.go_to(Tab::Breed);
        }
        true
    }

    /// The achievements you're closest to.
    fn home_almost(&mut self, ui: &mut egui::Ui) {
        let l = self.lifetime_full();
        let near = self.near_achievements(&l);
        let mut go = false;
        kit::card(ui, |ui| {
            go = super::profile_page::card_head(ui, "🎯", kit::C_PINK, "Almost there", Some("All ›"));
            ui.add_space(6.0);
            if near.is_empty() {
                ui.label(RichText::new("Start hunting and your next achievements show up here.").size(12.5).color(MUTED()));
                return;
            }
            for (a, f) in near.iter().take(3) {
                self.near_row(ui, a, *f, &l);
            }
        });
        if go {
            self.go_to(Tab::Profile);
            self.setup.profile_section = super::trainer::ProfileSection::Achievements;
        }
    }

    /// Everything else, one click away: a row of four (two by two when narrow).
    fn home_jump(&mut self, ui: &mut egui::Ui) {
        let items: [(Tab, &str); 4] = [
            (Tab::Pokedex, "Every Pokémon, and your Shiny Dex"),
            (Tab::Map, "Areas, Pokémon and rare grass"),
            (Tab::Stats, "Your hunting over time"),
            (Tab::Runs, "Every hunt, saved and caught"),
        ];
        let mut go: Option<Tab> = None;
        kit::card(ui, |ui| {
            super::profile_page::card_head(ui, "📌", kit::C_CYAN, "Jump to", None);
            ui.add_space(10.0);
            let w = ui.available_width();
            let cols = if w >= 760.0 { 4 } else { 2 };
            let gap = 10.0;
            let tw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in items.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (tab, about) in row {
                        let (icon, name, color) = tab_meta(*tab);
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(tw, 58.0), Sense::click());
                        let p = ui.painter_at(r.expand(2.0));
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        p.rect_filled(r, 14.0, theme::blend(kit::CARD_HOVER(), color, 0.06 * ht));
                        p.rect_stroke(r, 14.0, Stroke::new(1.0f32, theme::blend(LINE(), color, 0.5 * ht)));
                        let chip = Rect::from_center_size(egui::pos2(r.min.x + 26.0, r.center().y), egui::vec2(32.0, 32.0));
                        kit::icon_chip(&p, chip, icon, color, true);
                        let tx = chip.max.x + 12.0;
                        let maxw = r.max.x - tx - 26.0;
                        p.text(egui::pos2(tx, r.center().y - 8.0), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(13.5), TEXT());
                        p.text(egui::pos2(tx, r.center().y + 10.0), Align2::LEFT_CENTER, super::charts::clip(ui, about, &crate::fonts::sans_font(11.5), maxw), crate::fonts::sans_font(11.5), MUTED());
                        p.text(egui::pos2(r.max.x - 14.0, r.center().y), Align2::RIGHT_CENTER, "›", egui::FontId::proportional(18.0), if ht > 0.5 { TEXT() } else { MUTED() });
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            go = Some(*tab);
                        }
                    }
                });
                ui.add_space(gap);
            }
        });
        if let Some(t) = go {
            if t == Tab::Runs {
                self.setup.nav_collapsed.retain(|x| *x != Tab::Shiny);
            }
            self.go_to(t);
        }
    }

    /// "Good evening, Zifu."
    fn greeting(&self) -> String {
        let hour = crate::stats::local_hour(crate::config::now_unix());
        let part = match hour {
            5..=11 => "Good morning",
            12..=17 => "Good afternoon",
            _ => "Good evening",
        };
        let name = self.cfg.trainer_name.trim();
        if name.is_empty() {
            format!("{part}! Here's your hunt and today at a glance.")
        } else {
            format!("{part}, {name}! Here's your hunt and today at a glance.")
        }
    }

    /// The current hunt as a big card: target, count, odds, Start overlay.
    /// `home`: with the "Open Shiny Hunter" button. The card grows to fit
    /// its contents (fonts and window sizes differ), with a minimum height.
    pub(super) fn hunt_hero(&mut self, ui: &mut egui::Ui, home: bool) {
        let w = ui.available_width();
        let pl = theme::pal();
        let show_sprite = w >= 560.0;
        let art_w = if show_sprite { (w * 0.3).clamp(220.0, 320.0) } else { 0.0 };
        let text_w = (w - 56.0 - art_w).max(200.0);
        let (n, _) = self.odds_count();
        let odds = self.cfg.odds_one_in();
        let method = self.cfg.odds_caption();
        let mut open_hunter = false;
        let mut change = false;
        let mut stats = false;

        // Background goes underneath once the card's size is known.
        let slot = ui.painter().add(egui::Shape::Noop);
        let frame = egui::Frame::none()
            .inner_margin(egui::Margin { left: 28.0, right: 28.0, top: 22.0, bottom: 24.0 })
            .show(ui, |ui| {
                ui.set_width(w - 56.0);
                ui.set_min_height(186.0);
                ui.allocate_ui_with_layout(egui::vec2(text_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(text_w);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label(RichText::new("CURRENT HUNT").font(crate::fonts::sans_bold_font(11.5)).color(theme::blend(TEXT(), pl.accent, 0.35)));
                    ui.add(egui::Label::new(RichText::new(self.target_label()).font(crate::fonts::sans_bold_font(28.0)).color(TEXT())).wrap_mode(egui::TextWrapMode::Truncate));
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        chip(ui, &method);
                        if let Some(o) = odds {
                            chip(ui, &format!("1 in {}", super::fmt_one_in(o)));
                        }
                        if self.overlay_on {
                            chip_col(ui, "● Counting", theme::GREEN);
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(RichText::new(theme::thousands(self.cfg.count)).font(crate::fonts::sans_bold_font(34.0)).color(TEXT()));
                        ui.label(RichText::new("encounters").size(13.5).color(MUTED()));
                    });
                    if let Some(o) = odds {
                        let mult = n as f64 / o;
                        let (label, col) = if mult < 0.9 {
                            (format!("{:.2}× the odds", mult), theme::GREEN)
                        } else if mult <= 1.1 {
                            (format!("{:.1}× the odds", mult), theme::HP_YELLOW)
                        } else {
                            (format!("{:.1}× the odds", mult), theme::HP_RED)
                        };
                        ui.horizontal(|ui| {
                            let bw = (text_w - 130.0).clamp(80.0, 300.0);
                            kit::bar(ui, bw, 6.0, (mult as f32).min(1.0));
                            ui.label(RichText::new(label).size(12.5).color(col));
                        });
                    }
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                        self.overlay_button(ui, egui::vec2(150.0, 36.0));
                        change = sec_button(ui, "🎯  Change target", "Pick a different Pokémon to hunt");
                        if home {
                            open_hunter = sec_button(ui, "Open Shiny Hunter  ›", "Hunt settings, counting, runs and the overlay");
                        } else {
                            stats = sec_button(ui, "📊  Hunt stats", "This hunt's encounters, time and pace");
                        }
                    });
                });
            });
        let rect = frame.response.rect;
        let h = rect.height();

        // Card: shadow, gradient, edge.
        let a = theme::blend(CARD(), pl.accent, if pl.light { 0.18 } else { 0.34 });
        let b = theme::blend(CARD(), if pl.gradient { pl.accent2 } else { pl.accent }, if pl.light { 0.10 } else { 0.16 });
        ui.painter().set(
            slot,
            egui::Shape::Vec(vec![
                kit::card_shadow().as_shape(rect, 18.0).into(),
                theme::gradient_rect(rect, [18.0; 4], a, b),
                egui::Shape::rect_stroke(rect, 18.0, Stroke::new(1.0f32, theme::blend(LINE(), pl.accent, 0.35))),
            ]),
        );

        // The target, big, on a soft glow at the right.
        if show_sprite {
            let pc = ui.painter_at(rect.shrink(1.0));
            let sprite_c = egui::pos2(rect.max.x - art_w / 2.0 - 12.0, rect.center().y + 4.0);
            let s = h.min(art_w);
            for (rad, alpha) in [(s * 0.6, 6u8), (s * 0.45, 8), (s * 0.31, 12)] {
                pc.circle_filled(sprite_c, rad, Color32::from_rgba_premultiplied(alpha, alpha, alpha, alpha));
            }
            match self.target_idx() {
                Some(i) => {
                    let sp = self.species[i].clone();
                    self.draw_sprite(&pc, &sp, true, sprite_c + egui::vec2(0.0, s * 0.32), egui::vec2(s * 0.78, s * 0.72));
                }
                None => {
                    pc.text(sprite_c, Align2::CENTER_CENTER, "?", kit::font_bold(56.0), MUTED());
                }
            }
            for (dx, dy, r) in [(-0.34, -0.30, 7.0), (0.30, -0.22, 5.0), (0.24, 0.30, 4.0)] {
                theme::sparkle(&pc, sprite_c + egui::vec2(s * dx, s * dy), r, GOLD);
            }
        }

        if change {
            self.open_target_picker();
        }
        if open_hunter {
            self.go_to(Tab::Shiny);
            self.setup.nav_collapsed.retain(|t| *t != Tab::Shiny);
        }
        if stats {
            self.go_to(Tab::Runs);
            self.setup.view_run = Some(super::RunRef::Current);
        }
    }

    /// The other apps as tiles.
    #[allow(dead_code)]
    fn app_tiles(&mut self, ui: &mut egui::Ui, width: f32) {
        self.page_tiles(
            ui,
            width,
            &[
                (Tab::Pokedex, "Every Pokémon: evolutions, moves and where to find them."),
                (Tab::Map, "Sinnoh from your game's map: areas, Pokémon and rare grass."),
            ],
            2,
        );
    }

    /// Pages as tiles (icon chip in the page's colour, name, what's there),
    /// up to `max_cols` across.
    pub(super) fn page_tiles(&mut self, ui: &mut egui::Ui, width: f32, apps: &[(Tab, &str)], max_cols: usize) {
        let gap = 12.0;
        let cols = ((width + gap) / (210.0 + gap)).floor().clamp(1.0, max_cols as f32) as usize;
        let tw = ((width - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut go: Option<Tab> = None;
        for row in apps.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (tab, about) in row {
                    let (icon, name, color) = tab_meta(*tab);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(tw, 104.0), Sense::click());
                    let p = ui.painter_at(rect.expand(12.0));
                    let hov = resp.hovered();
                    p.add(kit::card_shadow().as_shape(rect, 14.0));
                    p.rect_filled(rect, 14.0, if hov { kit::CARD_HOVER() } else { CARD() });
                    p.rect_stroke(rect, 14.0, Stroke::new(1.0f32, if hov { theme::blend(LINE(), color, 0.6) } else { LINE() }));
                    let chip = Rect::from_min_size(rect.min + egui::vec2(16.0, 16.0), egui::vec2(36.0, 36.0));
                    kit::icon_chip(&p, chip, icon, color, true);
                    p.text(egui::pos2(chip.max.x + 12.0, chip.center().y), Align2::LEFT_CENTER, name, crate::fonts::sans_bold_font(15.0), TEXT());
                    p.text(rect.right_top() + egui::vec2(-16.0, 34.0), Align2::RIGHT_CENTER, "›", egui::FontId::proportional(18.0), if hov { TEXT() } else { MUTED() });
                    let g = p.layout(about.to_string(), crate::fonts::sans_font(12.5), MUTED(), rect.width() - 32.0);
                    p.galley(egui::pos2(rect.min.x + 16.0, chip.max.y + 10.0), g, MUTED());
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        go = Some(*tab);
                    }
                }
            });
            ui.add_space(gap);
        }
        if let Some(t) = go {
            self.go_to(t);
        }
    }

    /// The last few shinies caught, newest first.
    fn recent_shinies(&mut self, ui: &mut egui::Ui) {
        let recent: Vec<(Option<usize>, String, u64, u64)> = self
            .cfg
            .history
            .iter()
            .rev()
            .take(6)
            .map(|h| {
                let idx = h.slug.as_deref().and_then(|s| self.species.iter().position(|sp| sp.slug == s));
                (idx, h.target.clone(), h.encounters, h.when_unix)
            })
            .collect();
        let mut dex = false;
        let mut trophy: Option<String> = None;
        let slugs: Vec<Option<String>> = self.cfg.history.iter().rev().take(6).map(|h| h.slug.clone()).collect();
        kit::card(ui, |ui| {
            dex = super::profile_page::card_head(ui, "✨", kit::C_GOLD, "Recent shinies", Some("Shiny Dex ›"));
            ui.add_space(10.0);
            if recent.is_empty() {
                ui.label(RichText::new("No shinies yet. Your first one shows up here, with how long it took.").size(12.5).color(MUTED()));
                return;
            }
            let w = ui.available_width();
            let cols = ((w + 8.0) / (96.0 + 8.0)).floor().clamp(1.0, 6.0) as usize;
            let cw = ((w - 8.0 * (cols as f32 - 1.0)) / cols as f32).floor();
            for row in recent.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for (idx, name, enc, when) in row {
                        let k = recent.iter().position(|x| x.3 == *when && x.1 == *name).unwrap_or(0);
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 118.0), Sense::click());
                        let p = ui.painter_at(r);
                        let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                        p.rect_filled(r, 12.0, theme::blend(CARD(), GOLD, 0.06 + 0.06 * ht));
                        if ht > 0.0 {
                            p.rect_stroke(r, 12.0, Stroke::new(1.0f32, theme::blend(LINE(), GOLD, 0.5 * ht)));
                        }
                        if let Some(i) = idx {
                            let sp = self.species[*i].clone();
                            self.draw_sprite(&p, &sp, true, egui::pos2(r.center().x, r.min.y + 70.0), egui::vec2(cw - 16.0, 62.0));
                        }
                        theme::sparkle(&p, r.right_top() + egui::vec2(-12.0, 12.0), 4.5, GOLD);
                        p.text(egui::pos2(r.center().x, r.max.y - 34.0), Align2::CENTER_CENTER, name, crate::fonts::sans_bold_font(12.5), TEXT());
                        p.text(
                            egui::pos2(r.center().x, r.max.y - 16.0),
                            Align2::CENTER_CENTER,
                            format!("{} enc.", theme::thousands(*enc)),
                            crate::fonts::sans_font(11.5),
                            MUTED(),
                        );
                        if resp.on_hover_text(format!("{name}: {} encounters · {}\nClick for its trophy", theme::thousands(*enc), ago(*when))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            trophy = slugs.get(k).cloned().flatten();
                        }
                    }
                });
                ui.add_space(8.0);
            }
        });
        if dex || trophy.is_some() {
            self.setup.tab = Tab::Pokedex;
            self.setup.pokedex.shiny = true;
            self.setup.pokedex.open = None;
            self.setup.dex.open = trophy;
            // back from the trophy comes home
            self.setup.trophy_back = self.setup.dex.open.as_ref().map(|_| Tab::Home);
            self.setup.scroll_top = true;
        }
    }
}

fn chip(ui: &mut egui::Ui, text: &str) {
    chip_col(ui, text, TEXT());
}

/// A Cirrus chip: Cloud pill with a hairline.
fn chip_col(ui: &mut egui::Ui, text: &str, col: Color32) {
    egui::Frame::none()
        .fill(kit::CARD())
        .stroke(Stroke::new(1.0f32, kit::LINE()))
        .rounding(14.0)
        .inner_margin(egui::Margin::symmetric(11.0, 4.0))
        .show(ui, |ui| ui.label(RichText::new(text).size(12.0).color(col)));
}

/// A Cirrus secondary button: Cloud pill with a hairline; Sunken on hover.
fn sec_button(ui: &mut egui::Ui, text: &str, tip: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).color(TEXT()))
            .fill(kit::CARD())
            .stroke(Stroke::new(1.0f32, kit::LINE()))
            .rounding(18.0)
            .min_size(egui::vec2(0.0, 36.0)),
    )
    .on_hover_text(tip)
    .clicked()
}

/// "5m ago", "3h ago", "2d ago" for a unix time.
fn ago(unix: u64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let s = now.saturating_sub(unix);
    if s < 60 {
        "just now".into()
    } else if s < 3600 {
        format!("{}m ago", s / 60)
    } else if s < 86_400 {
        format!("{}h ago", s / 3600)
    } else {
        format!("{}d ago", s / 86_400)
    }
}
