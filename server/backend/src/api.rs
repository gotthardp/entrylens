// SPDX-License-Identifier: MPL-2.0

//! REST API (`/api/v1`), health endpoints and the OpenAPI document.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use sqlx::error::ErrorKind;
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::AppState;
use crate::config::Monitoring;
use crate::db::{self, DoorwayInput, Room, RoomInput};
use crate::occupancy::{self, Occupancy};
use crate::sensors::DoorwayWithStatus;
use crate::timelines::{self, Timeline};

#[derive(OpenApi)]
#[openapi(
    info(title = "Entrylens", description = "Entrylens application server API"),
    paths(
        list_rooms,
        create_room,
        update_room,
        delete_room,
        list_doorways,
        create_doorway,
        update_doorway,
        delete_doorway,
        get_occupancy,
        get_timeline
    )
)]
pub struct ApiDoc;

pub fn router() -> Router<AppState> {
    let v1 = Router::new()
        .route("/rooms", get(list_rooms).post(create_room))
        .route("/rooms/{id}", put(update_room).delete(delete_room))
        .route("/doorways", get(list_doorways).post(create_doorway))
        .route("/doorways/{id}", put(update_doorway).delete(delete_doorway))
        .route("/occupancy", get(get_occupancy))
        .route("/timeline", get(get_timeline));
    Router::new()
        .nest("/api/v1", v1)
        .route("/api/openapi.json", get(openapi))
        .route("/livez", get(livez))
        .route("/readyz", get(readyz))
}

/// Error response body.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
}

/// An error answered with its status code and `{ "error": "<message>" }`.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!("database error: {e}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
    }
}

impl From<QueryRejection> for ApiError {
    fn from(e: QueryRejection) -> Self {
        Self::bad_request(e.body_text())
    }
}

impl From<JsonRejection> for ApiError {
    fn from(e: JsonRejection) -> Self {
        Self::bad_request(e.body_text())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

/// The kind of constraint a database error violated, if any.
fn violation(e: &sqlx::Error) -> Option<ErrorKind> {
    e.as_database_error().map(|d| d.kind())
}

fn room_error(e: sqlx::Error, name: &str) -> ApiError {
    match violation(&e) {
        Some(ErrorKind::UniqueViolation) => {
            ApiError::conflict(format!("a room named \"{name}\" already exists"))
        }
        _ => e.into(),
    }
}

fn doorway_error(e: sqlx::Error) -> ApiError {
    match violation(&e) {
        Some(ErrorKind::ForeignKeyViolation) => ApiError::bad_request("unknown room"),
        Some(ErrorKind::UniqueViolation) => {
            ApiError::conflict("the DevEUI is used by another doorway")
        }
        _ => e.into(),
    }
}

#[utoipa::path(get, path = "/api/v1/rooms", tag = "rooms",
    responses((status = 200, description = "Rooms sorted by name", body = [Room])))]
async fn list_rooms(State(pool): State<SqlitePool>) -> ApiResult<Json<Vec<Room>>> {
    Ok(Json(db::list_rooms(&pool).await?))
}

#[utoipa::path(post, path = "/api/v1/rooms", tag = "rooms", request_body = RoomInput,
    responses(
        (status = 201, description = "Room created", body = Room),
        (status = 400, description = "Invalid name or threshold", body = ErrorBody),
        (status = 409, description = "Duplicate name", body = ErrorBody)))]
async fn create_room(
    State(pool): State<SqlitePool>,
    input: Result<Json<RoomInput>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<Room>)> {
    let Json(input) = input?;
    let name = input.valid_name().map_err(ApiError::bad_request)?;
    let threshold = input.valid_threshold().map_err(ApiError::bad_request)?;
    let room = db::create_room(&pool, name, threshold)
        .await
        .map_err(|e| room_error(e, name))?;
    Ok((StatusCode::CREATED, Json(room)))
}

#[utoipa::path(put, path = "/api/v1/rooms/{id}", tag = "rooms", request_body = RoomInput,
    params(("id" = i64, Path, description = "Room id")),
    responses(
        (status = 200, description = "Room updated", body = Room),
        (status = 400, description = "Invalid name or threshold", body = ErrorBody),
        (status = 404, description = "Unknown room", body = ErrorBody),
        (status = 409, description = "Duplicate name", body = ErrorBody)))]
async fn update_room(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    input: Result<Json<RoomInput>, JsonRejection>,
) -> ApiResult<Json<Room>> {
    let Json(input) = input?;
    let name = input.valid_name().map_err(ApiError::bad_request)?;
    let threshold = input.valid_threshold().map_err(ApiError::bad_request)?;
    db::update_room(&pool, id, name, threshold)
        .await
        .map_err(|e| room_error(e, name))?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("unknown room"))
}

#[utoipa::path(delete, path = "/api/v1/rooms/{id}", tag = "rooms",
    params(("id" = i64, Path, description = "Room id")),
    responses(
        (status = 204, description = "Room deleted"),
        (status = 404, description = "Unknown room", body = ErrorBody),
        (status = 409, description = "Room used by a doorway", body = ErrorBody)))]
async fn delete_room(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    match db::delete_room(&pool, id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(ApiError::not_found("unknown room")),
        Err(e) if violation(&e) == Some(ErrorKind::ForeignKeyViolation) => {
            Err(ApiError::conflict("the room is used by a doorway"))
        }
        Err(e) => Err(e.into()),
    }
}

#[utoipa::path(get, path = "/api/v1/doorways", tag = "doorways",
    responses((status = 200, description = "Doorways sorted by inside room, then outside room",
        body = [DoorwayWithStatus])))]
async fn list_doorways(
    State(pool): State<SqlitePool>,
    State(limits): State<Monitoring>,
) -> ApiResult<Json<Vec<DoorwayWithStatus>>> {
    let now = crate::now();
    let doorways = db::list_doorways(&pool).await?;
    Ok(Json(
        doorways
            .into_iter()
            .map(|d| DoorwayWithStatus::new(d, &limits, now))
            .collect(),
    ))
}

#[utoipa::path(post, path = "/api/v1/doorways", tag = "doorways", request_body = DoorwayInput,
    responses(
        (status = 201, description = "Doorway created", body = DoorwayWithStatus),
        (status = 400, description = "Invalid rooms or DevEUI", body = ErrorBody),
        (status = 409, description = "DevEUI of another doorway", body = ErrorBody)))]
async fn create_doorway(
    State(pool): State<SqlitePool>,
    State(limits): State<Monitoring>,
    input: Result<Json<DoorwayInput>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<DoorwayWithStatus>)> {
    let Json(input) = input?;
    let input = input.normalized().map_err(ApiError::bad_request)?;
    let doorway = db::create_doorway(&pool, &input)
        .await
        .map_err(doorway_error)?;
    Ok((
        StatusCode::CREATED,
        Json(DoorwayWithStatus::new(doorway, &limits, crate::now())),
    ))
}

#[utoipa::path(put, path = "/api/v1/doorways/{id}", tag = "doorways", request_body = DoorwayInput,
    params(("id" = i64, Path, description = "Doorway id")),
    responses(
        (status = 200, description = "Doorway updated", body = DoorwayWithStatus),
        (status = 400, description = "Invalid rooms or DevEUI", body = ErrorBody),
        (status = 404, description = "Unknown doorway", body = ErrorBody),
        (status = 409, description = "Rooms of a doorway with passages, or DevEUI of another \
            doorway", body = ErrorBody)))]
async fn update_doorway(
    State(pool): State<SqlitePool>,
    State(limits): State<Monitoring>,
    Path(id): Path<i64>,
    input: Result<Json<DoorwayInput>, JsonRejection>,
) -> ApiResult<Json<DoorwayWithStatus>> {
    let Json(input) = input?;
    let input = input.normalized().map_err(ApiError::bad_request)?;
    if let Some(doorway) = db::update_doorway(&pool, id, &input)
        .await
        .map_err(doorway_error)?
    {
        return Ok(Json(DoorwayWithStatus::new(doorway, &limits, crate::now())));
    }
    match db::get_doorway(&pool, id).await? {
        None => Err(ApiError::not_found("unknown doorway")),
        Some(_) => Err(ApiError::conflict(
            "passages were counted through the doorway, so its rooms cannot be changed",
        )),
    }
}

#[utoipa::path(delete, path = "/api/v1/doorways/{id}", tag = "doorways",
    params(("id" = i64, Path, description = "Doorway id")),
    responses(
        (status = 204, description = "Doorway deleted"),
        (status = 404, description = "Unknown doorway", body = ErrorBody),
        (status = 409, description = "Doorway has passages", body = ErrorBody)))]
async fn delete_doorway(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    match db::delete_doorway(&pool, id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(ApiError::not_found("unknown doorway")),
        Err(e) if violation(&e) == Some(ErrorKind::ForeignKeyViolation) => {
            Err(ApiError::conflict("the doorway has passages"))
        }
        Err(e) => Err(e.into()),
    }
}

#[utoipa::path(get, path = "/api/v1/occupancy", tag = "occupancy",
    responses((status = 200, description = "Occupancy since local midnight", body = Occupancy)))]
async fn get_occupancy(State(pool): State<SqlitePool>) -> ApiResult<Json<Occupancy>> {
    Ok(Json(occupancy::compute(&pool, crate::now()).await?))
}

/// An interval of time.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct IntervalQuery {
    /// Start, Unix time in seconds.
    start: i64,
    /// End, Unix time in seconds; at most 3660 days after the start.
    end: i64,
}

#[utoipa::path(get, path = "/api/v1/timeline", tag = "timeline", params(IntervalQuery),
    responses(
        (status = 200, description = "Occupancy of each room and the total, and entries into each \
            room, since local midnight; every point for up to 25 hours, otherwise the maximum per \
            bucket, and for more than 31 days per day", body = Timeline),
        (status = 400, description = "Invalid interval", body = ErrorBody)))]
async fn get_timeline(
    State(pool): State<SqlitePool>,
    query: Result<Query<IntervalQuery>, QueryRejection>,
) -> ApiResult<Json<Timeline>> {
    let Query(IntervalQuery { start, end }) = query?;
    if end <= start {
        return Err(ApiError::bad_request("the interval ends before it starts"));
    }
    if end - start > timelines::MAX_INTERVAL {
        return Err(ApiError::bad_request(format!(
            "the interval is longer than {} days",
            timelines::MAX_DAILY_DAYS
        )));
    }
    Ok(Json(
        timelines::timeline(&pool, start, end, crate::now()).await?,
    ))
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// The process is running.
async fn livez() -> &'static str {
    "ok"
}

/// The database is open and migrated.
async fn readyz(State(pool): State<SqlitePool>) -> (StatusCode, &'static str) {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "database unavailable"),
    }
}
