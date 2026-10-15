<script setup lang="ts">
import { RadioButtonCheckedIcon, RadioButtonIcon, UndoIcon } from '@modrinth/assets'
import { Button, Chips, Input, Slider, Toggle } from '@modrinth/ui'

import {
	type FontChoice,
	fontStack,
	SCALE_MAX,
	SCALE_MIN,
	SCALE_STEP,
	useInterface,
} from '@/composables/use-interface.ts'
import { config } from '@/config'
import { type NewsSource, newsSource } from '@/features/news/releases'

const ui = useInterface()

const newsLabels: Record<NewsSource, string> = {
	releases: 'Обновления лаунчера',
	modrinth: 'Новости Modrinth',
	both: 'Всё вместе',
	none: 'Скрыть',
}

const fontOptions: Array<{ id: FontChoice; name: string; hint: string }> = [
	{ id: 'inter', name: 'Inter', hint: 'Стандартный шрифт Modrinth' },
	{ id: 'golos', name: 'Golos Text', hint: 'Мягче, хорошо читается кириллица' },
	{ id: 'system', name: 'Системный', hint: 'Шрифт вашей системы' },
	{ id: 'custom', name: 'Свой', hint: 'Любой установленный шрифт' },
]
</script>

<template>
	<section class="mt-8 border-0 border-t border-solid border-divider pt-6">
		<div class="flex items-start justify-between gap-4">
			<div class="flex flex-col gap-1">
				<h2 class="m-0 text-xl font-semibold text-contrast">Масштаб интерфейса</h2>
				<p class="m-0 text-secondary">
					Размер всего интерфейса лаунчера. Также работают Ctrl + «+», Ctrl + «−» и Ctrl + 0.
				</p>
			</div>
			<Button v-if="ui.scale !== 100" type="quiet" @click="ui.setScale(100)">
				<UndoIcon />
				100%
			</Button>
		</div>
		<Slider
			class="mt-4"
			:model-value="ui.scale"
			:min="SCALE_MIN"
			:max="SCALE_MAX"
			:step="SCALE_STEP"
			:snap-points="[100]"
			:snap-range="4"
			unit="%"
			aria-label="Масштаб интерфейса"
			@update:model-value="ui.setScale($event)"
		/>
		<div class="mt-4 flex items-center justify-between gap-4">
			<div>
				<h3 class="m-0 text-base font-semibold text-contrast">Горячие клавиши масштаба</h3>
				<p class="m-0 mt-1 text-secondary">Менять масштаб сочетаниями Ctrl + «+» и Ctrl + «−».</p>
			</div>
			<Toggle id="zoom-hotkeys" v-model="ui.zoomHotkeys" />
		</div>
	</section>

	<section class="mt-8 border-0 border-t border-solid border-divider pt-6">
		<div class="flex flex-col gap-1">
			<h2 class="m-0 text-xl font-semibold text-contrast">Шрифт интерфейса</h2>
			<p class="m-0 text-secondary">Шрифт всех надписей лаунчера.</p>
		</div>
		<div class="font-grid mt-4" role="group" aria-label="Шрифт интерфейса">
			<button
				v-for="option in fontOptions"
				:key="option.id"
				type="button"
				class="preview-radio button-base"
				:class="{ selected: ui.font === option.id }"
				:aria-pressed="ui.font === option.id"
				@click="ui.font = option.id"
			>
				<div class="font-preview" :style="{ fontFamily: fontStack(option.id) }" aria-hidden="true">
					<span class="font-preview-big">Аа Bb</span>
					<span class="font-preview-small">Играть на сервере 1.21</span>
				</div>
				<div class="label">
					<RadioButtonCheckedIcon v-if="ui.font === option.id" class="radio shrink-0" />
					<RadioButtonIcon v-else class="radio shrink-0" />
					<span class="flex flex-col items-start">
						<span>{{ option.name }}</span>
						<span class="text-xs font-normal text-secondary">{{ option.hint }}</span>
					</span>
				</div>
			</button>
		</div>
		<div v-if="ui.font === 'custom'" class="mt-4 flex flex-col gap-2">
			<label for="custom-font" class="font-semibold text-contrast">Название шрифта</label>
			<Input
				id="custom-font"
				v-model="ui.customFont"
				placeholder="Например: JetBrains Mono, Montserrat, Ubuntu"
				wrapper-class="max-w-md"
			/>
			<p class="m-0 text-sm text-secondary">
				Шрифт должен быть установлен в системе. Если такого нет, используется Inter.
			</p>
		</div>
	</section>

	<section class="mt-8 border-0 border-t border-solid border-divider pt-6">
		<div class="flex flex-col gap-1">
			<h2 class="m-0 text-xl font-semibold text-contrast">Новости в правой панели</h2>
			<p class="m-0 text-secondary">
				Что показывать в блоке «Новости».
				<template v-if="!config.launcherRepo">
					Обновления лаунчера появятся после публикации репозитория на GitHub, до этого показываются
					новости Modrinth.
				</template>
			</p>
		</div>
		<Chips
			v-model="newsSource"
			class="mt-4"
			:items="['releases', 'modrinth', 'both', 'none'] as NewsSource[]"
			:format-label="(item) => newsLabels[item]"
			:capitalize="false"
		/>
	</section>
</template>

<style scoped lang="scss">
.font-grid {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
	gap: var(--gap-lg);
}

.font-preview {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
	padding: 1rem;
	background: var(--surface-2);
	color: var(--color-contrast);

	.font-preview-big {
		font-size: 1.75rem;
		font-weight: 700;
		line-height: 1.1;
	}

	.font-preview-small {
		font-size: 0.85rem;
		color: var(--color-base);
	}
}
</style>
