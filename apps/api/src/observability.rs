use std::time::Duration;

use axum::{
    Router,
    body::Body,
    extract::MatchedPath,
    http::{Request, Response},
};
use tower_http::trace::TraceLayer;
use tracing::{Span, field, info, info_span};

pub(crate) fn trace_http(router: Router) -> Router {
    router.layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &Request<Body>| {
                let route = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map_or("unmatched", MatchedPath::as_str);
                info_span!(
                    "http.request",
                    http.request.method = %request.method(),
                    http.route = route,
                    http.response.status_code = field::Empty,
                )
            })
            .on_response(
                |response: &Response<Body>, latency: Duration, span: &Span| {
                    span.record("http.response.status_code", response.status().as_u16());
                    info!(
                        parent: span,
                        duration_ms = latency.as_secs_f64() * 1_000.0,
                        "request completed"
                    );
                },
            )
            .on_failure(()),
    )
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };

    use axum::{
        Router,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt as _;
    use tracing::instrument::WithSubscriber as _;

    use super::*;

    struct LogWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for LogWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn request_trace_uses_route_template_and_omits_url_values() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let logs = Arc::clone(&output);
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(move || LogWriter(Arc::clone(&logs)))
            .finish();
        let router = trace_http(
            Router::new().route("/items/{item_id}", get(|| async { StatusCode::NO_CONTENT })),
        );

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/items/private-item?token=secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .with_subscriber(subscriber)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let logs = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("request completed"), "{logs}");
        assert!(logs.contains("/items/{item_id}"), "{logs}");
        assert!(logs.contains("204"), "{logs}");
        assert!(!logs.contains("private-item"), "{logs}");
        assert!(!logs.contains("secret"), "{logs}");
    }
}
