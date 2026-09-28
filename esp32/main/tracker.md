# Tracker Algorithm

This document explains the logic behind `tracker.c`.

---

## The problem

The VL53L8CX sensor produces an 8×8 grid of distance readings 15 times per second.
Each cell (zone) reports how far away the nearest surface is in millimetres.
The goal is to watch that grid and answer: *did a person just walk in or out?*

Three sub-problems must be solved:

1. **Background subtraction** — the sensor sees the static room too.  Subtract it so only
   people remain.
2. **Segmentation** — find where each person is in the height map each frame.
3. **Entry/exit logic** — decide whether a person crossed from outside to inside or vice
   versa, ignoring people who turn back.

---

## Background subtraction

### Exponential moving average (EMA)

Each zone has a running background estimate that tracks the static surface (floor or wall)
visible from above.  The update rule is:

```
bg_new = bg_old + α · (x − bg_old)
```

With α = 0.05 the time constant is τ = 1/α = 20 frames (~1.3 s at 15 Hz): the estimate
closes 63% of the gap to any new stable value within that window.

### Protecting the background from people

The EMA update is only applied when the new reading is within 100 mm of the current
estimate.  A person standing in the zone reflects from a much closer surface (~1700 mm
higher than the floor), producing a jump well above 100 mm, which is ignored.  This
prevents a stationary person from corrupting the background model.

The sensor is configured in CLOSEST order, returning up to two targets per zone.  The
background model always uses the *farthest* valid target — when a person is present, this
is the background surface peeking around them.  The height map uses the *closest* valid
target (maximum height), which is the top of the person's head when present.

### Height map

```
h = bg_mm − distance_mm
```

The sensor is ceiling-mounted, so a closer return means a taller object.  A person 1.7 m
tall in a 2.5 m room appears at roughly 800 mm distance, giving a height of ~1700 mm.  An
empty zone has h ≈ 0.

### Temporal min/max filter

After the height map is computed, each zone is filtered over two consecutive raw frames
using a morphological min/max rule:

```
filtered[z] = min(raw[n], raw[n-1])   if raw[n] < MIN_PERSON_HEIGHT (500 mm)
            = max(raw[n], raw[n-1])   otherwise
```

- **Below 500 mm** (background zone): the min of the two frames suppresses upward noise
  spikes that could be mistaken for a person.
- **At or above 500 mm** (person zone): the max of the two frames suppresses downward dips
  that could cause a detection to flicker out.

Because both inputs are raw readings, values never accumulate across frames — a high reading
lasts at most one extra frame.

---

## Segmentation

Each frame the filtered height map is processed to produce a list of **detections** — one
per person visible in the sensor field.  There are six stages.

Only pixels at or above `MIN_PERSON_HEIGHT = 500 mm` participate in the segmentation
pipeline.  Background pixels (height ≈ 0) are excluded from all stages.

### 1. Sorted processing order

All active pixels (height ≥ 500 mm) are collected and sorted in descending height order
using insertion sort.  This sorted list drives the activation order for the next step.

### 2. Union-Find connectivity (stops at saddles)

A Union-Find structure is built over the active pixels.  Pixels are activated one by one in
descending height order.  For each activated pixel the distinct component roots of its
already-active, non-saddle 8-connected neighbours are collected.

- **0 neighbours** — isolated peak; the pixel starts its own component.
- **1 neighbour** — absorbed into that component (basin growth).
- **≥ 2 neighbours** — the pixel is a *saddle*: marked, not unioned, and the saddle height
  recorded for each distinct component pair.

Because the sweep is descending, the first encounter between two basins is at their highest
connection — the correct topographic key col for prominence computation.  Saddle-marked
pixels are excluded from all subsequent neighbour scans, preventing lower-elevation
encounters from overwriting the recorded height.

Union-by-rank keeps trees flat; path-splitting is applied in `find()` for amortised
near-O(1) (inverse-Ackermann) lookup.

### 3. Component extraction

After the Union-Find is built, `find()` is called on every active pixel to obtain its
canonical component root.  For each component the following statistics are computed in one
pass:

- **component_size** — number of pixels.
- **component_max_height** — maximum height (mm) in the component.

Components with `component_size < MIN_COMPONENT_SIZE = 3` are discarded (small noise
speckles).

### 4. Prominence filtering and batch merge

Prominence measures how independently a component stands above the surrounding terrain.
For each canonical component pair (C, D) whose saddle height was recorded in step 2:

```
prominence = min(max_height[C], max_height[D]) − saddle_height[C][D]
```

If `prominence < MIN_PROMINENCE = 150 mm`, the pair is added to the **merge list**.  Low
prominence means the lower of the two peaks barely clears the saddle — the terrain looks
like a single bumpy ridge rather than two distinct heads, so the components are likely one
person split by sensor noise.

All merge-list pairs are then applied as a single batch of Union-Find union operations.
After all unions, component statistics are rebuilt from scratch.

### 5. Watershed segmentation

Pixels belonging to the surviving valid components are pre-assigned as *seeds*, each
carrying its component root as the region ID.  The remaining active pixels (those from
discarded small components, saddle pixels, or near component boundaries) are then labelled
in a single descending-height sweep:

- **Single region** — all assigned neighbours share one region; pixel joins it.
- **Multiple regions** — assigned neighbours belong to different regions; pixel becomes `REGION_BOUNDARY`.
- **No assigned neighbours** — pixel stays `REGION_UNASSIGNED`.

Saddle pixels from step 2 enter the sweep as unassigned.  Because they sit between two
valid regions, they almost always acquire neighbours from both sides and are marked
`REGION_BOUNDARY` — making them visible as boundary cells in the diagnostic display.

### 6. Region postprocessing

For each final region, compute a height-weighted centroid `(cx, cy)` and the peak height.
These values feed the tracker as detections.

```
cx = Σ h(z) · row(z)  /  Σ h(z)      (row = X axis)
cy = Σ h(z) · col(z)  /  Σ h(z)      (col = Y axis, the crossing axis)
```

---

## Track matching

### Nearest-neighbour assignment

All (detection, track) pairs with Euclidean distance below 3 zones (`MAX_MATCH_DIST_SQ = 9`
in squared units) are collected and sorted by distance.  Pairs are then assigned greedily
from closest to farthest: each detection and each track can be matched at most once.

At 15 Hz and typical walking speeds (≤ 1.5 zones/frame ≈ 3 m/s), the centroid never moves
more than 1.5 zones between frames, well within the 3-zone match distance.  No prediction
step is needed.

### Per-frame updates

A matched detection updates the track: position set to the centroid, `last_half`
derived from cy, peak height raised if the new peak exceeds the stored maximum.

- **Unmatched track** — `lost` incremented; deleted when it exceeds `COAST_MAX`.
- **Unmatched detection** — new track born at the detection centroid.

---

## Track lifecycle

### Coasting

A track that receives no matching detection holds its position and increments `lost`.
After `COAST_MAX = 3` frames (~200 ms) it is deleted and the entry/exit logic runs —
long enough to bridge a few bad sensor frames, short enough to avoid keeping ghost
tracks alive.

---

## Entry / exit detection

### Half convention

The crossing axis is Y (columns).  The sensor field is divided into two halves:

- **Outer half** — columns 0–3 (the approach side, outside the building).
- **Inner half** — columns 4–7 (the interior side).

Each track records `first_half` (half at birth) and `last_half` (most recent half).
Both use the same midline rule: cy ≥ 4.0 → INNER, cy < 4.0 → OUTER.
`last_half` is updated every frame from the current centroid position.

### Event emission

When a track is deleted, an event fires only if `first_half ≠ last_half`:

- outer → inner: **enter**
- inner → outer: **leave**

Emitting on deletion correctly handles reversals: if a person retreats, `last_half` returns
to `first_half` before deletion and no event fires.

---

## Diagnostic frame data (`TRACKER_FRAME`)

When `TRACKER_FRAME` is defined in `tracker.h`, `tracker_process_frame` fills a
`TrackerFrame` struct that `sensor.c` serialises to JSON and publishes on
`CONFIG_MQTT_FRAME_TOPIC` every frame.  `tools/display.py` subscribes to that topic and
renders the grids.

### `zone_height` — filtered height map

The 64-element array holds the filtered height values after the temporal min/max step.

### `region_id` — track labels

Each element holds one of:

- `−1` (`REGION_UNASSIGNED`) — pixel is below the height threshold, was not reached by the
  watershed sweep, or belongs to a detection that could not be matched to any track slot.
- `−2` (`REGION_BOUNDARY`) — pixel lies on the boundary between two watershed regions.
- `≥ 0` — the index of the track (0-based slot in `TrackerState.tracks[]`) that owns the
  detection whose region covers this pixel.

`tools/display.py` renders the grid and the track status lines in the same colours, keyed
by track slot index.
