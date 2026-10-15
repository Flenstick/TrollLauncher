import { invoke } from '@tauri-apps/api/core'

export type CloseAction = 'tray' | 'exit' | 'ask'
export type LeftClickAction = 'toggle' | 'launch_last' | 'nothing'
export type AutostartMode = 'off' | 'minimized' | 'window'

export type TraySettings = {
	enabled: boolean
	close_action: CloseAction
	left_click: LeftClickAction
	recent_count: number
	show_stop_game: boolean
	show_play_time: boolean
	autostart: AutostartMode
	hidden_hint: boolean
	hidden_hint_shown: boolean
}

export async function getTraySettings(): Promise<TraySettings> {
	return await invoke('plugin:troll-tray|get_settings')
}

export async function setTraySettings(settings: TraySettings): Promise<void> {
	await invoke('plugin:troll-tray|set_settings', { settings })
}

/** Answer to the "close or minimize to tray?" question. */
export async function resolveClose(toTray: boolean, remember: boolean): Promise<void> {
	await invoke('plugin:troll-tray|resolve_close', { toTray, remember })
}

export async function refreshTray(): Promise<void> {
	await invoke('plugin:troll-tray|refresh_tray')
}
