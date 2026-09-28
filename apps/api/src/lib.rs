pub mod aiostreams;
pub mod catalogue;
pub mod integration;
pub mod jellyfin;
pub mod media;
mod observability;
pub mod playback;
pub mod seerr;
pub mod stremio;

use axum::{Json, Router, routing::get};
use catalogue::Catalogue;
use serde::Serialize;

pub fn app(catalogue: Catalogue) -> Router {
    observability::trace_http(
        Router::new()
            .route("/healthz", get(health))
            .merge(catalogue::router(catalogue)),
    )
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}
