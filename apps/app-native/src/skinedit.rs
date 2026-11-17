//! Редактор скина (как в sfatm и Blockbench).
//!
//! - Рисование прямо по 3D-модели (ЛКМ — кисть, ПКМ/СКМ — крутить, колесо — приблизить)
//!   и по развёртке 64×64 рядом.
//! - Инструменты: карандаш, ластик, пипетка, заливка грани; отмена/повтор (Ctrl+Z, Ctrl+Y).
//! - Слои: основной и второй (шляпа, куртка, рукава, штанины).
//! - «Одежда с другого скина», как в гардеробе sfatm: берёшь скин (из библиотеки, файлом
//!   или по нику) и отмечаешь части — голову, тело, руки, ноги; можно взять только второй
//!   слой, чтобы надеть чужую одежду поверх своей. Смешивание не разрушает исходник:
//!   части пересчитываются заново, а правки кистью лежат отдельным слоем поверх.

use crate::app::App;
use crate::backend::{Pick, SkinItem};
use crate::skin3d::{self, Pose};
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align, Align2, Color32, Id, Key, Layout, Margin, PointerButton, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2, pos2, vec2};
use std::collections::HashMap;

pub type Px = [u8; 4];
const N: usize = 64 * 64;

#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Pencil,
    Eraser,
    Picker,
    Fill,
}

/// Части тела для «одежды»: прямоугольники развёртки основного и второго слоя (x, y, w, h).
const PARTS: [(&str, &[[usize; 4]], &[[usize; 4]]); 4] = [
    ("Голова", &[[0, 0, 32, 16]], &[[32, 0, 32, 16]]),
    ("Тело", &[[16, 16, 24, 16]], &[[16, 32, 24, 16]]),
    ("Руки", &[[40, 16, 16, 16], [32, 48, 16, 16]], &[[40, 32, 16, 16], [48, 48, 16, 16]]),
    ("Ноги", &[[0, 16, 16, 16], [16, 48, 16, 16]], &[[0, 32, 16, 16], [0, 48, 16, 16]]),
];

/// Кубы развёртки (u, v, ширина, высота, глубина, второй слой) — для заливки по граням и затемнения.
fn cubes(slim: bool) -> Vec<(usize, usize, usize, usize, usize, bool)> {
    let aw = if slim { 3 } else { 4 };
    vec![
        (0, 0, 8, 8, 8, false),
        (32, 0, 8, 8, 8, true),
        (16, 16, 8, 12, 4, false),
        (16, 32, 8, 12, 4, true),
        (40, 16, aw, 12, 4, false),
        (40, 32, aw, 12, 4, true),
        (32, 48, aw, 12, 4, false),
        (48, 48, aw, 12, 4, true),
        (0, 16, 4, 12, 4, false),
        (0, 32, 4, 12, 4, true),
        (16, 48, 4, 12, 4, false),
        (0, 48, 4, 12, 4, true),
    ]
}

/// Грани всех кубов: (x, y, w, h, второй слой).
fn faces(slim: bool) -> Vec<(usize, usize, usize, usize, bool)> {
    let mut out = vec![];
    for (u, v, w, h, d, o) in cubes(slim) {
        for (x, y, fw, fh) in [(u + d, v, w, d), (u + d + w, v, w, d), (u, v + d, d, h), (u + d, v + d, w, h), (u + d + w, v + d, d, h), (u + 2 * d + w, v + d, w, h)] {
            out.push((x, y, fw, fh, o));
        }
    }
    out
}

fn face_of(slim: bool, x: usize, y: usize) -> Option<(usize, usize, usize, usize, bool)> {
    faces(slim).into_iter().find(|&(fx, fy, w, h, _)| x >= fx && x < fx + w && y >= fy && y < fy + h)
}

/// Пиксель относится ко второму слою (или вне развёртки — `None`).
fn layer_of(slim: bool, x: usize, y: usize) -> Option<bool> {
    face_of(slim, x, y).map(|f| f.4)
}

/// Старый скин 64×32 → 64×64: левые рука и нога копией правых.
pub fn to_64(w: u32, h: u32, rgba: &[u8]) -> Option<Vec<Px>> {
    if w != 64 || (h != 64 && h != 32) || rgba.len() < (w * h * 4) as usize {
        return None;
    }
    let mut px = vec![[0u8; 4]; N];
    for (i, c) in rgba.chunks_exact(4).enumerate().take((w * h) as usize) {
        px[i] = [c[0], c[1], c[2], c[3]];
    }
    if h == 32 {
        for y in 0..16 {
            for x in 0..16 {
                px[(48 + y) * 64 + 16 + x] = px[(16 + y) * 64 + x];
                px[(48 + y) * 64 + 32 + x] = px[(16 + y) * 64 + 40 + x];
            }
        }
    }
    Some(px)
}

/// Заготовка нового скина: кожа, волосы, глаза, футболка, штаны, ботинки — чтобы было на чём рисовать.
fn template() -> Vec<Px> {
    let mut px = vec![[0u8; 4]; N];
    let rgb = |v: u32| [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255];
    let (skin, hair, shirt, pants, shoes) = (rgb(0xc68e6b), rgb(0x3b2a1e), rgb(0x2f9e9e), rgb(0x343a8c), rgb(0x55555e));
    for (u, v, w, h, d, outer) in cubes(false) {
        if outer {
            continue;
        }
        let body = (u, v) == (16, 16);
        let arm = (u, v) == (40, 16) || (u, v) == (32, 48);
        let leg = (u, v) == (0, 16) || (u, v) == (16, 48);
        for (fi, (x0, y0, fw, fh)) in [(u + d, v, w, d), (u + d + w, v, w, d), (u, v + d, d, h), (u + d, v + d, w, h), (u + d + w, v + d, d, h), (u + 2 * d + w, v + d, w, h)].into_iter().enumerate() {
            for y in 0..fh {
                for x in 0..fw {
                    let c = if (u, v) == (0, 0) {
                        if fi == 0 || (fi >= 2 && y < 2) || (fi == 5 && y < 6) { hair } else { skin }
                    } else if body {
                        shirt
                    } else if arm {
                        if fi >= 2 && y < 4 || fi == 0 { shirt } else { skin }
                    } else if leg {
                        if fi == 1 || fi >= 2 && y >= fh - 2 { shoes } else { pants }
                    } else {
                        skin
                    };
                    px[(y0 + y) * 64 + x0 + x] = c;
                }
            }
        }
    }
    // глаза и рот на лице (лицо — 8..16 × 8..16)
    for (x, c) in [(9, 0xffffff), (10, 0x3b4fd8), (13, 0x3b4fd8), (14, 0xffffff)] {
        px[12 * 64 + x] = rgb(c);
    }
    for x in 11..13 {
        px[14 * 64 + x] = rgb(0x8a4f3a);
    }
    px
}

pub struct Donor {
    pub name: String,
    pub px: Vec<Px>,
}

pub struct SkinEditor {
    /// Скин из библиотеки, который правим (при «Сохранить» заменяется)
    pub src: Option<String>,
    pub name: String,
    pub slim: bool,
    pub favorite: bool,
    orig: Vec<Px>,
    pub donor: Option<Donor>,
    take: [bool; 4],
    outer_only: bool,
    edits: HashMap<usize, Px>,
    undo: Vec<HashMap<usize, Option<Px>>>,
    redo: Vec<HashMap<usize, Option<Px>>>,
    stroke: Option<HashMap<usize, Option<Px>>>,
    last_texel: Option<(usize, usize)>,
    mixed: Vec<Px>,
    composed: Vec<Px>,
    tex: Option<TextureHandle>,
    dirty: bool,
    tool: Tool,
    color: Color32,
    recent: Vec<Color32>,
    /// 0 — основной слой, 1 — второй
    layer: usize,
    show_overlay: bool,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    pub nick: String,
    pub nick_src: usize,
    pub busy: bool,
    changed: bool,
}

const PALETTE: [u32; 16] = [
    0x000000, 0x3f3f46, 0x9a9aa6, 0xffffff, 0x7a4a2a, 0xc68642, 0xf1c27d, 0xffdbac, 0xd8263a, 0xff8a1f, 0xffd23f, 0x3fae4f, 0x2e9e9e, 0x2f2cf8, 0x7d3cff, 0xe0679f,
];

impl SkinEditor {
    pub fn new(src: Option<&SkinItem>) -> Self {
        let (orig, name, slim, favorite, id) = match src.and_then(|s| s.rgba.as_ref().and_then(|(w, h, d)| to_64(*w, *h, d)).map(|px| (px, s))) {
            Some((px, s)) => (px, s.name.clone(), s.slim, s.favorite, Some(s.id.clone())),
            None => (template(), "Новый скин".into(), false, false, None),
        };
        let mut e = Self {
            src: id,
            name,
            slim,
            favorite,
            mixed: orig.clone(),
            composed: orig.clone(),
            orig,
            donor: None,
            take: [false, true, true, true],
            outer_only: false,
            edits: HashMap::new(),
            undo: vec![],
            redo: vec![],
            stroke: None,
            last_texel: None,
            tex: None,
            dirty: true,
            tool: Tool::Pencil,
            color: Color32::from_rgb(0x2f, 0x2c, 0xf8),
            recent: vec![],
            layer: 0,
            show_overlay: true,
            yaw: 0.5,
            pitch: -0.15,
            zoom: 1.0,
            nick: String::new(),
            nick_src: 0,
            busy: false,
            changed: false,
        };
        e.recompose();
        e
    }

    pub fn set_donor(&mut self, name: String, px: Vec<Px>) {
        self.donor = Some(Donor { name, px });
        self.busy = false;
        self.changed = true;
        self.recompose();
    }

    /// Исходник + части донора, потом правки кистью.
    fn recompose(&mut self) {
        let mut m = self.orig.clone();
        if let Some(d) = &self.donor {
            for (k, (_, base, outer)) in PARTS.iter().enumerate() {
                if !self.take[k] {
                    continue;
                }
                let rects: Vec<&[usize; 4]> = if self.outer_only { outer.iter().collect() } else { base.iter().chain(outer.iter()).collect() };
                for &[x0, y0, w, h] in rects {
                    for y in y0..y0 + h {
                        for x in x0..x0 + w {
                            m[y * 64 + x] = d.px[y * 64 + x];
                        }
                    }
                }
            }
        }
        self.mixed = m;
        self.apply_edits();
    }

    fn apply_edits(&mut self) {
        self.composed = self.mixed.clone();
        for (&i, &c) in &self.edits {
            self.composed[i] = c;
        }
        self.dirty = true;
    }

    fn set_px(&mut self, i: usize, c: Option<Px>) {
        let prev = self.edits.get(&i).copied();
        if prev == c {
            return;
        }
        if let Some(st) = self.stroke.as_mut() {
            st.entry(i).or_insert(prev);
        }
        match c {
            Some(c) => {
                self.edits.insert(i, c);
                self.composed[i] = c;
            }
            None => {
                self.edits.remove(&i);
                self.composed[i] = self.mixed[i];
            }
        }
        self.changed = true;
        self.dirty = true;
    }

    fn begin_stroke(&mut self) {
        if self.stroke.is_none() {
            self.stroke = Some(HashMap::new());
            self.last_texel = None;
        }
    }

    fn end_stroke(&mut self) {
        if let Some(st) = self.stroke.take()
            && !st.is_empty()
        {
            self.undo.push(st);
            self.redo.clear();
            let c = self.color;
            self.recent.retain(|x| *x != c);
            self.recent.insert(0, c);
            self.recent.truncate(10);
        }
        self.last_texel = None;
    }

    fn swap_history(&mut self, back: bool) {
        let Some(rec) = (if back { self.undo.pop() } else { self.redo.pop() }) else { return };
        let mut inverse = HashMap::new();
        for (i, v) in rec {
            inverse.insert(i, self.edits.get(&i).copied());
            match v {
                Some(c) => self.edits.insert(i, c),
                None => self.edits.remove(&i),
            };
        }
        if back { self.redo.push(inverse) } else { self.undo.push(inverse) }
        self.apply_edits();
    }

    /// Применить инструмент к пикселю (x, y) развёртки.
    fn apply_tool(&mut self, x: usize, y: usize) {
        let Some(outer) = layer_of(self.slim, x, y) else { return };
        if outer != (self.layer == 1) {
            return;
        }
        let i = y * 64 + x;
        let c = self.color.to_srgba_unmultiplied();
        match self.tool {
            Tool::Pencil => {
                // линия между соседними точками мазка, чтобы быстрый мазок был без дыр
                let from = self.last_texel.unwrap_or((x, y));
                let n = from.0.abs_diff(x).max(from.1.abs_diff(y)).max(1);
                for k in 0..=n {
                    let t = k as f32 / n as f32;
                    let (px, py) = ((from.0 as f32 + (x as f32 - from.0 as f32) * t).round() as usize, (from.1 as f32 + (y as f32 - from.1 as f32) * t).round() as usize);
                    if layer_of(self.slim, px, py) == Some(outer) {
                        self.set_px(py * 64 + px, Some(c));
                    }
                }
                self.last_texel = Some((x, y));
            }
            Tool::Eraser => {
                // второй слой — прозрачность; основной — вернуть как было (дыр в теле не бывает)
                if outer {
                    self.set_px(i, Some([0, 0, 0, 0]));
                } else {
                    self.set_px(i, None);
                }
            }
            Tool::Picker => {
                let p = self.composed[i];
                if p[3] > 0 {
                    self.color = Color32::from_rgb(p[0], p[1], p[2]);
                    self.tool = Tool::Pencil;
                }
            }
            Tool::Fill => {
                if self.last_texel.is_some() {
                    return;
                }
                self.last_texel = Some((x, y));
                let Some((fx, fy, fw, fh, _)) = face_of(self.slim, x, y) else { return };
                let target = self.composed[i];
                if target == c {
                    return;
                }
                let mut stack = vec![(x, y)];
                let mut seen = vec![false; N];
                while let Some((px, py)) = stack.pop() {
                    let j = py * 64 + px;
                    if seen[j] || self.composed[j] != target {
                        continue;
                    }
                    seen[j] = true;
                    self.set_px(j, Some(c));
                    if px > fx {
                        stack.push((px - 1, py));
                    }
                    if px + 1 < fx + fw {
                        stack.push((px + 1, py));
                    }
                    if py > fy {
                        stack.push((px, py - 1));
                    }
                    if py + 1 < fy + fh {
                        stack.push((px, py + 1));
                    }
                }
            }
        }
    }

    fn texture(&mut self, ctx: &egui::Context) -> &TextureHandle {
        let img = egui::ColorImage::from_rgba_unmultiplied([64, 64], self.composed.as_flattened());
        if self.tex.is_none() {
            self.tex = Some(ctx.load_texture("skin-editor", img, TextureOptions::NEAREST));
        } else if self.dirty {
            self.tex.as_mut().unwrap().set(img, TextureOptions::NEAREST);
        }
        self.dirty = false;
        self.tex.as_ref().unwrap()
    }

    pub fn png(&self) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        let img = image::RgbaImage::from_raw(64, 64, self.composed.as_flattened().to_vec()).expect("64×64");
        let _ = img.write_to(&mut out, image::ImageFormat::Png);
        out.into_inner()
    }

    fn view3d(&mut self, ui: &mut Ui, size: Vec2) {
        let (r, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        ui.painter().rect_filled(r, rr(10), INPUT());
        if resp.dragged_by(PointerButton::Secondary) || resp.dragged_by(PointerButton::Middle) {
            let d = resp.drag_delta();
            self.yaw += d.x * 0.012;
            self.pitch = (self.pitch - d.y * 0.01).clamp(-1.4, 1.4);
        }
        if resp.hovered() {
            let s = ui.input(|i| i.smooth_scroll_delta.y);
            if s != 0.0 {
                self.zoom = (self.zoom * (1.0 + s * 0.002)).clamp(0.6, 3.0);
            }
        }
        let tex = self.texture(ui.ctx()).id();
        let pose = Pose { yaw: self.yaw, pitch: self.pitch, walk: 0.0, idle: 0.0 };
        let fig = Rect::from_center_size(r.center(), r.shrink(20.0).size() * self.zoom);
        let outer = self.layer == 1;
        let quads = skin3d::quads(fig, 64.0, self.slim, pose, true, outer || self.show_overlay);
        skin3d::paint(&ui.painter().with_clip_rect(r), tex, &quads);
        let hover = ui.input(|i| i.pointer.latest_pos()).filter(|p| r.contains(*p));
        // на основном слое видимый второй слой пропускается — рисуем под ним
        let hit = hover.and_then(|p| skin3d::pick(&quads, p, 64.0, |q, _, _| q.overlay == outer));
        if let Some(p) = hover
            && hit.is_some()
        {
            ui.painter().circle_stroke(p, 4.0, Stroke::new(1.5, self.color));
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        let down = ui.input(|i| i.pointer.primary_down());
        if down && (resp.is_pointer_button_down_on() || resp.dragged_by(PointerButton::Primary)) {
            if let Some((x, y)) = hit {
                self.begin_stroke();
                self.apply_tool(x, y);
            }
        }
        text_at(ui.painter(), pos2(r.left() + 10.0, r.bottom() - 12.0), Align2::LEFT_CENTER, "ЛКМ — рисовать · ПКМ — крутить · колесо — ближе", regular(11.5), TEXT_FAINT());
    }

    fn view2d(&mut self, ui: &mut Ui, side: f32) {
        let cell = (side / 64.0).floor().max(3.0);
        let (r, resp) = ui.allocate_exact_size(Vec2::splat(cell * 64.0), Sense::click_and_drag());
        let p = ui.painter().with_clip_rect(r);
        // шахматка под прозрачностью
        let a = Color32::from_gray(46);
        let b = Color32::from_gray(58);
        let k = cell * 4.0;
        for gy in 0..16 {
            for gx in 0..16 {
                p.rect_filled(Rect::from_min_size(r.min + vec2(gx as f32, gy as f32) * k, Vec2::splat(k)), 0, if (gx + gy) % 2 == 0 { a } else { b });
            }
        }
        let tex = self.texture(ui.ctx()).id();
        p.image(tex, r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        // неактивный слой и пустые места — притушить
        let outer = self.layer == 1;
        for (x, y, w, h, o) in faces(self.slim) {
            let fr = Rect::from_min_size(r.min + vec2(x as f32, y as f32) * cell, vec2(w as f32, h as f32) * cell);
            if o != outer {
                p.rect_filled(fr, 0, Color32::from_black_alpha(150));
            }
            p.rect_stroke(fr, 0, Stroke::new(1.0, Color32::from_white_alpha(28)), egui::StrokeKind::Inside);
        }
        let hover = ui.input(|i| i.pointer.latest_pos()).filter(|q| r.contains(*q)).map(|q| {
            let t = (q - r.min) / cell;
            ((t.x as usize).min(63), (t.y as usize).min(63))
        });
        if let Some((x, y)) = hover {
            let hr = Rect::from_min_size(r.min + vec2(x as f32, y as f32) * cell, Vec2::splat(cell));
            p.rect_stroke(hr, 0, Stroke::new(1.5, Color32::WHITE), egui::StrokeKind::Outside);
            if layer_of(self.slim, x, y).is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        }
        if ui.input(|i| i.pointer.primary_down()) && (resp.is_pointer_button_down_on() || resp.dragged_by(PointerButton::Primary))
            && let Some((x, y)) = hover
        {
            self.begin_stroke();
            self.apply_tool(x, y);
        }
    }

    fn tools(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 8.0;
        label(ui, "Инструмент", semibold(13.0), TEXT_DIM());
        let mut t = match self.tool {
            Tool::Pencil => 0,
            Tool::Eraser => 1,
            Tool::Picker => 2,
            Tool::Fill => 3,
        };
        if segmented(ui, Id::new("se-tool"), &["Кисть", "Ластик", "Пипетка", "Заливка"], &mut t) {
            self.tool = [Tool::Pencil, Tool::Eraser, Tool::Picker, Tool::Fill][t];
        }
        label(ui, "Слой", semibold(13.0), TEXT_DIM());
        segmented(ui, Id::new("se-layer"), &["Основной", "Второй (одежда)"], &mut self.layer);
        if self.layer == 0 {
            checkbox(ui, &mut self.show_overlay, "Показывать второй слой");
        }
        label(ui, "Цвет", semibold(13.0), TEXT_DIM());
        ui.horizontal(|ui| {
            egui::color_picker::color_edit_button_srgba(ui, &mut self.color, egui::color_picker::Alpha::Opaque);
            let mut hex = format!("#{:02x}{:02x}{:02x}", self.color.r(), self.color.g(), self.color.b());
            if ui.add(egui::TextEdit::singleline(&mut hex).desired_width(80.0).font(mono(12.5))).changed()
                && let Ok(v) = u32::from_str_radix(hex.trim().trim_start_matches('#'), 16)
                && hex.trim().trim_start_matches('#').len() == 6
            {
                self.color = Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
            }
        });
        let swatch = |ui: &mut Ui, c: Color32, cur: Color32| -> bool {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
            ui.painter().rect_filled(r, rr(4), c);
            if c == cur {
                ui.painter().rect_stroke(r.expand(1.5), rr(5), Stroke::new(2.0, TEXT()), egui::StrokeKind::Outside);
            }
            resp.clicked()
        };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
            for v in PALETTE {
                let c = hex(v);
                if swatch(ui, c, self.color) {
                    self.color = c;
                    if self.tool == Tool::Eraser || self.tool == Tool::Picker {
                        self.tool = Tool::Pencil;
                    }
                }
            }
        });
        if !self.recent.is_empty() {
            label(ui, "Недавние", regular(12.0), TEXT_FAINT());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                for c in self.recent.clone() {
                    if swatch(ui, c, self.color) {
                        self.color = c;
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add_enabled_ui(!self.undo.is_empty(), |ui| {
                if button(ui, "Отменить", None, Kind::Secondary, 30.0).on_hover_text("Ctrl+Z").clicked() {
                    self.swap_history(true);
                }
            });
            ui.add_enabled_ui(!self.redo.is_empty(), |ui| {
                if button(ui, "Повторить", None, Kind::Secondary, 30.0).on_hover_text("Ctrl+Y").clicked() {
                    self.swap_history(false);
                }
            });
        });
        let mut v = usize::from(self.slim);
        ui.horizontal(|ui| {
            label(ui, "Модель", regular(13.0), TEXT_DIM());
            if segmented(ui, Id::new("se-model"), &["Classic", "Slim"], &mut v) {
                self.slim = v == 1;
                self.changed = true;
            }
        });
    }
}

impl App {
    pub fn open_skin_editor(&mut self, skin: Option<&SkinItem>) {
        self.skin_edit = Some(SkinEditor::new(skin));
    }

    /// Окно редактора; вызывается из общих диалогов.
    pub fn skin_editor_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut ed) = self.skin_edit.take() else { return };
        let screen = ctx.content_rect();
        let frame = egui::Frame::new().fill(PANEL()).corner_radius(rr(14)).stroke(Stroke::new(1.0, BORDER())).inner_margin(Margin::same(18));
        let (w, h) = ((screen.width() - 60.0).clamp(760.0, 1240.0), (screen.height() - 80.0).clamp(520.0, 760.0));
        let mut close = false;
        let mut save: Option<bool> = None;
        // горячие клавиши
        let (undo, redo) = ctx.input(|i| (i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::Z), i.modifiers.command && (i.key_pressed(Key::Y) || (i.modifiers.shift && i.key_pressed(Key::Z)))));
        if undo {
            ed.swap_history(true);
        }
        if redo {
            ed.swap_history(false);
        }
        let skins = self.skins.clone();
        let r = egui::Modal::new(Id::new("skin-editor")).frame(frame).show(ctx, |ui| {
            ui.set_width(w);
            ui.set_height(h);
            ui.horizontal(|ui| {
                label(ui, "Редактор скина", bold(20.0), TEXT());
                ui.add(egui::TextEdit::singleline(&mut ed.name).desired_width(220.0).hint_text("Название"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, Icon::Close, 30.0, "Закрыть").clicked() {
                        close = true;
                    }
                    if ed.src.is_some() && button(ui, "Сохранить", Some(Icon::Check), Kind::Primary, 34.0).on_hover_text("Заменить скин в библиотеке").clicked() {
                        save = Some(true);
                    }
                    if button(ui, if ed.src.is_some() { "Сохранить копией" } else { "Сохранить" }, Some(Icon::Plus), if ed.src.is_some() { Kind::Secondary } else { Kind::Primary }, 34.0).clicked() {
                        save = Some(false);
                    }
                });
            });
            ui.add_space(10.0);
            let body_h = h - 56.0;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                let side_w = 270.0;
                let view_w = ((w - side_w - 32.0) * 0.38).clamp(220.0, 380.0);
                let canvas = (w - side_w - view_w - 32.0).min(body_h).max(192.0);
                ui.vertical(|ui| {
                    ui.set_width(view_w);
                    ed.view3d(ui, vec2(view_w, body_h));
                });
                ui.vertical(|ui| {
                    ui.set_width(canvas);
                    ed.view2d(ui, canvas);
                });
                ui.vertical(|ui| {
                    ui.set_width(side_w);
                    egui::ScrollArea::vertical().id_salt("se-side").max_height(body_h).show(ui, |ui| {
                        ed.tools(ui);
                        ui.add_space(10.0);
                        ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, BORDER()));
                        ui.add_space(10.0);
                        self.outfit_panel(ui, &mut ed, &skins);
                    });
                });
            });
        });
        if !ctx.input(|i| i.pointer.primary_down()) {
            ed.end_stroke();
        }
        if let Some(replace) = save {
            let png = ed.png();
            let name = if ed.name.trim().is_empty() { "Скин".to_string() } else { ed.name.trim().to_string() };
            self.be.save_edited_skin(if replace { ed.src.clone() } else { None }, png, name, ed.slim, ed.favorite);
            close = true;
        }
        if (r.should_close() && !ed.changed) || close {
            return;
        }
        self.skin_edit = Some(ed);
    }

    /// «Одежда с другого скина», как в гардеробе sfatm.
    fn outfit_panel(&mut self, ui: &mut Ui, ed: &mut SkinEditor, skins: &[SkinItem]) {
        ui.spacing_mut().item_spacing.y = 8.0;
        label(ui, "Одежда с другого скина", semibold(14.0), TEXT());
        ui.add(egui::Label::new(egui::RichText::new("Возьмите части у другого скина: например тело, руки и ноги (одежду), а голову оставьте свою.").font(regular(12.0)).color(TEXT_FAINT())).wrap());
        match ed.donor.as_ref().map(|d| d.name.clone()) {
            Some(dname) => {
                ui.horizontal(|ui| {
                    label(ui, format!("Донор: {dname}"), medium(13.0), TEXT());
                    if ui.small_button("✕").on_hover_text("Убрать").clicked() {
                        ed.donor = None;
                        ed.recompose();
                    }
                });
                let mut changed = false;
                ui.horizontal_wrapped(|ui| {
                    for (k, (name, _, _)) in PARTS.iter().enumerate() {
                        changed |= checkbox(ui, &mut ed.take[k], name).changed();
                    }
                });
                changed |= checkbox(ui, &mut ed.outer_only, "Только второй слой (поверх своего)").changed();
                if changed {
                    ed.changed = true;
                    ed.recompose();
                }
            }
            None => {
                label(ui, "Донор не выбран", regular(12.5), TEXT_DIM());
            }
        }
        ui.add_space(4.0);
        label(ui, "Из библиотеки", regular(12.0), TEXT_FAINT());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for s in skins {
                if Some(&s.id) == ed.src.as_ref() {
                    continue;
                }
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::click());
                ui.painter().rect_filled(r, rr(6), INPUT());
                if let Some(t) = self.skin_tex.get(&s.id) {
                    skin3d::draw_face(ui.painter(), r.shrink(5.0), t.tex.id(), t.h);
                }
                if resp.on_hover_text(&s.name).clicked()
                    && let Some(px) = s.rgba.as_ref().and_then(|(w, h, d)| to_64(*w, *h, d))
                {
                    ed.set_donor(s.name.clone(), px);
                }
            }
        });
        ui.horizontal(|ui| {
            if button(ui, "Файл…", Some(Icon::Folder), Kind::Secondary, 30.0).clicked() {
                self.be.pick(Pick::DonorSkin);
            }
        });
        segmented(ui, Id::new("se-nick-src"), &["Minecraft", "Ely.by"], &mut ed.nick_src);
        ui.horizontal(|ui| {
            let resp = ui.add(egui::TextEdit::singleline(&mut ed.nick).hint_text("Ник игрока").desired_width(150.0));
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            if (button(ui, if ed.busy { "…" } else { "Взять" }, None, Kind::Secondary, 30.0).clicked() || enter) && !ed.nick.trim().is_empty() && !ed.busy {
                ed.busy = true;
                self.be.fetch_donor(ed.nick.trim().to_string(), ed.nick_src == 1);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_cover_skin_layout_without_overlap() {
        let mut seen = vec![0u8; N];
        for (x, y, w, h, _) in faces(false) {
            for yy in y..y + h {
                for xx in x..x + w {
                    seen[yy * 64 + xx] += 1;
                }
            }
        }
        assert!(seen.iter().all(|&n| n <= 1));
        // лицо — основной слой, лицо шляпы — второй
        assert_eq!(layer_of(false, 10, 10), Some(false));
        assert_eq!(layer_of(false, 42, 10), Some(true));
        assert_eq!(layer_of(false, 60, 0), None);
    }

    #[test]
    fn edits_undo_and_redo() {
        let mut e = SkinEditor::new(None);
        e.layer = 0;
        e.color = Color32::from_rgb(1, 2, 3);
        e.begin_stroke();
        e.apply_tool(10, 10);
        e.end_stroke();
        assert_eq!(e.composed[10 * 64 + 10], [1, 2, 3, 255]);
        e.swap_history(true);
        assert_eq!(e.composed[10 * 64 + 10], template()[10 * 64 + 10]);
        e.swap_history(false);
        assert_eq!(e.composed[10 * 64 + 10], [1, 2, 3, 255]);
    }

    #[test]
    fn donor_outfit_keeps_own_head() {
        let mut e = SkinEditor::new(None);
        e.set_donor("d".into(), vec![[9, 9, 9, 255]; N]);
        assert_eq!(e.composed[10 * 64 + 10], template()[10 * 64 + 10], "голова своя");
        assert_eq!(e.composed[20 * 64 + 20], [9, 9, 9, 255], "тело с донора");
    }

    #[test]
    fn fill_stays_inside_one_face() {
        let mut e = SkinEditor::new(None);
        e.tool = Tool::Fill;
        e.color = Color32::from_rgb(200, 0, 0);
        e.begin_stroke();
        e.apply_tool(9, 15); // кожа лица (8..16 × 8..16)
        e.end_stroke();
        assert_eq!(e.composed[15 * 64 + 15], [200, 0, 0, 255]);
        assert_ne!(e.composed[7 * 64 + 8], [200, 0, 0, 255], "верх головы не залит");
        assert_ne!(e.composed[12 * 64 + 9], [200, 0, 0, 255], "глаз не залит");
    }
}
