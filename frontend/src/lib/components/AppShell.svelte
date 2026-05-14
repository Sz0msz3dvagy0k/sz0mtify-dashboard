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
		ListMusic,
		LogOut,
		Menu,
		Radio,
		Search,
		Settings,
		Tags,
		UsersRound,
		X
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import { clearAuthSession, loadStoredSession } from '$lib/auth';
	import { initNetworkStatus } from '$lib/mobileNetwork';
	import { warmStreamToken } from '$lib/player';
	import type { AuthSession, PlaylistSummary, SyncStatus } from '$lib/types';
	import { onMount } from 'svelte';
	import LoginPage from './LoginPage.svelte';
	import PlayerBar from './PlayerBar.svelte';

	const nav = [
		{ href: '/overview', label: 'Overview', icon: BarChart3 },
		{ href: '/albums', label: 'Albums', icon: Album },
		{ href: '/artists', label: 'Artists', icon: UsersRound },
		{ href: '/playlists', label: 'Playlists', icon: ListMusic },
		{ href: '/genres', label: 'Genres & Moods', icon: Tags },
		{ href: '/audio-quality', label: 'Audio Quality', icon: AudioLines },
		{ href: '/storage', label: 'Storage', icon: HardDrive },
		{ href: '/metadata-health', label: 'Metadata Health', icon: HeartPulse },
		{ href: '/listening', label: 'Listening Stats', icon: Radio },
		{ href: '/discovery', label: 'Discovery', icon: Compass },
		{ href: '/search', label: 'Search', icon: Search },
		{ href: '/settings', label: 'Settings', icon: Settings }
	];
	const mobileMenuBreakpoint = 980;
	const edgeSwipeWidth = 28;
	const swipeDistance = 56;
	const swipeOffAxisLimit = 70;

	let status: SyncStatus = [];
	let playlists: PlaylistSummary[] = [];
	let authChecked = false;
	let authenticated = false;
	let accountName = '';
	let mobileMenuOpen = false;
	let swipeStartX = 0;
	let swipeStartY = 0;
	let trackedSwipe: 'open' | 'close' | null = null;
	$: current = nav.find((item) => $page.url.pathname.startsWith(item.href)) ?? nav[0];
	$: subtitle = status.length ? status.map((row) => `${row[1]} ${row[3]}`).join(' · ') : 'Backend status pending';

	onMount(async () => {
		const session = loadStoredSession();
		if (!session) {
			authChecked = true;
			return;
		}

		try {
			const user = await api.me();
			accountName = user.username;
			authenticated = true;
			void initNetworkStatus();
			void warmStreamToken().catch((error) => console.warn('Unable to warm stream token', error));
			await loadShellData();
		} catch {
			clearAuthSession();
		} finally {
			authChecked = true;
		}
	});

	async function loadShellData() {
		[status, playlists] = await Promise.all([
			api.syncStatus().catch(() => []),
			api.playlists().catch(() => [])
		]);
	}

	async function handleAuthenticated(session: AuthSession) {
		accountName = session.username;
		authenticated = true;
		void initNetworkStatus();
		void warmStreamToken().catch((error) => console.warn('Unable to warm stream token', error));
		await loadShellData();
	}

	async function signOut() {
		await api.logout().catch(() => null);
		clearAuthSession();
		status = [];
		playlists = [];
		authenticated = false;
		accountName = '';
		mobileMenuOpen = false;
	}

	function closeMobileMenu() {
		mobileMenuOpen = false;
	}

	function isMobileViewport() {
		return window.innerWidth <= mobileMenuBreakpoint;
	}

	function handleTouchStart(event: TouchEvent) {
		if (!authenticated || !isMobileViewport() || event.touches.length !== 1) return;

		const touch = event.touches[0];
		swipeStartX = touch.clientX;
		swipeStartY = touch.clientY;

		if (!mobileMenuOpen && touch.clientX <= edgeSwipeWidth) {
			trackedSwipe = 'open';
			return;
		}

		const drawerWidth = Math.min(300, window.innerWidth * 0.84);
		trackedSwipe = mobileMenuOpen && touch.clientX <= drawerWidth ? 'close' : null;
	}

	function handleTouchMove(event: TouchEvent) {
		if (!trackedSwipe || event.touches.length !== 1) return;

		const touch = event.touches[0];
		const deltaX = touch.clientX - swipeStartX;
		const deltaY = Math.abs(touch.clientY - swipeStartY);

		if (deltaY > swipeOffAxisLimit) {
			trackedSwipe = null;
			return;
		}

		if (trackedSwipe === 'open' && deltaX >= swipeDistance) {
			mobileMenuOpen = true;
			trackedSwipe = null;
		}

		if (trackedSwipe === 'close' && deltaX <= -swipeDistance) {
			closeMobileMenu();
			trackedSwipe = null;
		}
	}

	function handleTouchEnd() {
		trackedSwipe = null;
	}
</script>

<svelte:window
	on:keydown={(event) => event.key === 'Escape' && closeMobileMenu()}
	on:touchstart={handleTouchStart}
	on:touchmove={handleTouchMove}
	on:touchend={handleTouchEnd}
	on:touchcancel={handleTouchEnd}
/>

<svelte:head>
	<title>{authenticated ? `${current.label} · Archive` : 'Sign in · Archive'}</title>
</svelte:head>

{#if !authChecked}
	<main class="login-page">
		<section class="login-panel">
			<div class="skeleton block"></div>
		</section>
	</main>
{:else if !authenticated}
	<LoginPage onAuthenticated={handleAuthenticated} />
{:else}
	<div class="app-shell">
		{#if mobileMenuOpen}
			<button class="mobile-menu-backdrop" aria-label="Close menu" on:click={closeMobileMenu}></button>
		{/if}
		<aside class="sidebar" class:open={mobileMenuOpen}>
			<a class="brand" href="/overview">
				<Disc3 size={28} strokeWidth={1.4} />
				<span>Archive</span>
			</a>
			<nav>
				{#each nav as item}
					<a class:active={$page.url.pathname.startsWith(item.href)} href={item.href} on:click={closeMobileMenu}>
						<svelte:component this={item.icon} size={18} strokeWidth={1.5} />
						<span>{item.label}</span>
					</a>
				{/each}
			</nav>
			{#if playlists.length}
				<div class="sidebar-section">
					<span>Playlists</span>
					{#each playlists.slice(0, 8) as playlist}
						<a class:active={$page.url.pathname === `/playlists/${encodeURIComponent(playlist.id)}`} href={`/playlists/${encodeURIComponent(playlist.id)}`} on:click={closeMobileMenu}>{playlist.name}</a>
					{/each}
				</div>
			{/if}
		</aside>
		<div class="main-column">
			<header class="topbar">
				<div class="topbar-heading">
					<button class="icon-button menu-button" aria-label={mobileMenuOpen ? 'Close menu' : 'Open menu'} aria-expanded={mobileMenuOpen} on:click={() => (mobileMenuOpen = !mobileMenuOpen)}>
						{#if mobileMenuOpen}<X size={20} strokeWidth={1.7} />{:else}<Menu size={20} strokeWidth={1.7} />{/if}
					</button>
					<div>
						<p>Music analytics</p>
						<h1>{current.label}</h1>
					</div>
				</div>
				<div class="topbar-actions">
					<div class="status-pill">
						<Activity size={16} strokeWidth={1.5} />
						<span>{subtitle}</span>
					</div>
					<button class="icon-button" aria-label={`Sign out ${accountName}`} title={`Signed in as ${accountName}`} on:click={signOut}>
						<LogOut size={18} strokeWidth={1.5} />
					</button>
				</div>
			</header>
			<main class="page-surface">
				<slot />
			</main>
		</div>
		<PlayerBar />
	</div>
{/if}
