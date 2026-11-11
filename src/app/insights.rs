//! Run stats tabs (Luck, Activity, Encounters), their charts, and the
//! lifetime records on the Profile page.
//!
//! Chart language, shared by every chart here so they read as one set:
//!   · the user's button colour marks "you" / the highlight (current point,
//!     busiest bar, this week); everything else is a quiet neutral
//!   · gold only for shiny things (a caught shiny's marker)
//!   · hairline grid, muted labels, values on hover

use super::hunt_pages::RunInfo;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::App;
use crate::stats::{self, fmt_day_short, fmt_duration, local_day, now_unix, RunStats, Session};
use crate::theme::{self, GOLD};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Shape, Stroke};
use std::collections::BTreeMap;

// ───────────────────────────── chart colours ─────────────────────────────

/// "You" / highlight colour: the user's button colour.
fn accent() -> Color32 {
    theme::BUTTON()
}

/// Quiet bar colour.
fn neutral() -> Color32 {
    theme::blend(CARD(), TEXT(), 0.22)
}

fn pct(x: f64) -> String {
    let p = x * 100.0;
    if p > 0.0 && p < 1.0 {
        "<1%".to_string()
    } else if p < 100.0 && p > 99.0 {
        ">99%".to_string()
    } else {
        format!("{:.0}%", p)
    }
}

fn odds_text(odds: f64) -> String {
    format!("1 in {}", theme::thousands(odds.round() as u64))
}

/// Text for a time to cover `n` encounters at `rate` per hour.
fn time_for(n: u64, rate: Option<f64>) -> Option<String> {
    let r = rate.filter(|r| *r > 0.0)?;
    let secs = (n as f64 / r * 3600.0).round() as u64;
    Some(if secs < 60 { "under a minute".to_string() } else { fmt_duration(secs) })
}

/// Does this method use Sweet Scent (so battles = Sweet Scents)?
fn uses_sweet_scent(method: &str) -> bool {
    method.contains("Sweet Scent") || method.contains("Rare grass")
}

/// Card title row: bold title left, optional caption right.
fn card_title(ui: &mut egui::Ui, title: &str, caption: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).font(kit::font_bold(15.0)).color(TEXT()));
        if !caption.is_empty() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(caption).size(kit::fs(12.5)).color(MUTED()));
            });
        }
    });
    ui.add_space(6.0);
}

/// Small explanatory line under a chart.
fn note(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).size(kit::fs(12.0)).color(MUTED()));
}

/// Floating label box (for hover read-outs and markers), kept inside `bounds`.
fn tag(p: &egui::Painter, anchor: Pos2, text: &str, fill: Color32, ink: Color32, bounds: Rect) {
    let galley = p.layout_no_wrap(text.to_string(), kit::font(12.0), ink);
    let size = galley.size() + egui::vec2(14.0, 6.0);
    let mut r = Rect::from_center_size(anchor, size);
    let dx = (bounds.min.x - r.min.x).max(0.0) + (bounds.max.x - r.max.x).min(0.0);
    let dy = (bounds.min.y - r.min.y).max(0.0) + (bounds.max.y - r.max.y).min(0.0);
    r = r.translate(egui::vec2(dx, dy));
    p.rect_filled(r, r.height() / 2.0, fill);
    p.galley(r.min + egui::vec2(7.0, 3.0), galley, ink);
}

// ───────────────────────────── charts ─────────────────────────────

/// Progress ring with a big number in the middle.
fn ring(p: &egui::Painter, c: Pos2, r: f32, frac: f32, color: Color32, label: &str) {
    p.circle_stroke(c, r, Stroke::new(8.0f32, LINE()));
    let frac = frac.clamp(0.0, 1.0);
    if frac > 0.0 {
        let steps = ((frac * 72.0).ceil() as usize).max(2);
        let start = -std::f32::consts::FRAC_PI_2;
        let pts: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let a = start + std::f32::consts::TAU * frac * i as f32 / steps as f32;
                c + egui::vec2(a.cos(), a.sin()) * r
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(8.0f32, color)));
        // round caps
        let end_a = start + std::f32::consts::TAU * frac;
        p.circle_filled(c + egui::vec2(start.cos(), start.sin()) * r, 4.0, color);
        p.circle_filled(c + egui::vec2(end_a.cos(), end_a.sin()) * r, 4.0, color);
    }
    p.text(c, Align2::CENTER_CENTER, label, kit::font_bold(19.0), TEXT());
}

/// The luck curve: share of hunters who'd have found the shiny by each
/// encounter count, with "you" marked on it.
fn luck_curve(ui: &mut egui::Ui, count: u64, odds: f64, caught: bool) {
    let h = 220.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), Sense::hover());
    let p = ui.painter_at(rect);
    let plot = Rect::from_min_max(rect.min + egui::vec2(42.0, 16.0), rect.max - egui::vec2(14.0, 28.0));
    let x_max = (odds * 3.0).max(count as f64 * 1.12).max(10.0);
    let to_x = |n: f64| plot.min.x + (n / x_max) as f32 * plot.width();
    let to_y = |q: f64| plot.max.y - q as f32 * plot.height();
    let f = |n: f64| stats::found_by(n.max(0.0).round() as u64, odds);
    let small = kit::font_small(11.0);

    // grid: 0% and 100% solid, 50% and 90% dashed
    for q in [0.0, 1.0] {
        p.line_segment([egui::pos2(plot.min.x, to_y(q)), egui::pos2(plot.max.x, to_y(q))], Stroke::new(1.0f32, LINE()));
    }
    for q in [0.5, 0.9] {
        p.extend(Shape::dashed_line(
            &[egui::pos2(plot.min.x, to_y(q)), egui::pos2(plot.max.x, to_y(q))],
            Stroke::new(1.0f32, LINE()),
            4.0,
            4.0,
        ));
    }
    for q in [0.0, 0.5, 0.9, 1.0] {
        p.text(egui::pos2(plot.min.x - 8.0, to_y(q)), Align2::RIGHT_CENTER, pct(q), small.clone(), MUTED());
    }
    // x ticks at whole multiples of the odds
    let mut k = 1;
    while (k as f64) * odds <= x_max {
        let x = to_x(k as f64 * odds);
        p.line_segment([egui::pos2(x, plot.min.y), egui::pos2(x, plot.max.y)], Stroke::new(1.0f32, LINE().gamma_multiply(0.6)));
        p.text(
            egui::pos2(x, plot.max.y + 14.0),
            Align2::CENTER_CENTER,
            format!("{k}× · {}", theme::thousands((k as f64 * odds).round() as u64)),
            small.clone(),
            MUTED(),
        );
        k += 1;
    }
    p.text(egui::pos2(plot.min.x, plot.max.y + 14.0), Align2::LEFT_CENTER, "0", small.clone(), MUTED());

    // curve
    let n_pts = 160;
    let pts: Vec<Pos2> = (0..=n_pts)
        .map(|i| {
            let n = x_max * i as f64 / n_pts as f64;
            egui::pos2(to_x(n), to_y(f(n)))
        })
        .collect();
    let col = if caught { GOLD } else { accent() };
    let cx = to_x(count as f64);
    let me = egui::pos2(cx, to_y(f(count as f64)));
    // shaded area up to "you"
    let fill = col.gamma_multiply(0.16);
    for w in pts.windows(2) {
        if w[0].x >= cx {
            break;
        }
        let a = w[0];
        let b = if w[1].x > cx { me } else { w[1] };
        p.add(Shape::convex_polygon(
            vec![a, b, egui::pos2(b.x, plot.max.y), egui::pos2(a.x, plot.max.y)],
            fill,
            Stroke::NONE,
        ));
    }
    p.add(Shape::line(pts.clone(), Stroke::new(2.0f32, neutral())));
    let mut done: Vec<Pos2> = pts.iter().copied().filter(|q| q.x <= cx).collect();
    done.push(me);
    if done.len() >= 2 {
        p.add(Shape::line(done, Stroke::new(3.0f32, col)));
    }
    // "you" marker
    p.line_segment([egui::pos2(cx, me.y), egui::pos2(cx, plot.max.y)], Stroke::new(1.0f32, col.gamma_multiply(0.6)));
    p.circle_filled(me, 7.0, col);
    p.circle_stroke(me, 7.0, Stroke::new(2.0f32, CARD()));
    if caught {
        theme::sparkle(&p, me, 4.0, Color32::WHITE);
    }
    let who = if caught { "Caught" } else { "You" };
    let you_label = format!("{who} · {} · {}", theme::thousands(count), pct(f(count as f64)));
    let ink = if caught { theme::OUTLINE } else { theme::button_ink() };
    tag(&p, me + egui::vec2(0.0, -22.0), &you_label, col, ink, rect);

    // hover read-out
    if let Some(hp) = resp.hover_pos() {
        if plot.expand2(egui::vec2(0.0, 10.0)).contains(hp) {
            let t = ((hp.x - plot.min.x) / plot.width()).clamp(0.0, 1.0) as f64;
            let n = (t * x_max).round();
            let q = egui::pos2(to_x(n), to_y(f(n)));
            p.line_segment([egui::pos2(q.x, plot.min.y), egui::pos2(q.x, plot.max.y)], Stroke::new(1.0f32, MUTED().gamma_multiply(0.6)));
            p.circle_filled(q, 4.0, TEXT());
            tag(
                &p,
                egui::pos2(q.x, plot.min.y + 2.0),
                &format!("{} encounters: {} would have it", theme::thousands(n as u64), pct(f(n))),
                CARD_HOVER(),
                TEXT(),
                rect,
            );
        }
    }
}

/// Vertical bar chart. `top` = text shown above the highlighted bar.
fn column_chart(
    ui: &mut egui::Ui,
    values: &[f64],
    labels: &[String],
    hovers: &[String],
    highlight: Option<usize>,
    top: Option<String>,
    height: f32,
) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    let p = ui.painter_at(rect);
    let chart = Rect::from_min_max(rect.min + egui::vec2(0.0, 22.0), rect.max - egui::vec2(0.0, 20.0));
    p.line_segment([chart.left_bottom(), chart.right_bottom()], Stroke::new(1.0f32, LINE()));
    let max = values.iter().cloned().fold(0.0, f64::max);
    if max <= 0.0 {
        p.text(chart.center(), Align2::CENTER_CENTER, "Not enough hunting yet", kit::font(12.5), MUTED());
        return;
    }
    let n = values.len().max(1);
    let slot = chart.width() / n as f32;
    let hover = resp.hover_pos();
    let mut hovered = None;
    for (i, v) in values.iter().enumerate() {
        let x = chart.min.x + slot * i as f32;
        let col_rect = Rect::from_min_max(egui::pos2(x, rect.min.y), egui::pos2(x + slot, chart.max.y));
        let is_hover = hover.is_some_and(|h| col_rect.contains(h));
        if is_hover {
            p.rect_filled(col_rect.shrink2(egui::vec2(1.0, 0.0)), 4.0, TEXT().gamma_multiply(0.05));
            hovered = Some(i);
        }
        let pad = (slot * 0.2).min(10.0);
        let bh = (chart.height() * (*v / max) as f32).max(if *v > 0.0 { 3.0 } else { 0.0 });
        if bh > 0.0 {
            let bar = Rect::from_min_max(egui::pos2(x + pad, chart.max.y - bh), egui::pos2(x + slot - pad, chart.max.y));
            let rr = (bar.width() * 0.3).min(4.0);
            let c = if Some(i) == highlight { accent() } else { neutral() };
            p.rect_filled(bar, egui::Rounding { nw: rr, ne: rr, sw: 0.0, se: 0.0 }, c);
            if Some(i) == highlight {
                if let Some(t) = &top {
                    p.text(egui::pos2(bar.center().x, bar.min.y - 9.0), Align2::CENTER_CENTER, t, kit::font_small(11.0), TEXT());
                }
            }
        }
        if let Some(l) = labels.get(i).filter(|l| !l.is_empty()) {
            p.text(
                egui::pos2(x + slot / 2.0, rect.max.y - 8.0),
                Align2::CENTER_CENTER,
                l,
                kit::font_small(11.0),
                if Some(i) == highlight { TEXT() } else { MUTED() },
            );
        }
    }
    if let Some(i) = hovered {
        if let Some(t) = hovers.get(i) {
            let x = chart.min.x + slot * (i as f32 + 0.5);
            tag(&p, egui::pos2(x, rect.min.y + 9.0), t, CARD_HOVER(), TEXT(), rect);
        }
    }
}

/// One "this week vs last week" row: two bars and the change.
fn compare_row(ui: &mut egui::Ui, label: &str, this: f64, last: f64, fmt: &dyn Fn(f64) -> String) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 46.0), Sense::hover());
    let p = ui.painter_at(rect);
    let label_w = 96.0;
    let change_w = 64.0;
    let value_w = 76.0;
    p.text(egui::pos2(rect.min.x, rect.center().y), Align2::LEFT_CENTER, label, kit::font(13.5), TEXT());
    let bar_x0 = rect.min.x + label_w;
    let bar_x1 = rect.max.x - change_w - value_w;
    let max = this.max(last).max(1e-9);
    for (k, (v, c)) in [(this, accent()), (last, neutral())].into_iter().enumerate() {
        let y = rect.min.y + 13.0 + k as f32 * 20.0;
        let w = ((bar_x1 - bar_x0) * (v / max) as f32).max(if v > 0.0 { 4.0 } else { 0.0 });
        p.rect_filled(Rect::from_min_size(egui::pos2(bar_x0, y - 6.0), egui::vec2(bar_x1 - bar_x0, 12.0)), 6.0, CARD_HOVER());
        if w > 0.0 {
            p.rect_filled(Rect::from_min_size(egui::pos2(bar_x0, y - 6.0), egui::vec2(w, 12.0)), 6.0, c);
        }
        p.text(
            egui::pos2(bar_x1 + 10.0, y),
            Align2::LEFT_CENTER,
            fmt(v),
            if k == 0 { kit::font_bold(12.5) } else { kit::font(12.5) },
            if k == 0 { TEXT() } else { MUTED() },
        );
    }
    // change pill
    let (text, fill, ink) = if last <= 0.0 {
        ("new".to_string(), CARD_HOVER(), MUTED())
    } else {
        let ch = (this - last) / last * 100.0;
        if ch.abs() < 0.5 {
            ("±0%".to_string(), CARD_HOVER(), MUTED())
        } else if ch > 0.0 {
            (format!("▲ {:.0}%", ch), Color32::from_rgb(22, 64, 38), Color32::from_rgb(140, 232, 160))
        } else {
            (format!("▼ {:.0}%", -ch), CARD_HOVER(), MUTED())
        }
    };
    tag(&p, egui::pos2(rect.max.x - change_w / 2.0, rect.center().y), &text, fill, ink, rect);
}

/// Stat tile with a small icon, for the fun facts. Fixed height and drawn
/// directly, so tiles side by side always line up.
fn fact(ui: &mut egui::Ui, w: f32, icon: &str, value: &str, label: &str) {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 64.0), Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, kit::r(8.0), CARD_HOVER());
    let icon_c = egui::pos2(r.min.x + 26.0, r.center().y);
    p.text(icon_c, Align2::CENTER_CENTER, icon, egui::FontId::proportional(19.0), MUTED());
    let x = r.min.x + 48.0;
    let clip = p.with_clip_rect(Rect::from_min_max(egui::pos2(x, r.min.y), egui::pos2(r.max.x - 10.0, r.max.y)));
    clip.text(egui::pos2(x, r.center().y - 9.0), Align2::LEFT_CENTER, value, kit::font_bold(17.0), TEXT());
    clip.text(egui::pos2(x, r.center().y + 12.0), Align2::LEFT_CENTER, label, kit::font(12.0), MUTED());
    resp.on_hover_text(format!("{value} · {label}"));
}

/// Lay tiles out in rows of `per` across the page.
fn tile_rows<T>(ui: &mut egui::Ui, items: &[T], per: usize, mut draw: impl FnMut(&mut egui::Ui, f32, &T)) {
    let per = per.max(1);
    let w = ((ui.available_width() - 8.0 * (per as f32 - 1.0)) / per as f32).floor();
    for row in items.chunks(per) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for it in row {
                draw(ui, w, it);
            }
        });
        ui.add_space(8.0);
    }
}

impl App {
    // ───────────────────────────── luck ─────────────────────────────

    /// Overview card: a ring with the headline luck number. Returns true if
    /// "Luck curve" was clicked.
    pub(super) fn luck_summary(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats, odds: f64) -> bool {
        let mut open = false;
        let found = stats::found_by(info.count, odds);
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(96.0, 96.0), Sense::hover());
                let p = ui.painter_at(r.expand(6.0));
                let (title, sub, frac, col) = if info.caught {
                    let beat = 1.0 - found;
                    let title = if beat >= 0.5 {
                        format!("Luckier than {} of hunters", pct(beat))
                    } else {
                        format!("{} of hunters would have found it sooner", pct(found))
                    };
                    let sub = format!(
                        "Found at {} encounters: {:.1}× the odds ({}).",
                        theme::thousands(info.count),
                        info.count as f64 / odds,
                        odds_text(odds)
                    );
                    (title, sub, beat, GOLD)
                } else {
                    let half = stats::encounters_for(0.5, odds);
                    let when = time_for(half, st.per_hour()).map_or(String::new(), |t| format!(" (about {t} at your pace)"));
                    (
                        format!("{} of hunters would have found it by now", pct(found)),
                        format!(
                            "You're at {:.1}× the odds. Each encounter is a fresh {} chance, so from here it's 50/50 within the next {}{when}.",
                            info.count as f64 / odds,
                            odds_text(odds),
                            theme::thousands(half)
                        ),
                        found,
                        accent(),
                    )
                };
                ring(&p, r.center(), 38.0, frac as f32, col, &pct(frac));
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new(title).font(kit::font_bold(16.0)).color(TEXT()));
                    ui.add_space(2.0);
                    ui.add(egui::Label::new(RichText::new(sub).size(kit::fs(12.5)).color(MUTED())).wrap());
                    ui.add_space(6.0);
                    if ui
                        .add(egui::Button::new(RichText::new("See the luck curve  ›").size(kit::fs(12.5))).fill(kit::btn_fill()).rounding(kit::btn_round(6.0)))
                        .clicked()
                    {
                        open = true;
                    }
                });
            });
        });
        open
    }

    /// Run stats → Luck: the curve, "from here" projections, plain-words maths.
    pub(super) fn run_luck(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        let Some(odds) = info.odds.filter(|o| *o > 1.0) else {
            kit::card(ui, |ui| {
                ui.label(RichText::new("No odds for this hunt").font(kit::font_bold(15.0)).color(TEXT()));
                note(ui, "Pick how you're hunting under Hunt and the luck stats appear here.");
            });
            return;
        };
        let rate = st.per_hour();
        kit::card(ui, |ui| {
            card_title(
                ui,
                if info.caught { "When hunters find this shiny" } else { "Chance you'd have found it by now" },
                &odds_text(odds),
            );
            luck_curve(ui, info.count, odds, info.caught);
            note(
                ui,
                "The curve shows what share of hunters find their shiny within each number of encounters. Hover it to read any point.",
            );
        });

        let w4 = ((ui.available_width() - 24.0) / 4.0).floor();
        if info.caught {
            kit::section(ui, "How your hunt compares");
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let found = stats::found_by(info.count, odds);
                kit::stat(ui, w4, "Found at", &theme::thousands(info.count));
                kit::stat(ui, w4, "Odds reached", &format!("{:.2}×", info.count as f64 / odds));
                kit::stat(ui, w4, "Luckier than", &pct(1.0 - found));
                kit::stat(ui, w4, "Typical hunter", &theme::thousands(stats::encounters_for(0.5, odds)));
            });
        } else {
            kit::section(ui, "From here");
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for q in [0.5, 0.75, 0.9, 0.99] {
                    let n = stats::encounters_for(q, odds);
                    let v = match time_for(n, rate) {
                        Some(t) => format!("{} · {t}", theme::thousands(n)),
                        None => theme::thousands(n),
                    };
                    kit::stat(ui, w4, &format!("{} chance within", pct(q)), &v);
                }
            });
            ui.add_space(8.0);
            if let Some((secs, k)) = st.eta_to_odds(info.count, odds) {
                let more = ((odds * k as f64).ceil() as u64).saturating_sub(info.count);
                note(
                    ui,
                    &format!(
                        "Next marker: {k}× the odds in {} more encounters (about {} of hunting).",
                        theme::thousands(more),
                        fmt_duration(secs)
                    ),
                );
            }
        }
        ui.add_space(6.0);
        kit::card(ui, |ui| {
            ui.label(RichText::new("How the maths works").font(kit::font_bold(14.0)).color(TEXT()));
            note(
                ui,
                &format!(
                    "Every encounter has the same {} chance, no matter how many came before. That's why \"from here\" never shrinks: being past the odds doesn't make a shiny due. It just means you've been unlucky so far, and the next one is as likely as the first.",
                    odds_text(odds)
                ),
            );
        });
    }

    // ───────────────────────────── activity ─────────────────────────────

    /// Run stats → Activity: time of day, weekdays, this week vs last, sessions.
    pub(super) fn run_activity(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        if st.sessions.is_empty() {
            kit::card(ui, |ui| {
                ui.label(RichText::new("No hunting time yet").font(kit::font_bold(15.0)).color(TEXT()));
                note(ui, "Once you've hunted for a few minutes, your habits show up here.");
            });
            return;
        }
        // Time of day
        let slots = st.by_hour_of_day();
        let vals: Vec<f64> = slots.iter().map(|s| s.encounters).collect();
        let busiest = vals.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(i, _)| i);
        let fastest = slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.rate().map(|r| (i, r)))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let mut caption = String::new();
        if let Some(b) = busiest {
            caption = format!("Busiest {}", stats::fmt_hour(b as i64));
        }
        if let Some((fi, fr)) = fastest {
            caption.push_str(&format!("  ·  Fastest {} ({}/h)", stats::fmt_hour(fi as i64), theme::thousands(fr.round() as u64)));
        }
        let labels: Vec<String> = (0..24).map(|h| if h % 3 == 0 { stats::fmt_hour(h) } else { String::new() }).collect();
        let hovers: Vec<String> = slots
            .iter()
            .enumerate()
            .map(|(h, s)| {
                let mut t = format!(
                    "{}–{}: {} encounters",
                    stats::fmt_hour(h as i64),
                    stats::fmt_hour(h as i64 + 1),
                    theme::thousands(s.encounters.round() as u64)
                );
                if let Some(r) = s.rate() {
                    t.push_str(&format!(" · {}/h", theme::thousands(r.round() as u64)));
                }
                t
            })
            .collect();
        kit::card(ui, |ui| {
            card_title(ui, "Time of day", &caption);
            let top = busiest.map(|b| theme::thousands(vals[b].round() as u64));
            column_chart(ui, &vals, &labels, &hovers, busiest, top, 170.0);
        });

        // Weekdays + this week vs last, side by side when there's room
        let wide = ui.available_width() >= 980.0;
        let today = local_day(now_unix());
        let days_panel = |ui: &mut egui::Ui| {
            let wd = st.by_weekday();
            let vals: Vec<f64> = wd.iter().map(|d| d.0 as f64).collect();
            let top_i = vals.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).filter(|(_, v)| **v > 0.0).map(|(i, _)| i);
            let labels: Vec<String> = stats::WEEKDAYS.iter().map(|s| s.to_string()).collect();
            let hovers: Vec<String> = wd
                .iter()
                .enumerate()
                .map(|(i, d)| format!("{}: {} · {}", stats::WEEKDAYS[i], theme::thousands(d.0), fmt_duration(d.1)))
                .collect();
            kit::card(ui, |ui| {
                card_title(ui, "Days of the week", &top_i.map_or(String::new(), |i| format!("Most on {}", stats::WEEKDAYS[i])));
                let top = top_i.map(|i| theme::thousands(vals[i] as u64));
                column_chart(ui, &vals, &labels, &hovers, top_i, top, 150.0);
            });
        };
        let week_panel = |ui: &mut egui::Ui| {
            let ((te, ts), (le, ls)) = st.week_compare(today);
            kit::card(ui, |ui| {
                card_title(ui, "This week vs last week", "");
                compare_row(ui, "Encounters", te as f64, le as f64, &|v: f64| theme::thousands(v.round() as u64));
                compare_row(ui, "Hunt time", ts as f64, ls as f64, &|v: f64| fmt_duration(v as u64));
                let r = |e: u64, s: u64| if s >= 300 { e as f64 * 3600.0 / s as f64 } else { 0.0 };
                compare_row(ui, "Per hour", r(te, ts), r(le, ls), &|v: f64| if v > 0.0 { theme::thousands(v.round() as u64) } else { "—".into() });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    legend_dot(ui, accent(), "Last 7 days");
                    ui.add_space(10.0);
                    legend_dot(ui, neutral(), "7 days before");
                });
            });
        };
        if wide {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                ui.columns(2, |cols| {
                    days_panel(&mut cols[0]);
                    week_panel(&mut cols[1]);
                });
            });
        } else {
            days_panel(ui);
            week_panel(ui);
        }

        // Session records
        kit::section(ui, "Sessions");
        let w4 = ((ui.available_width() - 24.0) / 4.0).floor();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            kit::stat(
                ui,
                w4,
                "Best day",
                &st.best_day()
                    .filter(|d| d.encounters > 0)
                    .map_or("—".into(), |d| format!("{} · {}", theme::thousands(d.encounters), fmt_day_short(d.day))),
            );
            kit::stat(ui, w4, "Average session", &st.avg_session_secs().map_or("—".into(), fmt_duration));
            kit::stat(
                ui,
                w4,
                "Longest session",
                &st.longest_session().map_or("—".into(), |s| format!("{} · {}", fmt_duration(s.secs), fmt_day_short(local_day(s.start)))),
            );
            kit::stat(
                ui,
                w4,
                "Fastest session",
                &st.fastest_session().map_or("—".into(), |(_, r)| format!("{}/h", theme::thousands(r.round() as u64))),
            );
        });
        ui.add_space(8.0);
        session_timeline(ui, &st.sessions, info.current);
    }

    // ───────────────────────────── encounters ─────────────────────────────

    /// Run stats → Encounters: what you're running into, target sightings,
    /// fun facts.
    pub(super) fn run_encounters(&mut self, ui: &mut egui::Ui, info: &RunInfo, st: &RunStats) {
        if !info.phase {
            self.breakdown_section(ui, info, st);
        }
        // Target sightings
        let target_dex = self.species_of(info.slug.as_deref()).map(|(i, _)| i as u32 + 1);
        if target_dex.is_some() && !info.phase {
            kit::section(ui, &format!("{} sightings", info.name));
            let w4 = ((ui.available_width() - 24.0) / 4.0).floor();
            let seen = info.target_seen;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                kit::stat(ui, w4, "Seen", &theme::thousands(seen));
                kit::stat(
                    ui,
                    w4,
                    "How often",
                    &if seen > 0 {
                        let every = info.count as f64 / seen as f64;
                        if every < 10.0 {
                            format!("1 in {every:.1}")
                        } else {
                            format!("1 in {}", theme::thousands(every.round() as u64))
                        }
                    } else {
                        "—".into()
                    },
                );
                let since = if seen == 0 {
                    "Not yet".to_string()
                } else if st.target_last > 0 && info.count == st.target_last {
                    "Just now".to_string()
                } else if st.target_last > 0 && info.count > st.target_last {
                    format!("{} ago", theme::thousands(info.count - st.target_last))
                } else {
                    "—".to_string()
                };
                kit::stat(ui, w4, "Last seen", &since);
                kit::stat(
                    ui,
                    w4,
                    "Longest without",
                    &if st.target_gap_max > 0 { theme::thousands(st.target_gap_max) } else { "Not yet".into() },
                );
            });
            ui.add_space(4.0);
        }

        // Fun facts
        kit::section(ui, "Fun facts");
        let rate = st.per_hour();
        let mut facts: Vec<(&str, String, String)> = Vec::new();
        let ss = uses_sweet_scent(&info.method);
        if info.battles > 0 {
            if ss {
                facts.push(("🌸", theme::thousands(info.battles), "Sweet Scents used".into()));
            } else {
                facts.push(("⚔", theme::thousands(info.battles), "Wild battles".into()));
            }
            facts.push((
                "👥",
                format!("{:.2}", info.count as f64 / info.battles as f64),
                "Pokémon per battle".into(),
            ));
        }
        if let Some(s) = st.secs_per_encounter() {
            facts.push(("⏱", format!("1 every {s:.1} s"), "Your encounter loop".into()));
        }
        if let Some(r) = rate {
            facts.push(("⚡", format!("{}/day", theme::thousands((r * 2.0).round() as u64)), "At 2 hours a day".into()));
        }
        let species = st.species.len();
        if species > 0 {
            facts.push(("🔎", species.to_string(), "Different species seen".into()));
        }
        if st.hunt_secs >= 3600 {
            facts.push(("🎬", format!("{:.1}", st.hunt_secs as f64 / 7200.0), "Films' worth (2 h each)".into()));
            facts.push(("✈", format!("{:.1}", st.hunt_secs as f64 / 36_000.0), "Long-haul flights (10 h)".into()));
        }
        if facts.is_empty() {
            kit::card(ui, |ui| note(ui, "Hunt a little longer and some fun numbers show up here."));
        } else {
            let per = if ui.available_width() >= 760.0 { 3 } else { 2 };
            tile_rows(ui, &facts, per, |ui, w, (icon, v, l)| fact(ui, w, icon, v, l));
        }
    }

    // ───────────────────────────── profile records ─────────────────────────────

    /// Profile → Overview: records across every hunt.
    pub(super) fn lifetime_records(&mut self, ui: &mut egui::Ui) {
        // Every run's stats once: current run, saved runs (not the current
        // one again), caught hunts (phases repeat their run, so they're skipped).
        let mut all: Vec<&RunStats> = vec![&self.cfg.run_stats];
        all.extend(self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id).map(|r| &r.stats));
        all.extend(self.cfg.history.iter().filter(|h| !h.phase).filter_map(|h| h.stats.as_ref()));

        let mut species: BTreeMap<u32, u64> = BTreeMap::new();
        let mut days: BTreeMap<i64, u64> = BTreeMap::new();
        let mut fastest: Option<f64> = None;
        for st in &all {
            for (d, n) in &st.species {
                *species.entry(*d).or_insert(0) += n;
            }
            for d in &st.days {
                *days.entry(d.day).or_insert(0) += d.encounters;
            }
            if let Some((_, r)) = st.fastest_session() {
                fastest = Some(fastest.map_or(r, |f: f64| f.max(r)));
            }
        }
        let days_hunted = days.values().filter(|n| **n > 0).count() as u64;
        let tracked: u64 = days.values().sum();
        let most = species.iter().max_by_key(|(_, n)| **n).map(|(d, n)| (*d, *n));

        // Luck of each caught (non-phase) hunt.
        let lucks: Vec<(String, f64, f64)> = self
            .cfg
            .history
            .iter()
            .filter(|h| !h.phase && h.encounters > 0)
            .filter_map(|h| {
                let o = h.odds_one_in.filter(|o| *o > 1.0)?;
                Some((h.target.clone(), 1.0 - stats::found_by(h.encounters, o), h.encounters as f64 / o))
            })
            .collect();
        let luckiest = lucks.iter().max_by(|a, b| a.1.total_cmp(&b.1)).cloned();
        let toughest = lucks.iter().min_by(|a, b| a.1.total_cmp(&b.1)).cloned();

        let mut whens: Vec<u64> = self.cfg.history.iter().map(|h| h.when_unix).filter(|w| *w > 0).collect();
        whens.sort_unstable();
        let longest_wait = whens.windows(2).map(|w| w[1] - w[0]).max();

        let battles = self.cfg.battles
            + self.cfg.saved_runs.iter().filter(|r| r.id != self.cfg.run_id).map(|r| r.battles).sum::<u64>()
            + self.cfg.history.iter().filter(|h| !h.phase).map(|h| h.battles).sum::<u64>();

        let name_of = |dex: u32| self.species.get((dex as usize).saturating_sub(1)).map(|s| s.name.to_string()).unwrap_or_default();
        let tiles: Vec<(String, String)> = vec![
            (
                luckiest.as_ref().map_or("Luckiest catch".into(), |l| format!("Luckiest · beat {}", pct(l.1))),
                luckiest.as_ref().map_or("—".into(), |l| l.0.clone()),
            ),
            (
                toughest.as_ref().map_or("Toughest catch".into(), |t| format!("Toughest · {:.1}× odds", t.2)),
                toughest.as_ref().filter(|_| lucks.len() > 1).map_or("—".into(), |t| t.0.clone()),
            ),
            (
                most.map_or("Most encountered".into(), |(_, n)| format!("Most seen · {}", theme::thousands(n))),
                most.map_or("—".into(), |(d, _)| name_of(d)),
            ),
            ("Species encountered".into(), if species.is_empty() { "—".into() } else { species.len().to_string() }),
            (
                "Avg per day hunted".into(),
                if days_hunted > 0 { theme::thousands(tracked / days_hunted) } else { "—".into() },
            ),
            (
                "Longest shiny wait".into(),
                longest_wait.map_or("—".into(), |s| if s >= 86_400 { format!("{} days", s / 86_400) } else { fmt_duration(s) }),
            ),
            ("Fastest session".into(), fastest.map_or("—".into(), |r| format!("{}/h", theme::thousands(r.round() as u64)))),
            ("Wild battles".into(), theme::thousands(battles)),
        ];
        kit::section(ui, "Records");
        let per = if ui.available_width() >= 1500.0 { 8 } else { 4 };
        tile_rows(ui, &tiles, per, |ui, w, (label, value)| kit::stat(ui, w, label, value));
    }
}

/// Small coloured dot + label for chart legends.
fn legend_dot(ui: &mut egui::Ui, c: Color32, text: &str) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
    ui.painter().circle_filled(r.center(), 4.0, c);
    ui.label(RichText::new(text).size(kit::fs(12.0)).color(MUTED()));
}

/// Recent sessions as a timeline: each a bar sized by its length.
fn session_timeline(ui: &mut egui::Ui, sessions: &[Session], current: bool) {
    let recent: Vec<&Session> = sessions.iter().rev().take(8).collect();
    if recent.is_empty() {
        return;
    }
    let longest = recent.iter().map(|s| s.secs).max().unwrap_or(1).max(1);
    let fastest = recent
        .iter()
        .filter(|s| s.secs >= 600 && s.encounters > 1)
        .map(|s| (s.encounters - 1) as f64 * 3600.0 / s.secs as f64)
        .fold(0.0, f64::max);
    let live = current && recent.first().is_some_and(|s| now_unix().saturating_sub(s.end) <= stats::SESSION_GAP);
    kit::card(ui, |ui| {
        card_title(ui, "Recent sessions", &format!("{} in total", sessions.len()));
        for (k, s) in recent.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), Sense::hover());
            let p = ui.painter_at(rect);
            if resp.hovered() {
                p.rect_filled(rect, 6.0, CARD_HOVER());
            }
            let is_live = k == 0 && live;
            // Column 1: when (and a "Live" badge for the session in progress)
            let when = format!("{}  ·  {}", fmt_day_short(local_day(s.start)), stats::fmt_clock(s.start));
            let wg = p.text(egui::pos2(rect.min.x + 8.0, rect.center().y), Align2::LEFT_CENTER, when, kit::font(13.0), TEXT());
            if is_live {
                tag(&p, egui::pos2(wg.max.x + 26.0, rect.center().y), "Live", accent(), theme::button_ink(), rect);
            }
            // Column 3: numbers in fixed spots so every row lines up
            let rate = (s.secs >= 60 && s.encounters > 1).then(|| (s.encounters - 1) as f64 * 3600.0 / s.secs as f64);
            let col3 = rect.max.x - 230.0;
            p.text(
                egui::pos2(col3, rect.center().y),
                Align2::LEFT_CENTER,
                format!("{} enc.", theme::thousands(s.encounters)),
                kit::font(12.5),
                TEXT(),
            );
            p.text(egui::pos2(col3 + 86.0, rect.center().y), Align2::LEFT_CENTER, fmt_duration(s.secs), kit::font(12.5), MUTED());
            p.text(
                egui::pos2(rect.max.x - 8.0, rect.center().y),
                Align2::RIGHT_CENTER,
                rate.map_or("—".to_string(), |r| format!("{}/h", theme::thousands(r.round() as u64))),
                kit::font(12.5),
                MUTED(),
            );
            // Column 2: the length bar, between the two
            let x0 = (wg.max.x + if is_live { 60.0 } else { 20.0 }).max(rect.min.x + 200.0);
            let x1 = (col3 - 24.0).max(x0 + 30.0);
            let track = Rect::from_min_max(egui::pos2(x0, rect.center().y - 4.0), egui::pos2(x1, rect.center().y + 4.0));
            p.rect_filled(track, 4.0, CARD_HOVER());
            let w = (track.width() * s.secs as f32 / longest as f32).max(4.0);
            let is_fast = rate.is_some_and(|r| fastest > 0.0 && (r - fastest).abs() < 1e-6);
            let c = if is_live { accent() } else if is_fast { theme::blend(neutral(), accent(), 0.5) } else { neutral() };
            p.rect_filled(Rect::from_min_size(track.min, egui::vec2(w, track.height())), 4.0, c);
        }
        note(ui, "Bars show each session's length. The fastest recent one is tinted.");
    });
}
