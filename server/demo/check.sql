-- SPDX-License-Identifier: MPL-2.0
--
-- Recomputes the derived occupancy and daily values from the passages with the doorways' rooms
-- and lists every row that differs from the stored ones; then prints today's occupancy and entries
-- per room, for comparison with the web interface. Fails (exit code 1) if any row differs.
--
--   sqlite3 entrylens.db < check.sql

.bail on
.mode box

CREATE TEMP VIEW expected_occupancy(room, timestamp, count) AS
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
SELECT room, t,
       SUM(delta) OVER (PARTITION BY room, date(t - 1, 'unixepoch', 'localtime') ORDER BY t)
FROM step;

-- For each room and local day, and for the total (room NULL), as in passages.sql.
CREATE TEMP VIEW expected_daily(room, day, peak, entries, last_entry) AS
WITH
occupancy_day(room, day, t, count) AS (
    SELECT room,
           CAST(strftime('%s', timestamp - 1, 'unixepoch', 'localtime', 'start of day', 'utc')
                AS INTEGER),
           timestamp, count
    FROM expected_occupancy
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
SELECT p.room, p.day, p.peak, COALESCE(e.entries, 0), e.last_entry
FROM peak p LEFT JOIN entered e ON e.room IS p.room AND e.day = p.day;

CREATE TEMP TABLE difference AS
SELECT 'occupancy missing' AS problem, * FROM (
    SELECT room, timestamp, count, NULL FROM expected_occupancy
    EXCEPT SELECT room, timestamp, count, NULL FROM occupancy)
UNION ALL
SELECT 'occupancy extra', * FROM (
    SELECT room, timestamp, count, NULL FROM occupancy
    EXCEPT SELECT room, timestamp, count, NULL FROM expected_occupancy)
UNION ALL
SELECT 'daily missing', * FROM (
    SELECT room, day, peak, entries || ' ' || IFNULL(last_entry, '-') FROM expected_daily
    EXCEPT SELECT room, day, peak, entries || ' ' || IFNULL(last_entry, '-') FROM daily)
UNION ALL
SELECT 'daily extra', * FROM (
    SELECT room, day, peak, entries || ' ' || IFNULL(last_entry, '-') FROM daily
    EXCEPT SELECT room, day, peak, entries || ' ' || IFNULL(last_entry, '-') FROM expected_daily);

SELECT 'differences in the occupancy and daily tables' AS check_result, COUNT(*) AS rows
FROM difference;
SELECT * FROM difference LIMIT 20;

SELECT date('now', 'localtime') AS today;

SELECT r.name AS room,
       COALESCE((SELECT o.count FROM occupancy o
                 WHERE o.room = r.id AND o.timestamp > midnight.t
                 ORDER BY o.timestamp DESC LIMIT 1), 0) AS occupancy,
       COALESCE((SELECT SUM(CASE WHEN d.inside_room = r.id THEN p.inbound ELSE p.outbound END)
                 FROM passages p JOIN doorways d ON d.id = p.doorway
                 WHERE p.interval_end > midnight.t
                   AND (d.inside_room = r.id OR d.outside_room = r.id)), 0) AS "entries today"
FROM rooms r,
     (SELECT CAST(strftime('%s', 'now', 'localtime', 'start of day', 'utc') AS INTEGER) AS t)
         AS midnight
ORDER BY r.name;

CREATE TEMP TABLE result (
    differences INTEGER
    CONSTRAINT "the derived tables differ from the passages" CHECK (differences = 0)
);
INSERT INTO result SELECT COUNT(*) FROM difference;
