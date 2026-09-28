#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "vl53l8cx_api.h"

#define TRACKER_MAX_TRACKS 8
#define TRACKER_ZONES      64

// Comment out to disable diagnostic frame data and MQTT frame publishing.
#define TRACKER_FRAME

typedef enum { HALF_OUTER = 0, HALF_INNER = 1 } Half;

typedef struct {
    bool  active;
    float cx, cy;          // position in zone-grid coordinates
    int   lost;            // consecutive frames without a matching detection
    float height;          // peak zone height seen by this track (mm)
    Half  first_half, last_half;
} Track;

typedef struct {
    float background_mm[TRACKER_ZONES];
    bool  bg_initialized[TRACKER_ZONES];
    float height_prev[TRACKER_ZONES];    // previous frame's raw height, for min/max filter
    Track tracks[TRACKER_MAX_TRACKS];
} TrackerState;

typedef enum { TRACKER_ENTER, TRACKER_LEAVE } TrackerEventType;

typedef struct {
    TrackerEventType type;
    float height;
} TrackerEvent;

#ifdef TRACKER_FRAME
typedef struct {
    float zone_height[TRACKER_ZONES];
    int   region_id[TRACKER_ZONES];  // track index (0-based), REGION_UNASSIGNED=-1, REGION_BOUNDARY=-2
} TrackerFrame;
#endif

// Zero-initialise all tracker state; must be called once before tracker_process_frame.
void tracker_init(TrackerState *st);

// Process one ranging result through the full pipeline; returns number of events written.
// See tracker.md for a detailed description of each stage.
#ifdef TRACKER_FRAME
int  tracker_process_frame(TrackerState *st, VL53L8CX_ResultsData *r,
                            TrackerEvent *events, int max_events,
                            TrackerFrame *frame);
#else
int  tracker_process_frame(TrackerState *st, VL53L8CX_ResultsData *r,
                            TrackerEvent *events, int max_events);
#endif
