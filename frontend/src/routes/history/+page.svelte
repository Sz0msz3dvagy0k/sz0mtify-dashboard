<script lang="ts">
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ImageWithFallback from '$lib/components/ImageWithFallback.svelte';
	import SectionHeader from '$lib/components/SectionHeader.svelte';
	import { api } from '$lib/api';
	import { formatDuration } from '$lib/format';
	import { playQueue, queueTrackImage, songHistory, type SongHistoryEntry } from '$lib/player';

	const albumCoverRequests = new Set<number>();

	$: groupedHistory = groupHistory($songHistory);
	$: hydrateMissingCovers($songHistory);

	function groupHistory(entries: SongHistoryEntry[]) {
		const groups = new Map<string, SongHistoryEntry[]>();
		for (const entry of entries) {
			const key = dateKey(entry.playedAt);
			const current = groups.get(key) ?? [];
			current.push(entry);
			groups.set(key, current);
		}
		return Array.from(groups.entries()).map(([key, songs]) => ({
			key,
			label: dayLabel(key),
			songs
		}));
	}

	function dateKey(value: string) {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return value.slice(0, 10);
		const year = date.getFullYear();
		const month = `${date.getMonth() + 1}`.padStart(2, '0');
		const day = `${date.getDate()}`.padStart(2, '0');
		return `${year}-${month}-${day}`;
	}

	function dayLabel(key: string) {
		const today = dateKey(new Date().toISOString());
		const yesterdayDate = new Date();
		yesterdayDate.setDate(yesterdayDate.getDate() - 1);
		const yesterday = dateKey(yesterdayDate.toISOString());
		if (key === today) return 'Today';
		if (key === yesterday) return 'Yesterday';

		const date = new Date(`${key}T12:00:00`);
		if (Number.isNaN(date.getTime())) return key;
		return new Intl.DateTimeFormat('en-US', { month: 'long', day: 'numeric', year: 'numeric' }).format(date);
	}

	function playedTime(value: string) {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		return new Intl.DateTimeFormat('en-US', { hour: 'numeric', minute: '2-digit' }).format(date);
	}

	function replay(entry: SongHistoryEntry) {
		playQueue(
			[
				{
					id: entry.id,
					title: entry.title,
					artist: entry.artist,
					album: entry.album,
					albumId: entry.albumId,
					coverArtId: entry.coverArtId,
					duration: entry.duration
				}
			],
			0
		);
	}

	function hydrateMissingCovers(entries: SongHistoryEntry[]) {
		for (const entry of entries) {
			if (entry.coverArtId || !entry.albumId || albumCoverRequests.has(entry.albumId)) continue;
			albumCoverRequests.add(entry.albumId);
			void hydrateAlbumCover(entry.albumId);
		}
	}

	async function hydrateAlbumCover(albumId: number) {
		try {
			const detail = await api.album(albumId);
			const coverArtId = detail.album?.[6] ?? null;
			if (!coverArtId) return;

			songHistory.update((entries) =>
				entries.map((entry) =>
					entry.albumId === albumId && !entry.coverArtId ? { ...entry, coverArtId } : entry
				)
			);
		} catch (error) {
			console.warn('Unable to load history album art', {
				albumId,
				error: error instanceof Error ? error.message : String(error)
			});
		}
	}
</script>

{#if !$songHistory.length}
	<EmptyState title="No songs played yet" />
{:else}
	{#each groupedHistory as group}
		<section class="history-section">
			<SectionHeader title={group.label} eyebrow={`${group.songs.length} songs`} />
			<div class="panel-list search-results">
				{#each group.songs as entry}
					<button class="search-result-row" on:click={() => replay(entry)}>
						<div class="search-result-art">
							<ImageWithFallback src={queueTrackImage(entry)} alt={entry.title} />
						</div>
						<span>
							<strong>{entry.title}</strong>
							<small><span class="artist-name artist-meta">{entry.artist}</span> · {entry.album}</small>
						</span>
						<em>{playedTime(entry.playedAt) || formatDuration(entry.duration)}</em>
					</button>
				{/each}
			</div>
		</section>
	{/each}
{/if}
