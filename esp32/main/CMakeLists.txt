idf_component_register(SRCS "main.c" "sensor.c" "tracker.c"
                       PRIV_REQUIRES esp_driver_gpio nvs_flash
                                     esp_wifi esp_event esp_netif
                                     espressif__mqtt espressif__cjson
                                     esp_driver_i2c rjrp44__vl53l8cx
                                     esp_timer
                       INCLUDE_DIRS "")
