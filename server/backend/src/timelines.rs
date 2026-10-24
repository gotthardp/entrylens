// SPDX-License-Identifier: MPL-2.0

//! The timeline of the occupancy and of the entries into rooms over an interval of time.
//!
//! A passage interval belongs to the local day in which it lies: `interval_end` in `(start, end]`.
//! Local-time handling is done in SQL with SQLite's `localtime` / `utc` modifiers. The series are
//! aggregated into one JSON array each in SQL, since moving many rows out of SQLite is far more
//! expensive than computing them.

use std::collections::HashMap;

use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use utoipa::ToSchema;

/// Longest interval with points within the days, in days (plus an hour for a DST change). Longer
/// intervals have a point per day, from the `daily` table.
pub const MAX_DAYS: i64 = 31;
/// Longest interval of the timeline, in days (plus an hour for a DST change).
pub const MAX_DAILY_DAYS: i64 = 3660;
/// The `bucket` of a timeline with a point per day.
pub const DAY: i64 = 86400;
/// Longest interval with points within the days, in seconds.
pub const MAX_BUCKETED: i64 = MAX_DAYS * DAY + 3600;
/// Longest interval of the timeline, in seconds.
pub const MAX_INTERVAL: i64 = MAX_DAILY_DAYS * DAY + 3600;
/// Most points per line (a DST change may add one). Up to this many 5-minute passage intervals (25
/// hours) the points are returned as they are; longer intervals are divided into buckets of the
/// shortest of [`BUCKETS`] that keeps their number within it.
pub const MAX_BUCKETS: i64 = 300;
/// Bucket lengths, in seconds.
pub const BUCKETS: [i64; 6] = [600, 900, 1800, 3600, 7200, 10800];
/// Length of a passage interval, in seconds.
const INTERVAL: i64 = 300;

/// A `[time, count]` pair; the count is `null` for a bucket starting after now.
pub type Point = (i64, Option<i64>);

#[derive(Debug, Serialize, ToSchema)]
pub struct Timeline {
    /// Length of the buckets in seconds; 0 if the points are those of every passage interval;
    /// [`DAY`] for a point per local day (which may be 23 or 25 hours long).
    pub bucket: i64,
    /// Total occupancy (the sum of the rooms).
    #[schema(value_type = Vec<Vec<i64>>)]
    pub total: Vec<Point>,
    /// Rooms sorted by name.
    pub rooms: Vec<RoomTimeline>,
}

/// Points of a room, `[time, count]` pairs ordered by time.
#[derive(Debug, Serialize, ToSchema)]
pub struct RoomTimeline {
    pub id: i64,
    pub name: String,
    /// People in the room since local midnight (from the `occupancy` table). With `bucket` 0: a
    /// point after each passage interval, and a count of 0 at each local midnight up to now, where
    /// the count restarts. Otherwise a point at the start of each bucket (with `bucket` [`DAY`],
    /// each local midnight) with the highest count within it.
    #[schema(value_type = Vec<Vec<i64>>)]
    pub occupancy: Vec<Point>,
    /// People who entered the room on each local day of the interval (from the `daily` table):
    /// one point per day with entries, at the end of the day's last passage interval with entries,
    /// with the day's total (for today, the total so far). Also for the day before and the day
    /// after the interval, so that the line reaches its edges.
    #[schema(value_type = Vec<Vec<i64>>)]
    pub entries: Vec<Point>,
}

/// A point of a series: time, count and local day (Julian day number).
type Row = (i64, i64, i64);
/// A local midnight: Unix time and local day (Julian day number).
type Midnight = (i64, i64);
/// A row of the `daily` table: local midnight, highest occupancy, entries and last entry.
type Day = (i64, i64, i64, Option<i64>);

#[derive(FromRow)]
struct SeriesJson {
    /// A room id, or -1 for the total occupancy.
    room: i64,
    /// JSON array of `[time, count, day]`, built in SQL; `day` is the local day the passage
    /// interval ending at `time` lies in, as a Julian day number.
    json: String,
}

/// The occupancy of every room and of the total, and the daily entries into every room, from
/// `start` to `end` (Unix times), evaluated at `now`: `every_point` up to 25 hours, `bucketed`
/// up to [`MAX_DAYS`], `per_day` beyond. The occupancy includes the points before `start` of its
/// local day, so that the lines start with the right value.
pub async fn timeline(pool: &SqlitePool, start: i64, end: i64, now: i64) -> sqlx::Result<Timeline> {
    // The local midnight before `start`; and the daily entries from the day before it to the day
    // after `end`, so that their line runs to the edges of the interval.
    let (day_start, entries_start, entries_end): (i64, i64, i64) = sqlx::query_as(
        "SELECT CAST(strftime('%s', ?1, 'unixepoch', 'localtime', 'start of day', 'utc')
                     AS INTEGER),
                CAST(strftime('%s', ?1, 'unixepoch', 'localtime', 'start of day', '-1 day', 'utc')
                     AS INTEGER),
                CAST(strftime('%s', ?2 - 1, 'unixepoch', 'localtime', 'start of day', '+2 days',
                              'utc') AS INTEGER)",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;
    let rooms: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM rooms ORDER BY name, id")
        .fetch_all(pool)
        .await?;
    let days = daily_rows(pool, entries_start, entries_end).await?;
    let midnights = midnights(pool, day_start, end).await?;

    // The occupancy line of a room (-1 for the total).
    let (bucket, occupancy): (i64, Box<dyn Fn(i64) -> Vec<Point> + '_>) =
        if end - start > MAX_BUCKETED {
            let line = |room| per_day(rows_of(&days, room), &midnights, end, now);
            (DAY, Box::new(line))
        } else {
            let series = occupancy_rows(pool, day_start, end).await?;
            let length = end - start;
            if length <= MAX_BUCKETS * INTERVAL {
                let line = move |room| every_point(rows_of(&series, room), &midnights, end, now);
                (0, Box::new(line))
            } else {
                let bucket = BUCKETS
                    .into_iter()
                    .find(|b| (length + b - 1) / b <= MAX_BUCKETS)
                    .unwrap_or(BUCKETS[BUCKETS.len() - 1]);
                let starts = bucket_starts(&midnights, start, end, bucket);
                let line = move |room| bucketed(rows_of(&series, room), &starts, bucket, now);
                (bucket, Box::new(line))
            }
        };
    Ok(Timeline {
        bucket,
        total: occupancy(-1),
        rooms: rooms
            .into_iter()
            .map(|(id, name)| RoomTimeline {
                id,
                occupancy: occupancy(id),
                entries: entries(rows_of(&days, id)),
                name,
            })
            .collect(),
    })
}

/// The rows of a room (-1 for the total), none if it has none.
fn rows_of<T>(series: &HashMap<i64, Vec<T>>, room: i64) -> &[T] {
    series.get(&room).map_or(&[][..], Vec::as_slice)
}

fn decode(e: serde_json::Error) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(e))
}

/// The local midnights from `from` (a local midnight) to the first one at or after `to`.
async fn midnights(pool: &SqlitePool, from: i64, to: i64) -> sqlx::Result<Vec<Midnight>> {
    let json: String = sqlx::query_scalar(
        "WITH RECURSIVE
             midnight(day, t) AS (
                 SELECT date(?1, 'unixepoch', 'localtime'), ?1
                 UNION ALL
                 SELECT date(day, '+1 day'), CAST(strftime('%s', day, '+1 day', 'utc') AS INTEGER)
                 FROM midnight WHERE t < ?2
             )
         SELECT json_group_array(json_array(t, CAST(julianday(day) AS INTEGER)) ORDER BY t)
         FROM midnight",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    serde_json::from_str(&json).map_err(decode)
}

/// The occupancy rows of every room, and the total's (-1), from `from` (a local midnight) to `to`.
/// The total changes by the sum of the rooms' changes, and restarts at 0 every local midnight.
/// CROSS JOIN reads each room's occupancy rows through the (room, timestamp) index.
async fn occupancy_rows(
    pool: &SqlitePool,
    from: i64,
    to: i64,
) -> sqlx::Result<HashMap<i64, Vec<Row>>> {
    let series: Vec<SeriesJson> = sqlx::query_as(
        "WITH
             point(room, t, count, day) AS MATERIALIZED (
                 SELECT o.room, o.timestamp, o.count,
                        CAST(julianday(date(o.timestamp - 1, 'unixepoch', 'localtime')) AS INTEGER)
                 FROM rooms r CROSS JOIN occupancy o
                     ON o.room = r.id AND o.timestamp > ?1 AND o.timestamp <= ?2
             ),
             change(t, day, delta) AS (
                 SELECT t, day,
                        count - COALESCE(LAG(count) OVER (PARTITION BY room, day ORDER BY t), 0)
                 FROM point
             ),
             step(t, day, delta) AS (SELECT t, day, SUM(delta) FROM change GROUP BY t, day),
             series(room, t, count, day) AS (
                 SELECT room, t, count, day FROM point
                 UNION ALL
                 SELECT -1, t, SUM(delta) OVER (PARTITION BY day ORDER BY t), day FROM step
             )
         SELECT room, json_group_array(json_array(t, count, day) ORDER BY t) AS json
         FROM series GROUP BY room",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    series
        .into_iter()
        .map(|s| Ok((s.room, serde_json::from_str(&s.json)?)))
        .collect::<serde_json::Result<_>>()
        .map_err(decode)
}

/// The rows of the `daily` table of the days from `from` to `to` (local midnights), by room (-1
/// for the total), through the (day, room) index.
async fn daily_rows(pool: &SqlitePool, from: i64, to: i64) -> sqlx::Result<HashMap<i64, Vec<Day>>> {
    let series: Vec<(i64, String)> = sqlx::query_as(
        "SELECT IFNULL(room, -1),
                json_group_array(json_array(day, peak, entries, last_entry) ORDER BY day)
         FROM daily WHERE day >= ?1 AND day < ?2
         GROUP BY room",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    series
        .into_iter()
        .map(|(room, json)| Ok((room, serde_json::from_str(&json)?)))
        .collect::<serde_json::Result<_>>()
        .map_err(decode)
}

/// A point after each passage interval, and a 0 at each local midnight up to `end` and `now`, where
/// the count restarts. A point at exactly midnight ends the previous day, so it comes before the
/// 0; a 0 at a midnight after now would draw a line down to it from the last count.
fn every_point(rows: &[Row], midnights: &[Midnight], end: i64, now: i64) -> Vec<Point> {
    let mut points: Vec<Point> = rows.iter().map(|&(t, c, _)| (t, Some(c))).collect();
    for &(midnight, _) in midnights.iter().rev() {
        if midnight <= end && midnight <= now {
            let at = points.partition_point(|&(t, _)| t <= midnight);
            points.insert(at, (midnight, Some(0)));
        }
    }
    points
}

/// The bucket starts, with their local days, from the bucket containing `start` to `end`: from
/// each local midnight in steps of `bucket` up to the next one, so that every day starts with a
/// bucket at midnight also after a DST change (and a 23- or 25-hour day has all its buckets).
fn bucket_starts(midnights: &[Midnight], start: i64, end: i64, bucket: i64) -> Vec<Midnight> {
    let mut starts: Vec<Midnight> = midnights
        .windows(2)
        .flat_map(|w| {
            let ((midnight, day), (next, _)) = (w[0], w[1]);
            (midnight..next)
                .step_by(bucket as usize)
                .map(move |t| (t, day))
        })
        .filter(|&(t, _)| t < end)
        .collect();
    let first = starts.iter().rposition(|&(t, _)| t <= start).unwrap_or(0);
    starts.drain(..first);
    starts
}

/// A point at the start of each bucket with the highest count within it, including the count
/// carried into it from earlier the same day; `null` for a bucket starting after `now`.
fn bucketed(rows: &[Row], starts: &[Midnight], bucket: i64, now: i64) -> Vec<Point> {
    let mut rows = rows.iter().peekable();
    let (mut carried, mut carried_day) = (0, i64::MIN);
    starts
        .iter()
        .enumerate()
        .map(|(i, &(start, day))| {
            // A bucket ends where the next one starts, which differs from its length at a DST
            // change.
            let bucket_end = starts.get(i + 1).map_or(start + bucket, |&(next, _)| next);
            // Points before the bucket only carry their count into it.
            while let Some(&(_, count, row_day)) = rows.next_if(|&&(t, _, _)| t <= start) {
                (carried, carried_day) = (count, row_day);
            }
            let mut peak = if carried_day == day { carried } else { 0 };
            while let Some(&(_, count, row_day)) = rows.next_if(|&&(t, _, _)| t <= bucket_end) {
                peak = peak.max(count);
                (carried, carried_day) = (count, row_day);
            }
            (start, (start <= now).then_some(peak))
        })
        .collect()
}

/// A point at each local midnight before `end` with the day's highest occupancy from the `daily`
/// table: 0 for a day without passages, `null` for a day after `now`.
fn per_day(days: &[Day], midnights: &[Midnight], end: i64, now: i64) -> Vec<Point> {
    let peaks: HashMap<i64, i64> = days.iter().map(|&(d, peak, _, _)| (d, peak)).collect();
    midnights
        .iter()
        .filter(|&&(m, _)| m < end)
        .map(|&(m, _)| (m, (m <= now).then(|| peaks.get(&m).copied().unwrap_or(0))))
        .collect()
}

/// The daily entries into a room: one point per day with entries, the day's total at its last
/// entry.
fn entries(days: &[Day]) -> Vec<Point> {
    days.iter()
        .filter(|&&(_, _, entries, _)| entries > 0)
        .filter_map(|&(_, _, entries, last)| Some((last?, Some(entries))))
        .collect()
}
