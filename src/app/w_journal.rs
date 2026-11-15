//! Dashboard widgets for the Journal.

use super::journal_app::JOURNAL_COL;
use super::journal_md as md;
use super::ui_kit::{self as kit, CARD_HOVER, LINE, MUTED, TEXT};
use super::w_build::{Head, Lead, Row, Stat};
use super::w_fit::{self as fit, wbox, Shape};
use crate::apps::WidgetCfg;
use super::App;
use crate::journal as j;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Rect, Sense, Stroke};

impl App {
    fn journal_head(icon: &'static str, title: &'static str) -> Head<'static> {
        Head { icon, col: JOURNAL_COL, title, link: Some("Open ›") }
    }

    /// Where today's note is (or would be).
    fn journal_today_path(&self) -> String {
        let day = crate::stats::local_day(crate::stats::now_unix());
        let (y, m, d) = crate::stats::civil_from_days(day);
        format!("{}/{y:04}-{m:02}-{d:02}.md", j::clean_name(&self.cfg.journal.daily_folder))
    }

    /// The notes you changed last.
    pub(super) fn w_journal_recent(&mut self, ui: &mut egui::Ui) {
        self.journal_scan(false);
        let b = wbox(ui, "journal_recent");
        let mut notes: Vec<(String, u64)> = self.setup.journal.index.iter().map(|(r, i)| (r.clone(), i.modified)).collect();
        notes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let rows: Vec<Row> = notes
            .iter()
            .map(|(rel, t)| {
                let folder = rel.rsplit_once('/').map(|(f, _)| format!("{f} · ")).unwrap_or_default();
                Row::new(j::title_of(rel)).lead(Lead::Icon("📄".into(), JOURNAL_COL)).sub(format!("{folder}{}", super::home::ago(*t))).tip("Open this note")
            })
            .collect();
        let c = self.f_list(ui, &b, Self::journal_head("📄", "Recent notes"), &rows, "No notes yet. Start one in the Journal.");
        if let Some(k) = c.item {
            let rel = notes[k].0.clone();
            self.journal_go(Some(&rel), None);
        } else if c.link || c.more {
            self.journal_go(None, None);
        }
    }

    /// Today's note, as a short read.
    pub(super) fn w_journal_today(&mut self, ui: &mut egui::Ui) {
        self.journal_scan(false);
        let b = wbox(ui, "journal_today");
        let rel = self.journal_today_path();
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, go) = if small { (b.inner(), false) } else { fit::head(ui, &b, "📅", JOURNAL_COL, "Today's note", Some("Open ›")) };
        let text = self.setup.journal.bufs.iter().find(|x| x.rel == rel).map(|x| x.text.clone()).or_else(|| j::read(&j::root(), &rel));
        let resp = ui.interact(body, ui.id().with(("j-today-w", b.r.min.x as i32, b.r.min.y as i32)), Sense::click());
        let p = ui.painter_at(b.r);
        match text {
            None => {
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                let btn = Rect::from_center_size(body.center(), vec2(body.width().min(b.px(240.0)), body.height().min(b.px(44.0))));
                p.rect_filled(btn, btn.height() / 2.0, theme::blend(JOURNAL_COL.gamma_multiply(0.2), JOURNAL_COL.gamma_multiply(0.35), ht));
                fit::hero(&p, btn.shrink2(vec2(b.px(12.0), btn.height() * 0.26)), "+  Start today's note", b.px(15.0), TEXT(), Align2::CENTER_CENTER);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Makes today's note in your daily notes folder").clicked() {
                    let r = self.journal_today_rel();
                    self.journal_go(Some(&r), None);
                }
            }
            Some(text) => {
                let (_, at) = j::front_matter(&text);
                let lines: Vec<&str> = text[at.min(text.len())..].lines().filter(|l| !l.trim().is_empty()).filter(|l| !l.starts_with("# ")).collect();
                let lh = b.px(19.0);
                let mut y = body.min.y + b.px(2.0);
                let mut shown = 0;
                for l in &lines {
                    if y + lh > body.max.y {
                        break;
                    }
                    let t = l.trim_start();
                    let (glyph, rest, bold, col) = if let Some(r) = t.strip_prefix("- [ ] ") {
                        ("☐ ", r, false, TEXT())
                    } else if let Some(r) = t.strip_prefix("- [x] ").or_else(|| t.strip_prefix("- [X] ")) {
                        ("☑ ", r, false, MUTED())
                    } else if let Some(r) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
                        ("•  ", r, false, TEXT())
                    } else if t.starts_with('#') {
                        ("", t.trim_start_matches('#').trim(), true, JOURNAL_COL)
                    } else {
                        ("", t.trim_start_matches('>').trim(), false, theme::blend(TEXT(), MUTED(), 0.25))
                    };
                    let plain: String = md::inline(rest).iter().filter(|s| !s.fx.marker).map(|s| s.text.as_str()).collect();
                    let f = if bold { b.bold(12.5) } else { b.font(12.5) };
                    let line = super::charts::clip(ui, &format!("{glyph}{plain}"), &f, body.width() - b.px(4.0));
                    p.text(pos2(body.min.x, y + lh / 2.0), Align2::LEFT_CENTER, line, f, col);
                    y += lh;
                    shown += 1;
                }
                if shown == 0 {
                    fit::empty(&p, &b, body, "📅", "Nothing written yet today. Click to write.");
                } else if shown < lines.len() && body.max.y - y > b.px(2.0) {
                    p.text(pos2(body.min.x, body.max.y - b.px(2.0)), Align2::LEFT_BOTTOM, format!("+{} more lines", lines.len() - shown), b.font(11.0), MUTED());
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Open today's note").clicked() {
                    self.journal_go(Some(&rel), None);
                }
            }
        }
        if go {
            let r = self.journal_today_rel();
            self.journal_go(Some(&r), None);
        }
    }

    /// Jot a line into today's note without leaving the dashboard.
    pub(super) fn w_journal_quick(&mut self, ui: &mut egui::Ui, w: &WidgetCfg, preview: bool) {
        let b = wbox(ui, "journal_quick");
        let small = b.shape() == Shape::Mini || b.flat();
        let (body, go) = if small { (b.inner(), false) } else { fit::head(ui, &b, "✏", JOURNAL_COL, "Quick note", Some("Open ›")) };
        let id = egui::Id::new(("w-jquick", w.id));
        let mut draft: String = ui.data(|d| d.get_temp(id)).unwrap_or_default();
        let btn_w = b.px(78.0).min(body.width() * 0.35);
        let row_h = b.px(34.0).min(body.height());
        let row = Rect::from_min_size(pos2(body.min.x, body.min.y + ((body.height() - row_h) / 2.0).min(b.px(4.0))), vec2(body.width(), row_h));
        let field = Rect::from_min_max(row.min, pos2(row.max.x - btn_w - b.px(6.0), row.max.y));
        let btn = Rect::from_min_max(pos2(field.max.x + b.px(6.0), row.min.y), row.max);
        let p = ui.painter_at(b.r);
        p.rect_filled(field, b.px(9.0), kit::PAGE());
        p.rect_stroke(field, b.px(9.0), Stroke::new(1.0f32, LINE()));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field.shrink2(vec2(b.px(8.0), 0.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let te = child.add(egui::TextEdit::singleline(&mut draft).id(id.with("edit")).frame(false).hint_text("A thought for today's note…").desired_width(field.width() - b.px(16.0)).font(b.font(13.0)));
        let enter = te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let ready = !draft.trim().is_empty() && !preview;
        let bresp = ui.interact(btn, id.with("add"), Sense::click());
        let ht = kit::anim(ui.ctx(), bresp.id.with("h"), bresp.hovered() && ready, kit::QUICK);
        p.rect_filled(btn, b.px(9.0), if ready { theme::blend(JOURNAL_COL, egui::Color32::WHITE, 0.1 * ht) } else { CARD_HOVER() });
        p.text(btn.center(), Align2::CENTER_CENTER, "Add", b.bold(13.0), if ready { egui::Color32::WHITE } else { MUTED() });
        let below = Rect::from_min_max(pos2(body.min.x, row.max.y + b.px(6.0)), body.max);
        let added: Option<u64> = ui.data(|d| d.get_temp(id.with("added")));
        let note = match added {
            Some(t) if crate::stats::now_unix().saturating_sub(t) < 4 => "✓ Added to today's note".to_string(),
            _ => "Goes to the end of today's note, with the time.".to_string(),
        };
        if below.height() > b.px(14.0) {
            fit::note(&p, below, &note, b.font(11.0));
        }
        if ready && (enter || bresp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Add it to today's note").clicked()) {
            let rel = self.journal_today_rel();
            let line = format!("- {}  {}", crate::stats::fmt_clock(crate::stats::now_unix()), draft.trim());
            self.journal_append(&rel, &line);
            draft.clear();
            ui.data_mut(|d| d.insert_temp(id.with("added"), crate::stats::now_unix()));
            ui.ctx().request_repaint_after(std::time::Duration::from_secs(4));
        }
        ui.data_mut(|d| d.insert_temp(id, draft));
        if go {
            let r = self.journal_today_rel();
            self.journal_go(Some(&r), None);
        }
    }

    /// Unticked tasks from every note, newest note first.
    pub(super) fn w_journal_tasks(&mut self, ui: &mut egui::Ui) {
        self.journal_scan(false);
        let b = wbox(ui, "journal_tasks");
        let mut notes: Vec<(&String, &super::journal_app::NoteInfo)> = self.setup.journal.index.iter().filter(|(_, i)| !i.tasks.is_empty()).collect();
        notes.sort_by(|a, b| b.1.modified.cmp(&a.1.modified).then(a.0.cmp(b.0)));
        let mut refs: Vec<(String, usize)> = Vec::new();
        let mut rows: Vec<Row> = Vec::new();
        for (rel, info) in notes {
            for (line, t) in &info.tasks {
                let plain: String = md::inline(t).iter().filter(|s| !s.fx.marker).map(|s| s.text.as_str()).collect();
                rows.push(Row::new(plain).lead(Lead::Icon("☐".into(), JOURNAL_COL)).sub(j::title_of(rel)).tip("Open it in its note (right-click to tick it off)").menu("✔ Tick it off"));
                refs.push((rel.clone(), *line));
            }
        }
        let c = self.f_list(ui, &b, Self::journal_head("✔", "Open tasks"), &rows, "No open tasks. Write - [ ] in any note to add one.");
        if let Some(k) = c.menu {
            let (rel, line) = refs[k].clone();
            self.journal_tick(&rel, line);
        } else if let Some(k) = c.item {
            let (rel, line) = refs[k].clone();
            self.journal_go(Some(&rel), Some(line));
        } else if c.link || c.more {
            self.journal_go(None, None);
        }
    }

    /// Tick a task in a note (open or not).
    fn journal_tick(&mut self, rel: &str, line: usize) {
        if let Some(b) = self.setup.journal.bufs.iter_mut().find(|b| b.rel == rel) {
            b.text = md::toggle_task(&b.text, line);
            b.edited = Some(std::time::Instant::now());
        } else if let Some(t) = j::read(&j::root(), rel) {
            let _ = j::write(&j::root(), rel, &md::toggle_task(&t, line));
        }
        self.journal_flush(true);
        self.setup.journal.scanned = None;
    }

    /// Notes, words, tasks.
    pub(super) fn w_journal_stats(&mut self, ui: &mut egui::Ui) {
        self.journal_scan(false);
        let b = wbox(ui, "journal_stats");
        let idx = &self.setup.journal.index;
        let words: usize = idx.values().map(|i| i.words).sum();
        let tasks: usize = idx.values().map(|i| i.tasks.len()).sum();
        let tags: std::collections::BTreeSet<&String> = idx.values().flat_map(|i| i.tags.iter()).collect();
        let last = idx.values().map(|i| i.modified).max();
        let stats = [Stat::new(idx.len().to_string(), "notes", JOURNAL_COL), Stat::new(theme::thousands(words as u64), "words", kit::C_SLATE), Stat::new(tasks.to_string(), "open tasks", kit::C_GOLD)];
        let note = format!("{} tag{}{}", tags.len(), if tags.len() == 1 { "" } else { "s" }, last.map_or(String::new(), |t| format!(" · last written {}", super::home::ago(t))));
        if self.f_stats(ui, &b, Self::journal_head("📓", "Journal in numbers"), &stats, Some(&note)) {
            self.journal_go(None, None);
        }
    }
}
