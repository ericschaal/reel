# Reel

A self-hosted, Netflix-like media catalogue that unifies discovery, local playback, on-demand streaming, and media acquisition.

Reel is a personal media frontend that connects Jellyfin, Stremio, Seerr, Radarr, and Sonarr into one cohesive experience. It is designed for the web and Android TV, with a Rust backend responsible for orchestration and service integration.

The goal is not to replace the existing media ecosystem. It is to make the underlying services feel like one product.

## Product vision

Users should be able to browse a beautiful catalogue, open a movie or TV episode, and press **Watch Now** without needing to know which service will provide playback.

Reel automatically prefers a playable local copy from Jellyfin. When the title is not available locally, it offers streams resolved through configured Stremio add-ons and an existing official Stremio streaming server.

Users can also request a title for download through Seerr, which delegates acquisition to Radarr or Sonarr. Once imported and available in Jellyfin, the local copy becomes the default playback source.

### Core principles

* One catalogue, independent of the underlying media services.
* One primary Watch Now action with intelligent source routing.
* Local Jellyfin playback is preferred when available.
* Users can explicitly choose another playback source.
* Downloading is a separate action from streaming.
* Availability is tracked at the movie or episode level.
* Existing services remain responsible for their specialized jobs.
* The interface should feel like a consumer media product, not an administration dashboard.

## User experience

### Movies

A movie detail page includes artwork, synopsis, metadata, availability, and playback actions.

If the movie is available in Jellyfin:

* **Watch Now** plays the local copy.
* **Other Sources** allows playback through Stremio.
* Library and download management actions remain available.

If the movie is not available locally:

* **Watch Now** resolves available Stremio streams.
* **Download** creates a request through Seerr.
* Request and download progress are displayed when available.

### TV series

Series availability is tracked at the episode level. Reel must not assume that an entire series is available merely because it exists in Jellyfin.

The user can browse seasons and episodes, resume playback, and select a source for a specific episode. Locally available episodes default to Jellyfin; missing episodes can use Stremio.

### Android TV

The TV application is a genuine ten-foot interface designed for remote control navigation. It should provide predictable focus behavior, large artwork, readable typography, and a simple playback experience.

The initial playback target is Android Media3/ExoPlayer, with VLC or LibVLC considered as an optional fallback where appropriate.

## Integrations

### Jellyfin

Jellyfin is the source of truth for local, playable media availability.

Reel uses Jellyfin APIs to discover library items, resolve playback information, obtain direct or transcoded streams, retrieve media tracks, and synchronize playback progress.

A completed download does not automatically mean the item is playable. Reel should wait until the exact movie or episode is available through Jellyfin.

### Stremio

Reel integrates with configured Stremio add-ons for metadata and stream discovery, and with an existing Docker deployment of the official Stremio streaming server for supported stream resolution.

The application does not implement its own torrent engine.

Stremio stream objects may contain direct HTTP URLs, torrent information, or other source types. Their handling must be based on verified protocol and streaming-server behavior. The exact API endpoints, stream URL construction, and capabilities must be investigated before implementation.

### Seerr, Radarr, and Sonarr

Seerr is the preferred user-facing request service. Radarr and Sonarr remain responsible for acquisition and library automation.

Reel may integrate with Radarr and Sonarr directly for detailed monitoring and administrative functionality, but should avoid duplicating their internal logic or configuration interfaces.

### Metadata and identifiers

The catalogue uses canonical external identifiers such as TMDb, IMDb, and TVDb, with explicit mappings between services.

Identity resolution must be reliable, particularly for individual TV episodes. Titles and release years alone are not sufficient identifiers.

## Architecture

Reel consists of three primary applications:

* A Rust backend that exposes a unified API and orchestrates external services.
* A React web application for discovery, playback, requests, and administration.
* An Android TV application optimized for remote navigation and media playback.

The backend owns service credentials, normalized domain state, and playback routing. Clients should not need to understand the APIs of every underlying service.

### Proposed stack

* Rust, Axum, Tokio, Reqwest, and Serde.
* SQLx with SQLite for the initial self-hosted deployment.
* React and TypeScript for the web frontend.
* Android TV using React Native TV or native Android, to be decided through a focused prototype.
* Media3/ExoPlayer as the initial embedded playback engine.
* Docker Compose for deployment.

These are starting preferences, not immutable requirements. Technical choices should be validated through small prototypes and documented tradeoffs.

### Backend responsibilities

The Rust backend is responsible for:

* Service configuration and health.
* External API adapters.
* Canonical media identity mapping.
* Catalogue aggregation and metadata caching.
* Local availability reconciliation.
* Request and download status normalization.
* Playback source selection.
* Playback session creation and progress synchronization.
* User preferences and application-level history.
* Authentication and secure handling of service credentials.

The backend should normally return playable URLs rather than proxying all media traffic. A media proxy should only be introduced when required for authentication, compatibility, or another concrete technical reason.

### Domain concepts

The initial domain model should include:

* `MediaId`
* `Movie`
* `Series`
* `Episode`
* `Availability`
* `MediaRequest`
* `DownloadStatus`
* `PlaybackSource`
* `PlaybackDescriptor`
* `PlaybackSession`

External API response types should remain separate from internal domain models.

## Playback routing

The default routing behavior is:

1. Resolve the exact movie or episode.
2. Check whether Jellyfin has a playable local copy.
3. If available, create a Jellyfin playback descriptor.
4. Otherwise, discover available Stremio stream candidates.
5. Allow the user to select a candidate when needed.
6. Resolve the selected stream through the existing Stremio streaming server if required.
7. Return a normalized playback descriptor to the client.
8. Start playback using the Android player or the appropriate web playback implementation.

Explicit user source overrides must take precedence over the default routing rule. Failed integrations should produce actionable errors rather than silently switching sources in unexpected ways.

## Initial MVP

The first milestone is a small, working vertical slice:

1. Display a catalogue and search for a movie.
2. Open a movie detail page.
3. Resolve its local availability through Jellyfin.
4. Play a local movie using a supported Jellyfin stream.
5. Discover Stremio stream candidates.
6. Resolve and play a supported stream through the existing streaming server.
7. Create a download request through Seerr.
8. Display request and download status.
9. Detect when the imported title becomes available in Jellyfin.
10. Prefer Jellyfin for subsequent playback.

A complete vertical slice is more valuable than several partially implemented features.

## Non-goals

The initial project will not:

* Reimplement Radarr or Sonarr.
* Build a new torrent client or streaming engine.
* Replace Jellyfin's transcoding infrastructure.
* Recreate every advanced setting from existing services.
* Implement a complex recommendation or AI system before the core experience works.
* Support every possible Stremio add-on or stream format from day one.
* Proxy all media through the backend without a demonstrated need.

## Development philosophy

Reel is a learning-oriented, hand-coded project. AI assistance is used for research, explanations, architectural review, debugging, and focused code review. The developer remains responsible for writing and understanding the implementation.

The project should favor small, testable increments, clear interfaces, and simple solutions over speculative abstractions. Integration behavior must be verified against real documentation, source code, or local experiments rather than assumed.

## Documentation

Maintain documentation as the project evolves:

* `docs/architecture.md` — Architecture and major design decisions.
* `docs/integrations.md` — Verified API contracts and integration limitations.
* `docs/playback.md` — Playback resolution and player behavior.
* `docs/adr/` — Short architecture decision records.
* `docs/development.md` — Local setup and development workflow.

## Status

Early design and investigation. The architecture and integration details are subject to change as prototypes validate the assumptions.
