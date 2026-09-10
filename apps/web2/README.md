# Reel web

The Next.js frontend for Reel's unified media catalogue.

## Development

Start the Reel API on port `3000`, then run:

```bash
npm run dev
```

Open [http://localhost:3001](http://localhost:3001). The frontend proxies catalogue requests to `REEL_API_URL`, which defaults to `http://localhost:3000`.

Series title pages load season summaries and episode metadata from Reel, then
enrich each episode with its Jellyfin local-copy status. Playback source,
streaming, and download controls remain interactive previews until the
corresponding action endpoints are implemented.

## Checks

Run `npm run lint` and `npm run build`. Run `npm test` with Node 22.18+
for navigation URL regression coverage.

Catalogue surfaces are URL-driven (`/?surface=movies` or `/?surface=series`).
Collection pages support automatic pagination and a manual Load more control.
Menus use shared Tailwind glass styles with a dark fallback for browsers without
backdrop filtering. Rails retain touch and keyboard scrolling with hidden scrollbars.
