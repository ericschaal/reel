# Playback

Reel currently activates local Jellyfin playback for canonical movies and
individual episodes. Stremio discovery and activation are not part of this
slice.

## Activation

`POST /v1/playback/activate` accepts a canonical target, a normalized web
capability report and an optional resume position. Movie targets carry a TMDb
movie ID. Episode targets carry the episode and series TMDb IDs plus their
season and episode coordinates. Reel resolves the exact Jellyfin item again at
activation time; catalogue availability is advisory and may be stale.

The response is a normalized descriptor containing an opaque playback session,
the selected delivery mode (`direct` or `hls`), a Reel media URL, container and
duration. Jellyfin item IDs and credentials are never returned.

## Media delivery

Playback sessions live in API memory for six hours. Their media route proxies
only Jellyfin `/Videos/{itemId}/...` resources on the configured Jellyfin
origin. Direct streams preserve byte-range response headers. HLS playlists are
rewritten so playlists, segments and keys remain inside the same scoped Reel
session. The Jellyfin API key is added only by the backend HTTP client.

The web app exposes `/v1/playback/*` as a same-origin Next.js rewrite to the
Reel API. It uses the browser video element for direct playback and native HLS.
When native HLS is unavailable, `hls.js` is loaded on demand after activation.

## Current limits

- Playback sessions are local to one API process and do not survive restarts.
- Reel does not yet persist progress or report playback progress to Jellyfin.
- Audio and subtitle source selection is left to the negotiated/default stream.
- Stremio sources and automatic remote fallback remain out of scope.
