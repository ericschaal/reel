# Playback

Reel activates local Jellyfin playback and direct HTTP(S) streams returned by a
personalized AIOStreams search. The official Stremio streaming server converts
remote sources to HLS when configured. Torrent activation remains unsupported.

## Discovery

`POST /v1/playback/sources` performs side-effect-free source discovery for a
canonical movie or episode target. Reel checks local Jellyfin availability and
queries AIOStreams `GET /api/v1/search` with Basic authentication, `format=true`
and `requiredFields=url`. Reel prefers the title's validated IMDb ID because it
matches the identifier Stremio sends to addons: movies use `tt…` and episodes use
`tt…:{season}:{episode}`. When Seerr has no IMDb mapping, Reel falls back to the
equivalent `tmdb:…` identifier.

The response preserves AIOStreams order and exposes presentation metadata plus
opaque discovery/candidate IDs. A result set with usable sources and provider
errors is reported as partial; zero returned results with provider failures is
upstream unavailable. With Stremio configured, an HLS-capable browser can select
MKV and other sources even when native direct playback is unsupported. Without
Stremio, Reel applies the provider's `notWebReady` flag and browser direct-play
profiles. Unknown codecs are not assumed compatible. Upstream URLs, request
headers, addon credentials, info hashes, and provider error details stay on the API. Movie
details begin discovery in the background when opened. Series details do the
same for the next episode and switch to the exact episode when its detail opens.
The web query cache deduplicates requests and cancels obsolete episode queries.
Discovery never accesses a returned media URL or activates a stream.

## Activation

`POST /v1/playback/activate` accepts a canonical target, a normalized web
capability report, an optional resume position, track indexes, and an optional
source selection. `auto` tries Jellyfin first and falls back to the first
usable AIOStreams result when no compatible local copy is available. Activation
resolves each candidate with a one-byte range request, rejects failed HTTP
responses and known provider error-video redirects, and prepares the Stremio
master playlist before returning a descriptor. Automatic activation tries the
next candidate on failure; explicit selection returns an actionable error.
When background discovery has completed, the client sends its opaque discovery
ID with `auto`; activation reuses that ordered snapshot instead of issuing a
second, potentially different AIOStreams search. It falls back to a fresh search
only when discovery was unavailable.
An explicit AIOStreams selection requires matching, unexpired opaque discovery
and candidate IDs; clients cannot submit upstream URLs.

The response is a normalized descriptor containing an opaque playback session,
the selected delivery mode (`direct` or `hls`), a Reel media URL, container and
duration. Jellyfin item IDs and credentials are never returned.

## Media delivery

Playback sessions live in API memory for six hours. Direct streams preserve
byte-range response headers. AIOStreams downloads that use a generic attachment
content type are served with the selected container's video MIME type (for
example `video/x-matroska`). An explicit upstream video MIME type is preserved.
Media requests use an inactivity timeout rather than a total transfer timeout,
so a long-playing stream is not cut off after 60 seconds. Jellyfin
resources remain limited to the selected item on the configured origin.
AIOStreams request headers remain in backend session state and are applied only
to the selected upstream. Its HLS playlists use opaque resource IDs for nested
playlists, segments, and keys.

AIOStreams media destinations must use HTTP(S), cannot contain URL credentials,
and are checked against local, private, link-local, reserved, and documentation
networks. DNS answers are validated and pinned for every request; redirects are
followed only after the same validation. The explicitly configured AIOStreams
origin is trusted so a self-hosted proxy URL can work. These checks prevent the
media routes from becoming a client-controlled open proxy or SSRF primitive.

When `STREMIO_BASE_URL` is configured, Reel uses the official server's
`/hlsv2/{sessionId}/master.m3u8` endpoint with H.264 video, AAC audio, and two
audio channels. The server copies compatible video and converts other codecs.
By default its `mediaURL` is the selected CDN URL, already resolved and validated
by Reel. The configured Stremio server is a trusted backend and may receive the
signed URL; the browser receives only opaque Reel media/resource URLs. This
allows a remote or VPN-hosted converter to fetch media without reaching back to
the client machine.

To keep upstream input traffic through Reel, set `REEL_STREAMING_BASE_URL` to the
API address reachable **from Stremio**. The server then receives Reel's scoped
`/sessions/{sessionId}/input` URL. This mode is required for sources that need
custom request headers; Reel never silently drops those headers. The input route
retains the existing DNS/redirect validation and range support. Both modes reuse
the same opaque HLS resource proxy, restricted to that session on the configured
Stremio origin.

The web app exposes `/v1/playback/*` as a same-origin Next.js rewrite to the
Reel API. It uses the browser video element for direct playback. HLS loads `hls.js` on
demand when Media Source Extensions are supported, with native HLS as a
fallback. This avoids native demuxers that advertise HLS but fail on subtitle
renditions. Jellyfin subtitle negotiation requests HLS delivery so the player
can load WebVTT cues through the same scoped session proxy.

Both local and direct sources use the same URL-backed play route and the same
full-featured `ReelVideoPlayer`. The primary segment of the split **Watch Now**
control activates the preferred source from cached background discovery; its
secondary segment opens a full-screen, TV-friendly ordered source view. The
recommended source receives initial focus, arrow keys move predictably through
the list, and Escape/back restores focus to the split-button trigger.

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
  tracks in the descriptor. Stremio HLS rendition tracks are available through
  the existing player controls.
- Without a configured Stremio server, direct MKV playback depends on the browser
  and encoded codecs.
- `infoHash`/magnet torrents, NZB, archive,
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
discovery, direct discovery for the reported series 5920 season 1 episode 1, and
a one-byte media range probe without printing upstream URLs or credentials.


## Real episode browser test

Start the API and web app with the integrations in `apps/api/.env.local`.
Configure `STREMIO_BASE_URL` as in `.env.example`. Optionally set
`REEL_STREAMING_BASE_URL` for proxy input; it must be the API address seen by
Stremio, not the container's own loopback address. Leave it unset when Stremio
cannot reach the API and the provider supplies a self-contained CDN URL.

From `apps/web2` run:

```sh
npx playwright install chromium webkit
npm run test:e2e
```

The test selects The Mentalist, opens season 1 episode 1 (Pilot), loads its real
AIOStreams candidates, and selects the first AIOStreams source in Reel. Both
Chromium and WebKit must return an AIOStreams HLS descriptor, decode actual
video, advance at least three seconds, and report a 40–50 minute duration. The
duration assertion rejects the provider's two-minute error video, which itself
can produce a successful HTTP response and an advancing playback clock. A
screenshot records the episode frame. Tests use no mocked media and do not skip
missing services or empty source lists. `REEL_WEB_URL` overrides the web origin.

Verified on 2026-09-11 with Stremio 4.21.0 at
`http://stremio.home.schaal.dev/`, using resolved CDN input: Chromium and WebKit
both passed (2 tests, 39.2 seconds). Screenshots confirmed the Pilot's opening
scene. A valid debrid subscription is required for these live provider sources.

The shared scenario in `tests/e2e/mentalist-playback.mjs` also runs through the
Codex browser API; this was used to reproduce the original error-video failure
and visually verify the actual Pilot after the fix.
