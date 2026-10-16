<script setup lang="ts">
import { SpinnerIcon } from '@modrinth/assets'
import { injectNotificationManager } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import { injectMinecraftAccounts } from '@/features/minecraft-accounts/context'
import { getPlayerHeadUrl } from '@/helpers/rendering/player-head'
import {
	asSkin,
	equipLibrarySkin,
	getCurrentAccount,
	getLibrary,
	type LibraryAccount,
	SKIN_LIBRARY_CHANGED,
	type SkinLibrary,
} from '@/helpers/skin-library'

const MAX = 6

const { addNotification, handleError } = injectNotificationManager()
const accounts = injectMinecraftAccounts()

const library = ref<SkinLibrary | null>(null)
const account = ref<LibraryAccount | null>(null)
const heads = ref<Record<string, string>>({})
const busy = ref<string | null>(null)

/** Favorites first; without favorites — the newest skins. */
const skins = computed(() => {
	const all = library.value?.skins ?? []
	const favorites = all.filter((skin) => skin.favorite)
	return (favorites.length ? favorites : [...all].sort((a, b) => b.added - a.added)).slice(0, MAX)
})
const equippedId = computed(() =>
	account.value ? library.value?.equipped[account.value.id] : undefined,
)

async function load() {
	try {
		;[library.value, account.value] = await Promise.all([getLibrary(), getCurrentAccount()])
	} catch {
		library.value = null
	}
}

watch(
	skins,
	async (list) => {
		for (const skin of list) {
			if (heads.value[skin.id]) continue
			const url = await getPlayerHeadUrl(asSkin(skin)).catch(() => undefined)
			if (url) heads.value = { ...heads.value, [skin.id]: url }
		}
	},
	{ immediate: true },
)

async function equip(id: string) {
	const skin = library.value?.skins.find((item) => item.id === id)
	await load()
	if (!skin || !account.value || busy.value) return
	busy.value = id
	try {
		const result = await equipLibrarySkin(skin, account.value)
		if (result.status === 'done') {
			await accounts.value?.setEquippedSkin(asSkin(skin))
			await accounts.value?.refreshValues()
		} else if (result.status === 'need_login') {
			addNotification({
				title: 'Войди на ely.by',
				text: 'Открыл окно ely.by: войди и нажми на скин ещё раз.',
				type: 'info',
			})
		} else if (result.status === 'manual') {
			addNotification({ title: 'Ely.by не принял скин', text: result.message, type: 'warn' })
		}
	} catch (error) {
		handleError(error as Error)
	} finally {
		busy.value = null
	}
}

const onChanged = () => void load()
onMounted(() => {
	void load()
	window.addEventListener(SKIN_LIBRARY_CHANGED, onChanged)
})
onUnmounted(() => window.removeEventListener(SKIN_LIBRARY_CHANGED, onChanged))
defineExpose({ reload: load })
</script>

<template>
	<div v-if="skins.length > 1" class="mt-3 flex flex-wrap gap-1.5" aria-label="Быстрая смена скина">
		<button
			v-for="skin in skins"
			:key="skin.id"
			v-tooltip="skin.id === equippedId ? `${skin.name} — надет` : `Надеть «${skin.name}»`"
			type="button"
			class="relative size-9 overflow-hidden rounded-lg border-2 border-solid bg-surface-3 p-0 transition-transform hover:scale-110"
			:class="skin.id === equippedId ? 'border-brand' : 'border-transparent'"
			:disabled="!!busy"
			@click="skin.id !== equippedId && equip(skin.id)"
		>
			<img
				v-if="heads[skin.id]"
				:src="heads[skin.id]"
				:alt="skin.name"
				class="size-full [image-rendering:pixelated]"
			/>
			<div
				v-if="busy === skin.id"
				class="absolute inset-0 flex items-center justify-center bg-black/50"
			>
				<SpinnerIcon class="size-4 animate-spin text-white" />
			</div>
		</button>
	</div>
</template>
