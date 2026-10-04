<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchMods, type ModListItem } from '$lib/api';

	const translations = {
		ru: {
			badge: 'GPL-3.0 - Open Source',
			title: 'Индекс модов с GitHub.',
			heroDesc: 'Только open-source. Только Releases.',
			modsBtn: 'Моды',
			pluginsBtn: 'Плагины',
			docsBtn: 'Документация',
			trending: 'Популярное',
			viewAll: 'Все моды',
			forDev: 'Для разработчиков',
			queryTitle: 'Один эндпоинт - весь индекс.',
			queryDesc: 'REST или CLI: проверенные хеши, лоадеры и версии - без хостинга файлов.',
			builtTitle: 'Поддержка ядер',
			ctaTitle: 'Опубликуй свой мод',
			ctaDesc: 'Вход через GitHub. Публичный репозиторий + LICENSE - и релизы индексируются сами.',
			ctaBtn: 'Войти через GitHub',
			searchPh: 'Поиск',
			footerDesc: 'Индекс open-source модов. Берем только GitHub Releases - SHA-256 - REST и CLI.',
		},
		en: {
			badge: 'GPL-3.0 - Open Source',
			title: 'Mod index from GitHub.',
			heroDesc: 'Open-source only. Releases only.',
			modsBtn: 'Mods',
			pluginsBtn: 'Plugins',
			docsBtn: 'Docs',
			trending: 'Trending',
			viewAll: 'All mods',
			forDev: 'For developers',
			queryTitle: 'One endpoint - the whole index.',
			queryDesc: 'REST or CLI: verified hashes, loaders and versions - no file hosting.',
			builtTitle: 'Server cores',
			ctaTitle: 'Publish your mod',
			ctaDesc: 'Sign in with GitHub. Public repo + LICENSE - releases index automatically.',
			ctaBtn: 'Sign in with GitHub',
			searchPh: 'Search',
			footerDesc: 'Index for open-source mods. GitHub Releases only - SHA-256 - REST & CLI.',
		}
	};
	let lang = $state<'ru' | 'en'>('ru');
	const t = $derived(translations[lang]);

	let heroTab = $state('curl');
	let activeTab = $state('curl');
	let loading = $state(true);
	let mods = $state<ModListItem[]>([]);

	const mockMods = [
		{ id: 'sodium', name: 'Sodium', author: 'jellysquid3', description: 'High FPS rendering engine.', version: 'v0.6.2', letter: 'S', gameVersion: '1.21.1', hearts: '2.4k', category: 'Optimization', loaders: ['Fabric', 'NeoForge'] },
		{ id: 'create', name: 'Create', author: 'simibubi', description: 'Gears and contraptions for automation.', version: 'v6.0.4', letter: 'C', gameVersion: '1.20.1', hearts: '1.9k', category: 'Automation', loaders: ['Forge', 'NeoForge'] },
		{ id: 'lithium', name: 'Lithium', author: 'jellysquid3', description: 'Server-side optimizations.', version: 'v0.13.0', letter: 'L', gameVersion: '1.21.1', hearts: '1.7k', category: 'Optimization', loaders: ['Fabric'] },
		{ id: 'ferritecore', name: 'FerriteCore', author: 'malte0811', description: 'Memory optimizations.', version: 'v7.0.1', letter: 'F', gameVersion: '1.20.1', hearts: '1.2k', category: 'Optimization', loaders: ['Fabric', 'Forge'] },
		{ id: 'iris', name: 'Iris Shaders', author: 'IrisShaders', description: 'Shader loader compatible with OptiFine.', version: 'v1.7.3', letter: 'I', gameVersion: '1.21.1', hearts: '1.1k', category: 'Optimization', loaders: ['Fabric'] },
		{ id: 'velocity', name: 'Velocity', author: 'PaperMC', description: 'High-performance proxy for networks.', version: 'v3.3.0', letter: 'V', gameVersion: 'Any', hearts: '860', category: 'Server', loaders: ['Proxy'] }
	];

	let visibleMods = $derived(mods.length ? mods.map(m => ({
		name: m.name, author: m.author, description: m.summary ?? '', version: m.latest_version ?? '-', letter: m.name[0]?.toUpperCase() ?? '?', gameVersion: m.game_versions[0] ?? '-', hearts: '♥ ' + Math.floor(Math.random()*900+100), loaders: m.loaders, id: m.slug
	})) : mockMods);

	async function loadMods() {
		loading = true;
		try { const res = await fetchMods({ page: 1, limit: 6 }); mods = res.data; } catch { mods = []; } finally { loading = false; }
	}
	onMount(loadMods);
</script>

<svelte:head><title>Indexium - {t.title}</title></svelte:head>

<div style="background:radial-gradient(circle, color-mix(in srgb, var(--color-accent) 9%, transparent) 1px, transparent 1px) 0 0/18px 18px, var(--color-bg);color:var(--color-text);min-height:100vh;">
	<nav class="nav" style="position:sticky;top:12px;z-index:50;background:color-mix(in srgb,var(--color-surface) 92%,transparent);backdrop-filter:blur(10px);border:1px solid var(--color-divider);border-radius:14px;max-width:1200px;margin:12px auto 0;padding:10px 16px;box-shadow:0 4px 20px rgba(0,0,0,0.25);">
		<span style="width:24px;height:24px;border-radius:6px;background:linear-gradient(135deg,var(--color-accent-500),var(--color-accent-700));display:inline-flex;align-items:center;justify-content:center;flex:none;margin-right:8px;"><svg width="13" height="13" viewBox="0 0 256 256" fill="var(--color-bg)"><path d="M223.68,66.15,135.68,18a15.88,15.88,0,0,0-15.36,0l-88,48.17a16,16,0,0,0-8.32,14v95.64a16,16,0,0,0,8.32,14l88,48.17a15.88,15.88,0,0,0,15.36,0l88-48.17a16,16,0,0,0,8.32-14V80.18A16,16,0,0,0,223.68,66.15ZM128,32,201.13,72,128,112,54.87,72Zm8,183.9V139.71l72-39.27v76.34ZM48,100.44l72,39.27v76.19L48,176.63Z"/></svg></span>
		<span class="nav-brand">Indexium</span>
		<a href="/mods">Mods</a>
		<a href="/publish">Publish</a>
		<a href="#">Docs</a>
		<a href="#">API</a>
		<div style="margin-left:auto;display:flex;border:1px solid var(--color-divider);border-radius:999px;overflow:hidden;">
			<button class="mono" onclick={() => lang = 'ru'} style="padding:6px 12px;font-size:12px;font-weight:600;border:none;cursor:pointer;background:{lang==='ru' ? 'var(--color-accent)' : 'transparent'};color:{lang==='ru' ? 'var(--color-bg)' : 'var(--color-text)'};">RU</button>
			<button class="mono" onclick={() => lang = 'en'} style="padding:6px 12px;font-size:12px;font-weight:600;border:none;cursor:pointer;background:{lang==='en' ? 'var(--color-accent)' : 'transparent'};color:{lang==='en' ? 'var(--color-bg)' : 'var(--color-text)'};">EN</button>
		</div>
		<a href="/publish" class="btn btn-primary" style="margin-left:8px;">{lang === 'ru' ? 'Войти' : 'Sign in'}</a>
	</nav>

	<div style="max-width:1200px;margin:0 auto;padding:0 clamp(20px,5vw,72px);">
		<section style="padding:72px 0 40px;text-align:center;">
			<div style="max-width:640px;margin:0 auto;">
				<span class="mono" style="display:inline-flex;font-size:11px;letter-spacing:.08em;text-transform:uppercase;padding:6px 12px;border-radius:999px;border:1px solid color-mix(in srgb,var(--color-accent) 30%,transparent);color:var(--color-accent);background:color-mix(in srgb,var(--color-accent) 8%,transparent);">{t.badge}</span>
				<h1 style="font-weight:700;font-size:clamp(36px,5vw,52px);line-height:1.05;letter-spacing:-0.03em;margin:16px 0 12px;color:var(--color-text);">{t.title}</h1>
				<p style="font-size:15px;opacity:.7;margin:0 0 24px;">{t.heroDesc}</p>
				<div style="display:flex;gap:12px;justify-content:center;flex-wrap:wrap;">
					<a href="/mods" class="btn btn-primary" style="padding:12px 22px;font-size:15px;min-width:140px;">{t.modsBtn}</a>
					<a href="/plugins" class="btn btn-secondary" style="padding:12px 22px;font-size:15px;min-width:140px;background:var(--color-surface);">{t.pluginsBtn}</a>
				</div>
				<p class="mono" style="font-size:11px;opacity:.45;margin-top:14px;">GitHub Releases - SHA-256 - loader-aware</p>
			</div>

			<div class="card elev-md" style="text-align:left;max-width:640px;margin:36px auto 0;padding:0;overflow:hidden;border:1px solid var(--color-divider);">
				<div style="display:flex;gap:0;padding:0 8px;background:color-mix(in srgb,var(--color-surface) 95%,transparent);border-bottom:1px solid var(--color-divider);">
					{#each [['curl','cURL'],['rust','Rust'],['gradle','Gradle'],['cli','CLI']] as [k,n]}
						<button onclick={() => heroTab = k} style="padding:10px 14px;font-size:12px;font-family:'JetBrains Mono',monospace;border:none;border-bottom:2px solid {heroTab===k ? 'var(--color-accent)' : 'transparent'};background:transparent;color:{heroTab===k ? 'var(--color-text)' : 'var(--color-neutral-400)'};cursor:pointer;">{n}</button>
					{/each}
				</div>
				<div style="padding:16px;background:#0B1210;overflow:auto;">
					{#if heroTab === 'curl'}
						<pre class="mono" style="margin:0;font-size:13px;line-height:1.6;"><span style="color:#7EE787;">curl</span> <span style="color:#E8E4D9;">https://api.indexium.dev/v1/mods/</span><span style="color:#4ADE80;">sodium</span> <span style="color:#8B949E;">\</span>
  <span style="color:#7EE787;">-H</span> <span style="color:#A5D6FF;">"Authorization: Bearer $TOKEN"</span></pre>
					{:else if heroTab === 'rust'}
						<pre class="mono" style="margin:0;font-size:13px;line-height:1.6;"><span style="color:#FF7B72;">let</span> <span style="color:#79C0FF;">m</span> = <span style="color:#FFA657;">indexium</span>::<span style="color:#79C0FF;">Client</span>::<span style="color:#79C0FF;">new</span>().<span style="color:#79C0FF;">mods</span>().<span style="color:#79C0FF;">get</span>(<span style="color:#A5D6FF;">"sodium"</span>).<span style="color:#79C0FF;">await</span>?;</pre>
					{:else if heroTab === 'gradle'}
						<pre class="mono" style="margin:0;font-size:13px;line-height:1.6;"><span style="color:#8B949E;">// build.gradle.kts</span>
<span style="color:#79C0FF;">implementation</span>(<span style="color:#A5D6FF;">"dev.indexium:client:1.4.0"</span>)</pre>
					{:else}
						<pre class="mono" style="margin:0;font-size:13px;line-height:1.6;"><span style="color:#7EE787;">$</span> <span style="color:#E8E4D9;">indexium</span> <span style="color:#79C0FF;">query</span> <span style="color:#A5D6FF;">sodium</span> <span style="color:#FFA657;">--loader</span> <span style="color:#A5D6FF;">fabric</span>
<span style="color:#8B949E;">→ sodium@0.6.2  sha256:9f2a...c31  fabric</span></pre>
					{/if}
				</div>
				<div style="padding:12px 16px;background:var(--color-surface);border-top:1px solid var(--color-divider);display:flex;gap:6px;flex-wrap:wrap;">
					<span class="mono" style="font-size:11px;opacity:.6;">response</span>
					<pre class="mono" style="margin:0;font-size:11px;opacity:.8;overflow:auto;flex:1;text-align:right;">{'{'}"mod":"sodium","sha256":"9f2a...c31"{'}'}</pre>
				</div>
			</div>
		</section>

		<section style="padding:0 0 48px;">
			<div style="display:flex;align-items:baseline;justify-content:space-between;margin-bottom:18px;">
				<h2 style="font-weight:600;font-size:24px;margin:0;">{t.trending}</h2>
				<a href="/mods" style="font-size:13px;color:var(--color-accent);"> {t.viewAll} →</a>
			</div>
			{#if loading}
				<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:12px;">{#each Array(6) as _}<div class="card" style="height:96px;"></div>{/each}</div>
			{:else}
				<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:12px;">
					{#each visibleMods as m}
						<a href="/mod/{m.id}" class="card elev-sm" style="padding:12px;gap:8px;text-decoration:none;color:inherit;border:1px solid var(--color-divider);">
							<div style="display:flex;gap:10px;align-items:center;">
								<div style="width:36px;height:36px;border-radius:8px;background:linear-gradient(135deg,var(--color-accent-500),var(--color-accent-700));color:var(--color-bg);display:flex;align-items:center;justify-content:center;font-weight:700;flex:none;">{m.letter}</div>
								<div style="min-width:0;"><p class="card-title" style="margin:0;font-size:14px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">{m.name}</p><p class="card-meta" style="margin:0;">by {m.author}</p></div>
								<span class="mono" style="margin-left:auto;font-size:11px;opacity:.5;">{m.version}</span>
							</div>
							<p class="card-body" style="font-size:13px;min-height:2.2em;">{m.description}</p>
							<div style="display:flex;gap:6px;flex-wrap:wrap;"><span class="tag tag-outline mono" style="font-size:10px;">{m.loaders[0] ?? 'fabric'}</span><span class="tag tag-neutral mono" style="font-size:10px;">{m.gameVersion}</span></div>
							<div style="display:flex;align-items:center;justify-content:space-between;margin-top:4px;"><span style="font-size:13px;color:var(--color-accent);">♥ {m.hearts}</span><span class="mono text-muted" style="font-size:11px;">{m.loaders.join(', ')}</span></div>
						</a>
					{/each}
				</div>
			{/if}
		</section>

		<section style="padding:0 0 48px;display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:32px;">
			<div>
				<span class="mono" style="font-size:11px;letter-spacing:.08em;text-transform:uppercase;color:var(--color-accent);">{t.forDev}</span>
				<h2 style="font-weight:600;font-size:26px;margin:10px 0 10px;">{t.queryTitle}</h2>
				<p style="opacity:.7;font-size:14px;line-height:1.6;margin:0 0 16px;">{t.queryDesc}</p>
				<div class="seg" style="margin-bottom:12px;">
					{#each [['curl','cURL'],['node','Node.js'],['cli','CLI']] as [k,n]}<label class="seg-opt"><input type="radio" name="devtab" checked={activeTab===k} onchange={() => activeTab=k} /><span class="mono">{n}</span></label>{/each}
				</div>
				<div style="background:#0B1210;border:1px solid var(--color-divider);border-radius:8px;padding:14px;overflow:auto;">
					{#if activeTab === 'curl'}
						<pre class="mono" style="margin:0;font-size:12px;line-height:1.6;"><span style="color:#7EE787;">curl</span> https://api.indexium.dev/v1/mods/<span style="color:#4ADE80;">sodium</span> <span style="color:#8B949E;">\</span>
  <span style="color:#7EE787;">-H</span> <span style="color:#A5D6FF;">"Authorization: Bearer $TOKEN"</span></pre>
					{:else if activeTab === 'node'}
						<pre class="mono" style="margin:0;font-size:12px;line-height:1.6;"><span style="color:#FF7B72;">const</span> <span style="color:#79C0FF;">res</span> = <span style="color:#FF7B72;">await</span> <span style="color:#79C0FF;">indexium</span>.<span style="color:#79C0FF;">mods</span>.<span style="color:#79C0FF;">get</span>(<span style="color:#A5D6FF;">'sodium'</span>);
<span style="color:#79C0FF;">console</span>.<span style="color:#79C0FF;">log</span>(<span style="color:#79C0FF;">res</span>.<span style="color:#79C0FF;">sha256</span>);</pre>
					{:else}
						<pre class="mono" style="margin:0;font-size:12px;line-height:1.6;"><span style="color:#7EE787;">$</span> indexium query sodium <span style="color:#FFA657;">--loader</span> <span style="color:#A5D6FF;">fabric</span></pre>
					{/if}
				</div>
			</div>
			<div class="card elev-md" style="padding:0;overflow:hidden;align-self:start;">
				<div style="display:flex;align-items:center;gap:8px;padding:10px 14px;border-bottom:1px solid var(--color-divider);background:color-mix(in srgb,var(--color-surface) 95%,transparent);">
					<span style="width:10px;height:10px;border-radius:50%;background:#ff5f56;border:1px solid #e0443e;"></span>
					<span style="width:10px;height:10px;border-radius:50%;background:#ffbd2e;border:1px solid #dea123;"></span>
					<span style="width:10px;height:10px;border-radius:50%;background:#4ade80;border:1px solid #22c55e;"></span>
					<span class="mono" style="font-size:11px;opacity:.5;margin-left:8px;">indexium --async</span>
				</div>
				<div class="mono" style="padding:16px;font-size:12px;line-height:1.8;opacity:.8;background:#0B1210;color:#E8E4D9;"><div><span style="color:#4ADE80;">$</span> indexium index ./mods --async</div><div style="opacity:.7;">→ discovered 214 jar files</div><div>→ hashing <span style="color:#4ADE80;">sha-256</span>... done</div><div>→ indexed 214/214 mods in 1.8s</div><div style="color:#4ADE80;">✓ job complete <span style="opacity:.5;">#a13f9c</span><span style="display:inline-block;width:7px;height:13px;background:#4ADE80;margin-left:6px;vertical-align:-2px;animation:blink 1s step-end infinite;"></span></div></div>
			</div>
		</section>

		<section style="padding:0 0 32px;">
			<h2 style="font-weight:600;font-size:22px;margin:0 0 12px;">{t.builtTitle}</h2>
			<div style="display:flex;flex-wrap:wrap;gap:8px;">
				{#each ['Paper','Purpur','Folia','Spigot','Bukkit','Velocity','Waterfall','Fabric','Forge','NeoForge','Quilt','Leaves','Pufferfish'] as core}
					<span class="tag mono" style="background:var(--color-surface);border:1px solid var(--color-divider);color:var(--color-text);padding:6px 12px;font-size:12px;">{core}</span>
				{/each}
				<a href="https://pumpkinmc.org" target="_blank" class="tag mono" style="background:var(--color-accent);border:1px solid var(--color-accent);color:var(--color-bg);padding:6px 12px;font-size:12px;font-weight:700;text-decoration:none;">Pumpkin <span style="opacity:.7;font-weight:400;margin-left:4px;">Rust - NEW</span></a>
			</div>
			<p class="mono text-muted" style="font-size:11px;margin-top:10px;">Fabric / Forge / NeoForge / Quilt - автоопределение по jar. Плагины - по ядру сервера. <a href="https://pumpkinmc.org" target="_blank" style="color:var(--color-accent);">Pumpkin</a> - Rust, 5ms старт, 100MB RAM.</p>
		</section>

		<section style="padding:32px 0 40px;text-align:center;border-top:1px solid var(--color-divider);">
			<h3 style="font-weight:600;font-size:22px;margin:0 0 8px;">{t.ctaTitle}</h3>
			<p style="opacity:.7;font-size:14px;margin:0 0 16px;max-width:48ch;margin-inline:auto;">{t.ctaDesc}</p>
			<a href="/publish" class="btn btn-primary" style="padding:12px 24px;">{t.ctaBtn} — GitHub</a>
		</section>
	</div>
	<footer style="border-top:1px solid var(--color-divider);">
		<div style="max-width:1200px;margin:0 auto;padding:24px clamp(20px,5vw,72px);display:flex;justify-content:space-between;flex-wrap:wrap;gap:12px;"><span class="text-muted" style="font-size:12px;">© 2026 Indexium - Open-source mods only.</span><span class="mono text-muted" style="font-size:11px;">GPL-3.0 · GitHub</span></div>
	</footer>
</div>
