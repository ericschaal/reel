# Playback prototype

Throwaway UI prototype. Run it with:

```sh
cd apps/web
pnpm prototype
```

Open <http://127.0.0.1:4173/prototype/playback>. Use the floating arrows or the
`variant` query parameter to switch among:

- `A` — cinematic detail with sources revealed on demand;
- `B` — playback options are the primary information hierarchy;
- `C` — compact detail with a persistent source sheet.

## Question

What is the simplest useful interaction for one movie, automatic **Watch Now**,
and explicit source override before Playback Resolution's interface is fixed?

## Verdict

Variant A, **Cinematic reveal**, was selected on 2026-09-09.

What won:

- the movie remains the visual focus;
- **Watch Now** is the dominant action;
- automatic source selection does not expose integration complexity;
- **Other Sources** reveals ordered alternatives only when requested.

The interaction should be rewritten as a real application route rather than
promoting this throwaway implementation directly. Delete variants B and C and
the prototype switcher when that rewrite begins.
