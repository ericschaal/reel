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
  download is not a local copy until Jellyfin exposes the media. A local copy
  remains the preferred source whether Jellyfin selects direct play, remuxing,
  direct streaming, or transcoding for the requesting player.

**Availability**
: Reel's current knowledge of whether canonical media has a playable local copy.
  Series availability is derived from individual episodes rather than assigned
  to the series as a whole.

## Playback

**Playback source**
: A specific local or remote way to play canonical media, such as a Jellyfin
  media item or an AIOStreams direct-stream candidate.

**Playback Resolution**
: The Reel capability that chooses or resolves a playback source for exact
  canonical media. It prefers a local copy unless the user explicitly overrides
  that preference. When no local copy is available, it tries remote candidates
  in the order returned by configured AIOStreams. A user may explicitly
  override either choice. Playback Resolution reports actionable failures rather
  than switching sources silently after a source has been selected.

**Candidate order**
: The priority order of remote playback sources returned by configured
  AIOStreams. Reel does not independently rank candidates in the initial
  implementation. It preserves this order for automatic resolution and for the
  user's source picker.

**Source discovery**
: The side-effect-free retrieval of available playback sources. Reel starts it
  in the background when a movie detail or exact episode detail is opened, then
  deduplicates and briefly caches the ordered result for the source picker. It
  may contact configured add-ons through AIOStreams, but it must not access a
  returned media URL or activate a torrent.

**Source activation**
: Turning a selected playback source into a playback descriptor. Activation may
  contact Jellyfin or create a scoped direct-stream proxy session. Reel performs
  it only in response to **Watch Now** or an explicit source selection.
  Activation considers the requesting player's capabilities when asking
  Jellyfin for playback information.

**Resolution fallback**
: If a candidate cannot be turned into a playback descriptor, Playback
  Resolution automatically tries the next candidate in candidate order. Once a
  playback descriptor has been handed to the player, Reel must not silently
  switch sources. A playback failure instead offers the user an explicit **Try
  Next Source** or **Choose Source** action.

**Playback descriptor**
: The normalized instructions a Reel client needs to start playback. It includes
  a scoped Reel media URL and may include format information, tracks, and the
  data needed to report progress. Upstream URLs, request headers, and credentials
  remain in backend session state.

**Player capabilities**
: A normalized, dynamically reported description of what the requesting player
  can play, including relevant containers, codecs, streaming protocols, HDR
  modes, and practical limits. Clients do not send Jellyfin-specific types. The
  Jellyfin module translates player capabilities into Jellyfin playback
  information, using conservative defaults when capabilities are unavailable.

**Playback session**
: Reel's record of an attempted or active playback, including its selected
  source and progress synchronization state. Reel owns watch progress and resume
  state; it does not synchronize them through a Jellyfin user. The disposable
  prototype may keep this state in `reel-client`. A later multi-device version
  moves it behind the Reel API.

**Jellyfin service identity**
: Reel authenticates to Jellyfin with a dedicated API key. It does not log in as,
  impersonate, or hold an access token for a Jellyfin user. Jellyfin requires a
  user identifier as context for some item-detail and playback-negotiation
  operations, so Reel supplies the dedicated `reel` user's identifier only for
  those operations. Reel does not use Jellyfin resume, played-state, or playback
  progress reporting. The API key is a privileged backend credential and must
  not be shipped in a client application.

**Trusted Reel client**
: The provisional MVP assumption that the native Android TV and React web
  clients run on user-controlled devices on a private network. They may receive
  narrowly scoped upstream credentials required for direct playback. The
  Jellyfin service API key is explicitly excluded and remains backend-only. Reel
  still keeps unrelated integration credentials out of playback descriptors.
  External access and untrusted clients are unsupported until this assumption
  is revisited.

## Acquisition

**Media request**
: A user's instruction to acquire canonical media through Seerr. Requesting or
  downloading media is separate from playing a remote stream.

**Download status**
: Normalized acquisition progress reported by Seerr, Radarr, or Sonarr. It does
  not imply local availability.
