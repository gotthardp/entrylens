// SPDX-License-Identifier: MPL-2.0

//! Current occupancy and timelines, against hand-written passages and derived rows. Run with
//! `TZ=Europe/Prague` (set in `.cargo/config.toml`), a time zone with DST changes.

use entrylens::db::{self, DoorwayInput};
use entrylens::timelines;
use serde_json::Value;
use sqlx::SqlitePool;

struct Layout {
    pool: SqlitePool,
    a: i64,
    b: i64,
    c: i64,
    /// Outside corridor -> A.
    to_a: i64,
    /// A -> B.
    a_b: i64,
}

/// Rooms A, B and C with occupancy thresholds 4, 3 and none, a doorway from the corridor to A and
/// one from A to B.
async fn layout() -> Layout {
    let pool = db::connect_in_memory().await.unwrap();
    let a = db::create_room(&pool, "A", Some(4)).await.unwrap().id;
    let b = db::create_room(&pool, "B", Some(3)).await.unwrap().id;
    let c = db::create_room(&pool, "C", None).await.unwrap().id;
    let doorway = |inside, outside| DoorwayInput {
        dev_eui: None,
        inside_room: inside,
        outside_room: outside,
    };
    let to_a = db::create_doorway(&pool, &doorway(a, None))
        .await
        .unwrap()
        .id;
    let a_b = db::create_doorway(&pool, &doorway(b, Some(a)))
        .await
        .unwrap()
        .id;
    Layout {
        pool,
        a,
        b,
        c,
        to_a,
        a_b,
    }
}

/// Unix time of a local time, e.g. `2026-06-10 10:05`.
async fn local(pool: &SqlitePool, time: &str) -> i64 {
    sqlx::query_scalar("SELECT CAST(strftime('%s', ?, 'utc') AS INTEGER)")
        .bind(time)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn occupancy_row(pool: &SqlitePool, room: i64, time: &str, count: i64) {
    let t = local(pool, time).await;
    sqlx::query("INSERT INTO occupancy (room, timestamp, count) VALUES (?, ?, ?)")
        .bind(room)
        .bind(t)
        .bind(count)
        .execute(pool)
        .await
        .unwrap();
}

async fn passage(pool: &SqlitePool, doorway: i64, time: &str, inbound: i64, outbound: i64) {
    let t = local(pool, time).await;
    sqlx::query(
        "INSERT INTO passages (doorway, interval_end, inbound, outbound) VALUES (?, ?, ?, ?)",
    )
    .bind(doorway)
    .bind(t)
    .bind(inbound)
    .bind(outbound)
    .execute(pool)
    .await
    .unwrap();
}

/// A row of the `daily` table; `room` `None` for the total.
async fn daily_row(
    pool: &SqlitePool,
    room: Option<i64>,
    day: &str,
    (peak, entries): (i64, i64),
    last_entry: Option<&str>,
) {
    let day = local(pool, &format!("{day} 00:00")).await;
    let last_entry = match last_entry {
        Some(t) => Some(local(pool, t).await),
        None => None,
    };
    sqlx::query("INSERT INTO daily (room, day, peak, entries, last_entry) VALUES (?, ?, ?, ?, ?)")
        .bind(room)
        .bind(day)
        .bind(peak)
        .bind(entries)
        .bind(last_entry)
        .execute(pool)
        .await
        .unwrap();
}

/// Unix times of the local midnights starting `from` and ending `to` (dates `YYYY-MM-DD`).
async fn days(pool: &SqlitePool, from: &str, to: &str) -> (i64, i64) {
    sqlx::query_as(
        "SELECT CAST(strftime('%s', ?1, 'utc') AS INTEGER),
                CAST(strftime('%s', ?2, '+1 day', 'utc') AS INTEGER)",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// The points of a room as `(local time, count)`.
async fn points(pool: &SqlitePool, json: &Value) -> Vec<(String, i64)> {
    let mut result = Vec::new();
    for point in json.as_array().unwrap() {
        let t = point[0].as_i64().unwrap();
        let time: String =
            sqlx::query_scalar("SELECT strftime('%Y-%m-%d %H:%M', ?, 'unixepoch', 'localtime')")
                .bind(t)
                .fetch_one(pool)
                .await
                .unwrap();
        result.push((time, point[1].as_i64().unwrap()));
    }
    result
}

fn expected(points: &[(&str, i64)]) -> Vec<(String, i64)> {
    points.iter().map(|(t, c)| (t.to_string(), *c)).collect()
}

#[tokio::test]
async fn current_occupancy_is_the_latest_count_since_midnight_with_entries_today() {
    let l = layout().await;
    occupancy_row(&l.pool, l.a, "2026-06-09 18:00", 7).await; // yesterday
    occupancy_row(&l.pool, l.a, "2026-06-10 09:00", 10).await;
    occupancy_row(&l.pool, l.a, "2026-06-10 10:00", 5).await;
    occupancy_row(&l.pool, l.a, "2026-06-10 16:00", 9).await; // after now
    occupancy_row(&l.pool, l.b, "2026-06-10 09:30", 3).await;
    occupancy_row(&l.pool, l.c, "2026-06-10 00:00", 2).await; // ends yesterday's last interval
    passage(&l.pool, l.to_a, "2026-06-09 18:00", 50, 50).await; // yesterday
    passage(&l.pool, l.to_a, "2026-06-10 00:00", 4, 0).await; // ends yesterday's last interval
    passage(&l.pool, l.to_a, "2026-06-10 09:00", 20, 9).await;
    passage(&l.pool, l.a_b, "2026-06-10 10:00", 6, 2).await; // 6 into B, 2 into A
    passage(&l.pool, l.to_a, "2026-06-10 16:00", 5, 0).await; // after now

    let now = local(&l.pool, "2026-06-10 12:00").await;
    let o = entrylens::occupancy::compute(&l.pool, now).await.unwrap();
    let rooms: Vec<_> = o
        .rooms
        .iter()
        .map(|r| (r.id, r.occupancy, r.entries))
        .collect();
    assert_eq!(rooms, vec![(l.a, 5, 22), (l.b, 3, 6), (l.c, 0, 0)]);
    assert_eq!(o.total, 8);
    // A exceeds its threshold; B only reaches it.
    let crowded: Vec<_> = o.rooms.iter().map(|r| r.crowded).collect();
    assert_eq!(crowded, vec![true, false, false]);
    assert_eq!(o.crowded_rooms, 1);
}

#[tokio::test]
async fn timeline_of_a_day_has_every_point() {
    let l = layout().await;
    occupancy_row(&l.pool, l.a, "2026-06-09 18:00", 7).await; // before the interval
    occupancy_row(&l.pool, l.a, "2026-06-10 09:00", 10).await;
    occupancy_row(&l.pool, l.a, "2026-06-10 21:00", 1).await;
    occupancy_row(&l.pool, l.a, "2026-06-11 00:00", 1).await; // the 10th's last interval
    occupancy_row(&l.pool, l.a, "2026-06-11 09:00", 6).await; // after the interval
    occupancy_row(&l.pool, l.b, "2026-06-10 09:00", 2).await;
    occupancy_row(&l.pool, l.b, "2026-06-10 10:00", 5).await;
    // The entries: 7 into A the day before; 12 into A at 09:00, 3 into B and 1 into A at 10:00.
    daily_row(
        &l.pool,
        Some(l.a),
        "2026-06-09",
        (7, 7),
        Some("2026-06-09 18:00"),
    )
    .await;
    daily_row(
        &l.pool,
        Some(l.a),
        "2026-06-10",
        (10, 13),
        Some("2026-06-10 10:00"),
    )
    .await;
    daily_row(
        &l.pool,
        Some(l.b),
        "2026-06-10",
        (5, 3),
        Some("2026-06-10 10:00"),
    )
    .await;

    let (start, end) = days(&l.pool, "2026-06-10", "2026-06-10").await;
    let now = local(&l.pool, "2026-06-12 00:00").await;
    let t = timelines::timeline(&l.pool, start, end, now).await.unwrap();
    assert_eq!(t.bucket, 0);
    let json: Value = serde_json::to_value(&t).unwrap();
    let a = &json["rooms"][0];
    assert_eq!(
        points(&l.pool, &a["occupancy"]).await,
        expected(&[
            ("2026-06-10 00:00", 0),
            ("2026-06-10 09:00", 10),
            ("2026-06-10 21:00", 1),
            ("2026-06-11 00:00", 1),
            ("2026-06-11 00:00", 0),
        ])
    );
    // The day's total, at its last entry; and the day before, so that the line reaches the start
    // of the interval.
    assert_eq!(
        points(&l.pool, &a["entries"]).await,
        expected(&[("2026-06-09 18:00", 7), ("2026-06-10 10:00", 13)])
    );
    assert_eq!(
        points(&l.pool, &json["rooms"][1]["entries"]).await,
        expected(&[("2026-06-10 10:00", 3)])
    );
    assert!(json["rooms"][2]["entries"].as_array().unwrap().is_empty());
    assert_eq!(
        points(&l.pool, &json["rooms"][2]["occupancy"]).await,
        expected(&[("2026-06-10 00:00", 0), ("2026-06-11 00:00", 0)])
    );
    assert_eq!(
        points(&l.pool, &json["total"]).await,
        expected(&[
            ("2026-06-10 00:00", 0),
            ("2026-06-10 09:00", 12),
            ("2026-06-10 10:00", 15),
            ("2026-06-10 21:00", 6),
            ("2026-06-11 00:00", 6),
            ("2026-06-11 00:00", 0),
        ])
    );

    // Today: no 0 at the coming midnight.
    let now = local(&l.pool, "2026-06-10 12:00").await;
    let t = timelines::timeline(&l.pool, start, end, now).await.unwrap();
    let json: Value = serde_json::to_value(&t).unwrap();
    assert_eq!(
        points(&l.pool, &json["rooms"][2]["occupancy"]).await,
        expected(&[("2026-06-10 00:00", 0)])
    );
}

#[tokio::test]
async fn timeline_of_longer_intervals_has_the_maximum_per_bucket() {
    let l = layout().await;
    occupancy_row(&l.pool, l.a, "2026-06-10 09:00", 10).await; // in 08:00-09:00
    occupancy_row(&l.pool, l.a, "2026-06-10 09:30", 4).await;
    occupancy_row(&l.pool, l.a, "2026-06-10 21:00", 1).await;
    occupancy_row(&l.pool, l.a, "2026-06-11 00:00", 1).await; // the 10th's last interval
    occupancy_row(&l.pool, l.a, "2026-06-11 10:00", 3).await;
    daily_row(
        &l.pool,
        Some(l.a),
        "2026-06-10",
        (10, 10),
        Some("2026-06-10 09:00"),
    )
    .await;
    daily_row(
        &l.pool,
        Some(l.a),
        "2026-06-11",
        (3, 3),
        Some("2026-06-11 10:00"),
    )
    .await;

    let (start, end) = days(&l.pool, "2026-06-10", "2026-06-16").await;
    let now = local(&l.pool, "2026-06-11 12:30").await;
    let t = timelines::timeline(&l.pool, start, end, now).await.unwrap();
    assert_eq!(t.bucket, 3600);
    let a = &t.rooms[0].occupancy;
    assert_eq!(a.len(), 7 * 24);
    assert_eq!(a[8].0, local(&l.pool, "2026-06-10 08:00").await);
    let at = |points: &[timelines::Point], buckets: &[usize]| -> Vec<Option<i64>> {
        buckets.iter().map(|&i| points[i].1).collect()
    };
    let buckets = [7, 8, 9, 10, 20, 21, 23, 24, 32, 33, 34, 36, 37];
    let some = |values: &[i64]| -> Vec<Option<i64>> {
        values.iter().copied().map(Some).chain([None]).collect()
    };
    assert_eq!(
        at(a, &buckets),
        some(&[0, 10, 10, 4, 4, 1, 1, 0, 0, 3, 3, 3])
    );
    assert_eq!(&t.total, a);
    // One point per day: the total at the day's last entry.
    assert_eq!(
        t.rooms[0].entries,
        vec![
            (local(&l.pool, "2026-06-10 09:00").await, Some(10)),
            (local(&l.pool, "2026-06-11 10:00").await, Some(3)),
        ]
    );

    // A zoomed-in window starting mid-day: buckets aligned to the hour, the counts carried in
    // from the morning.
    let from = local(&l.pool, "2026-06-10 09:40").await;
    let to = local(&l.pool, "2026-06-12 09:40").await;
    let t = timelines::timeline(&l.pool, from, to, now).await.unwrap();
    assert_eq!(t.bucket, 600);
    assert_eq!(t.rooms[0].occupancy[0], (from, Some(4)));
    assert_eq!(
        t.rooms[0].entries[0],
        (local(&l.pool, "2026-06-10 09:00").await, Some(10))
    );
    assert_eq!(
        t.rooms[0].occupancy.last().unwrap().0,
        local(&l.pool, "2026-06-12 09:30").await
    );

    for (to, bucket) in [
        ("2026-06-10", 0),
        ("2026-06-11", 600),
        ("2026-06-16", 3600),
        ("2026-06-23", 7200),
        ("2026-07-10", 10800),
        ("2026-07-11", timelines::DAY),
    ] {
        let (start, end) = days(&l.pool, "2026-06-10", to).await;
        let t = timelines::timeline(&l.pool, start, end, now).await.unwrap();
        assert_eq!(t.bucket, bucket, "{to}");
        if bucket > 0 {
            assert!(t.total.len() <= timelines::MAX_BUCKETS as usize, "{to}");
        }
    }
}

#[tokio::test]
async fn timeline_of_more_than_a_month_has_a_point_per_day() {
    let l = layout().await;
    daily_row(
        &l.pool,
        Some(l.a),
        "2026-05-31",
        (6, 5),
        Some("2026-05-31 20:00"),
    )
    .await;
    daily_row(
        &l.pool,
        Some(l.a),
        "2026-06-01",
        (10, 30),
        Some("2026-06-01 21:00"),
    )
    .await;
    daily_row(&l.pool, Some(l.a), "2026-06-03", (4, 0), None).await; // only exits
    daily_row(
        &l.pool,
        None,
        "2026-06-01",
        (12, 25),
        Some("2026-06-01 21:00"),
    )
    .await;

    let (start, end) = days(&l.pool, "2026-06-01", "2026-07-15").await;
    let now = local(&l.pool, "2026-07-10 12:00").await;
    let t = timelines::timeline(&l.pool, start, end, now).await.unwrap();
    assert_eq!(t.bucket, timelines::DAY);
    let a = &t.rooms[0].occupancy;
    assert_eq!(a.len(), 45);
    assert_eq!(a[0], (start, Some(10)));
    assert_eq!(a[1].0, local(&l.pool, "2026-06-02 00:00").await);
    let values = |points: &[timelines::Point], at: &[usize]| -> Vec<Option<i64>> {
        at.iter().map(|&i| points[i].1).collect()
    };
    // 1 and 3 June, a day without passages, 10 July (today), 11 July (after now).
    assert_eq!(
        values(a, &[0, 1, 2, 39, 40]),
        [Some(10), Some(0), Some(4), Some(0), None]
    );
    assert_eq!(values(&t.total, &[0, 1]), [Some(12), Some(0)]);
    assert_eq!(values(&t.rooms[1].occupancy, &[0]), [Some(0)]);
    // The day before the interval too, so that the line reaches its edge.
    assert_eq!(
        t.rooms[0].entries,
        vec![
            (local(&l.pool, "2026-05-31 20:00").await, Some(5)),
            (local(&l.pool, "2026-06-01 21:00").await, Some(30)),
        ]
    );
}

#[tokio::test]
async fn dst_change_days_have_23_and_25_hours() {
    let l = layout().await;
    let now = local(&l.pool, "2027-01-01 00:00").await;
    for (from, to, points) in [
        ("2026-03-29", "2026-03-30", 47 * 6),
        ("2026-10-25", "2026-10-26", 49 * 6),
    ] {
        let (start, end) = days(&l.pool, from, to).await;
        let t = timelines::timeline(&l.pool, start, end, now).await.unwrap();
        assert_eq!(t.bucket, 600, "{from}");
        assert_eq!(t.total.len(), points, "{from}");
    }
}

#[tokio::test]
async fn buckets_stay_aligned_to_local_midnight_across_a_dst_change() {
    let l = layout().await;
    let now = local(&l.pool, "2027-01-01 00:00").await;
    // 2- and 3-hour buckets, over the change to summer time (29 March) and back (25 October).
    for (from, to, bucket) in [
        ("2026-03-20", "2026-04-05", 7200),
        ("2026-03-20", "2026-04-15", 10800),
        ("2026-10-10", "2026-11-05", 10800),
    ] {
        let (start, end) = days(&l.pool, from, to).await;
        let t = timelines::timeline(&l.pool, start, end, now).await.unwrap();
        assert_eq!(t.bucket, bucket, "{from}");
        let (mut previous, mut days) = (0, 0);
        for &(t, _) in &t.total {
            let time: String = sqlx::query_scalar(
                "SELECT strftime('%Y-%m-%d %H:%M', ?, 'unixepoch', 'localtime')",
            )
            .bind(t)
            .fetch_one(&l.pool)
            .await
            .unwrap();
            // Only on the days of the change are the buckets after it an hour off.
            let hour: i64 = time[11..13].parse().unwrap();
            let change = ["2026-03-29", "2026-10-25"].contains(&&time[..10]);
            assert!(hour * 3600 % bucket == 0 || change, "{from}: {time}");
            assert_eq!(&time[14..], "00", "{from}: {time}");
            if &time[11..] == "00:00" {
                days += 1;
            }
            assert!(t > previous, "{from}: {time}");
            previous = t;
        }
        // Every day starts with a bucket at midnight.
        assert_eq!(days, (end - start + 3600) / 86400, "{from}");
    }
}
