<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchMods, ApiError } from '$lib/api';
	import type { ModListItem } from '$lib/api';

	let mods = $state<ModListItem[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let activeLoader = $state<string | null>(null);
	let pagination = $state<{ page: number; pages: number; total: number } | null>(null);

	const loaders: Array<{ label: string; value: string | null }> = [
		{ label: 'All', value: null },
		{ label: 'fabric', value: 'fabric' },
		{ label: 'quilt', value: 'quilt' },
		{ label: 'neoforge', value: 'neoforge' },
		{ label: 'forge', value: 'forge' }
	];

	async function load() {
		loading = true;
		error = null;
		try {
			const res = await fetchMods({
				page: 1,
				limit: 20,
				loader: activeLoader ?? undefined
			});
			mods = res.data;
			pagination = res.pagination;
		} catch (e) {
			if (e instanceof ApiError) error = `${e.status}: ${e.message}`;
			else if (e instanceof Error) error = e.message;
			else error = 'Unknown error';
		} finally {
			loading = false;
		}
	}

	function setLoader(value: string | null) {
		activeLoader = value;
		load();
	}

	onMount(load);
</script>

<svelte:head>
	<title>Indexium — Mods</title>
</svelte:head>

<main style="max-width:900px;margin:2rem auto;padding:0 1rem;font-family:system-ui,sans-serif">
	<h1>Indexium — Minecraft Mods</h1>
	<p style="color:#666">Лёгкий индекс модов поверх GitHub Releases CDN</p>

	<div style="display:flex;gap:0.5rem;margin:1rem 0;flex-wrap:wrap">
		{#each loaders as l}
			<button
				onclick={() => setLoader(l.value)}
				style="padding:0.4rem 0.8rem;border-radius:999px;border:1px solid {activeLoader === l.value
					? '#111'
					: '#ddd'};background:{activeLoader === l.value ? '#111' : '#fff'};color:{activeLoader ===
				l.value
					? '#fff'
					: '#111'};cursor:pointer"
			>
				{l.label}
			</button>
		{/each}
	</div>

	{#if loading}
		<p>Loading mods…</p>
	{:else if error}
		<div style="background:#fee;border:1px solid #fcc;padding:1rem;border-radius:8px;color:#900">
			<strong>Ошибка:</strong>
			{error}
			<button onclick={load} style="margin-left:1rem">Retry</button>
		</div>
	{:else if mods.length === 0}
		<p>Модов не найдено.</p>
	{:else}
		{#if pagination}
			<p style="color:#666;font-size:0.9rem">
				Найдено {pagination.total} · страница {pagination.page} из {pagination.pages}
			</p>
		{/if}
		<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:1rem">
			{#each mods as mod (mod.slug)}
				<article
					style="border:1px solid #e5e7eb;border-radius:12px;padding:1rem;display:flex;flex-direction:column;gap:0.5rem"
				>
					<h3 style="margin:0;font-size:1.05rem">{mod.name}</h3>
					<p style="margin:0;color:#555;font-size:0.9rem;min-height:2.2em">
						{mod.summary ?? 'Без описания'}
					</p>
					<div style="display:flex;gap:0.4rem;flex-wrap:wrap">
						{#each mod.loaders as loader}
							<span
								style="font-size:0.75rem;background:#eef;padding:0.2rem 0.5rem;border-radius:999px;border:1px solid #dde"
								>{loader}</span
							>
						{/each}
						{#each mod.game_versions as gv}
							<span
								style="font-size:0.75rem;background:#f3f4f6;padding:0.2rem 0.5rem;border-radius:999px"
								>{gv}</span
							>
						{/each}
					</div>
					<div style="margin-top:auto;display:flex;justify-content:space-between;align-items:center">
						<span style="font-size:0.8rem;color:#888">{mod.slug}</span>
						<a
							href="/mods/{mod.slug}"
							style="font-size:0.85rem;color:#2563eb;text-decoration:none">Подробнее →</a
						>
					</div>
				</article>
			{/each}
		</div>
	{/if}
</main>
