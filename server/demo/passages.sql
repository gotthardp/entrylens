-- SPDX-License-Identifier: MPL-2.0
--
-- Demo passages for the layout from layout.sql: 5-minute intervals for the last 400 days up to
-- now. Open 06:00-22:00 every day, 300-800 visitors a day with weekly and seasonal variation.
-- Each visitor enters A (40 %), B (30 %) or C (30 %) from the hallway and stays 10-90 minutes;
-- 30 % of the visitors in A then move on to B for 10-60 minutes. Visitors in B leave to the
-- hallway, visitors in A and C leave the way they came. Everyone leaves before closing, except a
-- group of 20 that has just entered B, so that B is crowded until midnight.
--
-- It also writes the derived occupancy and daily tables, and the sensors' last reports. The
-- database must contain no other passages: run purge.sql before running it again. Every run
-- differs (random()).
--
--   sqlite3 entrylens.db < passages.sql

.bail on
BEGIN;

-- The demo doorways by name: A, B, C (from the hallway) and AB (A -> B).
CREATE TEMP TABLE demo_doorway (name TEXT PRIMARY KEY, id INTEGER NOT NULL);
INSERT INTO demo_doorway
SELECT r.name, d.id FROM doorways d JOIN rooms r ON r.id = d.inside_room
WHERE d.outside_room IS NULL AND r.name IN ('A', 'B', 'C')
UNION ALL
SELECT 'AB', d.id FROM doorways d
JOIN rooms ri ON ri.id = d.inside_room
JOIN rooms ro ON ro.id = d.outside_room
WHERE ri.name = 'B' AND ro.name = 'A';

CREATE TEMP TABLE demo_check (
    doorways INTEGER
    CONSTRAINT "the demo layout is missing, run layout.sql first" CHECK (doorways = 4)
);
INSERT INTO demo_check SELECT COUNT(*) FROM demo_doorway;

-- Random values are stored in tables before use: an expression with random() may otherwise be
-- evaluated again wherever its column is referenced.
CREATE TEMP TABLE demo_day AS
WITH RECURSIVE day(k, date) AS (
    SELECT 0, date('now', 'localtime', '-399 days')
    UNION ALL
    SELECT k + 1, date(date, '+1 day') FROM day WHERE k < 399
)
SELECT date, min(800, max(300, CAST(
    (330 + abs(random() % 120))
    * CASE strftime('%w', date) WHEN '0' THEN 1.6 WHEN '6' THEN 1.5 ELSE 1.0 END
    * CASE WHEN strftime('%m', date) IN ('07', '08') THEN 1.2
           WHEN strftime('%m', date) IN ('01', '02', '11') THEN 0.85
           ELSE 1.0 END
    AS INTEGER))) AS visitors
FROM day;

CREATE TEMP TABLE demo_visitor AS
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 800)
SELECT d.date,
       (random() & 65535) / 65536.0 AS r_room,
       (random() & 65535) / 65536.0 AS r_move,
       (random() & 65535) / 65536.0 AS r_stay1,
       (random() & 65535) / 65536.0 AS r_stay2,
       (random() & 65535) / 65536.0 AS r_entry1,
       (random() & 65535) / 65536.0 AS r_entry2
FROM demo_day d JOIN n ON n.i <= d.visitors;

-- Times in seconds after local midnight. The entry time is triangular, peaking mid-day, and
-- leaves time to stay before closing.
CREATE TEMP TABLE demo_trip AS
SELECT date, room, moves, stay1, stay2,
       21600 + CAST((r_entry1 + r_entry2) / 2 * (57600 - stay1 - stay2) AS INTEGER) AS entry
FROM (
    SELECT date, r_entry1, r_entry2, room, moves,
           600 + CAST(r_stay1 * 4800 AS INTEGER) AS stay1,
           CASE WHEN moves THEN 600 + CAST(r_stay2 * 3000 AS INTEGER) ELSE 0 END AS stay2
    FROM (
        SELECT *,
               CASE WHEN r_room < 0.4 THEN 'A' WHEN r_room < 0.7 THEN 'B' ELSE 'C' END AS room,
               r_room < 0.4 AND r_move < 0.3 AS moves
        FROM demo_visitor
    )
);

WITH event(date, at, doorway, inbound, outbound) AS (
    SELECT date, entry, (SELECT id FROM demo_doorway WHERE name = room), 1, 0 FROM demo_trip
    UNION ALL
    SELECT date, entry + stay1, (SELECT id FROM demo_doorway WHERE name = 'AB'), 1, 0
    FROM demo_trip WHERE moves
    UNION ALL
    SELECT date, entry + stay1 + stay2,
           (SELECT id FROM demo_doorway WHERE name = CASE WHEN moves THEN 'B' ELSE room END),
           0, 1
    FROM demo_trip
),
timed AS (
    -- Local wall time to Unix time, rounded up to the end of the 5-minute interval.
    SELECT doorway, inbound, outbound,
           (CAST(strftime('%s', date, printf('+%d seconds', at), 'utc') AS INTEGER) + 299)
               / 300 * 300 AS interval_end
    FROM event
)
INSERT INTO passages (doorway, interval_end, inbound, outbound)
SELECT doorway, interval_end, SUM(inbound), SUM(outbound)
FROM timed
WHERE interval_end <= unixepoch()
GROUP BY doorway, interval_end;

-- A group of 20 enters B from the hallway in the last interval before now and stays, so that B
-- exceeds its threshold (12) whenever the demo data is created, until midnight.
INSERT INTO passages (doorway, interval_end, inbound, outbound)
VALUES ((SELECT id FROM demo_doorway WHERE name = 'B'), unixepoch() / 300 * 300, 20, 0)
ON CONFLICT (doorway, interval_end) DO UPDATE SET inbound = inbound + excluded.inbound;

-- The sensors' last reports. They report also without traffic, so the last report is now, except
-- that of the doorway from C into B, silent for two days and without passages. The sensor of the
-- doorway into C reports a fault, that of the doorway into B a low input voltage.
UPDATE doorways SET voltage_mv = 4950, last_report = unixepoch() / 300 * 300;
UPDATE doorways SET last_report = unixepoch() / 300 * 300 - 2 * 86400
WHERE id NOT IN (SELECT id FROM demo_doorway);
UPDATE doorways SET fault = 1 WHERE id = (SELECT id FROM demo_doorway WHERE name = 'C');
UPDATE doorways SET voltage_mv = 4300 WHERE id = (SELECT id FROM demo_doorway WHERE name = 'B');

-- The derived occupancy, with the doorways' rooms. A passage interval belongs to the local
-- day in which it lies, i.e. of interval_end - 1 s.

-- For each room and passage interval, the occupancy after it, counted from local midnight.
WITH delta(room, t, delta) AS (
    SELECT d.inside_room, p.interval_end, p.inbound - p.outbound
    FROM passages p JOIN doorways d ON d.id = p.doorway
    UNION ALL
    SELECT d.outside_room, p.interval_end, p.outbound - p.inbound
    FROM passages p JOIN doorways d ON d.id = p.doorway
    WHERE d.outside_room IS NOT NULL
),
step(room, t, delta) AS (
    SELECT room, t, SUM(delta) FROM delta GROUP BY room, t
)
INSERT INTO occupancy (room, timestamp, count)
SELECT room, t,
       SUM(delta) OVER (PARTITION BY room, date(t - 1, 'unixepoch', 'localtime') ORDER BY t)
FROM step;

-- For each room and local day, and for the total (room NULL), the highest occupancy and the
-- entries (recomputed the same way by check.sql).
WITH
occupancy_day(room, day, t, count) AS (
    SELECT room,
           CAST(strftime('%s', timestamp - 1, 'unixepoch', 'localtime', 'start of day', 'utc')
                AS INTEGER),
           timestamp, count
    FROM occupancy
),
change(day, t, delta) AS (
    SELECT day, t, count - COALESCE(LAG(count) OVER (PARTITION BY room, day ORDER BY t), 0)
    FROM occupancy_day
),
total(day, count) AS (
    SELECT day, SUM(SUM(delta)) OVER (PARTITION BY day ORDER BY t) FROM change GROUP BY day, t
),
peak(room, day, peak) AS (
    SELECT room, day, MAX(0, MAX(count)) FROM occupancy_day GROUP BY room, day
    UNION ALL
    SELECT NULL, day, MAX(0, MAX(count)) FROM total GROUP BY day
),
entry(room, t, n) AS (
    SELECT d.inside_room, p.interval_end, p.inbound
    FROM passages p JOIN doorways d ON d.id = p.doorway
    UNION ALL
    SELECT d.outside_room, p.interval_end, p.outbound
    FROM passages p JOIN doorways d ON d.id = p.doorway
    WHERE d.outside_room IS NOT NULL
    UNION ALL
    SELECT NULL, p.interval_end, p.inbound
    FROM passages p JOIN doorways d ON d.id = p.doorway
    WHERE d.outside_room IS NULL
),
entered(room, day, entries, last_entry) AS (
    SELECT room,
           CAST(strftime('%s', t - 1, 'unixepoch', 'localtime', 'start of day', 'utc') AS INTEGER)
               AS day,
           SUM(n), MAX(CASE WHEN n > 0 THEN t END)
    FROM entry GROUP BY room, day
)
INSERT INTO daily (room, day, peak, entries, last_entry)
SELECT p.room, p.day, p.peak, COALESCE(e.entries, 0), e.last_entry
FROM peak p LEFT JOIN entered e ON e.room IS p.room AND e.day = p.day;

COMMIT;
