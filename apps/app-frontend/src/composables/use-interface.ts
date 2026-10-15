import '@fontsource-variable/golos-text'

import { getCurrentWebview } from '@tauri-apps/api/webview'
import { reactive, ref, watch } from 'vue'

export type FontChoice = 'inter' | 'golos' | 'system' | 'custom'

export const SCALE_MIN = 80
export const SCALE_MAX = 150
export const SCALE_STEP = 5

const FALLBACK_STACK =
	'-apple-system, BlinkMacSystemFont, Segoe UI, Oxygen, Ubuntu, Roboto, Cantarell, Fira Sans, Droid Sans, Helvetica Neue, sans-serif'

export const FONT_STACKS: Record<Exclude<FontChoice, 'custom'>, string> = {
	inter: `Inter, ${FALLBACK_STACK}`,
	golos: `'Golos Text Variable', 'Golos Text', ${FALLBACK_STACK}`,
	system: `system-ui, ${FALLBACK_STACK}`,
}

const SCALE_KEY = 'trolllauncher-ui-scale'
const FONT_KEY = 'trolllauncher-ui-font'
const CUSTOM_FONT_KEY = 'trolllauncher-ui-custom-font'
const HOTKEYS_KEY = 'trolllauncher-ui-zoom-hotkeys'

function read<T>(key: string, fallback: T): T {
	try {
		const raw = window.localStorage.getItem(key)
		return raw === null ? fallback : (JSON.parse(raw) as T)
	} catch {
		return fallback
	}
}

function write(key: string, value: unknown): void {
	try {
		window.localStorage.setItem(key, JSON.stringify(value))
	} catch {
		// storage blocked or full
	}
}

function clampScale(value: number): number {
	const stepped = Math.round(value / SCALE_STEP) * SCALE_STEP
	return Math.min(SCALE_MAX, Math.max(SCALE_MIN, stepped))
}

const scale = ref(clampScale(read(SCALE_KEY, 100)))
const font = ref<FontChoice>(read(FONT_KEY, 'inter'))
const customFont = ref(read(CUSTOM_FONT_KEY, ''))
const zoomHotkeys = ref(read(HOTKEYS_KEY, true))

/** Font stack for a choice; a custom family falls back to Inter when not installed. */
export function fontStack(choice: FontChoice, custom = customFont.value): string {
	if (choice !== 'custom') return FONT_STACKS[choice]
	const family = custom.trim().replace(/["\\;{}]/g, '')
	return family ? `"${family}", ${FONT_STACKS.inter}` : FONT_STACKS.inter
}

let fontStyle: HTMLStyleElement | null = null

watch(
	[font, customFont],
	() => {
		fontStyle ??= document.head.appendChild(document.createElement('style'))
		fontStyle.id = 'trolllauncher-font'
		fontStyle.textContent = `:root body { --font-standard: ${fontStack(font.value)}; }`
	},
	{ immediate: true },
)

watch(
	scale,
	(value) => {
		getCurrentWebview()
			.setZoom(value / 100)
			.catch((error) => console.error('Failed to set interface scale', error))
	},
	{ immediate: true },
)

watch(scale, (value) => write(SCALE_KEY, value))
watch(font, (value) => write(FONT_KEY, value))
watch(customFont, (value) => write(CUSTOM_FONT_KEY, value))
watch(zoomHotkeys, (value) => write(HOTKEYS_KEY, value))

window.addEventListener('keydown', (event) => {
	if (!zoomHotkeys.value || !(event.ctrlKey || event.metaKey) || event.altKey) return
	if (event.key === '=' || event.key === '+') {
		scale.value = clampScale(scale.value + SCALE_STEP * 2)
	} else if (event.key === '-' || event.key === '_') {
		scale.value = clampScale(scale.value - SCALE_STEP * 2)
	} else if (event.key === '0') {
		scale.value = 100
	} else {
		return
	}
	event.preventDefault()
})

const ui = reactive({
	scale,
	font,
	customFont,
	zoomHotkeys,
	setScale: (value: number) => {
		scale.value = clampScale(value)
	},
})

export function useInterface() {
	return ui
}
