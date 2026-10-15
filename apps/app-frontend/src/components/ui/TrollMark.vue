<script setup lang="ts">
defineProps<{
	/** Draws thin gaps between blocks; use at sizes above ~48px. */
	gaps?: boolean
}>()
</script>

<template>
	<svg
		viewBox="-4 -4 250.5 323.0"
		class="troll-mark"
		:class="{ 'with-gaps': gaps }"
		aria-hidden="true"
	>
		<path class="face-top" d="M121.2 175.0L181.9 210.0L121.2 245.0L60.6 210.0Z" />
		<path class="face-left" d="M60.6 210.0L121.2 245.0L121.2 315.0L60.6 280.0Z" />
		<path class="face-right" d="M121.2 245.0L181.9 210.0L181.9 280.0L121.2 315.0Z" />
		<path class="face-top" d="M121.2 105.0L181.9 140.0L121.2 175.0L60.6 140.0Z" />
		<path class="face-left" d="M60.6 140.0L121.2 175.0L121.2 245.0L60.6 210.0Z" />
		<path class="face-right" d="M121.2 175.0L181.9 140.0L181.9 210.0L121.2 245.0Z" />
		<path class="face-top" d="M60.6 0.0L121.2 35.0L60.6 70.0L0.0 35.0Z" />
		<path class="face-left" d="M0.0 35.0L60.6 70.0L60.6 140.0L0.0 105.0Z" />
		<path class="face-right" d="M60.6 70.0L121.2 35.0L121.2 105.0L60.6 140.0Z" />
		<path class="face-top" d="M121.2 35.0L181.9 70.0L121.2 105.0L60.6 70.0Z" />
		<path class="face-left" d="M60.6 70.0L121.2 105.0L121.2 175.0L60.6 140.0Z" />
		<path class="face-right" d="M121.2 105.0L181.9 70.0L181.9 140.0L121.2 175.0Z" />
		<path class="face-top" d="M181.9 70.0L242.5 105.0L181.9 140.0L121.2 105.0Z" />
		<path class="face-left" d="M121.2 105.0L181.9 140.0L181.9 210.0L121.2 175.0Z" />
		<path class="face-right" d="M181.9 140.0L242.5 105.0L242.5 175.0L181.9 210.0Z" />
	</svg>
</template>

<style scoped>
.troll-mark {
	--troll-mark-top: color-mix(in srgb, var(--color-brand) 72%, white);
	--troll-mark-left: var(--color-brand);
	--troll-mark-right: color-mix(in srgb, var(--color-brand) 70%, black);
}

.face-top {
	fill: var(--troll-mark-top);
}

.face-left {
	fill: var(--troll-mark-left);
}

.face-right {
	fill: var(--troll-mark-right);
}

.with-gaps path {
	stroke: var(--troll-mark-gap, var(--color-bg));
	stroke-width: 5;
	stroke-linejoin: round;
}
</style>
