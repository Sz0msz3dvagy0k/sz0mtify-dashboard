<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import {
		Album,
		AudioLines,
		BarChart3,
		Compass,
		Disc3,
		HardDrive,
		History,
		HeartPulse,
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
	import { currentNetworkStatus, initNetworkStatus } from '$lib/mobileNetwork';
	import { loadLocalMedia } from '$lib/localMedia';
	import { warmStreamToken } from '$lib/player';
	import type { AuthSession, PlaylistSummary } from '$lib/types';
	import { onMount, tick } from 'svelte';
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
		{ href: '/history', label: 'Song History', icon: History },
		{ href: '/discovery', label: 'Discovery', icon: Compass },
		{ href: '/settings', label: 'Settings', icon: Settings }
	];
	const mobileMenuBreakpoint = 980;
	const edgeSwipeWidth = 28;
	const swipeDistance = 56;
	const swipeOffAxisLimit = 70;

	let playlists: PlaylistSummary[] = [];
	let authChecked = false;
	let authenticated = false;
	let accountName = '';
	let mobileMenuOpen = false;
	let searchOpen = false;
	let searchValue = '';
	let syncedSearchParam: string | null = null;
	let searchInput: HTMLInputElement;
	let swipeStartX = 0;
	let swipeStartY = 0;
	let trackedSwipe: 'open' | 'close' | null = null;
	$: current = nav.find((item) => $page.url.pathname.startsWith(item.href)) ?? nav[0];
	$: currentLabel = $page.url.pathname.startsWith('/search') ? 'Search' : current.label;
	$: if ($page.url.pathname === '/search') {
		const queryParam = $page.url.searchParams.get('q') ?? '';
		if (queryParam !== syncedSearchParam) {
			searchValue = queryParam;
			syncedSearchParam = queryParam;
		}
	}

	onMount(async () => {
		const session = loadStoredSession();
		if (!session) {
			authChecked = true;
			return;
		}

		void initNetworkStatus();
		void loadLocalMedia();

		try {
			const user = await api.me();
			accountName = user.username;
			authenticated = true;
			void warmStreamToken().catch((error) => console.warn('Unable to warm stream token', error));
			await loadShellData();
		} catch {
			const status = await currentNetworkStatus();
			if (!status.connected) {
				accountName = session.username;
				authenticated = true;
				await loadShellData();
			} else {
				clearAuthSession();
			}
		} finally {
			authChecked = true;
		}
	});

	async function loadShellData() {
		playlists = await api.playlists().catch(() => []);
	}

	async function handleAuthenticated(session: AuthSession) {
		accountName = session.username;
		authenticated = true;
		void initNetworkStatus();
		void loadLocalMedia();
		void warmStreamToken().catch((error) => console.warn('Unable to warm stream token', error));
		await loadShellData();
	}

	async function signOut() {
		await api.logout().catch(() => null);
		clearAuthSession();
		playlists = [];
		authenticated = false;
		accountName = '';
		mobileMenuOpen = false;
		searchOpen = false;
	}

	function closeMobileMenu() {
		mobileMenuOpen = false;
	}

	async function openMobileSearch() {
		searchOpen = true;
		await tick();
		searchInput?.focus();
	}

	function closeSearch() {
		searchOpen = false;
		searchInput?.blur();
	}

	async function submitSearch() {
		const query = searchValue.trim();
		if (!query) return;
		if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
		closeSearch();
		await goto(`/search?q=${encodeURIComponent(query)}`);
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
	<title>{authenticated ? `${currentLabel} · Archive` : 'Sign in · Archive'}</title>
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
						<h1>{currentLabel}</h1>
					</div>
				</div>
				<form class="topbar-search desktop-search" role="search" on:submit|preventDefault={submitSearch}>
					<Search size={18} strokeWidth={1.6} />
					<input bind:value={searchValue} placeholder="Search tracks, albums, artists" aria-label="Search tracks, albums, artists" />
				</form>
				<div class="topbar-actions">
					<button class="icon-button" aria-label={`Sign out ${accountName}`} title={`Signed in as ${accountName}`} on:click={signOut}>
						<LogOut size={18} strokeWidth={1.5} />
					</button>
					<button class="icon-button mobile-search-button" aria-label="Search" on:click={openMobileSearch}>
						<Search size={18} strokeWidth={1.5} />
					</button>
				</div>
			</header>
			{#if searchOpen}
				<div class="search-overlay" role="dialog" aria-label="Search">
					<button class="search-overlay-backdrop" aria-label="Close search" on:click={closeSearch}></button>
					<form class="topbar-search mobile-search" role="search" on:submit|preventDefault={submitSearch}>
						<Search size={18} strokeWidth={1.6} />
						<input bind:this={searchInput} bind:value={searchValue} placeholder="Search tracks, albums, artists" aria-label="Search tracks, albums, artists" on:keydown={(event) => event.key === 'Escape' && closeSearch()} />
						<button class="icon-button" type="button" aria-label="Close search" on:click={closeSearch}>
							<X size={18} strokeWidth={1.6} />
						</button>
					</form>
				</div>
			{/if}
			<main class="page-surface">
				<slot />
			</main>
		</div>
		<PlayerBar />
	</div>
{/if}
