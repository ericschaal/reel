# Architecture

## Current shape

Reel begins as a modular monolith with independently deployable applications:

- `reel-api`, a Rust backend that owns orchestration, credentials, normalized
  state, and playback policy;
- an Expo universal-client candidate targeting web and Android TV.

The backend and Expo client scaffold are initialized. A separate disposable web
prototype is used to settle interactions before they are rewritten with React
Native primitives. The final Android TV implementation remains subject to the
playback and focus spike recorded in ADR 0002.

## Backend organization

Backend behavior stays in feature-oriented modules within `apps/api`. The first
planned deep module is Playback Resolution. Media Identity and Availability may
become supporting modules as the vertical slice demonstrates their interfaces.

External response types remain private to their integration modules and are
translated into Reel's domain language at the point of use. Separate workspace
crates are introduced only after actual reuse, isolation, or build constraints
justify a seam.

The backend returns normalized playback descriptors to clients. For the
private-network MVP, the native TV and React web clients are trusted and a
descriptor may include upstream credential material required by the selected
source. This keeps media bytes flowing directly from Jellyfin or the configured
Stremio streaming server. Unrelated integration credentials remain in the
backend. External access and untrusted clients are unsupported until this
assumption is revisited.

## State ownership

Jellyfin is authoritative for playable local availability. Seerr, Radarr, and
Sonarr are authoritative for acquisition progress. Reel may store normalized
projections, identifier mappings, user preferences, and application history in
SQLite, but must not reinterpret a completed download as playable availability.

## Decisions

- [ADR 0001: Begin as a modular monolith](adr/0001-modular-monolith.md)
- [ADR 0002: Use a native Android TV application](adr/0002-native-android-tv.md)

Playback Resolution remains the next decision area rather than an accepted
interface.

## Catalogue API read models

The catalogue module owns compact cards, title summaries and title detail views.
Rails and collections include runtime and season counts directly in their cards.
Card enrichment, summaries and details share bounded metadata caching, while
availability is refreshed separately. Episode guides are an
explicit series detail expansion. The contract and migration notes are in
[Client API](api.md). Playback and progress remain separate future capabilities.
