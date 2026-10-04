<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		BASE,
		fetchCollections,
		createCollection,
		collectionExportUrl,
		type Collection
	} from '$lib/api';

	let collections = $state<Collection[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let query = $state('');
	let isAuthed = $state(false);
	let showCreate = $state(false);

	// create form
	let formSlug = $state('');
	let formTitle = $state('');
	let formDescription = $state('');
	let formModsRaw = $state(''); // comma separated "sodium, lithium@1.0.0"
	let creating = $state(false);
	let createError = $state<string | null>(null);

	let filtered = $derived(
		!query.trim()
			? collections
			: collections.filter(
					(c) =>
						c.title.toLowerCase().includes(query.toLowerCase()) ||
						c.slug.toLowerCase().includes(query.toLowerCase()) ||
						(c.description ?? '').toLowerCase().includes(query.toLowerCase())
				)
	);

	onMount(async () => {
		// auth check
		try {
			const r = await fetch(`${BASE}/api/v1/auth/me`, { credentials: 'include' });
			isAuthed = r.ok;
		} catch {
			isAuthed = false;
		}
		await load();
	});

	async function load() {
		loading = true;
		error = null;
		try {
			collections = await fetchCollections();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			collections = [];
		} finally {
			loading = false;
		}
	}

	function handleCreateClick() {
		if (!isAuthed) {
			goto('/publish');
			return;
		}
		showCreate = !showCreate;
		createError = null;
	}

	function parseMods(): { slug: string; version?: string }[] {
		if (!formModsRaw.trim()) return [];
		return formModsRaw
			.split(',')
			.map((s) => s.trim())
			.filter(Boolean)
			.map((entry) => {
				const [slug, version] = entry.split('@').map((x) => x.trim());
				return version ? { slug, version } : { slug };
			});
	}

	async function submitCreate() {
		if (!formSlug.trim() || !formTitle.trim()) {
			createError = 'slug и title обязательны';
			return;
		}
		creating = true;
		createError = null;
		try {
			const payload = {
				slug: formSlug.trim(),
				title: formTitle.trim(),
				description: formDescription.trim() || null,
				mods: parseMods()
			};
			const created = await createCollection(payload);
			collections = [created, ...collections];
			showCreate = false;
			formSlug = '';
			formTitle = '';
			formDescription = '';
			formModsRaw = '';
		} catch (e) {
			createError = e instanceof Error ? e.message : String(e);
		} finally {
			creating = false;
		}
	}

	function downloadPrism(slug: string) {
		const url = collectionExportUrl(slug, 'prism');
		// fetch then download blob to force filename
		fetch(url)
			.then((r) => {
				if (!r.ok) throw new Error(r.statusText);
				return r.blob();
			})
			.then((blob) => {
				const a = document.createElement('a');
				a.href = URL.createObjectURL(blob);
				a.download = `${slug}.prism.json`;
				document.body.appendChild(a);
				a.click();
				a.remove();
				setTimeout(() => URL.revokeObjectURL(a.href), 1000);
			})
			.catch(() => {
				// fallback open url
				window.open(url, '_blank');
			});
	}
</script>

<svelte:head><title>Collections - Indexium</title></svelte:head>

<div style="max-width:1280px;margin:0 auto;padding:0 clamp(16px,4vw,32px);">
	<!-- top nav pills like /mods -->
	<div
		style="display:flex;gap:8px;align-items:center;padding:14px 0 8px;flex-wrap:wrap;border-bottom:1px solid var(--color-divider);margin-bottom:16px;"
	>
		<a
			href="/mods"
			class="mono"
			style="padding:6px 12px;border-radius:999px;border:1px solid var(--color-divider);opacity:.7;font-size:13px;text-decoration:none;color:inherit;"
			>Mods</a
		>
		<a
			href="/plugins"
			class="mono"
			style="padding:6px 12px;border-radius:999px;border:1px solid var(--color-divider);opacity:.7;font-size:13px;text-decoration:none;color:inherit;"
			>Plugins</a
		>
		<a
			href="/collections"
			class="mono"
			style="padding:6px 12px;border-radius:999px;background:var(--color-accent);color:var(--color-bg);font-size:13px;font-weight:600;text-decoration:none;"
			>Collections</a
		>
		<span class="mono" style="margin-left:auto;opacity:.5;font-size:12px;">Moss Stone</span>
	</div>

	<div style="display:flex;gap:16px;align-items:center;margin:16px 0;flex-wrap:wrap;">
		<h1 style="margin:0;font-size:26px;flex:none;">Collections</h1>
		<span class="mono" style="opacity:.6;font-size:12px;">{collections.length} всего</span>
		<div style="flex:1;min-width:200px;max-width:460px;position:relative;">
			<svg
				width="16"
				height="16"
				viewBox="0 0 256 256"
				fill="currentColor"
				style="position:absolute;left:12px;top:50%;transform:translateY(-50%);opacity:.5;"
				><path
					d="M229.66,218.34l-50.07-50.06a88.11,88.11,0,1,0-11.31,11.31l50.06,50.07a8,8,0,0,0,11.32-11.32ZM40,112a72,72,0,1,1,72,72A72.08,72.08,0,0,1,40,112Z"
				/></svg
			>
			<input
				class="input"
				placeholder="Поиск по названию или slug..."
				style="padding-left:36px;height:40px;"
				bind:value={query}
			/>
		</div>
		<button class="btn btn-primary" style="margin-left:auto;" onclick={handleCreateClick}>
			{#if isAuthed}＋ Создать{:else}Войти чтобы создать{/if}
		</button>
	</div>

	{#if showCreate}
		<div
			class="card"
			style="padding:18px;gap:14px;margin-bottom:18px;border:1px solid var(--color-accent);background:color-mix(in srgb,var(--color-accent) 6%,var(--color-surface));"
		>
			<div style="display:flex;justify-content:space-between;align-items:center;">
				<p style="margin:0;font-weight:600;">Новая коллекция</p>
				<button class="btn btn-secondary" style="padding:6px 10px;font-size:12px;" onclick={() => (showCreate = false)}>Закрыть</button>
			</div>
			<div style="display:grid;grid-template-columns:1fr 1fr;gap:12px;">
				<label style="display:flex;flex-direction:column;gap:6px;">
					<span class="mono" style="font-size:11px;letter-spacing:.06em;text-transform:uppercase;opacity:.7;">slug*</span>
					<input class="input" placeholder="my-pack" bind:value={formSlug} />
				</label>
				<label style="display:flex;flex-direction:column;gap:6px;">
					<span class="mono" style="font-size:11px;letter-spacing:.06em;text-transform:uppercase;opacity:.7;">title*</span>
					<input class="input" placeholder="My Pack" bind:value={formTitle} />
				</label>
			</div>
			<label style="display:flex;flex-direction:column;gap:6px;">
				<span class="mono" style="font-size:11px;letter-spacing:.06em;text-transform:uppercase;opacity:.7;">description</span>
				<input class="input" placeholder="Короткое описание..." bind:value={formDescription} />
			</label>
			<label style="display:flex;flex-direction:column;gap:6px;">
				<span class="mono" style="font-size:11px;letter-spacing:.06em;text-transform:uppercase;opacity:.7;"
					>mods (через запятую, опц. @version)</span
				>
				<input class="input" placeholder="sodium, lithium@0.13.0, iris" bind:value={formModsRaw} />
				<span class="text-muted" style="font-size:11px;">пример: sodium, create@6.0.4</span>
			</label>
			{#if createError}<div class="card" style="padding:10px;background:#3a1a1a;border:1px solid #a00;color:#ffb4b4;font-size:13px;">{createError}</div>{/if}
			<button class="btn btn-primary" style="align-self:flex-start;" onclick={submitCreate} disabled={creating}>
				{creating ? 'Создание…' : 'Создать'}
			</button>
		</div>
	{/if}

	{#if loading}
		<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(320px,1fr));gap:12px;">
			{#each Array(6) as _}<div class="card" style="height:140px;animation:shimmer 1.4s infinite linear;background:linear-gradient(90deg,var(--color-surface) 25%,color-mix(in srgb,var(--color-surface) 85%,white) 50%,var(--color-surface) 75%);background-size:400px 100%;"></div>{/each}
		</div>
	{:else if error}
		<div class="card" style="padding:16px;border:1px solid #a00;color:#ffb4b4;background:#2a1212;">
			<strong>Ошибка загрузки:</strong>
			{error}
			<button class="btn btn-secondary" style="margin-left:12px;" onclick={load}>Повторить</button>
		</div>
	{:else if filtered.length === 0}
		<div class="card" style="padding:32px;text-align:center;opacity:.7;">
			{#if collections.length === 0}
				Коллекций пока нет — {#if isAuthed}<button class="btn btn-primary" style="margin-left:8px;" onclick={handleCreateClick}>создай первую</button>{:else}<a href="/publish">войди и создай</a>{/if}
			{:else}
				Ничего не найдено для "{query}" — попробуй другой запрос.
			{/if}
		</div>
	{:else}
		<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(340px,1fr));gap:14px;">
			{#each filtered as c (c.slug)}
				<a
					href="/collections/{c.slug}"
					class="card"
					style="padding:16px;gap:10px;text-decoration:none;color:inherit;border:1px solid var(--color-divider);transition:border-color .15s;"
				>
					<div style="display:flex;gap:12px;align-items:center;">
						<div
							style="width:48px;height:48px;border-radius:10px;background:linear-gradient(135deg,var(--color-accent-800),var(--color-accent-600));display:flex;align-items:center;justify-content:center;color:var(--color-bg);font-weight:700;font-size:18px;flex:none;"
						>
							{c.title[0]?.toUpperCase() ?? '?'}
						</div>
						<div style="min-width:0;flex:1;">
							<div style="font-weight:600;font-size:15px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">
								{c.title}
							</div>
							<div class="mono" style="font-size:11px;opacity:.6;">/{c.slug}</div>
						</div>
						<span class="mono" style="font-size:11px;opacity:.5;white-space:nowrap;"
							>{new Date(c.created_at).toLocaleDateString()}</span
						>
					</div>
					<p
						style="margin:0;font-size:13px;opacity:.8;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;min-height:1.8em;"
					>
						{c.description ?? 'Без описания'}
					</p>
					<div style="display:flex;gap:6px;flex-wrap:wrap;align-items:center;">
						<span class="tag tag-neutral mono" style="font-size:10px;padding:2px 6px;">{c.mods.length} mods</span>
						{#each c.mods.slice(0, 4) as m}<span class="tag tag-outline mono" style="font-size:10px;padding:2px 6px;">{m.slug}{m.version ? '@' + m.version : ''}</span>{/each}
						{#if c.mods.length > 4}<span class="mono" style="font-size:11px;opacity:.6;">+{c.mods.length - 4}</span>{/if}
						<button
							class="btn btn-secondary mono"
							style="margin-left:auto;padding:4px 8px;font-size:11px;"
							onclick={(e) => {
								e.preventDefault();
								downloadPrism(c.slug);
							}}>⤓ Prism</button
						>
					</div>
				</a>
			{/each}
		</div>
	{/if}

	<p class="mono text-muted" style="font-size:11px;margin-top:18px;">
		API: <code>GET {BASE}/api/v1/collections</code> · экспорт: <code>?format=prism</code> — Prism Launcher JSON
	</p>
</div>
