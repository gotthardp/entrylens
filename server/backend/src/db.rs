// SPDX-License-Identifier: MPL-2.0

//! Database connection and the room and doorway repositories.

use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{FromRow, SqlitePool};
use utoipa::ToSchema;

static MIGRATOR: Migrator = sqlx::migrate!();

/// Opens (creating if needed) the database file and applies pending migrations.
pub async fn connect(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        // Sorting for window functions and ordered aggregates stays in memory.
        .pragma("temp_store", "memory");
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Opens a migrated in-memory database, for tests. One connection, kept open for the lifetime
/// of the pool, since every connection to `:memory:` is a separate database.
pub async fn connect_in_memory() -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(options)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Room {
    pub id: i64,
    pub name: String,
    /// Occupancy above which the room is crowded; `null` for none.
    pub threshold: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RoomInput {
    /// Room name; leading and trailing spaces are removed, 1-100 characters, unique
    /// (case-insensitive).
    pub name: String,
    /// Occupancy above which the room is crowded, at least 1; `null` or missing for none.
    pub threshold: Option<i64>,
}

impl RoomInput {
    /// The name without leading and trailing spaces, if it is valid.
    pub fn valid_name(&self) -> Result<&str, &'static str> {
        let name = self.name.trim();
        match name.chars().count() {
            0 => Err("room name must not be empty"),
            1..=100 => Ok(name),
            _ => Err("room name must have at most 100 characters"),
        }
    }

    /// The threshold, if it is valid.
    pub fn valid_threshold(&self) -> Result<Option<i64>, &'static str> {
        match self.threshold {
            Some(t) if t < 1 => Err("occupancy threshold must be at least 1"),
            t => Ok(t),
        }
    }
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Doorway {
    pub id: i64,
    /// DevEUI of the sensor, 16 hexadecimal digits in upper case; `null` for none.
    pub dev_eui: Option<String>,
    pub inside_room: i64,
    /// `null` for a doorway to the outside corridor.
    pub outside_room: Option<i64>,
    /// Whether passages were counted through the doorway; it then cannot be deleted, nor can its
    /// rooms be changed.
    pub has_passages: bool,
    /// Fault code the sensor last reported; 0 = no fault.
    pub fault: i64,
    /// Input voltage the sensor last reported, in millivolts; `null` before its first report.
    pub voltage_mv: Option<i64>,
    /// When the sensor last reported (Unix time); `null` before its first report.
    pub last_report: Option<i64>,
    /// End of the last passage interval with a passage (Unix time); `null` if there is none.
    pub last_passage: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DoorwayInput {
    /// DevEUI of the sensor: 16 hexadecimal digits, optionally separated by spaces, `-` or `:`;
    /// `null`, missing or empty for none.
    #[serde(default)]
    pub dev_eui: Option<String>,
    pub inside_room: i64,
    /// `null` for a doorway to the outside corridor.
    pub outside_room: Option<i64>,
}

impl DoorwayInput {
    /// The input if it is valid, with the DevEUI as 16 upper-case digits (`None` if empty).
    pub fn normalized(mut self) -> Result<Self, &'static str> {
        if self.outside_room == Some(self.inside_room) {
            return Err("inside and outside room must differ");
        }
        if let Some(text) = &self.dev_eui {
            let digits: String = text
                .chars()
                .filter(|c| !matches!(c, ' ' | '-' | ':'))
                .map(|c| c.to_ascii_uppercase())
                .collect();
            self.dev_eui = match digits.len() {
                0 => None,
                16 if digits.chars().all(|c| c.is_ascii_hexdigit()) => Some(digits),
                _ => return Err("the DevEUI must have 16 hexadecimal digits"),
            };
        }
        Ok(self)
    }
}

pub async fn list_rooms(pool: &SqlitePool) -> sqlx::Result<Vec<Room>> {
    sqlx::query_as("SELECT id, name, threshold FROM rooms ORDER BY name, id")
        .fetch_all(pool)
        .await
}

pub async fn create_room(
    pool: &SqlitePool,
    name: &str,
    threshold: Option<i64>,
) -> sqlx::Result<Room> {
    sqlx::query_as(
        "INSERT INTO rooms (name, threshold) VALUES (?, ?) RETURNING id, name, threshold",
    )
    .bind(name)
    .bind(threshold)
    .fetch_one(pool)
    .await
}

pub async fn update_room(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    threshold: Option<i64>,
) -> sqlx::Result<Option<Room>> {
    sqlx::query_as(
        "UPDATE rooms SET name = ?, threshold = ? WHERE id = ? RETURNING id, name, threshold",
    )
    .bind(name)
    .bind(threshold)
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Returns `false` if there is no such room.
pub async fn delete_room(pool: &SqlitePool, id: i64) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM rooms WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// The last passage through the (doorway, interval_end) index, backwards from the latest.
const DOORWAY_COLUMNS: &str = "d.id, d.dev_eui, d.inside_room, d.outside_room,
     EXISTS (SELECT 1 FROM passages p WHERE p.doorway = d.id) AS has_passages, d.fault,
     d.voltage_mv, d.last_report,
     (SELECT p.interval_end FROM passages p
      WHERE p.doorway = d.id AND p.inbound + p.outbound > 0
      ORDER BY p.interval_end DESC LIMIT 1) AS last_passage";

/// Doorways sorted by inside room name, then outside room name (the outside corridor first).
pub async fn list_doorways(pool: &SqlitePool) -> sqlx::Result<Vec<Doorway>> {
    sqlx::query_as(&format!(
        "SELECT {DOORWAY_COLUMNS}
         FROM doorways d
         JOIN rooms ri ON ri.id = d.inside_room
         LEFT JOIN rooms ro ON ro.id = d.outside_room
         ORDER BY ri.name, ro.name NULLS FIRST, d.id"
    ))
    .fetch_all(pool)
    .await
}

pub async fn get_doorway(pool: &SqlitePool, id: i64) -> sqlx::Result<Option<Doorway>> {
    sqlx::query_as(&format!(
        "SELECT {DOORWAY_COLUMNS} FROM doorways d WHERE d.id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn create_doorway(pool: &SqlitePool, input: &DoorwayInput) -> sqlx::Result<Doorway> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO doorways (dev_eui, inside_room, outside_room) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(&input.dev_eui)
    .bind(input.inside_room)
    .bind(input.outside_room)
    .fetch_one(pool)
    .await?;
    get_doorway(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

/// Changes the doorway's rooms and sensor. The rooms cannot change once passages were counted
/// through it (the stored passages and occupancy belong to its rooms), but the same rooms are
/// accepted, so its sensor can still be replaced. `None` if there is no such doorway or its rooms
/// cannot change.
pub async fn update_doorway(
    pool: &SqlitePool,
    id: i64,
    input: &DoorwayInput,
) -> sqlx::Result<Option<Doorway>> {
    let updated = sqlx::query(
        "UPDATE doorways SET dev_eui = ?4, inside_room = ?1, outside_room = ?2
         WHERE id = ?3
           AND (NOT EXISTS (SELECT 1 FROM passages WHERE doorway = ?3)
                OR (inside_room = ?1 AND outside_room IS ?2))",
    )
    .bind(input.inside_room)
    .bind(input.outside_room)
    .bind(id)
    .bind(&input.dev_eui)
    .execute(pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Ok(None);
    }
    get_doorway(pool, id).await
}

/// Returns `false` if there is no such doorway.
pub async fn delete_doorway(pool: &SqlitePool, id: i64) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM doorways WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
