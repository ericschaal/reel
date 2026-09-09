use std::io;

use axum::{Json, Router, routing::get};
use serde::Serialize;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let app = Router::new().route("/healthz", get(health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app).await
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}
