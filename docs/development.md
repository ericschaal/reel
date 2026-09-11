# Development

Reel currently contains a Rust backend, a disposable browser prototype, and an
Expo universal-client candidate for web and Android TV.

## Prerequisites

- A current stable Rust toolchain with `rustfmt` and Clippy.
- Node.js and a JavaScript package manager are required for the client projects.
- Access to test Jellyfin, Stremio, and Seerr instances will be required for
  integration work.

## Backend

Check and test the complete workspace:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Run the backend:

```sh
cargo run -p reel-api
```

The backend loads `apps/api/.env.local` before `.env`; existing process
environment variables still take precedence. Copy the required integration
keys from `.env.example` when setting up a local instance.

The backend listens on port 3000. Its initial health endpoint is available at
`GET /healthz` and returns:

```json
{"status":"ok"}
```

## Project structure

- `apps/api` — Rust backend and orchestration logic.
- `apps/web` — disposable browser prototypes; never promote them directly.
- `apps/reel-client` — Expo web and Android TV implementation candidate.
- `docs` — architecture, verified integration notes, playback research, and
  development guidance.

Keep backend code in feature-oriented modules within `apps/api` until reuse or
isolation demonstrates that a separate crate would provide a real seam.
