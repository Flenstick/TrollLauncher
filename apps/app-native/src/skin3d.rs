//! 3D-модель игрока по текстуре скина. Рисуется прямо мешем egui:
//! грани кубов проецируются на CPU, сортируются по глубине, текстура с NEAREST-фильтром.

use eframe::egui::{self, Color32, Mesh, Painter, Pos2, Rect, Shape, TextureId, epaint::Vertex, pos2};

#[derive(Clone, Copy)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}
fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

/// Часть тела: размер (w,h,d) в пикселях скина, начало текстуры (u,v),
/// положение нижнего-левого-заднего угла, точка поворота и угол качания вокруг X.
struct Part {
    size: (f32, f32, f32),
    uv: (f32, f32),
    origin: V3,
    pivot: V3,
    swing: f32,
    inflate: f32,
}

#[derive(Clone, Copy)]
pub struct Pose {
    pub yaw: f32,
    pub pitch: f32,
    /// фаза ходьбы (радианы), 0 — стоит
    pub walk: f32,
    /// лёгкое «дыхание» рук
    pub idle: f32,
}

pub fn draw(p: &Painter, rect: Rect, tex: TextureId, tex_h: f32, slim: bool, pose: Pose) {
    let legacy = tex_h < 63.0;
    let aw = if slim { 3.0 } else { 4.0 };
    let swing = pose.walk.sin() * 0.6;
    let arm_idle = pose.idle.sin() * 0.05;
    let mut parts: Vec<Part> = vec![];
    let mut add = |size: (f32, f32, f32), uv: (f32, f32), origin: V3, pivot: V3, swing: f32, overlay: Option<((f32, f32), f32)>| {
        parts.push(Part { size, uv, origin, pivot, swing, inflate: 0.0 });
        if let Some((ouv, inf)) = overlay {
            parts.push(Part { size, uv: ouv, origin, pivot, swing, inflate: inf });
        }
    };
    let ov = |uv: (f32, f32), inf: f32| if legacy { None } else { Some((uv, inf)) };
    // голова (шляпа есть и у старых скинов)
    add((8.0, 8.0, 8.0), (0.0, 0.0), v3(-4.0, 24.0, -4.0), v3(0.0, 24.0, 0.0), 0.0, Some(((32.0, 0.0), 0.5)));
    // тело
    add((8.0, 12.0, 4.0), (16.0, 16.0), v3(-4.0, 12.0, -2.0), v3(0.0, 0.0, 0.0), 0.0, ov((16.0, 32.0), 0.25));
    // правая рука (слева на экране) и левая
    add((aw, 12.0, 4.0), (40.0, 16.0), v3(-4.0 - aw, 12.0, -2.0), v3(-4.0 - aw / 2.0, 22.0, 0.0), swing + arm_idle, ov((40.0, 32.0), 0.25));
    let luv = if legacy { (40.0, 16.0) } else { (32.0, 48.0) };
    add((aw, 12.0, 4.0), luv, v3(4.0, 12.0, -2.0), v3(4.0 + aw / 2.0, 22.0, 0.0), -swing - arm_idle, ov((48.0, 48.0), 0.25));
    // ноги
    add((4.0, 12.0, 4.0), (0.0, 16.0), v3(-4.0, 0.0, -2.0), v3(-2.0, 12.0, 0.0), -swing, ov((0.0, 32.0), 0.25));
    let lluv = if legacy { (0.0, 16.0) } else { (16.0, 48.0) };
    add((4.0, 12.0, 4.0), lluv, v3(0.0, 0.0, -2.0), v3(2.0, 12.0, 0.0), swing, ov((0.0, 48.0), 0.25));

    // проекция
    let scale = (rect.height() / 36.0).min(rect.width() / 20.0);
    let center = rect.center();
    let (sy, cy) = pose.yaw.sin_cos();
    let (sp, cp) = pose.pitch.sin_cos();
    let view = |v: V3| -> V3 {
        // центр модели на высоте 16
        let (x, y, z) = (v.x, v.y - 16.0, v.z);
        let (x, z) = (x * cy + z * sy, -x * sy + z * cy);
        let (y, z) = (y * cp - z * sp, y * sp + z * cp);
        v3(x, y, z)
    };
    let project = |v: V3| -> Pos2 {
        let f = 60.0 / (60.0 - v.z);
        pos2(center.x + v.x * scale * f, center.y - v.y * scale * f)
    };

    struct Quad {
        pts: [Pos2; 4],
        uv: [(f32, f32); 4],
        z: f32,
        shade: f32,
    }
    let mut quads: Vec<Quad> = vec![];
    let (tw, th) = (64.0, tex_h);
    for part in &parts {
        let (w, h, d) = part.size;
        let i = part.inflate;
        let (x0, y0, z0) = (part.origin.x - i, part.origin.y - i, part.origin.z - i);
        let (x1, y1, z1) = (part.origin.x + w + i, part.origin.y + h + i, part.origin.z + d + i);
        let (u, v) = part.uv;
        let (sa, ca) = part.swing.sin_cos();
        let local = |q: V3| -> V3 {
            // качание вокруг X через точку поворота
            let (dy, dz) = (q.y - part.pivot.y, q.z - part.pivot.z);
            v3(q.x, part.pivot.y + dy * ca - dz * sa, part.pivot.z + dy * sa + dz * ca)
        };
        // (4 угла: верх-лево, верх-право, низ-право, низ-лево), uv-прямоугольник, нормаль
        let faces: [([V3; 4], (f32, f32, f32, f32), V3); 6] = [
            ([v3(x0, y1, z1), v3(x1, y1, z1), v3(x1, y0, z1), v3(x0, y0, z1)], (u + d, v + d, w, h), v3(0.0, 0.0, 1.0)),
            ([v3(x1, y1, z0), v3(x0, y1, z0), v3(x0, y0, z0), v3(x1, y0, z0)], (u + 2.0 * d + w, v + d, w, h), v3(0.0, 0.0, -1.0)),
            ([v3(x0, y1, z0), v3(x0, y1, z1), v3(x0, y0, z1), v3(x0, y0, z0)], (u, v + d, d, h), v3(-1.0, 0.0, 0.0)),
            ([v3(x1, y1, z1), v3(x1, y1, z0), v3(x1, y0, z0), v3(x1, y0, z1)], (u + d + w, v + d, d, h), v3(1.0, 0.0, 0.0)),
            ([v3(x0, y1, z0), v3(x1, y1, z0), v3(x1, y1, z1), v3(x0, y1, z1)], (u + d, v, w, d), v3(0.0, 1.0, 0.0)),
            ([v3(x0, y0, z1), v3(x1, y0, z1), v3(x1, y0, z0), v3(x0, y0, z0)], (u + d + w, v, w, d), v3(0.0, -1.0, 0.0)),
        ];
        for (corners, (fu, fv, fw, fh), n) in faces {
            let n = view(v3(part.pivot.x + n.x, part.pivot.y + n.y * ca - n.z * sa, part.pivot.z + n.y * sa + n.z * ca));
            let o = view(part.pivot);
            let nz = n.z - o.z;
            let ny = n.y - o.y;
            if nz <= 0.001 {
                continue;
            }
            let vs: Vec<V3> = corners.iter().map(|c| view(local(*c))).collect();
            let z = vs.iter().map(|v| v.z).sum::<f32>() / 4.0 + i * 0.6;
            let e = 0.02;
            let (ua, va, ub, vb) = ((fu + e) / tw, (fv + e) / th, (fu + fw - e) / tw, (fv + fh - e) / th);
            quads.push(Quad {
                pts: [project(vs[0]), project(vs[1]), project(vs[2]), project(vs[3])],
                uv: [(ua, va), (ub, va), (ub, vb), (ua, vb)],
                z,
                shade: 0.72 + 0.28 * nz.clamp(0.0, 1.0) + 0.06 * ny.clamp(0.0, 1.0),
            });
        }
    }
    quads.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));
    let mut mesh = Mesh::with_texture(tex);
    for q in quads {
        let s = (q.shade.min(1.0) * 255.0) as u8;
        let col = Color32::from_rgb(s, s, s);
        let base = mesh.vertices.len() as u32;
        for k in 0..4 {
            mesh.vertices.push(Vertex { pos: q.pts[k], uv: egui::pos2(q.uv[k].0, q.uv[k].1), color: col });
        }
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    p.add(Shape::mesh(mesh));
}

/// Голова спереди (лицо + шляпа) — для аватаров.
pub fn draw_face(p: &Painter, r: Rect, tex: TextureId, tex_h: f32) {
    let uv = |u: f32, v: f32, w: f32, h: f32| Rect::from_min_max(pos2(u / 64.0, v / tex_h), pos2((u + w) / 64.0, (v + h) / tex_h));
    p.image(tex, r, uv(8.0, 8.0, 8.0, 8.0), Color32::WHITE);
    p.image(tex, r.expand(r.width() * 0.04), uv(40.0, 8.0, 8.0, 8.0), Color32::WHITE);
}
