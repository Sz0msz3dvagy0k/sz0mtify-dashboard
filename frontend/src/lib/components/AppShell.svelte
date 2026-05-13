<script lang="ts">
	import { page } from '$app/stores';
	import {
		Album,
		Activity,
		AudioLines,
		BarChart3,
		Compass,
		Disc3,
		HardDrive,
		HeartPulse,
		Library,
		Radio,
		Search,
		Settings,
		Tags,
		UsersRound
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import type { SyncStatus } from '$lib/types';
	import { onMount } from 'svelte';

	const nav = [
		{ href: '/overview', label: 'Overview', icon: BarChart3 },
		{ href: '/albums', label: 'Albums', icon: Album },
		{ href: '/artists', label: 'Artists', icon: UsersRound },
		{ href: '/genres', label: 'Genres & Moods', icon: Tags },
		{ href: '/audio-quality', label: 'Audio Quality', icon: AudioLines },
		{ href: '/storage', label: 'Storage', icon: HardDrive },
		{ href: '/metadata-health', label: 'Metadata Health', icon: HeartPulse },
		{ href: '/listening', label: 'Listening Stats', icon: Radio },
		{ href: '/discovery', label: 'Discovery', icon: Compass },
		{ href: '/search', label: 'Search', icon: Search },
		{ href: '/settings', label: 'Settings', icon: Settings }
	];

	let status: SyncStatus = [];
	$: current = nav.find((item) => $page.url.pathname.startsWith(item.href)) ?? nav[0];
	$: subtitle = status.length ? status.map((row) => `${row[1]} ${row[3]}`).join(' · ') : 'Backend status pending';

	onMount(async () => {
		status = await api.syncStatus().catch(() => []);
	});
</script>

<svelte:head>
	<title>{current.label} · Archive</title>
</svelte:head>

<div class="app-shell">
	<aside class="sidebar">
		<a class="brand" href="/overview">
			<Disc3 size={28} strokeWidth={1.4} />
			<span>Archive</span>
		</a>
		<nav>
			{#each nav as item}
				<a class:active={$page.url.pathname.startsWith(item.href)} href={item.href}>
					<svelte:component this={item.icon} size={18} strokeWidth={1.5} />
					<span>{item.label}</span>
				</a>
			{/each}
		</nav>
	</aside>
	<div class="main-column">
		<header class="topbar">
			<div>
				<p>Music analytics</p>
				<h1>{current.label}</h1>
			</div>
			<div class="status-pill">
				<Activity size={16} strokeWidth={1.5} />
				<span>{subtitle}</span>
			</div>
		</header>
		<main class="page-surface">
			<slot />
		</main>
	</div>
</div>
