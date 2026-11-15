//! A note and a canvas of its own for a journal elsewhere in the app (a
//! hunt's, the Pokégotchi's), switched with a toggle and edited right there.
//! Each is one file in the Journal, made the first time it's wanted; the
//! link to it is kept in the Journal's settings and follows renames.

use super::journal_app::JOURNAL_COL;
use super::ui_kit::{self as kit, CARD, LINE, MUTED, TEXT};
use super::App;
use crate::config::JLink;
use crate::journal as j;
use eframe::egui::{self, vec2, Align2, Rect, RichText, Sense, Stroke};

/// How tall the note or canvas is, in a page.
const HEIGHT: f32 = 600.0;

impl App {
    /// The toggle and the note or canvas under it. `key` names the journal
    /// ("run:…", "gotchi"), `name` is what a new file is called, in `folder`.
    pub(super) fn linked_journal(&mut self, ui: &mut egui::Ui, key: &str, name: &str, folder: &str, whose: &str) {
        let have = self.has_app(crate::apps::AppId::Journal);
        let link: JLink = self.cfg.journal.links.get(key).cloned().unwrap_or_default();
        let canvas = link.canvas_view;
        let rel = if canvas { link.canvas.clone() } else { link.note.clone() };
        if have {
            self.journal_scan(false);
        }
        let root = j::root();
        let exists = rel.as_ref().is_some_and(|r| root.join(r).is_file());
        // the heading, the toggle, and a way into the Journal
        let mut view = canvas;
        let mut open_tab = false;
        ui.horizontal(|ui| {
            kit::section(ui, if canvas { "Canvas" } else { "Note" });
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(vec2(200.0, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                kit::segmented(ui, &mut view, &[(false, "Note"), (true, "Canvas")]);
            });
            if exists && have {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.link(RichText::new("📓 Open as a tab in Journal ›").size(kit::fs(12.5))).on_hover_text("The same file, in the Journal app").clicked() {
                        open_tab = true;
                    }
                });
            }
        });
        if view != canvas {
            let l = self.cfg.journal.links.entry(key.to_string()).or_default();
            l.canvas_view = view;
            self.dirty = true;
        }
        if open_tab {
            if let Some(r) = &rel {
                self.journal_go(None, None);
                self.journal_open_any(r);
            }
            return;
        }
        ui.add_space(6.0);
        let w = ui.available_width();
        let h = if exists && have { HEIGHT } else { 190.0 };
        let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(r, 14.0, CARD());
        p.rect_stroke(r, 14.0, Stroke::new(1.0f32, LINE()));
        let inner = r.shrink(1.0);
        if !have {
            let c = r.center();
            p.text(c - vec2(0.0, 30.0), Align2::CENTER_CENTER, "📓", egui::FontId::proportional(28.0), JOURNAL_COL);
            p.text(c + vec2(0.0, 6.0), Align2::CENTER_CENTER, format!("A note and a canvas for {whose} live in the Journal app"), crate::fonts::sans_font(13.5), TEXT());
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(Rect::from_center_size(c + vec2(0.0, 48.0), vec2(220.0, 34.0))).layout(egui::Layout::top_down(egui::Align::Center)), |ui| {
                super::app_modal::lock_btn(ui, crate::apps::AppId::Journal, "Get the Journal");
            });
            return;
        }
        match (exists, &rel) {
            (true, Some(rel)) => {
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                    ui.set_clip_rect(inner.intersect(ui.clip_rect()));
                    if canvas {
                        let i = self.journal_cbuf(rel);
                        self.journal_canvas(ui, i);
                    } else {
                        let i = self.journal_buf(rel);
                        self.journal_note(ui, i);
                    }
                });
            }
            _ => {
                // nothing yet (or it was moved or deleted): make one
                let c = r.center();
                let what = if canvas { "canvas" } else { "note" };
                let gone = rel.is_some();
                p.text(c - vec2(0.0, 44.0), Align2::CENTER_CENTER, if canvas { "▣" } else { "📝" }, egui::FontId::proportional(26.0), JOURNAL_COL);
                p.text(
                    c - vec2(0.0, 12.0),
                    Align2::CENTER_CENTER,
                    if gone { format!("This {what} was moved or deleted") } else { format!("No {what} for {whose} yet") },
                    crate::fonts::sans_bold_font(14.5),
                    TEXT(),
                );
                let ext = if canvas { "canvas" } else { "md" };
                let new_rel = j::free_rel_ext(&root, folder, name, ext);
                let sub = if canvas { "An endless board for cards, pictures and live widgets." } else { "A page of your own, in Markdown." };
                p.text(c + vec2(0.0, 12.0), Align2::CENTER_CENTER, format!("{sub} Saved in your Journal as {new_rel}"), crate::fonts::sans_font(12.0), MUTED());
                let br = Rect::from_center_size(c + vec2(0.0, 52.0), vec2(170.0, 36.0));
                if super::store::pill_button(ui, br, if canvas { "+  Make a canvas" } else { "+  Make a note" }, true, ("jlink-make", key)).clicked() {
                    let text = if canvas { crate::jcanvas::Canvas::default().to_json() } else { format!("# {name}\n\n") };
                    match j::write(&root, &new_rel, &text) {
                        Ok(()) => {
                            let l = self.cfg.journal.links.entry(key.to_string()).or_default();
                            if canvas {
                                l.canvas = Some(new_rel);
                            } else {
                                l.note = Some(new_rel);
                            }
                            self.dirty = true;
                            self.journal_scan(true);
                        }
                        Err(e) => self.setup.journal.error = Some(format!("Couldn't make the {what}: {e}")),
                    }
                }
            }
        }
    }
}
