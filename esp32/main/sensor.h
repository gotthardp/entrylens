#pragma once
#include "mqtt_client.h"

void sensor_init(esp_mqtt_client_handle_t mqtt);
void sensor_step(void);
