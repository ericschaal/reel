# Reel web

The Next.js frontend for Reel's unified media catalogue.

## Development

Start the Reel API on port `3000`, then run:

```bash
npm run dev
```

Open [http://localhost:3001](http://localhost:3001). The frontend proxies catalogue requests to `REEL_API_URL`, which defaults to `http://localhost:3000`.

Series title pages load season summaries and episode metadata from Reel, then
enrich each episode with its Jellyfin local-copy status. Playback source and
streaming controls use Reel's playback endpoints. Movie and season requests
use Seerr through the Reel API.

Playback progress belongs to the movie or episode, not to a source. Reel keeps
the last source as the default for Resume, but can carry the saved timestamp to
another source when the user switches or that source disappears. The timestamp
is approximate across sources because different cuts may not align exactly.

Series requests select one or more regular seasons, defaulting to the viewed
season when it can be requested. Movie and series requests let the user choose
a quality profile from Seerr's default non-4K Radarr or Sonarr server. Seerr requests entire seasons, so episodes do
not have a separate request action. The detail page shows a compact request
status with its date and refresh action. Downloading and importing appear only
when Radarr or Sonarr reports a matching queue item; Jellyfin remains the
source of playable local availability.

The title page uses one split Play or Resume action for every source. Its main
action prefers the last source for Resume, then a local Jellyfin copy, then the
first available remote source; the dropdown exposes every available choice.
Series titles keep this action fixed on the authoritative next-up episode while
season browsing stays independent. Opening any episode, player, or download
flow uses a full-screen navigation surface rather than a modal so the same
information hierarchy can later support TV focus navigation. Playback uses its
own URL-backed route, so refreshing an open player reactivates the selected
Jellyfin movie or episode instead of returning to title details.

## Checks

Run `npm run lint` and `npm run build`. Run `npm test` with Node 22.18+
for navigation and player regression coverage. Player tests mount the real React
component in jsdom and simulate media events and HLS track discovery; real
browser decoding and Jellyfin streaming still require a playback smoke test.

Catalogue surfaces are URL-driven (`/?surface=movies` or `/?surface=series`).
Collection pages support automatic pagination and a manual Load more control.
Menus use shared Tailwind glass styles with a dark fallback for browsers without
backdrop filtering. Rails retain touch and keyboard scrolling with hidden scrollbars.
Visible media cards progressively enrich their compact metadata pills from the
title endpoints: movie runtime for films and season count for series. Requests
are cached by media identity so repeated titles across rails share one lookup.
