# Playback

Reel activates local Jellyfin playback and direct HTTP(S) streams returned by a
personalized AIOStreams search. Torrent and Stremio streaming-server activation
are deliberately outside this slice.

## Discovery

`POST /v1/playback/sources` performs side-effect-free source discovery for a
canonical movie or episode target. Reel checks local Jellyfin availability and
queries AIOStreams `GET /api/v1/search` with Basic authentication, `format=true`
and `requiredFields=url`. Movie searches use `tmdb:{movieId}`; episode searches
use `tmdb:{seriesId}:{season}:{episode}`.

The response preserves AIOStreams order and exposes presentation metadata plus
opaque discovery/candidate IDs. Upstream URLs, request headers, addon
credentials, info hashes, and provider error details stay on the API. Movie
details begin discovery in the background when opened. Series details do the
same for the next episode and switch to the exact episode when its detail opens.
The web query cache deduplicates requests and cancels obsolete episode queries.
Discovery never accesses a returned media URL or activates a stream.

## Activation

`POST /v1/playback/activate` accepts a canonical target, a normalized web
capability report, an optional resume position, track indexes, and an optional
source selection. `auto` tries Jellyfin first and falls back to the first
web-ready direct AIOStreams result when no compatible local copy is available.
An explicit AIOStreams selection requires matching, unexpired opaque discovery
and candidate IDs; clients cannot submit upstream URLs.

The response is a normalized descriptor containing an opaque playback session,
the selected delivery mode (`direct` or `hls`), a Reel media URL, container and
duration. Jellyfin item IDs and credentials are never returned.

## Media delivery

Playback sessions live in API memory for six hours. Direct streams preserve
byte-range response headers. Jellyfin resources remain limited to the selected
item on the configured origin. AIOStreams request headers remain in backend
session state and are applied only to the selected upstream. Its HLS playlists
use opaque resource IDs for nested playlists, segments, and keys.

AIOStreams media destinations must use HTTP(S), cannot contain URL credentials,
and are checked against local, private, link-local, reserved, and documentation
networks. DNS answers are validated and pinned for every request; redirects are
followed only after the same validation. The explicitly configured AIOStreams
origin is trusted so a self-hosted proxy URL can work. These checks prevent the
media routes from becoming a client-controlled open proxy or SSRF primitive.

The web app exposes `/v1/playback/*` as a same-origin Next.js rewrite to the
Reel API. It uses the browser video element for direct playback. HLS loads `hls.js` on
demand when Media Source Extensions are supported, with native HLS as a
fallback. This avoids native demuxers that advertise HLS but fail on subtitle
renditions. Jellyfin subtitle negotiation requests HLS delivery so the player
can load WebVTT cues through the same scoped session proxy.

Both local and direct sources use the same URL-backed play route and the same
full-featured `ReelVideoPlayer`. The primary segment of the split **Watch Now**
control activates the preferred source without waiting for discovery; its
secondary segment opens the cached, ordered source picker.

## Current limits

- Playback sessions are local to one API process and do not survive restarts.
- Reel does not yet persist progress or report playback progress to Jellyfin.
- Playback descriptors include Jellyfin's normalized audio and subtitle stream
  lists. Choosing a stream re-negotiates playback at the current position with
  that exact Jellyfin stream index. Reel first resolves the preferred media
  source, then supplies its ID with the selected indexes because Jellyfin
  ignores track selections without a matching media-source ID. HLS-native
  tracks remain available as a
  fallback when upstream metadata is absent.
- Direct AIOStreams sources do not currently expose alternate audio or subtitle
  tracks; the existing track controls remain available for Jellyfin descriptors.
- Direct MKV playback depends on the browser and encoded codecs; Reel does not
  transcode it in this slice.
- `infoHash`/magnet torrents, Stremio streaming-server activation, NZB, archive,
  YouTube, and external-app sources are unsupported.

## Integration tests

`cargo test -p reel-api --test playback` uses the real Jellyfin instance, following
`tests/jellyfin.rs` and `tests/seerr.rs`. It loads `JELLYFIN_BASE_URL`,
`JELLYFIN_API_KEY`, and `JELLYFIN_USERNAME` from the environment or
`apps/api/.env.local`. When Jellyfin episodes only have TVDB metadata, fixture
discovery also uses `SEERR_BASE_URL` and `SEERR_API_KEY` to resolve their canonical
TMDb IDs from the season guide.

Tests discover fixtures among the first 100 movies and episodes. The library
must contain a movie with TMDb metadata and an episode with series TMDb metadata,
audio, and at least two text subtitle tracks, including a non-default,
non-forced track with cues in its first ten subtitle segments. Missing credentials,
unavailable services, or unsuitable media fail the tests rather than skipping them.

Coverage includes movie activation and byte ranges, exact episode resource
scoping, audio selection, subtitle selection in the real HLS manifest, proxied
WebVTT cues, and turning subtitles off. For movies requiring HLS, the range check
uses the session's static media resource. The HLS check downloads one video
segment and stops its encoding session. Pure URL rewriting and resource
validation remain unit tests in `src/playback.rs`.

`cargo test -p reel-api --test aiostreams -- --ignored` opts into a sanitized
test against the configured real AIOStreams instance. It verifies direct movie
discovery, exact episode query acceptance, and a one-byte media range probe
without printing upstream URLs or credentials.
