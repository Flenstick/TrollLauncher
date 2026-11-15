//! Свои виджеты: кнопки, вкладки, переключатели, иконки, пиксельные картинки.
//! Всё рисуется painter'ом, чтобы не было «стандартного egui».

use crate::theme::*;
use eframe::egui::{
    self, Align, Align2, Color32, Context, CornerRadius, FontId, Frame, Id, Margin, Painter, Pos2, Rect, Response,
    Sense, Shape, Stroke, StrokeKind, TextEdit, Ui, Vec2, pos2, text::LayoutJob, vec2,
};

// ───────────────────────── анимации ─────────────────────────

/// Плавный переход к `target` с ease-out (кубическая), перезапускается при смене цели.
pub fn ease_to(ctx: &Context, id: Id, target: f32, dur: f32) -> f32 {
    let now = ctx.input(|i| i.time);
    let st: Option<(f32, f32, f64)> = ctx.data(|d| d.get_temp(id));
    let value_at = |(from, to, start): (f32, f32, f64)| {
        let p = (((now - start) / dur as f64).clamp(0.0, 1.0)) as f32;
        let e = 1.0 - (1.0 - p).powi(3);
        (from + (to - from) * e, p < 1.0)
    };
    match st {
        None => {
            ctx.data_mut(|d| d.insert_temp(id, (target, target, now)));
            target
        }
        Some(s) => {
            let (cur, running) = value_at(s);
            if (s.1 - target).abs() > 0.001 {
                ctx.data_mut(|d| d.insert_temp(id, (cur, target, now)));
                ctx.request_repaint();
                return cur;
            }
            if running {
                ctx.request_repaint();
            }
            cur
        }
    }
}

pub fn hover_anim(ui: &Ui, id: Id, on: bool) -> f32 {
    ui.ctx().animate_bool_with_time(id.with("hov"), on, 0.10)
}

pub fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

// ───────────────────────── текст ─────────────────────────

pub fn text_at(p: &Painter, pos: Pos2, anchor: Align2, s: impl ToString, font: FontId, c: Color32) -> Rect {
    p.text(pos, anchor, s.to_string(), font, c)
}

/// Текст с переносом и обрезкой «…» после `rows` строк.
pub fn text_wrap(p: &Painter, pos: Pos2, s: &str, font: FontId, c: Color32, width: f32, rows: usize) -> Rect {
    let mut job = LayoutJob::simple(s.to_owned(), font, c, width.max(10.0));
    job.wrap.max_rows = rows;
    job.wrap.break_anywhere = rows == 1;
    let g = p.layout_job(job);
    let r = Rect::from_min_size(pos, g.size());
    p.galley(pos, g, c);
    r
}

pub fn label(ui: &mut Ui, s: impl Into<String>, font: FontId, c: Color32) -> Response {
    ui.label(egui::RichText::new(s.into()).font(font).color(c))
}

pub fn heading(ui: &mut Ui, title: &str, caption: Option<&str>) {
    label(ui, title, bold(26.0), TEXT());
    if let Some(c) = caption {
        ui.add_space(-4.0);
        label(ui, c, regular(14.0), TEXT_DIM());
    }
}

pub fn section_title(ui: &mut Ui, s: &str) {
    label(ui, s, semibold(16.0), TEXT());
}

// ───────────────────────── иконки ─────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Home,
    Library,
    Browse,
    Skins,
    Plus,
    Settings,
    Play,
    Stop,
    Search,
    Download,
    Close,
    Min,
    Max,
    Back,
    Forward,
    Folder,
    Check,
    User,
    Clock,
    Cube,
    More,
    Server,
    Heart,
    Star,
    Refresh,
    Copy,
    Share,
    Globe,
    Bell,
    Image,
}

pub fn icon(p: &Painter, ic: Icon, r: Rect, c: Color32) {
    let k = r.width() / 24.0;
    let pt = |x: f32, y: f32| r.min + vec2(x, y) * k;
    let s = Stroke::new(1.7 * k.max(0.75), c);
    let line = |pts: &[(f32, f32)]| {
        p.add(Shape::line(pts.iter().map(|&(x, y)| pt(x, y)).collect(), s));
    };
    let closed = |pts: &[(f32, f32)]| {
        p.add(Shape::closed_line(pts.iter().map(|&(x, y)| pt(x, y)).collect(), s));
    };
    let fill = |pts: &[(f32, f32)]| {
        p.add(Shape::convex_polygon(pts.iter().map(|&(x, y)| pt(x, y)).collect(), c, Stroke::NONE));
    };
    let rrect = |x0: f32, y0: f32, x1: f32, y1: f32, rad: f32| {
        p.rect_stroke(Rect::from_min_max(pt(x0, y0), pt(x1, y1)), rad * k, s, StrokeKind::Middle);
    };
    match ic {
        Icon::Home => {
            line(&[(4.5, 11.0), (12.0, 4.5), (19.5, 11.0)]);
            line(&[(6.5, 9.5), (6.5, 19.5), (17.5, 19.5), (17.5, 9.5)]);
            line(&[(10.3, 19.5), (10.3, 14.5), (13.7, 14.5), (13.7, 19.5)]);
        }
        Icon::Library => {
            rrect(4.5, 4.5, 10.8, 10.8, 2.0);
            rrect(13.2, 4.5, 19.5, 10.8, 2.0);
            rrect(4.5, 13.2, 10.8, 19.5, 2.0);
            rrect(13.2, 13.2, 19.5, 19.5, 2.0);
        }
        Icon::Browse => {
            p.circle_stroke(pt(12.0, 12.0), 8.3 * k, s);
            fill(&[(15.6, 8.4), (13.3, 13.3), (8.4, 15.6), (10.7, 10.7)]);
        }
        Icon::Skins => closed(&[
            (8.5, 4.5),
            (4.0, 7.5),
            (5.8, 11.3),
            (8.0, 10.3),
            (8.0, 19.5),
            (16.0, 19.5),
            (16.0, 10.3),
            (18.2, 11.3),
            (20.0, 7.5),
            (15.5, 4.5),
            (14.0, 6.0),
            (10.0, 6.0),
        ]),
        Icon::Plus => {
            line(&[(12.0, 5.0), (12.0, 19.0)]);
            line(&[(5.0, 12.0), (19.0, 12.0)]);
        }
        Icon::Settings => {
            // шестерёнка: 8 зубцов
            let mut pts = vec![];
            let n = 8;
            for i in 0..n {
                let a0 = std::f32::consts::TAU * i as f32 / n as f32;
                let step = std::f32::consts::TAU / n as f32;
                for (da, rad) in [(-0.30, 6.4), (-0.17, 8.6), (0.17, 8.6), (0.30, 6.4)] {
                    let a = a0 + da * step * 1.4;
                    pts.push((12.0 + a.cos() * rad, 12.0 + a.sin() * rad));
                }
            }
            closed(&pts);
            p.circle_stroke(pt(12.0, 12.0), 2.8 * k, s);
        }
        Icon::Play => fill(&[(8.0, 5.0), (19.0, 12.0), (8.0, 19.0)]),
        Icon::Stop => {
            p.rect_filled(Rect::from_min_max(pt(6.5, 6.5), pt(17.5, 17.5)), 2.0 * k, c);
        }
        Icon::Search => {
            p.circle_stroke(pt(10.5, 10.5), 6.0 * k, s);
            line(&[(15.0, 15.0), (19.5, 19.5)]);
        }
        Icon::Download => {
            line(&[(12.0, 4.5), (12.0, 15.0)]);
            line(&[(7.5, 10.5), (12.0, 15.0), (16.5, 10.5)]);
            line(&[(5.0, 19.5), (19.0, 19.5)]);
        }
        Icon::Close => {
            line(&[(6.5, 6.5), (17.5, 17.5)]);
            line(&[(17.5, 6.5), (6.5, 17.5)]);
        }
        Icon::Min => line(&[(6.5, 12.0), (17.5, 12.0)]),
        Icon::Max => rrect(6.5, 6.5, 17.5, 17.5, 1.5),
        Icon::Back => line(&[(14.5, 6.0), (8.5, 12.0), (14.5, 18.0)]),
        Icon::Forward => line(&[(9.5, 6.0), (15.5, 12.0), (9.5, 18.0)]),
        Icon::Folder => closed(&[(4.0, 6.5), (10.0, 6.5), (12.0, 8.8), (20.0, 8.8), (20.0, 18.5), (4.0, 18.5)]),
        Icon::Check => line(&[(5.5, 12.5), (10.0, 17.0), (18.5, 7.5)]),
        Icon::User => {
            p.circle_stroke(pt(12.0, 8.5), 3.6 * k, s);
            line(&[(5.0, 19.5), (6.2, 15.6), (9.3, 13.6), (14.7, 13.6), (17.8, 15.6), (19.0, 19.5)]);
        }
        Icon::Clock => {
            p.circle_stroke(pt(12.0, 12.0), 8.0 * k, s);
            line(&[(12.0, 7.5), (12.0, 12.0), (15.3, 14.0)]);
        }
        Icon::Cube => {
            closed(&[(12.0, 3.8), (19.5, 8.0), (19.5, 16.0), (12.0, 20.2), (4.5, 16.0), (4.5, 8.0)]);
            line(&[(4.5, 8.0), (12.0, 12.2), (19.5, 8.0)]);
            line(&[(12.0, 12.2), (12.0, 20.2)]);
        }
        Icon::More => {
            for x in [6.0, 12.0, 18.0] {
                p.circle_filled(pt(x, 12.0), 1.7 * k, c);
            }
        }
        Icon::Server => {
            rrect(4.5, 4.5, 19.5, 11.0, 2.0);
            rrect(4.5, 13.0, 19.5, 19.5, 2.0);
            p.circle_filled(pt(8.0, 7.8), 1.2 * k, c);
            p.circle_filled(pt(8.0, 16.2), 1.2 * k, c);
        }
        Icon::Heart => closed(&[
            (12.0, 19.0),
            (5.0, 12.5),
            (4.2, 9.0),
            (5.8, 6.2),
            (8.8, 5.4),
            (12.0, 7.8),
            (15.2, 5.4),
            (18.2, 6.2),
            (19.8, 9.0),
            (19.0, 12.5),
        ]),
        Icon::Star => {
            let mut pts = vec![];
            for i in 0..10 {
                let a = std::f32::consts::TAU * i as f32 / 10.0 - std::f32::consts::FRAC_PI_2;
                let rad = if i % 2 == 0 { 8.5 } else { 3.8 };
                pts.push((12.0 + a.cos() * rad, 12.6 + a.sin() * rad));
            }
            closed(&pts);
        }
        Icon::Refresh => {
            let mut pts = vec![];
            for i in 0..=20 {
                let a = 0.6 + i as f32 / 20.0 * 4.9;
                pts.push((12.0 + a.cos() * 7.0, 12.0 + a.sin() * 7.0));
            }
            line(&pts);
            line(&[(13.5, 4.0), (17.0, 5.5), (15.5, 9.0)]);
        }
        Icon::Copy => {
            rrect(8.5, 8.5, 19.0, 19.0, 2.0);
            line(&[(15.5, 5.0), (5.0, 5.0), (5.0, 15.5)]);
        }
        Icon::Share => {
            line(&[(12.0, 4.5), (12.0, 15.0)]);
            line(&[(7.5, 9.0), (12.0, 4.5), (16.5, 9.0)]);
            line(&[(5.0, 13.0), (5.0, 19.5), (19.0, 19.5), (19.0, 13.0)]);
        }
        Icon::Globe => {
            p.circle_stroke(pt(12.0, 12.0), 8.0 * k, s);
            line(&[(4.0, 12.0), (20.0, 12.0)]);
            p.add(Shape::ellipse_stroke(pt(12.0, 12.0), vec2(3.5, 8.0) * k, s));
        }
        Icon::Bell => {
            closed(&[(6.0, 16.5), (7.0, 10.0), (9.0, 6.5), (12.0, 5.5), (15.0, 6.5), (17.0, 10.0), (18.0, 16.5)]);
            line(&[(10.0, 19.5), (14.0, 19.5)]);
        }
        Icon::Image => {
            rrect(4.0, 5.0, 20.0, 19.0, 2.0);
            line(&[(4.5, 17.0), (9.5, 11.5), (13.0, 15.0), (15.5, 12.5), (19.5, 16.5)]);
            p.circle_filled(pt(15.5, 9.0), 1.5 * k, c);
        }
    }
}

// ───────────────────────── кнопки ─────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub fn button(ui: &mut Ui, label: &str, ic: Option<Icon>, kind: Kind, height: f32) -> Response {
    let font = semibold(if height >= 38.0 { 14.5 } else { 13.5 });
    let g = ui.painter().layout_no_wrap(label.to_owned(), font, Color32::PLACEHOLDER);
    let isz = if height >= 38.0 { 18.0 } else { 16.0 };
    let icon_w = if ic.is_some() { isz + if label.is_empty() { 0.0 } else { 7.0 } } else { 0.0 };
    let pad = if label.is_empty() { (height - isz) / 2.0 } else { 14.0 };
    let w = g.size().x + icon_w + pad * 2.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click());
    paint_button(ui, &resp, rect, g, ic, isz, icon_w, kind);
    resp
}

fn paint_button(
    ui: &Ui,
    resp: &Response,
    rect: Rect,
    g: std::sync::Arc<egui::Galley>,
    ic: Option<Icon>,
    isz: f32,
    icon_w: f32,
    kind: Kind,
) {
    let t = hover_anim(ui, resp.id, resp.hovered());
    let press = ui.ctx().animate_bool_with_time(resp.id.with("press"), resp.is_pointer_button_down_on(), 0.06);
    let (bg, fg) = match kind {
        Kind::Primary => (accent().lerp_to_gamma(lighten(accent(), 0.14), t), on_accent()),
        Kind::Secondary => (RAISED().lerp_to_gamma(HOVER(), t), TEXT()),
        Kind::Ghost => (Color32::TRANSPARENT.lerp_to_gamma(HOVER(), t), TEXT_DIM().lerp_to_gamma(TEXT(), t)),
        Kind::Danger => (DANGER().lerp_to_gamma(lighten(DANGER(), 0.14), t), Color32::WHITE),
    };
    let r = rect.shrink(press * 1.0);
    let rad = (rect.height() * 0.25).min(10.0);
    ui.painter().rect_filled(r, rad, bg);
    let content_w = g.size().x + icon_w;
    let mut x = rect.center().x - content_w / 2.0;
    if let Some(ic) = ic {
        icon(ui.painter(), ic, Rect::from_min_size(pos2(x, rect.center().y - isz / 2.0), Vec2::splat(isz)), fg);
        x += icon_w;
    }
    ui.painter().galley(pos2(x, rect.center().y - g.size().y / 2.0), g, fg);
}

/// Квадратная кнопка-иконка (прозрачная, подсветка при наведении).
pub fn icon_button(ui: &mut Ui, ic: Icon, size: f32, tip: &str) -> Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let id = ui.auto_id_with("icon-btn");
    icon_button_at(ui, rect, id, ic, tip)
}

pub fn icon_button_at(ui: &mut Ui, rect: Rect, id: Id, ic: Icon, tip: &str) -> Response {
    let size = rect.width();
    let resp = ui.interact(rect, id, Sense::click());
    let t = hover_anim(ui, resp.id, resp.hovered());
    ui.painter().rect_filled(rect, 8, Color32::TRANSPARENT.lerp_to_gamma(HOVER(), t));
    let isz = (size * 0.55).round();
    icon(ui.painter(), ic, Rect::from_center_size(rect.center(), Vec2::splat(isz)), TEXT_DIM().lerp_to_gamma(TEXT(), t));
    if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
}

// ───────────────────────── переключатели ─────────────────────────

pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let (r, _) = ui.allocate_exact_size(vec2(38.0, 22.0), Sense::hover());
    let id = ui.auto_id_with("toggle");
    toggle_at(ui, r, id, on)
}

/// Переключатель в заданном месте (для ручной раскладки строк таблицы).
pub fn toggle_at(ui: &mut Ui, r: Rect, id: Id, on: &mut bool) -> Response {
    let mut resp = ui.interact(r, id, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = smooth(ui.ctx().animate_bool_with_time(resp.id, *on, 0.16));
    let h = hover_anim(ui, resp.id, resp.hovered());
    let off_bg = HOVER().lerp_to_gamma(lighten(HOVER(), 0.06), h);
    ui.painter().rect_filled(r, 11, off_bg.lerp_to_gamma(accent(), t));
    let x = egui::lerp(r.left() + 11.0..=r.right() - 11.0, t);
    ui.painter().circle_filled(pos2(x, r.center().y), 8.0, Color32::WHITE.gamma_multiply(0.75 + 0.25 * t.max(h)));
    resp
}

pub fn checkbox(ui: &mut Ui, on: &mut bool, text: &str) -> Response {
    let g = ui.painter().layout_no_wrap(text.to_owned(), regular(13.5), Color32::PLACEHOLDER);
    let (r, mut resp) = ui.allocate_exact_size(vec2(18.0 + 10.0 + g.size().x, 26.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = smooth(ui.ctx().animate_bool_with_time(resp.id, *on, 0.12));
    let h = hover_anim(ui, resp.id, resp.hovered());
    let b = Rect::from_min_size(pos2(r.left(), r.center().y - 9.0), Vec2::splat(18.0));
    ui.painter().rect(
        b,
        5,
        RAISED().lerp_to_gamma(HOVER(), h).lerp_to_gamma(accent(), t),
        Stroke::new(1.0, BORDER().lerp_to_gamma(accent(), t)),
        StrokeKind::Inside,
    );
    if t > 0.05 {
        icon(ui.painter(), Icon::Check, b.shrink(2.0), on_accent().gamma_multiply(t));
    }
    let fg = TEXT_DIM().lerp_to_gamma(TEXT(), h.max(t));
    ui.painter().galley(pos2(b.right() + 10.0, r.center().y - g.size().y / 2.0), g, fg);
    resp
}

/// Вкладки с ползущим подчёркиванием.
pub fn tabs(ui: &mut Ui, id: Id, labels: &[String], sel: &mut usize) -> bool {
    let mut changed = false;
    let mut rects = vec![];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, l) in labels.iter().enumerate() {
            let g = ui.painter().layout_no_wrap(l.clone(), semibold(14.0), Color32::PLACEHOLDER);
            let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 22.0, 36.0), Sense::click());
            let t = hover_anim(ui, resp.id, resp.hovered());
            let active = i == *sel;
            if !active && t > 0.0 {
                ui.painter().rect_filled(r.shrink2(vec2(0.0, 4.0)), 7, HOVER().gamma_multiply(t * 0.7));
            }
            let col = if active { TEXT() } else { TEXT_DIM().lerp_to_gamma(TEXT(), t * 0.7) };
            ui.painter().galley(r.center() - g.size() / 2.0 - vec2(0.0, 1.0), g, col);
            if resp.clicked() && !active {
                *sel = i;
                changed = true;
            }
            rects.push(r);
        }
    });
    if let Some(target) = rects.get(*sel) {
        let x0 = ease_to(ui.ctx(), id.with("x0"), target.left() + 11.0, 0.22);
        let x1 = ease_to(ui.ctx(), id.with("x1"), target.right() - 11.0, 0.22);
        let y = target.bottom();
        let full = Rect::from_min_max(pos2(rects[0].left(), y), pos2(rects.last().unwrap().right(), y + 1.0));
        let _ = full;
        ui.painter().rect_filled(Rect::from_min_max(pos2(x0, y - 2.5), pos2(x1, y)), 2, accent());
    }
    changed
}

/// Сегментированный переключатель с ползущей «таблеткой».
pub fn segmented(ui: &mut Ui, id: Id, labels: &[&str], sel: &mut usize) -> bool {
    let font = medium(13.0);
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| ui.painter().layout_no_wrap(l.to_string(), font.clone(), TEXT()).size().x + 24.0)
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + 6.0;
    let (r, _) = ui.allocate_exact_size(vec2(total, 32.0), Sense::hover());
    ui.painter().rect_filled(r, 9, INPUT());
    ui.painter().rect_stroke(r, 9, Stroke::new(1.0, BORDER()), StrokeKind::Inside);
    let mut x = r.left() + 3.0;
    let mut changed = false;
    let mut seg_rects = vec![];
    for w in widths.iter() {
        let sr = Rect::from_min_size(pos2(x, r.top() + 3.0), vec2(*w, r.height() - 6.0));
        seg_rects.push(sr);
        x += w;
    }
    let target = seg_rects[*sel];
    let px = ease_to(ui.ctx(), id.with("px"), target.left(), 0.2);
    let pw = ease_to(ui.ctx(), id.with("pw"), target.width(), 0.2);
    ui.painter().rect_filled(Rect::from_min_size(pos2(px, target.top()), vec2(pw, target.height())), 7, HOVER());
    for (i, sr) in seg_rects.iter().enumerate() {
        let resp = ui.interact(*sr, id.with(i), Sense::click());
        let t = hover_anim(ui, resp.id, resp.hovered());
        let col = if i == *sel { TEXT() } else { TEXT_DIM().lerp_to_gamma(TEXT(), t * 0.7) };
        text_at(ui.painter(), sr.center(), Align2::CENTER_CENTER, labels[i], font.clone(), col);
        if resp.clicked() && i != *sel {
            *sel = i;
            changed = true;
        }
    }
    changed
}

pub fn search_field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let (r, _) = ui.allocate_exact_size(vec2(width, 34.0), Sense::hover());
    ui.painter().rect_filled(r, 9, INPUT());
    icon(ui.painter(), Icon::Search, Rect::from_center_size(pos2(r.left() + 18.0, r.center().y), Vec2::splat(16.0)), TEXT_FAINT());
    let te = Rect::from_min_max(pos2(r.left() + 34.0, r.top()), pos2(r.right() - 10.0, r.bottom()));
    let resp = ui.put(
        te,
        TextEdit::singleline(text)
            .frame(Frame::NONE)
            .hint_text(egui::RichText::new(hint).color(TEXT_FAINT()))
            .font(regular(14.0))
            .text_color(TEXT())
            .vertical_align(Align::Center)
            .margin(Margin::ZERO)
            .desired_width(te.width()),
    );
    let f = ui.ctx().animate_bool_with_time(resp.id.with("focus"), resp.has_focus(), 0.12);
    ui.painter().rect_stroke(r, 9, Stroke::new(1.0, BORDER().lerp_to_gamma(accent(), f)), StrokeKind::Inside);
    resp
}

pub fn chip(ui: &mut Ui, text: &str, bg: Color32, fg: Color32) -> Response {
    let g = ui.painter().layout_no_wrap(text.to_owned(), medium(12.0), fg);
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 16.0, 22.0), Sense::hover());
    ui.painter().rect_filled(r, 6, bg);
    ui.painter().galley(r.center() - g.size() / 2.0, g, fg);
    resp
}

pub fn progress_bar(p: &Painter, r: Rect, t: f32) {
    p.rect_filled(r, CornerRadius::same((r.height() / 2.0) as u8), HOVER());
    let w = (r.width() * t.clamp(0.0, 1.0)).max(r.height());
    p.rect_filled(Rect::from_min_size(r.min, vec2(w, r.height())), CornerRadius::same((r.height() / 2.0) as u8), accent());
}

pub fn card_frame() -> Frame {
    Frame::new()
        .fill(PANEL())
        .corner_radius(12)
        .stroke(Stroke::new(1.0, BORDER()))
        .inner_margin(Margin::same(18))
}

/// Строка настройки: заголовок + описание слева, контрол справа, разделитель снизу.
pub fn setting_row(ui: &mut Ui, title: &str, desc: &str, add: impl FnOnce(&mut Ui)) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let w = ui.available_width();
        ui.allocate_ui_with_layout(vec2(w * 0.55, 0.0), egui::Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            label(ui, title, semibold(14.5), TEXT());
            if !desc.is_empty() {
                ui.add(egui::Label::new(egui::RichText::new(desc).font(regular(13.0)).color(TEXT_DIM())).wrap());
            }
        });
        ui.with_layout(egui::Layout::right_to_left(Align::Center), add);
    });
    ui.add_space(6.0);
    let y = ui.cursor().top();
    let x = ui.max_rect().x_range();
    ui.painter().hline(x, y, Stroke::new(1.0, BORDER()));
    ui.add_space(6.0);
}

// ───────────────────────── пиксельная графика ─────────────────────────

pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xD1B54A32D192ED03)
    }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn f(&mut self) -> f32 {
        (self.next() % 10_000) as f32 / 10_000.0
    }
}

pub fn hsv(h: f32, s: f32, v: f32) -> Color32 {
    egui::ecolor::Hsva::new(h.rem_euclid(1.0), s.clamp(0.0, 1.0), v.clamp(0.0, 1.0), 1.0).into()
}

/// Иконка сборки: симметричный пиксельный узор 8×8.
pub fn pixel_icon(p: &Painter, r: Rect, seed: u64, hue: f32, rounding: f32) {
    p.rect_filled(r, rounding, hsv(hue, 0.42, 0.20));
    let inner = r.shrink(r.width() * 0.16);
    let cell = inner.width() / 8.0;
    let mut rng = Rng::new(seed);
    for y in 0..8 {
        for x in 0..4 {
            let v = rng.next() % 100;
            if v < 58 {
                let c = if v < 18 {
                    hsv(hue, 0.50, 0.96)
                } else if v < 40 {
                    hsv(hue, 0.62, 0.74)
                } else {
                    hsv(hue + 0.07, 0.55, 0.52)
                };
                for xx in [x, 7 - x] {
                    let cr = Rect::from_min_size(inner.min + vec2(xx as f32 * cell, y as f32 * cell), Vec2::splat(cell));
                    p.rect_filled(cr.expand(0.3), 0, c);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct SkinLook {
    pub skin: u32,
    pub hair: u32,
    pub shirt: u32,
    pub pants: u32,
    pub shoes: u32,
    pub eyes: u32,
}

fn noisy(c: Color32, rng: &mut Rng, amt: f32) -> Color32 {
    let n = rng.f() * 2.0 - 1.0;
    if n > 0.0 { lighten(c, n * amt) } else { darken(c, -n * amt) }
}

/// Голова 8×8 (аватар аккаунта / друзей).
pub fn head(p: &Painter, r: Rect, look: &SkinLook, seed: u64) {
    let c = r.width() / 8.0;
    let mut rng = Rng::new(seed);
    for y in 0..8 {
        for x in 0..8 {
            let base = if y < 2 || (y == 2 && (x == 0 || x == 7)) {
                hex(look.hair)
            } else if y == 4 && (x == 1 || x == 6) {
                Color32::WHITE
            } else if y == 4 && (x == 2 || x == 5) {
                hex(look.eyes)
            } else if y == 6 && (3..=4).contains(&x) {
                darken(hex(look.skin), 0.35)
            } else {
                hex(look.skin)
            };
            let cr = Rect::from_min_size(r.min + vec2(x as f32 * c, y as f32 * c), Vec2::splat(c));
            p.rect_filled(cr.expand(0.25), 0, noisy(base, &mut rng, 0.07));
        }
    }
}

/// Фигурка скина спереди: 16×32 пикселя.
pub fn skin_figure(p: &Painter, r: Rect, look: &SkinLook, seed: u64, slim: bool) {
    let c = (r.width() / 16.0).min(r.height() / 32.0);
    let o = r.center() - vec2(8.0 * c, 16.0 * c);
    let mut rng = Rng::new(seed ^ 0xABCD);
    let px = |x: i32, y: i32, col: Color32, rng: &mut Rng| {
        let cr = Rect::from_min_size(o + vec2(x as f32 * c, y as f32 * c), Vec2::splat(c));
        p.rect_filled(cr.expand(0.25), 0, noisy(col, rng, 0.08));
    };
    // голова
    let hr = Rect::from_min_size(o + vec2(4.0 * c, 0.0), Vec2::splat(8.0 * c));
    head(p, hr, look, seed);
    let aw = if slim { 3 } else { 4 };
    for y in 8..20 {
        // тело
        for x in 4..12 {
            let col = if y == 8 && (6..10).contains(&x) { hex(look.skin) } else { hex(look.shirt) };
            px(x, y, col, &mut rng);
        }
        // руки
        for i in 0..aw {
            let col = if y >= 17 { hex(look.skin) } else { darken(hex(look.shirt), 0.12) };
            px(4 - aw + i, y, col, &mut rng);
            px(12 + i, y, col, &mut rng);
        }
    }
    for y in 20..32 {
        for x in 4..12 {
            let col = if y >= 30 { hex(look.shoes) } else if x == 7 || x == 8 { darken(hex(look.pants), 0.1) } else { hex(look.pants) };
            px(x, y, col, &mut rng);
        }
    }
}

/// Логотип «Т из блоков» — изометрические кубики, грани от акцента.
pub fn logo(p: &Painter, r: Rect) {
    const VB: (f32, f32) = (242.5, 315.0);
    // (тип грани, четыре точки)
    const CUBES: [(f32, f32); 5] = [(121.2, 175.0), (121.2, 105.0), (60.6, 0.0), (121.2, 35.0), (181.9, 70.0)];
    let k = (r.width() / VB.0).min(r.height() / VB.1);
    let o = r.center() - vec2(VB.0, VB.1) * k / 2.0;
    let pt = |x: f32, y: f32| o + vec2(x, y) * k;
    let (top, left, right) = (lighten(accent(), 0.18), accent(), darken(accent(), 0.25));
    for &(cx, cy) in &CUBES {
        let (w, h) = (60.6, 35.0);
        let t = vec![pt(cx, cy), pt(cx + w, cy + h), pt(cx, cy + 2.0 * h), pt(cx - w, cy + h)];
        let l = vec![pt(cx - w, cy + h), pt(cx, cy + 2.0 * h), pt(cx, cy + 4.0 * h), pt(cx - w, cy + 3.0 * h)];
        let rr = vec![pt(cx, cy + 2.0 * h), pt(cx + w, cy + h), pt(cx + w, cy + 3.0 * h), pt(cx, cy + 4.0 * h)];
        p.add(Shape::convex_polygon(t, top, Stroke::NONE));
        p.add(Shape::convex_polygon(l, left, Stroke::NONE));
        p.add(Shape::convex_polygon(rr, right, Stroke::NONE));
    }
}

/// «Скриншот» — блочный пейзаж из столбиков.
pub fn landscape(p: &Painter, r: Rect, seed: u64, rounding: f32) {
    let mut rng = Rng::new(seed);
    let hue = 0.53 + rng.f() * 0.1;
    let night = rng.f() < 0.3;
    let sky = if night { hsv(0.65, 0.5, 0.18) } else { hsv(hue, 0.35, 0.82) };
    p.rect_filled(r, rounding, sky);
    let painter = p.with_clip_rect(r.shrink(0.5));
    let cols = 32;
    let cw = r.width() / cols as f32;
    let mut h = 0.35 + rng.f() * 0.2;
    // солнце/луна
    let sun = pos2(r.left() + r.width() * (0.2 + rng.f() * 0.6), r.top() + r.height() * 0.22);
    painter.rect_filled(Rect::from_center_size(sun, Vec2::splat(cw * 2.6)), 0, if night { hsv(0.15, 0.08, 0.95) } else { hsv(0.13, 0.35, 1.0) });
    for i in 0..cols {
        h = (h + (rng.f() - 0.5) * 0.12).clamp(0.2, 0.62);
        let top = r.bottom() - r.height() * h;
        let x = r.left() + i as f32 * cw;
        let col = Rect::from_min_max(pos2(x, top), pos2(x + cw + 0.5, r.bottom()));
        let k = if night { 0.45 } else { 1.0 };
        painter.rect_filled(col, 0, hsv(0.08, 0.45, 0.42 * k));
        painter.rect_filled(Rect::from_min_size(col.min, vec2(col.width(), cw)), 0, hsv(0.28, 0.55, 0.62 * k));
        if rng.f() < 0.12 {
            // дерево
            let trunk = Rect::from_min_size(pos2(x + cw * 0.25, top - cw * 3.0), vec2(cw * 0.5, cw * 3.0));
            painter.rect_filled(trunk, 0, hsv(0.07, 0.5, 0.35 * k));
            painter.rect_filled(Rect::from_center_size(pos2(x + cw / 2.0, top - cw * 3.5), vec2(cw * 3.0, cw * 2.2)), 0, hsv(0.3, 0.6, 0.42 * k));
        }
    }
}
