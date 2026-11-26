//! Облик целиком: макеты (тема + расположение меню + скругление + плотность + шрифт + иконки),
//! выбор шрифта (встроенные, любой шрифт системы, свой файл) и свои наборы иконок
//! (`<данные>/icons/<набор>/<имя>.png|svg`).

use crate::theme::*;
use crate::widgets::{ICON_NAMES, Icon, IconStyle};
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ───────────────────────── меню ─────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Nav {
    /// Узкая панель с иконками слева
    #[default]
    Side,
    /// Широкая панель слева: иконки с подписями и список сборок
    Wide,
    /// Панель-тулбар сверху с подписями (как в Prism Launcher)
    Top,
    /// Док снизу
    Dock,
    /// Окно как в Prism Launcher: тулбар, сетка сборок по группам, панель действий справа, строка статуса
    Prism,
}

pub const NAVS: [(Nav, &str); 5] =
    [(Nav::Side, "Слева"), (Nav::Wide, "С подписями"), (Nav::Top, "Сверху"), (Nav::Dock, "Снизу"), (Nav::Prism, "Как в Prism")];

// ───────────────────────── макеты ─────────────────────────

pub enum PresetFont {
    Inter,
    Pixel,
    Mono,
    /// Первый найденный в системе, иначе Inter
    System(&'static [&'static str]),
}

pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub theme: &'static str,
    pub nav: Nav,
    pub round: f32,
    pub density: f32,
    pub icons: &'static str,
    pub font: PresetFont,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "trollland",
        name: "TrollLand",
        desc: "Родной вид: узкое меню слева, синий TrollLand, Inter",
        theme: "trollland",
        nav: Nav::Side,
        round: 1.0,
        density: 1.0,
        icons: "lines",
        font: PresetFont::Inter,
    },
    Preset {
        id: "prism",
        name: "PrismTroll",
        desc: "Тулбар с подписями сверху, серый Fusion, почти квадратные углы, цветные иконки и шрифт системы",
        theme: "prism",
        nav: Nav::Top,
        round: 0.3,
        density: 0.85,
        icons: "color",
        font: PresetFont::System(&["Noto Sans", "Segoe UI", "Cantarell", "DejaVu Sans", "Liberation Sans", "Arial"]),
    },
    Preset {
        id: "prismlauncher",
        name: "Prism Launcher",
        desc: "Окно один в один как Prism Launcher: тулбар, сетка сборок по группам, действия справа, котик и строка статуса",
        theme: "prismtroll",
        nav: Nav::Prism,
        round: 0.15,
        density: 0.85,
        icons: "color",
        font: PresetFont::System(&["Pixeloid Sans", "Noto Sans", "Segoe UI", "Cantarell", "DejaVu Sans", "Liberation Sans", "Arial"]),
    },
    Preset {
        id: "pixel",
        name: "Пиксель",
        desc: "Квадратные блоки, пиксельные иконки и пиксельный шрифт, трава и камень",
        theme: "pixel",
        nav: Nav::Side,
        round: 0.0,
        density: 1.0,
        icons: "pixel",
        font: PresetFont::Pixel,
    },
    Preset {
        id: "terminal",
        name: "Терминал",
        desc: "Зелёный на чёрном, моноширинный шрифт, тонкие линии, тулбар сверху",
        theme: "matrix",
        nav: Nav::Top,
        round: 0.15,
        density: 0.85,
        icons: "thin",
        font: PresetFont::Mono,
    },
    Preset {
        id: "bubble",
        name: "Пузырь",
        desc: "Круглое и мягкое: док снизу, большие отступы, жирные иконки",
        theme: "dreamcore",
        nav: Nav::Dock,
        round: 1.9,
        density: 1.2,
        icons: "bold",
        font: PresetFont::Inter,
    },
    Preset {
        id: "compact",
        name: "Компакт",
        desc: "Широкое меню с подписями и сборками, плотная раскладка — больше влезает на экран",
        theme: "midnight",
        nav: Nav::Wide,
        round: 0.6,
        density: 0.8,
        icons: "lines",
        font: PresetFont::Inter,
    },
    Preset {
        id: "minimal",
        name: "Минимал",
        desc: "Светлый и воздушный, тонкие иконки, меню с подписями",
        theme: "light",
        nav: Nav::Wide,
        round: 1.25,
        density: 1.15,
        icons: "thin",
        font: PresetFont::Inter,
    },
    Preset {
        id: "grimoire",
        name: "Гримуар",
        desc: "Шабаш: золото и кровь, шрифт с засечками, эмодзи вместо иконок",
        theme: "sabbath",
        nav: Nav::Wide,
        round: 0.5,
        density: 1.0,
        icons: "emoji",
        font: PresetFont::System(&["Noto Serif", "DejaVu Serif", "Liberation Serif", "Georgia", "Cambria", "Times New Roman"]),
    },
];

pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

// ───────────────────────── применение ─────────────────────────

static APPLIED_FONT: Mutex<String> = Mutex::new(String::new());
static APPLIED_ICONS: Mutex<String> = Mutex::new(String::new());

/// Применить облик из настроек. Шрифты и иконки перегружаются, только если поменялись.
pub fn apply(ctx: &egui::Context, p: &crate::prefs::Prefs) {
    set_theme(theme_index(&p.theme), p.accent, p.custom_colors);
    set_shape(p.round, p.density);
    let fk = format!("{}|{}", p.font, p.font_files.join(";"));
    let mut af = APPLIED_FONT.lock().unwrap();
    if *af != fk {
        setup_fonts(ctx, &p.font, &p.font_files);
        *af = fk;
    }
    let ik = format!("{}|{}", p.icons, p.icon_tint);
    let mut ai = APPLIED_ICONS.lock().unwrap();
    if *ai != ik {
        crate::widgets::set_icon_style(icon_style(ctx, &p.icons, p.icon_tint));
        *ai = ik;
    }
    apply_style(ctx);
    let speed = 40.0 * p.scroll_speed.clamp(0.5, 6.0);
    ctx.options_mut(|o| o.input_options.line_scroll_speed = speed);
}

/// Перечитать свой набор иконок с диска.
pub fn reload_icons(ctx: &egui::Context, p: &crate::prefs::Prefs) {
    APPLIED_ICONS.lock().unwrap().clear();
    apply(ctx, p);
}

/// Включить макет. Шрифт системы ищется в списке шрифтов; если список ещё не готов — `false`
/// (шрифт применится позже, см. `resolve_pending_font`).
pub fn apply_preset(p: &mut crate::prefs::Prefs, pr: &Preset) -> bool {
    p.preset = pr.id.into();
    p.theme = pr.theme.into();
    p.accent = None;
    p.nav = pr.nav;
    p.round = pr.round;
    p.density = pr.density;
    p.icons = pr.icons.into();
    p.font_files.clear();
    match pr.font {
        PresetFont::Inter => p.font = FONT_INTER.into(),
        PresetFont::Pixel => p.font = FONT_PIXEL.into(),
        PresetFont::Mono => p.font = FONT_MONO.into(),
        PresetFont::System(names) => {
            p.font = FONT_INTER.into();
            let Some(fams) = FONTS.get() else { return false };
            if let Some((f, files)) = resolve_system(names, &fams) {
                p.font = f;
                p.font_files = files;
            }
        }
    }
    true
}

// ───────────────────────── шрифты ─────────────────────────

pub const FONT_INTER: &str = "inter";
pub const FONT_PIXEL: &str = "pixel";
pub const FONT_MONO: &str = "mono";

pub struct Face {
    pub path: PathBuf,
    pub index: u32,
    pub weight: u16,
}

pub struct SysFamily {
    pub name: String,
    pub faces: Vec<Face>,
}

impl SysFamily {
    /// Файлы для обычного, medium, semibold и bold (ближайшие по толщине).
    pub fn files(&self) -> Vec<String> {
        let pick = |want: u16, up: bool| {
            self.faces
                .iter()
                .min_by_key(|f| {
                    let d = f.weight.abs_diff(want) as i32;
                    // при равенстве — в нужную сторону
                    d * 2 + i32::from((f.weight < want) == up && f.weight != want)
                })
                .map(|f| format!("{}#{}", f.path.display(), f.index))
                .unwrap_or_default()
        };
        vec![pick(400, false), pick(500, false), pick(600, true), pick(700, true)]
    }
}

/// Шрифты системы с кириллицей (без курсивов), по семействам. Ищется один раз, в фоне.
pub struct FontScan {
    state: Mutex<Option<Arc<Vec<SysFamily>>>>,
    started: std::sync::atomic::AtomicBool,
}

pub static FONTS: FontScan = FontScan { state: Mutex::new(None), started: std::sync::atomic::AtomicBool::new(false) };

impl FontScan {
    pub fn start(&'static self, ctx: &egui::Context) {
        if self.started.swap(true, std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let list = scan_system_fonts();
            *self.state.lock().unwrap() = Some(Arc::new(list));
            ctx.request_repaint();
        });
    }
    pub fn get(&self) -> Option<Arc<Vec<SysFamily>>> {
        self.state.lock().unwrap().clone()
    }
}

fn font_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if cfg!(windows) {
        v.push(PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into())).join("Fonts"));
        if let Ok(l) = std::env::var("LOCALAPPDATA") {
            v.push(PathBuf::from(l).join("Microsoft\\Windows\\Fonts"));
        }
    } else {
        v.extend(["/usr/share/fonts", "/usr/local/share/fonts"].map(PathBuf::from));
        if let Some(h) = dirs::home_dir() {
            v.push(h.join(".local/share/fonts"));
            v.push(h.join(".fonts"));
        }
    }
    v
}

fn scan_system_fonts() -> Vec<SysFamily> {
    let mut files = Vec::new();
    let mut stack = font_dirs();
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc")) {
                files.push(p);
            }
        }
    }
    let mut fams: HashMap<String, SysFamily> = HashMap::new();
    for path in files {
        let Ok(file) = std::fs::File::open(&path) else { continue };
        // SAFETY: только читаем; если файл поменяют во время чтения — в худшем случае мусор в имени
        let Ok(map) = (unsafe { memmap2::Mmap::map(&file) }) else { continue };
        let n = ttf_parser::fonts_in_collection(&map).unwrap_or(1);
        for index in 0..n {
            let Ok(face) = ttf_parser::Face::parse(&map, index) else { continue };
            if face.is_italic() || face.is_oblique() || face.glyph_index('Ж').is_none() || face.glyph_index('A').is_none() {
                continue;
            }
            let name = |id: u16| {
                face.names().into_iter().filter(|n| n.name_id == id && n.is_unicode()).find(|n| n.language_id == 0x409 || n.language_id == 0).and_then(|n| n.to_string())
            };
            let Some(family) = name(ttf_parser::name_id::TYPOGRAPHIC_FAMILY).or_else(|| name(ttf_parser::name_id::FAMILY)) else { continue };
            // переменные шрифты дают только стандартное начертание — этого хватает
            let weight = face.weight().to_number();
            let fam = fams.entry(family.clone()).or_insert_with(|| SysFamily { name: family, faces: Vec::new() });
            if !fam.faces.iter().any(|f| f.weight == weight) {
                fam.faces.push(Face { path: path.clone(), index, weight });
            }
        }
    }
    let mut v: Vec<_> = fams.into_values().filter(|f| f.faces.iter().any(|x| (300..=500).contains(&x.weight))).collect();
    v.sort_by_key(|f| f.name.to_lowercase());
    v
}

fn read_face(spec: &str) -> Option<FontData> {
    let (path, idx) = spec.rsplit_once('#').and_then(|(p, i)| Some((p, i.parse().ok()?))).unwrap_or((spec, 0));
    let bytes = std::fs::read(path).ok()?;
    let mut fd = FontData::from_owned(bytes);
    fd.index = idx;
    Some(fd)
}

/// Подключить шрифт из настроек. Inter остаётся запасным (для недостающих символов), за ним — эмодзи egui.
pub fn setup_fonts(ctx: &egui::Context, font: &str, files: &[String]) {
    let mut f = FontDefinitions::default();
    let inter: [(&str, &'static [u8]); 4] = [
        ("inter", include_bytes!("../assets/Inter-Regular.ttf")),
        ("inter-medium", include_bytes!("../assets/Inter-Medium.ttf")),
        ("inter-semibold", include_bytes!("../assets/Inter-SemiBold.ttf")),
        ("inter-bold", include_bytes!("../assets/Inter-Bold.ttf")),
    ];
    for (k, bytes) in inter {
        f.font_data.insert(k.into(), Arc::new(FontData::from_static(bytes)));
    }
    let fallback = f.families[&FontFamily::Proportional].clone();
    let mono_key = f.families[&FontFamily::Monospace].first().cloned().unwrap_or_default();
    // ключи для обычного/medium/semibold/bold
    let mut keys: [Option<String>; 4] = Default::default();
    match font {
        FONT_PIXEL => {
            f.font_data.insert("pixel".into(), Arc::new(FontData::from_static(include_bytes!("../assets/PixelifySans.ttf"))));
            keys = std::array::from_fn(|_| Some("pixel".into()));
        }
        FONT_MONO => keys = std::array::from_fn(|_| Some(mono_key.clone())),
        _ if font.starts_with("sys:") || font.starts_with("file:") => {
            let specs: Vec<String> = if let Some(p) = font.strip_prefix("file:") { vec![p.to_string(); 4] } else { files.to_vec() };
            let mut loaded: HashMap<String, String> = HashMap::new();
            for (i, spec) in specs.iter().take(4).enumerate() {
                if spec.is_empty() {
                    continue;
                }
                if let Some(k) = loaded.get(spec) {
                    keys[i] = Some(k.clone());
                } else if let Some(fd) = read_face(spec) {
                    let k = format!("user-{i}");
                    f.font_data.insert(k.clone(), Arc::new(fd));
                    loaded.insert(spec.clone(), k.clone());
                    keys[i] = Some(k);
                }
            }
            // недостающие толщины — ближайшей имеющейся
            for i in 0..4 {
                if keys[i].is_none() {
                    keys[i] = [i, 0, 1, 2, 3].iter().find_map(|&j| keys.get(j).cloned().flatten());
                }
            }
        }
        _ => {}
    }
    let fam = |own: &Option<String>, inter_key: &str| {
        let mut v: Vec<String> = own.iter().cloned().collect();
        v.push(inter_key.into());
        v.extend(fallback.iter().filter(|k| *k != inter_key).cloned());
        v
    };
    f.families.insert(FontFamily::Proportional, fam(&keys[0], "inter"));
    for (i, (name, key)) in [("medium", "inter-medium"), ("semibold", "inter-semibold"), ("bold", "inter-bold")].into_iter().enumerate() {
        f.families.insert(FontFamily::Name(name.into()), fam(&keys[i + 1], key));
    }
    ctx.set_fonts(f);
}

/// Подходящий шрифт системы для макета.
pub fn resolve_system(names: &[&str], fams: &[SysFamily]) -> Option<(String, Vec<String>)> {
    names.iter().find_map(|n| fams.iter().find(|f| f.name.eq_ignore_ascii_case(n))).map(|f| (format!("sys:{}", f.name), f.files()))
}

/// Скопировать свой файл шрифта в папку лаунчера.
pub fn add_font_file(src: &Path) -> Result<String, String> {
    let dir = crate::prefs::data_dir().join("fonts");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bytes = std::fs::read(src).map_err(|e| e.to_string())?;
    ttf_parser::Face::parse(&bytes, 0).map_err(|_| "это не файл шрифта TTF/OTF".to_string())?;
    let dst = dir.join(src.file_name().ok_or("нет имени файла")?);
    std::fs::write(&dst, bytes).map_err(|e| e.to_string())?;
    Ok(format!("file:{}", dst.display()))
}

/// Название шрифта для показа.
pub fn font_label(font: &str) -> String {
    match font {
        FONT_PIXEL => "Pixelify Sans — пиксельный".into(),
        FONT_MONO => "Hack — моноширинный".into(),
        _ => {
            if let Some(n) = font.strip_prefix("sys:") {
                n.into()
            } else if let Some(p) = font.strip_prefix("file:") {
                format!("Свой: {}", Path::new(p).file_stem().and_then(|s| s.to_str()).unwrap_or(p))
            } else {
                "Inter".into()
            }
        }
    }
}

// ───────────────────────── иконки ─────────────────────────

pub const ICON_STYLES: [(&str, &str); 6] =
    [("lines", "Линии"), ("thin", "Тонкие"), ("bold", "Жирные"), ("color", "Цветные"), ("pixel", "Пиксели"), ("emoji", "Эмодзи")];

pub fn icons_dir() -> PathBuf {
    crate::prefs::data_dir().join("icons")
}

/// Свои наборы: подпапки `icons/`, в каждой хотя бы одна картинка с именем иконки.
pub fn pack_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(icons_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .collect();
    v.sort_by_key(|s| s.to_lowercase());
    v
}

static PACK_GEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Загрузить набор в загрузчик картинок egui. `tint` — перекрасить в белый, чтобы иконка
/// принимала цвет темы (так рисуются и встроенные).
pub fn load_pack(ctx: &egui::Context, name: &str, tint: bool) -> HashMap<Icon, String> {
    let dir = icons_dir().join(name);
    let g = PACK_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut map = HashMap::new();
    for (ic, file) in ICON_NAMES {
        let (path, svg) = if dir.join(format!("{file}.svg")).is_file() {
            (dir.join(format!("{file}.svg")), true)
        } else if dir.join(format!("{file}.png")).is_file() {
            (dir.join(format!("{file}.png")), false)
        } else {
            continue;
        };
        let Ok(mut bytes) = std::fs::read(&path) else { continue };
        if tint {
            bytes = if svg { whiten_svg(&bytes) } else { whiten_png(&bytes).unwrap_or(bytes) };
        } else if svg {
            // «цвет текста» без покраски — белый, иначе чёрная иконка на тёмной теме
            bytes = String::from_utf8_lossy(&bytes).replace("currentColor", "#ffffff").into_bytes();
        }
        let uri = format!("bytes://iconpack/{g}/{name}/{file}.{}", if svg { "svg" } else { "png" });
        ctx.include_bytes(uri.clone(), bytes);
        map.insert(ic, uri);
    }
    map
}

fn whiten_svg(b: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(b);
    let mut s = s.replace("currentColor", "#ffffff");
    for black in ["\"#000000\"", "\"#000\"", "\"black\"", "'#000000'", "'#000'", "'black'"] {
        s = s.replace(black, "\"#ffffff\"");
    }
    for black in [":#000000", ":#000;", ":black", ": #000000", ": black"] {
        s = s.replace(black, ":#ffffff");
    }
    // без fill у корня SVG рисует чёрным
    if !s.contains("fill=") && !s.contains("fill:") {
        s = s.replacen("<svg", "<svg fill=\"#ffffff\"", 1);
    }
    s.into_bytes()
}

fn whiten_png(b: &[u8]) -> Option<Vec<u8>> {
    let mut img = image::load_from_memory(b).ok()?.to_rgba8();
    let opaque = img.pixels().all(|p| p[3] == 255);
    for p in img.pixels_mut() {
        // непрозрачная картинка — берём яркость как маску (тёмный рисунок на светлом фоне)
        if opaque {
            let l = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
            p[3] = 255 - l as u8;
        }
        p[0] = 255;
        p[1] = 255;
        p[2] = 255;
    }
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

/// Шаблон своего набора: встроенные иконки в SVG + README с именами файлов.
pub fn make_template() -> Result<PathBuf, String> {
    let mut dir = icons_dir().join("Мой набор");
    let mut n = 2;
    while dir.exists() {
        dir = icons_dir().join(format!("Мой набор {n}"));
        n += 1;
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for (ic, file) in ICON_NAMES {
        std::fs::write(dir.join(format!("{file}.svg")), crate::widgets::icon_svg(ic)).map_err(|e| e.to_string())?;
    }
    let names: Vec<String> = ICON_NAMES.iter().map(|(_, n)| format!("  {n}")).collect();
    let readme = format!(
        "Свой набор иконок TrollLauncher\n\n\
         Замените картинки своими: SVG или PNG (лучше квадратные, от 48×48), имя файла — имя иконки.\n\
         Каких-то файлов нет — на их месте будет встроенная иконка.\n\
         Если в настройках включено «Красить в цвет темы», иконка станет одноцветной (как встроенные);\n\
         выключите для цветных иконок.\n\
         Набор — эта папка; переименуйте её как хотите. После правок нажмите «Обновить» в Настройках → Внешний вид.\n\n\
         Имена иконок:\n{}\n",
        names.join("\n")
    );
    std::fs::write(dir.join("README.txt"), readme).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Стиль иконок по имени из настроек (`pack:<имя>` — свой набор).
pub fn icon_style(ctx: &egui::Context, id: &str, tint: bool) -> IconStyle {
    match id {
        "thin" => IconStyle::Lines(0.65),
        "bold" => IconStyle::Lines(1.55),
        "color" => IconStyle::Color,
        "pixel" => IconStyle::Pixel,
        "emoji" => IconStyle::Emoji,
        _ => match id.strip_prefix("pack:") {
            Some(name) => IconStyle::Pack(Arc::new(load_pack(ctx, name, tint)), tint),
            None => IconStyle::Lines(1.0),
        },
    }
}
