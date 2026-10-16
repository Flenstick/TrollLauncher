import { queryOptions } from '@tanstack/vue-query'
import { ref, watch } from 'vue'

import { config } from '@/config'

export type LauncherRelease = {
	id: number
	version: string
	title: string
	summary: string
	date: string
	url: string
	thumbnail?: string
	prerelease: boolean
}

type GithubRelease = {
	id: number
	tag_name: string
	name: string | null
	body: string | null
	html_url: string
	published_at: string | null
	created_at: string
	prerelease: boolean
	draft: boolean
}

const IMAGE_PATTERN = /!\[[^\]]*]\((\S+?)\)|<img[^>]+src="([^"]+)"/

/** Plain-text preview of a release body: no markdown, headings, links or images. */
function summarize(body: string): string {
	const text = body
		.replace(/<!--[\s\S]*?-->/g, '')
		.replace(/<[^>]+>/g, '')
		.replace(/!\[[^\]]*]\([^)]*\)/g, '')
		.replace(/\[([^\]]+)]\([^)]*\)/g, '$1')
		.replace(/^#+\s*/gm, '')
		.replace(/^[-*+]\s+/gm, '• ')
		.replace(/[*_`~]/g, '')
		.replace(/\n{2,}/g, '\n')
		.trim()
	return clip(text, SUMMARY_LENGTH)
}

const SUMMARY_LENGTH = 240

/** Cuts at the last whole line (or word) that fits and marks the cut with an ellipsis. */
function clip(text: string, max: number): string {
	if (text.length <= max) return text
	const head = text.slice(0, max)
	const lineEnd = head.lastIndexOf('\n')
	const cut = lineEnd > max / 2 ? lineEnd : head.lastIndexOf(' ')
	return `${head.slice(0, cut > 0 ? cut : max).trimEnd()}…`
}

export function launcherReleasesQueryOptions() {
	return queryOptions({
		queryKey: ['news', 'launcher-releases', config.launcherRepo] as const,
		enabled: config.launcherRepo !== '',
		queryFn: async ({ signal }): Promise<LauncherRelease[]> => {
			const response = await fetch(
				`https://api.github.com/repos/${config.launcherRepo}/releases?per_page=10`,
				{ signal, headers: { Accept: 'application/vnd.github+json' } },
			)
			if (!response.ok) {
				throw new Error(`Failed to fetch launcher releases: HTTP ${response.status}`)
			}
			const releases: GithubRelease[] = await response.json()
			return releases
				.filter((release) => !release.draft)
				.map((release) => {
					const body = release.body ?? ''
					const image = body.match(IMAGE_PATTERN)
					return {
						id: release.id,
						version: release.tag_name,
						title: release.name || release.tag_name,
						summary: summarize(body),
						date: release.published_at ?? release.created_at,
						url: release.html_url,
						thumbnail: image?.[1] ?? image?.[2],
						prerelease: release.prerelease,
					}
				})
		},
		staleTime: 30 * 60 * 1000,
		retry: 1,
		refetchOnWindowFocus: false,
	})
}

export type NewsSource = 'releases' | 'modrinth' | 'both' | 'none'

const NEWS_SOURCE_KEY = 'trolllauncher-news-source'

export function defaultNewsSource(): NewsSource {
	return config.launcherRepo ? 'releases' : 'modrinth'
}

function readNewsSource(): NewsSource {
	try {
		const stored = window.localStorage.getItem(NEWS_SOURCE_KEY)
		if (stored === 'releases' || stored === 'modrinth' || stored === 'both' || stored === 'none') {
			return stored
		}
	} catch {
		// storage blocked
	}
	return defaultNewsSource()
}

function writeNewsSource(source: NewsSource) {
	try {
		window.localStorage.setItem(NEWS_SOURCE_KEY, source)
	} catch {
		// storage blocked or full
	}
}

/** Which news the right sidebar shows; shared between the sidebar and settings. */
export const newsSource = ref<NewsSource>(readNewsSource())
watch(newsSource, writeNewsSource)
