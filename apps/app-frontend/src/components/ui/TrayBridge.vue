<script setup lang="ts">
import { MinimizeIcon, XIcon } from '@modrinth/assets'
import { Button, Checkbox, injectNotificationManager, NewModal } from '@modrinth/ui'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { onMounted, onUnmounted, ref } from 'vue'

import { handleSevereError } from '@/composables/use-error.js'
import { get, run } from '@/helpers/instance'
import { resolveClose } from '@/helpers/tray'

const { handleError } = injectNotificationManager()

const askModal = ref<InstanceType<typeof NewModal>>()
const remember = ref(false)
let unlisteners: UnlistenFn[] = []

/** Launches an instance picked in the tray menu with the same checks as the Play button. */
async function launchFromTray(instanceId: string) {
	const instance = await get(instanceId).catch(handleError)
	if (!instance || instance.quarantined || instance.install_stage !== 'installed') return
	await run(instance.id).catch((err) => handleSevereError(err, { instanceId: instance.id }))
}

async function answer(toTray: boolean) {
	askModal.value?.hide()
	await resolveClose(toTray, remember.value).catch(handleError)
}

onMounted(async () => {
	unlisteners = await Promise.all([
		listen<string>('tray-launch', (event) => launchFromTray(event.payload)),
		listen('tray-ask-close', () => {
			remember.value = false
			askModal.value?.show()
		}),
	])
})

onUnmounted(() => unlisteners.forEach((unlisten) => unlisten()))
</script>

<template>
	<NewModal ref="askModal" header="Закрыть TrollLauncher?" max-width="460px">
		<div class="flex flex-col gap-4">
			<p class="m-0">
				Лаунчер может продолжить работать в трее: запускать сборки из меню иконки и быстро
				открываться.
			</p>
			<Checkbox v-model="remember" label="Запомнить выбор" />
		</div>
		<template #actions>
			<div class="flex justify-end gap-2">
				<Button type="outlined" @click="answer(false)">
					<XIcon />
					Закрыть
				</Button>
				<Button type="colored" color="brand" @click="answer(true)">
					<MinimizeIcon />
					Свернуть в трей
				</Button>
			</div>
		</template>
	</NewModal>
</template>
