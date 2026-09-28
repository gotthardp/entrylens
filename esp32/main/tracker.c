#include "tracker.h"
#include <string.h>
#include <math.h>

// Configuration parameters
#define BG_ALPHA            0.05f
#define BG_STABLE_MM        100.0f
#define MIN_PERSON_HEIGHT   500.0f   // filtering threshold [mm]
#define MIN_PROMINENCE      150.0f   // minimum size of human head [mm]
#define MIN_COMPONENT_SIZE  4        // how many pixels correspond to one person
#define MAX_MATCH_DIST_SQ   (3.0f * 3.0f)
#define COAST_MAX           3

// Sentinel values
#define INVALID_SADDLE      (-1.0f)  // no saddle height recorded for this component pair
#define REGION_UNASSIGNED   (-1)     // zone not yet reached by the watershed sweep
#define REGION_BOUNDARY     (-2)     // zone on the boundary between two watershed regions

typedef struct {
    float cx, cy;        // height-weighted centroid in zone-grid coordinates
    float peak_height;   // maximum zone height in this region (mm)
} Detection;

// Status codes 5 (range valid) and 9 (range valid, no target) indicate a usable measurement.
static bool valid_target(uint8_t status) { return status == 5 || status == 9; }

// Remap non-negative zone IDs through map[] in-place; negative values (UNASSIGNED, BOUNDARY) are unchanged.
static void remap_zones(int *ids, const int *map)
{
    for (int z = 0; z < TRACKER_ZONES; z++)
        if (ids[z] >= 0) ids[z] = map[ids[z]];
}

// Fill nbrs[] with the indices of valid 8-connected neighbours of z; return count.
static int neighbours(int z, int nbrs[8])
{
    int row = z / 8, col = z % 8, n = 0;
    for (int dr = -1; dr <= 1; dr++)
        for (int dc = -1; dc <= 1; dc++) {
            if (dr == 0 && dc == 0) continue;
            int r2 = row + dr, c2 = col + dc;
            if (r2 < 0 || r2 >= 8 || c2 < 0 || c2 >= 8) continue;
            nbrs[n++] = r2 * 8 + c2;
        }
    return n;
}


// Update per-zone background EMA using the farthest valid target in each zone.
static void update_background(TrackerState *st, VL53L8CX_ResultsData *r)
{
    for (int z = 0; z < TRACKER_ZONES; z++) {
        // Find farthest valid target (background surface behind any person).
        int16_t farthest = 0;   // farthest valid distance seen in this zone (mm)
        bool found = false;     // whether any valid target was found
        for (int t = 0; t < VL53L8CX_NB_TARGET_PER_ZONE; t++) {
            int idx = VL53L8CX_NB_TARGET_PER_ZONE * z + t;
            uint8_t status = r->target_status[idx];
            if (valid_target(status) && r->distance_mm[idx] > farthest) {
                farthest = r->distance_mm[idx];
                found = true;
            }
        }
        if (!found) continue;
        // Seed uninitialised zones; update EMA only when reading is near the current estimate.
        if (!st->bg_initialized[z]) {
            st->background_mm[z] = farthest;
            st->bg_initialized[z] = true;
        } else if (fabsf(farthest - st->background_mm[z]) < BG_STABLE_MM) {
            st->background_mm[z] += BG_ALPHA * (farthest - st->background_mm[z]);
        }
    }
}

// Compute height above background for each zone from the closest valid target.
static void compute_height_map(TrackerState *st, VL53L8CX_ResultsData *r, float *zone_height)
{
    for (int z = 0; z < TRACKER_ZONES; z++) {
        zone_height[z] = 0.0f;
        if (!st->bg_initialized[z]) continue;
        // Use the closest valid target (maximum height above background).
        for (int t = 0; t < VL53L8CX_NB_TARGET_PER_ZONE; t++) {
            int idx = VL53L8CX_NB_TARGET_PER_ZONE * z + t;
            uint8_t status = r->target_status[idx];
            if (!valid_target(status)) continue;
            float h = st->background_mm[z] - r->distance_mm[idx];
            if (h > zone_height[z]) zone_height[z] = h;
        }
    }
}

// Return the root of z's component.
static int uf_find(int *parent, int z)
{
    while (parent[z] != z) {
        // path splitting
        parent[z] = parent[parent[z]];
        z = parent[z];
    }
    return z;
}

// Merge the components containing a and b.
static void uf_union(int *parent, int *rank, int a, int b)
{
    a = uf_find(parent, a);
    b = uf_find(parent, b);
    if (a == b) return;
    // union by rank: make the higher-rank root the parent
    if (rank[a] < rank[b]) { int t = a; a = b; b = t; }
    parent[b] = a;
    if (rank[a] == rank[b]) rank[a]++;
}

// Called after step 2 (initial stats) and again after step 5 (post-merge rebuild).
static void rebuild_stats(const float *height, int *uf_parent,
                           int *comp_of, float *comp_max_h, bool *comp_valid)
{
    static int comp_size[TRACKER_ZONES];  // pixel count per component root

    memset(comp_size,  0,    TRACKER_ZONES * sizeof(int));
    memset(comp_valid, false, TRACKER_ZONES * sizeof(bool));
    for (int z = 0; z < TRACKER_ZONES; z++) comp_max_h[z] = 0.0f;

    for (int z = 0; z < TRACKER_ZONES; z++) {
        if (height[z] < MIN_PERSON_HEIGHT) { comp_of[z] = -1; continue; }
        int root = uf_find(uf_parent, z);
        comp_of[z] = root;
        comp_size[root]++;
        if (height[z] > comp_max_h[root]) comp_max_h[root] = height[z];
    }

    // comp_valid is set only for root pixels (comp_of[z] == z)
    for (int z = 0; z < TRACKER_ZONES; z++)
        if (comp_of[z] == z)
            comp_valid[z] = (comp_size[z] >= MIN_COMPONENT_SIZE);
}

// Segmentation steps 1-5; see tracker.md, Segmentation section.
static void build_components(const float *height,
                              int *sorted_z, int *n_active,
                              int *comp_of, bool *comp_valid)
{
    static float sorted_h  [TRACKER_ZONES];                    // heights parallel to sorted_z, for insertion sort
    static int   uf_parent [TRACKER_ZONES];                    // Union-Find parent pointers
    static int   uf_rank   [TRACKER_ZONES];                    // Union-Find ranks (for union-by-rank)
    static bool  uf_active [TRACKER_ZONES];                    // zone has been activated in the descending sweep
    static bool  uf_saddle [TRACKER_ZONES];                    // zone is a saddle; excluded from subsequent neighbour scans
    static int   adj_list  [TRACKER_ZONES][TRACKER_ZONES];     // adj_list[C][k] = k-th adjacent root of C (16 KB)
    static int   adj_count [TRACKER_ZONES];                    // number of recorded adjacencies per component root
    static float saddle_h  [TRACKER_ZONES][TRACKER_ZONES];     // saddle height for the (C, adj_list[C][k]) pair (16 KB)
    static float comp_max_h[TRACKER_ZONES];                    // peak height per component root (mm)
    static int   merge_a   [256];                              // component A of each low-prominence merge pair
    static int   merge_b   [256];                              // component B of each low-prominence merge pair

    // Step 1: insertion-sort active pixels by descending height.
    *n_active = 0;
    for (int z = 0; z < TRACKER_ZONES; z++) {
        float h = height[z];
        if (h < MIN_PERSON_HEIGHT) continue;
        int j = *n_active - 1;
        while (j >= 0 && sorted_h[j] < h) {
            sorted_h[j + 1] = sorted_h[j];
            sorted_z[j + 1] = sorted_z[j];
            j--;
        }
        sorted_h[j + 1] = h;
        sorted_z[j + 1] = z;
        (*n_active)++;
    }

    // Step 2: activate pixels highest-first; record saddles and adjacency.
    for (int z = 0; z < TRACKER_ZONES; z++) {
        uf_parent[z] = z; uf_rank[z]   = 0;
        uf_active[z] = false; uf_saddle[z] = false;
    }
    memset(adj_count, 0, TRACKER_ZONES * sizeof(int));
    for (int c = 0; c < TRACKER_ZONES; c++)
        for (int k = 0; k < TRACKER_ZONES; k++)
            saddle_h[c][k] = INVALID_SADDLE;

    for (int i = 0; i < *n_active; i++) {
        int z = sorted_z[i];
        uf_active[z] = true;
        float h = height[z];

        // Collect distinct component roots from active, non-saddle 8-neighbours.
        int nbrs[8], n_nbrs = neighbours(z, nbrs);
        int roots[8], n_roots = 0;   // distinct component roots among active neighbours
        for (int n = 0; n < n_nbrs; n++) {
            int q = nbrs[n];
            if (!uf_active[q] || uf_saddle[q]) continue;
            int r = uf_find(uf_parent, q);
            bool dup = false;
            for (int k = 0; k < n_roots; k++) if (roots[k] == r) { dup = true; break; }
            if (!dup) roots[n_roots++] = r;
        }

        // n_roots == 0: isolated seed; uf_parent[z] = z is already set.
        if (n_roots == 1) {
            uf_union(uf_parent, uf_rank, z, roots[0]);
        } else if (n_roots >= 2) {
            uf_saddle[z] = true;
            for (int a = 0; a < n_roots; a++) {
                for (int b = a + 1; b < n_roots; b++) {
                    int rA = roots[a], rB = roots[b];
                    int C = (rA < rB) ? rA : rB, D = (rA < rB) ? rB : rA;  // canonical order: C < D
                    // First encounter = highest connection (descending sweep); don't overwrite.
                    int k = -1;
                    for (int j = 0; j < adj_count[C]; j++)
                        if (adj_list[C][j] == D) { k = j; break; }
                    if (k < 0) {
                        k = adj_count[C]++;
                        adj_list[C][k] = D;
                        saddle_h[C][k] = h;
                    }
                }
            }
        }
    }

    // Step 3: initial component statistics.
    rebuild_stats(height, uf_parent, comp_of, comp_max_h, comp_valid);

    // Step 4: collect low-prominence pairs, union them in one batch, rebuild stats.
    // adj_list is keyed by the root at saddle-detection time; subsequent unions in step 2
    // can change roots, so resolve C and D through uf_find before using comp_valid/comp_max_h.
    int n_merges = 0;
    for (int C = 0; C < TRACKER_ZONES; C++) {
        if (adj_count[C] == 0) continue;
        int rC = uf_find(uf_parent, C);
        if (!comp_valid[rC]) continue;
        for (int k = 0; k < adj_count[C]; k++) {
            int rD = uf_find(uf_parent, adj_list[C][k]);
            if (!comp_valid[rD] || rC == rD) continue;
            float s = saddle_h[C][k];
            if (s < 0.0f) continue;
            if (fminf(comp_max_h[rC], comp_max_h[rD]) - s < MIN_PROMINENCE) {
                if (n_merges < 256) {
                    merge_a[n_merges] = rC;
                    merge_b[n_merges] = rD;
                    n_merges++;
                }
            }
        }
    }
    for (int i = 0; i < n_merges; i++)
        uf_union(uf_parent, uf_rank, merge_a[i], merge_b[i]);
    rebuild_stats(height, uf_parent, comp_of, comp_max_h, comp_valid);
}

// Segmentation step 5; see tracker.md, Segmentation section.
static void watershed(const int *sorted_z, int n_active,
                       const int *comp_of, const bool *comp_valid,
                       int *region_id)
{
    // Seed valid component pixels with their root as the region ID.
    for (int z = 0; z < TRACKER_ZONES; z++)
        region_id[z] = REGION_UNASSIGNED;

    for (int z = 0; z < TRACKER_ZONES; z++) {
        int root = comp_of[z];
        if (root >= 0 && comp_valid[root]) region_id[z] = root;
    }

    // Descending sweep: assign unseeded pixels from already-assigned neighbours.
    for (int i = 0; i < n_active; i++) {
        int z = sorted_z[i];
        if (region_id[z] != REGION_UNASSIGNED) continue;

        int nbrs[8], n_nbrs = neighbours(z, nbrs);
        int seen = REGION_UNASSIGNED;   // single region ID seen among assigned neighbours so far
        bool conflict = false;          // true when neighbours belong to more than one region

        for (int n = 0; n < n_nbrs && !conflict; n++) {
            int r = region_id[nbrs[n]];
            if (r < 0) continue;
            if (seen == REGION_UNASSIGNED) seen = r;
            else if (seen != r)            conflict = true;
        }

        if (conflict)       region_id[z] = REGION_BOUNDARY;
        else if (seen >= 0) region_id[z] = seen;
    }
}

// out_region_id (may be NULL): remapped in-place from watershed roots to detection indices.
static int extract_detections(const float *height,
                               const int *region_id, const bool *comp_valid,
                               Detection *dets, int max_dets, int *out_region_id)
{
    static float sum_h [TRACKER_ZONES];       // total height weight per root
    static float sum_hr[TRACKER_ZONES];       // height-weighted row sum per root (-> cx)
    static float sum_hc[TRACKER_ZONES];       // height-weighted col sum per root (-> cy)
    static float peak_h[TRACKER_ZONES];       // peak zone height per root (mm)
    static int   root_to_det[TRACKER_ZONES];  // component root index -> detection index (-1 if invalid)

    // Accumulate height-weighted sums per region root.
    memset(sum_h,  0, sizeof(sum_h));
    memset(sum_hr, 0, sizeof(sum_hr));
    memset(sum_hc, 0, sizeof(sum_hc));
    memset(peak_h, 0, sizeof(peak_h));
    for (int z = 0; z < TRACKER_ZONES; z++) {
        int r = region_id[z];
        if (r < 0) continue;
        float h = height[z];
        sum_h [r] += h;
        // row -> X -> cx, col -> Y -> cy
        sum_hr[r] += h * (z / 8);
        sum_hc[r] += h * (z % 8);
        if (h > peak_h[r]) peak_h[r] = h;
    }

    // Emit one detection per valid root; build root -> detection index map.
    for (int i = 0; i < TRACKER_ZONES; i++) root_to_det[i] = -1;
    int n = 0;
    for (int r = 0; r < TRACKER_ZONES && n < max_dets; r++) {
        if (!comp_valid[r] || sum_h[r] <= 0.0f) continue;
        dets[n].cx          = sum_hr[r] / sum_h[r];
        dets[n].cy          = sum_hc[r] / sum_h[r];
        dets[n].peak_height = peak_h[r];
        root_to_det[r] = n++;
    }

    // Remap out_region_id: watershed roots -> detection indices.
    if (out_region_id)
        remap_zones(out_region_id, root_to_det);

    return n;
}

// Run the full segmentation pipeline; return number of detections written to dets[].
static int find_detections(const float *height, Detection *dets, int max_dets,
                            int *out_region_id)
{
    static int  sorted_z  [TRACKER_ZONES];   // active zone indices sorted by descending height
    static int  comp_of   [TRACKER_ZONES];   // component root for each zone (-1 if below threshold)
    static bool comp_valid[TRACKER_ZONES];   // true for roots that passed the size filter
    static int  region_id [TRACKER_ZONES];   // watershed region assignment per zone
    int n_active;                            // number of above-threshold zones

    build_components(height, sorted_z, &n_active, comp_of, comp_valid);
    watershed(sorted_z, n_active, comp_of, comp_valid, region_id);

    if (out_region_id)
        memcpy(out_region_id, region_id, TRACKER_ZONES * sizeof(int));

    return extract_detections(height, region_id, comp_valid, dets, max_dets, out_region_id);
}

// Map a Y centroid to its half of the sensor field.
static Half half_of(float cy)
{
    return cy >= TRACKER_MAX_TRACKS / 2.0f ? HALF_INNER : HALF_OUTER;
}

// Zero-initialise all tracker state; must be called once before tracker_process_frame.
void tracker_init(TrackerState *st)
{
    memset(st, 0, sizeof(*st));
}

// Process one ranging frame: update background, segment, match tracks, emit entry/exit events.
#ifdef TRACKER_FRAME
int tracker_process_frame(TrackerState *st, VL53L8CX_ResultsData *r,
                           TrackerEvent *events, int max_events,
                           TrackerFrame *frame)
#else
int tracker_process_frame(TrackerState *st, VL53L8CX_ResultsData *r,
                           TrackerEvent *events, int max_events)
#endif
{
    float zone_height     [TRACKER_ZONES];        // raw height map (before temporal filter)
    float zone_height_filt[TRACKER_ZONES];        // two-frame min/max filtered height map
    Detection dets        [TRACKER_MAX_TRACKS];   // current frame's detections

    update_background(st, r);
    compute_height_map(st, r, zone_height);

    // Two-frame min/max filter: suppress spikes below threshold, dips above it.
    for (int z = 0; z < TRACKER_ZONES; z++) {
        zone_height_filt[z] = (zone_height[z] < MIN_PERSON_HEIGHT)
            ? fminf(zone_height[z], st->height_prev[z])
            : fmaxf(zone_height[z], st->height_prev[z]);
        st->height_prev[z] = zone_height[z];
    }

#ifdef TRACKER_FRAME
    int n_dets = find_detections(zone_height_filt, dets, TRACKER_MAX_TRACKS, frame->region_id);
    memcpy(frame->zone_height, zone_height_filt, sizeof(zone_height_filt));
#else
    int n_dets = find_detections(zone_height_filt, dets, TRACKER_MAX_TRACKS, NULL);
#endif

    // Collect candidate (detection, track) pairs within MAX_MATCH_DIST_SQ.
    float pair_d2[TRACKER_MAX_TRACKS * TRACKER_MAX_TRACKS];  // squared distance for each candidate pair
    int   pair_d [TRACKER_MAX_TRACKS * TRACKER_MAX_TRACKS];  // detection index for each candidate pair
    int   pair_t [TRACKER_MAX_TRACKS * TRACKER_MAX_TRACKS];  // track slot index for each candidate pair
    int   n_pairs = 0;

    for (int d = 0; d < n_dets; d++) {
        for (int t = 0; t < TRACKER_MAX_TRACKS; t++) {
            if (!st->tracks[t].active) continue;
            float dr = dets[d].cx - st->tracks[t].cx;
            float dc = dets[d].cy - st->tracks[t].cy;
            float d2 = dr * dr + dc * dc;
            if (d2 < MAX_MATCH_DIST_SQ) {
                pair_d2[n_pairs] = d2;
                pair_d [n_pairs] = d;
                pair_t [n_pairs] = t;
                n_pairs++;
            }
        }
    }

    // Sort pairs by distance for greedy assignment.
    for (int i = 1; i < n_pairs; i++) {
        float v = pair_d2[i]; int pd = pair_d[i], pt = pair_t[i];
        int j = i - 1;
        while (j >= 0 && pair_d2[j] > v) {
            pair_d2[j + 1] = pair_d2[j];
            pair_d [j + 1] = pair_d[j];
            pair_t [j + 1] = pair_t[j];
            j--;
        }
        pair_d2[j + 1] = v; pair_d[j + 1] = pd; pair_t[j + 1] = pt;
    }

    // Greedy nearest-neighbour: each detection and track matched at most once.
    int matched_track[TRACKER_MAX_TRACKS];   // track slot matched to detection d (-1 if unmatched)
    int matched_det  [TRACKER_MAX_TRACKS];   // detection matched to track slot t (-1 if unmatched)
    for (int i = 0; i < TRACKER_MAX_TRACKS; i++) { matched_track[i] = -1; matched_det[i] = -1; }
    for (int i = 0; i < n_pairs; i++) {
        int d = pair_d[i], t = pair_t[i];
        if (matched_track[d] == -1 && matched_det[t] == -1) {
            matched_track[d] = t;
            matched_det[t]   = d;
        }
    }

    // Update matched tracks; increment lost counter on unmatched.
    for (int t = 0; t < TRACKER_MAX_TRACKS; t++) {
        if (!st->tracks[t].active) continue;
        int d = matched_det[t];
        if (d >= 0) {
            st->tracks[t].cx   = dets[d].cx;
            st->tracks[t].cy   = dets[d].cy;
            st->tracks[t].lost = 0;
            if (dets[d].peak_height > st->tracks[t].height)
                st->tracks[t].height = dets[d].peak_height;
            st->tracks[t].last_half = half_of(st->tracks[t].cy);
        } else {
            st->tracks[t].lost++;
        }
    }

    // Prune coasted tracks; emit entry/exit events on crossing.
    int n = 0;   // number of events written to events[]
    for (int t = 0; t < TRACKER_MAX_TRACKS; t++) {
        if (!st->tracks[t].active) continue;
        if (st->tracks[t].lost <= COAST_MAX) continue;
        if (n < max_events && st->tracks[t].first_half != st->tracks[t].last_half) {
            events[n].type   = (st->tracks[t].last_half == HALF_INNER) ? TRACKER_ENTER : TRACKER_LEAVE;
            events[n].height = st->tracks[t].height;
            n++;
        }
        st->tracks[t].active = false;
    }

    // Birth new tracks from unmatched detections.
    for (int d = 0; d < n_dets; d++) {
        if (matched_track[d] >= 0) continue;
        for (int t = 0; t < TRACKER_MAX_TRACKS; t++) {
            if (st->tracks[t].active) continue;
            st->tracks[t].cx         = dets[d].cx;
            st->tracks[t].cy         = dets[d].cy;
            st->tracks[t].height     = dets[d].peak_height;
            st->tracks[t].first_half = half_of(dets[d].cy);
            st->tracks[t].last_half  = st->tracks[t].first_half;
            st->tracks[t].lost       = 0;
            st->tracks[t].active     = true;
            matched_track[d]         = t;
            break;
        }
    }

#ifdef TRACKER_FRAME
    // Remap frame->region_id: detection index -> track index.
    remap_zones(frame->region_id, matched_track);
#endif

    return n;
}
