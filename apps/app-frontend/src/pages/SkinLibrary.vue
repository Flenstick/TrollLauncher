<script setup lang="ts">
import {
	DownloadIcon,
	FolderIcon,
	LinkIcon,
	PlusIcon,
	RefreshCwIcon,
	SearchIcon,
	ShirtIcon,
	SpinnerIcon,
	StarIcon,
	TrashIcon,
	UploadIcon,
	UserIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Button,
	Chips,
	ConfirmModal,
	DropdownSelect,
	IconButton,
	injectNotificationManager,
	Input,
	NewModal,
	SkinPreviewRenderer,
} from '@modrinth/ui'
import { type DragDropEvent, getCurrentWebview } from '@tauri-apps/api/webview'
import { save } from '@tauri-apps/plugin-dialog'
import { computed, onActivated, onMounted, onUnmounted, ref, watch } from 'vue'

import LibrarySkinCard from '@/components/ui/skin-library/LibrarySkinCard.vue'
import { injectMinecraftAccounts } from '@/features/minecraft-accounts/context'
import {
	addSkin,
	asSkin,
	equipLibrarySkin,
	exportSkin,
	type ExtraCapes,
	fetchByNick,
	fetchUrl,
	getCurrentAccount,
	getExtraCapes,
	getLibrary,
	type LibraryAccount,
	type LibrarySkin,
	type LibraryVariant,
	openElyby,
	removeSkin,
	rendererCapes,
	setEquipped,
	setFolders,
	SKIN_LIBRARY_CHANGED,
	type SkinLibrary,
	updateSkin,
} from '@/helpers/skin-library'
import {
	type Cape,
	get_available_capes,
	get_available_skins,
	get_dragged_skin_data,
} from '@/helpers/skins'
import { useRootBreadcrumb } from '@/shared/breadcrumbs'

defineOptions({ name: 'SkinLibraryPage' })

const breadcrumb = useRootBreadcrumb({
	slot: 'root',
	id: 'skins',
	label: 'Скины',
	to: '/skins',
	visual: { type: 'icon', component: ShirtIcon },
})

const { addNotification, handleError } = injectNotificationManager()
const accounts = injectMinecraftAccounts()

const ALL = '__all__'
const FAVORITES = '__favorites__'
const NO_FOLDER = 'Без папки'

const library = ref<SkinLibrary>({ folders: [], skins: [], equipped: {} })
const account = ref<LibraryAccount | null>(null)
const mojangCapes = ref<Cape[]>([])
const extraCapes = ref<ExtraCapes>({ optifine: null, elyby: null })
const selectedId = ref<string | null>(null)
const folderFilter = ref<string>(ALL)
const search = ref('')
const loading = ref(true)
const equipping = ref(false)
const syncing = ref(false)
const dragActive = ref(false)

const capes = computed(() => rendererCapes(mojangCapes.value, extraCapes.value))

const visibleSkins = computed(() => {
	const query = search.value.trim().toLowerCase()
	return library.value.skins
		.filter((skin) => {
			if (folderFilter.value === FAVORITES && !skin.favorite) return false
			if (folderFilter.value !== ALL && folderFilter.value !== FAVORITES) {
				if (skin.folder !== folderFilter.value) return false
			}
			return !query || skin.name.toLowerCase().includes(query)
		})
		.sort((a, b) => Number(b.favorite) - Number(a.favorite) || b.added - a.added)
})

const selected = computed(
	() => library.value.skins.find((skin) => skin.id === selectedId.value) ?? null,
)
const equippedId = computed(() =>
	account.value ? library.value.equipped[account.value.id] : undefined,
)

/** Cape options for the selected skin: value → label. */
const capeOptions = computed(() => {
	const options: Array<{ value: string | null; label: string }> = [
		{ value: null, label: 'Без плаща' },
	]
	if (account.value?.provider !== 'elyby') {
		for (const cape of mojangCapes.value)
			options.push({ value: `mojang:${cape.id}`, label: cape.name })
	}
	if (extraCapes.value.optifine) options.push({ value: 'optifine', label: 'OptiFine' })
	if (extraCapes.value.elyby) options.push({ value: 'elyby', label: 'Ely.by' })
	return options
})

function capeTexture(cape: string | null | undefined): string | undefined {
	if (!cape) return undefined
	if (cape === 'optifine') return extraCapes.value.optifine ?? undefined
	if (cape === 'elyby') return extraCapes.value.elyby ?? undefined
	return mojangCapes.value.find((item) => `mojang:${item.id}` === cape)?.texture
}

function cardSkin(skin: LibrarySkin) {
	const mojang = skin.cape?.startsWith('mojang:') ? skin.cape.slice(7) : undefined
	return asSkin(skin, mojang)
}

function errorText(error: unknown): string {
	if (typeof error === 'string') return error
	if (error && typeof error === 'object' && 'message' in error) return String(error.message)
	return String(error)
}

async function loadLibrary() {
	library.value = await getLibrary()
	if (!library.value.skins.some((skin) => skin.id === selectedId.value)) {
		selectedId.value =
			(equippedId.value && library.value.skins.some((s) => s.id === equippedId.value)
				? equippedId.value
				: library.value.skins[0]?.id) ?? null
	}
}

async function loadAccount() {
	account.value = await getCurrentAccount().catch(() => null)
	mojangCapes.value =
		account.value?.provider === 'microsoft' ? await get_available_capes().catch(() => []) : []
	extraCapes.value = account.value
		? await getExtraCapes(account.value.name)
		: { optifine: null, elyby: null }
}

async function refresh() {
	loading.value = true
	try {
		await Promise.all([loadLibrary(), loadAccount()])
		if (library.value.skins.length === 0 && account.value) await syncFromAccount(true)
	} catch (error) {
		handleError(error as Error)
	} finally {
		loading.value = false
	}
}

/** Saves the skin the account wears right now into the library. */
async function syncFromAccount(silent = false) {
	if (!account.value) return
	syncing.value = true
	try {
		let added: LibrarySkin | null = null
		if (account.value.provider === 'elyby') {
			const fetched = await fetchByNick(account.value.name, 'elyby')
			added = await addSkin(fetched.texture, `${account.value.name} (Ely.by)`, {
				variant: fetched.variant,
				source: 'account',
			})
		} else {
			const current = (await get_available_skins()).find((skin) => skin.is_equipped)
			if (current && current.source !== 'default') {
				const fetched = current.texture.startsWith('data:')
					? {
							texture: Array.from(atob(current.texture.split(',')[1]), (c) => c.charCodeAt(0)),
							variant: (current.variant === 'SLIM' ? 'slim' : 'classic') as LibraryVariant,
						}
					: await fetchUrl(current.texture)
				added = await addSkin(fetched.texture, current.name ?? account.value.name, {
					variant: fetched.variant,
					source: 'account',
				})
			}
		}
		if (added) {
			const activeCape = mojangCapes.value.find((cape) => cape.is_equipped)
			if (activeCape && !added.cape) {
				added.cape = `mojang:${activeCape.id}`
				await updateSkin(added)
			}
			await setEquipped(account.value.id, added.id)
		}
		await loadLibrary()
		if (added) {
			selectedId.value = added.id
			if (!silent) addNotification({ title: 'Текущий скин аккаунта сохранён', type: 'success' })
		} else if (!silent) {
			addNotification({ title: 'На аккаунте стандартный скин', type: 'info' })
		}
	} catch (error) {
		console.warn('[skins] sync from account failed', error)
		if (!silent) handleError(error as Error)
	} finally {
		syncing.value = false
	}
}

async function equip() {
	const skin = selected.value
	if (!skin || !account.value || equipping.value) return
	equipping.value = true
	try {
		const result = await equipLibrarySkin(skin, account.value)
		if (result.status === 'done') {
			await loadLibrary()
			await accounts.value?.setEquippedSkin(cardSkin(skin))
			await accounts.value?.refreshValues()
			addNotification({ title: `Скин «${skin.name}» надет`, type: 'success' })
		} else if (result.status === 'need_login') {
			addNotification({
				title: 'Войди на ely.by',
				text: 'Открыл окно ely.by. Войди в аккаунт и нажми «Надеть» ещё раз — дальше лаунчер всё сделает сам.',
				type: 'info',
			})
		} else if (result.status === 'manual') {
			addNotification({
				title: 'Ely.by не принял скин автоматически',
				text: `Открыл ely.by — доделай вручную. ${result.message}`.trim(),
				type: 'warn',
			})
		}
	} catch (error) {
		handleError(error as Error)
	} finally {
		equipping.value = false
	}
}

let saveTimer: ReturnType<typeof setTimeout> | undefined
function patchSelected(patch: Partial<LibrarySkin>) {
	const skin = selected.value
	if (!skin) return
	Object.assign(skin, patch)
	clearTimeout(saveTimer)
	saveTimer = setTimeout(() => updateSkin(skin).catch(handleError), 300)
}

function selectAndEquip(id: string) {
	selectedId.value = id
	void equip()
}

const deleteModal = ref<InstanceType<typeof ConfirmModal>>()
async function deleteSelected() {
	const skin = selected.value
	if (!skin) return
	await removeSkin(skin.id).catch(handleError)
	selectedId.value = null
	await loadLibrary()
}

async function exportSelected() {
	const skin = selected.value
	if (!skin) return
	const path = await save({
		defaultPath: `${skin.name.replace(/[\\/:*?"<>|]/g, '_')}.png`,
		filters: [{ name: 'PNG', extensions: ['png'] }],
	})
	if (!path) return
	await exportSkin(skin.id, path)
		.then(() => addNotification({ title: 'Скин сохранён', text: path, type: 'success' }))
		.catch(handleError)
}


const currentFolder = computed(() =>
	folderFilter.value === ALL || folderFilter.value === FAVORITES ? null : folderFilter.value,
)

async function addBytes(
	bytes: Uint8Array | number[],
	name: string,
	source: string,
	variant?: LibraryVariant,
) {
	const skin = await addSkin(bytes, name, { variant, folder: currentFolder.value, source })
	await loadLibrary()
	selectedId.value = skin.id
	return skin
}

const fileInput = ref<HTMLInputElement>()
async function onFiles(files: FileList | File[] | null | undefined) {
	for (const file of Array.from(files ?? [])) {
		if (!file.name.toLowerCase().endsWith('.png')) continue
		await addBytes(
			new Uint8Array(await file.arrayBuffer()),
			file.name.replace(/\.png$/i, ''),
			'file',
		).catch((error) =>
			addNotification({
				title: `Не получилось добавить ${file.name}`,
				text: errorText(error),
				type: 'error',
			}),
		)
	}
	if (fileInput.value) fileInput.value.value = ''
}

const importModal = ref<InstanceType<typeof NewModal>>()
const importMode = ref<'mojang' | 'elyby' | 'url'>('mojang')
const importValue = ref('')
const importError = ref('')
const importing = ref(false)
const importLabels = { mojang: 'Ник Minecraft', elyby: 'Ник Ely.by', url: 'Ссылка на PNG' }

function openImport(mode: 'mojang' | 'elyby' | 'url') {
	importMode.value = mode
	importValue.value = ''
	importError.value = ''
	importModal.value?.show()
}

async function runImport() {
	if (!importValue.value.trim() || importing.value) return
	importing.value = true
	importError.value = ''
	try {
		const fetched =
			importMode.value === 'url'
				? await fetchUrl(importValue.value)
				: await fetchByNick(importValue.value, importMode.value)
		await addBytes(
			fetched.texture,
			fetched.name,
			importMode.value === 'url' ? 'url' : `${importMode.value}:${fetched.name}`,
			fetched.variant,
		)
		importModal.value?.hide()
	} catch (error) {
		importError.value = errorText(error)
	} finally {
		importing.value = false
	}
}

// Native drag & drop of PNG files from the file manager.
let unlistenDrop: (() => void) | undefined
let unmounted = false
async function onNativeDrop(event: { payload: DragDropEvent }) {
	const payload = event.payload
	if (payload.type === 'enter' || payload.type === 'over') dragActive.value = true
	else if (payload.type === 'leave') dragActive.value = false
	else if (payload.type === 'drop') {
		dragActive.value = false
		for (const path of payload.paths.filter((p) => p.toLowerCase().endsWith('.png'))) {
			try {
				const data = await get_dragged_skin_data(path)
				const name =
					path
						.split(/[\\/]/)
						.pop()
						?.replace(/\.png$/i, '') ?? 'Скин'
				await addBytes(data, name, 'file')
			} catch (error) {
				addNotification({
					title: 'Не получилось добавить скин',
					text: errorText(error),
					type: 'error',
				})
			}
		}
	}
}


const folderModal = ref<InstanceType<typeof NewModal>>()
const folderName = ref('')
const folderRenaming = ref<string | null>(null)

function openFolderModal(rename: string | null = null) {
	folderRenaming.value = rename
	folderName.value = rename ?? ''
	folderModal.value?.show()
}

async function submitFolder() {
	const name = folderName.value.trim()
	if (!name) return
	const folders = [...library.value.folders]
	if (folderRenaming.value) {
		const index = folders.indexOf(folderRenaming.value)
		if (index >= 0) folders[index] = name
		await setFolders(folders, { [folderRenaming.value]: name }).catch(handleError)
		if (folderFilter.value === folderRenaming.value) folderFilter.value = name
	} else if (!folders.includes(name)) {
		folders.push(name)
		await setFolders(folders).catch(handleError)
		folderFilter.value = name
	}
	folderModal.value?.hide()
	await loadLibrary()
}

async function deleteFolder(name: string) {
	await setFolders(library.value.folders.filter((folder) => folder !== name)).catch(handleError)
	if (folderFilter.value === name) folderFilter.value = ALL
	await loadLibrary()
}

const folderOptions = computed(() => [NO_FOLDER, ...library.value.folders])


const onLibraryChanged = () => void loadLibrary()

onMounted(async () => {
	window.addEventListener(SKIN_LIBRARY_CHANGED, onLibraryChanged)
	await refresh()
	try {
		const unlisten = await getCurrentWebview().onDragDropEvent(onNativeDrop)
		if (unmounted) unlisten()
		else unlistenDrop = unlisten
	} catch (error) {
		console.warn('Drag and drop is unavailable', error)
	}
})

onActivated(() => {
	breadcrumb.reset()
	void loadAccount()
})

onUnmounted(() => {
	unmounted = true
	unlistenDrop?.()
	window.removeEventListener(SKIN_LIBRARY_CHANGED, onLibraryChanged)
	clearTimeout(saveTimer)
})

watch(selectedId, () => clearTimeout(saveTimer))
</script>

<template>
	<div class="relative box-border flex min-h-full flex-col gap-4 p-6">
		<!-- Header -->
		<div class="flex flex-wrap items-center gap-3">
			<h1 class="m-0 mr-auto text-2xl font-bold text-contrast">Скины</h1>
			<div
				v-if="account"
				class="flex items-center gap-2 rounded-full bg-surface-3 py-1 pl-3 pr-3 text-sm"
			>
				<UserIcon class="size-4 text-secondary" />
				<span class="font-semibold text-contrast">{{ account.name }}</span>
				<span
					class="rounded-full px-2 py-0.5 text-xs font-bold"
					:class="
						account.provider === 'elyby'
							? 'bg-highlight-green text-brand-green'
							: 'bg-highlight-blue text-brand-blue'
					"
					>{{ account.provider === 'elyby' ? 'Ely.by' : 'Microsoft' }}</span
				>
			</div>
			<Button :disabled="!account || syncing" @click="syncFromAccount()">
				<SpinnerIcon v-if="syncing" class="animate-spin" />
				<RefreshCwIcon v-else />
				Забрать скин с аккаунта
			</Button>
			<Button v-if="account?.provider === 'elyby'" @click="openElyby('login')">
				<LinkIcon /> Ely.by
			</Button>
		</div>

		<div
			class="grid min-h-0 grow grid-cols-[minmax(260px,340px)_1fr] gap-6 max-[900px]:grid-cols-1"
		>
			<!-- Preview & editor -->
			<aside class="sticky top-4 flex flex-col gap-4 self-start">
				<div
					class="relative flex h-[min(40vh,440px)] items-center justify-center overflow-hidden rounded-3xl bg-surface-2"
				>
					<SkinPreviewRenderer
						v-if="selected"
						:key="selected.id"
						:texture-src="selected.texture"
						:cape-src="capeTexture(selected.cape)"
						:variant="selected.variant === 'slim' ? 'SLIM' : 'CLASSIC'"
						:nametag="account?.name"
						:initial-rotation="Math.PI / 8"
						:ears-enabled="false"
					/>
					<div v-else class="flex flex-col items-center gap-2 p-6 text-center text-secondary">
						<ShirtIcon class="size-10" />
						<span>Выбери скин справа или добавь новый</span>
					</div>
				</div>

				<template v-if="selected">
					<Input
						:model-value="selected.name"
						placeholder="Название"
						@update:model-value="patchSelected({ name: String($event) })"
					/>

					<div class="flex flex-col gap-2">
						<span class="text-sm font-semibold text-secondary">Модель рук</span>
						<Chips
							:model-value="selected.variant"
							:items="['classic', 'slim'] as LibraryVariant[]"
							:format-label="(item) => (item === 'slim' ? 'Тонкие (Alex)' : 'Обычные (Steve)')"
							:capitalize="false"
							size="small"
							@update:model-value="patchSelected({ variant: $event as LibraryVariant })"
						/>
					</div>

					<div class="flex flex-col gap-2">
						<span class="text-sm font-semibold text-secondary">Плащ</span>
						<div class="flex flex-wrap gap-2">
							<button
								v-for="option in capeOptions"
								:key="option.value ?? 'none'"
								type="button"
								class="rounded-full border border-solid px-3 py-1 text-sm font-semibold transition-colors"
								:class="
									selected.cape === option.value
										? 'border-brand bg-highlight text-contrast'
										: 'border-surface-5 bg-surface-3 text-secondary hover:text-contrast'
								"
								@click="patchSelected({ cape: option.value })"
							>
								{{ option.label }}
							</button>
						</div>
						<span v-if="selected.cape === 'optifine'" class="text-xs text-secondary">
							Плащ OptiFine нельзя сменить из лаунчера: его видят игроки с OptiFine или модами,
							которые его поддерживают.
						</span>
						<span v-else-if="selected.cape === 'elyby'" class="text-xs text-secondary">
							Плащ Ely.by привязан к аккаунту ely.by и меняется на сайте.
						</span>
					</div>

					<div class="flex flex-col gap-2">
						<span class="text-sm font-semibold text-secondary">Папка</span>
						<DropdownSelect
							name="skin-folder"
							:options="folderOptions"
							:model-value="selected.folder ?? NO_FOLDER"
							@update:model-value="
								patchSelected({ folder: $event === NO_FOLDER ? null : String($event) })
							"
						/>
					</div>

					<div class="flex flex-wrap gap-2">
						<Button
							type="colored"
							color="brand"
							:disabled="!account || equipping || equippedId === selected.id"
							class="grow"
							@click="equip"
						>
							<SpinnerIcon v-if="equipping" class="animate-spin" />
							<ShirtIcon v-else />
							{{ equippedId === selected.id ? 'Надет' : 'Надеть' }}
						</Button>
						<IconButton
							:label="selected.favorite ? 'Убрать из избранного' : 'В избранное'"
							@click="patchSelected({ favorite: !selected.favorite })"
						>
							<StarIcon :class="selected.favorite ? 'fill-current text-orange' : ''" />
						</IconButton>
						<IconButton label="Сохранить PNG" @click="exportSelected">
							<DownloadIcon />
						</IconButton>
						<IconButton label="Удалить" color="red" @click="deleteModal?.show()">
							<TrashIcon />
						</IconButton>
					</div>
					<span v-if="!account" class="text-sm text-secondary">
						Войди в аккаунт Minecraft или Ely.by, чтобы надевать скины.
					</span>
				</template>
			</aside>

			<!-- Library -->
			<section class="flex min-w-0 flex-col gap-4">
				<div class="flex flex-wrap items-center gap-2">
					<button
						v-for="filter in [ALL, FAVORITES, ...library.folders]"
						:key="filter"
						type="button"
						class="group flex items-center gap-1.5 rounded-full border border-solid px-3 py-1.5 text-sm font-semibold transition-colors"
						:class="
							folderFilter === filter
								? 'border-brand bg-highlight text-contrast'
								: 'border-surface-4 bg-surface-2 text-secondary hover:text-contrast'
						"
						@click="folderFilter = filter"
						@dblclick="filter !== ALL && filter !== FAVORITES && openFolderModal(filter)"
					>
						<StarIcon v-if="filter === FAVORITES" class="size-4" />
						<FolderIcon v-else-if="filter !== ALL" class="size-4" />
						{{ filter === ALL ? 'Все' : filter === FAVORITES ? 'Избранное' : filter }}
						<span class="text-xs opacity-70">{{
							filter === ALL
								? library.skins.length
								: filter === FAVORITES
									? library.skins.filter((s) => s.favorite).length
									: library.skins.filter((s) => s.folder === filter).length
						}}</span>
						<XIcon
							v-if="filter !== ALL && filter !== FAVORITES"
							class="size-3.5 opacity-0 transition-opacity group-hover:opacity-70 hover:!opacity-100"
							:aria-label="`Удалить папку ${filter}`"
							@click.stop="deleteFolder(filter)"
						/>
					</button>
					<button
						type="button"
						class="flex items-center gap-1 rounded-full border border-dashed border-surface-5 bg-transparent px-3 py-1.5 text-sm font-semibold text-secondary hover:text-contrast"
						@click="openFolderModal()"
					>
						<PlusIcon class="size-4" /> Папка
					</button>
				</div>

				<div class="flex flex-wrap items-center gap-2">
					<div class="relative min-w-[200px] grow">
						<SearchIcon
							class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-secondary"
						/>
						<input
							v-model="search"
							type="search"
							placeholder="Поиск по названию"
							class="w-full !pl-9"
						/>
					</div>
					<Button type="colored" color="brand" @click="fileInput?.click()"
						><UploadIcon /> Файл PNG</Button
					>
					<Button @click="openImport('mojang')"><UserIcon /> По нику</Button>
					<Button @click="openImport('elyby')">Ely.by</Button>
					<Button @click="openImport('url')"><LinkIcon /> По ссылке</Button>
					<input
						ref="fileInput"
						type="file"
						accept="image/png"
						multiple
						class="hidden"
						@change="onFiles(($event.target as HTMLInputElement).files)"
					/>
				</div>

				<div v-if="loading" class="flex justify-center py-16 text-secondary">
					<SpinnerIcon class="size-6 animate-spin" />
				</div>
				<div
					v-else-if="visibleSkins.length === 0"
					class="flex flex-col items-center gap-2 rounded-3xl border-2 border-dashed border-surface-4 px-6 py-16 text-center text-secondary"
				>
					<ShirtIcon class="size-10" />
					<span v-if="library.skins.length === 0">
						Библиотека пуста. Перетащи сюда PNG-скины, добавь по нику или забери скин с аккаунта.
					</span>
					<span v-else>Здесь ничего нет.</span>
				</div>
				<div v-else class="grid grid-cols-[repeat(auto-fill,minmax(132px,1fr))] gap-3">
					<LibrarySkinCard
						v-for="skin in visibleSkins"
						:key="skin.id"
						:skin="cardSkin(skin)"
						:name="skin.name"
						:capes="capes"
						:selected="skin.id === selectedId"
						:equipped="skin.id === equippedId"
						:favorite="skin.favorite"
						@click="selectedId = skin.id"
						@dblclick="selectAndEquip(skin.id)"
					/>
				</div>
			</section>
		</div>

		<div
			v-if="dragActive"
			class="pointer-events-none fixed inset-4 z-50 flex items-center justify-center rounded-3xl border-4 border-dashed border-brand bg-black/50 text-xl font-bold text-contrast"
		>
			Отпусти, чтобы добавить скины{{ currentFolder ? ` в «${currentFolder}»` : '' }}
		</div>
	</div>

	<NewModal
		ref="importModal"
		:header="`Добавить: ${importLabels[importMode].toLowerCase()}`"
		max-width="440px"
	>
		<form class="flex flex-col gap-3" @submit.prevent="runImport">
			<Chips
				v-model="importMode"
				:items="['mojang', 'elyby', 'url'] as const"
				:format-label="(item) => ({ mojang: 'Minecraft', elyby: 'Ely.by', url: 'Ссылка' })[item]"
				:capitalize="false"
				size="small"
			/>
			<Input
				v-model="importValue"
				:placeholder="importMode === 'url' ? 'https://…/skin.png' : 'Ник игрока'"
			/>
			<Admonition v-if="importError" type="critical" header="Не получилось">
				{{ importError }}
			</Admonition>
			<button type="submit" class="hidden" />
		</form>
		<template #actions>
			<div class="flex justify-end">
				<Button
					type="colored"
					color="brand"
					:disabled="!importValue.trim() || importing"
					@click="runImport"
				>
					<SpinnerIcon v-if="importing" class="animate-spin" />
					<PlusIcon v-else />
					Добавить
				</Button>
			</div>
		</template>
	</NewModal>

	<NewModal
		ref="folderModal"
		:header="folderRenaming ? 'Переименовать папку' : 'Новая папка'"
		max-width="400px"
	>
		<form @submit.prevent="submitFolder">
			<Input v-model="folderName" placeholder="Название папки" />
			<button type="submit" class="hidden" />
		</form>
		<template #actions>
			<div class="flex justify-end">
				<Button type="colored" color="brand" :disabled="!folderName.trim()" @click="submitFolder">
					{{ folderRenaming ? 'Переименовать' : 'Создать' }}
				</Button>
			</div>
		</template>
	</NewModal>

	<ConfirmModal
		ref="deleteModal"
		title="Удалить скин?"
		:description="`«${selected?.name ?? ''}» пропадёт из библиотеки. На аккаунте он останется, если надет.`"
		proceed-label="Удалить"
		@proceed="deleteSelected"
	/>
</template>
