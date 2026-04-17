#ifndef VOIDPOINTER_C_API_H
#define VOIDPOINTER_C_API_H

#include <stdint.h>
#include <stdbool.h>

typedef struct {
    uint16_t x;
    uint16_t y;
    uint16_t z;
} sflp_game_rotation_raw_t;

bool c_read_sflp_game_rotation_raw(sflp_game_rotation_raw_t *raw);

void c_print_to_uart(const char *ptr, unsigned int len);

typedef enum {
    HID_SEND_OK = 0,
    HID_SEND_RETRY = 1,
    HID_SEND_FATAL = 2,
} hid_send_status_t;

hid_send_status_t c_send_ble_hid_mouse_report(uint8_t buttons, int8_t dx,
                                              int8_t dy, int8_t wheel);

#endif
