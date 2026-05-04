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
#include <stdio.h>
#include "usbhs_hid_device.h"
#include "lsm6dsv.h"


typedef struct {
    uint8_t buttons;
    int8_t  dx;
    int8_t  dy;
    int8_t  wheel;
} mouse_report_t;

static uint16_t gpioa_exti_both_sim_mask = 0u;
static vp_bool_t debounce_timer_running = 0u;
static vp_usb_state_t current_usb_state = VP_USB_STATE_DETACHED;

typedef struct {
    vp_input_id_t input_id;
    uint32_t      pin_mask;
} input_pin_map_t;

static const input_pin_map_t INPUT_PIN_MAP[] = {
    {VP_INPUT_LEFT, LEFT_BTN},     {VP_INPUT_RIGHT, RIGHT_BTN},
    {VP_INPUT_MIDDLE, MIDDLE_BTN}, {VP_INPUT_ACTION, ACTION_BTN},
    {VP_INPUT_ENCODER_A, ENC_A},   {VP_INPUT_ENCODER_B, ENC_B},
};

static int8_t clamp_i8_to_hid_range(const int8_t v) {
    if (v == -128) {
        return -127;
    }
    return v;
}

static vp_bool_t active_low_pin_level(uint32_t port_data, uint32_t pin_mask) {
    return (port_data & pin_mask) ? 0u : 1u;
}

static vp_bool_t input_id_to_port_a_pin(const vp_input_id_t input_id,
                                        uint32_t*           out_pin_mask) {
    if (out_pin_mask == NULL) {
        return 0u;
    }

    for (uint8_t i = 0u; i < sizeof(INPUT_PIN_MAP) / sizeof(INPUT_PIN_MAP[0]); i++) {
        if (INPUT_PIN_MAP[i].input_id == input_id) {
            *out_pin_mask = INPUT_PIN_MAP[i].pin_mask;
            return 1u;
        }
    }

    *out_pin_mask = 0u;
    return 0u;
}

static void gpioa_config_next_edge_for_pin(const uint32_t pin_mask) {
    if (GPIOA_ReadPortPin(pin_mask)) {
        GPIOA_ITModeCfg(pin_mask, GPIO_ITMode_FallEdge);
    } else {
        GPIOA_ITModeCfg(pin_mask, GPIO_ITMode_RiseEdge);
    }
}

static vp_status_t map_exti_edge_to_gpioa_mode(const vp_exti_edge_t edge,
                                               GPIOITModeTpDef*     out_mode) {
    if (out_mode == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    switch (edge) {
        case VP_EXTI_EDGE_RISING:
            *out_mode = GPIO_ITMode_RiseEdge;
            return VP_STATUS_OK;
        case VP_EXTI_EDGE_FALLING:
            *out_mode = GPIO_ITMode_FallEdge;
            return VP_STATUS_OK;
        case VP_EXTI_EDGE_BOTH:
            return VP_STATUS_UNSUPPORTED;
        default:
            return VP_STATUS_INVALID_ARG;
    }
}

static vp_bool_t is_encoder_input(const vp_input_id_t input_id) {
    return input_id == VP_INPUT_ENCODER_A || input_id == VP_INPUT_ENCODER_B;
}

static vp_bool_t input_id_to_button_id(const vp_input_id_t input_id,
                                       vp_button_id_t*     out_button_id) {
    if (out_button_id == NULL) {
        return 0u;
    }

    switch (input_id) {
        case VP_INPUT_LEFT:
            *out_button_id = VP_BUTTON_LEFT;
            return 1u;
        case VP_INPUT_RIGHT:
            *out_button_id = VP_BUTTON_RIGHT;
            return 1u;
        case VP_INPUT_MIDDLE:
            *out_button_id = VP_BUTTON_MIDDLE;
            return 1u;
        case VP_INPUT_ACTION:
            *out_button_id = VP_BUTTON_ACTION;
            return 1u;
        case VP_INPUT_LASER:
            *out_button_id = VP_BUTTON_LASER;
            return 1u;
        default:
            *out_button_id = 0u;
            return 0u;
    }
}

vp_bool_t c_vp_gpio_read(const vp_input_id_t input_id) {
    uint32_t pin_mask = 0u;
    if (!input_id_to_port_a_pin(input_id, &pin_mask)) {
        return 0u;
    }

    return active_low_pin_level(GPIOA_ReadPort(), pin_mask);
}

vp_status_t c_vp_gpio_read_inputs(uint16_t* out_snapshot) {
    if (out_snapshot == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    const uint32_t portA_data = GPIOA_ReadPort();
    uint16_t       snapshot = 0u;
    for (uint8_t input = VP_INPUT_LEFT; input <= VP_INPUT_IMU_INT2; input++) {
        uint32_t pin_mask = 0u;
        if (input_id_to_port_a_pin((vp_input_id_t)input, &pin_mask) &&
            active_low_pin_level(portA_data, pin_mask)) {
            snapshot |= (uint16_t)(1u << input);
        }
    }
    *out_snapshot = snapshot;
    return VP_STATUS_OK;
}

vp_status_t c_vp_gpio_write(const vp_output_id_t output_id,
                            const vp_bool_t      level) {
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
    uint32_t pin_mask = 0u;
    if (!input_id_to_port_a_pin(input_id, &pin_mask)) {
        return VP_STATUS_INVALID_ARG;
    }

    R16_PA_INT_EN &= (uint16_t)(~pin_mask);
    return VP_STATUS_OK;
}

vp_status_t c_vp_exti_unmask(const vp_input_id_t input_id) {
    uint32_t pin_mask = 0u;
    if (!input_id_to_port_a_pin(input_id, &pin_mask)) {
        return VP_STATUS_INVALID_ARG;
    }

    if (gpioa_exti_both_sim_mask & pin_mask) {
        GPIOA_ClearITFlagBit(GPIOA_ReadITFlagPort());
        gpioa_config_next_edge_for_pin(pin_mask);
    } else {
        R16_PA_INT_EN &= (uint16_t)(~pin_mask);
        R16_PA_INT_MODE &= (uint16_t)(~pin_mask);
        R32_PA_CLR |= pin_mask;
        GPIOA_ClearITFlagBit(GPIOA_ReadITFlagPort());
        GPIOA_ClearITFlagBit(pin_mask);
        R16_PA_INT_EN |= (uint16_t)pin_mask;
    }
    PFIC_EnableIRQ(GPIO_A_IRQn);
    return VP_STATUS_OK;
}

vp_status_t c_vp_exti_clear_pending(const vp_input_id_t input_id) {
    uint32_t pin_mask = 0u;
    if (!input_id_to_port_a_pin(input_id, &pin_mask)) {
        return VP_STATUS_INVALID_ARG;
    }

    GPIOA_ClearITFlagBit(pin_mask);
    return VP_STATUS_OK;
}

vp_status_t c_vp_exti_set_edge(const vp_input_id_t  input_id,
                               const vp_exti_edge_t edge) {
    uint32_t pin_mask = 0u;
    if (!input_id_to_port_a_pin(input_id, &pin_mask)) {
        return VP_STATUS_INVALID_ARG;
    }

    if (edge == VP_EXTI_EDGE_BOTH) {
        if (!is_encoder_input(input_id)) {
            return VP_STATUS_UNSUPPORTED;
        }
        gpioa_exti_both_sim_mask |= (uint16_t)pin_mask;
        gpioa_config_next_edge_for_pin(pin_mask);
        PFIC_EnableIRQ(GPIO_A_IRQn);
        return VP_STATUS_OK;
    }

    GPIOITModeTpDef mode;
    const vp_status_t status = map_exti_edge_to_gpioa_mode(edge, &mode);
    if (status != VP_STATUS_OK) {
        return status;
    }

    vp_button_id_t button_id;
    if (input_id_to_button_id(input_id, &button_id)) {
        // 低有效二态输入使用电平触发 GPIO 中断。Rust 用 Falling/Rising
        // 表达下一次语义转换；CH585 平台映射为 LowLevel/HighLevel，
        // 以避开机械触点上不可靠的一次性边沿锁存。
        if (edge == VP_EXTI_EDGE_FALLING) {
            mode = GPIO_ITMode_LowLevel;
        } else if (edge == VP_EXTI_EDGE_RISING) {
            mode = GPIO_ITMode_HighLevel;
        }
    }

    gpioa_exti_both_sim_mask &= (uint16_t)(~pin_mask);
    GPIOA_ITModeCfg(pin_mask, mode);
    PFIC_EnableIRQ(GPIO_A_IRQn);
    return VP_STATUS_OK;
}

void GPIOA_ServicePendingInterrupts(void) {
    // CH585 可能已经锁存 GPIOA IF，但 PFIC 不再派发新的 GPIO_A IRQ。
    // 事件来源仍然只认硬件 IF；main runtime service 发现 IF & EN 已经
    // pending 时，可以调用同一个 service 例程补处理。
    const uint16_t flags = GPIOA_ReadITFlagPort();
    const uint16_t active_flags = (uint16_t)(flags & R16_PA_INT_EN);
    if (active_flags == 0u) {
        if (flags != 0u) {
            GPIOA_ClearITFlagBit(flags);
            PFIC_ClearPendingIRQ(GPIO_A_IRQn);
        }
        return;
    }

    const uint32_t port_data = GPIOA_ReadPort();

    for (uint8_t i = 0u; i < sizeof(INPUT_PIN_MAP) / sizeof(INPUT_PIN_MAP[0]); i++) {
        const vp_input_id_t input_id = INPUT_PIN_MAP[i].input_id;
        const uint32_t pin_mask = INPUT_PIN_MAP[i].pin_mask;
        if ((active_flags & pin_mask) == 0u) {
            continue;
        }

        GPIOA_ClearITFlagBit(pin_mask);
        if (is_encoder_input(input_id)) {
            const vp_bool_t a_level = active_low_pin_level(port_data, ENC_A);
            const vp_bool_t b_level = active_low_pin_level(port_data, ENC_B);
            gpioa_config_next_edge_for_pin(pin_mask);
            vp_on_encoder_exti(a_level, b_level, c_vp_rtc_millis());
            continue;
        }

        vp_button_id_t button_id = 0u;
        if (input_id_to_button_id(input_id, &button_id)) {
            const vp_bool_t level = active_low_pin_level(port_data, pin_mask);
            (void)c_vp_exti_mask(input_id);
            vp_on_button_exti(button_id, level, c_vp_rtc_millis());
        }
    }

    PFIC_ClearPendingIRQ(GPIO_A_IRQn);
}

__INTERRUPT
__HIGH_CODE
void GPIOA_IRQHandler(void) {
    GPIOA_ServicePendingInterrupts();
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
    sflp_game_rotation_raw_t raw = {0};
    uint16_t                 dropped_count = 0u;
    const vp_timestamp_t     timestamp = c_vp_rtc_millis();

    if (!LSM6DSV_ReadLatestSFLPGameRotationRaw(&raw, max_samples, &dropped_count)) {
        vp_on_imu_fifo_done(VP_STATUS_NOT_READY, 0u, timestamp);
        return VP_STATUS_NOT_READY;
    }

    vp_on_imu_sample(raw.x, raw.y, raw.z, timestamp);
    vp_on_imu_fifo_done(VP_STATUS_OK, dropped_count, timestamp);
    return VP_STATUS_OK;
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

static vp_hid_send_status_t dongle_send_mouse_report(const mouse_report_t* rpt) {
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

    return USBHS_HidDevice_SendVendorReport(ptr, len)
               ? VP_HID_SEND_SENT
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
    PRINT("Power prepare suspend unsupported\n");
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_enter_suspend(void) {
    PRINT("Power enter suspend unsupported\n");
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_prepare_sleep(void) {
    PRINT("Power prepare sleep unsupported\n");
    return VP_STATUS_UNSUPPORTED;
}

vp_status_t c_vp_power_enter_sleep(void) {
    PRINT("Power enter sleep unsupported\n");
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

vp_status_t c_vp_flash_write(const uint32_t offset, const uint8_t* ptr,
                             const uint32_t len) {
    (void)offset;
    (void)ptr;
    (void)len;
    return VP_STATUS_UNSUPPORTED;
}

void c_vp_debug_print(const char* ptr, const uint16_t len) {
    printf("%.*s", (int)len, ptr);
}

vp_status_t c_vp_platform_reset(const uint32_t reason) {
    (void)reason;
    return VP_STATUS_UNSUPPORTED;
}
