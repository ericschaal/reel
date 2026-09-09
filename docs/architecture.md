# Architecture

## Current shape

Reel begins as a modular monolith with three independently deployable
applications:

- `reel-api`, a Rust backend that owns orchestration, credentials, normalized
  state, and playback policy;
- a React web application;
- a native Android TV application built with Kotlin and Compose for TV.

Only the backend is initialized. The web and TV applications should be created
when their first end-to-end behavior is ready to exercise.

## Backend organization

Backend behavior stays in feature-oriented modules within `apps/api`. The first
planned deep module is Playback Resolution. Media Identity and Availability may
become supporting modules as the vertical slice demonstrates their interfaces.

External response types remain private to their integration modules and are
translated into Reel's domain language at the point of use. Separate workspace
crates are introduced only after actual reuse, isolation, or build constraints
justify a seam.

The backend returns normalized playback descriptors to clients. Media bytes
should flow directly from Jellyfin or the configured Stremio streaming server
unless authentication or compatibility testing demonstrates a need for a Reel
proxy.

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
