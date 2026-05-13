export function albumTrackHref(trackId: number | null | undefined, albumId: number | null | undefined) {
	if (!trackId || !albumId) return null;
	return `/albums/${albumId}?track=${trackId}`;
}
