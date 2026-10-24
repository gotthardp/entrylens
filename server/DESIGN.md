# Application Server

The system monitors visitor movement in an exhibition area consisting of one or more rooms
connected by doorways. Each doorway has clearly defined inside and outside sides, determined by
the door opening direction. The target installation has 22 rooms and 44 doorways.

Each doorway is equipped with a people-counting sensor that sends reports over LoRaWAN to a
ChirpStack server. The application server, located in a private LAN, receives the sensor reports,
stores the passages through each doorway, and provides a web interface for monitoring and
configuration. Every page of the web interface shows the server's version, its licence and a link
to the project's source code.


## Monitoring

Anybody in the LAN can view the Monitoring page of the web interface. The displayed values are
updated as new sensor reports arrive, without reloading the page.

### Total occupancy

The Monitoring page displays the estimated total occupancy across all rooms, which is the sum of
the occupancies of the rooms.

Next to the total, warnings are displayed: the number of doorways whose sensors report a fault
(e.g. "1 sensor fault") and the number of rooms whose occupancy exceeds their threshold (e.g. "2
crowded rooms"). When there is nothing to warn about, "No warnings" is displayed.

### Room occupancy

The Monitoring page displays a list of rooms with the estimated current occupancy of each room,
the number of people who have entered each room since the beginning of the current day, and the
number of doorways into or out of the room whose sensors report a fault. The list shows the
fullest rooms first, and can be sorted by any column.

The occupancy of each room is calculated as the difference between the cumulative number of entries
and exits since the beginning of the current day. This calculation assumes that the exhibition area
is empty at the start of each day.

The current day begins at midnight in the server's local time zone.

If counting errors make the calculated occupancy of a room negative, it is displayed as calculated,
so that they are visible.

The occupancy of rooms that exceed their threshold is marked with a warning sign.

### Occupancy history

The Monitoring page displays a graph of the occupancy of each room and of the total over time, and
of the entries into each room per day. The graph covers a chosen range of days. Ranges longer than
one month show one value per day: the day's highest occupancy and the day's entries.

To keep the graph readable, each room shows either its occupancy or its entries, not both.

### Alarms

The server raises two alarms, and sends an e-mail notification to the alarm recipients when either
of them is raised:

- The **occupancy alarm** is raised when the occupancy of any room first exceeds its threshold;
  the Monitoring page warns about the crowded rooms. It is cleared when the occupancy of all rooms
  has remained below their respective thresholds continuously for a configurable period (e.g. one
  hour).
- The **fault alarm** is raised when any sensor first reports a fault; the Monitoring page warns
  about the sensor faults. It is cleared when no sensor has reported a fault continuously for a
  configurable period.


## Configuration

Only the server Administrator can access the Configuration page of the web interface and change
the server settings. The sensors' LoRaWAN keys are managed in ChirpStack, not by the application
server.

### Rooms and doorways

The Configuration page enables the Administrator to define rooms and doorways. Each doorway
connects either two rooms or one room and an outside corridor.

Each room has the following properties:

- Name
- Occupancy threshold

Each doorway has the following properties:

- Inside room
- Outside room (empty if the doorway leads to an outside corridor)
- Sensor serial number (printed on the PCB)
- Sensor DevEUI (configured in the LoRaWAN module)

Several doorways may connect the same pair of rooms, or the same room and the outside corridor.

The rooms and doorways are defined before the counting starts. A doorway through which passages
have been counted keeps its rooms and cannot be deleted, so that the counted data stays
consistent; its sensor can still be replaced or removed, and new doorways can be added. A room
that a doorway uses cannot be deleted.

### Sensor status

The Configuration page displays the list of doorways, including their configuration and the
following sensor status information (like the list of rooms, it can be sorted by any column):

- Last reported fault code
- Last reported input voltage
- Time of the last detected passage
- Time of the last received sensor report

The Configuration page highlights doorways whose sensors meet any of the following conditions:

- The sensor reports a fault
- No passage has been detected for a configurable period
- No sensor report has been received within a configurable reporting interval
- The reported input voltage is below the lowest acceptable voltage

### Server settings

The Administrator sets up the server in its settings file, not in the web interface:

- Name of the site shown in the web interface
- Connection to the MQTT broker that receives the sensor reports
- E-mail server and the alarm recipients
- How long the occupancy must stay below the thresholds, and how long the sensors must report no
  fault, before the respective alarm is cleared
- When a sensor counts as silent: how long without a passage, and how long without a report
- Lowest acceptable input voltage of the sensors


## Architecture

The reports from the Entrylens sensors are received by the [ChirpStack](https://www.chirpstack.io/)
server, which forwards them to an MQTT broker. The application server receives them from the
broker, stores the passages in its database, and serves the web interface and the API.

A sensor report contains the number of people who have passed through the doorway in each
direction (inward and outward) since the sensor started counting, the sensor's fault code (0 when
there is no fault) and its input voltage. Sensors also report after a period without traffic. A
sensor that rebooted counts from zero again, which the server recognises by the LoRaWAN frame
counter starting again from zero; there is no dedicated reboot indicator. From consecutive
reports, the server works out the passages in each direction since the previous report and stores
them indefinitely.

### Technology

- **Backend**: Rust (edition 2024), Tokio, axum, `sqlx` with SQLite, `rumqttc` with `rustls` for
  MQTT. The built frontend is embedded in the binary (`rust-embed`).
- **Frontend**: React with TypeScript, built with Vite; Ant Design, Chart.js (with the zoom
  plugin and the Day.js adapter), Day.js, TanStack Query, React Router. English only.
- **Settings**: TOML file `/etc/entrylens.toml` (path overridable with `ENTRYLENS_CONFIG`), not the
  database; unknown keys are rejected at start-up. The site's name and the server's version are
  filled into `index.html` as the server serves it, so there is no settings endpoint.
- **Logging**: `tracing` to stdout, collected by journald; level set by `RUST_LOG`. Requests are
  logged at the debug level (`tower-http`), server errors at the error level.

### Database

SQLite, a single file in WAL mode. Times are Unix time in seconds; local days are computed in SQL
(`localtime` / `utc` modifiers), in the server's time zone. Migrations are embedded in the server,
run on start-up and never changed once released.

```sql
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
    CHECK (outside_room IS NULL OR outside_room <> inside_room)
) STRICT;

CREATE TABLE passages (
    id           INTEGER PRIMARY KEY,
    doorway      INTEGER NOT NULL REFERENCES doorways(id),
    interval_end INTEGER NOT NULL,  -- Unix time
    inbound      INTEGER NOT NULL CHECK (inbound  >= 0),
    outbound     INTEGER NOT NULL CHECK (outbound >= 0),
    UNIQUE (doorway, interval_end)  -- no duplicate intervals
) STRICT;
CREATE INDEX passages_by_time ON passages (interval_end);

-- Derived from the passages, written together with them.
CREATE TABLE occupancy (
    id        INTEGER PRIMARY KEY,
    room      INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    timestamp INTEGER NOT NULL,  -- an interval_end of the passages
    count     INTEGER NOT NULL,  -- people in the room after the interval, since local midnight
    UNIQUE (room, timestamp)
) STRICT;

-- Derived from the passages and the occupancy, written together with them.
CREATE TABLE daily (
    id         INTEGER PRIMARY KEY,
    room       INTEGER NULL REFERENCES rooms(id) ON DELETE CASCADE,  -- NULL = the total
    day        INTEGER NOT NULL,  -- Unix time of the local midnight starting the day
    peak       INTEGER NOT NULL,  -- highest occupancy of the day, at least 0 (that of midnight)
    entries    INTEGER NOT NULL,  -- people who entered; for the total, from the outside corridor
    last_entry INTEGER NULL       -- end of the day's last passage interval with entries
) STRICT;
CREATE UNIQUE INDEX daily_by_day ON daily (day, IFNULL(room, 0));
```

The sensor's serial number column of `doorways` is not designed yet.

- **`occupancy` table**: one row per room and passage interval in which people entered or left
  the room, written together with the passages (by the demo scripts or the MQTT input). Never
  rebuilt or recalculated (no rebuild code, no triggers), except the later rows of the same day
  when a passage arrives out of order.
- **`daily` table**: one row per room and local day with passages, and one for the total, for the
  daily entries of the timelines and the timelines longer than a month, which would otherwise read
  every passage of the range. Written and recalculated together with the `occupancy` rows of the
  day.
- **Fixed doorways**: the rooms of a doorway with passages cannot be changed, so the stored
  passages, occupancy and daily values always match the doorways' rooms.
- **Deletions** of a room used by a doorway, or of a doorway with passages, are refused by the
  foreign keys.
- **Room occupancy** since local midnight = `SUM inbound(inside = room) + SUM outbound(outside =
  room) - SUM outbound(inside = room) - SUM inbound(outside = room)`. Negative values are kept.
- **Current occupancy** of a room = the `count` of its latest `occupancy` row since local
  midnight, or 0.
- **Total occupancy** = the sum of the room occupancies.
- **Entries into a room** since local midnight = `SUM inbound(inside = room) + SUM outbound(outside
  = room)`, from `passages` joined with the doorways.
- **Crowded room**: current occupancy > threshold.
- **Sensor status** of a doorway, checked at each request against the `[monitoring]` settings: no
  passage (the last interval with a passage, or none) for `no_passage_hours`, no report for
  `no_report_hours` (both also when there was none yet), voltage below `min_voltage`.

### API

REST under `/api/v1`, JSON in and out. The OpenAPI document is generated from the code with
`utoipa` and served at `/api/openapi.json`; the frontend's API client is typed from it
(`openapi-typescript`, `openapi-fetch`). The same access rules apply as to the web interface.

`POST` returns `201` with the created object, `PUT` `200` with the updated object, `DELETE` `204`.
Errors: status code with `{ "error": "<message>" }` - `400` invalid input, `404` unknown id, `409`
a duplicate room name or DevEUI, a delete blocked by references, or other rooms for a doorway with
passages.

| Method & path                          | Purpose                                                |
|----------------------------------------|--------------------------------------------------------|
| `GET    /rooms`                        | List rooms                                             |
| `POST   /rooms`                        | Create room                                            |
| `PUT    /rooms/{id}`                   | Update room (name and threshold)                       |
| `DELETE /rooms/{id}`                   | Delete room (409 if used by a doorway)                 |
| `GET    /doorways`                     | List doorways, with the sensor status: last passage, `no_passage`, `no_report`, `low_voltage` |
| `POST   /doorways`                     | Create doorway                                         |
| `PUT    /doorways/{id}`                | Update doorway: DevEUI, and rooms only while it has no passages (else 409) |
| `DELETE /doorways/{id}`                | Delete doorway (409 if it has passages)                |
| `GET    /occupancy`                    | `{ total, sensor_faults, crowded_rooms, rooms: [{ id, name, occupancy, entries, sensor_faults, crowded }] }` - current occupancy and entries today; doorways whose sensor reports a fault, in all and per room; crowded rooms |
| `GET    /timeline?start=<unix>&end=<unix>` | `{ bucket, total: [[time, count]], rooms: [{ id, name, occupancy: [[time, count]], entries: [[time, count]] }] }` - see below; at most 3660 days and an hour (DST), otherwise `400` |
| `GET    /events`                       | Server-Sent Events: "changed" whenever a passage is stored or the configuration is edited |

Timeline, for all rooms at once:

- **occupancy** of each room and the **total**, restarting at 0 at each local midnight:
  - up to 25 hours (300 passage intervals), `bucket` 0: a point after each passage interval, and
    a 0 at each local midnight up to now;
  - up to 31 days and an hour: at most 300 buckets of the shortest of 10, 15, 30 minutes, 1, 2, 3
    hours that fits, stepped from each local midnight (on the day of a DST change, 2- and 3-hour
    buckets after the change start an hour off); a point at the start of each bucket with the
    highest count within it (so no peak is lost); `null` for buckets after now;
  - longer than 31 days and an hour, `bucket` 86400: a point at each local midnight with the
    day's highest count, from the `daily` table; `null` for days after today.
- **entries** per local day, from the `daily` table: one point per day with entries, at the day's
  last passage interval with entries, with the day's total; including the days before and after
  the interval, so the line reaches its edges.

Outside `/api/v1`: `GET /livez` (process running), `GET /readyz` (database migrated, MQTT
connected). Other paths under `/api/` return a JSON `404`; all other paths serve the frontend.

### Build and deployment

- One program serving the API and the frontend, one process, one port, one SQLite file; plain HTTP,
  as the server runs only in the private LAN; no Docker.
- Installed on a Linux server in the LAN, separate from ChirpStack, running Ubuntu 26.04 or newer,
  as a `.deb` built with `dpkg-deb`. A binary built against a newer glibc does not start on an
  older release, so the package is built on Ubuntu 26.04.
- The package installs the binary, a system user `entrylens`, the data directory
  `/var/lib/entrylens/` (kept when the package is removed), the settings file (a conffile, mode
  `640`, as it holds the MQTT and SMTP passwords), a hardened systemd unit and the demo scripts.
  An upgrade restarts the service, which applies new migrations.
- Backups with `sqlite3 ... ".backup <file>"`; copying the file directly is unsafe in WAL mode.
- Built and developed with the tools packaged in Ubuntu (no `rustup`, `nvm` or `pnpm`). No
  CPU-specific code or dependencies (bundled SQLite, `rustls` instead of OpenSSL), so the server
  can be built for any CPU that Linux runs on.
