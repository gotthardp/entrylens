// SPDX-License-Identifier: MPL-2.0

//! Entrylens application server: rooms, doorways and passages in SQLite, served over a REST API
//! together with the embedded web frontend.

pub mod api;
pub mod config;
pub mod db;
pub mod occupancy;
pub mod sensors;
pub mod timelines;
mod web;

use std::sync::Arc;

use axum::Router;
use axum::extract::FromRef;
use axum::http::Uri;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

use config::Monitoring;

/// What the request handlers need; each extracts its part with `State`.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    /// The limits of the sensor status.
    pub monitoring: Monitoring,
}

impl FromRef<AppState> for SqlitePool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Monitoring {
    fn from_ref(state: &AppState) -> Self {
        state.monitoring.clone()
    }
}

/// The complete HTTP application: REST API, health endpoints and the embedded frontend. `title`
/// is the name of the site, filled into the frontend's `index.html`.
pub fn app(state: AppState, title: &str) -> Router {
    let frontend = Arc::new(web::Frontend::new(title));
    api::router()
        .fallback(move |uri: Uri| web::static_file(uri, frontend.clone()))
        // Requests at the debug level (RUST_LOG=debug), server errors at the error level.
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Current Unix time in seconds.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}
