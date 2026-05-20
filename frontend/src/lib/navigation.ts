export function albumTrackHref(trackId: number | null | undefined, albumId: number | null | undefined) {
	if (!trackId || !albumId) return null;
	return `/albums/${albumId}?track=${trackId}`;
}

export function trackHref(trackId: number | null | undefined) {
	return trackId ? `/tracks/${trackId}` : null;
}
