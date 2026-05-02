#include "c_api.h"

#include "HAL.h"  // IWYU pragma: keep
#include "CH58x_common.h"  // IWYU pragma: keep
#include <CH58xBLE_LIB.h>
#include "CH58x_gpio.h"

#include "main.h"
#include <hiddev.h>
#include <hidmouseservice.h>
#include <stdio.h>

typedef struct {
    uint8_t buttons;
    int8_t dx;
    int8_t dy;
    int8_t wheel;
} mouse_report_t;

static int8_t clamp_i8_to_hid_range(const int8_t v) {
    if (v == -128) {
        return -127;
    }
    return v;
}

static vp_bool_t active_low_pin_level(uint32_t port_data, uint32_t pin_mask) {
    return (port_data & pin_mask) ? 0u : 1u;
}

vp_bool_t c_vp_gpio_read(const vp_input_id_t input_id) {
    const uint32_t portA_data = GPIOA_ReadPort();

    switch (input_id) {
        case VP_INPUT_LEFT:
            return active_low_pin_level(portA_data, LEFT_BTN);
        case VP_INPUT_RIGHT:
            return active_low_pin_level(portA_data, RIGHT_BTN);
        case VP_INPUT_MIDDLE:
            return active_low_pin_level(portA_data, MIDDLE_BTN);
        case VP_INPUT_ACTION:
            return active_low_pin_level(portA_data, ACTION_BTN);
        case VP_INPUT_LASER:
            return active_low_pin_level(portA_data, LIGHT_BTN);
        case VP_INPUT_ENCODER_A:
            return active_low_pin_level(portA_data, ENC_A);
        case VP_INPUT_ENCODER_B:
            return active_low_pin_level(portA_data, ENC_B);
        case VP_INPUT_MODE_SWITCH:
        case VP_INPUT_IMU_INT1:
        case VP_INPUT_IMU_INT2:
        default:
            return 0u;
    }
}

vp_status_t c_vp_gpio_read_inputs(uint16_t *out_snapshot) {
    if (out_snapshot == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    uint16_t snapshot = 0u;
    for (uint8_t input = VP_INPUT_LEFT; input <= VP_INPUT_IMU_INT2; input++) {
        if (c_vp_gpio_read((vp_input_id_t)input)) {
            snapshot |= (uint16_t)(1u << input);
        }
    }
    *out_snapshot = snapshot;
    return VP_STATUS_OK;
}

vp_status_t c_vp_gpio_write(const vp_output_id_t output_id, const vp_bool_t level) {
    switch (output_id) {
        case VP_OUTPUT_LASER:
            if (level) {
                GPIOA_SetBits(LIGHT_BTN);
            } else {
                GPIOA_ResetBits(LIGHT_BTN);
            }
            return VP_STATUS_OK;
        default:
            return VP_STATUS_INVALID_ARG;
    }
}

vp_status_t c_vp_exti_mask(const vp_input_id_t input_id) {
    (void)input_id;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_exti_unmask(const vp_input_id_t input_id) {
    (void)input_id;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_exti_clear_pending(const vp_input_id_t input_id) {
    (void)input_id;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_exti_set_edge(const vp_input_id_t input_id, const vp_exti_edge_t edge) {
    (void)input_id;
    (void)edge;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_debounce_timer_start(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_debounce_timer_stop(void) {
    return VP_STATUS_UNSUPPORTED;
}

uint32_t c_vp_rtc_tick(void) {
    return RTC_GetCycle32k();
}

vp_timestamp_t c_vp_rtc_millis(void) {
    return RTC_TO_MS(c_vp_rtc_tick());
}

uint32_t c_vp_rtc_micros(void) {
    return RTC_TO_US(c_vp_rtc_tick());
}

vp_status_t c_vp_rtc_set_wake_after(const uint32_t ms) {
    (void)ms;
    return VP_STATUS_UNSUPPORTED;
}

void c_vp_request_core_poll(void) {
    /* TODO: Set/merge the dedicated TMOS event that calls vp_core_poll(). */
}

vp_status_t c_vp_i2c_init(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_i2c_recover_bus(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_i2c_abort(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_imu_config_active(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_imu_config_suspend(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_imu_config_sleep(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_imu_read_fifo_async(const uint16_t max_samples) {
    (void)max_samples;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_imu_read_whoami(uint8_t *out_id) {
    if (out_id == NULL) {
        return VP_STATUS_INVALID_ARG;
    }
    *out_id = 0u;
    return VP_STATUS_UNSUPPORTED;
}

vp_bool_t c_vp_hid_route_ready(const vp_hid_route_t route) {
    return route == VP_HID_ROUTE_BLE ? 1u : 0u;
}

vp_hid_send_status_t c_vp_hid_send_mouse(const vp_hid_route_t route, const uint8_t buttons,
                                          const int8_t dx, const int8_t dy,
                                          const int8_t wheel) {
    if (route != VP_HID_ROUTE_BLE) {
        return VP_HID_SEND_NOT_CONNECTED;
    }

    mouse_report_t rpt;
    rpt.buttons = buttons;
    rpt.dx = clamp_i8_to_hid_range(dx);
    rpt.dy = clamp_i8_to_hid_range(dy);
    rpt.wheel = clamp_i8_to_hid_range(wheel);

    const uint8_t status = HidDev_Report(HID_RPT_ID_MOUSE_IN, HID_REPORT_TYPE_INPUT,
                                         sizeof(mouse_report_t), (uint8_t *)&rpt);
    switch (status) {
        case SUCCESS:
            return VP_HID_SEND_SENT;
        case bleMemAllocError:
        case bleNotReady:
            return VP_HID_SEND_RETRY_LATER;
        case bleNoResources:
        default:
            return VP_HID_SEND_FATAL;
    }
}

vp_hid_send_status_t c_vp_hid_send_vendor(const vp_hid_route_t route, const uint8_t *ptr,
                                           const uint16_t len) {
    (void)route;
    (void)ptr;
    (void)len;
    return VP_HID_SEND_NOT_CONNECTED;
}

vp_status_t c_vp_hid_route_enable(const vp_hid_route_t route, const vp_bool_t enabled) {
    (void)route;
    (void)enabled;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_hid_route_reset(const vp_hid_route_t route) {
    (void)route;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_prepare_suspend(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_enter_suspend(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_prepare_sleep(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_enter_sleep(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_restore_from_sleep(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_wake_source_enable(const vp_wake_source_t source, const vp_bool_t enabled) {
    (void)source;
    (void)enabled;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_flash_config_region(vp_flash_region_t *out_info) {
    if (out_info == NULL) {
        return VP_STATUS_INVALID_ARG;
    }
    out_info->offset = 0u;
    out_info->length = 0u;
    out_info->page_size = 0u;
    out_info->write_alignment = 0u;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_flash_read(const uint32_t offset, uint8_t *ptr, const uint32_t len) {
    (void)offset;
    (void)ptr;
    (void)len;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_flash_erase(const uint32_t offset, const uint32_t len) {
    (void)offset;
    (void)len;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_flash_write(const uint32_t offset, const uint8_t *ptr, const uint32_t len) {
    (void)offset;
    (void)ptr;
    (void)len;
    return VP_STATUS_UNSUPPORTED;
}

void c_vp_debug_print(const char *ptr, const uint16_t len) {
    printf("%.*s", (int)len, ptr);
}

vp_status_t c_vp_platform_reset(const uint32_t reason) {
    (void)reason;
    return VP_STATUS_UNSUPPORTED;
}
