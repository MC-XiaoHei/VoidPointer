/********************************** (C) COPYRIGHT *******************************
 * File Name          : c_api.c
 * Description        : C platform bindings for Rust core
 *******************************************************************************/
#include "c_api.h"

#include "HAL.h"  // IWYU pragma: keep
#include "CH58x_common.h"  // IWYU pragma: keep
#include <CH58xBLE_LIB.h>
#include "CH58x_gpio.h"

#include "main.h"
#include "rust_api.h"
#include <hiddev.h>
#include <hidmouseservice.h>
#include "ble_hid_app.h"
#include "usbhs_hid_device.h"
#include "lsm6dsv.h"
#include "board_map.h"
#include "board_gpio.h"
#include "board_input.h"

typedef struct {
    uint8_t buttons;
    int8_t  dx;
    int8_t  dy;
    int8_t  wheel;
} mouse_report_t;

static vp_bool_t      debounce_timer_running = 0u;
static vp_usb_state_t current_usb_state = VP_USB_STATE_DETACHED;

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
    BoardGpio gpio = {0};
    if (!board_input_id_to_gpio(input_id, &gpio)) {
        return 0u;
    }

    return board_gpio_read_level(gpio) ? 0u : 1u;
}

vp_status_t c_vp_gpio_read_inputs(uint16_t* out_snapshot) {
    if (out_snapshot == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    const uint32_t portA_data = board_gpio_read_port(BOARD_GPIO_GROUP_A);
    const uint32_t portB_data = board_gpio_read_port(BOARD_GPIO_GROUP_B);
    uint16_t       snapshot = 0u;
    for (uint8_t input = VP_INPUT_LEFT; input <= VP_INPUT_IMU_INT2; input++) {
        BoardGpio gpio = {0};
        if (board_input_id_to_gpio((vp_input_id_t)input, &gpio)) {
            vp_bool_t active = 0u;

            if (gpio.group == BOARD_GPIO_GROUP_A) {
                active = active_low_pin_level(portA_data, gpio.pin);
            } else if (gpio.group == BOARD_GPIO_GROUP_B) {
                active = active_low_pin_level(portB_data, gpio.pin);
            }

            if (active) {
                snapshot |= (uint16_t)(1u << input);
            }
        }
    }
    *out_snapshot = snapshot;
    return VP_STATUS_OK;
}

vp_status_t c_vp_gpio_write(const vp_output_id_t output_id,
                            const vp_bool_t      level) {
    switch (output_id) {
        case VP_OUTPUT_LASER:
            if (!board_gpio_is_valid(board_btn_laser)) {
                return VP_STATUS_UNSUPPORTED;
            }
            if (level) {
                board_gpio_set(board_btn_laser);
            } else {
                board_gpio_reset(board_btn_laser);
            }
            return VP_STATUS_OK;
        default:
            return VP_STATUS_INVALID_ARG;
    }
}

vp_status_t c_vp_exti_mask(const vp_input_id_t input_id) {
    BoardGpio gpio = {0};
    if (!board_input_id_to_gpio(input_id, &gpio)) {
        return VP_STATUS_INVALID_ARG;
    }

    return board_gpio_int_mask(gpio);
}

vp_status_t c_vp_exti_unmask(const vp_input_id_t input_id) {
    BoardGpio gpio = {0};
    if (!board_input_id_to_gpio(input_id, &gpio)) {
        return VP_STATUS_INVALID_ARG;
    }

    return board_input_exti_unmask(input_id, gpio);
}

vp_status_t c_vp_exti_clear_pending(const vp_input_id_t input_id) {
    BoardGpio gpio = {0};
    if (!board_input_id_to_gpio(input_id, &gpio)) {
        return VP_STATUS_INVALID_ARG;
    }

    board_gpio_clear_it_flag(gpio);
    return VP_STATUS_OK;
}

vp_status_t c_vp_exti_set_edge(const vp_input_id_t  input_id,
                               const vp_exti_edge_t edge) {
    BoardGpio gpio = {0};
    if (!board_input_id_to_gpio(input_id, &gpio)) {
        return VP_STATUS_INVALID_ARG;
    }

    return board_input_exti_set_edge(input_id, gpio, edge);
}

vp_status_t c_vp_debounce_timer_start(void) {
    if (debounce_timer_running) {
        return VP_STATUS_OK;
    }
    debounce_timer_running = 1u;
    RuntimeTask_StartDebounceTimer();
    return VP_STATUS_OK;
}

vp_status_t c_vp_debounce_timer_stop(void) {
    if (!debounce_timer_running) {
        return VP_STATUS_OK;
    }
    debounce_timer_running = 0u;
    RuntimeTask_StopDebounceTimer();
    return VP_STATUS_OK;
}

uint32_t c_vp_rtc_tick(void) { return RTC_GetCycle32k(); }

vp_timestamp_t c_vp_rtc_millis(void) { return TMOS_GetSystemClock(); }

uint32_t c_vp_rtc_micros(void) {
    return (uint32_t)((uint64_t)c_vp_rtc_millis() * 1000u);
}

vp_status_t c_vp_rtc_set_wake_after(const uint32_t ms) {
    (void)ms;
    return VP_STATUS_UNSUPPORTED;
}

void c_vp_request_core_poll(void) { RuntimeTask_RequestPoll(); }

void c_vp_request_core_poll_after(const uint32_t ms) {
    RuntimeTask_RequestPollAfter(ms);
}

void Platform_NotifyUsbStateChanged(const vp_usb_state_t state) {
    const vp_usb_state_t previous_state = current_usb_state;
    const vp_usb_state_t effective_state =
        (state == VP_USB_STATE_SUSPENDED) ? VP_USB_STATE_DETACHED : state;

    if (previous_state == effective_state) {
        return;
    }

    if (effective_state == VP_USB_STATE_DETACHED &&
        previous_state != VP_USB_STATE_DETACHED) {
        USBHS_HidDevice_ResetLinkState();
    }

    current_usb_state = effective_state;

    if (effective_state == VP_USB_STATE_CONFIGURED) {
        (void)BleHidApp_SetAdvertisingEnabled(FALSE);
        (void)BleHidApp_Disconnect();
    } else if (previous_state == VP_USB_STATE_CONFIGURED) {
        (void)BleHidApp_SetAdvertisingEnabled(TRUE);
    }

    vp_on_usb_state_changed(effective_state, c_vp_rtc_millis());
}

vp_status_t c_vp_i2c_init(void) { return VP_STATUS_UNSUPPORTED; }

vp_status_t c_vp_i2c_recover_bus(void) { return VP_STATUS_UNSUPPORTED; }

vp_status_t c_vp_i2c_abort(void) { return VP_STATUS_UNSUPPORTED; }

vp_status_t c_vp_imu_config_active(void) { return VP_STATUS_UNSUPPORTED; }

vp_status_t c_vp_imu_config_suspend(void) { return VP_STATUS_UNSUPPORTED; }

vp_status_t c_vp_imu_config_sleep(void) { return VP_STATUS_UNSUPPORTED; }

vp_status_t c_vp_imu_read_fifo_async(const uint16_t max_samples) {
    return LSM6DSV_StartAsyncFifoRead(max_samples);
}

vp_status_t c_vp_imu_read_whoami(uint8_t* out_id) {
    if (out_id == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    if (!LSM6DSV_ReadWhoAmI(out_id)) {
        *out_id = 0u;
        return VP_STATUS_IO_ERROR;
    }

    return VP_STATUS_OK;
}

static vp_bool_t ble_mouse_route_ready(void) {
    if (current_usb_state == VP_USB_STATE_CONFIGURED) {
        return 0u;
    }

    return HidDev_IsReportNotifyEnabled(HID_RPT_ID_MOUSE_IN,
                                        HID_REPORT_TYPE_INPUT)
               ? 1u
               : 0u;
}

static vp_bool_t usb_mouse_route_ready(void) {
    return current_usb_state == VP_USB_STATE_CONFIGURED ? 1u : 0u;
}

static vp_bool_t dongle_mouse_route_ready(void) { return 0u; }

static vp_hid_send_status_t ble_send_mouse_report(const mouse_report_t* rpt) {
    if (rpt == NULL) {
        return VP_HID_SEND_FATAL;
    }

    const uint8_t status =
        HidDev_Report(HID_RPT_ID_MOUSE_IN, HID_REPORT_TYPE_INPUT,
                      sizeof(mouse_report_t), (uint8_t*)rpt);
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

static vp_hid_send_status_t usb_send_mouse_report(const mouse_report_t* rpt) {
    if (rpt == NULL) {
        return VP_HID_SEND_FATAL;
    }

    if (current_usb_state != VP_USB_STATE_CONFIGURED) {
        return VP_HID_SEND_NOT_CONNECTED;
    }

    return USBHS_HidDevice_SendMouseReport((const uint8_t*)rpt, sizeof(*rpt))
               ? VP_HID_SEND_SENT
               : VP_HID_SEND_RETRY_LATER;
}

static vp_hid_send_status_t dongle_send_mouse_report(
    const mouse_report_t* rpt) {
    (void)rpt;
    return VP_HID_SEND_NOT_CONNECTED;
}

static vp_hid_send_status_t usb_send_vendor_report(const uint8_t* ptr,
                                                   const uint16_t len) {
    if (ptr == NULL) {
        return VP_HID_SEND_FATAL;
    }

    if (current_usb_state != VP_USB_STATE_CONFIGURED) {
        return VP_HID_SEND_NOT_CONNECTED;
    }

    return USBHS_HidDevice_SendVendorReport(ptr, len) ? VP_HID_SEND_SENT
                                                      : VP_HID_SEND_RETRY_LATER;
}

vp_bool_t c_vp_hid_route_ready(const vp_hid_route_t route) {
    switch (route) {
        case VP_HID_ROUTE_BLE:
            return ble_mouse_route_ready();
        case VP_HID_ROUTE_USB:
            return usb_mouse_route_ready();
        case VP_HID_ROUTE_DONGLE_2G4:
            return dongle_mouse_route_ready();
        default:
            return 0u;
    }
}

vp_hid_send_status_t c_vp_hid_send_mouse(const vp_hid_route_t route,
                                         const uint8_t buttons, const int8_t dx,
                                         const int8_t dy, const int8_t wheel) {
    mouse_report_t rpt;
    rpt.buttons = buttons;
    rpt.dx = clamp_i8_to_hid_range(dx);
    rpt.dy = clamp_i8_to_hid_range(dy);
    rpt.wheel = clamp_i8_to_hid_range(wheel);

    switch (route) {
        case VP_HID_ROUTE_BLE:
            return ble_send_mouse_report(&rpt);
        case VP_HID_ROUTE_USB:
            return usb_send_mouse_report(&rpt);
        case VP_HID_ROUTE_DONGLE_2G4:
            return dongle_send_mouse_report(&rpt);
        default:
            return VP_HID_SEND_NOT_CONNECTED;
    }
}

vp_hid_send_status_t c_vp_hid_send_vendor(const vp_hid_route_t route,
                                          const uint8_t*       ptr,
                                          const uint16_t       len) {
    switch (route) {
        case VP_HID_ROUTE_USB:
            return usb_send_vendor_report(ptr, len);
        case VP_HID_ROUTE_BLE:
        case VP_HID_ROUTE_DONGLE_2G4:
        default:
            return VP_HID_SEND_NOT_CONNECTED;
    }
}

vp_status_t c_vp_hid_route_enable(const vp_hid_route_t route,
                                  const vp_bool_t      enabled) {
    (void)route;
    (void)enabled;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_hid_route_reset(const vp_hid_route_t route) {
    (void)route;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_prepare_suspend(void) {
    VP_LOG_WARN("platform",
                "feature unavailable;feature=power_prepare_suspend");
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_enter_suspend(void) {
    VP_LOG_WARN("platform", "feature unavailable;feature=power_enter_suspend");
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_prepare_sleep(void) {
    VP_LOG_WARN("platform", "feature unavailable;feature=power_prepare_sleep");
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_enter_sleep(void) {
    VP_LOG_WARN("platform", "feature unavailable;feature=power_enter_sleep");
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_restore_from_sleep(void) {
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_wake_source_enable(const vp_wake_source_t source,
                                    const vp_bool_t        enabled) {
    (void)source;
    (void)enabled;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_flash_config_region(vp_flash_region_t* out_info) {
    if (out_info == NULL) {
        return VP_STATUS_INVALID_ARG;
    }
    out_info->offset = 0u;
    out_info->length = 0u;
    out_info->page_size = 0u;
    out_info->write_alignment = 0u;
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_flash_read(const uint32_t offset, uint8_t* ptr,
                            const uint32_t len) {
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
