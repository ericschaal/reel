use std::io;

use reel_api::{app, catalogue::Catalogue, jellyfin::Jellyfin, playback::Playback, seerr::Seerr};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let jellyfin = Jellyfin::new(
        required_env("JELLYFIN_BASE_URL")?,
        required_env("JELLYFIN_API_KEY")?,
    )
    .map_err(io::Error::other)?;
    let playback = Playback::new(jellyfin.clone(), required_env("JELLYFIN_USERNAME")?);
    let catalogue = Catalogue::new(
        Seerr::new(
            required_env("SEERR_BASE_URL")?,
            required_env("SEERR_API_KEY")?,
        )
        .map_err(io::Error::other)?,
        jellyfin,
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app(catalogue).merge(playback.router())).await
}

fn required_env(name: &str) -> io::Result<String> {
    std::env::var(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("{name} must be set")))
}
