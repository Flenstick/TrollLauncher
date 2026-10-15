<script setup lang="ts">
import { NewspaperIcon } from '@modrinth/assets'
import { ButtonLink, defineMessages, NewsArticleCard, useVIntl } from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, watch } from 'vue'

import { config } from '@/config'

import { newsArticlesQueryOptions } from './queries'
import ReleaseCard from './release-card.vue'
import { launcherReleasesQueryOptions, newsSource } from './releases'

const { formatMessage } = useVIntl()
const messages = defineMessages({
	news: {
		id: 'app.news.title',
		defaultMessage: 'News',
	},
	viewAllNews: {
		id: 'app.news.view-all',
		defaultMessage: 'View all news',
	},
})

const showReleases = computed(
	() =>
		config.launcherRepo !== '' && (newsSource.value === 'releases' || newsSource.value === 'both'),
)
const showModrinth = computed(
	() =>
		newsSource.value === 'modrinth' ||
		newsSource.value === 'both' ||
		(newsSource.value === 'releases' && config.launcherRepo === ''),
)

const releasesQuery = useQuery(
	computed(() => ({ ...launcherReleasesQueryOptions(), enabled: showReleases.value })),
)
const newsQuery = useQuery(
	computed(() => ({ ...newsArticlesQueryOptions(), enabled: showModrinth.value })),
)

const releases = computed(() => releasesQuery.data.value?.slice(0, 4) ?? [])
const news = computed(
	() => newsQuery.data.value?.slice(0, newsSource.value === 'both' ? 2 : 4) ?? [],
)
const error = newsQuery.error
const releasesError = releasesQuery.error

watch([error, releasesError], ([newsError, releaseError]) => {
	if (newsError) console.error('Failed to fetch news articles', newsError)
	if (releaseError) console.error('Failed to fetch launcher releases', releaseError)
})

const hasContent = computed(
	() =>
		(showReleases.value && releases.value.length > 0) ||
		(showModrinth.value && news.value.length > 0),
)
</script>

<template>
	<div v-if="newsSource !== 'none' && hasContent" class="p-4 flex flex-col items-center">
		<h3 class="text-base mb-4 text-primary font-medium m-0 text-left w-full">
			{{ formatMessage(messages.news) }}
		</h3>
		<div class="space-y-4 flex flex-col items-center w-full">
			<template v-if="showReleases">
				<ReleaseCard v-for="release in releases" :key="release.id" :release="release" />
				<ButtonLink
					type="colored"
					color="brand"
					size="xl"
					:href="`https://github.com/${config.launcherRepo}/releases`"
					target="_blank"
					class="my-4"
				>
					<NewspaperIcon />
					Все обновления
				</ButtonLink>
			</template>
			<template v-if="showModrinth">
				<NewsArticleCard v-for="item in news" :key="item.path" :article="item" />
				<ButtonLink
					type="colored"
					color="brand"
					size="xl"
					href="https://modrinth.com/news"
					target="_blank"
					class="my-4"
				>
					<NewspaperIcon />
					{{ formatMessage(messages.viewAllNews) }}
				</ButtonLink>
			</template>
		</div>
	</div>
</template>
