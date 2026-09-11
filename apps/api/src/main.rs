use std::{io, path::Path};

use reel_api::{
    aiostreams::AioStreams, app, catalogue::Catalogue, jellyfin::Jellyfin, playback::Playback,
    seerr::Seerr,
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    dotenvy::from_path(manifest_dir.join(".env.local")).ok();
    dotenvy::from_path(manifest_dir.join(".env")).ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let jellyfin = Jellyfin::new(
        required_env("JELLYFIN_BASE_URL")?,
        required_env("JELLYFIN_API_KEY")?,
    )
    .map_err(io::Error::other)?;
    let mut playback = Playback::new(jellyfin.clone(), required_env("JELLYFIN_USERNAME")?)
        .with_aiostreams(
            AioStreams::new(
                required_env("AIOSTREAMS_BASE_URL")?,
                required_env("AIOSTREAMS_UUID")?,
                required_env("AIOSTREAMS_PASSWORD")?,
            )
            .map_err(io::Error::other)?,
        );
    if let Ok(url) = std::env::var("STREMIO_BASE_URL") {
        playback = playback.with_stremio(
            reel_api::stremio::Stremio::new(
                &url,
                std::env::var("REEL_STREAMING_BASE_URL").ok().as_deref(),
            )
            .map_err(io::Error::other)?,
        );
    }
    let catalogue = Catalogue::new(
        Seerr::new(
            required_env("SEERR_BASE_URL")?,
            required_env("SEERR_API_KEY")?,
        )
        .map_err(io::Error::other)?,
        jellyfin,
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!(listen.address = %listener.local_addr()?, "API server listening");

    axum::serve(listener, app(catalogue).merge(playback.router())).await
}

fn required_env(name: &str) -> io::Result<String> {
    std::env::var(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("{name} must be set")))
}
