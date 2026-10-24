// SPDX-License-Identifier: MPL-2.0

//! API integration tests against an in-memory database.

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use entrylens::config::Monitoring;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

async fn setup() -> (Router, SqlitePool) {
    let pool = entrylens::db::connect_in_memory().await.unwrap();
    let state = entrylens::AppState {
        pool: pool.clone(),
        monitoring: Monitoring::default(),
    };
    (entrylens::app(state, "Nasi Nemci"), pool)
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(body) => request
            .header("content-type", "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    };
    let response = app.clone().oneshot(request.unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

/// A GET of `uri`: the status, the content type and caching headers, and the body as text.
async fn get(app: &Router, uri: &str) -> (StatusCode, String, String, String) {
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let header = |name| {
        response
            .headers()
            .get(name)
            .map_or("", |v| v.to_str().unwrap())
            .to_owned()
    };
    let (content_type, cache) = (header("content-type"), header("cache-control"));
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        content_type,
        cache,
        String::from_utf8_lossy(&bytes).into(),
    )
}

async fn create_room(app: &Router, name: &str) -> i64 {
    let (status, room) = call(
        app,
        Method::POST,
        "/api/v1/rooms",
        Some(json!({ "name": name })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    room["id"].as_i64().unwrap()
}

async fn create_doorway(app: &Router, inside: i64, outside: Option<i64>) -> i64 {
    let body = json!({ "inside_room": inside, "outside_room": outside });
    let (status, doorway) = call(app, Method::POST, "/api/v1/doorways", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    doorway["id"].as_i64().unwrap()
}

#[tokio::test]
async fn rooms_are_created_listed_renamed_and_deleted() {
    let (app, _) = setup().await;
    let b = create_room(&app, "  B ").await;
    let a = create_room(&app, "A").await;

    let (status, rooms) = call(&app, Method::GET, "/api/v1/rooms", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        rooms,
        json!([
            { "id": a, "name": "A", "threshold": null },
            { "id": b, "name": "B", "threshold": null }
        ])
    );

    let uri = format!("/api/v1/rooms/{b}");
    let body = json!({ "name": "C", "threshold": 20 });
    let (status, room) = call(&app, Method::PUT, &uri, Some(body)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(room, json!({ "id": b, "name": "C", "threshold": 20 }));

    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invalid_room_names_are_rejected() {
    let (app, _) = setup().await;
    create_room(&app, "Hall").await;
    for (name, expected) in [
        ("hall", StatusCode::CONFLICT),
        ("   ", StatusCode::BAD_REQUEST),
        (&"x".repeat(101), StatusCode::BAD_REQUEST),
    ] {
        let (status, body) = call(
            &app,
            Method::POST,
            "/api/v1/rooms",
            Some(json!({ "name": name })),
        )
        .await;
        assert_eq!(status, expected, "{name}");
        assert!(body["error"].is_string());
    }
    let (status, _) = call(&app, Method::POST, "/api/v1/rooms", Some(json!({ "x": 1 }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body = json!({ "name": "X", "threshold": 0 });
    let (status, body) = call(&app, Method::POST, "/api/v1/rooms", Some(body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "occupancy threshold must be at least 1");
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/v1/rooms/99",
        Some(json!({ "name": "X" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn doorways_are_validated_and_protect_their_rooms() {
    let (app, pool) = setup().await;
    let a = create_room(&app, "A").await;
    let b = create_room(&app, "B").await;
    let ab = create_doorway(&app, b, Some(a)).await;
    let to_a = create_doorway(&app, a, None).await;

    // Without any sensor report or passage yet.
    let status = json!({
        "has_passages": false, "fault": 0, "voltage_mv": null, "last_report": null,
        "last_passage": null, "no_passage": true, "no_report": true, "low_voltage": false
    });
    let doorway = |id: i64, inside: i64, outside: Option<i64>| {
        let mut d = json!({
            "id": id, "dev_eui": null, "inside_room": inside, "outside_room": outside
        });
        d.as_object_mut()
            .unwrap()
            .extend(status.as_object().unwrap().clone());
        d
    };
    let (_, doorways) = call(&app, Method::GET, "/api/v1/doorways", None).await;
    assert_eq!(
        doorways,
        json!([doorway(to_a, a, None), doorway(ab, b, Some(a))])
    );

    for body in [
        json!({ "inside_room": a, "outside_room": a }),
        json!({ "inside_room": 99, "outside_room": null }),
    ] {
        let (status, _) = call(&app, Method::POST, "/api/v1/doorways", Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    // The DevEUI: normalized, unique, and removed again with an empty one.
    let uri = format!("/api/v1/doorways/{to_a}");
    let body =
        json!({ "dev_eui": "ac-1f-09-ff-fe-0a-00-01", "inside_room": a, "outside_room": null });
    let (status, doorway) = call(&app, Method::PUT, &uri, Some(body)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(doorway["dev_eui"], "AC1F09FFFE0A0001");
    for (dev_eui, expected) in [
        ("AC1F09FFFE0A0001", StatusCode::CONFLICT),
        ("AC1F09FFFE0A00", StatusCode::BAD_REQUEST),
        ("AC1F09FFFE0A000G", StatusCode::BAD_REQUEST),
    ] {
        let body = json!({ "dev_eui": dev_eui, "inside_room": b, "outside_room": a });
        let (status, body) = call(
            &app,
            Method::PUT,
            &format!("/api/v1/doorways/{ab}"),
            Some(body),
        )
        .await;
        assert_eq!(status, expected, "{dev_eui}");
        assert!(body["error"].is_string());
    }
    let body = json!({ "dev_eui": "", "inside_room": a, "outside_room": null });
    let (_, doorway) = call(&app, Method::PUT, &uri, Some(body)).await;
    assert_eq!(doorway["dev_eui"], Value::Null);

    let (status, body) = call(&app, Method::DELETE, &format!("/api/v1/rooms/{a}"), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "the room is used by a doorway");

    // A doorway with passages can be neither deleted nor given other rooms.
    sqlx::query(
        "INSERT INTO passages (doorway, interval_end, inbound, outbound) VALUES (?, 0, 1, 0)",
    )
    .bind(ab)
    .execute(&pool)
    .await
    .unwrap();
    let uri = format!("/api/v1/doorways/{ab}");
    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let body = json!({ "inside_room": a, "outside_room": null });
    let (status, body) = call(&app, Method::PUT, &uri, Some(body)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body["error"],
        "passages were counted through the doorway, so its rooms cannot be changed"
    );
    let body = json!({ "inside_room": b, "outside_room": a });
    let (status, doorway) = call(&app, Method::PUT, &uri, Some(body.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(doorway["has_passages"], true);
    // Its sensor can still be replaced.
    let replaced = json!({ "dev_eui": "AC1F09FFFE0A0002", "inside_room": b, "outside_room": a });
    let (status, doorway) = call(&app, Method::PUT, &uri, Some(replaced)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(doorway["dev_eui"], "AC1F09FFFE0A0002");
    assert_eq!(doorway["last_passage"], 0);
    let (status, _) = call(&app, Method::PUT, "/api/v1/doorways/99", Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/doorways/{to_a}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn sensor_status_is_checked_against_the_limits() {
    let (app, pool) = setup().await;
    let a = create_room(&app, "A").await;
    let d = create_doorway(&app, a, None).await;
    let now = entrylens::now();
    // Defaults: 24 hours without a passage, 6 hours without a report, 4.5 V.
    for (passage_age, report_age, voltage_mv, expected) in [
        (3600, 600, 4800, [false, false, false]),
        (25 * 3600, 600, 4800, [true, false, false]),
        (3600, 7 * 3600, 4499, [false, true, true]),
    ] {
        sqlx::query("DELETE FROM passages")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO passages (doorway, interval_end, inbound, outbound) VALUES (?1, ?2, 1, 0),
                    (?1, ?3, 0, 0)",
        )
        .bind(d)
        .bind(now - passage_age)
        .bind(now - 60) // no passage in the latest interval
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE doorways SET voltage_mv = ?, last_report = ? WHERE id = ?")
            .bind(voltage_mv)
            .bind(now - report_age)
            .bind(d)
            .execute(&pool)
            .await
            .unwrap();
        let (_, doorways) = call(&app, Method::GET, "/api/v1/doorways", None).await;
        let status = &doorways[0];
        assert_eq!(status["last_passage"], now - passage_age);
        assert_eq!(
            [
                &status["no_passage"],
                &status["no_report"],
                &status["low_voltage"]
            ],
            expected.map(Value::Bool).each_ref(),
            "{passage_age} {report_age} {voltage_mv}"
        );
    }
}

#[tokio::test]
async fn occupancy_and_timelines_are_served() {
    let (app, pool) = setup().await;
    let a = create_room(&app, "A").await;
    create_doorway(&app, a, None).await;

    let (status, occupancy) = call(&app, Method::GET, "/api/v1/occupancy", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        occupancy,
        json!({
            "total": 0,
            "rooms": [{
                "id": a, "name": "A", "occupancy": 0, "entries": 0, "sensor_faults": 0,
                "crowded": false
            }],
            "sensor_faults": 0,
            "crowded_rooms": 0
        })
    );

    // A sensor reporting a fault.
    let d = create_doorway(&app, a, None).await;
    sqlx::query("UPDATE doorways SET fault = 3 WHERE id = ?")
        .bind(d)
        .execute(&pool)
        .await
        .unwrap();
    let (_, occupancy) = call(&app, Method::GET, "/api/v1/occupancy", None).await;
    assert_eq!(occupancy["sensor_faults"], 1);
    assert_eq!(occupancy["rooms"][0]["sensor_faults"], 1);

    // March 2025 in Europe/Prague: 743 hours (with a DST change), in 3-hour buckets.
    let uri = "/api/v1/timeline?start=1740783600&end=1743458400";
    let (status, timeline) = call(&app, Method::GET, uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(timeline["bucket"], 10800);
    assert_eq!(timeline["total"].as_array().unwrap().len(), 248);
    assert_eq!(
        timeline["rooms"][0]["occupancy"].as_array().unwrap().len(),
        248
    );
    // No passages, so no daily entries.
    assert!(
        timeline["rooms"][0]["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // A second longer than 31 days and an hour: a point per day, up to 1 April 00:00.
    let uri = "/api/v1/timeline?start=1740783600&end=1743465601";
    let (status, timeline) = call(&app, Method::GET, uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(timeline["bucket"], 86400);
    assert_eq!(timeline["total"].as_array().unwrap().len(), 32);

    for uri in [
        // 3660 days, an hour and a second.
        "/api/v1/timeline?start=1740783600&end=2057011201",
        "/api/v1/timeline?start=1740783600&end=1740783600",
        "/api/v1/timeline?start=x&end=1740783600",
        "/api/v1/timeline?start=1740783600",
    ] {
        let (status, body) = call(&app, Method::GET, uri, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert!(body["error"].is_string(), "{uri}");
    }
}

#[tokio::test]
async fn health_and_unknown_paths() {
    let (app, _) = setup().await;
    let (status, _) = call(&app, Method::GET, "/livez", None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&app, Method::GET, "/readyz", None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = call(&app, Method::GET, "/api/v1/nothing", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not found");
    let (status, document) = call(&app, Method::GET, "/api/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(document["paths"]["/api/v1/timeline"].is_object());
    let (status, _) = call(&app, Method::DELETE, "/api/v1/doorways/99", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Needs the built frontend (`make test` builds it).
#[tokio::test]
async fn frontend_is_served_with_the_site_name() {
    let (app, _) = setup().await;
    // Any path outside the API is the frontend's index.html, so that its routes work on reload.
    for uri in ["/", "/admin?tab=rooms"] {
        let (status, content_type, cache, html) = get(&app, uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}: is the frontend built?");
        assert!(content_type.starts_with("text/html"), "{uri}");
        assert_eq!(cache, "no-cache", "{uri}");
        assert!(html.contains("<title>Nasi Nemci</title>"), "{uri}");
        let version = format!("content=\"{}\"", env!("CARGO_PKG_VERSION"));
        assert!(html.contains(&version), "{uri}: the server's version");
    }
    // Its content-hashed assets are cached for good.
    let (_, _, _, html) = get(&app, "/").await;
    let script = html
        .split('"')
        .find(|s| s.starts_with("/assets/") && s.ends_with(".js"));
    let (status, content_type, cache, _) = get(&app, script.unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.contains("javascript"));
    assert_eq!(cache, "public, max-age=31536000, immutable");
}

#[tokio::test]
async fn database_errors_are_internal_errors() {
    let (app, pool) = setup().await;
    pool.close().await;
    let (status, body) = call(&app, Method::GET, "/api/v1/rooms", None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["error"], "internal error");
    let (status, _) = call(&app, Method::GET, "/readyz", None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn database_file_is_created_and_migrated() {
    let path = std::env::temp_dir().join(format!("entrylens-test-{}.db", std::process::id()));
    let pool = entrylens::db::connect(&path).await.unwrap();
    let state = entrylens::AppState {
        pool: pool.clone(),
        monitoring: Monitoring::default(),
    };
    let app = entrylens::app(state, "Nasi Nemci");
    create_room(&app, "A").await;
    pool.close().await;
    // Opened again, the database keeps its data and needs no migration.
    let pool = entrylens::db::connect(&path).await.unwrap();
    let rooms = entrylens::db::list_rooms(&pool).await.unwrap();
    assert_eq!(rooms.len(), 1);
    pool.close().await;
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
}

/// The documented longest timeline matches the code (the doc comment cannot name the constant).
#[test]
fn openapi_document_states_the_longest_timeline() {
    use utoipa::OpenApi;
    let doc = serde_json::to_string(&entrylens::api::ApiDoc::openapi()).unwrap();
    let limit = format!("at most {} days", entrylens::timelines::MAX_DAILY_DAYS);
    assert!(
        doc.contains(&limit),
        "the IntervalQuery documentation must say {limit}"
    );
}

/// The committed `openapi.json`, from which the frontend types are generated, must match the
/// code. Run with `UPDATE_OPENAPI=1` to rewrite it.
#[test]
fn openapi_document_is_up_to_date() {
    use utoipa::OpenApi;
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");
    let current = entrylens::api::ApiDoc::openapi().to_pretty_json().unwrap() + "\n";
    if std::env::var_os("UPDATE_OPENAPI").is_some() {
        std::fs::write(path, &current).unwrap();
    }
    let committed = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        committed == current,
        "openapi.json is out of date; run `make api`"
    );
}
