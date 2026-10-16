<script setup lang="ts">
import { DownloadIcon, RefreshCwIcon, SpinnerIcon } from '@modrinth/assets'
import { Admonition, Button, NewModal } from '@modrinth/ui'
import { computed, ref } from 'vue'

import { get_all as getRunningProcesses } from '@/helpers/process'
import { installTrollUpdate, restartAfterUpdate, trollUpdate } from '@/helpers/troll-update'

const { available, installing, installed, error } = trollUpdate
const modal = ref<InstanceType<typeof NewModal>>()
const gameRunning = ref(false)

const label = computed(() => {
	if (installed.value) return 'Перезапустить'
	if (installing.value) return 'Обновляю…'
	return `Обновить до ${available.value?.version}`
})

const publishedDate = computed(() =>
	available.value?.published
		? new Date(available.value.published).toLocaleDateString('ru-RU', {
				day: 'numeric',
				month: 'long',
				year: 'numeric',
			})
		: '',
)

async function open() {
	gameRunning.value = ((await getRunningProcesses().catch(() => [])) as unknown[]).length > 0
	modal.value?.show()
}

async function act() {
	if (installed.value) {
		await restartAfterUpdate()
		return
	}
	gameRunning.value = ((await getRunningProcesses().catch(() => [])) as unknown[]).length > 0
	if (gameRunning.value) return
	await installTrollUpdate()
}
</script>

<template>
	<template v-if="available">
		<Button
			type="outlined"
			color="brand"
			class="!h-[34px] text-sm"
			:class="{ 'animate-pulse': available.critical && !installed }"
			@click="open"
		>
			<RefreshCwIcon v-if="installed" />
			<SpinnerIcon v-else-if="installing" class="animate-spin" />
			<DownloadIcon v-else />
			{{ label }}
		</Button>

		<NewModal ref="modal" :header="`TrollLauncher ${available.version}`" max-width="560px">
			<div class="flex flex-col gap-4">
				<p class="m-0 text-secondary">
					Сейчас установлена {{ available.current }}<template v-if="publishedDate">
						· релиз от {{ publishedDate }}</template
					>
				</p>
				<Admonition v-if="available.critical" type="warning" header="Важное обновление">
					Это обновление исправляет серьёзную проблему, лучше установить его сразу.
				</Admonition>
				<div
					v-if="available.notes"
					class="max-h-[45vh] overflow-y-auto whitespace-pre-wrap rounded-2xl bg-surface-2 p-4 text-sm leading-relaxed"
				>
					{{ available.notes }}
				</div>
				<Admonition v-if="gameRunning && !installed" type="info" header="Игра запущена">
					Закрой Minecraft, чтобы обновить лаунчер: во время игры обновление не ставится.
				</Admonition>
				<Admonition v-if="error" type="critical" header="Не получилось обновить">
					<span class="whitespace-pre-wrap">{{ error }}</span>
				</Admonition>
				<Admonition v-if="installed" type="success" header="Обновление установлено">
					Перезапусти лаунчер, чтобы перейти на {{ available.version }}.
				</Admonition>
				<p v-else-if="!installing" class="m-0 text-sm text-secondary">
					Если лаунчер установлен пакетом, система попросит пароль для pacman.
				</p>
			</div>
			<template #actions>
				<div class="flex justify-end gap-2">
					<Button @click="modal?.hide()">Позже</Button>
					<Button
						type="colored"
						color="brand"
						:disabled="installing || (gameRunning && !installed)"
						@click="act"
					>
						<RefreshCwIcon v-if="installed" />
						<SpinnerIcon v-else-if="installing" class="animate-spin" />
						<DownloadIcon v-else />
						{{ installed ? 'Перезапустить' : installing ? 'Обновляю…' : 'Обновить' }}
					</Button>
				</div>
			</template>
		</NewModal>
	</template>
</template>
