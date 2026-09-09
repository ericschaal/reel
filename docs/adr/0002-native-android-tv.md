# ADR 0002: Use a native Android TV application

- Status: Accepted
- Date: 2026-09-09

## Context

The Android TV client needs predictable remote focus, a ten-foot interface,
device-aware media playback, track selection, and playback-session integration.
Sharing React code with the web client would reduce some UI duplication, but the
TV experience and player integration are the higher-risk requirements.

Google recommends Compose for TV for Android TV interfaces and Media3 for
playback and media-session behavior.

## Decision

Build the TV client natively in Kotlin with Compose for TV. Use Media3/ExoPlayer
as the initial playback engine. Validate the required Jellyfin and Stremio media
formats in a focused playback prototype before building catalogue screens.

## Consequences

- Web and TV share backend contracts and product language, not UI code.
- Remote navigation and player behavior use the platform's primary toolchain.
- The project carries separate TypeScript and Kotlin frontend stacks.
- VLC or LibVLC remains a possible fallback only if format testing demonstrates
  a concrete Media3 gap.

## Sources

- <https://developer.android.com/training/tv/get-started/create>
- <https://developer.android.com/training/tv/playback>
