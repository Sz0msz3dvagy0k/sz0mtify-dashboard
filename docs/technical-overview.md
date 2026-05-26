# Technical Overview

This document gives a broad technical view of how sz0mtify works internally. It is meant for developers or maintainers who want to understand the main project flows without reading implementation code first.

## System Shape

sz0mtify has three main parts:

- The frontend is a SvelteKit application that renders the dashboard, library pages, player, settings, discovery, and mobile-friendly views.
- The backend is a Rust Axum API that owns authentication, talks to Subsonic/Navidrome and Last.fm, reads and writes SQLite, and proxies protected media.
- SQLite stores the local library index, settings, sync state, discovery cache, sessions, play metadata, and derived rollups.

The frontend normally never talks directly to Subsonic/Navidrome or Last.fm. It talks to the backend. This keeps music server credentials and API keys server-side.

## Request And Authentication Flow

At login, the user enters the backend URL and app password. The frontend checks backend health first, then sends the login request. A successful login returns session data and also sets a session cookie. Later API requests include the saved bearer token and browser credentials.

Most frontend API calls expect a standard response envelope:

- A success marker.
- A data object for successful responses.
- An error value for failed responses.

Short-lived frontend read caching is used for repeated GET requests so page changes and dashboard widgets do not immediately refetch the same data. Mutating requests clear that read cache because they may change the data shown elsewhere.

## Library Sync

The Subsonic sync is the main ingestion path. A typical sync looks like this:

1. The backend loads Subsonic settings from SQLite or environment variables.
2. It requests albums from the Subsonic-compatible server in pages.
3. For each album, it fetches full album detail, including tracks.
4. Artists, albums, tracks, genres, credits, cover art identifiers, play counts, sizes, formats, and durations are upserted into SQLite.
5. Tracks that disappeared upstream can be removed from the local index.
6. Rollup fields are refreshed so artist, genre, and dashboard queries stay fast.
7. Sync state is recorded as running, successful, or errored.

The sync endpoint starts work in the background and returns quickly with a started status. The settings and sync status pages can then show progress or the last error.

## Analytics And Library Pages

Most dashboard and library screens are backed by SQL queries against the local SQLite index. The analytics service groups these queries by feature area:

- Overview totals count tracks, albums, artists, and play rows.
- Library pages return compact tuple-style lists for albums, artists, tracks, genres, and detail views.
- Storage and audio quality pages summarize size, format, bitrate, and content metadata.
- Metadata health looks for missing or suspicious fields.
- Listening views use play counts and play history to surface current rotation, rediscovery candidates, and timeline data.

This design keeps normal browsing fast because the app reads from SQLite instead of repeatedly asking Subsonic for every page load.

## Image Retrieval Example

Image retrieval is a good example of the whole project pattern: the frontend asks the backend, the backend talks to the trusted upstream service, and the browser caches the result.

A common album cover flow works like this:

1. During sync, the backend stores the Subsonic cover art identifier on album and track rows.
2. A frontend component receives that cover art identifier while rendering an album, track, playlist, or player item.
3. The frontend turns the identifier into an internal backend URL under the cover endpoint.
4. The image component asks the image cache helper to resolve that URL.
5. The image cache helper first checks whether the URL is eligible for dashboard image caching. Only same-origin backend image routes are cached this way.
6. If the app has a downloaded local image for that cover, the local file object URL wins.
7. Otherwise, the browser Cache API is checked.
8. If the image is not already cached, the frontend fetches the backend image URL with the current auth headers and browser credentials.
9. The backend validates the cover art identifier, loads Subsonic credentials server-side, requests the real image from Subsonic getCoverArt, and returns only the image bytes to the browser.
10. The frontend stores a cacheable response, creates an object URL for display, and keeps a smaller in-memory cache so repeated renders do not recreate the same image immediately.

The backend adds image cache headers and an ETag based on the requested image and response size. The frontend also prunes its disk image cache by count and total byte size so old images do not grow forever.

Artist images follow a similar proxy pattern, but their source is the artist image URL saved from Navidrome or Last.fm-related metadata. The backend resolves those URLs carefully against the configured Subsonic origin and only allows expected Navidrome image paths. That prevents artist image proxying from becoming an open fetch endpoint.

When an image cannot load, the image component falls back from primary art to secondary art where available, then to a generated visual fallback with initials or an icon. Failed image loads are temporarily remembered so the UI does not retry the same broken image in a tight loop.

## Discovery Flow

Discovery uses Last.fm to suggest music related to the local library.

The refresh flow starts from favorite local artists. "Favorite" is based on signals already in SQLite, such as play count, track count, album count, and Last.fm play count. For each selected artist, the backend asks Last.fm for artist info, top albums, top tracks, and similar artists.

Each returned candidate is normalized and compared with the local library. The result is stored as a discovered release with fields such as source, release type, title, discovered artist, match status, confidence score, reason, external URL, cover URL, and raw upstream metadata.

Discovery pages then read from SQLite instead of calling Last.fm live. Last.fm responses are also cached with expirations, so refreshes are repeatable and avoid unnecessary external API calls.

## Playback And Streaming

Playback is also proxied through the backend. The frontend asks for a short-lived stream token, then builds a backend stream URL for the selected track. The backend looks up the local track row, finds its Subsonic identifier, decides whether direct or transcoded streaming should be used, forwards range-related headers when needed, and streams the upstream response back to the browser.

This keeps Subsonic credentials hidden, supports safer mobile playback, and lets the app apply network-aware stream preferences from settings.

The player can also send now-playing and scrobble events. Those are translated into Subsonic scrobble requests by the backend.

## Lyrics Flow

Lyrics are loaded per track through the backend. The backend first checks whether Subsonic can provide lyrics for the track. If not, it can fall back to LRCLIB using track metadata such as title, artist, album, and duration. The result is normalized into a single shape for the frontend, including whether the lyrics are synced, instrumental, plain text, or line-based.

## Offline And Local Media

The mobile-oriented local media layer can download tracks and related cover art for offline use. It stores a manifest of downloaded albums, playlists, tracks, and images. When offline mode is active, or when a network request fails in an offline-like way, selected library calls can fall back to local data.

For images, local object URLs are preferred before remote cache lookup. For audio, downloaded files can be played directly from local storage instead of using the backend stream route.

## Settings And Configuration

Configuration can come from environment variables or from saved settings. Environment variables are useful for deployment and first startup, while saved settings let the app update Subsonic, Last.fm, and playback behavior from the UI.

Sensitive values stay backend-side. The frontend only needs the backend URL, login session, and stream tokens issued by the backend.

## Error Handling Pattern

The backend generally returns explicit API errors instead of silently returning empty data. Upstream failures from Subsonic, Last.fm, image fetches, stream fetches, or lyrics lookups are logged and converted into frontend-readable error values.

The frontend catches these errors at page or component boundaries and shows either an error state, a fallback image, offline data, or a loading state depending on the feature.

## Mental Model

The simplest way to reason about the project is:

1. Sync builds a private local index from Subsonic/Navidrome and optional Last.fm data.
2. SQLite becomes the fast source for browsing, stats, discovery, and health checks.
3. Protected media and images are proxied by the backend so secrets stay private.
4. The frontend focuses on state, rendering, caching, playback controls, and offline-friendly behavior.
