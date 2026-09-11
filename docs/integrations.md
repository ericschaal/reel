# Integration contracts

## AIOStreams Search API

Verified against the configured live instance on 2026-09-11:

- `GET /api/v1/search` accepts Basic authentication with the stored user UUID
  and password.
- Movie identity is `type=movie&id=tmdb:{movieId}`.
- Exact episode identity is
  `type=series&id=tmdb:{seriesId}:{season}:{episode}`.
- The `requiredFields=url` filter and `format=true` are accepted.
- The response envelope contains `success`, optional `error`, and `data` with
  ordered `results`, `filtered`, `statistics`, and per-source `errors`.
- Direct results expose an HTTP(S) `url`, optional `requestHeaders`, addon and
  service metadata, cache state, size, duration, `notWebReady`, and optional
  parsed file facts such as container, extension, resolution, and quality.
- The observed movie fixture returned cached HTTPS debrid links. A one-byte
  range probe reached a direct source through one redirect and received HTTP
  206 with `application/force-download`.
- Exact episode searches are accepted but the number of direct results can
  vary between calls as configured upstream addons respond or time out.

Reel asks AIOStreams only for results containing `url`, validates every result
again, and preserves the surviving order. `infoHash`, torrent trackers/file
selection, NZB, archive URL collections, YouTube IDs, and external application
URLs are ignored. No Stremio account or Stremio streaming-server request occurs
in this integration.

The Search API is an external versioned contract. Pin the deployed AIOStreams
image and run `cargo test -p reel-api --test aiostreams -- --ignored` after
upgrades. The test uses `apps/api/.env.local` and deliberately emits no
credentials or stream URLs.
