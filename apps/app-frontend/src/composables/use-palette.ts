import { computed, reactive, ref, watch } from 'vue'

/**
 * Color palettes layered on top of the base light/dark/oled modes.
 * An accent-only palette recolors the brand color; a full palette also
 * replaces surfaces and text and forces its own base mode.
 */
export type PaletteColors = {
	accent: string
	accentContrast?: string
	surfaces?: [string, string, string, string, string]
	textPrimary?: string
	textDefault?: string
	textTertiary?: string
}

export type Palette = {
	id: string
	name: string
	base: 'dark' | 'light' | null
	colors: PaletteColors
}

export const CUSTOM_PALETTE_ID = 'custom'

export const ACCENT_PALETTES: Palette[] = [
	{ id: 'troll-red', name: 'Красный', base: null, colors: { accent: '#e5484d' } },
	{ id: 'orange', name: 'Оранжевый', base: null, colors: { accent: '#f76b15' } },
	{ id: 'amber', name: 'Янтарный', base: null, colors: { accent: '#ffc53d', accentContrast: '#1a1300' } },
	{ id: 'green', name: 'Зелёный', base: null, colors: { accent: '#1bd96a', accentContrast: '#000000' } },
	{ id: 'teal', name: 'Бирюзовый', base: null, colors: { accent: '#12a594' } },
	{ id: 'blue', name: 'Синий', base: null, colors: { accent: '#3e8bff' } },
	{ id: 'violet', name: 'Фиолетовый', base: null, colors: { accent: '#8e4ec6' } },
	{ id: 'pink', name: 'Розовый', base: null, colors: { accent: '#d6409f' } },
]

export const FULL_PALETTES: Palette[] = [
	{
		id: 'blood-night',
		name: 'Кровавая ночь',
		base: 'dark',
		colors: {
			accent: '#ff3b47',
			surfaces: ['#0d0a0b', '#141011', '#1b1516', '#251c1e', '#33272a'],
			textPrimary: '#ffffff',
			textDefault: '#d4c3c5',
			textTertiary: '#9c8789',
		},
	},
	{
		id: 'catppuccin-mocha',
		name: 'Catppuccin Mocha',
		base: 'dark',
		colors: {
			accent: '#cba6f7',
			accentContrast: '#1e1e2e',
			surfaces: ['#11111b', '#181825', '#1e1e2e', '#313244', '#45475a'],
			textPrimary: '#cdd6f4',
			textDefault: '#bac2de',
			textTertiary: '#9399b2',
		},
	},
	{
		id: 'catppuccin-latte',
		name: 'Catppuccin Latte',
		base: 'light',
		colors: {
			accent: '#8839ef',
			surfaces: ['#dce0e8', '#e6e9ef', '#eff1f5', '#ffffff', '#ccd0da'],
			textPrimary: '#4c4f69',
			textDefault: '#5c5f77',
			textTertiary: '#7c7f93',
		},
	},
	{
		id: 'nord',
		name: 'Nord',
		base: 'dark',
		colors: {
			accent: '#88c0d0',
			accentContrast: '#2e3440',
			surfaces: ['#242933', '#2e3440', '#3b4252', '#434c5e', '#4c566a'],
			textPrimary: '#eceff4',
			textDefault: '#d8dee9',
			textTertiary: '#a3abb9',
		},
	},
	{
		id: 'dracula',
		name: 'Dracula',
		base: 'dark',
		colors: {
			accent: '#ff79c6',
			accentContrast: '#282a36',
			surfaces: ['#191a21', '#21222c', '#282a36', '#343746', '#44475a'],
			textPrimary: '#f8f8f2',
			textDefault: '#e2e2dc',
			textTertiary: '#9ea8c7',
		},
	},
	{
		id: 'gruvbox',
		name: 'Gruvbox',
		base: 'dark',
		colors: {
			accent: '#fe8019',
			accentContrast: '#1d2021',
			surfaces: ['#1d2021', '#282828', '#32302f', '#3c3836', '#504945'],
			textPrimary: '#fbf1c7',
			textDefault: '#ebdbb2',
			textTertiary: '#a89984',
		},
	},
	{
		id: 'tokyo-night',
		name: 'Tokyo Night',
		base: 'dark',
		colors: {
			accent: '#7aa2f7',
			accentContrast: '#16161e',
			surfaces: ['#16161e', '#1a1b26', '#1f2335', '#292e42', '#3b4261'],
			textPrimary: '#e0e6ff',
			textDefault: '#c0caf5',
			textTertiary: '#8089b3',
		},
	},
	{
		id: 'rose-pine',
		name: 'Rosé Pine',
		base: 'dark',
		colors: {
			accent: '#ebbcba',
			accentContrast: '#191724',
			surfaces: ['#191724', '#1f1d2e', '#26233a', '#2a2837', '#393552'],
			textPrimary: '#e0def4',
			textDefault: '#c4c1de',
			textTertiary: '#908caa',
		},
	},
	{
		id: 'everforest',
		name: 'Everforest',
		base: 'dark',
		colors: {
			accent: '#a7c080',
			accentContrast: '#232a2e',
			surfaces: ['#1e2326', '#232a2e', '#2d353b', '#343f44', '#3d484d'],
			textPrimary: '#e7e1cf',
			textDefault: '#d3c6aa',
			textTertiary: '#9da9a0',
		},
	},
	{
		id: 'one-dark',
		name: 'One Dark',
		base: 'dark',
		colors: {
			accent: '#61afef',
			accentContrast: '#1e2127',
			surfaces: ['#1b1d23', '#21252b', '#282c34', '#2f343e', '#3e4451'],
			textPrimary: '#f0f2f5',
			textDefault: '#abb2bf',
			textTertiary: '#848b98',
		},
	},
	{
		id: 'solarized-light',
		name: 'Solarized Light',
		base: 'light',
		colors: {
			accent: '#268bd2',
			surfaces: ['#e8e1cc', '#eee8d5', '#f5efdc', '#fdf6e3', '#d9d2bd'],
			textPrimary: '#073642',
			textDefault: '#586e75',
			textTertiary: '#7b8c8f',
		},
	},
]

export const DEFAULT_CUSTOM_COLORS: Required<PaletteColors> = {
	accent: '#e5484d',
	accentContrast: '#ffffff',
	surfaces: ['#0a0a0c', '#0f0f12', '#131316', '#1c1c21', '#2a2a30'],
	textPrimary: '#ffffff',
	textDefault: '#c8c8d0',
	textTertiary: '#8b8b96',
}

const PALETTE_KEY = 'trolllauncher-palette'
const CUSTOM_KEY = 'trolllauncher-custom-palette'

function readStorage<T>(key: string, fallback: T): T {
	try {
		const raw = window.localStorage.getItem(key)
		return raw ? (JSON.parse(raw) as T) : fallback
	} catch {
		return fallback
	}
}

function writeStorage(key: string, value: unknown): void {
	try {
		window.localStorage.setItem(key, JSON.stringify(value))
	} catch {
		// storage blocked or full
	}
}

function hexToRgb(hex: string): [number, number, number] {
	const value = hex.replace('#', '')
	const full = value.length === 3 ? [...value].map((c) => c + c).join('') : value
	const int = Number.parseInt(full.slice(0, 6), 16)
	return [(int >> 16) & 255, (int >> 8) & 255, int & 255]
}

function rgba(hex: string, alpha: number): string {
	const [r, g, b] = hexToRgb(hex)
	return `rgba(${r}, ${g}, ${b}, ${alpha})`
}

function mix(hex: string, target: string, amount: number): string {
	const a = hexToRgb(hex)
	const b = hexToRgb(target)
	const channel = (i: number) =>
		Math.round(a[i] + (b[i] - a[i]) * amount)
			.toString(16)
			.padStart(2, '0')
	return `#${channel(0)}${channel(1)}${channel(2)}`
}

/** Picks black or white text for the given background by relative luminance. */
export function contrastText(hex: string): string {
	const [r, g, b] = hexToRgb(hex).map((c) => {
		const s = c / 255
		return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
	})
	return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.4 ? '#000000' : '#ffffff'
}

export function paletteVars(palette: Palette): Record<string, string> {
	const { colors } = palette
	const accent = colors.accent
	const vars: Record<string, string> = {
		'--color-brand': accent,
		'--color-brand-highlight': rgba(accent, 0.25),
		'--color-brand-shadow': rgba(accent, 0.7),
		'--color-accent-contrast': colors.accentContrast ?? contrastText(accent),
		'--loading-bar-gradient': `linear-gradient(to right, ${accent} 0%, ${mix(accent, '#ffffff', 0.4)} 100%)`,
	}

	if (colors.surfaces) {
		const [s1, s2, s3, s4, s5] = colors.surfaces
		Object.assign(vars, {
			'--surface-1': s1,
			'--surface-1-5': mix(s1, s2, 0.5),
			'--surface-2': s2,
			'--surface-2-5': mix(s2, s3, 0.5),
			'--surface-3': s3,
			'--surface-4': s4,
			'--surface-5': s5,
			'--color-scrollbar': s5,
			'--brand-gradient-bg': `linear-gradient(0deg, ${rgba(accent, 0.04)} 0%, ${rgba(accent, 0.1)} 100%)`,
			'--brand-gradient-strong-bg': `linear-gradient(270deg, ${s1} 10%, ${mix(s2, accent, 0.08)} 100%)`,
			'--brand-gradient-border': rgba(accent, 0.12),
			'--brand-gradient-fade-out-color': `linear-gradient(to bottom, ${rgba(s2, 0)}, ${s2} 80%)`,
			'--splash-tint-top': rgba(accent, 0.2),
			'--splash-overlay': rgba(s1, 0.64),
		})
	}
	if (colors.textPrimary) vars['--color-text-primary'] = colors.textPrimary
	if (colors.textDefault) vars['--color-text-default'] = colors.textDefault
	if (colors.textTertiary) vars['--color-text-tertiary'] = colors.textTertiary

	return vars
}

const selectedId = ref<string>(readStorage(PALETTE_KEY, 'troll-red'))
const previewId = ref<string | null>(null)
const customColors = ref<Required<PaletteColors>>({
	...DEFAULT_CUSTOM_COLORS,
	...readStorage<Partial<PaletteColors>>(CUSTOM_KEY, {}),
})
const customBase = ref<'dark' | 'light'>(readStorage(`${CUSTOM_KEY}-base`, 'dark'))

const customPalette = computed<Palette>(() => ({
	id: CUSTOM_PALETTE_ID,
	name: 'Своя тема',
	base: customBase.value,
	colors: customColors.value,
}))

const allPalettes = computed<Palette[]>(() => [
	...ACCENT_PALETTES,
	...FULL_PALETTES,
	customPalette.value,
])

const active = computed<Palette>(() => {
	const id = previewId.value ?? selectedId.value
	return allPalettes.value.find((palette) => palette.id === id) ?? ACCENT_PALETTES[0]
})

/** Base mode a full palette forces, or null to keep the user's light/dark choice. */
const forcedBase = computed(() => active.value.base)

let styleElement: HTMLStyleElement | null = null

watch(
	active,
	(palette) => {
		styleElement ??= document.head.appendChild(document.createElement('style'))
		styleElement.id = 'trolllauncher-palette'
		const body = Object.entries(paletteVars(palette))
			.map(([name, value]) => `\t${name}: ${value};`)
			.join('\n')
		styleElement.textContent = `:root:root:root {\n${body}\n}`
		document.documentElement.dataset.palette = palette.id
	},
	{ immediate: true, deep: true },
)

watch(selectedId, (id) => writeStorage(PALETTE_KEY, id))
watch(customColors, (colors) => writeStorage(CUSTOM_KEY, colors), { deep: true })
watch(customBase, (base) => writeStorage(`${CUSTOM_KEY}-base`, base))

function resetCustom(): void {
	customColors.value = { ...DEFAULT_CUSTOM_COLORS, surfaces: [...DEFAULT_CUSTOM_COLORS.surfaces] }
	customBase.value = 'dark'
}

const palette = reactive({
	selected: selectedId,
	preview: previewId,
	active,
	forcedBase,
	custom: customColors,
	customBase,
	accentPalettes: ACCENT_PALETTES,
	fullPalettes: FULL_PALETTES,
	resetCustom,
})

export function usePalette() {
	return palette
}
