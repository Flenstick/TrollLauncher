<script setup lang="ts">
import { AutoLink, useFormatDateTime } from '@modrinth/ui'

import type { LauncherRelease } from './releases'

defineProps<{
	release: LauncherRelease
}>()

const formatDate = useFormatDateTime({ dateStyle: 'long' })
</script>

<template>
	<AutoLink
		:to="release.url"
		class="group flex w-full flex-col gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-3 transition-all hover:brightness-125 active:scale-[0.99]"
	>
		<img
			v-if="release.thumbnail"
			:src="release.thumbnail"
			alt=""
			class="aspect-video w-full rounded-lg object-cover"
		/>
		<div class="flex items-center gap-2">
			<span class="rounded-full bg-brand-highlight px-2 py-0.5 text-xs font-bold text-brand">
				{{ release.version }}
			</span>
			<span
				v-if="release.prerelease"
				class="rounded-full bg-surface-4 px-2 py-0.5 text-xs font-bold text-secondary"
			>
				Бета
			</span>
		</div>
		<h3 class="m-0 text-base leading-tight text-contrast group-hover:underline">
			{{ release.title }}
		</h3>
		<p v-if="release.summary" class="m-0 whitespace-pre-line text-sm leading-snug text-primary">
			{{ release.summary }}
		</p>
		<div class="text-sm text-secondary">{{ formatDate(release.date) }}</div>
	</AutoLink>
</template>
