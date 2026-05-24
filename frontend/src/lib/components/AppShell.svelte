<script lang="ts">
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { afterNavigate, goto } from '$app/navigation';
	import {
		Album,
		AudioLines,
		BarChart3,
		ChevronLeft,
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
	import { initTheme } from '$lib/theme';
	import type { AuthSession } from '$lib/types';
	import { onDestroy, onMount, tick } from 'svelte';
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

	let authChecked = false;
	let authenticated = false;
	let accountName = '';
	let mobileMenuOpen = false;
	let searchOpen = false;
	let searchValue = '';
	let syncedSearchParam: string | null = null;
	let searchTimer: ReturnType<typeof setTimeout> | null = null;
	let searchInput: HTMLInputElement;
	let swipeStartX = 0;
	let swipeStartY = 0;
	let trackedSwipe: 'open' | 'close' | 'back' | null = null;
	let lockedScrollY = 0;
	let appBackStack: string[] = [];
	let navigatingBackTo: string | null = null;
	$: current = nav.find((item) => $page.url.pathname.startsWith(item.href)) ?? nav[0];
	$: currentLabel = $page.url.pathname.startsWith('/search') ? 'Search' : $page.url.pathname.startsWith('/tracks') ? 'Track' : current.label;
	$: showBackButton = isDetailRoute($page.url.pathname);
	$: if ($page.url.pathname === '/search') {
		const queryParam = $page.url.searchParams.get('q') ?? '';
		if (queryParam !== syncedSearchParam) {
			searchValue = queryParam;
			syncedSearchParam = queryParam;
		}
	}
	$: if (browser) setPageScrollLock(authenticated && mobileMenuOpen);

	afterNavigate((navigation) => {
		if (!browser) return;
		const from = navigation.from?.url;
		const to = navigation.to?.url;
		if (!to) return;

		const toPath = routePath(to);
		if (navigatingBackTo === toPath) {
			navigatingBackTo = null;
			return;
		}

		if (!from || from.origin !== to.origin || from.href === to.href) return;

		if (navigation.type === 'popstate') {
			const targetIndex = appBackStack.lastIndexOf(toPath);
			if (targetIndex >= 0) appBackStack = appBackStack.slice(0, targetIndex);
			return;
		}

		const fromPath = routePath(from);
		if (appBackStack[appBackStack.length - 1] !== fromPath) {
			appBackStack = [...appBackStack, fromPath].slice(-24);
		}
	});

	onMount(async () => {
		initTheme();
		const session = await loadStoredSession();
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
		} catch {
			const status = await currentNetworkStatus();
			if (!status.connected) {
				accountName = session.username;
				authenticated = true;
			} else {
				clearAuthSession();
			}
		} finally {
			authChecked = true;
		}
	});

	onDestroy(() => {
		if (searchTimer) clearTimeout(searchTimer);
		setPageScrollLock(false);
	});

	async function handleAuthenticated(session: AuthSession) {
		accountName = session.username;
		authenticated = true;
		void initNetworkStatus();
		void loadLocalMedia();
		void warmStreamToken().catch((error) => console.warn('Unable to warm stream token', error));
	}

	async function signOut() {
		await api.logout().catch(() => null);
		clearAuthSession();
		authenticated = false;
		accountName = '';
		mobileMenuOpen = false;
		searchOpen = false;
	}

	function closeMobileMenu() {
		mobileMenuOpen = false;
	}

	function setPageScrollLock(locked: boolean) {
		if (!browser) return;
		const body = document.body;
		const isLocked = body.classList.contains('mobile-menu-scroll-lock');
		if (locked === isLocked) return;

		if (locked) {
			lockedScrollY = window.scrollY;
			body.classList.add('mobile-menu-scroll-lock');
			body.style.top = `-${lockedScrollY}px`;
			return;
		}

		body.classList.remove('mobile-menu-scroll-lock');
		body.style.top = '';
		window.scrollTo(0, lockedScrollY);
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

	function handleSearchInput(event: Event) {
		searchValue = event.currentTarget instanceof HTMLInputElement ? event.currentTarget.value : searchValue;
		scheduleSearch(searchValue);
	}

	function scheduleSearch(value: string) {
		if (searchTimer) clearTimeout(searchTimer);
		searchTimer = setTimeout(() => {
			void navigateToSearch(value.trim(), true);
		}, 1000);
	}

	async function submitSearch() {
		if (searchTimer) clearTimeout(searchTimer);
		searchTimer = null;
		await navigateToSearch(searchValue.trim(), false);
	}

	async function navigateToSearch(query: string, debounced: boolean) {
		if (!query) return;
		if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
		if (searchOpen) closeSearch();
		const target = `/search?q=${encodeURIComponent(query)}`;
		if ($page.url.pathname === '/search' && $page.url.searchParams.get('q') === query) return;
		await goto(target);
		if (!debounced) syncedSearchParam = query;
	}

	function isDetailRoute(pathname: string) {
		return /^\/(albums|artists|playlists|tracks)\/[^/]+$/.test(pathname);
	}

	function routePath(url: URL) {
		return `${url.pathname}${url.search}${url.hash}`;
	}

	function fallbackBackPath(pathname: string) {
		if (pathname.startsWith('/artists/')) return '/artists';
		if (pathname.startsWith('/playlists/')) return '/playlists';
		return '/albums';
	}

	function returnTarget(value: string | null) {
		if (!value || !value.startsWith('/') || value.startsWith('//')) return null;
		const currentPath = routePath($page.url);
		return value === currentPath ? null : value;
	}

	async function goBack() {
		const explicitTarget = returnTarget($page.url.searchParams.get('from'));
		if (explicitTarget) {
			if (appBackStack[appBackStack.length - 1] === explicitTarget) {
				appBackStack = appBackStack.slice(0, -1);
			}
			navigatingBackTo = explicitTarget;
			await goto(explicitTarget, { replaceState: true });
			return;
		}

		const target = appBackStack[appBackStack.length - 1];
		if (target) {
			appBackStack = appBackStack.slice(0, -1);
			navigatingBackTo = target;
			await goto(target, { replaceState: true });
			return;
		}

		await goto(fallbackBackPath($page.url.pathname));
	}

	function isBackSwipeIgnoredTarget(target: EventTarget | null) {
		if (!(target instanceof Element)) return false;
		return Boolean(
			target.closest(
				'button, a, input, textarea, select, [role="button"], [role="slider"], .swipe-queue-target, .player-bar, .search-overlay, .sidebar'
			)
		);
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
		if (mobileMenuOpen) {
			trackedSwipe = touch.clientX <= drawerWidth ? 'close' : null;
			return;
		}

		trackedSwipe = showBackButton && !isBackSwipeIgnoredTarget(event.target) ? 'back' : null;
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

		if (trackedSwipe === 'back' && deltaX >= swipeDistance) {
			trackedSwipe = null;
			void goBack();
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
	<title>{authenticated ? `${currentLabel} · sz0mtify` : 'Sign in · sz0mtify'}</title>
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
				<span>sz0mtify</span>
			</a>
			<nav>
				{#each nav as item}
					<a class:active={$page.url.pathname.startsWith(item.href)} href={item.href} on:click={closeMobileMenu}>
						<svelte:component this={item.icon} size={18} strokeWidth={1.5} />
						<span>{item.label}</span>
					</a>
				{/each}
			</nav>
		</aside>
		<div class="main-column">
			<header class="topbar">
				<div class="topbar-heading">
					<div class="topbar-nav">
						<button class="icon-button menu-button" aria-label={mobileMenuOpen ? 'Close menu' : 'Open menu'} aria-expanded={mobileMenuOpen} on:click={() => (mobileMenuOpen = !mobileMenuOpen)}>
							{#if mobileMenuOpen}<X size={20} strokeWidth={1.7} />{:else}<Menu size={20} strokeWidth={1.7} />{/if}
						</button>
						{#if showBackButton}
							<button class="icon-button back-button" aria-label="Go back" on:click={goBack}>
								<ChevronLeft size={20} strokeWidth={1.8} />
							</button>
						{/if}
					</div>
					<div class="topbar-title">
						<p>Music analytics</p>
						<h1>{currentLabel}</h1>
					</div>
				</div>
				<form class="topbar-search desktop-search" role="search" on:submit|preventDefault={submitSearch}>
					<Search size={18} strokeWidth={1.6} />
					<input bind:value={searchValue} placeholder="Search tracks, albums, artists" aria-label="Search tracks, albums, artists" on:input={handleSearchInput} />
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
						<input bind:this={searchInput} bind:value={searchValue} placeholder="Search tracks, albums, artists" aria-label="Search tracks, albums, artists" on:input={handleSearchInput} on:keydown={(event) => event.key === 'Escape' && closeSearch()} />
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
