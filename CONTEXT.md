# Reel domain context

This document defines the shared language used in Reel's code and architecture.
It is intentionally narrower than the product overview in `README.md`.

## Media

**Canonical media**
: A movie or individual episode whose identity is stable across Reel's external
  integrations. Canonical media is resolved through verified external
  identifiers, not by matching titles and years alone.

**Local copy**
: An exact movie or episode that Jellyfin reports as playable. A completed
  download is not a local copy until Jellyfin exposes the media.

**Availability**
: Reel's current knowledge of whether canonical media has a playable local copy.
  Series availability is derived from individual episodes rather than assigned
  to the series as a whole.

## Playback

**Playback source**
: A specific local or remote way to play canonical media, such as a Jellyfin
  media item or a Stremio stream candidate.

**Playback Resolution**
: The Reel capability that chooses or resolves a playback source for exact
  canonical media. It prefers a local copy unless the user explicitly overrides
  that preference, and it reports actionable failures rather than switching
  sources silently.

**Playback descriptor**
: The normalized instructions a Reel client needs to start playback. It may
  include a playable URL, request headers, format information, tracks, and the
  data needed to report progress. Its exact shape is not yet decided.

**Playback session**
: Reel's record of an attempted or active playback, including its selected
  source and progress synchronization state.

## Acquisition

**Media request**
: A user's instruction to acquire canonical media through Seerr. Requesting or
  downloading media is separate from playing a remote stream.

**Download status**
: Normalized acquisition progress reported by Seerr, Radarr, or Sonarr. It does
  not imply local availability.
