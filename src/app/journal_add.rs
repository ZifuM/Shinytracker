//! "Add to Journal" from anywhere: a Pokédex entry, a Game Info page, a
//! Map place, a widget, a share image, a logged shiny or a hunt. Each is
//! made into a clip (a card, a one-line link and a full page of Markdown,
//! maybe a picture or a live widget) and the popup puts it in a note or a
//! canvas. Links in a clip (`ptc://dex/GIBLE`) open the app's own page.

use super::journal_app::JOURNAL_COL;
use super::store::pill_button;
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::{App, RunRef, Tab};
use crate::jcanvas::{self, Canvas};
use crate::journal as j;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};
use std::time::{Duration, Instant};

/// What's being added.
#[derive(Clone, Default, Debug)]
pub(super) struct Clip {
    /// "Gible"
    pub title: String,
    /// "Pokédex entry"
    pub what: String,
    /// A short block (a callout in a note, a card on a canvas).
    pub card: String,
    /// One line.
    pub link: String,
    /// Everything there is.
    pub full: String,
    /// A picture to keep in the Journal: (png bytes, file name).
    pub image: Option<(Vec<u8>, String)>,
    /// A live widget (canvases only).
    pub widget: Option<&'static str>,
}

/// Where a clip goes.
#[derive(Clone, PartialEq, Debug)]
pub(super) enum Target {
    Note(String),
    Canvas(String),
    Today,
    NewNote,
    NewCanvas,
}

/// How it's added.
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub(super) enum How {
    #[default]
    Card,
    Link,
    Full,
}

#[derive(Default)]
pub(super) struct AddState {
    pub clip: Option<Clip>,
    pub target: Option<Target>,
    pub how: How,
    pub query: String,
    /// "Added to …" with what to open.
    pub toast: Option<(Instant, String, Option<Target>)>,
}

/// A `ptc://` link's address part, with spaces kept safe.
fn enc(s: &str) -> String {
    s.replace('%', "%25").replace(' ', "%20").replace(')', "%29")
}

/// The app link to a Game Info place.
pub(super) fn place_link(name: &str) -> String {
    format!("ptc://info/{}", enc(&format!("place:{name}")))
}

fn dec(s: &str) -> String {
    s.replace("%20", " ").replace("%29", ")").replace("%25", "%")
}

/// A callout block from a title and lines.
fn callout(kind: &str, title: &str, lines: &[String]) -> String {
    let mut s = format!("> [!{kind}] {title}\n");
    for l in lines.iter().filter(|l| !l.trim().is_empty()) {
        s.push_str("> ");
        s.push_str(l);
        s.push('\n');
    }
    s
}

impl App {
    /// Open the popup for a clip.
    pub(super) fn journal_add(&mut self, clip: Clip) {
        let st = &mut self.setup.jadd;
        // a widget only goes on a canvas
        if clip.widget.is_some() && !matches!(st.target, Some(Target::Canvas(_)) | Some(Target::NewCanvas)) {
            st.target = None;
        }
        st.how = How::Card;
        st.query.clear();
        st.clip = Some(clip);
        self.journal_scan(true);
    }

    /// Follow a `ptc://` link to the app's own page.
    pub(super) fn open_ptc(&mut self, url: &str) -> bool {
        let Some(rest) = url.strip_prefix("ptc://") else { return false };
        let (kind, key) = rest.split_once('/').unwrap_or((rest, ""));
        let key = dec(key);
        match kind {
            "dex" if !key.is_empty() => self.open_dex_entry(&key),
            "info" => match super::info_home::Page::from_key(&key) {
                Some(super::info_home::Page::Guide(g)) => {
                    self.go_to(Tab::Info);
                    self.info_read(&g, None, None);
                }
                Some(p) => self.info_open_page(p),
                None => self.go_to(Tab::Info),
            },
            "runs" => {
                self.go_to(Tab::Runs);
                self.setup.view_run = None;
            }
            "trophies" => self.go_to(Tab::Runs),
            _ => return false,
        }
        true
    }

    // ───────────────────────────── clips ─────────────────────────────

    /// A Pokémon, by its Game Info id ("GIBLE").
    pub(super) fn dex_clip(&mut self, id: &str) -> Option<Clip> {
        let w = self.wiki.clone()?;
        let m = w.mon(id)?.clone();
        let sp = self.species.iter().find(|s| s.dex == m.dex).cloned();
        let extra = sp.as_ref().and_then(|s| match self.dexdb.get(&s.slug) {
            crate::dexextra::State::Ready(e) => Some(*e),
            _ => None,
        });
        let types = m.types.iter().map(|t| title_case(t)).collect::<Vec<_>>().join(" / ");
        let link = format!("ptc://dex/{}", enc(&m.id));
        let mut wild: Vec<(String, String, String, u32)> = Vec::new();
        for e in w.wild(&m.id).into_iter().filter(|e| e.setting.is_empty()) {
            let place = w.place(&e.area).map(|p| p.name.clone()).unwrap_or_else(|| e.area.clone());
            let best = e.chance.morning.max(e.chance.day).max(e.chance.night);
            let how = super::dex_tabs::method_label(&e.method).to_string();
            match wild.iter_mut().find(|x| x.0 == place && x.1 == how) {
                Some(x) => x.3 = x.3.max(best),
                None => wild.push((place, how, e.levels.clone(), best)),
            }
        }
        wild.sort_by(|a, b| b.3.cmp(&a.3));
        let cat = extra.as_ref().map(|e| e.category.clone()).filter(|c| !c.is_empty());
        let mut lines = vec![format!("{types}{}", cat.as_ref().map(|c| format!(" · {c}")).unwrap_or_default())];
        match wild.first() {
            Some((p, how, _, ch)) => lines.push(format!("Most common at {p} ({how}) · {ch}%")),
            None => lines.push("Not found in the wild".into()),
        }
        lines.push(format!("[Open in the Pokédex]({link})"));
        let card = callout("pokedex", &format!("{} · #{:04}", m.name, m.dex), &lines);
        let mut full = format!("## {}\n\n#{:04} · {types}{}\n\n", m.name, m.dex, cat.map(|c| format!(" · {c}")).unwrap_or_default());
        if let Some(e) = &extra {
            if e.stats.len() == 6 {
                full.push_str("| HP | Atk | Def | SpA | SpD | Spe | Total |\n| --- | --- | --- | --- | --- | --- | --- |\n");
                let s = &e.stats;
                full.push_str(&format!("| {} | {} | {} | {} | {} | {} | {} |\n\n", s[0], s[1], s[2], s[3], s[4], s[5], s.iter().sum::<u32>()));
            }
            if !e.abilities.is_empty() {
                let hidden = if e.hidden_ability.is_empty() { String::new() } else { format!(" (hidden: {})", e.hidden_ability) };
                full.push_str(&format!("- **Abilities:** {}{hidden}\n", e.abilities.join(", ")));
            }
            for (k, v) in [("Catch rate", &e.catch_rate), ("EV yield", &e.ev_yield), ("Growth", &e.growth_rate), ("Gender", &e.gender)] {
                if !v.is_empty() {
                    full.push_str(&format!("- **{k}:** {v}\n"));
                }
            }
            if !e.egg_groups.is_empty() {
                full.push_str(&format!("- **Egg groups:** {}\n", e.egg_groups.join(", ")));
            }
            full.push('\n');
        }
        full.push_str("### Where to find it\n\n");
        if wild.is_empty() {
            let other = w.other(&m.id);
            match other.first() {
                Some(o) if !o.how.is_empty() => full.push_str(&format!("- {}{}\n", o.how, if o.place.is_empty() { String::new() } else { format!(" · {}", o.place) })),
                _ => full.push_str("- Not in the wild\n"),
            }
        }
        for (p, how, lv, ch) in wild.iter().take(10) {
            full.push_str(&format!("- {p} · {how}{} · {ch}%\n", if lv.is_empty() { String::new() } else { format!(" · Lv {lv}") }));
        }
        full.push_str(&format!("\n[Open in the Pokédex]({link})\n"));
        Some(Clip { title: m.name.clone(), what: "Pokédex entry".into(), card, link: format!("[{}]({link}) (#{:04}, {types})", m.name, m.dex), full, ..Default::default() })
    }

    /// A Game Info page (a move, an item, a place or a guide).
    pub(super) fn info_clip(&mut self, page: &super::info_home::Page) -> Option<Clip> {
        use super::info_home::Page;
        let w = self.wiki.clone()?;
        let key = page.key()?;
        let link = format!("ptc://info/{}", enc(&key));
        let open = format!("[Open in Game Info]({link})");
        match page {
            Page::Move(k) => {
                let m = w.move_info(k)?;
                let nums = format!("Power {} · Accuracy {} · PP {}", dash(&m.p), dash(&m.a), dash(&m.pp));
                let card = callout("move", &m.n, &[format!("{} · {}", title_case(&m.t), title_case(&m.c)), nums.clone(), m.d.clone(), open.clone()]);
                let full = format!("## {}\n\n{} · {}\n\n{nums}\n\n{}\n\n{open}\n", m.n, title_case(&m.t), title_case(&m.c), m.d);
                Some(Clip { title: m.n.clone(), what: "Move".into(), card, link: format!("[{}]({link}) ({} move)", m.n, title_case(&m.t)), full, ..Default::default() })
            }
            Page::Item(id) => {
                let it = w.data.items.iter().find(|i| &i.id == id)?;
                let price = it.price.trim();
                let mut lines = vec![it.pocket.clone(), it.description.clone()];
                if !price.is_empty() && price != "0" {
                    lines.push(format!("Price: {price}"));
                }
                lines.push(open.clone());
                let card = callout("item", &it.name, &lines);
                let mut full = format!("## {}\n\n*{}*\n\n{}\n\n", it.name, it.pocket, it.description);
                if !price.is_empty() && price != "0" {
                    full.push_str(&format!("- **Price:** {price}\n\n"));
                }
                full.push_str(&format!("{open}\n"));
                Some(Clip { title: it.name.clone(), what: "Item".into(), card, link: format!("[{}]({link})", it.name), full, ..Default::default() })
            }
            Page::Place(name) => Some(self.place_clip(&w, name, &link)),
            Page::Guide(id) => {
                let g = w.guide(id)?;
                let url = format!("ptc://info/{}", enc(&format!("guide:{}", g.id)));
                let card = callout("guide", &g.title, &[g.summary.clone(), format!("[Read the guide]({url})")]);
                let full = format!("## {}\n\n{}\n\n[Read the guide]({url})\n", g.title, g.summary);
                Some(Clip { title: g.title.clone(), what: "Guide".into(), card, link: format!("[{}]({url})", g.title), full, ..Default::default() })
            }
            _ => None,
        }
    }

    /// A place: who lives there, by how you meet them.
    pub(super) fn place_clip(&mut self, w: &crate::wiki::Wiki, name: &str, link: &str) -> Clip {
        let areas: Vec<String> = w.data.places.iter().filter(|(k, p)| p.name == name || p.group == name || *k == name).map(|(k, _)| k.clone()).collect();
        let mut by: Vec<(String, Vec<(String, u32)>)> = Vec::new();
        for e in w.data.wild.iter().filter(|e| areas.contains(&e.area) && e.setting.is_empty()) {
            let Some(mon) = w.mon(&e.species) else { continue };
            let how = super::dex_tabs::method_label(&e.method).to_string();
            let best = e.chance.morning.max(e.chance.day).max(e.chance.night);
            let list = match by.iter().position(|(h, _)| *h == how) {
                Some(i) => &mut by[i].1,
                None => {
                    by.push((how, Vec::new()));
                    &mut by.last_mut().unwrap().1
                }
            };
            match list.iter_mut().find(|(n, _)| *n == mon.name) {
                Some(x) => x.1 += best,
                None => list.push((mon.name.clone(), best)),
            }
        }
        for (_, l) in by.iter_mut() {
            l.sort_by(|a, b| b.1.cmp(&a.1));
        }
        let open = format!("[Open in Game Info]({link})");
        let mut lines: Vec<String> = by.iter().take(3).map(|(h, l)| format!("**{h}:** {}", l.iter().take(5).map(|(n, _)| n.clone()).collect::<Vec<_>>().join(", "))).collect();
        if lines.is_empty() {
            lines.push("No wild Pokémon here".into());
        }
        let today: Vec<String> = self.rare_spots_today().iter().filter(|s| s.place.eq_ignore_ascii_case(name) || s.place.starts_with(&format!("{name} "))).map(|s| s.species.name.to_string()).collect();
        if !today.is_empty() {
            lines.push(format!("🌿 Rare grass today: {}", today.join(", ")));
        }
        lines.push(open.clone());
        let card = callout("place", name, &lines);
        let mut full = format!("## {name}\n\n");
        for (h, l) in &by {
            full.push_str(&format!("### {h}\n\n"));
            for (n, c) in l {
                full.push_str(&format!("- {n} · {c}%\n"));
            }
            full.push('\n');
        }
        full.push_str(&format!("{open}\n"));
        Clip { title: name.to_string(), what: "Place".into(), card, link: format!("[{name}]({link})"), full, ..Default::default() }
    }

    /// A Dashboard widget, live on a canvas.
    pub(super) fn widget_clip(&self, kind: &'static str) -> Option<Clip> {
        let d = super::widgets::def(kind)?;
        Some(Clip {
            title: d.name.to_string(),
            what: format!("Live widget · {}", d.app.map_or("Dashboard", |a| a.name())),
            card: format!("**{}**", d.name),
            link: format!("**{}**", d.name),
            full: format!("**{}**", d.name),
            widget: Some(d.kind),
            ..Default::default()
        })
    }

    /// A hunt (finished, saved or the current one), with its share image.
    pub(super) fn run_clip(&mut self, run: RunRef, with_image: bool) -> Option<Clip> {
        let info = self.run_info(run)?;
        let image = if with_image {
            let img = self.render_card(&info);
            crate::share::png_bytes(&img).map(|b| (b, format!("{} {}", info.name, info.count)))
        } else {
            None
        };
        let mult = info.odds.filter(|o| *o > 1.0).map(|o| info.count as f64 / o);
        let mut facts = vec![format!("{} encounters", theme::thousands(info.count))];
        if !info.method.is_empty() {
            facts.push(info.method.clone());
        }
        if let Some(m) = mult {
            facts.push(format!("{m:.2}× the odds"));
        }
        let when = match info.ended {
            Some(t) if info.caught => format!("Caught {}", crate::stats::fmt_date(t)),
            _ => format!("Started {}", crate::stats::fmt_date(info.started)),
        };
        let title = if info.caught { format!("Shiny {}", info.name) } else { format!("{} hunt", info.name) };
        let mut lines = vec![facts.join(" · "), when.clone()];
        if let Some(st) = &info.stats {
            if st.hunt_secs >= 60 {
                lines.push(format!("{} hunting", crate::stats::fmt_duration(st.hunt_secs)));
            }
        }
        let kind = if info.caught { "shiny" } else { "hunt" };
        let card = callout(kind, &title, &lines);
        let mut full = format!("## {title}\n\n");
        for l in &lines {
            full.push_str(&format!("- {l}\n"));
        }
        if let Some(st) = &info.stats {
            if !st.notes.is_empty() {
                full.push_str("\n### Hunt journal\n\n");
                for n in &st.notes {
                    full.push_str(&format!("- {} · {}\n", crate::stats::fmt_date(n.unix), n.text.trim()));
                }
            }
        }
        Some(Clip { title: title.clone(), what: if info.caught { "Shiny caught".into() } else { "Hunt".into() }, card, link: format!("**{title}** · {}", facts.join(" · ")), full, image, ..Default::default() })
    }

    /// A shiny logged by hand.
    pub(super) fn logged_clip(&mut self, id: u64) -> Option<Clip> {
        let l = self.cfg.logged.iter().find(|l| l.id == id)?.clone();
        let w = self.wiki.clone();
        let m = &l.mon;
        let name = if m.nickname.trim().is_empty() { m.species.clone() } else { format!("{} ({})", m.nickname.trim(), m.species) };
        let dex_id = w.as_ref().and_then(|w| w.data.species.iter().find(|s| s.name.eq_ignore_ascii_case(&m.species)).map(|s| s.id.clone()));
        let mut lines = vec![format!("Caught {}", crate::stats::fmt_date(l.when_unix))];
        lines.push(l.encounters.map_or("Encounters N/A".into(), |n| format!("{} encounters", theme::thousands(n))));
        let mut more = Vec::new();
        if !l.method.is_empty() {
            more.push(l.method.clone());
        }
        if !l.place.trim().is_empty() {
            more.push(l.place.trim().to_string());
        }
        if !more.is_empty() {
            lines.push(more.join(" · "));
        }
        let mut build = Vec::new();
        if l.nature_known && !m.nature.is_empty() {
            build.push(format!("{} nature", m.nature));
        }
        if !m.ability.is_empty() {
            build.push(m.ability.clone());
        }
        if l.level_known {
            build.push(format!("Lv {}", m.level));
        }
        if !build.is_empty() {
            lines.push(build.join(" · "));
        }
        let moves: Vec<String> = m.moves.iter().filter(|x| !x.is_empty()).map(|k| w.as_ref().and_then(|w| w.move_info(k).map(|mv| mv.n.clone())).unwrap_or_else(|| k.clone())).collect();
        if !moves.is_empty() {
            lines.push(format!("Moves: {}", moves.join(", ")));
        }
        if let Some(id) = &dex_id {
            lines.push(format!("[Open in the Pokédex](ptc://dex/{})", enc(id)));
        }
        let title = format!("Shiny {name}");
        let card = callout("shiny", &title, &lines);
        let mut full = format!("## {title}\n\n");
        for l2 in &lines {
            full.push_str(&format!("- {l2}\n"));
        }
        if l.stats_known {
            let s = ["HP", "Atk", "Def", "SpA", "SpD", "Spe"];
            full.push_str(&format!("\n| | {} |\n| --- |{}\n", s.join(" | "), " --- |".repeat(6)));
            full.push_str(&format!("| IVs | {} |\n", m.ivs.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" | ")));
            full.push_str(&format!("| EVs | {} |\n", m.evs.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" | ")));
        }
        if !m.notes.trim().is_empty() {
            full.push_str(&format!("\n{}\n", m.notes.trim()));
        }
        Some(Clip { title: title.clone(), what: "Logged shiny".into(), card, link: format!("**{title}** · {}", lines[0]), full, ..Default::default() })
    }

    /// A share image that's been made.
    pub(super) fn image_clip(&mut self, title: &str, png: Vec<u8>) -> Clip {
        Clip { title: title.to_string(), what: "Share image".into(), card: String::new(), link: String::new(), full: String::new(), image: Some((png, title.to_string())), ..Default::default() }
    }

    // ───────────────────────────── putting it in ─────────────────────────────

    /// Save the clip's picture in the Journal (once) and give its `rel`.
    fn clip_image(&mut self, clip: &mut Clip) -> Option<String> {
        let (bytes, name) = clip.image.take()?;
        let root = j::root();
        let rel = j::free_rel_ext(&root, j::ATTACH, &name, "png");
        match j::write_bytes(&root, &rel, &bytes) {
            Ok(()) => Some(rel),
            Err(e) => {
                self.setup.journal.error = Some(format!("Couldn't save the picture: {e}"));
                None
            }
        }
    }

    /// Put the clip where it was asked to go. Returns where it went.
    pub(super) fn journal_put(&mut self, mut clip: Clip, target: Target, how: How) -> Option<Target> {
        let img = self.clip_image(&mut clip);
        let embed = img.as_ref().map(|r| format!("![[{r}]]\n"));
        // the Markdown for a note
        let body = |how: How| -> String {
            let text = match how {
                How::Card => clip.card.clone(),
                How::Link => format!("- {}\n", clip.link),
                How::Full => clip.full.clone(),
            };
            match (&embed, how) {
                (Some(_), How::Link) => format!("- [[{}]]\n{}", img.clone().unwrap_or_default(), if clip.link.is_empty() { String::new() } else { text }),
                (Some(e), _) => format!("{e}{}", if text.is_empty() { String::new() } else { format!("\n{text}") }),
                _ => text,
            }
        };
        let root = j::root();
        match target {
            Target::Note(rel) => {
                self.journal_append(&rel, &format!("\n{}", body(how).trim_end()));
                Some(Target::Note(rel))
            }
            Target::Today => {
                let rel = self.journal_today_rel();
                self.journal_append(&rel, &format!("\n{}", body(how).trim_end()));
                Some(Target::Note(rel))
            }
            Target::NewNote => {
                let rel = j::free_rel(&root, "", &clip.title);
                let text = match how {
                    How::Link => body(How::Card),
                    _ => body(how),
                };
                if let Err(e) = j::write(&root, &rel, &text) {
                    self.setup.journal.error = Some(format!("Couldn't make the note: {e}"));
                    return None;
                }
                self.journal_scan(true);
                Some(Target::Note(rel))
            }
            Target::Canvas(rel) => {
                let node = self.clip_node(&clip, img.as_deref(), how);
                self.canvas_add_nodes(&rel, vec![node]);
                Some(Target::Canvas(rel))
            }
            Target::NewCanvas => {
                let rel = j::free_rel_ext(&root, "", &clip.title, "canvas");
                let mut c = Canvas::default();
                let mut n = self.clip_node(&clip, img.as_deref(), how);
                n.x = 0.0;
                n.y = 0.0;
                c.nodes.push(n);
                if let Err(e) = j::write(&root, &rel, &c.to_json()) {
                    self.setup.journal.error = Some(format!("Couldn't make the canvas: {e}"));
                    return None;
                }
                self.journal_scan(true);
                Some(Target::Canvas(rel))
            }
        }
    }

    /// The card a clip makes on a canvas (placed later).
    fn clip_node(&self, clip: &Clip, img: Option<&str>, how: How) -> jcanvas::Node {
        if let Some(kind) = clip.widget {
            let d = super::widgets::def(kind);
            let (cw, ch) = d.map_or((4, 2), super::widgets::start_size);
            let (w, h) = super::journal_canvas::widget_size(cw, ch);
            return jcanvas::widget_node(0.0, 0.0, w, h, kind, &clip.title);
        }
        if let Some(rel) = img {
            if how != How::Full || clip.full.is_empty() {
                let (w, h) = super::journal_canvas::image_size(rel);
                return jcanvas::file_node(0.0, 0.0, w, h, rel);
            }
        }
        let text = match how {
            How::Card => clip.card.clone(),
            How::Link => clip.link.clone(),
            How::Full => match img {
                Some(r) => format!("![[{r}]]\n\n{}", clip.full),
                None => clip.full.clone(),
            },
        };
        let lines = text.lines().count() as f32;
        let (w, h) = match how {
            How::Link => (360.0, 60.0),
            How::Card => (360.0, (lines * 30.0 + 60.0).clamp(120.0, 360.0)),
            How::Full => (440.0, (lines * 24.0 + 60.0).clamp(160.0, 720.0)),
        };
        jcanvas::text_node(0.0, 0.0, w, jcanvas::snap(h), &text)
    }

    // ───────────────────────────── the popup and toast ─────────────────────────────

    /// The popup and the "Added" toast (every frame, on any page).
    pub(super) fn journal_overlays(&mut self, ctx: &egui::Context) {
        self.journal_add_popup(ctx);
        let Some((at, msg, target)) = self.setup.jadd.toast.clone() else { return };
        if at.elapsed() > Duration::from_secs(5) {
            self.setup.jadd.toast = None;
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(250));
        let screen = ctx.screen_rect();
        let mut open = false;
        let mut close = false;
        egui::Area::new(egui::Id::new("jadd-toast")).order(egui::Order::Foreground).anchor(Align2::CENTER_BOTTOM, vec2(0.0, -28.0)).show(ctx, |ui| {
            let _ = screen;
            egui::Frame::none().fill(CARD()).stroke(Stroke::new(1.0f32, JOURNAL_COL.gamma_multiply(0.6))).rounding(22.0).inner_margin(egui::Margin::symmetric(16.0, 9.0)).shadow(kit::shadow_tier(3)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📓").color(JOURNAL_COL));
                    ui.label(RichText::new(&msg).size(kit::fs(13.0)).color(TEXT()));
                    if target.is_some() && ui.link(RichText::new("Open ›").size(kit::fs(13.0)).strong()).clicked() {
                        open = true;
                    }
                    if ui.add(egui::Button::new(RichText::new("×").size(14.0).color(MUTED())).frame(false)).clicked() {
                        close = true;
                    }
                });
            });
        });
        if open {
            self.setup.jadd.toast = None;
            match target {
                Some(Target::Note(r)) => self.journal_go(Some(&r), None),
                Some(Target::Canvas(r)) => {
                    self.journal_go(None, None);
                    self.journal_open_canvas(&r);
                }
                _ => {}
            }
        } else if close {
            self.setup.jadd.toast = None;
        }
    }

    fn journal_add_popup(&mut self, ctx: &egui::Context) {
        let Some(clip) = self.setup.jadd.clip.clone() else { return };
        self.journal_scan(false);
        let widget = clip.widget.is_some();
        // where it could go: today, the latest notes and canvases, or new
        let root = j::root();
        let mut recent: Vec<(Target, String, &'static str, u64)> = Vec::new();
        for (rel, info) in &self.setup.journal.index {
            recent.push((Target::Note(rel.clone()), j::title_of(rel), "note", info.modified));
        }
        for rel in &self.setup.journal.canvases {
            let t = std::fs::metadata(root.join(rel)).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
            recent.push((Target::Canvas(rel.clone()), j::title_of(rel), "canvas", t));
        }
        if widget {
            recent.retain(|r| matches!(r.0, Target::Canvas(_)));
        }
        recent.sort_by(|a, b| b.3.cmp(&a.3).then(a.1.cmp(&b.1)));
        let q = self.setup.jadd.query.trim().to_lowercase();
        let list: Vec<(Target, String, String)> = if q.is_empty() {
            recent.iter().take(4).map(|(t, n, k, m)| (t.clone(), n.clone(), format!("{k} · {}", super::home::ago(*m)))).collect()
        } else {
            recent.iter().filter(|r| r.1.to_lowercase().contains(&q)).take(6).map(|(t, n, k, _)| (t.clone(), n.clone(), k.to_string())).collect()
        };
        let mut target = self.setup.jadd.target.clone();
        if target.is_none() {
            target = Some(list.first().map(|l| l.0.clone()).unwrap_or(if widget { Target::NewCanvas } else { Target::Today }));
        }
        let mut how = self.setup.jadd.how;
        let mut query = self.setup.jadd.query.clone();
        let mut cancel = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut add = false;
        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("jadd-popup")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(120));
            if scrim.clicked() {
                cancel = true;
            }
            let w = 460.0f32.min(sr.width() - 40.0);
            // as tall as what's in it
            let rows = usize::from(!widget && q.is_empty()) + list.len().max(usize::from(!q.is_empty())) + usize::from(!widget) + 1;
            let how_h = if widget { 22.0 } else { 18.0 + 4.0 + 34.0 + 4.0 + 18.0 };
            let h = 18.0 + 46.0 + 12.0 + 18.0 + rows as f32 * 36.0 + 6.0 + 38.0 + 14.0 + how_h + 20.0 + 36.0 + 18.0;
            let panel = Rect::from_center_size(sr.center(), vec2(w, h));
            ui.painter().add(kit::shadow_tier(4).as_shape(panel, 16.0));
            ui.painter().rect_filled(panel, 16.0, CARD());
            ui.painter().rect_stroke(panel, 16.0, Stroke::new(1.0f32, LINE()));
            ui.interact(panel, ui.id().with("jadd-panel"), Sense::click());
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(panel.shrink2(vec2(20.0, 18.0))).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let iw = ui.available_width();
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                    kit::icon_chip(ui.painter(), r, "📓", JOURNAL_COL, false);
                    ui.vertical(|ui| {
                        ui.add(egui::Label::new(RichText::new(format!("Add {} to your Journal", clip.title)).font(crate::fonts::sans_bold_font(15.5)).color(TEXT())).truncate());
                        ui.label(RichText::new(&clip.what).size(kit::fs(12.0)).color(MUTED()));
                    });
                });
                ui.add_space(12.0);
                ui.label(RichText::new("WHERE").size(kit::fs(10.5)).strong().color(MUTED()));
                ui.add_space(4.0);
                let row = |ui: &mut egui::Ui, on: bool, dot: Color32, name: &str, sub: &str, id: &str| -> bool {
                    let (r, resp) = ui.allocate_exact_size(vec2(iw, 32.0), Sense::click());
                    let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
                    let p = ui.painter();
                    if on {
                        p.rect_filled(r, 9.0, theme::blend(CARD(), JOURNAL_COL, 0.16));
                        p.rect_stroke(r, 9.0, Stroke::new(1.0f32, JOURNAL_COL.gamma_multiply(0.7)));
                    } else if ht > 0.0 {
                        p.rect_filled(r, 9.0, CARD_HOVER().gamma_multiply(ht));
                    }
                    if dot != Color32::TRANSPARENT {
                        p.rect_filled(Rect::from_center_size(pos2(r.min.x + 14.0, r.center().y), vec2(7.0, 7.0)), 2.0, dot);
                    }
                    let f = crate::fonts::sans_font(13.0);
                    let n = super::charts::clip(ui, name, &f, iw - 150.0);
                    p.text(pos2(r.min.x + 28.0, r.center().y), Align2::LEFT_CENTER, n, f, TEXT());
                    p.text(pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, sub, crate::fonts::sans_font(11.5), MUTED());
                    let _ = id;
                    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                };
                if !widget && q.is_empty() && row(ui, target == Some(Target::Today), kit::C_GOLD, "Today's note", "daily", "today") {
                    target = Some(Target::Today);
                }
                for (t, name, sub) in &list {
                    let dot = if matches!(t, Target::Canvas(_)) { kit::C_ORANGE } else { kit::C_VIOLET };
                    if row(ui, target.as_ref() == Some(t), dot, name, sub, name) {
                        target = Some(t.clone());
                    }
                }
                if list.is_empty() && !q.is_empty() {
                    ui.label(RichText::new("Nothing with that name.").size(kit::fs(12.0)).color(MUTED()));
                }
                if !widget && row(ui, target == Some(Target::NewNote), Color32::TRANSPARENT, &format!("+  New note “{}”", clip.title), "", "new-note") {
                    target = Some(Target::NewNote);
                }
                if row(ui, target == Some(Target::NewCanvas), Color32::TRANSPARENT, &format!("+  New canvas “{}”", clip.title), "", "new-canvas") {
                    target = Some(Target::NewCanvas);
                }
                ui.add_space(6.0);
                kit::search_box_keyed(ui, Some(egui::Id::new("jadd-q")), &mut query, if widget { "Find a canvas" } else { "Find a note or canvas" }, iw);
                ui.add_space(12.0);
                if !widget {
                    ui.label(RichText::new("ADD AS").size(kit::fs(10.5)).strong().color(MUTED()));
                    ui.add_space(4.0);
                    kit::segmented(ui, &mut how, &[(How::Card, "Card"), (How::Link, "Link only"), (How::Full, "Full page")]);
                    ui.add_space(4.0);
                    let what = match how {
                        How::Card => "A short summary box",
                        How::Link => "One line that opens it in the app",
                        How::Full => "Everything, as a page",
                    };
                    ui.label(RichText::new(what).size(kit::fs(11.5)).color(MUTED()));
                } else {
                    ui.label(RichText::new("Widgets go on canvases, where they stay live.").size(kit::fs(12.0)).color(MUTED()));
                }
                // buttons along the bottom
                let br = Rect::from_min_max(pos2(panel.max.x - 20.0 - 200.0, panel.max.y - 18.0 - 36.0), pos2(panel.max.x - 20.0, panel.max.y - 18.0));
                let cancel_r = Rect::from_min_size(br.min, vec2(96.0, 36.0));
                let add_r = Rect::from_min_size(pos2(br.min.x + 104.0, br.min.y), vec2(96.0, 36.0));
                if pill_button(ui, cancel_r, "Cancel", false, "jadd-cancel").clicked() {
                    cancel = true;
                }
                if pill_button(ui, add_r, "Add", true, "jadd-add").clicked() {
                    add = true;
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !ctx.wants_keyboard_input() {
            add = true;
        }
        let st = &mut self.setup.jadd;
        st.how = how;
        st.query = query;
        st.target = target.clone();
        if cancel {
            st.clip = None;
            return;
        }
        if add {
            st.clip = None;
            if let Some(t) = target {
                let how = if widget { How::Card } else { how };
                if let Some(done) = self.journal_put(clip, t, how) {
                    let name = match &done {
                        Target::Note(r) | Target::Canvas(r) => j::title_of(r),
                        _ => String::new(),
                    };
                    self.setup.jadd.toast = Some((Instant::now(), format!("Added to {name}"), Some(done)));
                }
            }
        }
    }
}

/// "dragon" -> "Dragon"
fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}

fn dash(s: &str) -> &str {
    if s.trim().is_empty() || s == "0" {
        "—"
    } else {
        s
    }
}

/// The "✎ Add to Journal" button (only when the Journal is installed).
pub(super) fn add_button(ui: &mut egui::Ui, have: bool) -> bool {
    if !have {
        return false;
    }
    ui.add(egui::Button::new(RichText::new("📓  Add to Journal").color(JOURNAL_COL)).fill(kit::btn_fill()).stroke(Stroke::new(1.0f32, JOURNAL_COL.gamma_multiply(0.5))))
        .on_hover_text("Put it in a note or on a canvas in your Journal")
        .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_and_callouts() {
        assert_eq!(enc("place:Route 210"), "place:Route%20210");
        assert_eq!(dec(&enc("a b)c%")), "a b)c%");
        let c = callout("pokedex", "Gible", &["Dragon".into(), String::new(), "x".into()]);
        assert_eq!(c, "> [!pokedex] Gible\n> Dragon\n> x\n");
        assert_eq!(title_case("DRAGON"), "Dragon");
        assert_eq!(dash("0"), "—");
    }
}
