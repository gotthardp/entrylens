// SPDX-License-Identifier: MPL-2.0

//! Current occupancy of the rooms.

use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct Occupancy {
    /// Sum of the room occupancies.
    pub total: i64,
    /// Rooms sorted by name.
    pub rooms: Vec<RoomOccupancy>,
    /// Number of doorways whose sensor last reported a fault.
    pub sensor_faults: i64,
    /// Number of rooms whose occupancy exceeds their threshold.
    pub crowded_rooms: i64,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct RoomOccupancy {
    pub id: i64,
    pub name: String,
    /// People in the room, counted since local midnight. May be negative with counting errors.
    pub occupancy: i64,
    /// People who entered the room since local midnight, with the doorways' rooms.
    pub entries: i64,
    /// Number of doorways to or from the room whose sensor last reported a fault.
    pub sensor_faults: i64,
    /// Whether the occupancy exceeds the room's threshold.
    pub crowded: bool,
}

/// Occupancy of every room at `now` (Unix time): the `count` of the room's latest `occupancy` row
/// since the preceding local midnight, or 0 if there is none; and the room's entries since then,
/// from `passages`. Also counts the doorways whose sensor reports a fault, per room and in all,
/// and the rooms whose occupancy exceeds their threshold.
pub async fn compute(pool: &SqlitePool, now: i64) -> sqlx::Result<Occupancy> {
    // Bound as a value, so that both ends of the range use the (room, timestamp) index.
    let midnight: i64 = sqlx::query_scalar(
        "SELECT CAST(strftime('%s', ?, 'unixepoch', 'localtime', 'start of day', 'utc')
                     AS INTEGER)",
    )
    .bind(now)
    .fetch_one(pool)
    .await?;
    let rooms: Vec<RoomOccupancy> = sqlx::query_as(
        "SELECT id, name, occupancy, entries, sensor_faults,
                threshold IS NOT NULL AND occupancy > threshold AS crowded
         FROM (SELECT r.id, r.name, r.threshold,
                      COALESCE((SELECT o.count FROM occupancy o
                                WHERE o.room = r.id AND o.timestamp > ?1 AND o.timestamp <= ?2
                                ORDER BY o.timestamp DESC LIMIT 1), 0) AS occupancy,
                      COALESCE((SELECT SUM(CASE WHEN d.inside_room = r.id THEN p.inbound
                                                ELSE p.outbound END)
                                FROM passages p JOIN doorways d ON d.id = p.doorway
                                WHERE p.interval_end > ?1 AND p.interval_end <= ?2
                                  AND (d.inside_room = r.id OR d.outside_room = r.id)), 0)
                          AS entries,
                      (SELECT COUNT(*) FROM doorways d
                       WHERE d.fault <> 0 AND (d.inside_room = r.id OR d.outside_room = r.id))
                          AS sensor_faults
               FROM rooms r)
         ORDER BY name, id",
    )
    .bind(midnight)
    .bind(now)
    .fetch_all(pool)
    .await?;
    let total = rooms.iter().map(|r| r.occupancy).sum();
    let crowded_rooms = rooms.iter().filter(|r| r.crowded).count() as i64;
    let sensor_faults = sqlx::query_scalar("SELECT COUNT(*) FROM doorways WHERE fault <> 0")
        .fetch_one(pool)
        .await?;
    Ok(Occupancy {
        total,
        rooms,
        sensor_faults,
        crowded_rooms,
    })
}
