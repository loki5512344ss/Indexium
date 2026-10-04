<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { BASE, fetchCollection, collectionExportUrl, type Collection } from '$lib/api';

	let slug = $derived($page.params.slug ?? '');
	let collection = $state<Collection | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let exporting = $state(false);

	onMount(async () => {
		try {
			collection = await fetchCollection(slug);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	});

	async function download(format: 'prism' | 'json') {
		if (!collection) return;
		exporting = true;
		try {
			const url =
				format === 'json'
					? `${BASE}/api/v1/collections/${encodeURIComponent(slug)}`
					: collectionExportUrl(slug, 'prism');
			const res = await fetch(url);
			if (!res.ok) throw new Error(res.statusText);
			const blob = await res.blob();
			const a = document.createElement('a');
			a.href = URL.createObjectURL(blob);
			a.download = format === 'prism' ? `${slug}.prism.json` : `${slug}.json`;
			document.body.appendChild(a);
			a.click();
			a.remove();
			setTimeout(() => URL.revokeObjectURL(a.href), 1000);
		} catch (e) {
			alert(e instanceof Error ? e.message : String(e));
		} finally {
			exporting = false;
		}
	}
</script>

<svelte:head><title>{collection?.title ?? slug} - Collections - Indexium</title></svelte:head>

<div style="max-width:1100px;margin:0 auto;padding:0 clamp(16px,4vw,32px);">
	<div style="margin:14px 0 8px;">
		<a href="/collections" class="mono" style="font-size:13px;text-decoration:none;opacity:.7;">← Все коллекции</a>
	</div>

	{#if loading}
		<p class="text-muted">Загрузка…</p>
	{:else if error}
		<div class="card" style="border:1px solid #a00;color:#ffb4b4;background:#2a1212;padding:16px;">
			{error}
			{#if error.toLowerCase().includes('not found') || error.includes('404')}
				— коллекция <code class="mono">{slug}</code> не найдена.
			{/if}
		</div>
	{:else if collection}
		<div style="display:grid;grid-template-columns:1fr 320px;gap:24px;margin-top:12px;">
			<div style="display:flex;flex-direction:column;gap:16px;">
				<div class="card" style="flex-direction:row;gap:16px;padding:18px;">
					<div
						style="width:64px;height:64px;border-radius:12px;background:linear-gradient(135deg,var(--color-accent-500),var(--color-accent-700));display:flex;align-items:center;justify-content:center;color:var(--color-bg);font-weight:700;font-size:26px;flex:none;"
					>
						{collection.title[0]?.toUpperCase() ?? '?'}
					</div>
					<div style="flex:1;min-width:0;">
						<h1 style="margin:0;font-size:24px;word-break:break-word;">{collection.title}</h1>
						<p class="mono" style="margin:4px 0 0;opacity:.6;font-size:12px;">/{collection.slug} · {new Date(collection.created_at).toLocaleString()}</p>
						<p style="margin:10px 0 0;opacity:.85;white-space:pre-wrap;word-break:break-word;">
							{collection.description ?? 'Без описания'}
						</p>
					</div>
				</div>

				<div class="card" style="padding:0;overflow:hidden;">
					<div
						style="padding:12px 16px;border-bottom:1px solid var(--color-divider);display:flex;align-items:center;gap:10px;"
					>
						<span style="font-weight:600;">Mods · {collection.mods.length}</span>
						<span class="mono" style="margin-left:auto;font-size:11px;opacity:.6;">{collection.slug}</span>
					</div>
					{#if collection.mods.length === 0}
						<p class="text-muted" style="padding:16px;margin:0;">В коллекции пока нет модов.</p>
					{:else}
						{#each collection.mods as m}
							<div
								style="display:flex;gap:10px;padding:12px 16px;border-bottom:1px solid var(--color-divider);align-items:center;flex-wrap:wrap;"
							>
								<a href="/mod/{m.slug}" class="mono" style="font-weight:600;text-decoration:none;color:var(--color-accent);"
									>{m.slug}</a
								>
								{#if m.version}<span class="tag tag-neutral mono" style="font-size:10px;">{m.version}</span>{/if}
								<a
									href="/mod/{m.slug}"
									class="btn btn-secondary"
									style="margin-left:auto;padding:6px 10px;font-size:12px;text-decoration:none;">Открыть</a
								>
							</div>
						{/each}
					{/if}
				</div>

				<div class="card" style="gap:10px;">
					<p style="margin:0;font-weight:600;">Raw JSON</p>
					<pre class="mono" style="margin:0;font-size:11px;opacity:.8;overflow:auto;background:#0B1210;padding:12px;border-radius:8px;border:1px solid var(--color-divider);">{JSON.stringify(
							collection,
							null,
							2
						)}</pre>
				</div>
			</div>

			<aside style="display:flex;flex-direction:column;gap:12px;position:sticky;top:16px;align-self:start;">
				<div class="card" style="gap:10px;">
					<button class="btn btn-primary" style="width:100%;" onclick={() => download('prism')} disabled={exporting}>
						{exporting ? 'Экспорт…' : '⤓ Экспорт Prism'}
					</button>
					<button class="btn btn-secondary" style="width:100%;" onclick={() => download('json')} disabled={exporting}>
						⤓ Скачать JSON
					</button>
					<p class="mono" style="font-size:11px;opacity:.6;margin:0;">
						GET /api/v1/collections/{collection.slug}/export?format=prism — Prism Launcher
					</p>
					<a
						href={collectionExportUrl(collection.slug, 'prism')}
						target="_blank"
						class="mono"
						style="font-size:11px;word-break:break-all;"
						>{collectionExportUrl(collection.slug, 'prism')}</a
					>
				</div>

				<div class="card" style="gap:8px;">
					<p style="margin:0;font-weight:600;">Детали</p>
					<p class="text-muted" style="margin:0;font-size:13px;">ID: <span class="mono" style="font-size:11px;word-break:break-all;">{collection.id}</span></p>
					<p class="text-muted" style="margin:0;font-size:13px;">Slug: <span class="mono">{collection.slug}</span></p>
					{#if collection.author_id !== null && collection.author_id !== undefined}
						<p class="text-muted" style="margin:0;font-size:13px;">Author ID: {collection.author_id}</p>
					{/if}
					<p class="text-muted" style="margin:0;font-size:13px;">
						Создана: {new Date(collection.created_at).toLocaleDateString()}
					</p>
				</div>

				<a href="/collections" class="btn btn-secondary" style="width:100%;text-decoration:none;">← К списку</a>
			</aside>
		</div>
	{/if}
</div>
