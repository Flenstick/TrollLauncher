<script setup lang="ts">
import { LogInIcon, SpinnerIcon } from '@modrinth/assets'
import { Admonition, Button, Input, NewModal } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, nextTick, ref } from 'vue'

import { elyby_login, ELYBY_TWO_FACTOR_REQUIRED } from '@/helpers/auth'

const emit = defineEmits<{
	'logged-in': [account: { profile: { id: string; name: string } }]
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const username = ref('')
const password = ref('')
const totp = ref('')
const needsTotp = ref(false)
const pending = ref(false)
const error = ref('')

const canSubmit = computed(
	() =>
		username.value.trim() !== '' &&
		password.value !== '' &&
		(!needsTotp.value || totp.value.trim().length >= 6) &&
		!pending.value,
)

function show() {
	username.value = ''
	password.value = ''
	totp.value = ''
	needsTotp.value = false
	error.value = ''
	modal.value?.show()
}

function errorText(err: unknown): string {
	if (typeof err === 'string') return err
	if (err && typeof err === 'object' && 'message' in err) return String(err.message)
	return String(err)
}

async function submit() {
	if (!canSubmit.value) return
	pending.value = true
	error.value = ''
	try {
		const account = await elyby_login(
			username.value.trim(),
			password.value,
			needsTotp.value ? totp.value.trim() : null,
		)
		modal.value?.hide()
		emit('logged-in', account)
	} catch (err) {
		const text = errorText(err)
		if (text.includes(ELYBY_TWO_FACTOR_REQUIRED)) {
			needsTotp.value = true
			await nextTick()
			document.getElementById('elyby-totp')?.focus()
		} else {
			error.value = text.replace(/^.*?Ely\.by: /, '')
		}
	} finally {
		pending.value = false
	}
}

defineExpose({ show })
</script>

<template>
	<NewModal ref="modal" header="Вход через Ely.by" max-width="440px">
		<form class="flex flex-col gap-4" @submit.prevent="submit">
			<p class="m-0 text-secondary">
				Войди аккаунтом Ely.by, чтобы играть со скином Ely.by на серверах, которые его поддерживают.
			</p>
			<div class="flex flex-col gap-2">
				<label for="elyby-username" class="font-semibold text-contrast">Ник или e-mail</label>
				<Input id="elyby-username" v-model="username" :input-attrs="{ autocomplete: 'username' }" />
			</div>
			<div class="flex flex-col gap-2">
				<label for="elyby-password" class="font-semibold text-contrast">Пароль</label>
				<Input
					id="elyby-password"
					v-model="password"
					type="password"
					:input-attrs="{ autocomplete: 'current-password' }"
				/>
			</div>
			<div v-if="needsTotp" class="flex flex-col gap-2">
				<label for="elyby-totp" class="font-semibold text-contrast">Код двухфакторной защиты</label>
				<Input
					id="elyby-totp"
					v-model="totp"
					inputmode="numeric"
					:input-attrs="{ autocomplete: 'one-time-code' }"
					placeholder="123456"
				/>
				<span class="text-sm text-secondary">Код из приложения-аутентификатора.</span>
			</div>
			<Admonition v-if="error" type="critical" header="Не удалось войти">
				{{ error }}
			</Admonition>
			<button type="submit" class="hidden" />
		</form>
		<template #actions>
			<div class="flex items-center justify-between gap-2">
				<button
					type="button"
					class="border-0 bg-transparent p-0 text-sm text-link cursor-pointer hover:underline"
					@click="openUrl('https://account.ely.by/register')"
				>
					Нет аккаунта? Регистрация
				</button>
				<Button type="colored" color="brand" :disabled="!canSubmit" @click="submit">
					<SpinnerIcon v-if="pending" class="animate-spin" />
					<LogInIcon v-else />
					Войти
				</Button>
			</div>
		</template>
	</NewModal>
</template>
