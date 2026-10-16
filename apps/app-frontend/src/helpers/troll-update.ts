import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

export interface TrollUpdateInfo {
	current: string
	version: string
	notes: string
	published: string
	critical: boolean
}

export type TrollInstallMethod = 'pacman' | 'replaced'

const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000
const FIRST_CHECK_DELAY_MS = 15 * 1000

/** Shared update state for the title bar button and the update modal. */
export const trollUpdate = {
	available: ref<TrollUpdateInfo | null>(null),
	installing: ref(false),
	installed: ref<TrollInstallMethod | null>(null),
	error: ref(''),
}

export async function checkTrollUpdate(): Promise<TrollUpdateInfo | null> {
	try {
		const info = await invoke<TrollUpdateInfo | null>('plugin:troll-update|check')
		trollUpdate.available.value = info
		return info
	} catch (error) {
		console.warn('Update check failed', error)
		return null
	}
}

export async function installTrollUpdate(): Promise<void> {
	trollUpdate.installing.value = true
	trollUpdate.error.value = ''
	try {
		trollUpdate.installed.value = await invoke<TrollInstallMethod>('plugin:troll-update|install')
	} catch (error) {
		trollUpdate.error.value =
			error && typeof error === 'object' && 'message' in error
				? String(error.message)
				: String(error)
	} finally {
		trollUpdate.installing.value = false
	}
}

export async function restartAfterUpdate(): Promise<void> {
	await invoke('plugin:troll-update|restart')
}

let timer: ReturnType<typeof setInterval> | undefined

/** Checks GitHub shortly after start and then every 6 hours. */
export function startTrollUpdateChecks(): () => void {
	const first = setTimeout(() => void checkTrollUpdate(), FIRST_CHECK_DELAY_MS)
	timer = setInterval(() => void checkTrollUpdate(), CHECK_INTERVAL_MS)
	return () => {
		clearTimeout(first)
		clearInterval(timer)
	}
}
