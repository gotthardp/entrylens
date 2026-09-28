#!/usr/bin/env python3
"""
Entrylens diagnostic display.

Requires TRACKER_FRAME to be defined in tracker.h (enabled by default).
Requires: pip install paho-mqtt

Usage:
  display.py [broker [port]]          live display via MQTT
  display.py -s FILE                  render a saved JSON snapshot and exit
  display.py -w N [broker [port]]     live display; save a snapshot whenever active
                                      track count exceeds N (edge-triggered: one file
                                      per crossing, re-armed when count drops back)
"""

import json
import sys
import time
import paho.mqtt.client as mqtt

BROKER = "127.0.0.1"
PORT   = 1883
TOPIC  = "entrylens/frame"
MAX_DIST_MM = 2000

RESET = "\033[0m"
GAP   = "      "

# ---------------------------------------------------------------------------
# Track colour palettes — one per track slot, 5 brightness levels each.
# Index 0 = darkest (low height), 4 = brightest (full height).
# _TRACK_COLORS[t] is the brightest shade for track t; used for status lines.
# ---------------------------------------------------------------------------

_PALETTES = [
    # track 0: green
    ["\033[38;5;22m", "\033[38;5;28m", "\033[38;5;34m", "\033[38;5;40m", "\033[38;5;46m"],
    # track 1: blue
    ["\033[38;5;17m", "\033[38;5;18m", "\033[38;5;19m", "\033[38;5;20m", "\033[38;5;21m"],
    # track 2: magenta
    ["\033[38;5;53m", "\033[38;5;90m", "\033[38;5;127m", "\033[38;5;164m", "\033[38;5;201m"],
    # track 3: orange
    ["\033[38;5;94m", "\033[38;5;130m", "\033[38;5;166m", "\033[38;5;172m", "\033[38;5;208m"],
    # track 4: cyan
    ["\033[38;5;23m", "\033[38;5;30m", "\033[38;5;37m", "\033[38;5;44m", "\033[38;5;51m"],
    # track 5: yellow
    ["\033[38;5;58m", "\033[38;5;100m", "\033[38;5;142m", "\033[38;5;184m", "\033[38;5;226m"],
    # track 6: violet
    ["\033[38;5;54m", "\033[38;5;55m", "\033[38;5;91m", "\033[38;5;128m", "\033[38;5;129m"],
    # track 7: red
    ["\033[38;5;52m", "\033[38;5;88m", "\033[38;5;124m", "\033[38;5;160m", "\033[38;5;196m"],
]

_TRACK_COLORS = [p[4] for p in _PALETTES]

# Gray/white ramp for pixels with no track assignment (unassigned or boundary).
_GRAY_LEVELS = ["\033[38;5;240m", "\033[38;5;245m",
                "\033[38;5;250m", "\033[38;5;253m", "\033[38;5;255m"]


def _height_level(h, max_mm=800):
    """Map height (mm) to a brightness index 0..4."""
    return round(min(max(h, 0) / max_mm, 1.0) * 4)


def track_height_color(track_id, h, max_mm=800):
    """ANSI colour: hue = track, intensity = height.  Gray if no track assigned."""
    level = _height_level(h, max_mm)
    if track_id < 0:
        return _GRAY_LEVELS[level]
    return _PALETTES[track_id % len(_PALETTES)][level]


def dist_color(d, max_mm=MAX_DIST_MM):
    """ANSI 256-color: red (close) → blue (far), gray for invalid."""
    if d < 0:
        return "\033[38;5;240m"
    ratio = min(d / max_mm, 1.0)
    r = round((1 - ratio) * 5)
    b = round(ratio * 5)
    return f"\033[38;5;{16 + 36 * r + b}m"


def combined_grid_rows(heights, regions, label, max_mm=800, width=4):
    """Height-above-background grid coloured by track."""
    rows = [f"{label:<44}"]
    for row in range(8):
        parts = []
        for col in range(8):
            z   = col * 8 + row
            h   = heights[z]
            tid = regions[z]
            color = track_height_color(tid, h, max_mm)
            cell  = f"{int(h):{width}d}" if h > 0 else " " * (width - 1) + "."
            parts.append(f"{color}{cell}{RESET}")
        rows.append(" ".join(parts))
    return rows


def dist_grid_rows(values, label, width=4):
    """Background distance grid."""
    rows = [f"{label:<44}"]
    for row in range(8):
        parts = []
        for col in range(8):
            d    = values[col * 8 + row]
            cell = f"{d:{width}d}" if d > 0 else " " * (width - 1) + "."
            parts.append(f"{dist_color(d)}{cell}{RESET}")
        rows.append(" ".join(parts))
    return rows


def render(data):
    lines = []
    if "height0" in data and "region" in data and "bg" in data:
        h_rows = combined_grid_rows(data["height0"], data["region"],
                                    "height above bg (mm, coloured by region)")
        b_rows = dist_grid_rows(data["bg"], "background distance (mm)")
        lines = [GAP.join((h_rows[j], b_rows[j])) for j in range(9)]

    if "tracks" in data:
        lines.append("")
        for tr in data["tracks"]:
            t     = tr.get("t", 0)
            cx    = tr["cx"] / 10
            cy    = tr["cy"] / 10
            h     = tr["h"]
            lost  = tr.get("lost", 0)
            half  = "inner" if cy >= 4.0 else "outer"
            color = _TRACK_COLORS[t % len(_TRACK_COLORS)]
            extra = f"  lost={lost}" if lost > 0 else ""
            lines.append(
                f"{color}track[{t}]{RESET} ({cx:.1f},{cy:.1f}) {half}"
                f"  h={h}mm{extra}"
            )

    return lines


def mqtt_loop(broker, port, watch_n):
    """Connect to MQTT and display frames; save snapshots if watch_n is set."""
    armed = True   # edge-trigger: True = waiting for count to exceed watch_n

    def on_message(_client, _userdata, msg):
        nonlocal armed
        try:
            data  = json.loads(msg.payload)
            lines = render(data)
            print("\033[2J\033[H" + "\n".join(lines), end="", flush=True)

            if watch_n is not None:
                n_tracks = len(data.get("tracks", []))
                if n_tracks > watch_n and armed:
                    armed = False
                    fname = "snapshot_" + time.strftime("%Y%m%d_%H%M%S") + ".json"
                    with open(fname, "w") as f:
                        json.dump(data, f)
                    print(f"\nsaved {fname} ({n_tracks} tracks)", flush=True)
                elif n_tracks <= watch_n:
                    armed = True   # re-arm for the next crossing
        except Exception as e:
            print(f"\033[2J\033[Herror: {e}", flush=True)

    client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2)
    client.on_message = on_message
    client.connect(broker, port)
    client.subscribe(TOPIC)
    print(f"Connected to {broker}:{port}, watching {TOPIC} ...")
    client.loop_forever()


def main():
    args    = sys.argv[1:]
    watch_n = None

    if args and args[0] == "-s":
        with open(args[1]) as f:
            data = json.load(f)
        print("\n".join(render(data)))
        return

    if args and args[0] == "-w":
        watch_n = int(args[1])
        args    = args[2:]

    broker = args[0] if len(args) > 0 else BROKER
    port   = int(args[1]) if len(args) > 1 else PORT

    mqtt_loop(broker, port, watch_n)


if __name__ == "__main__":
    main()
