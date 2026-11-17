//! Окно «О моде»: клик по строке контента сборки → описание с Modrinth, изменения версии,
//! сведения о файле и действия (вкл/выкл, обновить, удалить, открыть на сайте).

use crate::app::*;
use crate::backend::{ContentRow, ModDetails};
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align, Color32, FontId, Id, Layout, Margin, Rect, Sense, Stroke, TextFormat, Ui, Vec2, text::LayoutJob, vec2};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

pub struct ModInfoView {
    pub instance_id: String,
    pub file_path: String,
    pub tab: usize,
}

#[derive(Clone)]
pub enum Blk {
    Text(LayoutJob),
    Image(String),
    Code(String),
    Rule,
}

const KIND_NAMES: [&str; 4] = ["Мод", "Ресурспак", "Шейдер", "Датапак"];
const KIND_URL: [&str; 4] = ["mod", "resourcepack", "shader", "datapack"];

fn short_num(n: u64) -> String {
    match n {
        0..=999 => n.to_string(),
        1_000..=999_999 => format!("{:.1} тыс.", n as f64 / 1e3),
        _ => format!("{:.1} млн", n as f64 / 1e6),
    }
}

fn human_size(b: u64) -> String {
    match b {
        0..=1023 => format!("{b} Б"),
        1024..=1_048_575 => format!("{:.0} КБ", b as f64 / 1024.0),
        _ => format!("{:.1} МБ", b as f64 / 1_048_576.0),
    }
}

fn side_name(s: &str) -> &'static str {
    match s {
        "required" => "нужен",
        "optional" => "по желанию",
        "unsupported" => "не нужен",
        _ => "неизвестно",
    }
}

fn date_only(s: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(s).map(|d| d.format("%d.%m.%Y").to_string()).unwrap_or_else(|_| s.chars().take(10).collect())
}

/// Ссылки на картинки из HTML-вставок (`<img src="…">`), которыми любят оформлять описания.
fn html_images(html: &str, out: &mut Vec<Blk>) {
    let mut rest = html;
    while let Some(i) = rest.find("<img") {
        rest = &rest[i + 4..];
        let end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..end];
        if let Some(s) = tag.find("src=") {
            let t = &tag[s + 4..];
            let q = t.chars().next().unwrap_or('"');
            if q == '"' || q == '\'' {
                if let Some(e) = t[1..].find(q) {
                    push_image(&t[1..1 + e], out);
                }
            }
        }
        rest = &rest[end..];
    }
}

fn push_image(url: &str, out: &mut Vec<Blk>) {
    let low = url.to_ascii_lowercase();
    // значки shields.io и SVG egui не нарисует, да и смысла в них в лаунчере нет
    if !low.starts_with("http") || low.ends_with(".svg") || low.contains("shields.io") || low.contains("badge") {
        return;
    }
    out.push(Blk::Image(url.replacen("http://", "https://", 1)));
}

/// Простой Markdown → блоки egui: заголовки, абзацы, списки, код, картинки.
pub fn parse_md(src: &str) -> Vec<Blk> {
    let mut out = vec![];
    let mut job = LayoutJob::default();
    let (mut bold_n, mut italic_n, mut link_n, mut strike_n) = (0, 0, 0, 0);
    let mut heading: Option<HeadingLevel> = None;
    let mut list_depth: usize = 0;
    let mut code: Option<String> = None;

    let flush = |job: &mut LayoutJob, out: &mut Vec<Blk>| {
        if !job.text.trim().is_empty() {
            out.push(Blk::Text(std::mem::take(job)));
        } else {
            *job = LayoutJob::default();
        }
    };

    for ev in Parser::new_ext(src, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut job, &mut out);
                heading = Some(level);
            }
            Event::End(TagEnd::Heading(_)) => {
                flush(&mut job, &mut out);
                heading = None;
            }
            Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Item) | Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                flush(&mut job, &mut out)
            }
            Event::Start(Tag::List(_)) => {
                flush(&mut job, &mut out);
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Start(Tag::Item) => {
                flush(&mut job, &mut out);
                let pad = "    ".repeat(list_depth.saturating_sub(1));
                job.append(&format!("{pad}•  "), 0.0, TextFormat::simple(regular(14.0), accent()));
            }
            Event::Start(Tag::TableCell) => {
                if !job.text.is_empty() {
                    job.append("   ·   ", 0.0, TextFormat::simple(regular(14.0), TEXT_FAINT()));
                }
            }
            Event::Start(Tag::Strong) => bold_n += 1,
            Event::End(TagEnd::Strong) => bold_n -= 1,
            Event::Start(Tag::Emphasis) => italic_n += 1,
            Event::End(TagEnd::Emphasis) => italic_n -= 1,
            Event::Start(Tag::Strikethrough) => strike_n += 1,
            Event::End(TagEnd::Strikethrough) => strike_n -= 1,
            Event::Start(Tag::Link { .. }) => link_n += 1,
            Event::End(TagEnd::Link) => link_n -= 1,
            Event::Start(Tag::Image { dest_url, .. }) => {
                flush(&mut job, &mut out);
                push_image(&dest_url, &mut out);
                link_n += 1000; // подпись картинки не выводим
            }
            Event::End(TagEnd::Image) => link_n -= 1000,
            Event::Start(Tag::CodeBlock(_)) => {
                flush(&mut job, &mut out);
                code = Some(String::new());
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(c) = code.take() {
                    out.push(Blk::Code(c.trim_end().to_string()));
                }
            }
            Event::Rule => {
                flush(&mut job, &mut out);
                out.push(Blk::Rule);
            }
            Event::Html(h) | Event::InlineHtml(h) => {
                if h.contains("<img") {
                    flush(&mut job, &mut out);
                    html_images(&h, &mut out);
                } else if h.contains("<br") {
                    job.append("\n", 0.0, TextFormat::simple(regular(14.0), TEXT()));
                }
            }
            Event::SoftBreak => job.append(" ", 0.0, TextFormat::simple(regular(14.0), TEXT())),
            Event::HardBreak => job.append("\n", 0.0, TextFormat::simple(regular(14.0), TEXT())),
            Event::Code(t) => job.append(&t, 0.0, TextFormat { font_id: mono(13.0), color: TEXT(), background: RAISED(), ..Default::default() }),
            Event::Text(t) => {
                if let Some(c) = code.as_mut() {
                    c.push_str(&t);
                    continue;
                }
                if link_n >= 1000 {
                    continue;
                }
                let font: FontId = match heading {
                    Some(HeadingLevel::H1) => bold(21.0),
                    Some(HeadingLevel::H2) => bold(18.0),
                    Some(_) => semibold(16.0),
                    None if bold_n > 0 => semibold(14.0),
                    None => regular(14.0),
                };
                let color = if link_n > 0 { accent() } else if heading.is_some() { TEXT() } else { TEXT().gamma_multiply(0.92) };
                let mut f = TextFormat::simple(font, color);
                f.italics = italic_n > 0;
                if strike_n > 0 {
                    f.strikethrough = Stroke::new(1.0, color);
                }
                if link_n > 0 {
                    f.underline = Stroke::new(1.0, color.gamma_multiply(0.5));
                }
                job.append(&t, 0.0, f);
            }
            _ => {}
        }
    }
    flush(&mut job, &mut out);
    out
}

pub fn md_view(ui: &mut Ui, blocks: &[Blk]) {
    let w = ui.available_width();
    ui.spacing_mut().item_spacing.y = 8.0;
    for b in blocks {
        match b {
            Blk::Text(job) => {
                let mut job = job.clone();
                job.wrap.max_width = w;
                ui.label(job);
            }
            Blk::Image(url) => {
                ui.add(egui::Image::from_uri(url.clone()).max_width(w.min(640.0)).max_height(420.0).corner_radius(rr(8)));
            }
            Blk::Code(c) => {
                egui::Frame::new().fill(INPUT()).corner_radius(rr(8)).inner_margin(10).show(ui, |ui| {
                    ui.set_width(w - 20.0);
                    ui.label(egui::RichText::new(c).font(mono(12.5)).color(TEXT_DIM()));
                });
            }
            Blk::Rule => {
                let y = ui.cursor().top() + 4.0;
                ui.painter().hline(ui.max_rect().x_range(), y, Stroke::new(1.0, BORDER()));
                ui.add_space(8.0);
            }
        }
    }
}

impl App {
    pub fn open_mod_info(&mut self, instance_id: &str, row: &ContentRow) {
        self.mod_info = Some(ModInfoView { instance_id: instance_id.to_string(), file_path: row.file_path.clone(), tab: 0 });
        if let Some(pid) = &row.project_id
            && !self.mod_details.contains_key(pid)
        {
            self.be.load_mod_details(pid.clone(), row.version_id.clone());
        }
    }

    pub fn mod_info_dialog(&mut self, ctx: &egui::Context) {
        let Some(view) = &self.mod_info else { return };
        let iid = view.instance_id.clone();
        let Some(row) = self.content.get(&iid).and_then(|c| c.iter().find(|r| r.file_path == view.file_path)).cloned() else {
            self.mod_info = None;
            return;
        };
        let details: Option<Result<ModDetails, String>> = row.project_id.as_ref().and_then(|p| self.mod_details.get(p).cloned());
        let mut close = false;
        let kind = (row.kind as usize).min(3);
        let screen = ctx.content_rect();
        let frame = egui::Frame::new().fill(PANEL()).corner_radius(rr(14)).stroke(Stroke::new(1.0, BORDER())).inner_margin(Margin::same(22));

        let r = egui::Modal::new(Id::new("mod-info")).frame(frame).show(ctx, |ui| {
            ui.set_width(760.0f32.min(screen.width() - 80.0));
            ui.spacing_mut().item_spacing.y = 10.0;

            // ── шапка
            ui.horizontal(|ui| {
                let (ir, _) = ui.allocate_exact_size(Vec2::splat(72.0), Sense::hover());
                match &row.icon_url {
                    Some(u) => {
                        ui.painter().rect_filled(ir, rr(14), RAISED());
                        egui::Image::from_uri(u.clone()).corner_radius(rr(14)).paint_at(ui, ir);
                    }
                    None => {
                        let (seed, hue) = seed_of(&row.name);
                        pixel_icon(ui.painter(), ir, seed, hue, 14.0);
                    }
                }
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.set_max_width(ui.available_width() - 50.0);
                    ui.add(egui::Label::new(egui::RichText::new(&row.name).font(bold(22.0)).color(TEXT())).wrap());
                    let mut by = String::new();
                    if !row.author.is_empty() {
                        by = format!("от {}", row.author);
                    }
                    if let Some(Ok(d)) = &details
                        && d.team.len() > 1
                    {
                        by = format!("от {}", d.team.join(", "));
                    }
                    if !by.is_empty() {
                        label(ui, by, regular(13.0), TEXT_DIM());
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        chip(ui, KIND_NAMES[kind], RAISED(), TEXT());
                        if !row.version.is_empty() {
                            chip(ui, &row.version, RAISED(), TEXT_DIM());
                        }
                        if let Some(l) = &row.license {
                            chip(ui, l, RAISED(), TEXT_DIM());
                        }
                        if !row.enabled {
                            chip(ui, "выключен", DANGER().gamma_multiply(0.18), lighten(DANGER(), 0.3));
                        }
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if icon_button(ui, Icon::Close, 32.0, "Закрыть").clicked() {
                        close = true;
                    }
                });
            });

            if let Some(Ok(d)) = &details {
                if !d.description.is_empty() {
                    ui.add(egui::Label::new(egui::RichText::new(&d.description).font(regular(14.0)).color(TEXT_DIM())).wrap());
                }
                let stats = [
                    ("Скачиваний", short_num(d.downloads)),
                    ("Подписчиков", short_num(d.followers)),
                    ("Обновлён", date_only(&d.updated)),
                    ("Клиент / сервер", format!("{} / {}", side_name(&d.client_side), side_name(&d.server_side))),
                ];
                ui.columns(4, |c| {
                    for (i, (k, v)) in stats.iter().enumerate() {
                        egui::Frame::new().fill(RAISED()).corner_radius(rr(10)).inner_margin(Margin::symmetric(12, 8)).show(&mut c[i], |ui| {
                            ui.set_width(ui.available_width());
                            ui.spacing_mut().item_spacing.y = 1.0;
                            label(ui, v.clone(), semibold(15.0), TEXT());
                            label(ui, *k, regular(12.0), TEXT_DIM());
                        });
                    }
                });
            }

            // ── действия
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let mut en = row.enabled;
                let (tr, _) = ui.allocate_exact_size(vec2(38.0, 34.0), Sense::hover());
                if toggle_at(ui, Rect::from_center_size(tr.center(), vec2(38.0, 22.0)), Id::new("mi-toggle"), &mut en).changed() {
                    self.be.toggle_content(iid.clone(), row.file_path.clone(), en);
                    if let Some(rows) = self.content.get_mut(&iid)
                        && let Some(r) = rows.iter_mut().find(|r| r.file_path == row.file_path)
                    {
                        r.enabled = en;
                    }
                }
                label(ui, if row.enabled { "Включён" } else { "Выключен" }, medium(13.5), TEXT_DIM());
                ui.add_space(8.0);
                if row.has_update && button(ui, "Обновить", Some(Icon::Download), Kind::Primary, 34.0).clicked() {
                    self.be.update_content(iid.clone(), vec![row.file_path.clone()], self.running.contains_key(&iid));
                }
                if let Some(pid) = &row.project_id {
                    let slug = row.slug.clone().unwrap_or_else(|| pid.clone());
                    if button(ui, "Modrinth", Some(Icon::Globe), Kind::Secondary, 34.0).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(format!("https://modrinth.com/{}/{slug}", KIND_URL[kind])));
                    }
                }
                if button(ui, "Показать файл", Some(Icon::Folder), Kind::Secondary, 34.0).clicked() {
                    self.be.reveal_content(iid.clone(), row.file_path.clone());
                }
                let ask = Id::new(("mi-del", &row.file_path));
                let armed: bool = ui.ctx().data(|d| d.get_temp(ask)).unwrap_or(false);
                if button(ui, if armed { "Точно удалить?" } else { "Удалить" }, None, Kind::Danger, 34.0).clicked() {
                    if armed {
                        self.be.remove_content(iid.clone(), row.file_path.clone());
                        close = true;
                    }
                    ui.ctx().data_mut(|d| d.insert_temp(ask, !armed));
                }
            });
            if let Some(Ok(d)) = &details
                && !d.links.is_empty()
            {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (name, url) in &d.links {
                        let resp = chip(ui, &format!("↗ {name}"), accent().gamma_multiply(0.12), accent()).interact(Sense::click());
                        if resp.on_hover_text(url).clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        }
                    }
                });
            }

            // ── вкладки
            let mut labels = vec!["Описание".to_string(), "Изменения".into(), "Файл".into()];
            if row.project_id.is_none() {
                labels = vec!["Файл".into()];
            }
            let mut tab = self.mod_info.as_ref().map(|v| v.tab).unwrap_or(0).min(labels.len() - 1);
            tabs(ui, Id::new("mi-tabs"), &labels, &mut tab);
            if let Some(v) = self.mod_info.as_mut() {
                v.tab = tab;
            }
            let file_tab = row.project_id.is_none() || tab == 2;
            let used = ui.min_rect().height();
            let h = (screen.height() * 0.88 - used - 70.0).max(160.0);
            egui::ScrollArea::vertical().id_salt(("mi-scroll", &row.file_path, tab)).max_height(h).min_scrolled_height(h).auto_shrink([false, true]).show(ui, |ui| {
                ui.set_width(ui.available_width() - 8.0);
                if file_tab {
                    let mut kv = vec![("Файл", row.file_name.clone()), ("Размер", human_size(row.size)), ("Путь в сборке", row.file_path.clone())];
                    if let Some(p) = &row.published {
                        kv.push(("Версия вышла", date_only(p)));
                    }
                    if let Some(Ok(d)) = &details {
                        if !d.loaders.is_empty() {
                            kv.push(("Загрузчики", d.loaders.join(", ")));
                        }
                        if let Some(last) = d.game_versions.last() {
                            kv.push(("Версии игры", format!("{} … {last}", d.game_versions.first().cloned().unwrap_or_default())));
                        }
                    }
                    if !row.categories.is_empty() {
                        kv.push(("Категории", row.categories.join(", ")));
                    }
                    for (k, v) in kv {
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(vec2(150.0, 18.0), Layout::left_to_right(Align::Center), |ui| label(ui, k, regular(13.0), TEXT_DIM()));
                            ui.add(egui::Label::new(egui::RichText::new(v).font(mono(12.5)).color(TEXT())).wrap());
                        });
                    }
                    if row.project_id.is_none() {
                        ui.add_space(6.0);
                        label(ui, "Этого файла нет на Modrinth — описание и обновления недоступны.", regular(13.0), TEXT_FAINT());
                    }
                    return;
                }
                match &details {
                    None => {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| label(ui, "Загружаю описание…", medium(14.0), TEXT_DIM()));
                    }
                    Some(Err(e)) => {
                        label(ui, format!("Не удалось загрузить: {e}"), regular(13.0), lighten(DANGER(), 0.3));
                    }
                    Some(Ok(d)) => {
                        let key = format!("{}#{tab}", row.project_id.clone().unwrap_or_default());
                        if !self.md_cache.contains_key(&key) {
                            let src = if tab == 0 { &d.body } else { &d.changelog };
                            let blocks = if src.trim().is_empty() {
                                vec![Blk::Text(LayoutJob::simple_singleline(
                                    if tab == 0 { "Описания нет".into() } else { "Автор не написал, что изменилось в этой версии".into() },
                                    regular(14.0),
                                    TEXT_FAINT(),
                                ))]
                            } else {
                                parse_md(src)
                            };
                            self.md_cache.insert(key.clone(), blocks);
                        }
                        let blocks = self.md_cache.get(&key).cloned().unwrap_or_default();
                        md_view(ui, &blocks);
                    }
                }
            });
        });
        if close || r.should_close() {
            self.mod_info = None;
        }
    }
}

/// Русские названия категорий Modrinth (остальные — как есть, с заглавной).
pub fn category_ru(id: &str) -> String {
    let ru = match id {
        "adventure" => "Приключения", "cursed" => "Проклятое", "decoration" => "Декор", "economy" => "Экономика",
        "equipment" => "Снаряжение", "food" => "Еда", "game-mechanics" => "Механики игры", "library" => "Библиотеки",
        "magic" => "Магия", "management" => "Управление", "minigame" => "Мини-игры", "mobs" => "Мобы",
        "optimization" => "Оптимизация", "social" => "Общение", "storage" => "Хранение", "technology" => "Технологии",
        "transportation" => "Транспорт", "utility" => "Утилиты", "worldgen" => "Генерация мира", "challenging" => "Сложное",
        "combat" => "Бой", "kitchen-sink" => "Всё подряд", "lightweight" => "Лёгкое", "multiplayer" => "Мультиплеер",
        "quests" => "Квесты", "atmosphere" => "Атмосфера", "bloom" => "Свечение", "cartoon" => "Мультяшное",
        "colored-lighting" => "Цветной свет", "fantasy" => "Фэнтези", "foliage" => "Листва", "high" => "Высокая",
        "low" => "Низкая", "medium" => "Средняя", "path-tracing" => "Трассировка путей", "pbr" => "PBR",
        "potato" => "Картошка", "realistic" => "Реализм", "reflections" => "Отражения", "screenshot" => "Для скриншотов",
        "semi-realistic" => "Полуреализм", "shadows" => "Тени", "vanilla-like" => "Как ванилла", "audio" => "Звук",
        "blocks" => "Блоки", "core-shaders" => "Core-шейдеры", "entities" => "Сущности", "environment" => "Окружение",
        "items" => "Предметы", "fonts" => "Шрифты", "gui" => "Интерфейс", "locale" => "Перевод",
        "models" => "Модели", "simplistic" => "Простое", "themed" => "Тематическое", "tweaks" => "Твики",
        "8x-" => "8x и меньше", "16x" => "16x", "32x" => "32x", "48x" => "48x", "64x" => "64x", "128x" => "128x",
        "256x" => "256x", "512x+" => "512x и больше", "modded" => "Для модов", "iris" => "Iris", "optifine" => "OptiFine",
        "canvas" => "Canvas", "vanilla" => "Ванилла",
        _ => "",
    };
    if !ru.is_empty() {
        return ru.to_string();
    }
    let mut c = id.replace('-', " ");
    if let Some(f) = c.get(..1) {
        c = f.to_uppercase() + &c[1..];
    }
    c
}

pub fn category_group_ru(header: &str) -> String {
    match header {
        "categories" => "Категории",
        "features" => "Особенности",
        "resolutions" => "Разрешение",
        "performance impact" => "Нагрузка",
        other => other,
    }
    .to_string()
}

/// Очень простой HTML → Markdown для описаний CurseForge: абзацы, списки, заголовки, картинки.
pub fn html_to_md(html: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let b = html.as_bytes();
    while i < b.len() {
        if b[i] == b'<' {
            let end = html[i..].find('>').map(|e| i + e + 1).unwrap_or(b.len());
            let tag = html[i..end].to_ascii_lowercase();
            let name: String = tag.trim_start_matches(['<', '/']).chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            let closing = tag.starts_with("</");
            match name.as_str() {
                "br" => out.push('\n'),
                "p" | "div" => out.push_str("\n\n"),
                "li" if !closing => out.push_str("\n- "),
                "h1" | "h2" | "h3" | "h4" if !closing => out.push_str(&format!("\n\n{} ", "#".repeat(name[1..].parse::<usize>().unwrap_or(2).min(3)))),
                "h1" | "h2" | "h3" | "h4" => out.push_str("\n\n"),
                "strong" | "b" => out.push_str("**"),
                "img" => {
                    let raw = &html[i..end];
                    if let Some(sp) = raw.find("src=\"") {
                        let rest = &raw[sp + 5..];
                        if let Some(e) = rest.find('"') {
                            out.push_str(&format!("\n\n![]({})\n\n", &rest[..e]));
                        }
                    }
                }
                _ => {}
            }
            i = end;
        } else {
            let next = html[i..].find('<').map(|n| i + n).unwrap_or(b.len());
            out.push_str(&html[i..next].replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'"));
            i = next;
        }
    }
    let mut cleaned = String::new();
    let mut blank = 0;
    for line in out.lines() {
        if line.trim().is_empty() {
            blank += 1;
            if blank <= 1 {
                cleaned.push('\n');
            }
        } else {
            blank = 0;
            cleaned.push_str(line.trim_end());
            cleaned.push('\n');
        }
    }
    cleaned
}

/// Окно проекта из обзора (Modrinth или CurseForge).
pub struct ProjectView {
    pub hit: crate::backend::Hit,
    pub tab: usize,
}

impl App {
    pub fn open_project(&mut self, hit: &crate::backend::Hit) {
        self.project_view = Some(ProjectView { hit: hit.clone(), tab: 0 });
        if !self.mod_details.contains_key(&hit.project_id) {
            if hit.source == "cf" {
                self.be.load_cf_details(hit.clone(), self.prefs.cf_key.clone());
            } else {
                self.be.load_mod_details(hit.project_id.clone(), None);
            }
        }
    }

    pub fn project_dialog(&mut self, ctx: &egui::Context) {
        let Some(view) = &self.project_view else { return };
        let h = view.hit.clone();
        let details = self.mod_details.get(&h.project_id).cloned();
        let screen = ctx.content_rect();
        let mut close = false;
        let frame = egui::Frame::new().fill(PANEL()).corner_radius(rr(14)).stroke(Stroke::new(1.0, BORDER())).inner_margin(Margin::same(22));
        let site = if h.source == "cf" { "CurseForge" } else { "Modrinth" };
        let r = egui::Modal::new(Id::new("project")).frame(frame).show(ctx, |ui| {
            ui.set_width(820.0f32.min(screen.width() - 80.0));
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.horizontal(|ui| {
                let (ir, _) = ui.allocate_exact_size(Vec2::splat(80.0), Sense::hover());
                ui.painter().rect_filled(ir, rr(16), RAISED());
                if let Some(u) = h.icon_url.as_ref().filter(|u| !u.is_empty()) {
                    egui::Image::from_uri(u.clone()).corner_radius(rr(16)).paint_at(ui, ir);
                }
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.set_max_width(ui.available_width() - 50.0);
                    ui.add(egui::Label::new(egui::RichText::new(&h.title).font(bold(23.0)).color(TEXT())).wrap());
                    let team = details.as_ref().and_then(|d| d.as_ref().ok()).filter(|d| d.team.len() > 1).map(|d| d.team.join(", ")).unwrap_or_else(|| h.author.clone());
                    label(ui, format!("от {team} · {site}"), regular(13.0), TEXT_DIM());
                    ui.add(egui::Label::new(egui::RichText::new(&h.description).font(regular(14.0)).color(TEXT_DIM())).wrap());
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if icon_button(ui, Icon::Close, 32.0, "Закрыть").clicked() {
                        close = true;
                    }
                });
            });
            let updated = h.date_modified.get(..10).unwrap_or("").split('-').rev().collect::<Vec<_>>().join(".");
            let side = details.as_ref().and_then(|d| d.as_ref().ok()).filter(|d| !d.client_side.is_empty()).map(|d| format!("{} / {}", side_name(&d.client_side), side_name(&d.server_side)));
            let mut stats = vec![("Скачиваний", short_num(h.downloads)), (if h.source == "cf" { "Лайков" } else { "Подписчиков" }, short_num(h.follows)), ("Обновлён", updated)];
            if let Some(sd) = side {
                stats.push(("Клиент / сервер", sd));
            }
            let n = stats.len();
            ui.columns(n, |c| {
                for (i, (k, v)) in stats.iter().enumerate() {
                    egui::Frame::new().fill(RAISED()).corner_radius(rr(10)).inner_margin(Margin::symmetric(12, 8)).show(&mut c[i], |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 1.0;
                        label(ui, v.clone(), semibold(15.0), TEXT());
                        label(ui, *k, regular(12.0), TEXT_DIM());
                    });
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let installing = self.installing.contains(&h.project_id);
                let installed = self.installed.contains(&h.project_id);
                let is_pack = h.project_type == "modpack";
                if installing {
                    button(ui, "Установка…", Some(Icon::Download), Kind::Secondary, 36.0);
                } else if installed && !is_pack {
                    button(ui, "Установлен", Some(Icon::Check), Kind::Secondary, 36.0);
                } else if button(ui, "Установить", Some(Icon::Download), Kind::Primary, 36.0).clicked() {
                    self.install_hit(&h, Self::now(ui));
                }
                let url = if h.source == "cf" { h.page_url.clone() } else { format!("https://modrinth.com/{}/{}", if h.project_type == "shader" { "shader" } else { h.project_type.as_str() }, h.slug.clone().unwrap_or(h.project_id.clone())) };
                if button(ui, site, Some(Icon::Globe), Kind::Secondary, 36.0).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                }
                if let Some(Ok(d)) = &details {
                    for (name, url) in &d.links {
                        if chip(ui, &format!("↗ {name}"), accent().gamma_multiply(0.12), accent()).interact(Sense::click()).on_hover_text(url).clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        }
                    }
                }
            });
            let mut labels = vec!["Описание".to_string()];
            let has_gallery = details.as_ref().and_then(|d| d.as_ref().ok()).is_some_and(|d| !d.gallery.is_empty());
            if has_gallery {
                labels.push("Галерея".into());
            }
            if h.source != "cf" {
                labels.push("Версии".into());
            }
            let mut tab = self.project_view.as_ref().map(|v| v.tab).unwrap_or(0).min(labels.len() - 1);
            if tabs(ui, Id::new("proj-tabs"), &labels, &mut tab) && labels[tab] == "Версии" && !self.proj_versions.contains_key(&h.project_id) {
                self.be.load_versions_of(h.project_id.clone());
            }
            if let Some(v) = self.project_view.as_mut() {
                v.tab = tab;
            }
            let used = ui.min_rect().height();
            let hgt = (screen.height() * 0.88 - used - 60.0).max(160.0);
            egui::ScrollArea::vertical().id_salt(("proj", &h.project_id, tab)).max_height(hgt).min_scrolled_height(hgt).auto_shrink([false, true]).show(ui, |ui| {
                ui.set_width(ui.available_width() - 8.0);
                match (labels[tab].as_str(), &details) {
                    (_, None) => {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| label(ui, "Загружаю…", medium(14.0), TEXT_DIM()));
                    }
                    (_, Some(Err(e))) => {
                        label(ui, format!("Не удалось загрузить: {e}"), regular(13.0), lighten(DANGER(), 0.3));
                    }
                    ("Галерея", Some(Ok(d))) => {
                        let w = ui.available_width();
                        for url in &d.gallery {
                            ui.add(egui::Image::from_uri(url.clone()).max_width(w).corner_radius(rr(10)));
                        }
                    }
                    ("Версии", _) => match self.proj_versions.get(&h.project_id) {
                        None => {
                            label(ui, "Загружаю версии…", regular(13.0), TEXT_DIM());
                        }
                        Some(list) => {
                            for v in list {
                                egui::Frame::new().fill(RAISED()).corner_radius(rr(10)).inner_margin(Margin::symmetric(12, 8)).show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let (kc, kt) = match v.kind.as_str() {
                                            "beta" => (Color32::from_rgb(230, 170, 60), "бета"),
                                            "alpha" => (DANGER(), "альфа"),
                                            _ => (GREEN(), "релиз"),
                                        };
                                        chip(ui, kt, kc.gamma_multiply(0.18), kc);
                                        ui.add(egui::Label::new(egui::RichText::new(&v.name).font(semibold(13.5)).color(TEXT())).truncate());
                                    });
                                    let gv = match (v.game_versions.first(), v.game_versions.last()) {
                                        (Some(a), Some(b)) if a != b => format!("{a} … {b}"),
                                        (Some(a), _) => a.clone(),
                                        _ => String::new(),
                                    };
                                    label(ui, format!("{} · {} · {} · {} скач. · {}", v.number, v.loaders.join(", "), gv, short_num(v.downloads), v.date.get(..10).unwrap_or("")), regular(12.5), TEXT_DIM());
                                });
                            }
                        }
                    },
                    (_, Some(Ok(d))) => {
                        let key = format!("{}#p", h.project_id);
                        if !self.md_cache.contains_key(&key) {
                            let blocks = if d.body.trim().is_empty() { vec![Blk::Text(LayoutJob::simple_singleline("Описания нет".into(), regular(14.0), TEXT_FAINT()))] } else { parse_md(&d.body) };
                            self.md_cache.insert(key.clone(), blocks);
                        }
                        let blocks = self.md_cache.get(&key).cloned().unwrap_or_default();
                        md_view(ui, &blocks);
                    }
                }
            });
        });
        if close || r.should_close() {
            self.project_view = None;
        }
    }
}
