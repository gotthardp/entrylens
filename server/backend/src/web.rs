// SPDX-License-Identifier: MPL-2.0

//! The web frontend, embedded in the binary.

use std::sync::Arc;

use axum::body::Bytes;
use axum::http::{HeaderValue, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

use crate::api::ApiError;

/// Built by `npm run build` in `frontend/`. Without it, the server still builds and serves the
/// API, but no web pages.
#[derive(RustEmbed)]
#[folder = "../frontend/dist"]
#[allow_missing = true]
struct Assets;

/// Placeholder for the site's name in `index.html`.
const TITLE: &str = "%ENTRYLENS_TITLE%";
/// Placeholder for the server's version in `index.html`.
const VERSION: &str = "%ENTRYLENS_VERSION%";

/// The frontend's `index.html` with the site's name and the server's version, prepared once.
pub struct Frontend {
    /// `None` if the frontend is not built.
    index: Option<Bytes>,
}

impl Frontend {
    /// Fills `title` (HTML-escaped) into `index.html` in place of [`TITLE`], and the server's version
    /// in place of [`VERSION`].
    pub fn new(title: &str) -> Self {
        let index = Assets::get("index.html").map(|file| {
            let html = String::from_utf8_lossy(&file.data)
                .replace(TITLE, &escape_html(title))
                .replace(VERSION, env!("CARGO_PKG_VERSION"));
            Bytes::from(html)
        });
        Self { index }
    }
}

/// Serves a frontend file, or `index.html` for any other path so that the frontend's routes
/// work on reload. Unknown paths under `/api/` get a JSON 404.
pub async fn static_file(uri: Uri, frontend: Arc<Frontend>) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path == "api" || path.starts_with("api/") {
        return ApiError::not_found("not found").into_response();
    }
    match Assets::get(path).filter(|_| !path.is_empty()) {
        Some(file) => respond(path, file.data),
        None => match &frontend.index {
            Some(index) => respond("index.html", index.clone()),
            None => ApiError::not_found("the frontend is not built").into_response(),
        },
    }
}

fn respond(path: &str, body: impl IntoResponse) -> Response {
    // Vite puts content-hashed files into `assets/`; everything else may change on upgrade.
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or(HeaderValue::from_static("application/octet-stream")),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static(cache)),
        ],
        body,
    )
        .into_response()
}

/// Escapes text for HTML content and attribute values.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    #[test]
    fn html_is_escaped() {
        assert_eq!(
            super::escape_html(r#"Tom & "Jerry's" <b>"#),
            "Tom &amp; &quot;Jerry&#39;s&quot; &lt;b&gt;"
        );
    }
}
