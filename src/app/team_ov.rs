//! The Team Planner checklist on the overlay: its own small always-on-top
//! window over the game, separate from the shiny counter. It follows one
//! team and ticks steps off as the game shows them done (catches straight
//! away, the rest when the game saves). Three looks:
//!
//! * Strip: one line per team member, with its next step.
//! * Card: one member at a time, with every step.
//! * Bar: just the team's next step, in a thin bar.
//!
//! Off unless ticked on the Start overlay screen. Only reads the game.

use super::team_check::steps;
use super::team_game::{find, Kind};
use super::{App, Tab};
use crate::config::TeamLook;
use crate::teams::Team;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke};
use std::time::{Duration, Instant};

/// Room above the panel for the "ticked off" pop-up.
const TOAST_H: f32 = 34.0;
const MAX_CARD_STEPS: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum St {
    Todo,
    Half,
    /// Done in your game.
    Game,
    /// Ticked by you.
    Mine,
}

#[derive(Clone, Debug)]
pub(super) struct OvStep {
    pub key: String,
    pub title: String,
    /// "TM26 in your Bag", "Lv 41 now", or the step's own detail, short.
    pub note: String,
    pub st: St,
}

#[derive(Clone, Debug)]
pub(super) struct OvMember {
    pub name: String,
    pub species: String,
    pub shiny: bool,
    pub steps: Vec<OvStep>,
}

impl OvMember {
    fn done(&self) -> usize {
        self.steps.iter().filter(|s| matches!(s.st, St::Game | St::Mine)).count()
    }
    fn next(&self) -> Option<&OvStep> {
        self.steps.iter().find(|s| !matches!(s.st, St::Game | St::Mine))
    }
}

#[derive(Clone, Debug)]
pub(super) struct OvData {
    pub team_id: u64,
    pub team: String,
    pub members: Vec<OvMember>,
}

impl OvData {
    fn done(&self) -> usize {
        self.members.iter().map(|m| m.done()).sum()
    }
    fn total(&self) -> usize {
        self.members.iter().map(|m| m.steps.len()).sum()
    }
    /// The member whose step is next (the first not done).
    fn next_member(&self) -> Option<usize> {
        self.members.iter().position(|m| m.next().is_some())
    }
    /// Steps done in your game, as "member:key" (for the pop-up).
    fn game_keys(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (i, m) in self.members.iter().enumerate() {
            for s in &m.steps {
                if s.st == St::Game {
                    out.push(format!("{i}:{}", s.key));
                }
            }
        }
        out
    }
}

#[derive(Default)]
pub(super) struct TeamOvState {
    /// Open now (Start overlay → Go, with the tile ticked).
    pub shown: bool,
    geom: Option<[f32; 2]>,
    applied: Option<egui::Vec2>,
    keeper: Option<isize>,
    clip: Option<(isize, Vec<i32>)>,
    /// Worked out at most once a second.
    pub data: Option<(Instant, OvData)>,
    /// Steps already done in the game (team, keys), for the pop-up.
    pub seen: Option<(u64, Vec<String>)>,
    toast: Option<(String, Instant)>,
}

/// `s` cut to fit `w` points in `font`, with "…".
fn fit(p: &Painter, s: &str, font: &FontId, w: f32) -> String {
    let width = |t: &str| p.layout_no_wrap(t.to_string(), font.clone(), Color32::WHITE).size().x;
    if width(s) <= w {
        return s.to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    while chars.len() > 1 {
        chars.pop();
        let t = format!("{}…", chars.iter().collect::<String>().trim_end());
        if width(&t) <= w {
            return t;
        }
    }
    "…".into()
}

/// The first bit of a step's detail ("Route 201 (grass, Lv 2)  ·  …").
fn short_detail(d: &str) -> String {
    d.split("  ·  ").next().unwrap_or("").split(". ").next().unwrap_or("").trim_end_matches('.').to_string()
}

const INK: Color32 = Color32::WHITE;
fn muted() -> Color32 {
    Color32::from_rgb(166, 173, 189)
}
fn violet() -> Color32 {
    Color32::from_rgb(196, 181, 253)
}
fn green() -> Color32 {
    Color32::from_rgb(110, 231, 183)
}
fn gold() -> Color32 {
    Color32::from_rgb(252, 211, 77)
}

/// The panel's size for a look (points, at scale 1).
pub(super) fn panel_size(d: &OvData, look: TeamLook, focus: usize) -> egui::Vec2 {
    match look {
        TeamLook::Strip => vec2(250.0, 12.0 + 24.0 + d.members.len().max(1) as f32 * 40.0 + 8.0),
        TeamLook::Card => {
            let n = d.members.get(focus).map_or(1, |m| m.steps.len().min(MAX_CARD_STEPS + 1)).max(1);
            vec2(270.0, 12.0 + 46.0 + 16.0 + n as f32 * 21.0 + 10.0)
        }
        TeamLook::Bar => vec2(460.0, 52.0),
    }
}

/// A made-up team, for the Start overlay preview before you have one.
pub(super) fn sample() -> OvData {
    let s = |t: &str, n: &str, st: St| OvStep { key: t.into(), title: t.into(), note: n.into(), st };
    OvData {
        team_id: 0,
        team: "My team".into(),
        members: vec![
            OvMember { name: "Garchomp".into(), species: "Garchomp".into(), shiny: false, steps: vec![s("Catch Gible", "", St::Game), s("Evolve into Garchomp", "Lv 41 now", St::Todo)] },
            OvMember { name: "Lucario".into(), species: "Lucario".into(), shiny: false, steps: vec![s("Get Riolu", "Iron Island", St::Todo)] },
            OvMember { name: "Rotom".into(), species: "Rotom-Wash".into(), shiny: false, steps: vec![s("Teach Thunderbolt", "TM24 in your Bag", St::Half)] },
        ],
    }
}

impl App {
    /// The team the overlay follows: the one picked, else the one open in
    /// the Team Planner, else the first.
    pub(super) fn team_ov_team(&mut self) -> Option<Team> {
        let want = self.cfg.team_ov.team.or(self.setup.team.open);
        let teams = &self.setup.team.teams().teams;
        want.and_then(|id| teams.iter().find(|t| t.id == id)).or_else(|| teams.iter().find(|t| !t.members.is_empty())).cloned()
    }

    /// Ticked on the Start overlay screen, and there's a team to show.
    pub(super) fn team_ov_pick(&mut self) -> bool {
        self.has_app(crate::apps::AppId::Team) && self.cfg.team_ov.show && self.team_ov_team().is_some_and(|t| !t.members.is_empty())
    }

    /// The overlay's checklist, worked out now.
    pub(super) fn team_ov_data(&mut self) -> Option<OvData> {
        let t = self.team_ov_team()?;
        let w = self.wiki.clone()?;
        let story = self.story_stage();
        let snap = if t.hand_only { None } else { self.team_snap() };
        let mut members = Vec::new();
        for m in &t.members {
            let ss = steps(&w, t.world, m, story);
            let f = snap.as_deref().map(|sn| find(&w, m, &ss, sn));
            let steps = ss
                .iter()
                .enumerate()
                .map(|(k, s)| {
                    let mark = f.as_ref().and_then(|f| f.marks.get(k));
                    let kind = mark.map_or(Kind::No, |x| x.kind);
                    let st = if m.done.contains(&s.key) {
                        St::Mine
                    } else {
                        match kind {
                            Kind::Game => St::Game,
                            Kind::Half => St::Half,
                            Kind::No => St::Todo,
                        }
                    };
                    let note = match mark {
                        Some(x) if !x.note.is_empty() && st != St::Game => x.note.clone(),
                        _ => short_detail(&s.detail),
                    };
                    OvStep { key: s.key.clone(), title: s.title.clone(), note, st }
                })
                .collect();
            let name = if m.nickname.trim().is_empty() { m.species.clone() } else { m.nickname.trim().to_string() };
            members.push(OvMember { name, species: m.species.clone(), shiny: m.shiny, steps });
        }
        Some(OvData { team_id: t.id, team: t.name.clone(), members })
    }

    /// The data, refreshed once a second; new game ticks make a pop-up.
    fn team_ov_fresh(&mut self) -> Option<OvData> {
        let st = &self.setup.team_ov;
        if let Some((at, d)) = &st.data {
            if at.elapsed() < Duration::from_secs(1) {
                return Some(d.clone());
            }
        }
        let d = self.team_ov_data()?;
        let keys = d.game_keys();
        let st = &mut self.setup.team_ov;
        if let Some((team, old)) = &st.seen {
            if *team == d.team_id {
                let new: Vec<&String> = keys.iter().filter(|k| !old.contains(k)).collect();
                if let Some(first) = new.first() {
                    let (mi, key) = first.split_once(':').unwrap_or(("0", first));
                    let mi: usize = mi.parse().unwrap_or(0);
                    if let Some(m) = d.members.get(mi) {
                        let title = m.steps.iter().find(|s| s.key == key).map(|s| s.title.clone()).unwrap_or_default();
                        let more = if new.len() > 1 { format!(" (+{} more)", new.len() - 1) } else { String::new() };
                        let text = format!("✔ {title}{more} · {} {}/{}", m.name, m.done(), m.steps.len());
                        if self.cfg.all_toasts {
                            st.toast = Some((text, Instant::now()));
                        }
                    }
                }
            }
        }
        st.seen = Some((d.team_id, keys));
        st.data = Some((Instant::now(), d.clone()));
        Some(d)
    }

    pub(super) fn team_ov_viewport(&mut self, ctx: &egui::Context) {
        let on = self.setup.team_ov.shown && crate::apps::live(crate::apps::AppId::Team);
        let data = if on { self.team_ov_fresh() } else { None };
        let Some(d) = data else {
            let st = &mut self.setup.team_ov;
            st.geom = None;
            st.keeper = None;
            st.applied = None;
            st.clip = None;
            return;
        };
        let look = self.cfg.team_ov.look;
        let focus = self.cfg.team_ov.focus.filter(|f| *f < d.members.len()).or_else(|| d.next_member()).unwrap_or(0);
        let ps = panel_size(&d, look, focus);
        let size = vec2(ps.x.max(300.0), ps.y + TOAST_H);
        crate::topmost::ensure_keeper_titled(crate::topmost::TEAM_TITLE, &mut self.setup.team_ov.keeper, &self.borderless_fix);
        if self.setup.team_ov.geom.is_none() {
            // where it was left, else the upper right of the screen
            let want = self.cfg.team_ov.pos.or_else(|| {
                let m = ctx.input(|i| i.viewport().monitor_size)?;
                Some([m.x - size.x - 40.0, 90.0])
            });
            let at = crate::topmost::fit_on_screen(want, [size.x, size.y], ctx.pixels_per_point(), true);
            self.setup.team_ov.geom = Some(at);
            self.setup.team_ov.applied = Some(size);
        }
        let Some(pos) = self.setup.team_ov.geom else { return };
        let builder = egui::ViewportBuilder::default()
            .with_title(crate::topmost::TEAM_TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false)
            .with_position(pos)
            .with_inner_size(size);
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("team-checklist-overlay"), builder, |ctx, _class| {
            if self.setup.team_ov.applied.map_or(true, |a| (a - size).length() > 0.5) {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                self.setup.team_ov.applied = Some(size);
            }
            if let Some(here) = ctx.input(|i| i.viewport().outer_rect) {
                let now = [here.min.x, here.min.y];
                if self.cfg.team_ov.pos.map_or(true, |s| (s[0] - now[0]).abs() > 0.5 || (s[1] - now[1]).abs() > 0.5) {
                    self.cfg.team_ov.pos = Some(now);
                    self.dirty = true;
                }
            }
            let mut shapes: Vec<(Rect, f32)> = Vec::new();
            let mut drag = false;
            let mut open = false;
            let mut pick_focus: Option<usize> = None;
            egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
                ctx.request_repaint_after(Duration::from_millis(250));
                let full = ui.max_rect();
                // the panel hangs from the right edge, under the pop-up lane
                let panel = Rect::from_min_size(pos2(full.max.x - ps.x, full.min.y + TOAST_H), ps);
                let p = ui.painter().clone();
                let resp = ui.interact(panel, egui::Id::new("team-ov-panel"), Sense::click_and_drag());
                if resp.drag_started() {
                    drag = true;
                }
                if resp.on_hover_text("Drag to move · right-click to open the Team Planner").secondary_clicked() {
                    open = true;
                }
                let hits = self.team_ov_paint(&p, panel, &d, look, focus, 1.0);
                for (r, i) in hits {
                    let h = ui.interact(r, egui::Id::new(("team-ov-who", i)), Sense::click());
                    if h.on_hover_text(d.members.get(i).map(|m| m.name.clone()).unwrap_or_default()).clicked() {
                        pick_focus = Some(i);
                    }
                }
                shapes.push((panel, 12.0));
                // the pop-up
                let toast = self.setup.team_ov.toast.clone().filter(|(_, at)| at.elapsed() < Duration::from_secs(5));
                if let Some((text, at)) = toast {
                    let a = (1.0 - (at.elapsed().as_secs_f32() - 4.4).max(0.0) / 0.6).clamp(0.0, 1.0);
                    let font = crate::fonts::sans_bold_font(12.0);
                    let text = fit(&p, &text, &font, full.width() - 30.0);
                    let g = p.layout_no_wrap(text, font, Color32::from_rgba_unmultiplied(220, 252, 231, (255.0 * a) as u8));
                    let tw = g.size().x + 26.0;
                    let tr = Rect::from_center_size(pos2((panel.center().x).clamp(full.min.x + tw / 2.0, full.max.x - tw / 2.0), full.min.y + TOAST_H / 2.0 - 1.0), vec2(tw, 26.0));
                    p.rect_filled(tr, 13.0, Color32::from_rgba_unmultiplied(20, 83, 45, (240.0 * a) as u8));
                    p.rect_stroke(tr, 13.0, Stroke::new(1.0f32, Color32::from_rgba_unmultiplied(34, 197, 94, (110.0 * a) as u8)));
                    p.galley(tr.center() - g.size() / 2.0, g, Color32::WHITE);
                    shapes.push((tr, 13.0));
                } else {
                    self.setup.team_ov.toast = None;
                }
            });
            if drag {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if let Some(i) = pick_focus {
                self.cfg.team_ov.focus = Some(i);
                self.dirty = true;
            }
            if open {
                self.go_to(Tab::Team);
                self.setup.team.open = Some(d.team_id);
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
            }
            self.team_ov_clip(ctx, &shapes);
        });
    }

    /// Cut the window to the panel and the pop-up (see-through parts can
    /// come out black on some PCs).
    fn team_ov_clip(&mut self, ctx: &egui::Context, shapes: &[(Rect, f32)]) {
        let Some(hwnd) = self.setup.team_ov.keeper else { return };
        let ppp = ctx.pixels_per_point();
        let px = |v: f32| (v * ppp).round() as i32;
        let rounded: Vec<[i32; 5]> = shapes.iter().map(|(r, rad)| [px(r.min.x), px(r.min.y), px(r.width()), px(r.height()), px(*rad)]).collect();
        let key: Vec<i32> = rounded.iter().flatten().copied().collect();
        let st = &mut self.setup.team_ov;
        if st.clip.as_ref().is_some_and(|(h, k)| *h == hwnd && *k == key) {
            return;
        }
        crate::topmost::clip_window_shapes(hwnd, &[], &[], &rounded);
        st.clip = Some((hwnd, key));
    }

    /// A member's picture (or its first letters) in a circle.
    fn team_ov_dot(&mut self, p: &Painter, c: Pos2, r: f32, m: &OvMember, k: f32) {
        p.circle_filled(c, r, Color32::from_rgb(59, 64, 80));
        if !self.team_sprite(p, &m.species, m.shiny, c, vec2(r * 2.6, r * 2.6)) {
            p.text(c, Align2::CENTER_CENTER, m.species.chars().take(2).collect::<String>(), crate::fonts::sans_bold_font(9.5 * k), INK);
        }
    }

    /// Draw a look in `panel` (also the Start overlay preview, smaller by
    /// `k`). Returns the member buttons (the Card look's bars).
    pub(super) fn team_ov_paint(&mut self, p: &Painter, panel: Rect, d: &OvData, look: TeamLook, focus: usize, k: f32) -> Vec<(Rect, usize)> {
        let mut hits = Vec::new();
        p.rect_filled(panel, 12.0 * k, Color32::from_rgba_unmultiplied(17, 19, 25, 236));
        p.rect_stroke(panel, 12.0 * k, Stroke::new(1.0f32, Color32::from_white_alpha(30)));
        let p = p.with_clip_rect(panel.shrink(1.0));
        let pad = 12.0 * k;
        let (done, total) = (d.done(), d.total());
        let count = |m: &OvMember| format!("{}/{}", m.done(), m.steps.len());
        let f_b = |s: f32| crate::fonts::sans_bold_font(s * k);
        let f_r = |s: f32| crate::fonts::sans_font(s * k);
        match look {
            TeamLook::Strip => {
                let top = panel.min.y + pad;
                p.text(pos2(panel.max.x - pad, top + 8.0 * k), Align2::RIGHT_CENTER, format!("{done}/{total}"), f_b(11.0), if done == total { green() } else { violet() });
                let name = fit(&p, &d.team, &f_b(12.5), panel.width() - pad * 2.0 - 50.0 * k);
                p.text(pos2(panel.min.x + pad, top + 8.0 * k), Align2::LEFT_CENTER, name, f_b(12.5), INK);
                for (i, m) in d.members.iter().enumerate() {
                    let y = top + 24.0 * k + i as f32 * 40.0 * k;
                    if i > 0 {
                        p.line_segment([pos2(panel.min.x + pad, y), pos2(panel.max.x - pad, y)], Stroke::new(1.0f32, Color32::from_white_alpha(18)));
                    }
                    let c = pos2(panel.min.x + pad + 12.0 * k, y + 20.0 * k);
                    self.team_ov_dot(&p, c, 12.0 * k, m, k);
                    let tx = c.x + 20.0 * k;
                    let tw = panel.max.x - pad - 34.0 * k - tx;
                    let (title, sub) = match m.next() {
                        Some(s) => (s.title.clone(), if s.note.is_empty() { m.name.clone() } else { s.note.clone() }),
                        None => ("All done".to_string(), m.name.clone()),
                    };
                    p.text(pos2(tx, y + 13.0 * k), Align2::LEFT_CENTER, fit(&p, &title, &f_r(12.0), tw), f_r(12.0), INK);
                    let half = m.next().is_some_and(|s| s.st == St::Half);
                    p.text(pos2(tx, y + 28.0 * k), Align2::LEFT_CENTER, fit(&p, &sub, &f_r(10.5), tw), f_r(10.5), if half { gold() } else { muted() });
                    let right = pos2(panel.max.x - pad, y + 20.0 * k);
                    if m.next().is_none() {
                        p.text(right, Align2::RIGHT_CENTER, "✔", f_b(12.0), green());
                    } else {
                        p.text(right, Align2::RIGHT_CENTER, count(m), f_b(10.5), violet());
                    }
                }
            }
            TeamLook::Card => {
                let Some(m) = d.members.get(focus) else { return hits };
                let top = panel.min.y + pad;
                // progress ring with the count
                let c = pos2(panel.min.x + pad + 20.0 * k, top + 20.0 * k);
                let rr = 17.0 * k;
                p.circle_stroke(c, rr, Stroke::new(4.0 * k, Color32::from_rgb(59, 64, 80)));
                let frac = if m.steps.is_empty() { 0.0 } else { m.done() as f32 / m.steps.len() as f32 };
                let n = 48;
                let pts: Vec<Pos2> = (0..=((n as f32 * frac) as usize))
                    .map(|i| {
                        let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * (i as f32 / n as f32);
                        c + vec2(a.cos(), a.sin()) * rr
                    })
                    .collect();
                if pts.len() > 1 {
                    p.add(egui::Shape::line(pts, Stroke::new(4.0 * k, if frac >= 1.0 { green() } else { Color32::from_rgb(139, 92, 246) })));
                }
                p.text(c, Align2::CENTER_CENTER, count(m), f_b(9.5), INK);
                let tx = c.x + 28.0 * k;
                let tw = panel.max.x - pad - tx;
                p.text(pos2(tx, top + 11.0 * k), Align2::LEFT_CENTER, fit(&p, &m.species, &f_b(13.0), tw), f_b(13.0), INK);
                let sub = if m.name != m.species { format!("{} · {}", m.name, d.team) } else { d.team.clone() };
                p.text(pos2(tx, top + 28.0 * k), Align2::LEFT_CENTER, fit(&p, &sub, &f_r(10.5), tw), f_r(10.5), muted());
                // one bar per member: click to show it
                let by = top + 46.0 * k;
                let nm = d.members.len().max(1) as f32;
                let gap = 4.0 * k;
                let bw = (panel.width() - pad * 2.0 - gap * (nm - 1.0)) / nm;
                for (i, mm) in d.members.iter().enumerate() {
                    let b = Rect::from_min_size(pos2(panel.min.x + pad + i as f32 * (bw + gap), by), vec2(bw, 5.0 * k));
                    p.rect_filled(b, 3.0 * k, Color32::from_rgb(59, 64, 80));
                    let fr = if mm.steps.is_empty() { 0.0 } else { mm.done() as f32 / mm.steps.len() as f32 };
                    if fr > 0.0 {
                        p.rect_filled(Rect::from_min_size(b.min, vec2(b.width() * fr, b.height())), 3.0 * k, if fr >= 1.0 { green() } else { Color32::from_rgb(139, 92, 246) });
                    }
                    if i == focus {
                        p.rect_stroke(b.expand(2.0 * k), 4.0 * k, Stroke::new(1.0f32, Color32::from_white_alpha(140)));
                    }
                    hits.push((b.expand2(vec2(0.0, 6.0 * k)), i));
                }
                // the steps
                let sy = by + 16.0 * k;
                let more = m.steps.len().saturating_sub(MAX_CARD_STEPS);
                let shown = if more > 0 { MAX_CARD_STEPS } else { m.steps.len() };
                for (j, s) in m.steps.iter().take(shown).enumerate() {
                    let y = sy + j as f32 * 21.0 * k + 10.0 * k;
                    let b = Rect::from_center_size(pos2(panel.min.x + pad + 7.0 * k, y), vec2(14.0 * k, 14.0 * k));
                    let on = matches!(s.st, St::Game | St::Mine);
                    match s.st {
                        St::Game | St::Mine => {
                            p.rect_filled(b, 4.0 * k, if s.st == St::Game { Color32::from_rgb(31, 157, 107) } else { Color32::from_rgb(139, 92, 246) });
                            p.add(egui::Shape::line(vec![b.min + vec2(3.5, 7.0) * k, b.min + vec2(6.0, 9.8) * k, b.min + vec2(10.8, 4.4) * k], Stroke::new(1.8 * k, INK)));
                        }
                        St::Half => {
                            p.rect_filled(b, 4.0 * k, Color32::from_rgb(58, 44, 16));
                            p.rect_filled(Rect::from_min_max(b.min, pos2(b.center().x, b.max.y)), egui::Rounding { nw: 4.0 * k, sw: 4.0 * k, ne: 0.0, se: 0.0 }, Color32::from_rgb(201, 149, 42));
                            p.rect_stroke(b, 4.0 * k, Stroke::new(1.5 * k, Color32::from_rgb(201, 149, 42)));
                        }
                        St::Todo => {
                            p.rect_stroke(b, 4.0 * k, Stroke::new(1.5 * k, Color32::from_rgb(90, 96, 112)));
                        }
                    }
                    let tx = b.max.x + 8.0 * k;
                    let tw = panel.max.x - pad - tx;
                    let text = if on || s.note.is_empty() { s.title.clone() } else { format!("{} · {}", s.title, s.note) };
                    let text = fit(&p, &text, &f_r(11.5), tw);
                    let col = if on { Color32::from_rgb(140, 147, 163) } else { INK };
                    let g = p.layout_no_wrap(text, f_r(11.5), col);
                    let gw = g.size().x;
                    p.galley(pos2(tx, y - g.size().y / 2.0), g, col);
                    if on {
                        p.line_segment([pos2(tx, y), pos2(tx + gw, y)], Stroke::new(1.0f32, col));
                    }
                }
                if more > 0 {
                    let y = sy + shown as f32 * 21.0 * k + 10.0 * k;
                    p.text(pos2(panel.min.x + pad, y), Align2::LEFT_CENTER, format!("+{more} more in the Team Planner"), f_r(10.5), muted());
                }
            }
            TeamLook::Bar => {
                let c = pos2(panel.min.x + pad + 13.0 * k, panel.center().y);
                let nm = d.next_member();
                match nm.and_then(|i| d.members.get(i)) {
                    Some(m) => self.team_ov_dot(&p, c, 13.0 * k, m, k),
                    None => {
                        p.circle_filled(c, 13.0 * k, Color32::from_rgb(31, 157, 107));
                        p.text(c, Align2::CENTER_CENTER, "✔", f_b(12.0), INK);
                    }
                }
                let tx = c.x + 22.0 * k;
                let right = 74.0 * k;
                let tw = panel.max.x - pad - right - tx;
                let (title, sub) = match nm.and_then(|i| d.members.get(i)) {
                    Some(m) => {
                        let s = m.next().expect("has a next step");
                        let then = m.steps.iter().skip_while(|x| x.key != s.key).skip(1).find(|x| !matches!(x.st, St::Game | St::Mine));
                        let mut sub = m.name.clone();
                        if !s.note.is_empty() {
                            sub = format!("{sub} · {}", s.note);
                        }
                        if let Some(t) = then {
                            sub = format!("{sub} · then: {}", t.title);
                        }
                        (s.title.clone(), sub)
                    }
                    None => ("Team ready!".to_string(), d.team.clone()),
                };
                p.text(pos2(tx, panel.center().y - 8.0 * k), Align2::LEFT_CENTER, fit(&p, &title, &f_b(12.5), tw), f_b(12.5), INK);
                p.text(pos2(tx, panel.center().y + 9.0 * k), Align2::LEFT_CENTER, fit(&p, &sub, &f_r(10.5), tw), f_r(10.5), muted());
                let bx = panel.max.x - pad - 60.0 * k;
                p.text(pos2(panel.max.x - pad, panel.center().y - 7.0 * k), Align2::RIGHT_CENTER, format!("{done}/{total}"), f_b(10.5), violet());
                let b = Rect::from_min_size(pos2(bx, panel.center().y + 4.0 * k), vec2(60.0 * k, 5.0 * k));
                p.rect_filled(b, 3.0 * k, Color32::from_rgb(59, 64, 80));
                let fr = if total == 0 { 0.0 } else { done as f32 / total as f32 };
                if fr > 0.0 {
                    p.rect_filled(Rect::from_min_size(b.min, vec2(b.width() * fr, b.height())), 3.0 * k, if fr >= 1.0 { green() } else { Color32::from_rgb(139, 92, 246) });
                }
            }
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_steps_and_counts() {
        let d = sample();
        assert_eq!(d.done(), 1);
        assert_eq!(d.total(), 4);
        assert_eq!(d.next_member(), Some(0));
        assert_eq!(d.members[0].next().map(|s| s.title.as_str()), Some("Evolve into Garchomp"));
        assert_eq!(d.game_keys(), vec!["0:Catch Gible".to_string()]);
        assert_eq!(short_detail("Route 201 (grass, Lv 2)  ·  Route 203"), "Route 201 (grass, Lv 2)");
        assert!(panel_size(&d, TeamLook::Card, 0).y < panel_size(&d, TeamLook::Strip, 0).y + 80.0);
    }
}
