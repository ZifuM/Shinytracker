//! Small pieces the Dashboard widgets share: the card heading, a hover row,
//! type pills, and pickers for a Pokémon or a place that live inside a
//! widget.

use super::ui_kit::{self as kit, CARD_HOVER, MUTED, TEXT};
use super::App;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense};

/// A widget's heading: icon, title and a link on the right. True when the
/// link was clicked.
pub(super) fn head(ui: &mut egui::Ui, icon: &str, col: Color32, title: &str, link: Option<&str>) -> bool {
    super::profile_page::card_head(ui, icon, col, title, link)
}

/// Muted small text that wraps.
pub(super) fn note(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).size(kit::fs(12.5)).color(MUTED()));
}

/// A full-width row that lights up on hover. Returns its rect and response.
pub(super) fn row(ui: &mut egui::Ui, h: f32) -> (Rect, egui::Response) {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
    if ht > 0.0 {
        ui.painter().rect_filled(r.expand2(vec2(6.0, 0.0)), 10.0, CARD_HOVER().gamma_multiply(ht));
    }
    (r, resp.on_hover_cursor(egui::CursorIcon::PointingHand))
}

/// A type label painted at `at` (left-centre). Returns its rect.
pub(super) fn type_tag(p: &egui::Painter, at: egui::Pos2, t: &str, extra: &str) -> Rect {
    let c = super::pokedex::type_color(t);
    let label = if extra.is_empty() { t.to_uppercase() } else { format!("{}  {extra}", t.to_uppercase()) };
    let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    let ink = if lum > 150.0 { Color32::from_rgb(34, 34, 40) } else { Color32::WHITE };
    let g = p.layout_no_wrap(label, kit::font_bold(10.0), ink);
    let r = Rect::from_min_size(pos2(at.x, at.y - (g.size().y + 6.0) / 2.0), g.size() + vec2(14.0, 6.0));
    p.rect_filled(r, r.height() / 2.0, c);
    p.galley(r.min + vec2(7.0, 3.0), g, ink);
    r
}

/// A small text button.
pub(super) fn link(ui: &mut egui::Ui, text: &str) -> bool {
    ui.link(RichText::new(text).size(kit::fs(12.5))).clicked()
}

/// "1,234"
pub(super) fn n(v: u64) -> String {
    theme::thousands(v)
}

impl App {
    /// A widget's working text (a search box…), kept while the app is open.
    pub(super) fn wtext(&mut self, id: u64, key: &'static str) -> &mut String {
        self.setup.home.text.entry((id, key)).or_default()
    }

    /// Type to find a Pokémon; the matches show under the box. Returns the
    /// wiki id of the one clicked.
    pub(super) fn mon_picker(&mut self, ui: &mut egui::Ui, id: u64, key: &'static str, hint: &str) -> Option<String> {
        let w = self.wiki.clone()?;
        let mut q = self.wtext(id, key).clone();
        kit::search_box_keyed(ui, Some(egui::Id::new(("w-pick", id, key))), &mut q, hint, ui.available_width().min(320.0));
        *self.wtext(id, key) = q.clone();
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return None;
        }
        let num: Option<u32> = q.trim_start_matches('#').parse().ok();
        let mut hits: Vec<(String, String, u32)> = w
            .all()
            .filter(|m| num.map_or_else(|| m.name.to_lowercase().contains(&q), |d| m.dex == d))
            .map(|m| (m.id.clone(), m.name.clone(), m.dex))
            .collect();
        hits.sort_by_key(|(_, name, _)| (!name.to_lowercase().starts_with(&q), name.len()));
        hits.truncate(5);
        let mut picked = None;
        ui.add_space(4.0);
        if hits.is_empty() {
            note(ui, "No Pokémon by that name.");
        }
        for (wid, name, dex) in hits {
            let (r, resp) = row(ui, 34.0);
            let p = ui.painter().clone();
            self.team_sprite(&p, &wid, false, pos2(r.min.x + 18.0, r.center().y), vec2(32.0, 32.0));
            p.text(pos2(r.min.x + 42.0, r.center().y), Align2::LEFT_CENTER, &name, crate::fonts::sans_bold_font(13.0), TEXT());
            p.text(pos2(r.max.x - 4.0, r.center().y), Align2::RIGHT_CENTER, format!("#{dex:03}"), crate::fonts::sans_font(11.5), MUTED());
            if resp.clicked() {
                picked = Some(wid);
            }
        }
        if picked.is_some() {
            self.wtext(id, key).clear();
        }
        picked
    }

    /// Type to find a place (as the Game Info groups them). Returns the
    /// place's name.
    pub(super) fn place_picker(&mut self, ui: &mut egui::Ui, id: u64, key: &'static str, hint: &str) -> Option<String> {
        let w = self.wiki.clone()?;
        let mut q = self.wtext(id, key).clone();
        kit::search_box_keyed(ui, Some(egui::Id::new(("w-place", id, key))), &mut q, hint, ui.available_width().min(320.0));
        *self.wtext(id, key) = q.clone();
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return None;
        }
        let mut hits: Vec<String> = super::info_places::groups(&w).into_iter().map(|g| g.name).filter(|n| n.to_lowercase().contains(&q)).collect();
        hits.sort_by_key(|n| (!n.to_lowercase().starts_with(&q), n.len()));
        hits.truncate(5);
        let mut picked = None;
        ui.add_space(4.0);
        if hits.is_empty() {
            note(ui, "No place by that name.");
        }
        for name in hits {
            let (r, resp) = row(ui, 30.0);
            ui.painter().text(pos2(r.min.x + 6.0, r.center().y), Align2::LEFT_CENTER, format!("📍  {name}"), crate::fonts::sans_font(13.0), TEXT());
            if resp.clicked() {
                picked = Some(name);
            }
        }
        if picked.is_some() {
            self.wtext(id, key).clear();
        }
        picked
    }

    /// A Pokémon's display name from its wiki id.
    pub(super) fn mon_name(&self, id: &str) -> String {
        self.wiki.as_ref().and_then(|w| w.mon(id).map(|m| m.name.clone())).unwrap_or_else(|| id.to_string())
    }
}

