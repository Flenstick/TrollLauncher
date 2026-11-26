//! Палитра, шрифты и стиль egui. Темы переключаются на лету: цвета читаются из общей палитры,
//! поэтому `BG()`, `TEXT()` и т.д. — функции. Первая тема — цвета логотипа TrollLand.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};
use std::sync::{
    RwLock,
    atomic::{AtomicU32, Ordering},
};

/// Цвета темы (0xRRGGBB).
#[derive(Clone, Copy)]
pub struct Palette {
    /// Боковая и верхняя панели
    pub bg: u32,
    /// Фон страниц
    pub content: u32,
    /// Карточки
    pub panel: u32,
    /// Кнопки, приподнятые элементы
    pub raised: u32,
    pub hover: u32,
    pub border: u32,
    pub input: u32,
    pub text: u32,
    pub dim: u32,
    pub faint: u32,
    pub green: u32,
    pub yellow: u32,
    pub blue: u32,
    pub danger: u32,
    pub accent: u32,
    pub light: bool,
}

pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    pub p: Palette,
}

macro_rules! theme {
    ($id:literal, $name:literal, light: $light:literal,
     $bg:literal $content:literal $panel:literal $raised:literal $hover:literal $border:literal $input:literal,
     $text:literal $dim:literal $faint:literal,
     $green:literal $yellow:literal $blue:literal $danger:literal, accent $accent:literal) => {
        Theme {
            id: $id,
            name: $name,
            p: Palette {
                bg: $bg, content: $content, panel: $panel, raised: $raised, hover: $hover, border: $border, input: $input,
                text: $text, dim: $dim, faint: $faint,
                green: $green, yellow: $yellow, blue: $blue, danger: $danger, accent: $accent, light: $light,
            },
        }
    };
}

#[rustfmt::skip]
pub const THEMES: &[Theme] = &[
    //      фоны: bg content panel raised hover border input; текст: text dim faint; green yellow blue danger
    theme!("trollland", "TrollLand", light: false,
        0x07070d 0x0a0a14 0x0f0f1d 0x15152a 0x1e1e3a 0x1c1c34 0x06060e, 0xeceeff 0x9a9ec4 0x5c6088,
        0x46a758 0xf5a524 0x7f8bfe 0xd13d45, accent 0x2f2cf8),
    theme!("prism", "Prism (Fusion)", light: false,
        0x2a2a2a 0x313131 0x383838 0x424242 0x4d4d4d 0x232323 0x1e1e1e, 0xececec 0xb4b4b4 0x828282,
        0x5cb85c 0xf0ad4e 0x5aa0e6 0xd9534f, accent 0x2a82da),
    theme!("prismtroll", "Prism Launcher", light: false,
        0x181616 0x100e0e 0x181616 0x2a2727 0x34302f 0x2a2727 0x100e0e, 0xe4dedc 0x928d8c 0x5c5858,
        0x5cb85c 0xf0ad4e 0x5aa0e6 0xd9534f, accent 0x645f5c),
    theme!("pixel", "Пиксель", light: false,
        0x1b1b1b 0x262626 0x313131 0x3c3c3c 0x4a4a4a 0x101010 0x141414, 0xf0f0f0 0xb8b8b8 0x7a7a7a,
        0x6fbf3a 0xffd83d 0x5ab4ff 0xe0443a, accent 0x5d9e2f),
    theme!("graphite", "Графит", light: false,
        0x0b0b0d 0x101013 0x16161a 0x1d1d22 0x27272e 0x232329 0x0d0d10, 0xeeeef2 0x9a9aa6 0x62626e,
        0x46a758 0xf5a524 0x4c9aff 0xd13d45, accent 0xe5484d),
    theme!("midnight", "Midnight", light: false,
        0x070b16 0x0a1020 0x0f172b 0x152038 0x1d2a48 0x1a2540 0x060a14, 0xe6edff 0x94a3c8 0x56658a,
        0x34d399 0xfbbf24 0x60a5fa 0xf05252, accent 0x3b82f6),
    theme!("inferno", "Inferno", light: false,
        0x0a0505 0x100707 0x180b0a 0x22100e 0x2e1512 0x2a1310 0x080404, 0xf6e7df 0xc09a8c 0x7a5a50,
        0x6fbf4a 0xffb020 0xff9a5c 0xff3b1f, accent 0xff4a1c),
    theme!("sabbath", "Шабаш", light: false,
        0x0b0705 0x110a07 0x1a100b 0x241710 0x311f15 0x2c1c12 0x090604, 0xf3e6cf 0xbfa47e 0x7a6448,
        0x7fa650 0xe0b440 0xc9a96e 0xb3122a, accent 0xc9a227),
    theme!("vampire", "Вампир", light: false,
        0x0b0609 0x10080d 0x170b13 0x21101b 0x2d1625 0x281421 0x090508, 0xf4e4ec 0xb98fa4 0x765467,
        0x5fae6e 0xf0a83a 0xb48cff 0xff3355, accent 0xb0123a),
    theme!("void", "Void", light: false,
        0x000000 0x050505 0x0b0b0b 0x141414 0x1f1f1f 0x1a1a1a 0x000000, 0xf5f5f5 0x9e9e9e 0x5e5e5e,
        0x7bd88f 0xe6c35c 0x9ab8ff 0xff5c5c, accent 0xe8e8e8),
    theme!("dreamcore", "Дримкор", light: false,
        0x120d1a 0x171121 0x1e162b 0x271d37 0x342646 0x2e2240 0x100b17, 0xfbefff 0xc6aed6 0x806a94,
        0x9ff0c0 0xffe08a 0xa8c8ff 0xff7aa8, accent 0xf0a6ff),
    theme!("kyoto", "Киото · Осень", light: false,
        0x110c09 0x16100c 0x1e1610 0x281d15 0x35271c 0x2f2219 0x0e0a07, 0xf5eadf 0xc2a890 0x7f6a57,
        0x8fb65a 0xf2b84b 0x86b3d1 0xd9493a, accent 0xe8763a),
    theme!("mocha", "Catppuccin Mocha", light: false,
        0x11111b 0x181825 0x1e1e2e 0x313244 0x45475a 0x313244 0x11111b, 0xcdd6f4 0xa6adc8 0x6c7086,
        0xa6e3a1 0xf9e2af 0x89b4fa 0xf38ba8, accent 0xcba6f7),
    theme!("nord", "Nord", light: false,
        0x242933 0x2a303c 0x2e3440 0x3b4252 0x4c566a 0x3b4252 0x232831, 0xeceff4 0xb4bccc 0x7b88a1,
        0xa3be8c 0xebcb8b 0x81a1c1 0xbf616a, accent 0x88c0d0),
    theme!("dracula", "Dracula", light: false,
        0x1e1f29 0x21222c 0x282a36 0x343746 0x44475a 0x383a4a 0x1b1c25, 0xf8f8f2 0xc0c4d6 0x6272a4,
        0x50fa7b 0xf1fa8c 0x8be9fd 0xff5555, accent 0xbd93f9),
    theme!("gruvbox", "Gruvbox", light: false,
        0x1d2021 0x232526 0x282828 0x32302f 0x45403d 0x3c3836 0x1a1c1d, 0xebdbb2 0xbdae93 0x7c6f64,
        0xb8bb26 0xfabd2f 0x83a598 0xfb4934, accent 0xfe8019),
    theme!("tokyo", "Tokyo Night", light: false,
        0x13131a 0x16161e 0x1a1b26 0x222436 0x2f334d 0x292e42 0x121218, 0xc0caf5 0xa9b1d6 0x565f89,
        0x9ece6a 0xe0af68 0x7aa2f7 0xf7768e, accent 0xbb9af7),
    theme!("rosepine", "Rosé Pine", light: false,
        0x15131f 0x191724 0x1f1d2e 0x26233a 0x393552 0x2a273f 0x13111c, 0xe0def4 0x908caa 0x6e6a86,
        0x9ccfd8 0xf6c177 0xc4a7e7 0xeb6f92, accent 0xebbcba),
    theme!("solarized", "Solarized Dark", light: false,
        0x00212b 0x002b36 0x073642 0x0d404d 0x174c5a 0x124553 0x001d26, 0xeee8d5 0x93a1a1 0x586e75,
        0x859900 0xb58900 0x6c9fd8 0xdc322f, accent 0x268bd2),
    theme!("matrix", "Матрица", light: false,
        0x020604 0x030a06 0x06110a 0x0a1a10 0x0f2617 0x0d2214 0x010402, 0xc8ffd4 0x7dc191 0x427a52,
        0x22e05a 0xd7e04a 0x4ad7c0 0xff5a4a, accent 0x22e05a),
    theme!("ocean", "Океан", light: false,
        0x05121a 0x071820 0x0b2029 0x102a35 0x173846 0x14323f 0x041016, 0xe2f5f7 0x8fb7bf 0x52767e,
        0x3fcf8e 0xf2c14e 0x5cc8ff 0xf0605d, accent 0x12a5b4),
    theme!("forest", "Лес", light: false,
        0x0a0f0a 0x0d140d 0x121b12 0x19241a 0x223123 0x1e2b1f 0x080c08, 0xe6f0e0 0xa3b799 0x637560,
        0x6aa84f 0xd9b44a 0x7fb0c9 0xd65745, accent 0x6aa84f),
    theme!("sakura", "Сакура", light: false,
        0x150a10 0x1a0d14 0x22111a 0x2c1622 0x3a1e2d 0x331a28 0x12080e, 0xffe9f2 0xd2a3b8 0x8d6274,
        0x7fd1a0 0xffcf6e 0xa9b8ff 0xff5470, accent 0xff6fa8),
    theme!("light", "Светлая", light: true,
        0xeceef3 0xf6f7fa 0xffffff 0xeef0f4 0xe1e4ea 0xdcdfe6 0xffffff, 0x16161c 0x5b5f6c 0x9599a6,
        0x2f9e44 0xd98a00 0x2f6fdd 0xd6334a, accent 0x2f2cf8),
    theme!("latte", "Catppuccin Latte", light: true,
        0xdce0e8 0xe6e9ef 0xeff1f5 0xdfe2ea 0xccd0da 0xccd0da 0xeff1f5, 0x4c4f69 0x6c6f85 0x9ca0b0,
        0x40a02b 0xdf8e1d 0x1e66f5 0xd20f39, accent 0x8839ef),
    theme!("sepia", "Сепия", light: true,
        0xe9dfcc 0xf3ebdc 0xfbf6ec 0xede3d0 0xe0d3bb 0xd8cab0 0xfdf9f1, 0x3b2f22 0x6e5d48 0xa08e74,
        0x5f8a32 0xb7791f 0x3e6f9e 0xb8382c, accent 0xb5651d),
    theme!("solarized-light", "Solarized Light", light: true,
        0xeee8d5 0xf5efdc 0xfdf6e3 0xefe9d6 0xe3dcc6 0xddd6c1 0xfffbee, 0x2a4048 0x657b83 0x93a1a1,
        0x859900 0xb58900 0x268bd2 0xdc322f, accent 0x268bd2),
];

// ── форма интерфейса (из макета): скругление и плотность ──
static ROUND: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0
static DENSITY: AtomicU32 = AtomicU32::new(0x3f80_0000);

/// Скругление углов (1 — как задумано, 0 — квадратные) и плотность (0.75 — тесно, 1.25 — просторно).
pub fn set_shape(round: f32, density: f32) {
    ROUND.store(round.clamp(0.0, 3.0).to_bits(), Ordering::Relaxed);
    DENSITY.store(density.clamp(0.5, 1.6).to_bits(), Ordering::Relaxed);
}
pub fn round_k() -> f32 {
    f32::from_bits(ROUND.load(Ordering::Relaxed))
}
pub fn density() -> f32 {
    f32::from_bits(DENSITY.load(Ordering::Relaxed))
}
/// Скругление `v` пикселей с поправкой макета.
pub fn rr(v: impl Into<f64>) -> CornerRadius {
    CornerRadius::same((v.into() as f32 * round_k()).round().clamp(0.0, 255.0) as u8)
}

static PAL: RwLock<Palette> = RwLock::new(THEMES[0].p);
static ACCENT: AtomicU32 = AtomicU32::new(THEMES[0].p.accent);

fn pal() -> Palette {
    *PAL.read().unwrap()
}

macro_rules! color_fns {
    ($($name:ident => $field:ident),* $(,)?) => {$(
        #[allow(non_snake_case)]
        #[inline]
        pub fn $name() -> Color32 {
            hex(pal().$field)
        }
    )*};
}
color_fns!(BG => bg, CONTENT => content, PANEL => panel, RAISED => raised, HOVER => hover, BORDER => border, INPUT => input,
    TEXT => text, TEXT_DIM => dim, TEXT_FAINT => faint, GREEN => green, YELLOW => yellow, BLUE => blue, DANGER => danger);

pub const ACCENTS: [(&str, u32); 8] = [
    ("TrollLand", 0x2f2cf8),
    ("Красный", 0xe5484d),
    ("Оранжевый", 0xf76b15),
    ("Янтарь", 0xffb224),
    ("Зелёный", 0x46a758),
    ("Бирюза", 0x12a594),
    ("Фиолетовый", 0x8e4ec6),
    ("Розовый", 0xd6409f),
];

#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CustomColors {
	pub accent: Option<u32>,
	pub content: Option<u32>,
	pub sidebar: Option<u32>,
	pub cards: Option<u32>,
	pub controls: Option<u32>,
	pub input: Option<u32>,
	pub text: Option<u32>,
}

impl CustomColors {
	pub fn palette(self, theme: usize, accent: Option<usize>) -> Palette {
		let mut p = THEMES[theme].p;
		p.accent = self.accent.or_else(|| accent.and_then(|i| ACCENTS.get(i)).map(|a| a.1)).unwrap_or(p.accent);
		if let Some(content) = self.content {
			p.content = content;
			let color = hex(content);
			let light = (299 * u32::from(color.r()) + 587 * u32::from(color.g()) + 114 * u32::from(color.b())) >= 145_000;
			if light != p.light {
				let fallback = THEMES[theme_index(if light { "light" } else { "graphite" })].p;
				p = Palette { content, accent: p.accent, ..fallback };
			}
		}
		p.bg = self.sidebar.unwrap_or(p.bg);
		p.panel = self.cards.unwrap_or(p.panel);
		p.raised = self.controls.unwrap_or(p.raised);
		p.input = self.input.unwrap_or(p.input);
		if let Some(text) = self.text {
			p.text = text;
			p.dim = rgb(hex(text).lerp_to_gamma(hex(p.content), 0.28));
			p.faint = rgb(hex(text).lerp_to_gamma(hex(p.content), 0.48));
		}
		if self.cards.is_some() || self.controls.is_some() || self.text.is_some() {
			p.hover = rgb(hex(p.raised).lerp_to_gamma(hex(p.text), 0.12));
			p.border = rgb(hex(p.panel).lerp_to_gamma(hex(p.text), 0.17));
		}
		p
	}
}

pub fn set_theme(theme: usize, accent: Option<usize>, custom: CustomColors) {
	let p = custom.palette(theme, accent);
	*PAL.write().unwrap() = p;
	set_accent(p.accent);
}
pub fn theme_index(id: &str) -> usize {
    THEMES.iter().position(|t| t.id == id).unwrap_or(0)
}
pub fn is_light() -> bool {
    pal().light
}

pub fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}
pub fn rgb(c: Color32) -> u32 {
	u32::from_be_bytes([0, c.r(), c.g(), c.b()])
}
pub fn accent() -> Color32 {
    hex(ACCENT.load(Ordering::Relaxed))
}
pub fn set_accent(v: u32) {
    ACCENT.store(v, Ordering::Relaxed);
}
pub fn lighten(c: Color32, t: f32) -> Color32 {
    c.lerp_to_gamma(Color32::WHITE, t)
}
pub fn darken(c: Color32, t: f32) -> Color32 {
    c.lerp_to_gamma(Color32::BLACK, t)
}
/// Текст поверх акцента: на светлых акцентах (янтарь, Void) — тёмный.
pub fn on_accent() -> Color32 {
    on_color(accent())
}
pub fn on_color(c: Color32) -> Color32 {
    let l = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if l > 170.0 { Color32::from_rgb(0x1a, 0x14, 0x08) } else { Color32::WHITE }
}

pub fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

pub fn apply_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|s| {
        s.visuals = if is_light() { egui::Visuals::light() } else { egui::Visuals::dark() };
        let v = &mut s.visuals;
        v.panel_fill = BG();
        v.window_fill = RAISED();
        v.extreme_bg_color = INPUT();
        v.faint_bg_color = PANEL();
        v.window_stroke = Stroke::new(1.0, BORDER());
        v.window_corner_radius = rr(10);
        v.menu_corner_radius = rr(10);
        v.selection.bg_fill = accent().gamma_multiply(0.45);
        v.selection.stroke = Stroke::new(1.0, TEXT());
        v.hyperlink_color = lighten(accent(), 0.35);
        v.slider_trailing_fill = true;
        let w = &mut v.widgets;
        for st in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            st.corner_radius = rr(8);
            st.bg_stroke = Stroke::NONE;
            st.fg_stroke = Stroke::new(1.0, TEXT());
            st.expansion = 0.0;
        }
        w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER());
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM());
        w.inactive.bg_fill = RAISED();
        w.inactive.weak_bg_fill = RAISED();
        w.hovered.bg_fill = HOVER();
        w.hovered.weak_bg_fill = HOVER();
        w.active.bg_fill = accent();
        w.active.weak_bg_fill = HOVER();
        w.open.bg_fill = HOVER();
        w.open.weak_bg_fill = HOVER();

        s.text_styles.insert(TextStyle::Body, regular(14.0));
        s.text_styles.insert(TextStyle::Button, medium(13.5));
        s.text_styles.insert(TextStyle::Small, regular(12.0));
        s.text_styles.insert(TextStyle::Heading, bold(22.0));
        s.text_styles.insert(TextStyle::Monospace, mono(12.5));

        let d = density();
        s.spacing.item_spacing = egui::vec2(8.0, 8.0) * d;
        s.spacing.button_padding = egui::vec2(12.0, 6.0 * d);
        s.spacing.interact_size.y = (30.0 * d).max(24.0);
        s.spacing.slider_width = 240.0;
        s.interaction.selectable_labels = false;
        s.interaction.tooltip_delay = 0.35;
        s.animation_time = 0.14;
    });
}
