CREATE TABLE rooms (
    id        INTEGER PRIMARY KEY,
    name      TEXT    NOT NULL UNIQUE COLLATE NOCASE,  -- trimmed, 1-100 characters
    threshold INTEGER NULL CHECK (threshold > 0)       -- crowded above it; NULL = none
) STRICT;

CREATE TABLE doorways (
    id           INTEGER PRIMARY KEY,
    dev_eui      TEXT    NULL UNIQUE,  -- the sensor's, 16 upper-case hex digits; NULL = none
    inside_room  INTEGER NOT NULL REFERENCES rooms(id),
    outside_room INTEGER NULL     REFERENCES rooms(id),  -- NULL = outside corridor
    fault        INTEGER NOT NULL DEFAULT 0,  -- last fault code from the sensor; 0 = none
    voltage_mv   INTEGER NULL,  -- last reported input voltage, mV; NULL before the first report
    last_report  INTEGER NULL,  -- Unix time of the last report; NULL before the first report
    CHECK (length(dev_eui) = 16 AND dev_eui NOT GLOB '*[^0-9A-F]*'),
    CHECK (outside_room IS NULL OR outside_room <> inside_room)
) STRICT;

CREATE TABLE passages (
    id           INTEGER PRIMARY KEY,
    doorway      INTEGER NOT NULL REFERENCES doorways(id),
    interval_end INTEGER NOT NULL,  -- Unix time, seconds
    inbound      INTEGER NOT NULL CHECK (inbound  >= 0),
    outbound     INTEGER NOT NULL CHECK (outbound >= 0),
    UNIQUE (doorway, interval_end)  -- no duplicate intervals
) STRICT;

-- Time ranges of entries.
CREATE INDEX passages_by_time ON passages (interval_end);

-- Derived data, written together with the passages; see DESIGN.md.

-- For each room, one row per passage interval in which people entered or left the room.
CREATE TABLE occupancy (
    id        INTEGER PRIMARY KEY,
    room      INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    timestamp INTEGER NOT NULL,  -- an interval_end of the passages
    count     INTEGER NOT NULL,  -- people in the room after the interval, since local midnight
    UNIQUE (room, timestamp)
) STRICT;

-- For each room and local day with passages, and for the total (room NULL): the timelines' daily
-- entries and their points per day.
CREATE TABLE daily (
    id         INTEGER PRIMARY KEY,
    room       INTEGER NULL REFERENCES rooms(id) ON DELETE CASCADE,  -- NULL = the total
    day        INTEGER NOT NULL,  -- Unix time of the local midnight starting the day
    peak       INTEGER NOT NULL,  -- highest occupancy of the day, at least 0 (that of midnight)
    entries    INTEGER NOT NULL,  -- people who entered; for the total, from the outside corridor
    last_entry INTEGER NULL       -- end of the day's last passage interval with entries
) STRICT;
CREATE UNIQUE INDEX daily_by_day ON daily (day, IFNULL(room, 0));
