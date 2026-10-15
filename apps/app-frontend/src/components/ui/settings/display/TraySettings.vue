<script setup lang="ts">
import { Chips, CopyCode, injectNotificationManager, Slider, Toggle } from '@modrinth/ui'
import { platform } from '@tauri-apps/plugin-os'
import { ref, watch } from 'vue'

import {
	type AutostartMode,
	type CloseAction,
	getTraySettings,
	type LeftClickAction,
	setTraySettings,
} from '@/helpers/tray'

const { handleError } = injectNotificationManager()
const os = platform()
const settings = ref(await getTraySettings())

const closeLabels: Record<CloseAction, string> = {
	tray: 'Свернуть в трей',
	exit: 'Закрыть лаунчер',
	ask: 'Спрашивать',
}
const leftClickLabels: Record<LeftClickAction, string> = {
	toggle: 'Показать / скрыть',
	launch_last: 'Запустить последнюю',
	nothing: 'Ничего',
}
const autostartLabels: Record<AutostartMode, string> = {
	off: 'Выключен',
	minimized: 'Свёрнутым в трей',
	window: 'С окном',
}

const binary = os === 'linux' ? 'trolllauncher' : 'TrollLauncher'

watch(
	settings,
	(value) => {
		setTraySettings(value).catch(handleError)
	},
	{ deep: true },
)
</script>

<template>
	<div class="flex flex-col gap-6">
		<div class="flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">Иконка в трее</h2>
				<p class="m-0 mt-1">Показывать TrollLauncher в области уведомлений у часов.</p>
			</div>
			<Toggle id="tray-enabled" v-model="settings.enabled" />
		</div>

		<template v-if="settings.enabled">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">Кнопка закрытия окна</h2>
				<p class="m-0 mt-1 mb-3">Что делать, когда закрываешь окно лаунчера.</p>
				<Chips
					v-model="settings.close_action"
					:items="['tray', 'exit', 'ask'] as CloseAction[]"
					:format-label="(item) => closeLabels[item]"
					:capitalize="false"
				/>
			</div>

			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">Клик по иконке</h2>
				<p class="m-0 mt-1 mb-3">
					Действие при нажатии левой кнопкой мыши.
					<template v-if="os === 'linux'">
						На Linux многие панели по левому клику сразу открывают меню, тогда настройка не
						действует.
					</template>
				</p>
				<Chips
					v-model="settings.left_click"
					:items="['toggle', 'launch_last', 'nothing'] as LeftClickAction[]"
					:format-label="(item) => leftClickLabels[item]"
					:capitalize="false"
				/>
			</div>

			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">Последние сборки в меню</h2>
				<p class="m-0 mt-1 mb-3">Сколько сборок показывать в меню трея для быстрого запуска.</p>
				<Slider
					:model-value="settings.recent_count"
					:min="0"
					:max="10"
					:step="1"
					aria-label="Последние сборки в меню"
					@update:model-value="settings.recent_count = $event"
				/>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h2 class="m-0 text-lg font-semibold text-contrast">Кнопка «Остановить игру»</h2>
					<p class="m-0 mt-1">Показывать в меню трея запущенные игры с кнопкой остановки.</p>
				</div>
				<Toggle id="tray-stop-game" v-model="settings.show_stop_game" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h2 class="m-0 text-lg font-semibold text-contrast">Время игры в подсказке</h2>
					<p class="m-0 mt-1">Показывать при наведении на иконку, во что и сколько играешь.</p>
				</div>
				<Toggle id="tray-play-time" v-model="settings.show_play_time" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h2 class="m-0 text-lg font-semibold text-contrast">Подсказка при сворачивании</h2>
					<p class="m-0 mt-1">
						Один раз показать уведомление, что лаунчер продолжает работать в трее.
					</p>
				</div>
				<Toggle id="tray-hidden-hint" v-model="settings.hidden_hint" />
			</div>
		</template>

		<div class="border-0 border-t border-solid border-divider pt-6">
			<h2 class="m-0 text-lg font-semibold text-contrast">Автозапуск</h2>
			<p class="m-0 mt-1 mb-3">Запускать TrollLauncher вместе с системой.</p>
			<Chips
				v-model="settings.autostart"
				:items="['off', 'minimized', 'window'] as AutostartMode[]"
				:format-label="(item) => autostartLabels[item]"
					:capitalize="false"
			/>
		</div>

		<div class="border-0 border-t border-solid border-divider pt-6">
			<h2 class="m-0 text-lg font-semibold text-contrast">Горячие клавиши</h2>
			<p class="m-0 mt-1 mb-3">
				Назначь эти команды на любые сочетания клавиш в настройках системы или композитора. Они
				управляют уже запущенным лаунчером и работают и на Wayland.
			</p>
			<div class="flex flex-col gap-2">
				<div class="flex flex-wrap items-center gap-3">
					<span class="w-56">Показать / скрыть окно</span>
					<CopyCode :text="`${binary} --toggle`" />
				</div>
				<div class="flex flex-wrap items-center gap-3">
					<span class="w-56">Запустить последнюю сборку</span>
					<CopyCode :text="`${binary} --launch-last`" />
				</div>
				<div class="flex flex-wrap items-center gap-3">
					<span class="w-56">Открыть окно</span>
					<CopyCode :text="`${binary} --show`" />
				</div>
			</div>
		</div>
	</div>
</template>
