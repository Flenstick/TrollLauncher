import { invoke } from '@tauri-apps/api/core'

export type JvmPreset = 'off' | 'g1' | 'zgc'

export type PerformanceSettings = {
	jvm_preset: JvmPreset
	full_heap_at_start: boolean
	gamemode: boolean
	mangohud: boolean
	pause_ui_while_playing: boolean
}

export type PerformanceTools = {
	gamemode: boolean
	mangohud: boolean
}

export const PERFORMANCE_CHANGED_EVENT = 'troll-performance-changed'

export async function getPerformanceSettings(): Promise<PerformanceSettings> {
	return await invoke('plugin:troll-perf|get_settings')
}

export async function setPerformanceSettings(settings: PerformanceSettings): Promise<void> {
	await invoke('plugin:troll-perf|set_settings', { settings })
	window.dispatchEvent(new CustomEvent(PERFORMANCE_CHANGED_EVENT, { detail: settings }))
}

export async function getPerformanceTools(): Promise<PerformanceTools> {
	return await invoke('plugin:troll-perf|get_tools')
}
