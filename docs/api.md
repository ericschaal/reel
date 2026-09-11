# Client API

Reel exposes read models for browsing and title details. Provider payloads stay
inside the Rust integrations. JSON uses camelCase, canonical media IDs remain
stable, nullable facts mean unknown, and successful collections use empty arrays
rather than null. These views are assembled in the catalogue module; there is no
application database requiring SQL views today.

## Choose the smallest view that serves the interaction

| Interaction | GET route | Result |
| --- | --- | --- |
| Catalogue layout | `/v1/catalogue/{surface}/manifest` | Ordered rail descriptors and `itemsHref` links; no upstream requests |
| One rail | Follow `itemsHref` | `{section, issues}` with compact cards |
| Entire surface | `/v1/catalogue/{surface}` | `{surface, sections, issues}`; useful for clients that need all rails together |
| Browse more | Follow a section/category `href`, then `next` | `{id, title, items, totalResults, next, issues}` |
| Optional standalone facts | `/v1/titles/summaries?ids=tmdb:movie:1,tmdb:series:2` | Batched runtime/season-count summaries |
| One title's extra facts | `/v1/titles/{kind}/{tmdbId}?view=summary` | The same compact summary for one title |
| Title details | `/v1/titles/{kind}/{tmdbId}` | Movie or series details, discriminated by `kind` |
| Series detail page and initial guide | `/v1/titles/series/{tmdbId}?include=initialSeason` | Series details with one initial season embedded |
| Switch seasons | `/v1/titles/series/{tmdbId}/seasons/{seasonNumber}` | One season and its episodes |
| Discover playback sources | `POST /v1/playback/sources` | Ordered local and direct AIOStreams candidates represented by opaque IDs |
| Activate playback | `POST /v1/playback/activate` | A short-lived Jellyfin or direct AIOStreams descriptor for an exact movie or episode |

`surface` is `discover`, `movies`, or `series`; title `kind` is `movie` or
`series`. TMDb IDs must be positive integers. Season zero means specials.
All routes accept optional `language`; it is preserved in server-generated
navigation links. Clients must follow `next` unchanged, not decode or increment
its opaque cursor. Cursors are tied to the collection and language.
`totalResults` is the provider's reported total; unsupported discovery item types
can be filtered out, so it is not a guarantee of the number of displayed cards.

Title `view` is `detail` by default, or `summary`. `include=initialSeason` is
valid only for a series detail view. Unknown query parameters, unknown views,
invalid combinations and invalid batches return 400 with `error.code` equal to
`invalid_query`. This avoids silently fulfilling an expensive, misspelled query.

## Cards and availability

Media cards contain `kind`, `id`, `tmdbId`, `title`, `year`, `rating`, `images`
(`poster`, `backdrop`), `availability`, `runtimeMinutes`, and `numberOfSeasons`.
Rails, full surfaces and collection pages include these facts before returning.
The inapplicable fact is null (season count on a movie, runtime on a series).
An unavailable metadata lookup leaves the card in place with null facts and a
Seerr issue identifying the affected section. They do not contain a synopsis,
episode hierarchy, watch progress, or provider playback IDs. Category cards retain
their own `kind: "category"`, artwork and collection links.

`availability` describes local-library knowledge:

- `local`: a local copy was found for the exact movie or episode.
- `notLocal`: the library check succeeded and found no local copy.
- `unknown`: the library check failed; do not infer absence.
- `episodeBased`: a series requires an episode-level decision.

An unavailable Jellyfin check never prevents metadata from being shown. Movie
details, seasons, rails, full surfaces and collections disclose partial failures
through `issues: [{source, sectionId, code: "upstreamUnavailable"}]`. A series
whose initial guide fails still returns its title metadata with an issue and
`initialSeason: null`. A successful guide reports episode-availability issues
inside `initialSeason.issues`.

Availability is separate from download status and from whether remote playback
could succeed. `notLocal` does not mean unplayable. A series is never marked local
based on the presence of a Jellyfin series container.

## Summary batches

Example request:

```text
GET /v1/titles/summaries?ids=tmdb:movie:1,tmdb:series:2&language=en
```

Example response:

```json
{
  "items": [
    {"kind": "movie", "id": "tmdb:movie:1", "runtimeMinutes": 124},
    {"kind": "series", "id": "tmdb:series:2", "numberOfSeasons": 3}
  ],
  "issues": []
}
```

Batches accept 1–40 comma-separated canonical IDs. Duplicates count toward the
input limit but are fetched and returned once. Successful items and issues each
preserve first-occurrence input order. An item appears in either `items` or
`issues`; clients match by ID, not array position. Missing titles use
`{id, code: "media_not_found"}` and failed upstream reads use
`{id, code: "catalogue_unavailable"}`. A valid batch returns 200 even if all items
fail. Invalid batches are rejected in full before any upstream I/O.

Summaries deliberately omit availability and episode guides. Seerr does not
supply runtime and season counts in discovery results, so a cold summary still
requires a provider title lookup. Reel caches metadata for five minutes, up to
512 title/language entries, shares concurrent cache misses and allows at most
eight simultaneous title-metadata requests per Catalogue instance. Both summary
and detail views reuse this metadata. Failures are not cached. Jellyfin movie
availability retains its separate 30-second cache; episodes are checked per
season request. This is an in-memory cache, not persisted user state.

web2 renders runtime and season count directly from rail and collection cards.
It performs no visibility-triggered metadata queries. Reel performs the necessary
provider lookups before returning a rail, using the shared cache and concurrency
limit described above. A cold rail may therefore take longer to assemble, but
its response is complete for the client. Unrelated rails still load independently.
Standalone summary routes remain available for consumers that already have title
identities; web2 does not need them for catalogue cards.

Availability-bearing rails, collections and seasons use a 30-second client
freshness window. These are read caches, not a live availability subscription.

## Details and extension boundary

Details include the card identity and artwork, synopsis and title-specific facts.
Series details include season summaries; episodes are loaded only through the
explicit expansion or the season endpoint. web2 uses the expansion for its detail
page, then loads only the selected season when the selector changes. It passes
one title response through the server/client component boundary, avoiding repeated
copies of the episode guide.

Playback discovery and activation are separate resources keyed by canonical
movie or episode identity. Neither is a side effect of catalogue, summary or
detail reads. Discovery is lazy and side-effect-free; activation rechecks the
exact local copy or validates an opaque AIOStreams candidate, then returns a
credential-free Reel media URL. See [Playback](playback.md).
Personalized resume state still has its own future freshness and authorization
policy rather than entering the shared metadata cache.

## Migration

This is an intentional change to the current pre-release v1 contract. Deploy the
API and web2 together. Other experimental clients must migrate before using it:

- Replace `localCopy` presence checks with `availability === "local"`.
- Read synopses from title details, not catalogue cards.
- Read card facts directly from rail and collection items.
- Explicitly request `include=initialSeason` when a series page needs episodes.
- Consume the `kind` discriminator on details and retain `issues` for degraded reads.

The old catalogue routes and collection/cursor link format are retained. HTTP
errors use `{error: {code, message}}` for domain and query errors; Axum's path
extractor still rejects malformed numeric path segments before domain handling.

## Verification

`cargo test -p reel-api --test title_views` uses local mock upstreams and verifies
batch bounds, partial results, validation before I/O, metadata reuse, language
isolation, explicit guide loading, complete rail/collection cards and partial enrichment failures. `npm test` in
`apps/web2` covers direct card-fact rendering without extra queries and link handling.
The full Rust suite additionally reads the configured live integrations.
