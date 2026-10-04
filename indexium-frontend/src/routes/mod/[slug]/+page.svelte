<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { fetchMod, fetchAnalytics, type ModDetail } from '$lib/api';

	let slug = $derived($page.params.slug ?? '');
	let mod = $state<ModDetail | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let tab = $state<'versions' | 'analytics'>('versions');
	let analytics = $state<{ daily: {date:string,active_servers:number}[], breakdown:any } | null>(null);

	onMount(async () => {
		try {
			mod = await fetchMod(slug);
			try { const a = await fetchAnalytics(slug, '30d'); analytics = { daily: a.daily, breakdown: a.breakdown }; } catch {}
		} catch (e:any) { error = e.message; } finally { loading = false; }
	});
</script>

<svelte:head><title>{mod?.name ?? slug} - Indexium</title></svelte:head>

<div style="max-width:1100px;margin:0 auto;padding:0 clamp(16px,4vw,32px);">
	{#if loading}<p class="text-muted">Loading...</p>
	{:else if error}<div class="card" style="border:1px solid #a00;color:#a00;padding:16px;">{error}</div>
	{:else if mod}
		<div style="display:grid;grid-template-columns:1fr 320px;gap:24px;margin-top:20px;">
			<div style="display:flex;flex-direction:column;gap:16px;">
				<div class="card" style="flex-direction:row;gap:16px;padding:18px;">
					<div style="width:72px;height:72px;border-radius:12px;background:linear-gradient(135deg,var(--color-accent-500),var(--color-accent-700));display:flex;align-items:center;justify-content:center;color:var(--color-bg);font-weight:700;font-size:28px;flex:none;">{mod.name[0]}</div>
					<div style="flex:1;">
						<h1 style="margin:0;font-size:26px;">{mod.name}</h1>
						<p class="text-muted" style="margin:4px 0;">by {mod.author.login} · <a href="https://github.com/{mod.github_repo}" target="_blank">{mod.github_repo}</a> {#if mod.verified}<span class="tag tag-accent mono" style="margin-left:8px;">verified</span>{/if}</p>
						<p style="margin:8px 0 0;opacity:.85;">{mod.summary ?? ''}</p>
						<p class="text-muted" style="font-size:13px;margin-top:8px;white-space:pre-wrap;">{mod.description ?? ''}</p>
					</div>
				</div>
				<div class="seg" style="align-self:flex-start;">
					<label class="seg-opt"><input type="radio" name="tab" checked={tab==='versions'} onchange={()=>tab='versions'} /><span class="mono">Versions</span></label>
					<label class="seg-opt"><input type="radio" name="tab" checked={tab==='analytics'} onchange={()=>tab='analytics'} /><span class="mono">Analytics</span></label>
				</div>
				{#if tab==='versions'}
					<div class="card" style="padding:0;overflow:hidden;">
						<div style="padding:12px 16px;border-bottom:1px solid var(--color-divider);font-weight:600;">Versions · {mod.versions.length}</div>
						{#each mod.versions as v}
							<div style="display:flex;gap:12px;padding:12px 16px;border-bottom:1px solid var(--color-divider);flex-wrap:wrap;align-items:center;">
								<span class="mono" style="font-weight:600;">{v.version_number}</span>
								{#each v.game_versions as gv}<span class="tag tag-neutral mono" style="font-size:10px;">{gv}</span>{/each}
								{#each v.loaders as l}<span class="tag tag-outline mono" style="font-size:10px;">{l}</span>{/each}
								<span class="mono text-muted" style="margin-left:auto;font-size:11px;">{new Date(v.published_at).toLocaleDateString()}</span>
								<a href={v.download_url} class="btn btn-secondary" style="padding:6px 12px;font-size:12px;" target="_blank">Download</a>
							</div>
						{:else}<p class="text-muted" style="padding:16px;">No versions yet.</p>{/each}
					</div>
				{:else}
					<div class="card" style="gap:12px;">
						<h3 style="margin:0;">Active servers (bStats)</h3>
						{#if analytics?.daily?.length}<div style="display:flex;gap:4px;align-items:end;height:60px;">{#each analytics.daily.slice(-14) as d}<div style="flex:1;background:var(--color-accent);border-radius:4px 4px 0 0;height:{Math.max(8, d.active_servers % 60)}px;"></div>{/each}</div>{:else}<p class="text-muted">No analytics yet - add Indexium SDK to your mod.</p>{/if}
						<pre class="mono" style="font-size:11px;opacity:.7;overflow:auto;">{JSON.stringify(analytics?.breakdown ?? {}, null, 2)}</pre>
					</div>
				{/if}
			</div>
			<aside style="display:flex;flex-direction:column;gap:12px;position:sticky;top:76px;">
				<div class="card" style="gap:10px;">
					<button class="btn btn-primary" style="width:100%;">Install</button>
					<a href={mod.versions[0]?.download_url ?? '#'} class="btn btn-secondary" style="width:100%;" target="_blank">Download jar</a>
					<div class="mono" style="font-size:11px;opacity:.6;">SHA256: {mod.versions[0]?.file_sha256?.slice(0,16) ?? '-'}…</div>
				</div>
				<div class="card" style="gap:8px;">
					<p style="margin:0;font-weight:600;">Details</p>
					<p class="text-muted" style="margin:0;font-size:13px;">Repo: {mod.github_repo}</p>
					<p class="text-muted" style="margin:0;font-size:13px;">Updated: {new Date(mod.updated_at).toLocaleDateString()}</p>
					<img src="/api/v1/badges/{mod.slug}/downloads.svg" alt="downloads badge" style="margin-top:8px;max-width:100%;" />
				</div>
			</aside>
		</div>
	{/if}
</div>
