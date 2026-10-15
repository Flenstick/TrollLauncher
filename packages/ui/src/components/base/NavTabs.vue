<template>
	<div
		v-if="filteredLinks.length > 1"
		:class="pageNav ? '-mx-6 -mt-2 mb-1 overflow-x-auto px-6 py-2' : 'contents'"
		v-bind="pageNav ? $attrs : {}"
	>
		<nav
			ref="scrollContainer"
			class="relative flex w-fit rounded-full bg-bg-raised p-1 text-xs sm:text-sm font-bold"
			:class="{
				'card-shadow border border-solid border-surface-4': mode === 'navigation',
				'tab-color-delayed': colorChangeDelayed,
			}"
			v-bind="pageNav ? {} : $attrs"
		>
			<template v-if="mode === 'navigation'">
				<RouterLink
					v-for="(link, index) in filteredLinks"
					v-show="link.shown ?? true"
					v-slot="{ href, navigate }"
					:key="link.href"
					:replace="replace"
					:to="query ? (link.href ? `?${query}=${link.href}` : '?') : link.href"
					custom
				>
					<a
						ref="tabLinkElements"
						:href="href"
						class="button-animation z-[1] flex flex-row items-center gap-2 px-4 py-2 focus:rounded-full"
						:class="getSSRFallbackClasses(index)"
						@mouseenter="link.onHover?.()"
						@focus="link.onHover?.()"
						@click="onTabClick($event, index, navigate)"
					>
						<component
							:is="link.icon"
							v-if="link.icon"
							class="tab-color hidden sm:block size-5"
							:class="getIconClasses(index)"
						/>
						<span class="tab-color text-nowrap" :class="getLabelClasses(index)">
							{{ link.label }}
						</span>
					</a>
				</RouterLink>
			</template>

			<template v-else>
				<div
					v-for="(link, index) in filteredLinks"
					v-show="link.shown ?? true"
					:key="link.href"
					ref="tabLinkElements"
					class="button-animation z-[1] flex flex-row items-center gap-2 px-4 py-2 hover:cursor-pointer focus:rounded-full"
					:class="getSSRFallbackClasses(index)"
					@click="emit('tabClick', index, link)"
				>
					<component
						:is="link.icon"
						v-if="link.icon"
						class="tab-color size-5"
						:class="getIconClasses(index)"
					/>
					<span class="tab-color text-nowrap" :class="getLabelClasses(index)">
						{{ link.label }}
					</span>
				</div>
			</template>

			<div
				v-if="sliderReady && currentActiveIndex !== -1"
				class="navtabs-slider pointer-events-none absolute left-0 top-0"
				:class="{ 'is-subpage': subpageSelected }"
				aria-hidden="true"
			>
				<span ref="sliderStart" class="navtabs-slider-cap" />
				<span ref="sliderMiddle" class="navtabs-slider-middle" />
				<span ref="sliderEnd" class="navtabs-slider-cap" />
			</div>
		</nav>
	</div>
</template>

<script setup lang="ts">
import type { Component } from 'vue'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { RouterLink, useRoute } from 'vue-router'

const route = useRoute()

defineOptions({ inheritAttrs: false })

interface Tab {
	label: string
	href: string
	shown?: boolean
	icon?: Component
	subpages?: string[]
	onHover?: () => void
}

const props = withDefaults(
	defineProps<{
		replace?: boolean
		links: Tab[]
		query?: string
		mode?: 'navigation' | 'local'
		activeIndex?: number
		pageNav?: boolean
		/** Delays the route change after a click so the slide is not stalled by the new page rendering. */
		navigationDelay?: number
	}>(),
	{
		navigationDelay: 0,
		mode: 'navigation',
		query: undefined,
		activeIndex: undefined,
		pageNav: false,
	},
)

const emit = defineEmits<{
	tabClick: [index: number, tab: Tab]
}>()

// DOM refs
const scrollContainer = ref<HTMLElement | null>(null)
const tabLinkElements = ref<HTMLElement[]>()

const sliderStart = ref<HTMLElement | null>(null)
const sliderMiddle = ref<HTMLElement | null>(null)
const sliderEnd = ref<HTMLElement | null>(null)

const currentActiveIndex = ref(-1)
const subpageSelected = ref(false)

const sliderReady = ref(false)
const colorChangeDelayed = ref(false)

const filteredLinks = computed(() => props.links.filter((link) => link.shown ?? true))

const isActiveAndNotSubpage = computed(
	() => (index: number) => currentActiveIndex.value === index && !subpageSelected.value,
)

function getSSRFallbackClasses(index: number) {
	if (sliderReady.value) return {}
	if (currentActiveIndex.value !== index) return {}

	return {
		'rounded-full': true,
		'bg-button-bgSelected': !subpageSelected.value,
		'bg-button-bg': subpageSelected.value,
	}
}

function getIconClasses(index: number) {
	return {
		'text-button-textSelected': isActiveAndNotSubpage.value(index),
		'text-secondary': !isActiveAndNotSubpage.value(index),
	}
}

function getLabelClasses(index: number) {
	return {
		'text-button-textSelected': isActiveAndNotSubpage.value(index),
		'text-contrast': !isActiveAndNotSubpage.value(index),
	}
}

function computeActiveIndex(): { index: number; isSubpage: boolean } {
	if (props.mode === 'local' && props.activeIndex !== undefined) {
		return {
			index: Math.min(props.activeIndex, filteredLinks.value.length - 1),
			isSubpage: false,
		}
	}

	for (let i = filteredLinks.value.length - 1; i >= 0; i--) {
		const link = filteredLinks.value[i]
		const decodedPath = decodeURIComponent(route.path)
		const decodedHref = decodeURIComponent(link.href.split('?')[0])

		if (props.query) {
			const queryValue = route.query[props.query]
			if (queryValue === link.href || (!queryValue && !link.href)) {
				return { index: i, isSubpage: false }
			}
			continue
		}

		if (decodedPath === decodedHref) {
			return { index: i, isSubpage: false }
		}

		const isSubpageMatch =
			(decodedPath.startsWith(decodedHref) &&
				(decodedPath.length === decodedHref.length || decodedPath[decodedHref.length] === '/')) ||
			link.subpages?.some((subpage) => decodedPath.includes(subpage))

		if (isSubpageMatch) {
			return { index: i, isSubpage: true }
		}
	}

	return { index: -1, isSubpage: false }
}

function getTabElement(index: number): HTMLElement | null {
	if (index === -1) return null

	const container = scrollContainer.value as HTMLElement | undefined
	if (!container) return null

	const tabs = container.querySelectorAll('.button-animation')
	const element = tabs[index] as HTMLElement | undefined

	if (!element) return null

	return element
}

type SliderRect = { x: number; y: number; width: number; height: number }

const SLIDE_DURATION = 320
const SLIDE_FRAMES = 32

let sliderRect: SliderRect | null = null
let slide: { from: SliderRect; to: SliderRect; startedAt: number } | null = null

function getSliderPosition(): SliderRect | null {
	const el = getTabElement(currentActiveIndex.value)
	if (!el || !scrollContainer.value) return null

	return { x: el.offsetLeft, y: el.offsetTop, width: el.offsetWidth, height: el.offsetHeight }
}

/** Fast ease for the edge leading the motion. */
function leadEase(t: number) {
	return 1 - (1 - t) ** 4
}

/** Same curve, slightly delayed, for the trailing edge, giving a liquid stretch. */
function trailEase(t: number) {
	return leadEase(Math.min(1, Math.max(0, (t - 0.1) / 0.9)))
}

function interpolate(from: SliderRect, to: SliderRect, t: number): SliderRect {
	const movingRight = to.x > from.x
	const left = from.x + (to.x - from.x) * (movingRight ? trailEase(t) : leadEase(t))
	const fromRight = from.x + from.width
	const toRight = to.x + to.width
	const right = fromRight + (toRight - fromRight) * (movingRight ? leadEase(t) : trailEase(t))
	return {
		x: left,
		y: from.y + (to.y - from.y) * leadEase(t),
		width: Math.max(right - left, 0),
		height: from.height + (to.height - from.height) * leadEase(t),
	}
}

function sliderTransforms(rect: SliderRect) {
	const cap = rect.height
	const middle = Math.max(rect.width - cap, 0)
	return {
		start: `translate(${rect.x}px, ${rect.y}px)`,
		middle: `translate(${rect.x + cap / 2}px, ${rect.y}px) scaleX(${middle})`,
		end: `translate(${rect.x + rect.width - cap}px, ${rect.y}px)`,
	}
}

function sliderParts() {
	const parts = [sliderStart.value, sliderMiddle.value, sliderEnd.value]
	return parts.every(Boolean) ? (parts as HTMLElement[]) : null
}

function setSliderInstantly(rect: SliderRect) {
	const parts = sliderParts()
	if (!parts) return
	const transforms = sliderTransforms(rect)
	const [start, middle, end] = parts
	for (const part of parts) {
		for (const animation of part.getAnimations()) animation.cancel()
		part.style.height = `${rect.height}px`
	}
	start.style.width = `${rect.height}px`
	end.style.width = `${rect.height}px`
	start.style.transform = transforms.start
	middle.style.transform = transforms.middle
	end.style.transform = transforms.end
	sliderRect = rect
	slide = null
}

/** Where the slider visually is right now, including mid-animation. */
function currentSliderRect(): SliderRect | null {
	if (!slide) return sliderRect
	const t = Math.min(1, (performance.now() - slide.startedAt) / SLIDE_DURATION)
	return interpolate(slide.from, slide.to, t)
}

function slideSliderTo(to: SliderRect) {
	const parts = sliderParts()
	const from = currentSliderRect()
	if (!parts || !from || from.height !== to.height) {
		setSliderInstantly(to)
		return
	}

	const frames = Array.from({ length: SLIDE_FRAMES + 1 }, (_, i) =>
		sliderTransforms(interpolate(from, to, i / SLIDE_FRAMES)),
	)
	const final = sliderTransforms(to)
	const [start, , end] = parts
	const keys = ['start', 'middle', 'end'] as const

	parts.forEach((part, i) => {
		for (const animation of part.getAnimations()) animation.cancel()
		part.style.transform = final[keys[i]]
		part.animate(
			frames.map((frame) => ({ transform: frame[keys[i]] })),
			{ duration: SLIDE_DURATION, easing: 'linear' },
		)
	})
	start.style.width = end.style.width = `${to.height}px`

	sliderRect = to
	slide = { from, to, startedAt: performance.now() }
	window.setTimeout(() => {
		if (slide?.to === to) slide = null
	}, SLIDE_DURATION)
}

function positionSlider(animate = true) {
	const newPosition = getSliderPosition()
	if (!newPosition) return

	if (!sliderReady.value) {
		sliderReady.value = true
		nextTick(() => setSliderInstantly(newPosition))
		return
	}

	const unchanged =
		sliderRect &&
		sliderRect.x === newPosition.x &&
		sliderRect.y === newPosition.y &&
		sliderRect.width === newPosition.width &&
		sliderRect.height === newPosition.height
	if (unchanged) return

	if (animate) slideSliderTo(newPosition)
	else setSliderInstantly(newPosition)
}

function resetSliderPosition() {
	if (!sliderReady.value || currentActiveIndex.value === -1) {
		return
	}
	positionSlider(slide !== null)
}

let pendingNavigation: number | undefined

/** Starts the slide on click instead of waiting for the route to resolve. */
function onTabClick(event: MouseEvent, index: number, navigate: (event?: MouseEvent) => unknown) {
	const modified = event.metaKey || event.altKey || event.ctrlKey || event.shiftKey
	if (modified || event.button !== 0) return

	event.preventDefault()
	window.clearTimeout(pendingNavigation)
	if (index !== currentActiveIndex.value) {
		colorChangeDelayed.value = false
		currentActiveIndex.value = index
		subpageSelected.value = false
		positionSlider()
	}

	if (props.navigationDelay > 0) {
		pendingNavigation = window.setTimeout(() => {
			pendingNavigation = undefined
			navigate()
		}, props.navigationDelay)
	} else {
		navigate()
	}
}

async function updateActiveTab() {
	await nextTick()
	if (pendingNavigation !== undefined) return
	const { index, isSubpage } = computeActiveIndex()
	colorChangeDelayed.value = sliderReady.value && index !== currentActiveIndex.value
	currentActiveIndex.value = index
	subpageSelected.value = isSubpage

	if (index !== -1) {
		positionSlider()
	}
}

const initialActive = computeActiveIndex()
currentActiveIndex.value = initialActive.index
subpageSelected.value = initialActive.isSubpage

let resizeObserver: ResizeObserver | undefined

onMounted(() => {
	updateActiveTab()
	resizeObserver = new ResizeObserver(resetSliderPosition)
	if (scrollContainer.value) {
		resizeObserver.observe(scrollContainer.value)
	}
})

onUnmounted(() => {
	resizeObserver?.disconnect()
	window.clearTimeout(pendingNavigation)
})

watch(
	() => [route.path, route.query],
	() => {
		if (props.mode === 'navigation') {
			updateActiveTab()
		}
	},
)

watch(
	() => props.activeIndex,
	() => {
		if (props.mode === 'local') {
			updateActiveTab()
		}
	},
)

watch(
	() => props.links,
	async () => {
		await nextTick()
		updateActiveTab()
	},
	{ deep: true },
)
</script>

<style scoped>
.navtabs-slider {
	--navtabs-slider-bg: color-mix(in srgb, var(--color-brand) 25%, var(--surface-3));
}

:global(.light-mode) .navtabs-slider {
	--navtabs-slider-bg: var(--color-button-bg-selected);
}

.navtabs-slider.is-subpage {
	--navtabs-slider-bg: var(--color-button-bg);
}

.navtabs-slider-cap,
.navtabs-slider-middle {
	position: absolute;
	left: 0;
	top: 0;
	background: var(--navtabs-slider-bg);
	will-change: transform;
	transition: background-color 150ms ease;
}

.navtabs-slider-cap {
	border-radius: 9999px;
}

.navtabs-slider-middle {
	width: 1px;
	transform-origin: left center;
}

.tab-color {
	transition: color 100ms cubic-bezier(0.4, 0, 0.2, 1);
}

.tab-color-delayed .tab-color {
	transition-delay: 100ms;
}
</style>
