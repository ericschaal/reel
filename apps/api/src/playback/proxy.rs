use super::{
    Error, Playback,
    ids::{ResourceId, SessionId},
    session::{PlaybackSession, ProxyTarget, SessionDelivery, SessionSource},
    types::{ActivationRequest, DiscoveryRequest, DiscoveryResponse, PlaybackDescriptor},
};
use axum::{
    Json,
    body::Body,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, header},
    response::Response,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Url;
use tracing::{debug, info};

pub(super) async fn activate(
    State(playback): State<Playback>,
    request: Result<Json<ActivationRequest>, JsonRejection>,
) -> Result<Json<PlaybackDescriptor>, Error> {
    let Json(request) = request.map_err(Error::InvalidRequest)?;
    let descriptor = playback.activate(request).await?;
    info!(
        session.id = %descriptor.session_id,
        playback.source = ?descriptor.source,
        playback.delivery = ?descriptor.delivery,
        "playback session activated"
    );
    Ok(Json(descriptor))
}

pub(super) async fn discover(
    State(playback): State<Playback>,
    request: Result<Json<DiscoveryRequest>, JsonRejection>,
) -> Result<Json<DiscoveryResponse>, Error> {
    let Json(request) = request.map_err(Error::InvalidRequest)?;
    let response = playback.discover(request).await?;
    info!(
        discovery.id = %response.discovery_id,
        source_count = response.sources.len(),
        issue_count = response.issues.len(),
        "playback sources discovered"
    );
    Ok(Json(response))
}

pub(super) async fn media(
    State(playback): State<Playback>,
    Path(session_id): Path<SessionId>,
    headers: HeaderMap,
) -> Result<Response, Error> {
    let session = playback.session(&session_id).await?;
    proxy(&playback, &session_id, &session, session.output(), headers).await
}

// Optional callback input keeps required upstream headers in Reel's proxy.
pub(super) async fn input(
    State(playback): State<Playback>,
    Path(session_id): Path<SessionId>,
    headers: HeaderMap,
) -> Result<Response, Error> {
    let session = playback.session(&session_id).await?;
    if matches!(session.delivery, SessionDelivery::Original) {
        return Err(Error::InvalidResource);
    }
    proxy(
        &playback,
        &session_id,
        &session,
        ProxyTarget::Original(session.source_url.clone()),
        headers,
    )
    .await
}

pub(super) async fn resource(
    State(playback): State<Playback>,
    Path((session_id, resource)): Path<(SessionId, String)>,
    headers: HeaderMap,
) -> Result<Response, Error> {
    let session = playback.session(&session_id).await?;
    let target = match &session.source {
        SessionSource::Jellyfin { item_id } => {
            let decoded = URL_SAFE_NO_PAD
                .decode(resource)
                .map_err(|_| Error::InvalidResource)?;
            let url =
                Url::parse(std::str::from_utf8(&decoded).map_err(|_| Error::InvalidResource)?)
                    .map_err(|_| Error::InvalidResource)?;
            if !playback.jellyfin.has_same_origin(&url)
                || !is_item_video_url(&url, item_id.as_str())
            {
                return Err(Error::InvalidResource);
            }
            ProxyTarget::Original(url)
        }
        SessionSource::AioStreams { .. } => session
            .resources
            .read()
            .await
            .get(&ResourceId::from_wire(resource))
            .cloned()
            .ok_or(Error::InvalidResource)?,
    };
    proxy(&playback, &session_id, &session, target, headers).await
}

async fn proxy(
    playback: &Playback,
    session_id: &SessionId,
    session: &PlaybackSession,
    target: ProxyTarget,
    request_headers: HeaderMap,
) -> Result<Response, Error> {
    let range = request_headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let converted = matches!(target, ProxyTarget::Converted(_));
    let url = target.url();
    let provider = target.upstream(&session.source);
    let upstream = if converted {
        let server = playback.stremio.as_ref().ok_or(Error::StremioUnavailable)?;
        if !server.owns_resource(url, session_id.as_str()) {
            return Err(Error::InvalidResource);
        }
        server.media_response(url.clone()).await?
    } else {
        match &session.source {
            SessionSource::Jellyfin { .. } => {
                playback.jellyfin.media_response(url.clone(), range).await?
            }
            SessionSource::AioStreams {
                request_headers, ..
            } => {
                playback
                    .aiostreams
                    .as_ref()
                    .ok_or(Error::AioStreamsNotConfigured)?
                    .media_response(url.clone(), range, request_headers)
                    .await?
            }
        }
    };
    let status = upstream.status();
    let upstream_headers = upstream.headers().clone();
    let upstream_url = upstream.url().clone();
    let is_playlist = upstream_url.path().to_ascii_lowercase().ends_with(".m3u8")
        || upstream_headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("mpegurl"));
    debug!(
        upstream = ?provider,
        upstream_status = status.as_u16(),
        range_requested = range.is_some(),
        is_playlist,
        "upstream media response received"
    );

    if is_playlist && status.is_success() {
        let text = upstream
            .text()
            .await
            .map_err(|source| Error::InvalidUpstreamBody {
                upstream: provider,
                source,
            })?;
        let rewritten = rewrite_playlist(
            playback,
            &text,
            &target.with_url(upstream_url),
            session_id,
            session,
        )
        .await?;
        return Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")
            .header(header::CACHE_CONTROL, "no-store")
            .body(Body::from(rewritten))
            .map_err(|_| Error::InvalidUpstreamResponse { upstream: provider });
    }

    let mut response = Response::builder().status(status);
    for name in [
        header::CONTENT_TYPE,
        header::CONTENT_LENGTH,
        header::CONTENT_RANGE,
        header::ACCEPT_RANGES,
        header::CACHE_CONTROL,
        header::ETAG,
        header::LAST_MODIFIED,
    ] {
        if name == header::CONTENT_TYPE
            && !converted
            && upstream_headers
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_none_or(|v| !v.starts_with("video/") && !v.contains("mpegurl"))
            && let SessionSource::AioStreams {
                content_type: Some(content_type),
                ..
            } = &session.source
        {
            response = response.header(name, *content_type);
        } else if let Some(value) = upstream_headers.get(&name) {
            response = response.header(name, value);
        }
    }
    response
        .body(Body::from_stream(upstream.bytes_stream()))
        .map_err(|_| Error::InvalidUpstreamResponse { upstream: provider })
}

pub(super) async fn rewrite_playlist(
    playback: &Playback,
    text: &str,
    base: &ProxyTarget,
    session_id: &SessionId,
    session: &PlaybackSession,
) -> Result<String, Error> {
    let mut output = String::with_capacity(text.len() + 256);
    for line in text.lines() {
        let rewritten = if line.starts_with('#') {
            rewrite_uri_attributes(playback, line, base, session_id, session).await?
        } else if line.trim().is_empty() {
            String::new()
        } else {
            register_resource_url(
                playback,
                base.with_url(base.url().join(line).map_err(|_| Error::InvalidResource)?),
                session_id,
                session,
            )
            .await?
        };
        output.push_str(&rewritten);
        output.push('\n');
    }
    Ok(output)
}

async fn rewrite_uri_attributes(
    playback: &Playback,
    line: &str,
    base: &ProxyTarget,
    session_id: &SessionId,
    session: &PlaybackSession,
) -> Result<String, Error> {
    let mut output = line.to_owned();
    let mut search_from = 0;
    while let Some(relative_start) = output[search_from..].find("URI=\"") {
        let value_start = search_from + relative_start + 5;
        let Some(relative_end) = output[value_start..].find('"') else {
            return Err(Error::InvalidUpstreamResponse {
                upstream: base.upstream(&session.source),
            });
        };
        let value_end = value_start + relative_end;
        let resolved = base
            .url()
            .join(&output[value_start..value_end])
            .map_err(|_| Error::InvalidResource)?;
        let replacement =
            register_resource_url(playback, base.with_url(resolved), session_id, session).await?;
        output.replace_range(value_start..value_end, &replacement);
        search_from = value_start + replacement.len() + 1;
    }
    Ok(output)
}

async fn register_resource_url(
    playback: &Playback,
    target: ProxyTarget,
    session_id: &SessionId,
    session: &PlaybackSession,
) -> Result<String, Error> {
    let mut url = target.url().clone();
    match &session.source {
        SessionSource::Jellyfin { item_id } => {
            if !playback.jellyfin.has_same_origin(&url)
                || !is_item_video_url(&url, item_id.as_str())
            {
                return Err(Error::InvalidResource);
            }
            url = without_jellyfin_credentials(url);
            return Ok(proxy_resource_url(url, session_id));
        }
        SessionSource::AioStreams { .. } => {
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err(Error::InvalidResource);
            }
            url.set_fragment(None);
        }
    }
    let resource_id = session
        .resources
        .write()
        .await
        .register(target.with_url(url))?;
    Ok(session_id.resource_path(resource_id))
}

fn proxy_resource_url(url: Url, session_id: &SessionId) -> String {
    let encoded = URL_SAFE_NO_PAD.encode(url.as_str());
    session_id.resource_path(encoded)
}

pub(super) fn without_jellyfin_credentials(mut url: Url) -> Url {
    let pairs = url
        .query_pairs()
        .filter(|(name, _)| {
            let normalized = name
                .chars()
                .filter(|character| *character != '_' && *character != '-')
                .flat_map(char::to_lowercase)
                .collect::<String>();
            normalized != "apikey" && normalized != "xembytoken" && normalized != "accesstoken"
        })
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    url.set_query(None);
    if !pairs.is_empty() {
        url.query_pairs_mut().extend_pairs(pairs);
    }
    url
}

pub(super) fn is_item_video_url(url: &Url, item_id: &str) -> bool {
    let segments = url
        .path_segments()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let normalized_item_id = item_id.replace('-', "");
    segments.windows(2).any(|pair| {
        pair[0].eq_ignore_ascii_case("videos")
            && pair[1]
                .replace('-', "")
                .eq_ignore_ascii_case(&normalized_item_id)
    })
}
