#include "c_api.h"

#include "HAL.h"
#include "CH58x_common.h"
#include <CH58xBLE_LIB.h>
#include "CH58x_gpio.h"

#include "main.h"
#include <hiddev.h>
#include <hidmouseservice.h>
#include <lsm6dsv.h>
#include <stdio.h>

bool c_read_sflp_game_rotation_raw(sflp_game_rotation_raw_t *raw) {
    return LSM6DSV_ReadSFLPGameRotationRaw(raw);
}

void c_print_to_uart(const char *ptr, const unsigned int len) {
    printf("%.*s", (int)len, ptr);
}

typedef struct {
    uint8_t buttons;
    int8_t  dx;
    int8_t  dy;
    int8_t  wheel;
} mouse_report_t;

static int8_t clamp_i8_to_hid_range(const int8_t v) {
    if (v == -128) {
        return -127;
    }
    return v;
}

hid_send_status_t c_send_ble_hid_mouse_report(const uint8_t buttons,
                                              const int8_t dx, const int8_t dy,
                                              const int8_t wheel) {
    mouse_report_t rpt;
    rpt.buttons = buttons;
    rpt.dx = clamp_i8_to_hid_range(dx);
    rpt.dy = clamp_i8_to_hid_range(dy);
    rpt.wheel = clamp_i8_to_hid_range(wheel);
    const uint8_t status =
        HidDev_Report(HID_RPT_ID_MOUSE_IN, HID_REPORT_TYPE_INPUT,
                      sizeof(mouse_report_t), (uint8_t *)&rpt);
    switch (status) {
        case SUCCESS:
            return HID_SEND_OK;
        case bleMemAllocError:
        case bleNotReady:
            return HID_SEND_RETRY;
        case bleNoResources:
        default:
            return HID_SEND_FATAL;
    }
}

input_status_t c_get_input_status() {
    const uint32_t portA_data = GPIOA_ReadPort();
    input_status_t input_status;

    input_status.left = portA_data & LEFT_BTN ? 0 : 1;
    input_status.right = portA_data & RIGHT_BTN ? 0 : 1;
    input_status.middle = portA_data & MIDDLE_BTN ? 0 : 1;
    input_status.action = portA_data & ACTION_BTN ? 0 : 1;
    input_status.light = portA_data & LIGHT_BTN ? 0 : 1;
    input_status.enc_a = portA_data & ENC_A ? 0 : 1;
    input_status.enc_b = portA_data & ENC_B ? 0 : 1;

    return input_status;
}

uint32_t c_get_rtc_tick() {
    return RTC_GetCycle32k();
}

uint32_t c_get_rtc_millis() {
    return RTC_TO_MS(c_get_rtc_tick());
}

uint32_t c_get_rtc_micros() {
    return RTC_TO_US(c_get_rtc_tick());
}