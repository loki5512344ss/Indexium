<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { fetchMods } from '$lib/api';
	let login = $derived($page.params.login ?? 'unknown');
	let mods: any[] = $state([]);
	let loading = $state(true);
	onMount(async () => {
		try { const r = await fetchMods({ query: login }); mods = r.data.filter(m => m.author.toLowerCase()===login.toLowerCase()); } catch {} finally { loading=false; }
	});
</script>

<svelte:head><title>{login} - Indexium</title></svelte:head>

<div style="max-width:1100px;margin:0 auto;padding:0 clamp(16px,4vw,32px);">
	<div class="card" style="flex-direction:row;gap:16px;padding:20px;margin-top:20px;">
		<div style="width:72px;height:72px;border-radius:50%;background:linear-gradient(135deg,var(--color-accent-500),var(--color-accent-700));display:flex;align-items:center;justify-content:center;color:var(--color-bg);font-weight:700;font-size:28px;flex:none;">{login[0]?.toUpperCase()}</div>
		<div>
			<h1 style="margin:0;">{login}</h1>
			<p class="text-muted" style="margin:4px 0;"><a href="https://github.com/{login}" target="_blank">github.com/{login}</a> · Indexium author</p>
			<div style="display:flex;gap:8px;margin-top:8px;"><span class="tag tag-accent mono">verified</span><span class="tag tag-neutral mono">{mods.length} mods</span></div>
		</div>
	</div>
	<h2 style="margin:24px 0 12px;">Mods by {login}</h2>
	{#if loading}<p class="text-muted">Loading…</p>
	{:else if mods.length===0}<div class="card" style="padding:24px;text-align:center;opacity:.7;">No mods yet. <a href="/publish">Publish</a> one.</div>
	{:else}<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:12px;">{#each mods as m}<a href="/mod/{m.slug}" class="card" style="text-decoration:none;color:inherit;"><span style="font-weight:600;">{m.name}</span><span class="text-muted" style="font-size:13px;">{m.summary ?? ''}</span></a>{/each}</div>{/if}
</div>
