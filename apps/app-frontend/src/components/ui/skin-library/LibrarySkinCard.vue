<script setup lang="ts">
import { CheckIcon, StarIcon } from '@modrinth/assets'
import { useIntersectionObserver } from '@vueuse/core'
import { computed, ref, watch } from 'vue'

import { acquireSkinPreview, getSkinPreviewKey } from '@/helpers/rendering/skin-previews'
import type { Cape, Skin } from '@/helpers/skins'

const props = defineProps<{
	skin: Skin
	name: string
	capes: Cape[]
	selected: boolean
	equipped: boolean
	favorite: boolean
}>()

const root = ref<HTMLElement>()
const visible = ref(false)
const url = ref<string>()
const key = computed(() => getSkinPreviewKey(props.skin))

const { stop } = useIntersectionObserver(
	root,
	([entry]) => {
		if (!entry?.isIntersecting) return
		visible.value = true
		stop()
	},
	{ rootMargin: '300px' },
)

watch(
	[visible, key, () => props.capes],
	([isVisible], _, onCleanup) => {
		url.value = undefined
		if (!isVisible) return
		const preview = acquireSkinPreview(props.skin, props.capes)
		let active = true
		onCleanup(() => {
			active = false
			preview.release()
		})
		void preview.promise
			.then((result) => {
				if (active) url.value = result?.forwards
			})
			.catch((error) => console.warn('Could not render skin preview', error))
	},
	{ immediate: true },
)
</script>

<template>
	<button
		ref="root"
		type="button"
		class="skin-card group relative flex flex-col items-center gap-1 rounded-2xl border-2 border-solid p-2 pb-2.5 text-left transition-colors"
		:class="
			selected ? 'border-brand bg-surface-3' : 'border-transparent bg-surface-2 hover:bg-surface-3'
		"
	>
		<div class="flex aspect-[3/4] w-full items-center justify-center">
			<img
				v-if="url"
				:src="url"
				:alt="name"
				class="h-full w-full object-contain [image-rendering:auto]"
				draggable="false"
			/>
			<div v-else class="h-3/4 w-1/2 animate-pulse rounded-xl bg-surface-4" />
		</div>
		<span class="w-full truncate text-center text-sm font-semibold text-contrast">{{ name }}</span>
		<span
			v-if="equipped"
			class="absolute left-2 top-2 flex items-center gap-1 rounded-full bg-brand px-2 py-0.5 text-xs font-bold text-[var(--color-accent-contrast)]"
		>
			<CheckIcon class="size-3" /> Надет
		</span>
		<StarIcon
			v-if="favorite"
			class="absolute right-2 top-2 size-4 fill-current text-orange"
			aria-label="Избранный"
		/>
	</button>
</template>
