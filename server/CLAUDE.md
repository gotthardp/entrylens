# CLAUDE.md

Guidance for working on the Entrylens application server. [DESIGN.md](DESIGN.md) describes the
system (what it does, and its architecture); [README.md](README.md) covers installation, operation
and development; the details are in the code.


## Layout

- `backend/` - Rust server: `src/`, `migrations/`, `tests/`; `openapi.json` is generated and
  committed, `dev.toml` is the development configuration.
- `frontend/` - React app: one file per page in `src/pages/`. Every server call goes through the
  hooks in `src/api/queries.ts`, which own the query keys. `src/site.ts` reads the site's name and
  version that the server fills into `index.html`.
- `demo/` - SQL scripts with the demo data; `deploy/` - the `.deb` package; CI in the repository
  root, `.github/workflows/server.yml`.


## Commands

Run from `server/`; the [README](README.md#development) and `make help` list the targets.

- Before finishing a change, run `make lint` and `make test`; after an API change, `make api` (a
  test fails if `openapi.json` is out of date).
- One backend test: `cd backend && cargo test --test calculations <name>`.
- The user often has `make dev` running; do not stop it, but say when the backend must be
  restarted (backend code or a new migration). `make demo` needs it stopped.


## Conventions

- In reviews, do not report what a later milestone of the [Plan](#plan) covers as a defect.
- Terms: a sensor reports a **fault** (a fault code, 0 = none), never an "error"; a room is
  **crowded** when its current occupancy exceeds its threshold; the pages are **Monitoring** (`/`)
  and **Configuration** (`/admin`); a doorway without an outside room leads to the **outside
  corridor**.
- Every source file and script starts with an `SPDX-License-Identifier: MPL-2.0` comment (after
  a shebang). Exempt are settings and package metadata files, which allow no comments or declare
  the licence themselves, and the migrations, whose every byte sqlx checksums.
- Released migrations are never changed (sqlx checksums them, and installed databases have applied
  them); add a new numbered migration. Unreleased ones may be squashed into one; development
  databases are then recreated with `make demo`.
- Times are Unix seconds; local days are computed in SQL. Results depend on the time zone, so the
  tests run in `Europe/Prague` (`backend/.cargo/config.toml`, `frontend/package.json`).
- Frontend: colours from the Ant Design theme tokens; red (`colorError`) marks faults and warnings
  only, so the chart palette has no red. Match the existing code's style and comment density.
- The demo data shows every feature of the web interface, including both warnings and every
  highlighted sensor status; update `demo/layout.sql` (rooms and doorways) or `demo/passages.sql`
  (passages and the sensors' reports) when a feature needs it.
- Docs: Markdown wrapped at 100 characters (tables excepted). `DESIGN.md` records only what the
  code does not show; its Monitoring and Configuration sections are informal, for users, without
  IDs; its Architecture is terse lists, tables and SQL. Code details belong in code comments.


## Testing

- Backend: the calculations against hand-written passages, `occupancy` and `daily` rows; the API
  against an in-memory database; `make coverage` shows what is covered.
- CI loads the demo data into a fresh database and fails if the derived tables differ from those
  recomputed from the passages (`demo/check.sql`).
- Frontend: `tsc`, ESLint and Prettier; unit tests for the pure logic (`range.ts`, `sort.ts`); the
  pages are tested manually against the demo data.


## Plan

Only what is still to be developed. The server works with demo data; the next milestone is **M2**,
the sensor communication. The milestone numbers are kept as they are referred to elsewhere.

Guiding principle: keep the server simple. Anything not needed for the current milestone is added
when a later milestone needs it.

### Milestones

#### M2 - Sensor communication (DESIGN.md "Architecture")
- Doorway serial number; assign each uplink to the doorway with its DevEUI.
- MQTT subscription to ChirpStack uplink events (`rumqttc` with `rustls`; an `[mqtt]` section in the
  settings file); decode the Entrylens payload (cumulative counters; a reboot shows as the LoRaWAN
  frame counter restarting at zero) and convert the cumulative counters into passages, ignoring
  duplicates. Store each report's fault code, input voltage and time as the doorway's `fault`,
  `voltage_mv` and `last_report`. `/readyz` also checks the MQTT connection.
- Descriptions of the fault codes (`FAULTS` in `frontend/src/pages/Configuration.tsx`).
- With each passage, write the next `occupancy` row of each affected room, from the room's
  previous row of the same local day, and update the room's and the total's `daily` row; for a
  passage that arrives out of order, recalculate the later `occupancy` rows and the `daily` rows
  of that day.
- Live updates: `GET /api/v1/events` (axum's built-in `Sse`); the frontend's `EventSource`
  refetches the affected queries (now the data is loaded when a page is opened or the browser tab
  is focused again).

#### M4 - Alarms (DESIGN.md "Alarms")
- Occupancy alarm: raised when a room first exceeds its threshold, cleared after all rooms stay
  below their thresholds for the configured period.
- Fault alarm: raised when a sensor first reports a fault, cleared after no sensor reports a fault
  for the configured period.
- E-mail notification when either alarm is raised. The e-mail server, the recipients and both
  periods go into an `[alarm]` section of the settings file.

#### M5 - Access control (DESIGN.md "Monitoring", "Configuration")
- Everyone in the LAN can view; only the Administrator can change the configuration, i.e. use the
  Configuration pages (`/admin`) and the API's `POST`, `PUT` and `DELETE` endpoints.

#### M6 - Deployment
- Install the `.deb` on the target Linux server in the private LAN and check it there: the service
  starts, the web interface is reachable from the LAN, and an upgrade keeps the database.

### Open Questions

For M2:

- Uplink payload format (byte layout, fPort, counter width, fault codes); decoded by a ChirpStack
  codec or by this server.
- ChirpStack/MQTT details: JSON or protobuf events, broker address, TLS, credentials, application
  ID(s).
- How long a sensor waits without traffic before it reports, and whether zero-count intervals are
  stored. If they are, keep the last passage as a `doorways` column written with the passages: the
  doorway list now finds it by walking back through the doorway's passages.
