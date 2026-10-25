<script lang="ts">
	import { onMount } from 'svelte';
	let repo = $state('');
	let status = $state<'idle'|'loading'|'ok'|'error'>('idle');
	let msg = $state('');
	let isAuthed = $state(false);

	onMount(async () => {
		try {
			const r = await fetch('http://localhost:8080/api/v1/auth/me', { credentials: 'include' });
			isAuthed = r.ok;
		} catch { isAuthed = false; }
	});

	async function publish() {
		if (!isAuthed) { window.location.href = 'http://localhost:8080/api/v1/auth/github'; return; }
		status='loading'; msg='';
		try {
			const res = await fetch(`http://localhost:8080/api/v1/mods/import`, {
				method:'POST', headers:{'Content-Type':'application/json'}, credentials: 'include',
				body: JSON.stringify({ repo })
			});
			const data = await res.json();
			if (!res.ok) throw new Error(data.message ?? data.error ?? 'Failed');
			status='ok'; msg=`Imported ${repo} - ${data.slug ?? 'ok'}`;
		} catch(e:any){ status='error'; msg=e.message; }
	}
</script>

<svelte:head><title>Publish mod - Indexium</title></svelte:head>

<div style="max-width:720px;margin:0 auto;padding:32px clamp(16px,4vw,32px);">
	<h1 style="margin:0 0 8px;">Publish your mod</h1>
	<p class="text-muted" style="margin:0 0 24px;">Open-source only. We index your GitHub Releases - no file upload.</p>

	{#if !isAuthed}
		<div class="card" style="gap:14px;padding:20px;border:1px solid var(--color-accent);background:color-mix(in srgb,var(--color-accent) 8%,var(--color-surface));">
			<p style="margin:0;font-weight:600;">Требуется вход через GitHub</p>
			<p class="text-muted" style="margin:0;font-size:13px;">Публикация доступна только авторам. Мы проверим что репо твое, публичное и с LICENSE.</p>
			<a href="http://localhost:8080/api/v1/auth/github" class="btn btn-primary" style="align-self:flex-start;">Войти через GitHub - Publish</a>
		</div>
	{:else}
	<div class="card" style="gap:16px;">
		<label style="display:flex;flex-direction:column;gap:6px;">
			<span class="mono" style="font-size:12px;letter-spacing:.06em;text-transform:uppercase;opacity:.7;">GitHub repo</span>
			<input class="input" placeholder="owner/repo  e.g. jellysquid3/sodium" bind:value={repo} />
			<span class="text-muted" style="font-size:12px;">Repo must be public, have LICENSE and fabric.mod.json / mods.toml in default branch.</span>
		</label>
		<button class="btn btn-primary" style="align-self:flex-start;" onclick={publish} disabled={status==='loading' || !repo.includes('/')}>
			{status==='loading' ? 'Publishing…' : 'Publish'}
		</button>
		{#if msg}<div class="card" style="padding:12px;background:{status==='ok' ? 'color-mix(in srgb,var(--color-accent) 12%,var(--color-surface))' : '#3a1a1a'};border:1px solid {status==='ok' ? 'var(--color-accent)' : '#a00'};">{msg}</div>{/if}
	</div>
	{/if}

	<div class="card" style="margin-top:18px;gap:12px;">
		<h3 style="margin:0;font-size:16px;">How it works</h3>
		<ol style="margin:0;padding-left:18px;opacity:.8;font-size:14px;line-height:1.6;">
			<li>Login with GitHub (read:user)</li>
			<li>Paste <code class="mono">owner/repo</code> → we verify LICENSE + manifest</li>
			<li>Install webhook → every release is indexed via Range-Request (no full download)</li>
		</ol>
		<p class="text-muted" style="margin:0;font-size:13px;">See <a href="/docs">docs</a> and <a href="https://github.com/loki5512344ss/Indexium" target="_blank">GitHub</a>.</p>
	</div>
</div>
