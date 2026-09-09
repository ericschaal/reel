# Development

Reel is currently a Rust workspace containing a single backend application. The
web and Android TV applications will be introduced when their first vertical
slices are ready to be exercised.

## Prerequisites

- A current stable Rust toolchain with `rustfmt` and Clippy.
- Node.js and pnpm will be required once the web application is initialized.
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

The backend listens on port 3000. Its initial health endpoint is available at
`GET /healthz` and returns:

```json
{"status":"ok"}
```

## Project structure

- `apps/api` — Rust backend and orchestration logic.
- `apps/web` — reserved for the React web application.
- `apps/tv` — reserved for the native Android TV application.
- `docs` — architecture, verified integration notes, playback research, and
  development guidance.

Keep backend code in feature-oriented modules within `apps/api` until reuse or
isolation demonstrates that a separate crate would provide a real seam.
