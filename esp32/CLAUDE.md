# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Hardware

- **Board:** Arduino Nano ESP32 (ABX00083) — ESP32-S3, 16 MB flash, 8 MB PSRAM
- **Sensor:** VL53L8CX ToF ranging sensor (I2C via `rjrp44/vl53l8cx` component)

### Pin assignments

| Signal      | GPIO      | Arduino pin | Notes |
|-------------|-----------|-------------|-------|
| LED Red     | GPIO46    | —           |       |
| LED Green   | GPIO0     | —           |       |
| LED Blue    | GPIO45    | —           |       |
| I2C SDA     | GPIO11    | A4          |       |
| I2C SCL     | GPIO12    | A5          |       |
| Sensor PWREN| GPIO4     | A3          |       |
| SPI_I2C_N   | —         | —           | Must be hardwired to GND — 47kΩ pull-down on SATEL board is insufficient |

## Project overview

- **Framework:** ESP-IDF v6.0.1
- **Environment:** `idf.py` requires the ESP-IDF virtual environment. Because `source` only affects the current shell, the activation and the command must be in the same invocation:
  ```
  bash -c 'source ~/.espressif/tools/activate_idf_v6.0.1.sh && idf.py build'
  ```
- **Build:** `idf.py build` / `idf.py flash monitor`
- **Config:** `idf.py menuconfig` — credentials and sensor settings under **Entrylens** menu
- **Credentials:** WiFi SSID/password and HTTP URL are stored in `sdkconfig` (gitignored); set via menuconfig

## Libraries

- `rjrp44/vl53l8cx` — vendor sensor driver (I2C, managed component)
- `esp-mqtt` — ESP-IDF MQTT client (`esp_mqtt_client_*`)
- `cJSON` — JSON serialisation for MQTT payloads (ESP-IDF component)
- FreeRTOS — task creation and vTaskDelete (ESP-IDF component)

## Architecture

- `main/main.c` — entry point: prints chip info, connects WiFi, starts LED blink loop
- `main/sensor.c` / `sensor.h` — VL53L8CX task: I2C init, 8×8 ranging at 15 Hz, calls tracker
- `main/tracker.c` / `tracker.h` — six-stage person tracking pipeline
- `main/tracker.md` — algorithm documentation; read this before modifying tracker.c
- `main/Kconfig.projbuild` — project-specific menuconfig options (WiFi, MQTT broker/topics, sensor max distance)
- `sdkconfig.defaults` — committed non-sensitive defaults (target chip, sensor reset polarity)
- `tools/display.py` — diagnostic display tool; see below

## MQTT topics

- `entrylens/frame` (`CONFIG_MQTT_FRAME_TOPIC`) — per-frame diagnostic data (JSON); only published when `TRACKER_FRAME` is defined
- `entrylens/persons` (`CONFIG_MQTT_TOPIC`) — per-person entry/exit events (JSON): `{"event":"enter","height":1720}`

## Diagnostic display

`#define TRACKER_FRAME` in `tracker.h` controls per-frame diagnostic output. When defined,
`tracker_process_frame` copies its internal arrays (`zone_height`, `residual`,
`attribution_map`) into a `TrackerFrame` struct, and `sensor.c` serialises it to JSON and
publishes it on `CONFIG_MQTT_FRAME_TOPIC` every frame.

`tools/display.py` subscribes to `entrylens/frame` and renders a live terminal view of
two side-by-side 8×8 grids (height above background coloured by track, background distance)
followed by one status line per active track.

```
display.py [broker [port]]         live display via MQTT
display.py -s FILE                 render a saved JSON snapshot and exit
display.py -w N [broker [port]]    live display; save snapshot_YYYYMMDD_HHMMSS.json
                                   whenever active track count exceeds N (edge-triggered)
```

## Code style

- Source files (`.c`, `.h`) must use ASCII characters only. Use `->` instead of `→`, `-` instead of `–`, and spell out section references instead of `§`.

## Coordinate convention

The VL53L8CX flat array is **row-major**: `z = row*8 + col` where `row = z/8` (slow index) and `col = z%8` (fast index). The array is **transposed** relative to the physical crossing direction: `row` maps to **X** and `col` maps to **Y** (the crossing axis). Entry/exit is detected along Y (columns), not rows. All code reading zone indices must follow this — use `row` for X coordinates and `col` for Y coordinates. `display.py` reads grids as `values[col*8 + row]` for the same reason.
