<script setup lang="ts">
import { RadioButtonCheckedIcon, RadioButtonIcon, UndoIcon } from '@modrinth/assets'
import { Button } from '@modrinth/ui'
import { computed } from 'vue'

import {
	CUSTOM_PALETTE_ID,
	DEFAULT_CUSTOM_COLORS,
	type Palette,
	usePalette,
} from '@/composables/use-palette.ts'

const palette = usePalette()

const customPalette = computed<Palette>(() => ({
	id: CUSTOM_PALETTE_ID,
	name: 'Своя тема',
	base: palette.customBase,
	colors: palette.custom,
}))

const surfaceLabels = ['Фон', 'Боковая панель', 'Карточки', 'Кнопки', 'Рамки']

function previewStyle(option: Palette): Record<string, string> {
	const surfaces = option.colors.surfaces
	return {
		'--pv-bg': surfaces?.[0] ?? 'var(--surface-1)',
		'--pv-card': surfaces?.[2] ?? 'var(--surface-3)',
		'--pv-border': surfaces?.[3] ?? 'var(--surface-4)',
		'--pv-text': option.colors.textDefault ?? 'var(--color-text-default)',
		'--pv-muted': option.colors.textTertiary ?? 'var(--color-text-tertiary)',
		'--pv-accent': option.colors.accent,
	}
}

function select(id: string): void {
	palette.selected = id
}

function setSurface(index: number, value: string): void {
	const surfaces = [...palette.custom.surfaces] as typeof palette.custom.surfaces
	surfaces[index] = value
	palette.custom.surfaces = surfaces
}
</script>

<template>
	<section class="mt-8 border-0 border-t border-solid border-divider pt-6">
		<div class="flex flex-col gap-1">
			<h2 class="m-0 text-xl font-semibold text-contrast">Акцентный цвет</h2>
			<p class="m-0 text-secondary">Цвет кнопок, ссылок и выделения поверх выбранной темы.</p>
		</div>
		<div class="mt-4 flex flex-wrap gap-3" role="group" aria-label="Акцентный цвет">
			<button
				v-for="option in palette.accentPalettes"
				:key="option.id"
				v-tooltip="option.name"
				type="button"
				class="accent-swatch"
				:class="{ selected: palette.selected === option.id }"
				:style="{ '--pv-accent': option.colors.accent }"
				:aria-label="option.name"
				:aria-pressed="palette.selected === option.id"
				@click="select(option.id)"
			/>
		</div>
	</section>

	<section class="mt-8 border-0 border-t border-solid border-divider pt-6">
		<div class="flex flex-col gap-1">
			<h2 class="m-0 text-xl font-semibold text-contrast">Цветовые темы</h2>
			<p class="m-0 text-secondary">
				Полные палитры: заменяют фон, панели, текст и акцент. Светлая или тёмная основа задаётся
				самой темой.
			</p>
		</div>
		<div class="palette-grid mt-4" role="group" aria-label="Цветовые темы">
			<button
				v-for="option in [...palette.fullPalettes, customPalette]"
				:key="option.id"
				type="button"
				class="preview-radio button-base"
				:class="{ selected: palette.selected === option.id }"
				:aria-pressed="palette.selected === option.id"
				@click="select(option.id)"
			>
				<div class="palette-preview" :style="previewStyle(option)" aria-hidden="true">
					<div class="palette-card">
						<div class="palette-icon" />
						<div class="palette-line" />
						<div class="palette-line short" />
						<div class="palette-button" />
					</div>
				</div>
				<div class="label">
					<RadioButtonCheckedIcon
						v-if="palette.selected === option.id"
						class="radio shrink-0"
						aria-hidden="true"
					/>
					<RadioButtonIcon v-else class="radio shrink-0" aria-hidden="true" />
					{{ option.name }}
				</div>
			</button>
		</div>

		<div
			v-if="palette.selected === CUSTOM_PALETTE_ID"
			class="mt-6 flex flex-col gap-4 rounded-2xl border border-solid border-surface-4 bg-surface-2 p-4"
		>
			<div class="flex items-center justify-between gap-4">
				<h3 class="m-0 text-base font-semibold text-contrast">Редактор своей темы</h3>
				<Button type="quiet" @click="palette.resetCustom()">
					<UndoIcon />
					Сбросить
				</Button>
			</div>

			<div class="flex gap-2">
				<Button
					:type="palette.customBase === 'dark' ? 'colored' : 'outlined'"
					color="brand"
					@click="palette.customBase = 'dark'"
				>
					Тёмная основа
				</Button>
				<Button
					:type="palette.customBase === 'light' ? 'colored' : 'outlined'"
					color="brand"
					@click="palette.customBase = 'light'"
				>
					Светлая основа
				</Button>
			</div>

			<div class="color-fields">
				<label class="color-field">
					<input v-model="palette.custom.accent" type="color" />
					<span>Акцент</span>
				</label>
				<label class="color-field">
					<input v-model="palette.custom.accentContrast" type="color" />
					<span>Текст на акценте</span>
				</label>
				<label v-for="(label, index) in surfaceLabels" :key="label" class="color-field">
					<input
						type="color"
						:value="palette.custom.surfaces[index]"
						@input="setSurface(index, ($event.target as HTMLInputElement).value)"
					/>
					<span>{{ label }}</span>
				</label>
				<label class="color-field">
					<input v-model="palette.custom.textPrimary" type="color" />
					<span>Заголовки</span>
				</label>
				<label class="color-field">
					<input v-model="palette.custom.textDefault" type="color" />
					<span>Основной текст</span>
				</label>
				<label class="color-field">
					<input v-model="palette.custom.textTertiary" type="color" />
					<span>Подписи</span>
				</label>
			</div>
			<p class="m-0 text-sm text-secondary">
				Изменения применяются сразу. Исходные значения: акцент
				{{ DEFAULT_CUSTOM_COLORS.accent }}, фон {{ DEFAULT_CUSTOM_COLORS.surfaces[0] }}.
			</p>
		</div>
	</section>
</template>

<style scoped lang="scss">
.accent-swatch {
	width: 2.25rem;
	height: 2.25rem;
	border-radius: 999px;
	border: none;
	padding: 0;
	cursor: pointer;
	background: var(--pv-accent);
	box-shadow:
		0 0 0 2px var(--surface-1),
		0 0 0 2px transparent;
	transition:
		box-shadow 0.15s ease,
		transform 0.1s ease;

	&:hover {
		transform: scale(1.06);
	}

	&:active {
		transform: scale(0.96);
	}

	&.selected {
		box-shadow:
			0 0 0 3px var(--surface-1),
			0 0 0 5px var(--pv-accent);
	}
}

.palette-grid {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
	gap: var(--gap-lg);
}

.palette-preview {
	background: var(--pv-bg);
	padding: 1rem;

	.palette-card {
		display: grid;
		grid-template: 'icon line1' 'icon line2' 'button button';
		grid-template-columns: auto 1fr;
		gap: 0.5rem;
		padding: 0.75rem;
		border-radius: var(--radius-lg);
		background: var(--pv-card);
		border: 1px solid var(--pv-border);
	}

	.palette-icon {
		grid-area: icon;
		width: 2rem;
		height: 2rem;
		border-radius: var(--radius-sm);
		background: var(--pv-border);
	}

	.palette-line {
		grid-area: line1;
		height: 0.5rem;
		border-radius: var(--radius-sm);
		background: var(--pv-text);

		&.short {
			grid-area: line2;
			width: 60%;
			background: var(--pv-muted);
		}
	}

	.palette-button {
		grid-area: button;
		height: 0.75rem;
		width: 45%;
		border-radius: 999px;
		background: var(--pv-accent);
	}
}

.color-fields {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
	gap: 0.75rem;
}

.color-field {
	display: flex;
	align-items: center;
	gap: 0.6rem;
	color: var(--color-text-default);
	cursor: pointer;

	input[type='color'] {
		width: 2rem;
		height: 2rem;
		padding: 0;
		border: 1px solid var(--surface-5);
		border-radius: var(--radius-sm);
		background: none;
		cursor: pointer;

		&::-webkit-color-swatch-wrapper {
			padding: 0;
		}

		&::-webkit-color-swatch {
			border: none;
			border-radius: calc(var(--radius-sm) - 1px);
		}
	}
}
</style>
