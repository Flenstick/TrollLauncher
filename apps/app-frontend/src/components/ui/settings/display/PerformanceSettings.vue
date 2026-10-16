<script setup lang="ts">
import { Chips, injectNotificationManager, Toggle } from '@modrinth/ui'
import { platform } from '@tauri-apps/plugin-os'
import { ref, watch } from 'vue'

import {
	getPerformanceSettings,
	getPerformanceTools,
	type JvmPreset,
	setPerformanceSettings,
} from '@/helpers/performance'

const { handleError } = injectNotificationManager()
const isLinux = platform() === 'linux'
const settings = ref(await getPerformanceSettings())
const tools = await getPerformanceTools()

const presetLabels: Record<JvmPreset, string> = {
	g1: 'G1 (как в Prism)',
	zgc: 'ZGC без фризов',
	off: 'Выключены',
}
const presetHints: Record<JvmPreset, string> = {
	g1: 'Флаги Aikar для сборщика мусора G1: те же, что стоят в Prism-сборке TrollLand. Стабильный FPS и меньше подлагиваний от сборки мусора.',
	zgc: 'Поколенческий ZGC: паузы сборщика меньше миллисекунды, почти нет микрофризов в больших модпаках. Ест немного больше памяти и процессора. Нужна Java 17+, иначе включится G1.',
	off: 'Только аргументы Java, которые ты указал сам.',
}

watch(
	settings,
	(value) => {
		setPerformanceSettings(value).catch(handleError)
	},
	{ deep: true },
)
</script>

<template>
	<div class="flex flex-col gap-6">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">Оптимизация Java</h2>
			<p class="m-0 mt-1 mb-3">
				Настройки сборщика мусора для каждого запуска. Если в аргументах Java сборки указан свой
				сборщик (<code>-XX:+Use…GC</code>), набор не добавляется, а твои флаги всегда важнее.
			</p>
			<Chips
				v-model="settings.jvm_preset"
				:items="['g1', 'zgc', 'off'] as JvmPreset[]"
				:format-label="(item) => presetLabels[item]"
				:capitalize="false"
			/>
			<p class="m-0 mt-3 text-secondary">{{ presetHints[settings.jvm_preset] }}</p>
		</div>

		<div class="flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">Сразу занимать всю память</h2>
				<p class="m-0 mt-1">
					Выделять всю отведённую память при старте (<code>-Xms</code> = <code>-Xmx</code>). Чуть
					ровнее FPS, но игра дольше запускается и сразу забирает всю RAM. Выключено — стартует с 2
					ГБ.
				</p>
			</div>
			<Toggle id="perf-full-heap" v-model="settings.full_heap_at_start" />
		</div>

		<template v-if="isLinux">
			<div class="flex items-center justify-between gap-4">
				<div>
					<h2 class="m-0 text-lg font-semibold text-contrast">Feral GameMode</h2>
					<p class="m-0 mt-1">
						Запускать игру через <code>gamemoderun</code>: процессор в режиме производительности,
						повышенный приоритет игры.
						<template v-if="!tools.gamemode">
							<br />GameMode не установлен (пакет <code>gamemode</code>).
						</template>
					</p>
				</div>
				<Toggle id="perf-gamemode" v-model="settings.gamemode" :disabled="!tools.gamemode" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h2 class="m-0 text-lg font-semibold text-contrast">MangoHud</h2>
					<p class="m-0 mt-1">
						Оверлей с FPS, временем кадра и загрузкой железа поверх игры.
						<template v-if="!tools.mangohud">
							<br />MangoHud не установлен (пакет <code>mangohud</code>).
						</template>
					</p>
				</div>
				<Toggle id="perf-mangohud" v-model="settings.mangohud" :disabled="!tools.mangohud" />
			</div>
		</template>

		<div class="flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">Замораживать лаунчер во время игры</h2>
				<p class="m-0 mt-1">
					Пока игра запущена, а окно лаунчера не в фокусе, останавливать анимации и размытие, чтобы
					лаунчер не отнимал видеокарту у Minecraft.
				</p>
			</div>
			<Toggle id="perf-pause-ui" v-model="settings.pause_ui_while_playing" />
		</div>
	</div>
</template>
