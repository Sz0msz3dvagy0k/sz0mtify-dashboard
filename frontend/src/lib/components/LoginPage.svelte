<script lang="ts">
	import { Disc3, KeyRound, LoaderCircle, UserRound } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { saveAuthSession } from '$lib/auth';
	import type { AuthSession } from '$lib/types';

	export let onAuthenticated: (session: AuthSession) => void = () => {};

	let username = '';
	let password = '';
	let busy = false;
	let error = '';

	async function submit() {
		if (busy) return;
		busy = true;
		error = '';

		try {
			const session = await api.login({ username: username.trim(), password });
			saveAuthSession(session);
			onAuthenticated(session);
			password = '';
		} catch (e) {
			error = e instanceof Error ? e.message : 'Unable to sign in';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head>
	<title>Sign in · Archive</title>
</svelte:head>

<main class="login-page">
	<section class="login-panel">
		<div class="login-brand">
			<div class="login-mark"><Disc3 size={32} strokeWidth={1.4} /></div>
			<div>
				<p>Archive</p>
				<h1>Sign in</h1>
			</div>
		</div>

		<form class="login-form" on:submit|preventDefault={submit}>
			<label>
				<span>Account</span>
				<div class="login-input">
					<UserRound size={18} strokeWidth={1.5} />
					<input bind:value={username} autocomplete="username" placeholder="admin" />
				</div>
			</label>
			<label>
				<span>Password</span>
				<div class="login-input">
					<KeyRound size={18} strokeWidth={1.5} />
					<input bind:value={password} type="password" autocomplete="current-password" placeholder="••••••••" />
				</div>
			</label>

			{#if error}<p class="login-error">{error}</p>{/if}

			<button class="button login-submit" disabled={busy || !username.trim() || !password}>
				{#if busy}<LoaderCircle size={18} class="spin" />{/if}
				<span>{busy ? 'Signing in' : 'Continue'}</span>
			</button>
		</form>
	</section>
</main>
