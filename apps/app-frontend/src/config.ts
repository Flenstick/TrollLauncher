const trimTrailingSlash = (url: string) => url.replace(/\/$/, '')

const siteUrl = trimTrailingSlash(import.meta.env.MODRINTH_URL || 'https://modrinth.com')
const labrinthBaseUrl = trimTrailingSlash(
	import.meta.env.MODRINTH_API_BASE_URL || 'https://api.modrinth.com',
)
const archonBaseUrl = trimTrailingSlash(
	import.meta.env.MODRINTH_ARCHON_BASE_URL || 'https://archon.modrinth.com',
)
const sharedInstancesBaseUrl = trimTrailingSlash(
	import.meta.env.SHARED_INSTANCES_API_BASE_URL || 'https://shared-instances.modrinth.com',
)

/**
 * GitHub repository with TrollLauncher releases, as "owner/repo".
 * Can be overridden with VITE_LAUNCHER_GITHUB_REPO for test builds.
 */
const launcherRepo = import.meta.env.VITE_LAUNCHER_GITHUB_REPO || 'Flenstick/TrollLauncher'

export const config = {
	siteUrl,
	launcherRepo,
	stripePublishableKey:
		import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY ||
		'pk_test_51JbFxJJygY5LJFfKV50mnXzz3YLvBVe2Gd1jn7ljWAkaBlRz3VQdxN9mXcPSrFbSqxwAb0svte9yhnsmm7qHfcWn00R611Ce7b',
	labrinthBaseUrl,
	archonBaseUrl,
	sharedInstancesBaseUrl,
}
