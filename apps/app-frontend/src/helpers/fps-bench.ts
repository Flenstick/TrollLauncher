import type { Router } from 'vue-router'

/** Dev-only smoothness benchmark: scrolls the page and switches tabs, logging frame stats. */

function nextFrame(): Promise<number> {
	return new Promise((resolve) => requestAnimationFrame(resolve))
}

function wait(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms))
}

function findScroller(): HTMLElement | null {
	let best: HTMLElement | null = null
	let bestRange = 0
	for (const element of document.querySelectorAll<HTMLElement>('*')) {
		const range = element.scrollHeight - element.clientHeight
		if (range < 300 || element.clientHeight <= bestRange) continue
		const overflow = getComputedStyle(element).overflowY
		if (overflow === 'auto' || overflow === 'scroll') {
			best = element
			bestRange = element.clientHeight
		}
	}
	return best
}

async function measure(label: string, frames: number, step: (i: number) => void): Promise<void> {
	const deltas: number[] = []
	let last = await nextFrame()
	for (let i = 0; i < frames; i++) {
		step(i)
		const now = await nextFrame()
		deltas.push(now - last)
		last = now
	}
	const sorted = [...deltas].sort((a, b) => a - b)
	const avg = deltas.reduce((sum, d) => sum + d, 0) / deltas.length
	const p95 = sorted[Math.floor(sorted.length * 0.95)]
	const worst = sorted[sorted.length - 1]
	const janky = deltas.filter((d) => d > 25).length
	console.warn(
		`[FPSBENCH] ${label}: avg ${(1000 / avg).toFixed(1)} fps, p95 ${p95.toFixed(1)} ms, worst ${worst.toFixed(1)} ms, janky ${janky}/${frames}`,
	)
}

async function clickTabs(router: Router): Promise<void> {
	await router.push('/browse/modpack')
	await wait(5000)
	console.warn('[FPSBENCH] tabs start')
	for (const label of ['Моды', 'Шейдеры', 'Сборки', 'Серверы', 'Моды', 'Сборки']) {
		const tab = [...document.querySelectorAll<HTMLElement>('nav a')].find(
			(element) => element.textContent?.trim() === label,
		)
		tab?.click()
		const samples: string[] = []
		for (let i = 0; i < 30; i++) {
			const cap = document.querySelector<HTMLElement>('.navtabs-slider-cap')
			samples.push(
				cap
					? `${Math.round(cap.getBoundingClientRect().left)}@${Math.round(performance.now() % 100000)}`
					: 'none',
			)
			await nextFrame()
		}
		console.warn(`[FPSBENCH] ${label}: ${samples.join(' ')}`)
		await wait(900)
	}
	console.warn('[FPSBENCH] done')
}

function clickText(text: string): boolean {
	const element = [...document.querySelectorAll<HTMLElement>('button, a, [role="tab"], li, div')]
		.filter((candidate) => candidate.children.length < 4)
		.find((candidate) => candidate.textContent?.trim() === text)
	element?.click()
	return Boolean(element)
}

async function step(name: string, ms = 2500): Promise<void> {
	console.warn(`[FPSBENCH] step ${name}`)
	await wait(ms)
}

/** Dev-only walkthrough that opens screens one by one so an external script can screenshot them. */
async function tour(router: Router): Promise<void> {
	await router.push('/')
	await step('home')
	;(window as unknown as { __trollOpenSettings?: () => void }).__trollOpenSettings?.()
	await wait(800)
	clickText('Внешний вид')
	await wait(800)
	await step('appearance')
	document.querySelector('.modal-body, [class*="overflow-y-auto"]')
	for (const element of document.querySelectorAll<HTMLElement>('[class*="overflow-y-auto"]')) {
		element.scrollTop = element.scrollHeight
	}
	await step('appearance-bottom')
	clickText('Трей и фон')
	await step('tray')
	clickText('Конфиденциальность')
	await step('privacy')
	document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
	window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
	await wait(800)
	document.querySelector<HTMLElement>('.app-sidebar button.button-base')?.click()
	await step('accounts')
	clickText('Добавить аккаунт Ely.by') || clickText('Войти через Ely.by')
	await step('elyby')
	const fill = (id: string, value: string) => {
		const input = document.querySelector<HTMLInputElement>(`#${id}, #${id} input`)
		if (!input) return
		input.value = value
		input.dispatchEvent(new Event('input', { bubbles: true }))
	}
	fill('elyby-username', 'trolllauncher-test-nonexistent')
	fill('elyby-password', 'wrong-password')
	await wait(300)
	clickText('Войти')
	await step('elyby-error', 4000)
	console.warn('[FPSBENCH] done')
}

async function contentScroll(router: Router): Promise<void> {
	await router.push(`/instance/${encodeURIComponent(import.meta.env.VITE_FPS_BENCH_INSTANCE)}`)
	await wait(8000)
	const scroller = findScroller()
	console.warn(
		`[FPSBENCH] scroller: ${scroller?.className ?? 'none'} range ${scroller ? scroller.scrollHeight - scroller.clientHeight : 0} rows ${document.querySelectorAll('[data-content-card-item]').length}`,
	)
	if (!scroller) return
	const shadows = new Map<string, number>()
	for (const element of document.querySelectorAll<HTMLElement>('*')) {
		for (const pseudo of [null, '::before', '::after']) {
			const shadow = getComputedStyle(element, pseudo).boxShadow
			if (!shadow || shadow === 'none') continue
			const rect = element.getBoundingClientRect()
			const key = `${element.tagName.toLowerCase()}${pseudo ?? ''}.${[...element.classList].slice(0, 5).join('.')} ${Math.round(rect.width)}x${Math.round(rect.height)}`
			shadows.set(key, (shadows.get(key) ?? 0) + 1)
		}
	}
	for (const [key, count] of shadows) console.warn(`[FPSBENCH] shadow ${count}x ${key}`)
	const variants: Array<[string, string]> = [
		['baseline', ''],
		['no table shadow', '[role="table"] { box-shadow: none !important; }'],
		['no rowgroup shadow', '[role="rowgroup"] { box-shadow: none !important; }'],
		['no row-level shadow', '[data-content-card-item], [data-content-card-item] * { box-shadow: none !important; }'],
		['no pseudo shadow', '*::before, *::after { box-shadow: none !important; }'],
		['no shadow', '* { box-shadow: none !important; }'],
		['no filter', '* { filter: none !important; }'],
	]
	const style = document.head.appendChild(document.createElement('style'))
	for (const [name, css] of variants) {
		style.textContent = css
		scroller.scrollTop = 0
		await wait(300)
		let js = 0
		await measure(`content ${name}`, 240, (i) => {
			const t = performance.now()
			scroller.scrollTop += i < 120 ? 25 : -25
			js += performance.now() - t
		})
		console.warn(`[FPSBENCH]   sync scroll js ${js.toFixed(0)} ms`)
	}
	style.textContent = ''
	scroller.scrollTop = 0
	await wait(300)
	await measure('content small steps', 240, (i) => (scroller.scrollTop += i < 120 ? 3 : -3))
	// Time spent in rendering a range shift (Vue patch + layout).
	const rows = () => document.querySelectorAll('[data-content-card-item]').length
	let worst = 0
	let total = 0
	for (let i = 0; i < 40; i++) {
		await nextFrame()
		scroller.scrollTop += 74
		scroller.dispatchEvent(new Event('scroll'))
		await nextFrame()
		const t = performance.now()
		void document.body.offsetHeight
		const d = performance.now() - t
		total += d
		worst = Math.max(worst, d)
	}
	console.warn(`[FPSBENCH] forced layout after row shift: avg ${(total / 40).toFixed(1)} ms, worst ${worst.toFixed(1)} ms, rows ${rows()}`)
	console.warn('[FPSBENCH] done')
}

/** Dev-only: walks through the main screens so an external script can take screenshots. */
async function shots(router: Router): Promise<void> {
	const { getCurrentWindow, LogicalPosition, LogicalSize } = await import('@tauri-apps/api/window')
	const win = getCurrentWindow()
	void win.setSize(new LogicalSize(1680, 1000)).catch(() => {})
	void win.setPosition(new LogicalPosition(0, 0)).catch(() => {})
	await wait(1500)
	const instance = import.meta.env.VITE_FPS_BENCH_INSTANCE
	await router.push('/')
	await step('home', 6000)
	await router.push('/browse/modpack')
	await step('browse', 9000)
	await router.push(`/instance/${encodeURIComponent(instance)}`)
	await step('instance', 7000)
	await router.push('/skins')
	await wait(5000)
	clickText('OptiFine')
	await step('skins', 5000)
	await router.push('/')
	await wait(1500)
	;(window as unknown as { __trollOpenSettings?: () => void }).__trollOpenSettings?.()
	await wait(1200)
	clickText('Производительность')
	await step('performance', 3000)
	clickText('Внешний вид')
	await step('appearance', 3000)
	console.warn('[FPSBENCH] done')
}

export async function runFpsBench(router: Router): Promise<void> {
	if (import.meta.env.VITE_FPS_BENCH_MODE === 'tour') {
		await wait(Number(import.meta.env.VITE_FPS_BENCH_DELAY ?? 20000))
		return tour(router)
	}
	await wait(Number(import.meta.env.VITE_FPS_BENCH_DELAY ?? 20000))
	if (import.meta.env.VITE_FPS_BENCH_MODE === 'tabs') return clickTabs(router)
	if (import.meta.env.VITE_FPS_BENCH_MODE === 'content') return contentScroll(router)
	if (import.meta.env.VITE_FPS_BENCH_MODE === 'shots') return shots(router)
	if (import.meta.env.VITE_FPS_BENCH_MODE === 'skins') {
		await router.push('/skins')
		await step('skins', 6000)
		clickText('Забрать скин с аккаунта')
		await step('skins-sync', 5000)
		document.querySelectorAll<HTMLElement>('.skin-card')[0]?.click()
		await step('skins-second', 5000)
		clickText('OptiFine')
		await step('skins-cape', 5000)
		console.warn('[FPSBENCH] done')
		return
	}
	await measure('idle', 120, () => {})
	await router.push('/browse/mod')
	await wait(6000)

	const scroller = findScroller()
	console.warn(
		`[FPSBENCH] scroller: ${scroller?.className ?? 'none'} range ${scroller ? scroller.scrollHeight - scroller.clientHeight : 0}`,
	)
	if (scroller) {
		const variants: Array<[string, string]> = [
			['baseline', ''],
			['no backdrop', '* { backdrop-filter: none !important; -webkit-backdrop-filter: none !important; }'],
			['no shadow', '* { box-shadow: none !important; }'],
			['no filter', '* { filter: none !important; }'],
			['no app frame', '.app-contents::before { display: none !important; }'],
			['frame without blur', '.app-contents::before { box-shadow: none !important; }'],
			['no pseudo shadow', '*::before, *::after { box-shadow: none !important; }'],
			['no transition', '* { transition: none !important; animation: none !important; }'],
			[
				'flat',
				'* { backdrop-filter: none !important; box-shadow: none !important; filter: none !important; transition: none !important; animation: none !important; }',
			],
		]
		const style = document.head.appendChild(document.createElement('style'))
		for (const [name, css] of variants) {
			style.textContent = css
			scroller.scrollTop = 0
			await nextFrame()
			await measure(`scroll ${name}`, 240, (i) => (scroller.scrollTop += i < 120 ? 25 : -25))
		}
		style.remove()
	}

	const synthetic = document.body.appendChild(document.createElement('div'))
	synthetic.style.cssText =
		'position:fixed;inset:0;z-index:99999;overflow-y:auto;background:#16181c;color:#ccc;font:14px sans-serif'
	synthetic.innerHTML = Array.from(
		{ length: 400 },
		(_, i) => `<p style="margin:8px 16px">Synthetic line ${i} lorem ipsum dolor sit amet</p>`,
	).join('')
	await nextFrame()
	await measure('scroll synthetic', 240, (i) => (synthetic.scrollTop += i < 120 ? 25 : -25))
	const appRoot = document.getElementById('app')
	const canvases = [...document.querySelectorAll('canvas')].map(
		(canvas) => `${canvas.width}x${canvas.height}${canvas.offsetParent ? '' : ' hidden'}`,
	)
	console.warn(`[FPSBENCH] canvases: ${canvases.join(', ') || 'none'}`)
	if (appRoot) appRoot.style.display = 'none'
	await nextFrame()
	await measure('scroll synthetic app hidden', 240, (i) => (synthetic.scrollTop += i < 120 ? 25 : -25))
	if (appRoot) appRoot.style.display = ''
	await nextFrame()

	const describe = (element: Element) =>
		`${element.tagName.toLowerCase()}.${[...element.classList].slice(0, 4).join('.')}`
	const quickFps = async () => {
		const start = await nextFrame()
		for (let i = 0; i < 60; i++) {
			synthetic.scrollTop += i < 30 ? 25 : -25
			await nextFrame()
		}
		return (60 * 1000) / (performance.now() - start)
	}
	const probe = async (elements: Element[], depth: number) => {
		for (const element of elements) {
			const rect = element.getBoundingClientRect()
			if (rect.width * rect.height < 2000 && depth > 0) continue
			const html = element as HTMLElement
			const previous = html.style.display
			html.style.display = 'none'
			await nextFrame()
			const fps = await quickFps()
			html.style.display = previous
			console.warn(`[FPSBENCH] hide d${depth} ${describe(element)}: ${fps.toFixed(0)} fps`)
			if (fps > base * 1.25 && depth < 9) await probe([...element.children], depth + 1)
		}
	}
	const base = await quickFps()
	console.warn(`[FPSBENCH] bisect base ${base.toFixed(0)} fps`)
	if (appRoot && import.meta.env.VITE_FPS_BENCH_BISECT === 'true') await probe([...appRoot.children], 0)
	synthetic.style.width = '400px'
	synthetic.style.height = '300px'
	await nextFrame()
	await measure('scroll synthetic small', 240, (i) => (synthetic.scrollTop += i < 120 ? 25 : -25))
	synthetic.remove()

	const tabs = ['mod', 'resourcepack', 'shader', 'modpack']
	await measure('tab switch', 240, (i) => {
		if (i % 60 === 0) void router.push(`/browse/${tabs[(i / 60) % tabs.length]}`)
	})
	console.warn('[FPSBENCH] done')
}
