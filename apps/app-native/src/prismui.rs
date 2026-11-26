//! Макет «Prism Launcher»: окно один в один как Prism Launcher.
//! Тулбар с подписями сверху, сетка сборок по группам, панель действий выбранной сборки справа,
//! строка новостей и строка статуса снизу, котик за сеткой. Остальные страницы (параметры, обзор,
//! скины, сборка) открываются окном поверх сетки — как отдельные окна Prism.
//! Иконки — свой пиксель-арт в духе цветного набора Prism (рамка-«земля» и зелёная «трава»).

use crate::app::*;
use crate::backend::Inst;
use crate::instcfg::ExportFormat;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{
    self, Align, Align2, Color32, Frame, Id, Layout, Margin, Mesh, Painter, Rect, Sense, Shape, Stroke, Ui, Vec2, ViewportCommand,
    pos2, text::LayoutJob, vec2,
};

const REPO_URL: &str = "https://github.com/Flenstick/TrollLauncher";

// ───────────────────────── иконки ─────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Pe {
    Home,
    Add,
    Folder,
    Settings,
    Browse,
    Skins,
    Launch,
    Stop,
    Edit,
    Group,
    Export,
    Copy,
    Delete,
    News,
    Cat,
}

/// Буквы рисунка → цвет. `.` — пусто.
fn pe_color(ch: u8) -> Option<Color32> {
    let c = |v: u32| Some(hex(v));
    match ch {
        b'G' => c(0x3cb44b), // трава
        b'g' => c(0x2c8f3c), // тень травы
        b'B' => c(0x8b5d33), // земля
        b'b' => c(0x6b4526),
        b'W' => c(0xf1f1f1),
        b'w' => c(0xc9c9c9),
        b'D' => c(0x0b8f41), // тёмно-зелёный знак
        b'K' => c(0x1e1e1e),
        b'k' => c(0x6f6f6f),
        b'X' => c(0x666c73), // серая кнопка «Остановить»
        b'x' => c(0xc8ccd0),
        b'S' => c(0xc89a6c), // кожа
        b'H' => c(0x4a3020), // волосы
        b'E' => c(0x4a5fc1), // глаза
        _ => None,
    }
}

fn pe_art(ic: Pe) -> [&'static str; 16] {
    match ic {
        Pe::Add => [
            "................",
            "..GGGGGGGGGGGG..",
            ".GKGGGGGGGGGGGG.",
            ".GGGGGGGGGGGGGG.",
            ".BWWWWWWWWWWWWB.",
            ".BWWWWWDDWWWWWB.",
            ".BWWWWWDDWWWWWB.",
            ".BWWDDDDDDDDWWB.",
            ".BWWDDDDDDDDWWB.",
            ".BWWWWWDDWWWWWB.",
            ".BWWWWWDDWWWWWB.",
            ".BWWWWWWWWWWWWB.",
            ".BwwwwwwwwwwwwB.",
            ".BBBBBBBBBBBBBB.",
            "..bbbbbbbbbbbb..",
            "................",
        ],
        Pe::Folder => [
            "................",
            "..GGGGG.........",
            ".GGGGGGG........",
            ".GWWWWWGGGGGGGG.",
            ".BWWWWWWWWWWWWB.",
            ".BWWWWWWWWWWWWB.",
            ".BWWWWWWWWWWWWB.",
            ".BWWWWWWWWWWWWB.",
            ".BWWWWWWWWWWWWB.",
            ".BWWWWWWWWWWWWB.",
            ".BWWWWWWWWWWWWB.",
            ".BWWWWWWWWWWWWB.",
            ".BwwwwwwwwwwwwB.",
            ".BBBBBBBBBBBBBB.",
            "..bbbbbbbbbbbb..",
            "................",
        ],
        Pe::Settings | Pe::Edit => [
            "................",
            "..GGGGGGGGGGGG..",
            ".GGGGGGGGGGGGGG.",
            ".GGGGGGGGGGGGGG.",
            ".BWWWWWWWWWWWWB.",
            ".BWKKKWWWWWWWWB.",
            ".BWKWKWkkkkkWWB.",
            ".BWKKKWWWWWWWWB.",
            ".BWWWWWWWWWWWWB.",
            ".BWKKKWWWWWWWWB.",
            ".BWKWKWkkkkkWWB.",
            ".BWKKKWWWWWWWWB.",
            ".BwwwwwwwwwwwwB.",
            ".BBBBBBBBBBBBBB.",
            "..bbbbbbbbbbbb..",
            "................",
        ],
        Pe::Browse => [
            "................",
            ".....GGGGGG.....",
            "...GGGGGGGGGG...",
            "..GGWWDDWWWWGG..",
            ".BBWDDDDWWDDWBB.",
            ".BWWDDDWWWDDDWB.",
            "BBWWWDDWWWWDWWBB",
            "BWWWWWWWWWWWWWWB",
            "BWDDWWWWWDDDWWWB",
            "BBDDDDWWWDDDDWBB",
            ".BWDDDWWWWDDWWB.",
            ".BWWDWWWWWWWWWB.",
            "..BBWWWWWWWWBB..",
            "...BBBBBBBBBB...",
            ".....bbbbbb.....",
            "................",
        ],
        Pe::Skins => [
            "................",
            "...GGGG..GGGG...",
            "..GGGGGWWGGGGG..",
            ".GGGGGGWWGGGGGG.",
            "GGGGGWWWWWWGGGGG",
            "BWWWBWWWWWWBWWWB",
            "BWWWBWWWWWWBWWWB",
            ".BBBBWWWWWWBBBB.",
            "....BWWWWWWB....",
            "....BWWWWWWB....",
            "....BWWWWWWB....",
            "....BWWWWWWB....",
            "....BwwwwwwB....",
            "....BBBBBBBB....",
            ".....bbbbbb.....",
            "................",
        ],
        Pe::Home => [
            "................",
            ".......GG.......",
            "......GGGG......",
            ".....GGGGGG.....",
            "....GGGGGGGG....",
            "...GGGGGGGGGG...",
            "..GGGGGGGGGGGG..",
            ".GGGGGGGGGGGGGG.",
            "..BWWWWWWWWWWB..",
            "..BWWWWWWWWWWB..",
            "..BWWWBBBWWWWB..",
            "..BWWWBKBWWWWB..",
            "..BWWWBKBWWWWB..",
            "..BBBBBBBBBBBB..",
            "...bbbbbbbbbb...",
            "................",
        ],
        Pe::Launch => [
            "................",
            "..GG............",
            "..GGGG..........",
            "..BWGGGG........",
            "..BWWWWGGG......",
            "..BWWWWWWBBB....",
            "..BWWWWWWWWBBB..",
            "..BWWWWWWWWWWBB.",
            "..BWWWWWWWWWWBB.",
            "..BWWWWWWWWBBB..",
            "..BWWWWWWBBB....",
            "..BWWWWBBB......",
            "..BWWBBB........",
            "..BBBB..........",
            "..bb............",
            "................",
        ],
        Pe::Stop => [
            "................",
            "..XXXXXXXXXXXX..",
            ".XXXXXXXXXXXXXX.",
            ".XXXXXXXXXXXXXX.",
            ".XXXXXXXXXXXXXX.",
            ".XXXXxXXXXxXXXX.",
            ".XXXXXxxxxXXXXX.",
            ".XXXXXxxxxXXXXX.",
            ".XXXXXxxxxXXXXX.",
            ".XXXXXxxxxXXXXX.",
            ".XXXXxXXXXxXXXX.",
            ".XXXXXXXXXXXXXX.",
            ".XXXXXXXXXXXXXX.",
            ".XXXXXXXXXXXXXX.",
            "..XXXXXXXXXXXX..",
            "................",
        ],
        Pe::Group => [
            "................",
            "..GGGGGGGGGGGG..",
            ".GGGGGGGGGGGGGG.",
            ".GGGGGGGGGGGGGG.",
            ".BWWWWWWWWWWWWB.",
            ".BWDDDDDWWWWWWB.",
            ".BWDWWDDDWWWWWB.",
            ".BWDDDDDDDWWWWB.",
            ".BWDDDDDDDDWWWB.",
            ".BWWDDDDDDDDWWB.",
            ".BWWWWDDDDDDWWB.",
            ".BWWWWWWDDDDWWB.",
            ".BwwwwwwwwDDwwB.",
            ".BBBBBBBBBBBBBB.",
            "..bbbbbbbbbbbb..",
            "................",
        ],
        Pe::Export => [
            "................",
            "..GGGGG.........",
            ".GGGGGGG........",
            ".GWWWWWGGGGGGGG.",
            ".BWWWWWWWWDWWWB.",
            ".BWWWWWWWWDDWWB.",
            ".BWWWWDDDDDDDWB.",
            ".BWWWDDDDDDDDDB.",
            ".BWWDDDWWWDDWWB.",
            ".BWWDDWWWWDWWWB.",
            ".BWDDWWWWWWWWWB.",
            ".BWDWWWWWWWWWWB.",
            ".BwwwwwwwwwwwwB.",
            ".BBBBBBBBBBBBBB.",
            "..bbbbbbbbbbbb..",
            "................",
        ],
        Pe::Copy => [
            "................",
            ".....GGGGGGGGG..",
            "....GGGGGGGGGGG.",
            "..GGGGGGGGGGGGG.",
            ".GGGGGGGGGGGGGG.",
            ".GKGGGGGGGGGGGB.",
            ".GGGGGGGGGGGGGB.",
            ".BWWWWWWWWWWBGB.",
            ".BWWWWWWWWWWBGB.",
            ".BWWWWWWWWWWBGB.",
            ".BWWWWWWWWWWBB..",
            ".BWWWWWWWWWWB...",
            ".BwwwwwwwwwwB...",
            ".BBBBBBBBBBBB...",
            "..bbbbbbbbbb....",
            "................",
        ],
        Pe::Delete => [
            "................",
            "......GGGG......",
            "..GGGGGGGGGGGG..",
            ".GGGGGGGGGGGGGG.",
            "................",
            "..BWWWWWWWWWWB..",
            "..BWWBWWWWBWWB..",
            "..BWWBWWWWBWWB..",
            "..BWWBWWWWBWWB..",
            "..BWWBWWWWBWWB..",
            "..BWWBWWWWBWWB..",
            "..BWWBWWWWBWWB..",
            "..BwwwwwwwwwwB..",
            "...BBBBBBBBBB...",
            "....bbbbbbbb....",
            "................",
        ],
        Pe::News => [
            "................",
            "................",
            ".GGGGGGGGGGGGGG.",
            ".GDDDDDDDDDDDDG.",
            ".GDWWWWWWWWWWDG.",
            ".GDDDDDDDDDDDDG.",
            ".GDWWWWWWWWDDDG.",
            ".GDDDDDDDDDDDDG.",
            ".GDWWWWWWDDDDDG.",
            ".GDDDDDDDDDDDDG.",
            ".GGGGGGGGGGGGGG.",
            "...GGG..........",
            "...GG...........",
            "...G............",
            "................",
            "................",
        ],
        Pe::Cat => [
            "................",
            "...K.....K......",
            "...KK...KK......",
            "...KKKKKKK......",
            "...KKKKKKK......",
            "...KwKKKwK......",
            "...KKKKKKK......",
            "....KKKKK.......",
            "....KKKKKK......",
            "...KKKKKKKK.....",
            "...KKKKKKKK.....",
            "...KKKKKKKKK....",
            "...KKKKKKKKK.KK.",
            "...KKKKKKKKKKK..",
            "....KK..KK......",
            "................",
        ],
    }
}

/// Настройка «Иконки» макета Prism Launcher: пиксельные как в Prism или стиль иконок лаунчера.
static THEME_ICONS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn theme_icon(ic: Pe) -> Icon {
    match ic {
        Pe::Home => Icon::Home,
        Pe::Add => Icon::Plus,
        Pe::Folder => Icon::Folder,
        Pe::Settings => Icon::Settings,
        Pe::Browse => Icon::Browse,
        Pe::Skins => Icon::Skins,
        Pe::Launch => Icon::Play,
        Pe::Stop => Icon::Stop,
        Pe::Edit => Icon::Cube,
        Pe::Group => Icon::Star,
        Pe::Export => Icon::Share,
        Pe::Copy => Icon::Copy,
        Pe::Delete => Icon::Close,
        Pe::News => Icon::Bell,
        Pe::Cat => Icon::Heart,
    }
}

/// Иконка макета: пиксельная (как в Prism) или в стиле иконок лаунчера.
pub fn pe_icon(p: &Painter, ic: Pe, r: Rect, dim: bool) {
    if THEME_ICONS.load(std::sync::atomic::Ordering::Relaxed) && ic != Pe::Cat {
        let col = match ic {
            _ if dim => TEXT_FAINT(),
            Pe::Launch | Pe::Add | Pe::News => GREEN(),
            Pe::Delete | Pe::Stop => lighten(DANGER(), 0.15),
            _ => TEXT_DIM().lerp_to_gamma(TEXT(), 0.5),
        };
        icon(p, theme_icon(ic), r.shrink(r.width() * 0.08), col);
        return;
    }
    pe_pixels(p, ic, r, dim);
}

/// Иконка целыми пикселями экрана (одним мешем — см. pixicons::draw). `dim` — серая (недоступно).
fn pe_pixels(p: &Painter, ic: Pe, r: Rect, dim: bool) {
    let ppp = p.ctx().pixels_per_point();
    // клетка не обязательно целая: края клеток прижимаются к пикселям экрана,
    // поэтому 22-px иконка не сжимается до 16 px и не размывается
    let side = ((r.width().min(r.height()) * ppp).round() / ppp).max(16.0 / ppp);
    let cell = side / 16.0;
    let snap = |v: f32| (v * ppp).round() / ppp;
    let o = pos2(snap(r.center().x - side / 2.0), snap(r.center().y - side / 2.0));
    let mut mesh = Mesh::default();
    for (y, row) in pe_art(ic).iter().enumerate() {
        for (x, ch) in row.bytes().enumerate() {
            let Some(mut col) = pe_color(ch) else { continue };
            if dim {
                let l = (col.r() as u32 * 3 + col.g() as u32 * 6 + col.b() as u32) / 10;
                let l = (l as f32 * 0.55) as u8 + 30;
                col = Color32::from_rgb(l, l, l + 4);
            }
            let a = o + vec2(x as f32, y as f32) * cell;
            let b = a + Vec2::splat(cell);
            mesh.add_colored_rect(Rect::from_min_max(pos2(snap(a.x), snap(a.y)), pos2(snap(b.x), snap(b.y))), col);
        }
    }
    p.add(Shape::mesh(mesh));
}

fn chevron(p: &Painter, c: egui::Pos2, open: bool, col: Color32) {
    let s = 4.0;
    let pts = if open {
        vec![c + vec2(-s, -s * 0.5), c + vec2(0.0, s * 0.5), c + vec2(s, -s * 0.5)]
    } else {
        vec![c + vec2(-s * 0.5, -s), c + vec2(s * 0.5, 0.0), c + vec2(-s * 0.5, s)]
    };
    p.add(Shape::line(pts, Stroke::new(1.6, col)));
}

// ───────────────────────── мелкие виджеты ─────────────────────────

/// Кнопка тулбара Prism: иконка 22 px и подпись, при наведении — плашка.
fn tool_button(ui: &mut Ui, ic: Pe, text: &str, menu: bool) -> egui::Response {
    tool_button_on(ui, ic, text, menu, false)
}

/// Узкое окно: кнопки тулбара без подписей (подпись — во всплывающей подсказке).
static COMPACT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Кнопка тулбара; `active` — открыт этот раздел (нажатая кнопка, как checked в Qt).
fn tool_button_on(ui: &mut Ui, ic: Pe, text: &str, menu: bool, active: bool) -> egui::Response {
    let font = regular(14.5);
    let compact = COMPACT.load(std::sync::atomic::Ordering::Relaxed);
    let tw = if compact { -4.0 } else { ui.painter().layout_no_wrap(text.to_string(), font.clone(), TEXT()).size().x };
    let w = 8.0 + 22.0 + 4.0 + tw + if menu { 16.0 } else { 0.0 } + 8.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    let t = hover_anim(ui, resp.id, resp.hovered());
    if active {
        ui.painter().rect(r, rr(3), hex(0x413e3e), Stroke::new(1.0, hex(0x524e4e)), egui::StrokeKind::Inside);
    } else if t > 0.0 {
        ui.painter().rect_filled(r, rr(3), RAISED().gamma_multiply(t));
    }
    pe_icon(ui.painter(), ic, Rect::from_center_size(pos2(r.left() + 19.0, r.center().y), Vec2::splat(22.0)), false);
    if compact {
        return resp.on_hover_text(text);
    }
    text_at(ui.painter(), pos2(r.left() + 34.0, r.center().y), Align2::LEFT_CENTER, text, font, TEXT());
    if menu {
        chevron(ui.painter(), pos2(r.right() - 12.0, r.center().y), true, TEXT_DIM());
    }
    resp
}

/// Строка действия в правой панели. Возвращает (клик по строке, клик по стрелке).
fn action_row(ui: &mut Ui, ic: Pe, text: &str, enabled: bool, menu: bool) -> (egui::Response, Rect) {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 31.0), if enabled { Sense::click() } else { Sense::hover() });
    let t = if enabled { hover_anim(ui, resp.id, resp.hovered()) } else { 0.0 };
    if t > 0.0 {
        ui.painter().rect_filled(r, rr(3), RAISED().gamma_multiply(t));
    }
    pe_icon(ui.painter(), ic, Rect::from_center_size(pos2(r.left() + 20.0, r.center().y), Vec2::splat(17.0)), !enabled);
    let col = if enabled { TEXT() } else { hex(0x75828d) };
    text_at(ui.painter(), pos2(r.left() + 32.0, r.center().y), Align2::LEFT_CENTER, text, regular(14.5), col);
    let arrow = Rect::from_center_size(pos2(r.right() - 14.0, r.center().y), vec2(24.0, 28.0));
    if menu {
        chevron(ui.painter(), arrow.center(), true, TEXT_DIM());
    }
    (resp, arrow)
}

fn centered_job(text: &str, size: f32, col: Color32, width: f32, rows: usize) -> LayoutJob {
    let mut job = LayoutJob::simple(text.to_string(), regular(size), col, width);
    job.halign = Align::Center;
    job.wrap.max_rows = rows;
    job.wrap.break_anywhere = true;
    job
}

// ───────────────────────── окно ─────────────────────────

impl App {
    /// Группа сборки для сетки: первая не «Избранное» (её id в движке — `group:favorites`), иначе "" — «Без группы».
    fn group_of(&self, inst: &Inst) -> String {
        if crate::horizon::is_horizon(&inst.id) {
            return "horizon".into();
        }
        inst.groups
            .iter()
            .find(|g| g.as_str() != "group:favorites" && self.groups.iter().any(|(id, _)| id == *g))
            .cloned()
            .unwrap_or_default()
    }

    fn group_name(&self, id: &str) -> String {
        if id.is_empty() {
            return "Без группы".into();
        }
        self.groups.iter().find(|(g, _)| g == id).map(|(_, n)| n.clone()).unwrap_or_default()
    }

    fn selected(&self) -> Option<Inst> {
        // как в Prism: если ничего не выбрано — последняя сборка, в которую играли (список уже отсортирован)
        self.prefs.prism_sel.as_deref().and_then(|id| self.inst(id)).or(self.instances.first()).cloned()
    }

    fn select(&mut self, id: &str) {
        if self.prefs.prism_sel.as_deref() != Some(id) {
            self.prefs.prism_sel = Some(id.to_string());
            crate::prefs::save(&self.prefs);
        }
    }

    /// Всё окно в макете Prism Launcher.
    pub fn prism_ui(&mut self, ui: &mut Ui) {
        THEME_ICONS.store(self.prefs.prism_icons == "theme", std::sync::atomic::Ordering::Relaxed);
        egui::Panel::top("p-tool")
            .exact_size(42.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(BG()))
            .show_inside(ui, |ui| self.prism_toolbar(ui));
        if self.prefs.prism_status {
            egui::Panel::bottom("p-status")
                .exact_size(24.0)
                .resizable(false)
                .show_separator_line(false)
                .frame(Frame::new().fill(BG()))
                .show_inside(ui, |ui| self.prism_status(ui));
        }
        if self.prefs.prism_news {
            egui::Panel::bottom("p-news")
                .exact_size(30.0)
                .resizable(false)
                .show_separator_line(false)
                .frame(Frame::new().fill(BG()))
                .show_inside(ui, |ui| self.prism_news(ui));
        }
        egui::Panel::right("p-side")
            .exact_size(210.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(PANEL()).inner_margin(Margin { left: 8, right: 8, top: 14, bottom: 8 }))
            .show_inside(ui, |ui| self.prism_side(ui));
        let mut grid_rect = Rect::NOTHING;
        egui::CentralPanel::default().frame(Frame::new().fill(CONTENT())).show_inside(ui, |ui| {
            grid_rect = ui.max_rect();
            if !self.ready {
                return;
            }
            if !self.prefs.cat_off {
                self.draw_cat(ui);
            }
            egui::ScrollArea::vertical().id_salt("p-grid").auto_shrink(false).show(ui, |ui| {
                Frame::new().inner_margin(Margin { left: 4, right: 10, top: 8, bottom: 16 }).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.prism_grid(ui);
                });
            });
        });
        // остальные страницы — окном поверх сетки и панели действий
        if self.page != Page::Library && self.ready {
            let area = Rect::from_min_max(grid_rect.min, pos2(ui.max_rect().right(), grid_rect.bottom()));
            self.prism_page_window(ui, area.shrink(14.0));
        }
    }

    fn prism_toolbar(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let drag = ui.interact(full, Id::new("drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!max));
        }
        ui.painter().hline(full.x_range(), full.bottom() - 0.5, Stroke::new(1.0, hex(0x262525)));
        // подписи на тулбаре не прячем: окно просто не даём сузить меньше, чем нужно тулбару
        let compact = false;
        COMPACT.store(compact, std::sync::atomic::Ordering::Relaxed);
        let (mut left_end, mut right_start) = (full.left(), full.right());
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add_space(14.0);
            let page = self.page.clone();
            // «Главная» — сетка сборок: закрывает открытое поверх неё окно
            if tool_button_on(ui, Pe::Home, "Главная", false, matches!(page, Page::Library | Page::Home)).clicked() {
                self.go(ui, Page::Library);
            }
            let add = tool_button(ui, Pe::Add, "Добавить экземпляр", false);
            egui::Popup::menu(&add).show(|ui| {
                if ui.button("Новый экземпляр…").clicked() {
                    self.create.open = true;
                }
                if ui.button("Импорт…").clicked() {
                    self.import_open = true;
                }
                if ui.button("Сборки Modrinth / CurseForge…").clicked() {
                    self.browse_tab = 1;
                    self.go(ui, Page::Browse);
                }
            });
            for (ic, name, p) in [(Pe::Settings, "Параметры", Page::Settings), (Pe::Browse, "Обзор", Page::Browse), (Pe::Skins, "Скины", Page::Skins)] {
                let active = page == p;
                if tool_button_on(ui, ic, name, false, active).clicked() {
                    if p == Page::Browse && !active {
                        self.browse_tab = 0;
                    }
                    self.go(ui, if active { Page::Library } else { p });
                }
            }
            // котик: вкл/выкл
            let (r, resp) = ui.allocate_exact_size(if self.prefs.prism_cat_button { Vec2::splat(32.0) } else { Vec2::ZERO }, Sense::click());
            let t = hover_anim(ui, resp.id, resp.hovered());
            let on = !self.prefs.cat_off;
            ui.painter().rect_filled(r, rr(5), if on { hex(0x413e3e) } else { RAISED().gamma_multiply(t) });
            pe_icon(ui.painter(), Pe::Cat, r.shrink(5.0), !on);
            if resp.on_hover_text("Котик").clicked() {
                self.prefs.cat_off = on;
                crate::prefs::save(&self.prefs);
            }
            left_end = ui.cursor().left();

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.add_space(4.0);
                self.window_buttons(ui);
                ui.add_space(8.0);
                // учётные записи
                let font = regular(14.5);
                let tw = if compact { -10.0 } else { ui.painter().layout_no_wrap("Учётные записи".into(), font.clone(), TEXT()).size().x };
                let (r, resp) = ui.allocate_exact_size(vec2(tw + 46.0, 32.0), Sense::click());
                let t = hover_anim(ui, resp.id, resp.hovered());
                if t > 0.0 {
                    ui.painter().rect_filled(r, rr(3), RAISED().gamma_multiply(t));
                }
                self.account_face(ui, Rect::from_center_size(pos2(r.left() + 19.0, r.center().y), Vec2::splat(20.0)));
                if !compact {
                    text_at(ui.painter(), pos2(r.left() + 36.0, r.center().y), Align2::LEFT_CENTER, "Учётные записи", font, TEXT());
                }
                egui::Popup::menu(&resp).show(|ui| {
                    let current = self.default_account().map(|a| a.id);
                    let mut pick = None;
                    for a in &self.accounts {
                        let mark = if Some(a.id) == current { "● " } else { "   " };
                        let kind = if a.elyby { "Ely.by" } else { "Microsoft" };
                        if ui.button(format!("{mark}{} · {kind}", a.name)).clicked() {
                            pick = Some(a.id);
                        }
                    }
                    if let Some(id) = pick {
                        self.be.set_default_user(id);
                    }
                    if !self.accounts.is_empty() {
                        ui.separator();
                    }
                    if ui.button("Управление учётными записями…").clicked() {
                        self.settings_section = ACCOUNTS_SECTION;
                        self.go(ui, Page::Settings);
                    }
                });
                ui.add_space(6.0);
                self.update_button(ui);
                self.downloads_button(ui);
                ui.add_space(4.0);
                self.running_pill(ui);
                right_start = ui.min_rect().left();
            });
        });
        let need = ((left_end - full.left()) + (full.right() - right_start) + 24.0).max(760.0).ceil();
        let id = Id::new("p-min-width");
        if ui.ctx().data(|d| d.get_temp::<f32>(id)) != Some(need) {
            ui.ctx().data_mut(|d| d.insert_temp(id, need));
            ui.ctx().send_viewport_cmd(ViewportCommand::MinInnerSize(vec2(need, 520.0)));
        }
    }

    fn prism_grid(&mut self, ui: &mut Ui) {
        if self.instances.is_empty() && self.horizon.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                label(ui, "Экземпляров пока нет", semibold(16.0), TEXT_DIM());
                label(ui, "«Добавить экземпляр» — создать, импортировать или скачать сборку", regular(13.5), TEXT_FAINT());
            });
            return;
        }
        // группы: «Без группы» первой, остальные по имени
        let mut order: Vec<(String, String)> = vec![(String::new(), "Без группы".into())];
        let mut named: Vec<(String, String)> = self.groups.iter().filter(|(id, _)| id != "group:favorites").cloned().collect();
        named.sort_by_key(|(_, n)| n.to_lowercase());
        order.extend(named);
        let mut by: Vec<(String, String, Vec<Inst>)> = order.into_iter().map(|(id, n)| (id, n, vec![])).collect();
        if !self.horizon.is_empty() {
            by.push(("horizon".into(), "Horizon (Rust)".into(), vec![]));
        }
        for inst in self.instances.iter().chain(self.horizon.iter()) {
            let g = self.group_of(inst);
            if let Some(slot) = by.iter_mut().find(|(id, ..)| *id == g) {
                slot.2.push(inst.clone());
            }
        }
        for slot in &mut by {
            slot.2.sort_by_key(|i| i.name.to_lowercase());
        }
        let now = Self::now(ui);
        let sel_id = self.selected().map(|i| i.id);
        let (mut select, mut play, mut toggle) = (None::<String>, None::<String>, None::<String>);
        let mut ctx_action: Option<(String, u8)> = None;
        for (gid, gname, list) in by {
            if list.is_empty() {
                continue;
            }
            let collapsed = self.prefs.prism_collapsed.contains(&gid);
            // заголовок группы
            let (hr, hresp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
            chevron(ui.painter(), pos2(hr.left() + 10.0, hr.center().y), !collapsed, hex(0x8f8b8a));
            let tr = text_at(ui.painter(), pos2(hr.left() + 22.0, hr.center().y), Align2::LEFT_CENTER, &gname, semibold(14.5), hex(0x8f8b8a));
            ui.painter().hline(tr.right() + 10.0..=hr.right() - 6.0, hr.center().y, Stroke::new(1.0, hex(0x1f1c1c)));
            if hresp.clicked() {
                toggle = Some(gid.clone());
            }
            if collapsed {
                continue;
            }
            // плитки
            let tile = self.prefs.prism_tile.clamp(32.0, 96.0);
            let (cw, gap) = ((tile + 52.0).max(100.0), 5.0);
            let w = ui.available_width();
            let cols = ((w + gap) / (cw + gap)).floor().max(1.0) as usize;
            for row in list.chunks(cols) {
                let jobs: Vec<_> = row
                    .iter()
                    .map(|i| {
                        let sel = sel_id.as_deref() == Some(i.id.as_str());
                        ui.fonts_mut(|f| f.layout_job(centered_job(&i.name, 14.0, if sel { Color32::WHITE } else { TEXT() }, cw - 8.0, if sel { 8 } else { 4 })))
                    })
                    .collect();
                let text_h = jobs.iter().map(|g| g.size().y).fold(0.0, f32::max);
                let h = 8.0 + tile + 6.0 + text_h + 10.0;
                let (rr_, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                for (k, (inst, galley)) in row.iter().zip(jobs).enumerate() {
                    let cell = Rect::from_min_size(pos2(rr_.left() + k as f32 * (cw + gap), rr_.top()), vec2(cw, h));
                    let resp = ui.interact(cell, Id::new(("p-tile", &inst.id)), Sense::click());
                    let sel = sel_id.as_deref() == Some(inst.id.as_str());
                    let t = hover_anim(ui, resp.id, resp.hovered());
                    let ir = Rect::from_center_size(pos2(cell.center().x, cell.top() + 8.0 + tile / 2.0), Vec2::splat(tile));
                    if t > 0.0 && !sel {
                        ui.painter().rect_filled(ir.expand(3.0), rr(3), RAISED().gamma_multiply(t * 0.7));
                    }
                    Self::inst_icon(ui, ir, inst, 4.0);
                    if self.is_running(&inst.id) || self.is_launching(&inst.id, now) {
                        let b = Rect::from_min_size(ir.right_bottom() - vec2(14.0, 14.0), Vec2::splat(16.0));
                        ui.painter().rect_filled(b.expand(1.0), rr(3), CONTENT());
                        pe_icon(ui.painter(), Pe::Launch, b, false);
                    }
                    let pos = pos2(cell.center().x, ir.bottom() + 6.0);
                    let tr = galley.rect.translate(pos.to_vec2()).expand2(vec2(4.0, 2.0));
                    let lbg = Rect::from_min_max(pos2(cell.left(), tr.top()), pos2(cell.right(), tr.bottom()));
                    ui.painter().rect_filled(lbg, 0.0, if sel { accent() } else { hex(0x151313).lerp_to_gamma(RAISED(), t * 0.5) });
                    ui.painter().galley(pos, galley, TEXT());
                    if resp.clicked() {
                        select = Some(inst.id.clone());
                    }
                    if resp.double_clicked() {
                        play = Some(inst.id.clone());
                    }
                    resp.context_menu(|ui| {
                        let running = self.is_running(&inst.id);
                        for (k, name) in [(0u8, if running { "Остановить" } else { "Запустить" }), (1, "Изменить"), (2, "Изменить группу"), (3, "Папка"), (4, "Экспортировать"), (5, "Копировать"), (6, "Удалить")] {
                            if ui.button(name).clicked() {
                                ctx_action = Some((inst.id.clone(), k));
                            }
                        }
                    });
                    if resp.secondary_clicked() {
                        select = Some(inst.id.clone());
                    }
                }
                ui.add_space(4.0);
            }
            ui.add_space(4.0);
        }
        if let Some(g) = toggle {
            if let Some(p) = self.prefs.prism_collapsed.iter().position(|x| *x == g) {
                self.prefs.prism_collapsed.remove(p);
            } else {
                self.prefs.prism_collapsed.push(g);
            }
            crate::prefs::save(&self.prefs);
        }
        if let Some(id) = select {
            self.select(&id);
        }
        if let Some(id) = play {
            if !self.is_running(&id) {
                self.play(ui, &id);
            }
        }
        if let Some((id, k)) = ctx_action {
            self.select(&id);
            self.prism_action(ui, &id, k);
        }
    }

    /// Действия сборки: 0 запустить/остановить, 1 изменить, 2 группа, 3 папка, 4 экспорт, 5 копия, 6 удалить.
    fn prism_action(&mut self, ui: &Ui, id: &str, k: u8) {
        match k {
            0 => self.play(ui, id),
            1 => self.go(ui, Page::Instance(id.to_string())),
            // у сборок Horizon нет групп, экспорта и копий — это в Horizon Launcher
            2 | 4 | 5 if crate::horizon::is_horizon(id) => {}
            2 => {
                let g = self.inst(id).map(|i| self.group_of(i)).unwrap_or_default();
                let name = if g.is_empty() { String::new() } else { self.group_name(&g) };
                self.group_edit = Some((id.to_string(), name));
            }
            3 if crate::horizon::is_horizon(id) => {
                if let Some(h) = self.horizon_home.get(id) {
                    crate::sys::open(h);
                }
            }
            3 => self.be.open_folder(id.to_string()),
            4 => self.open_export(id, ExportFormat::Prism),
            5 => self.be.duplicate(id.to_string()),
            6 => self.confirm_delete = Some(id.to_string()),
            _ => {}
        }
    }

    fn prism_side(&mut self, ui: &mut Ui) {
        let Some(inst) = self.selected() else { return };
        let now = Self::now(ui);
        ui.vertical_centered(|ui| {
            let hz = crate::horizon::is_horizon(&inst.id);
            let (r, resp) = ui.allocate_exact_size(vec2(64.0, 64.0), if hz { Sense::hover() } else { Sense::click() });
            Self::inst_icon(ui, r, &inst, 4.0);
            if !hz && resp.on_hover_text("Сменить иконку").clicked() {
                self.be.pick(crate::backend::Pick::InstanceIcon(inst.id.clone()));
            }
            ui.add_space(14.0);
            let g = ui.fonts_mut(|f| f.layout_job(centered_job(&inst.name, 14.5, TEXT(), ui.available_width() - 8.0, 4)));
            let (tr, _) = ui.allocate_exact_size(vec2(ui.available_width(), g.size().y), Sense::hover());
            ui.painter().galley(pos2(tr.center().x, tr.top()), g, TEXT());
        });
        ui.add_space(10.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        let id = inst.id.clone();
        let running = self.is_running(&id);
        let busy = running || self.is_launching(&id, now) || inst.installing;
        let mut act = None;
        let (resp, arrow) = action_row(ui, Pe::Launch, "Запустить", !busy, true);
        let arrow_hit = ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| arrow.contains(p));
        if resp.clicked() && !arrow_hit {
            act = Some(0);
        }
        if resp.clicked() && arrow_hit {
            egui::Popup::toggle_id(ui.ctx(), Id::new("p-launch-menu"));
        }
        egui::Popup::new(Id::new("p-launch-menu"), ui.ctx().clone(), egui::PopupAnchor::from(&resp), resp.layer_id)
            .open_memory(None)
            .show(|ui| {
                if ui.button("Запустить").clicked() {
                    act = Some(0);
                }
                if ui.button("Журнал и настройки…").clicked() {
                    act = Some(1);
                }
            });
        if action_row(ui, Pe::Stop, "Остановить", running, false).0.clicked() {
            act = Some(0);
        }
        ui.add_space(8.0);
        let rows = [(Pe::Edit, "Изменить", 1u8, false), (Pe::Group, "Изменить группу", 2, false), (Pe::Folder, "Папка", 3, false), (Pe::Export, "Экспортировать", 4, true), (Pe::Copy, "Копировать", 5, false), (Pe::Delete, "Удалить", 6, false)];
        for (ic, name, k, menu) in rows {
            // у сборок Horizon только запуск, папка и удаление — остальное в Horizon Launcher
            let enabled = !crate::horizon::is_horizon(&id) || matches!(k, 1 | 3 | 6);
            let (resp, arrow) = action_row(ui, ic, name, enabled, menu && enabled);
            let arrow_hit = ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| arrow.contains(p));
            if menu {
                let pid = Id::new("p-export-menu");
                if resp.clicked() && arrow_hit {
                    egui::Popup::toggle_id(ui.ctx(), pid);
                } else if resp.clicked() {
                    act = Some(k);
                }
                egui::Popup::new(pid, ui.ctx().clone(), egui::PopupAnchor::from(&resp), resp.layer_id).open_memory(None).show(|ui| {
                    for f in [ExportFormat::Prism, ExportFormat::Modrinth, ExportFormat::CurseForge] {
                        let name = match f {
                            ExportFormat::Prism => "Prism Launcher (.zip)",
                            ExportFormat::Modrinth => "Modrinth (.mrpack)",
                            ExportFormat::CurseForge => "CurseForge (.zip)",
                        };
                        if ui.button(name).clicked() {
                            self.open_export(&id, f);
                        }
                    }
                });
            } else if resp.clicked() {
                act = Some(k);
            }
        }
        if let Some(k) = act {
            self.prism_action(ui, &id, k);
        }
    }

    fn prism_news(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let (text, has_update) = match &self.update {
            Some(u) => (format!("TrollLauncher {}, доступно обновление", u.version), true),
            None => (format!("TrollLauncher {}", env!("CARGO_PKG_VERSION")), false),
        };
        let font = regular(14.0);
        let tw = ui.painter().layout_no_wrap(text.clone(), font.clone(), TEXT()).size().x;
        let r = Rect::from_center_size(full.center(), vec2(tw + 24.0, 26.0));
        let resp = ui.interact(r, Id::new("p-news"), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(r, rr(3), RAISED());
        }
        pe_icon(ui.painter(), Pe::News, Rect::from_center_size(pos2(r.left() + 8.0, r.center().y), Vec2::splat(16.0)), false);
        text_at(ui.painter(), pos2(r.left() + 20.0, r.center().y), Align2::LEFT_CENTER, &text, font.clone(), TEXT());
        if resp.clicked() {
            if has_update {
                self.update_dialog = true;
            } else {
                crate::sys::open(format!("{REPO_URL}/releases"));
            }
        }
        let more = "Больше новостей";
        let mw = ui.painter().layout_no_wrap(more.into(), font.clone(), TEXT()).size().x;
        let mr = Rect::from_min_max(pos2(full.right() - mw - 36.0, r.top()), pos2(full.right() - 8.0, r.bottom()));
        let mresp = ui.interact(mr, Id::new("p-more"), Sense::click());
        if mresp.hovered() {
            ui.painter().rect_filled(mr, rr(3), RAISED());
        }
        pe_icon(ui.painter(), Pe::News, Rect::from_center_size(pos2(mr.left() + 10.0, mr.center().y), Vec2::splat(16.0)), false);
        text_at(ui.painter(), pos2(mr.left() + 22.0, mr.center().y), Align2::LEFT_CENTER, more, font, TEXT());
        if mresp.clicked() {
            crate::sys::open(format!("{REPO_URL}/releases"));
        }
    }

    fn prism_status(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let total: u64 = self.instances.iter().map(|i| i.played_secs).sum();
        let mut s = String::new();
        if let Some(inst) = self.selected() {
            s.push_str(&format!("Minecraft {}", inst.game_version));
            if inst.loader != "Vanilla" && !inst.loader.is_empty() {
                s.push_str(&format!(" ({})", inst.loader));
            }
            if let Some(t) = inst.last_played {
                s.push_str(&format!(", в последний раз играли {}", t.with_timezone(&chrono::Local).format("%d.%m.%Y %H:%M")));
            }
            s.push_str(&format!(", общее время игры {}", prism_time(inst.played_secs)));
            s.push(' ');
        }
        s.push_str(&format!("Общее время игры: {}", prism_time(total)));
        ui.painter().with_clip_rect(full).text(pos2(full.left() + 2.0, full.center().y), Align2::LEFT_CENTER, s, regular(14.0), TEXT());
    }

    /// Страница (параметры, обзор, скины, сборка) окном поверх сетки, как отдельное окно Prism.
    fn prism_page_window(&mut self, ui: &mut Ui, r: Rect) {
        let title = match &self.page {
            Page::Home => "Главная".to_string(),
            Page::Settings => "Параметры".to_string(),
            Page::Browse => "Обзор".into(),
            Page::Skins => "Скины".into(),
            Page::Instance(id) => format!("Изменить экземпляр · {}", self.inst(id).map(|i| i.name.clone()).unwrap_or_default()),
            _ => String::new(),
        };
        // новое окно egui показывает со второго кадра — дорисовать сразу, не дожидаясь движения мыши
        if Self::now(ui) - self.page_changed < 0.5 {
            ui.ctx().request_repaint();
        }
        let mut close = ui.input(|i| i.key_pressed(egui::Key::Escape)) && ui.ctx().memory(|m| m.top_modal_layer().is_none());
        egui::Area::new(Id::new("p-page"))
            .fixed_pos(r.min)
            .order(egui::Order::Middle)
            .show(ui.ctx(), |ui| {
                Frame::new().fill(PANEL()).stroke(Stroke::new(1.0, hex(0x3a3636))).corner_radius(rr(4)).show(ui, |ui| {
                    ui.set_min_size(r.size());
                    ui.set_max_size(r.size());
                    // заголовок
                    let (hr, _) = ui.allocate_exact_size(vec2(r.width(), 34.0), Sense::hover());
                    ui.painter().rect_filled(hr, egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, BG());
                    ui.painter().hline(hr.x_range(), hr.bottom() - 0.5, Stroke::new(1.0, hex(0x262525)));
                    text_at(ui.painter(), pos2(hr.left() + 14.0, hr.center().y), Align2::LEFT_CENTER, &title, semibold(14.5), TEXT());
                    let cr = Rect::from_center_size(pos2(hr.right() - 20.0, hr.center().y), vec2(30.0, 26.0));
                    let cresp = ui.interact(cr, Id::new("p-page-x"), Sense::click());
                    let t = hover_anim(ui, cresp.id, cresp.hovered());
                    ui.painter().rect_filled(cr, rr(3), DANGER().gamma_multiply(t));
                    icon(ui.painter(), Icon::Close, cr.shrink(7.0), TEXT_DIM().lerp_to_gamma(Color32::WHITE, t));
                    if cresp.on_hover_text("Закрыть (Esc)").clicked() {
                        close = true;
                    }
                    let body_h = r.height() - 34.0 - 46.0;
                    let page = self.page.clone();
                    egui::ScrollArea::vertical().id_salt(format!("p-{page:?}")).max_height(body_h).min_scrolled_height(body_h).auto_shrink(false).show(ui, |ui| {
                        Frame::new().inner_margin(Margin { left: 22, right: 22, top: 18, bottom: 24 }).show(ui, |ui| {
                            ui.set_width(r.width() - 44.0 - 14.0);
                            match page {
                                Page::Home => self.page_home(ui),
                                Page::Browse => self.page_browse(ui),
                                Page::Skins => self.page_skins(ui),
                                Page::Settings => self.page_settings(ui),
                                Page::Instance(id) => self.page_instance(ui, &id),
                                _ => {}
                            }
                        });
                    });
                    // низ: «Закрыть»
                    let (br, _) = ui.allocate_exact_size(vec2(r.width(), 46.0), Sense::hover());
                    ui.painter().hline(br.x_range(), br.top() + 0.5, Stroke::new(1.0, hex(0x262525)));
                    let bb = Rect::from_min_size(pos2(br.right() - 112.0, br.top() + 9.0), vec2(100.0, 28.0));
                    let bresp = ui.interact(bb, Id::new("p-page-close"), Sense::click());
                    let t = hover_anim(ui, bresp.id, bresp.hovered());
                    ui.painter().rect(bb, rr(3), RAISED().lerp_to_gamma(HOVER(), t), Stroke::new(1.0, hex(0x3a3636)), egui::StrokeKind::Inside);
                    text_at(ui.painter(), bb.center(), Align2::CENTER_CENTER, "Закрыть", regular(14.0), TEXT());
                    if bresp.clicked() {
                        close = true;
                    }
                });
            });
        if close {
            self.page = Page::Library;
        }
    }

    /// Окна макета Prism Launcher: «Изменить группу».
    pub fn prism_dialogs(&mut self, ctx: &egui::Context) {
        let Some((id, mut name)) = self.group_edit.clone() else { return };
        let frame = Frame::new().fill(PANEL()).corner_radius(rr(6)).stroke(Stroke::new(1.0, BORDER())).inner_margin(Margin::same(18));
        let mut done: Option<bool> = None;
        let groups: Vec<String> = {
            let mut g: Vec<String> = self.groups.iter().filter(|(i, _)| i != "group:favorites").map(|(_, n)| n.clone()).collect();
            g.sort_by_key(|n| n.to_lowercase());
            g
        };
        let r = egui::Modal::new(Id::new("p-group")).frame(frame).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            label(ui, "Изменить группу", bold(17.0), TEXT());
            label(ui, "Название группы (пусто — «Без группы»):", regular(13.5), TEXT_DIM());
            let te = ui.add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY).hint_text("Без группы"));
            if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                done = Some(true);
            }
            te.request_focus();
            if !groups.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for g in &groups {
                        if chip(ui, g, RAISED(), TEXT()).clicked() {
                            name = g.clone();
                        }
                    }
                });
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if button(ui, "OK", None, Kind::Primary, 32.0).clicked() {
                    done = Some(true);
                }
                if button(ui, "Отмена", None, Kind::Secondary, 32.0).clicked() {
                    done = Some(false);
                }
            });
        });
        if r.should_close() {
            done = Some(false);
        }
        match done {
            Some(true) => {
                let keep: Vec<String> = self.inst(&id).map(|i| i.groups.iter().filter(|g| g.as_str() == "group:favorites").cloned().collect()).unwrap_or_default();
                self.be.set_group(id, name, keep);
                self.group_edit = None;
            }
            Some(false) => self.group_edit = None,
            None => self.group_edit = Some((id, name)),
        }
    }
}

/// Время как в строке статуса Prism: «3д 15ч 17мин», «4мин 26с».
fn prism_time(secs: u64) -> String {
    let (d, h, m, s) = (secs / 86400, secs / 3600 % 24, secs / 60 % 60, secs % 60);
    if d > 0 {
        format!("{d}д {h}ч {m}мин")
    } else if h > 0 {
        format!("{h}ч {m}мин")
    } else if m > 0 {
        format!("{m}мин {s}с")
    } else {
        format!("{s}с")
    }
}
