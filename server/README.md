# Entrylens Application Server

The application server stores the passages counted by Entrylens sensors and provides a web interface
with the occupancy of the rooms, entry statistics and the status of the sensors. See
[DESIGN.md](DESIGN.md) for the system and [CLAUDE.md](CLAUDE.md#plan) for the development plan.
Until the sensors' reports arrive over MQTT (milestone M2), the server runs with
[demo data](#demo-data). Licensed under the
[Mozilla Public License 2.0](https://mozilla.org/MPL/2.0/).


## Installation

On Ubuntu 26.04 or newer, from the `.deb` package (see [Building](#building)):

```sh
sudo apt install ./entrylens_0.1.0_amd64.deb
```

It installs the server `/usr/bin/entrylens` as the systemd service `entrylens` (running as the user
`entrylens`, enabled and started on installation), the configuration `/etc/entrylens.toml`, the
database directory `/var/lib/entrylens/` and the demo scripts in `/usr/share/entrylens/demo/`. The
web interface is then at `http://<server>:8080/`, without a login: the server is meant for a private
LAN.


## Configuration

`/etc/entrylens.toml` ([TOML](https://toml.io/)); all keys are optional, unknown keys stop the
server.

| Key        | Default                             | Description                                  |
|------------|-------------------------------------|----------------------------------------------|
| `listen`   | `"0.0.0.0:8080"`                    | Address and port of the web interface        |
| `database` | `"/var/lib/entrylens/entrylens.db"` | SQLite database file                         |
| `title`    | `"Entrylens"`                       | Name of the site, shown in the web interface |

The `[monitoring]` section sets when the Configuration page highlights a doorway's sensor:

| Key                | Default | Description                               |
|--------------------|---------|-------------------------------------------|
| `no_passage_hours` | `24`    | Hours without a detected passage          |
| `no_report_hours`  | `6`     | Hours without a sensor report             |
| `min_voltage`      | `4.5`   | Lowest acceptable input voltage, in volts |

`ENTRYLENS_CONFIG` names another configuration file; `RUST_LOG` (default `info`, set in the systemd
unit) the log level. Occupancy is counted from midnight in the server's time zone (`timedatectl`).


## Operation

```sh
sudo systemctl status entrylens     # state
sudo systemctl restart entrylens    # e.g. after changing the configuration
journalctl -u entrylens             # logs
```

- **Upgrade**: `sudo apt install ./entrylens_<version>_amd64.deb`; the service restarts and
  migrates the database.
- **Back up** while running: `sudo sqlite3 /var/lib/entrylens/entrylens.db ".backup <file>"`. Do not
  copy the database file while the server runs.
- **Restore**: stop the service, copy the backup to `/var/lib/entrylens/entrylens.db` (owner
  `entrylens`), start the service.

Removing the package keeps the database.


## Demo Data

The scripts in `/usr/share/entrylens/demo/` (`demo/` in the source) create three rooms, five
doorways and 400 days of passages, with sensors in every state the web interface shows; their
comments describe the details. The server creates the tables when it starts, so it must have run
once:

```sh
sudo systemctl stop entrylens
cd /usr/share/entrylens/demo
sudo -u entrylens sqlite3 /var/lib/entrylens/entrylens.db < layout.sql
sudo -u entrylens sqlite3 /var/lib/entrylens/entrylens.db < passages.sql
sudo systemctl start entrylens
```

The data ends when `passages.sql` runs, so the sensors show as silent after `no_report_hours`. To
refresh it, run `purge.sql` and then `passages.sql`. `purge.sql` also clears the demo data before
going live: the passages, the occupancy and daily values, and the sensors' reported status (it keeps
the rooms and doorways); `check.sql` verifies the stored occupancy and daily values against the
passages, and prints today's occupancy and entries per room for comparison with the web interface.


## Development

Development uses the tools packaged in Ubuntu 26.04:

```sh
sudo apt install build-essential git sqlite3 rustc cargo rustfmt rust-clippy nodejs npm
```

To run from the source tree, without installing:

```sh
make demo    # create backend/entrylens.db with demo data (the backend must be stopped)
make dev     # run the backend and the frontend development server
```

The web interface is then at `http://<this machine>:5173/` (also from the LAN), and frontend changes
show immediately; Ctrl+C stops both. Run `make demo` again to refresh the demo data. The development
configuration is `backend/dev.toml`; add e.g. `title = "Nasi Nemci"` to it to set the site's name.
`backend/.cargo/config.toml` sets the time zone to `Europe/Prague` for the backend run from the
source (`make dev`, `make run`) and its tests, unless `TZ` is set.

| Target          | Does                                                                    |
|-----------------|-------------------------------------------------------------------------|
| `make dev`      | runs the backend and the frontend development server                    |
| `make run`      | runs only the backend, with the built frontend at `127.0.0.1:8080`      |
| `make demo`     | creates `backend/entrylens.db` anew with demo data                      |
| `make test`     | runs the frontend and backend tests                                     |
| `make coverage` | measures the backend's test coverage, see below                         |
| `make lint`     | runs rustfmt, clippy, tsc, ESLint and Prettier                          |
| `make format`   | reformats the backend and the frontend                                  |
| `make api`      | regenerates `openapi.json` and the frontend's types after an API change |
| `make build`    | builds the frontend and the release binary, which embeds it             |
| `make deb`      | builds the `.deb` package                                               |

`make help` lists all targets. A backend test fails if `openapi.json` is out of date; the API is
described there, and served at `/api/openapi.json`. The frontend tests also run in `Europe/Prague`
(set in `frontend/package.json`). The frontend's pages are tested manually against the demo data.

`make coverage` prints the coverage per file and writes a report to
`backend/target/coverage/html/index.html`. It needs the LLVM tools of `rustc`'s LLVM version
(`rustc -vV`), e.g. `sudo apt install llvm-21`.


## Building

`make deb` builds the package into `backend/target/debian/entrylens_<version>_<arch>.deb`, with the
`version` of `backend/Cargo.toml`. Build it on Ubuntu 26.04 and on the CPU architecture of the
target server.

