#include "sensor.h"
#include "tracker.h"
#include "sdkconfig.h"
#include "driver/gpio.h"
#include "esp_log.h"
#include "driver/i2c_master.h"
#include "vl53l8cx_api.h"
#include <string.h>
#include <math.h>
#include <stdio.h>
#include "mqtt_client.h"
#include "cJSON.h"

static const char *TAG = "sensor";
static esp_mqtt_client_handle_t s_mqtt;
static TrackerState s_tracker;
static VL53L8CX_Configuration s_dev;

#define SENSOR_SCL_GPIO      GPIO_NUM_12
#define SENSOR_SDA_GPIO      GPIO_NUM_11
#define SENSOR_RESET_GPIO    GPIO_NUM_4

#ifdef TRACKER_FRAME
static void print_frame(const TrackerState *st, const TrackerFrame *frame)
{
    cJSON *root = cJSON_CreateObject();

    cJSON *bg_arr = cJSON_CreateArray();
    for (int z = 0; z < TRACKER_ZONES; z++)
        cJSON_AddItemToArray(bg_arr, cJSON_CreateNumber(st->bg_initialized[z] ? (int)st->background_mm[z] : -1));
    cJSON_AddItemToObject(root, "bg", bg_arr);

    cJSON *height0_arr = cJSON_CreateArray();
    for (int z = 0; z < TRACKER_ZONES; z++)
        cJSON_AddItemToArray(height0_arr, cJSON_CreateNumber((int)frame->zone_height[z]));
    cJSON_AddItemToObject(root, "height0", height0_arr);

    cJSON *region_arr = cJSON_CreateArray();
    for (int z = 0; z < TRACKER_ZONES; z++)
        cJSON_AddItemToArray(region_arr, cJSON_CreateNumber(frame->region_id[z]));
    cJSON_AddItemToObject(root, "region", region_arr);

    cJSON *tracks_arr = cJSON_CreateArray();
    for (int t = 0; t < TRACKER_MAX_TRACKS; t++) {
        if (!st->tracks[t].active) continue;
        cJSON *tr = cJSON_CreateObject();
        cJSON_AddItemToObject(tr, "t",    cJSON_CreateNumber(t));
        cJSON_AddItemToObject(tr, "cx",   cJSON_CreateNumber((int)roundf(st->tracks[t].cx * 10)));
        cJSON_AddItemToObject(tr, "cy",   cJSON_CreateNumber((int)roundf(st->tracks[t].cy * 10)));
        cJSON_AddItemToObject(tr, "h",    cJSON_CreateNumber((int)st->tracks[t].height));
        cJSON_AddItemToObject(tr, "lost", cJSON_CreateNumber(st->tracks[t].lost));
        cJSON_AddItemToArray(tracks_arr, tr);
    }
    cJSON_AddItemToObject(root, "tracks", tracks_arr);

    char *payload = cJSON_PrintUnformatted(root);
    cJSON_Delete(root);

    if (payload) {
        esp_mqtt_client_publish(s_mqtt, CONFIG_MQTT_FRAME_TOPIC, payload, 0, 0, 0);
        cJSON_free(payload);
    }
}
#endif  // TRACKER_FRAME

static void process_and_publish(VL53L8CX_ResultsData *r)
{
    TrackerEvent events[TRACKER_MAX_TRACKS];
#ifdef TRACKER_FRAME
    TrackerFrame frame;
    int n = tracker_process_frame(&s_tracker, r, events, TRACKER_MAX_TRACKS, &frame);
#else
    int n = tracker_process_frame(&s_tracker, r, events, TRACKER_MAX_TRACKS);
#endif

    for (int i = 0; i < n; i++) {
        const char *dir = (events[i].type == TRACKER_ENTER) ? "enter" : "leave";
        ESP_LOGI(TAG, "%s height=%.0fmm", dir, events[i].height);
        char payload[48];
        snprintf(payload, sizeof(payload), "{\"event\":\"%s\",\"height\":%.0f}",
                 dir, events[i].height);
        esp_mqtt_client_publish(s_mqtt, CONFIG_MQTT_TOPIC, payload, 0, 1, 0);
    }

#ifdef TRACKER_FRAME
    print_frame(&s_tracker, &frame);
#endif
}

void sensor_init(esp_mqtt_client_handle_t mqtt)
{
    s_mqtt = mqtt;
    tracker_init(&s_tracker);

    i2c_master_bus_config_t bus_cfg = {
        .clk_source                   = I2C_CLK_SRC_DEFAULT,
        .i2c_port                     = I2C_NUM_1,
        .scl_io_num                   = SENSOR_SCL_GPIO,
        .sda_io_num                   = SENSOR_SDA_GPIO,
        .glitch_ignore_cnt            = 7,
        .flags.enable_internal_pullup = true,
    };
    i2c_master_bus_handle_t bus;
    ESP_ERROR_CHECK(i2c_new_master_bus(&bus_cfg, &bus));

    i2c_device_config_t dev_cfg = {
        .dev_addr_length = I2C_ADDR_BIT_LEN_7,
        .device_address  = VL53L8CX_DEFAULT_I2C_ADDRESS >> 1,
        .scl_speed_hz    = VL53L8CX_MAX_CLK_SPEED,
    };

    s_dev.platform.bus_config = bus_cfg;
    s_dev.platform.reset_gpio = SENSOR_RESET_GPIO;
    i2c_master_bus_add_device(bus, &dev_cfg, &s_dev.platform.handle);
    gpio_reset_pin(SENSOR_RESET_GPIO);
    VL53L8CX_Reset_Sensor(&s_dev.platform);

    uint8_t alive;
    vl53l8cx_is_alive(&s_dev, &alive);
    if (!alive) {
        ESP_LOGE(TAG, "sensor not found");
        abort();
    }

    if (vl53l8cx_init(&s_dev)) {
        ESP_LOGE(TAG, "init failed");
        abort();
    }

    vl53l8cx_set_resolution(&s_dev, VL53L8CX_RESOLUTION_8X8);
    vl53l8cx_set_ranging_frequency_hz(&s_dev, 15);
    // Return closest object per zone (person in front of background, not strongest reflector)
    vl53l8cx_set_target_order(&s_dev, VL53L8CX_TARGET_ORDER_CLOSEST);
    // Moderate sharpening to reduce zone boundary bleed without losing marginal detections
    vl53l8cx_set_sharpener_percent(&s_dev, 20);
    vl53l8cx_start_ranging(&s_dev);
    ESP_LOGI(TAG, "ranging started (8x8 @ 15 Hz)");
}

void sensor_step(void)
{
    VL53L8CX_ResultsData results;
    uint8_t ready;
    do {
        vl53l8cx_check_data_ready(&s_dev, &ready);
        if (!ready)
            VL53L8CX_WaitMs(&s_dev.platform, 5);
    } while (!ready);
    vl53l8cx_get_ranging_data(&s_dev, &results);
    process_and_publish(&results);
}
