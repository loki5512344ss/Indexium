<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchMods, type ModListItem } from '$lib/api';

	let query = $state('');
	let gameVersion = $state<string | null>(null);
	let loader = $state<string | null>(null);
	let category = $state<string | null>(null);
	let sort: 'relevance' | 'newest' | 'popular' = $state('relevance');
	let page = $state(1);
	let mods = $state<ModListItem[]>([]);
	let total = $state(0);
	let pages = $state(1);
	let loading = $state(true);
	let versionFilter = $state('');
	let showAllVersions = $state(false);

	const gameVersions = ['1.21.1','1.21','1.20.1','1.20','1.19.4','1.19.2','1.18.2','1.17.1','1.16.5','1.12.2'];
	const loaders = ['fabric','forge','neoforge','quilt'];
	const categories = ['Optimization','Library','Utility','Adventure','Technology','World Generation'];
	let filteredVersions = $derived(gameVersions.filter(v => !versionFilter || v.includes(versionFilter)));
	let visibleVersions = $derived(showAllVersions ? filteredVersions : filteredVersions.slice(0,4));

	async function load() {
		loading = true;
		try {
			const res = await fetchMods({ query: query || undefined, gameVersion: gameVersion ?? undefined, loader: loader ?? undefined, page, limit: 20, sort });
			mods = res.data; total = res.pagination.total; pages = res.pagination.pages;
		} catch { mods = []; } finally { loading = false; }
	}
	onMount(load);
	function toggle(v: string | null, setter: (x: string | null) => void, cur: string | null) { setter(cur === v ? null : v); load(); }
</script>

<svelte:head><title>Discover mods - Indexium</title></svelte:head>

<div style="max-width:1280px;margin:0 auto;padding:0 clamp(16px,4vw,32px);">
	<div style="display:flex;gap:8px;align-items:center;padding:14px 0 8px;flex-wrap:wrap;border-bottom:1px solid var(--color-divider);margin-bottom:16px;">
		<a href="/mods" class="mono" style="padding:6px 12px;border-radius:999px;background:var(--color-accent);color:var(--color-bg);font-size:13px;font-weight:600;">Mods</a>
		<a href="/plugins" class="mono" style="padding:6px 12px;border-radius:999px;border:1px solid var(--color-divider);opacity:.7;font-size:13px;">Plugins</a>
		<a href="#" class="mono" style="padding:6px 12px;border-radius:999px;border:1px solid var(--color-divider);opacity:.5;font-size:13px;">Resource Packs</a>
		<a href="#" class="mono" style="padding:6px 12px;border-radius:999px;border:1px solid var(--color-divider);opacity:.5;font-size:13px;">Shaders</a>
	</div>

	<div style="display:flex;gap:16px;align-items:center;margin:16px 0;">
		<div style="position:relative;flex:1;max-width:560px;">
			<svg width="16" height="16" viewBox="0 0 256 256" fill="currentColor" style="position:absolute;left:12px;top:50%;transform:translateY(-50%);opacity:.5;"><path d="M229.66,218.34l-50.07-50.06a88.11,88.11,0,1,0-11.31,11.31l50.06,50.07a8,8,0,0,0,11.32-11.32ZM40,112a72,72,0,1,1,72,72A72.08,72.08,0,0,1,40,112Z"/></svg>
			<input class="input" placeholder="Search mods..." style="padding-left:36px;height:40px;" bind:value={query} oninput={load} />
		</div>
		<select class="input" style="width:160px;height:40px;" bind:value={sort} onchange={load}>
			<option value="relevance">Relevance</option><option value="newest">Newest</option><option value="popular">Popular</option>
		</select>
		<span class="mono" style="opacity:.6;font-size:12px;">{total} results</span>
	</div>

	<div style="display:grid;grid-template-columns:260px 1fr;gap:24px;align-items:start;">
		<!-- filters sidebar modrinth-like -->
		<aside style="position:sticky;top:76px;display:flex;flex-direction:column;gap:18px;">
			<div class="card" style="padding:14px;gap:10px;">
				<p class="mono" style="font-size:12px;letter-spacing:.06em;text-transform:uppercase;opacity:.7;margin:0;">Game version</p>
				<div style="position:relative;">
					<input class="input" placeholder="Filter versions..." style="height:32px;font-size:13px;padding-right:28px;" bind:value={versionFilter} />
					{#if versionFilter}<button onclick={() => versionFilter=''} style="position:absolute;right:6px;top:50%;transform:translateY(-50%);background:none;border:none;cursor:pointer;opacity:.5;">✕</button>{/if}
				</div>
				<div style="display:flex;flex-direction:column;gap:4px;max-height:180px;overflow:auto;">
					{#each visibleVersions as v}<label class="radio" style="font-size:13px;"><input type="radio" name="gv" checked={gameVersion===v} onchange={() => toggle(v, (x)=>gameVersion=x, gameVersion)} /><span class="dot"></span>{v}</label>{/each}
					{#if !showAllVersions && filteredVersions.length > 4}<button class="mono" onclick={() => showAllVersions = true} style="text-align:left;font-size:12px;color:var(--color-accent);background:none;border:none;cursor:pointer;padding:4px 0;">... +{filteredVersions.length - 4} еще</button>{/if}
					{#if showAllVersions}<button class="mono" onclick={() => showAllVersions = false} style="text-align:left;font-size:12px;opacity:.6;background:none;border:none;cursor:pointer;padding:4px 0;">свернуть</button>{/if}
					{#if filteredVersions.length===0}<span class="mono text-muted" style="font-size:12px;">нет совпадений</span>{/if}
				</div>
			</div>
			<div class="card" style="padding:14px;gap:10px;">
				<p class="mono" style="font-size:12px;letter-spacing:.06em;text-transform:uppercase;opacity:.7;margin:0;">Loader</p>
				{#each loaders as l}<label class="radio" style="font-size:13px;"><input type="radio" name="loader" checked={loader===l} onchange={() => toggle(l, (x)=>loader=x, loader)} /><span class="dot"></span>{l}</label>{/each}
			</div>
			<div class="card" style="padding:14px;gap:10px;">
				<p class="mono" style="font-size:12px;letter-spacing:.06em;text-transform:uppercase;opacity:.7;margin:0;">Category</p>
				{#each categories as c}<label class="radio" style="font-size:13px;"><input type="radio" name="cat" checked={category===c} onchange={() => toggle(c, (x)=>category=x, category)} /><span class="dot"></span>{c}</label>{/each}
			</div>
		</aside>

		<!-- list -->
		<main style="display:flex;flex-direction:column;gap:12px;">
			{#if loading}
				{#each Array(6) as _}<div class="card" style="height:110px;animation:shimmer 1.4s infinite linear;"></div>{/each}
			{:else if mods.length===0}
				<div class="card" style="padding:32px;text-align:center;opacity:.7;">No mods found - try different filters or <a href="/publish">publish</a> one.</div>
			{:else}
				{#each mods as m}
					<a href="/mod/{m.slug}" class="card" style="flex-direction:row;gap:14px;padding:14px;text-decoration:none;color:inherit;border:1px solid var(--color-divider);transition:border-color .15s;">
						<div style="width:64px;height:64px;border-radius:8px;background:linear-gradient(135deg,var(--color-accent-800),var(--color-accent-600));display:flex;align-items:center;justify-content:center;color:var(--color-bg);font-weight:700;font-size:20px;flex:none;">{m.name[0]}</div>
						<div style="flex:1;min-width:0;">
							<div style="display:flex;gap:8px;align-items:baseline;flex-wrap:wrap;">
								<span style="font-weight:600;font-size:15px;">{m.name}</span>
								<span class="text-muted" style="font-size:13px;">by {m.author}</span>
								<span class="mono" style="margin-left:auto;font-size:11px;opacity:.6;">{new Date(m.updated_at).toLocaleDateString()}</span>
							</div>
							<p style="margin:4px 0 6px;font-size:13px;opacity:.8;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;">{m.summary ?? 'No description'}</p>
							<div style="display:flex;gap:6px;flex-wrap:wrap;align-items:center;">
								{#each m.loaders as l}<span class="tag tag-outline mono" style="font-size:10px;padding:2px 6px;">{l}</span>{/each}
								{#each m.game_versions.slice(0,3) as gv}<span class="tag tag-neutral mono" style="font-size:10px;padding:2px 6px;">{gv}</span>{/each}
								<span class="mono" style="margin-left:auto;font-size:11px;opacity:.6;">{m.latest_version ?? ''}</span>
							</div>
						</div>
					</a>
				{/each}
				<div style="display:flex;gap:8px;justify-content:center;margin-top:12px;">
					<button class="btn btn-secondary" disabled={page<=1} onclick={() => {page--; load();}}>Prev</button>
					<span class="mono" style="padding:8px 12px;opacity:.7;">{page} / {pages}</span>
					<button class="btn btn-secondary" disabled={page>=pages} onclick={() => {page++; load();}}>Next</button>
				</div>
			{/if}
		</main>
	</div>
</div>
