#include "c_api.h"

#include <CH58xBLE_LIB.h>
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