import { invoke } from '@tauri-apps/api/core'

import { get_default_user, users } from '@/helpers/auth'
import {
	add_and_equip_custom_skin,
	type Cape,
	flush_pending_skin_change,
	get_available_capes,
	type Skin,
} from '@/helpers/skins'

export type LibraryVariant = 'classic' | 'slim'

export interface LibrarySkin {
	id: string
	name: string
	variant: LibraryVariant
	folder: string | null
	favorite: boolean
	/** `mojang:<uuid>`, `optifine`, `elyby` or null. */
	cape: string | null
	source: string
	added: number
	/** data:image/png;base64,… */
	texture: string
}

export interface SkinLibrary {
	folders: string[]
	skins: LibrarySkin[]
	equipped: Record<string, string>
}

export interface FetchedSkin {
	texture: number[]
	variant: LibraryVariant
	name: string
}

export interface ExtraCapes {
	optifine: string | null
	elyby: string | null
}

export interface LibraryAccount {
	id: string
	name: string
	provider: 'microsoft' | 'elyby'
}

export type ElybyApplyResult =
	| { status: 'done' }
	| { status: 'need_login' }
	| { status: 'manual'; message: string }
	| { status: 'cancelled' }

export const SKIN_LIBRARY_CHANGED = 'troll-skin-library-changed'

function changed() {
	window.dispatchEvent(new CustomEvent(SKIN_LIBRARY_CHANGED))
}

export async function getLibrary(): Promise<SkinLibrary> {
	return await invoke('plugin:troll-skins|get_library')
}

export async function addSkin(
	texture: Uint8Array | number[],
	name: string,
	options: { variant?: LibraryVariant; folder?: string | null; source: string },
): Promise<LibrarySkin> {
	const skin = await invoke<LibrarySkin>('plugin:troll-skins|add_skin', {
		texture: Array.from(texture),
		name,
		variant: options.variant ?? null,
		folder: options.folder ?? null,
		source: options.source,
	})
	changed()
	return skin
}

export async function updateSkin(skin: LibrarySkin): Promise<void> {
	await invoke('plugin:troll-skins|update_skin', { skin: { ...skin, texture: '' } })
	changed()
}

export async function removeSkin(id: string): Promise<void> {
	await invoke('plugin:troll-skins|remove_skin', { id })
	changed()
}

export async function setFolders(
	folders: string[],
	renamed: Record<string, string> = {},
): Promise<void> {
	await invoke('plugin:troll-skins|set_folders', { folders, renamed })
	changed()
}

export async function setEquipped(account: string, id: string): Promise<void> {
	await invoke('plugin:troll-skins|set_equipped', { account, id })
	changed()
}

export async function exportSkin(id: string, path: string): Promise<void> {
	await invoke('plugin:troll-skins|export_skin', { id, path })
}

export async function fetchByNick(nick: string, source: 'mojang' | 'elyby'): Promise<FetchedSkin> {
	return await invoke('plugin:troll-skins|fetch_by_nick', { nick, source })
}

export async function fetchUrl(url: string): Promise<FetchedSkin> {
	return await invoke('plugin:troll-skins|fetch_url', { url })
}

const capeCache = new Map<string, Promise<ExtraCapes>>()

/** OptiFine and Ely.by capes of a nickname (cached for the session). */
export function getExtraCapes(nick: string): Promise<ExtraCapes> {
	const key = nick.toLowerCase()
	let cached = capeCache.get(key)
	if (!cached) {
		cached = invoke<ExtraCapes>('plugin:troll-skins|extra_capes', { nick }).catch(() => ({
			optifine: null,
			elyby: null,
		}))
		capeCache.set(key, cached)
	}
	return cached
}

export async function openElyby(page?: 'login'): Promise<void> {
	await invoke('plugin:troll-skins|elyby_open', { page: page ?? null })
}

export async function getCurrentAccount(): Promise<LibraryAccount | null> {
	const [id, list] = await Promise.all([get_default_user(), users()])
	const account = (
		list as Array<{ provider?: 'microsoft' | 'elyby'; profile: { id: string; name: string } }>
	).find((item) => item.profile.id === id)
	if (!account) return null
	return {
		id: account.profile.id,
		name: account.profile.name,
		provider: account.provider ?? 'microsoft',
	}
}

/** A library skin shaped like the launcher's own skin type (for renders and heads). */
export function asSkin(skin: LibrarySkin, mojangCapeId?: string): Skin {
	return {
		texture_key: `library-${skin.id}`,
		name: skin.name,
		variant: skin.variant === 'slim' ? 'SLIM' : 'CLASSIC',
		cape_id:
			mojangCapeId ?? (skin.cape && !skin.cape.startsWith('mojang:') ? skin.cape : undefined),
		texture: skin.texture,
		source: 'custom',
		is_equipped: false,
	}
}

/** Capes the renderer can use: Mojang capes of the account + OptiFine / Ely.by ones. */
export function rendererCapes(mojang: Cape[], extra: ExtraCapes): Cape[] {
	const capes = [...mojang]
	if (extra.optifine)
		capes.push({ id: 'optifine', name: 'OptiFine', texture: extra.optifine, is_equipped: false })
	if (extra.elyby)
		capes.push({ id: 'elyby', name: 'Ely.by', texture: extra.elyby, is_equipped: false })
	return capes
}

function base64ToBytes(dataUrl: string): Uint8Array {
	const binary = atob(dataUrl.slice(dataUrl.indexOf(',') + 1))
	const bytes = new Uint8Array(binary.length)
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
	return bytes
}

/**
 * Puts a library skin on the current account. Microsoft: through the Minecraft
 * API (with the chosen Mojang cape). Ely.by: through the ely.by website window.
 */
export async function equipLibrarySkin(
	skin: LibrarySkin,
	account: LibraryAccount,
): Promise<ElybyApplyResult> {
	if (account.provider === 'elyby') {
		const result = await invoke<ElybyApplyResult>('plugin:troll-skins|elyby_apply', {
			id: skin.id,
			slim: skin.variant === 'slim',
		})
		if (result.status === 'done') {
			await invoke('plugin:troll-skins|set_equipped', { account: account.id, id: skin.id })
			changed()
		}
		return result
	}

	let cape: Cape | undefined
	if (skin.cape?.startsWith('mojang:')) {
		const capes = await get_available_capes()
		cape = capes.find((item) => `mojang:${item.id}` === skin.cape)
	}
	await add_and_equip_custom_skin(
		base64ToBytes(skin.texture),
		skin.variant === 'slim' ? 'SLIM' : 'CLASSIC',
		cape,
	)
	await flush_pending_skin_change()
	await invoke('plugin:troll-skins|set_equipped', { account: account.id, id: skin.id })
	changed()
	return { status: 'done' }
}
