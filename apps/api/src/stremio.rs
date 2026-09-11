//! Browser-compatible delivery using the existing official Stremio server.
use reqwest::{Client, Url};
use std::time::Duration;

#[derive(Clone)]
pub struct Stremio {
    base_url: Url,
    reel_url: Option<Url>,
    http: Client,
}

impl Stremio {
    pub fn new(base_url: &str, reel_url: Option<&str>) -> Result<Self, crate::playback::Error> {
        let parse = |value: &str| {
            Url::parse(value)
                .ok()
                .filter(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
                .ok_or(crate::playback::Error::StremioUnavailable)
        };
        Ok(Self {
            base_url: parse(base_url)?,
            reel_url: reel_url.map(parse).transpose()?,
            http: Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .read_timeout(Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| crate::playback::Error::StremioUnavailable)?,
        })
    }

    pub fn playlist_url(
        &self,
        session_id: &str,
        source_url: &Url,
        requires_headers: bool,
    ) -> Result<Url, crate::playback::Error> {
        let mut url = self
            .base_url
            .join(&format!("/hlsv2/{session_id}/master.m3u8"))
            .map_err(|_| crate::playback::Error::StremioUnavailable)?;
        let input = if let Some(reel_url) = &self.reel_url {
            reel_url
                .join(&format!("/v1/playback/sessions/{session_id}/input"))
                .map_err(|_| crate::playback::Error::StremioUnavailable)?
        } else if requires_headers {
            return Err(crate::playback::Error::StremioInputUnavailable);
        } else {
            // This URL has already passed Reel's redirect/DNS checks. Signed
            // CDN URLs need no callback from a remote/VPN-hosted Stremio server.
            source_url.clone()
        };
        url.query_pairs_mut()
            .append_pair("mediaURL", input.as_str())
            .append_pair("videoCodecs", "h264")
            .append_pair("audioCodecs", "aac")
            .append_pair("maxAudioChannels", "2");
        Ok(url)
    }

    pub fn owns_resource(&self, url: &Url, session_id: &str) -> bool {
        url.origin() == self.base_url.origin()
            && url.path().starts_with(&format!("/hlsv2/{session_id}/"))
    }

    pub async fn media_response(
        &self,
        url: Url,
    ) -> Result<reqwest::Response, crate::playback::Error> {
        self.http
            .get(url)
            .send()
            .await
            .map_err(|_| crate::playback::Error::StremioUnavailable)?
            .error_for_status()
            .map_err(|_| crate::playback::Error::StremioUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_server_uses_validated_url_without_a_lan_callback() {
        let server = Stremio::new("http://stremio:11470", None).unwrap();
        let source = Url::parse("https://cdn.example/selected/video.mkv?token=test").unwrap();
        let playlist = server.playlist_url("session", &source, false).unwrap();
        let query = playlist
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(query["mediaURL"], source.as_str());
        assert_eq!(query["videoCodecs"], "h264");
        assert_eq!(query["audioCodecs"], "aac");
        assert!(server.owns_resource(&playlist, "session"));
        assert!(!server.owns_resource(&playlist, "another-session"));
        assert!(!server.owns_resource(&source, "session"));
        assert!(
            server.playlist_url("session", &source, true).is_err(),
            "do not silently discard required upstream request headers"
        );
    }

    #[test]
    fn callback_mode_keeps_provider_url_out_of_the_converter_request() {
        let server = Stremio::new("http://stremio:11470", Some("http://reel:3000")).unwrap();
        let source = Url::parse("https://cdn.example/video.mkv?token=test").unwrap();
        let playlist = server.playlist_url("session", &source, true).unwrap();
        let input = playlist
            .query_pairs()
            .find(|(key, _)| key == "mediaURL")
            .unwrap()
            .1;
        assert_eq!(input, "http://reel:3000/v1/playback/sessions/session/input");
        assert!(!playlist.as_str().contains("token"));
    }
}
