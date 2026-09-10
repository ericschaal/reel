pub mod catalogue;
pub mod integration;
pub mod jellyfin;
pub mod media;
pub mod seerr;

use axum::{Json, Router, routing::get};
use catalogue::Catalogue;
use serde::Serialize;

pub fn app(catalogue: Catalogue) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .merge(catalogue::router(catalogue))
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}
