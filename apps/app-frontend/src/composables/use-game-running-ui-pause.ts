import { onScopeDispose, ref, watchEffect } from 'vue'

import { useAppEvent } from '@/composables/use-app-event'
import {
	getPerformanceSettings,
	PERFORMANCE_CHANGED_EVENT,
	type PerformanceSettings,
} from '@/helpers/performance'
import { get_all } from '@/helpers/process.js'
import type { AppEvents } from '@/providers/app-events'

const PAUSED_CLASS = 'troll-game-running'

/**
 * While a game is running and the launcher window is not focused, freezes
 * launcher animations and blur so the webview stops redrawing and leaves the
 * GPU/CPU to Minecraft.
 */
export function useGameRunningUiPause(events: AppEvents) {
	const enabled = ref(true)
	const running = ref(new Set<string>())
	const focused = ref(document.hasFocus())

	getPerformanceSettings()
		.then((settings) => (enabled.value = settings.pause_ui_while_playing))
		.catch(() => {})
	get_all()
		.then((uuids: string[]) => (running.value = new Set(uuids)))
		.catch(() => {})

	useAppEvent(
		'process',
		(payload) => {
			const next = new Set(running.value)
			if (payload.event === 'launched') next.add(payload.uuid)
			else next.delete(payload.uuid)
			running.value = next
		},
		events,
	)

	const onFocus = () => (focused.value = true)
	const onBlur = () => (focused.value = false)
	const onSettings = (event: Event) => {
		enabled.value = (event as CustomEvent<PerformanceSettings>).detail.pause_ui_while_playing
	}
	window.addEventListener('focus', onFocus)
	window.addEventListener('blur', onBlur)
	window.addEventListener(PERFORMANCE_CHANGED_EVENT, onSettings)

	watchEffect(() => {
		const paused = enabled.value && running.value.size > 0 && !focused.value
		document.documentElement.classList.toggle(PAUSED_CLASS, paused)
	})

	onScopeDispose(() => {
		window.removeEventListener('focus', onFocus)
		window.removeEventListener('blur', onBlur)
		window.removeEventListener(PERFORMANCE_CHANGED_EVENT, onSettings)
		document.documentElement.classList.remove(PAUSED_CLASS)
	})
}
