# Integration contracts

## AIOStreams Search API

Verified against the configured live instance on 2026-09-11:

- `GET /api/v1/search` accepts Basic authentication with the stored user UUID
  and password.
- Reel prefers Stremio's IMDb title identity (`tt…`) and falls back to TMDb when
  Seerr has no IMDb mapping. Exact episodes append `:{season}:{episode}`.
- The `requiredFields=url` filter and `format=true` are accepted.
- The response envelope contains `success`, optional `error`, and `data` with
  ordered `results`, `filtered`, `statistics`, and per-source `errors`.
- Direct results expose an HTTP(S) `url`, optional `requestHeaders`, addon and
  service metadata, cache state, size, duration, `notWebReady`, and optional
  parsed file facts such as container, extension, resolution, quality, and
  encoded video codec. Without a streaming server, Reel combines those facts with
  the requesting browser's direct-play profiles; `notWebReady: false` alone does not prove that a browser
  can demux and decode a stream.
- The observed movie fixture returned cached HTTPS debrid links. A one-byte
  range probe reached a direct source through one redirect and received HTTP
  206 with `application/force-download`.
- Identifier parity with Stremio matters. For series 5920 season 1 episode 1,
  both the personalized Stremio endpoint and Search API returned seven direct,
  web-ready results for `tt1196946:1:1`, while both returned zero for
  `tmdb:5920:1:1`. Reel therefore uses the IMDb identifier exposed by Seerr and
  retains the TMDb form only as a fallback.
- Partial provider failures can accompany usable results. They remain a
  non-fatal `partialResults` issue while the direct results stay available.

Reel asks AIOStreams only for results containing `url`, validates every result
again, and preserves the surviving order. `infoHash`, torrent trackers/file
selection, NZB, archive URL collections, YouTube IDs, and external application
URLs are ignored. Discovery does not contact the Stremio streaming server.
Activation uses it for HLS delivery when configured (see `playback.md`).

The Search API is an external versioned contract. Pin the deployed AIOStreams
image and run `cargo test -p reel-api --test aiostreams -- --ignored` after
upgrades. The test uses `apps/api/.env.local` and deliberately emits no
credentials or stream URLs.


## Official Stremio streaming server

Verified on 2026-09-11 against the running official Stremio server and its bundled
`server.js` implementation:

- `/hlsv2/{id}/master.m3u8?mediaURL=…&videoCodecs=h264&audioCodecs=aac&maxAudioChannels=2`
  probes the input and returns an HLS master playlist.
- Nested video/audio playlists, `init.mp4`, and `segmentN.m4s` resources remain
  under `/hlsv2/{id}/`. Query parameters carry the input into subsequent requests.
- Reel supplies the resolved CDN URL to the trusted server, or an opaque API
  input URL when `REEL_STREAMING_BASE_URL` is set. Playlist resources are always
  rewritten to Reel's scoped session routes; provider URLs do not reach clients.
- The Mentalist S01E01's recommended VC-1/AC3 MKV was converted to H.264/AAC HLS
  and played at 1920×1080, duration 2709 seconds. Safari can therefore use HLS
  rather than being excluded by the MKV direct-play filter.
- Two MediaFusion candidates marked cached redirected to
  `slate.elfhosted.com` error videos. Reel rejects this destination during
  activation; HTTP success and `cached: true` do not establish playability.
