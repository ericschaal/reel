use std::io;

use reel_api::{app, catalogue::Catalogue, jellyfin::Jellyfin, seerr::Seerr};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    tracing_subscriber::fmt::init();

    let catalogue = Catalogue::new(
        Seerr::new(
            required_env("SEERR_BASE_URL")?,
            required_env("SEERR_API_KEY")?,
        )
        .map_err(io::Error::other)?,
        Jellyfin::new(
            required_env("JELLYFIN_BASE_URL")?,
            required_env("JELLYFIN_API_KEY")?,
        )
        .map_err(io::Error::other)?,
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app(catalogue)).await
}

fn required_env(name: &str) -> io::Result<String> {
    std::env::var(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("{name} must be set")))
}
