/********************************** (C) COPYRIGHT *******************************
 * File Name          : c_api.c
 * Description        : C platform bindings for Rust core
 *******************************************************************************/
#include "c_api.h"

#include "HAL.h"  // IWYU pragma: keep
#include "CH58x_common.h"  // IWYU pragma: keep
#include <CH58xBLE_LIB.h>
#include "CH58x_gpio.h"
#include "CH58x_pwr.h"

#include "main.h"
#include "rust_api.h"
#include "led_platform.h"
#include "pwm_platform.h"
#include <hiddev.h>
#include <hidmouseservice.h>
#include <stdio.h>
#include "ble_hid_app.h"
#include "usbhs_hid_device.h"
#include "lsm6dsv.h"
#include "board_map.h"
#include "board_gpio.h"
#include "board_input.h"
#include "imu_platform.h"

typedef struct {
    uint8_t buttons;
    int8_t  dx;
    int8_t  dy;
    int8_t  wheel;
} mouse_report_t;

#define VP_CONFIG_REGION_SIZE_BYTES (EEPROM_BLOCK_SIZE * 2u)
#define VP_CONFIG_REGION_END_ADDR   ((uint32_t)BLE_SNV_ADDR)
#define VP_CONFIG_REGION_BASE_ADDR  (VP_CONFIG_REGION_END_ADDR - VP_CONFIG_REGION_SIZE_BYTES)

static vp_bool_t flash_config_range_valid(const uint32_t offset,
                                          const uint32_t len) {
    if (offset > VP_CONFIG_REGION_SIZE_BYTES) {
        return 0u;
    }

    return len <= (VP_CONFIG_REGION_SIZE_BYTES - offset) ? 1u : 0u;
}

static vp_bool_t      debounce_timer_running = 0u;
static vp_usb_state_t current_usb_state = VP_USB_STATE_DETACHED;
static vp_wake_source_t enabled_wake_sources = 0u;

static vp_status_t configure_input_wake_source(const vp_input_id_t  input_id,
                                               const vp_exti_edge_t edge) {
    const vp_status_t edge_status = c_vp_exti_set_edge(input_id, edge);
    if (edge_status != VP_STATUS_OK) {
        return edge_status;
    }

    const vp_status_t clear_status = c_vp_exti_clear_pending(input_id);
    if (clear_status != VP_STATUS_OK) {
        return clear_status;
    }

    return c_vp_exti_unmask(input_id);
}

static void sync_platform_wake_sources(void) {
    const vp_bool_t gpio_wake_enabled =
        (enabled_wake_sources &
         (VP_WAKE_SOURCE_BUTTON | VP_WAKE_SOURCE_ENCODER | VP_WAKE_SOURCE_IMU)) != 0u;

    PWR_PeriphWakeUpCfg(gpio_wake_enabled ? ENABLE : DISABLE,
                        RB_SLP_GPIO_WAKE, Short_Delay);
    // 当前 suspend 仍未进入真正 deep-sleep，先不打开全局 GPIO any-edge wake，
    // 避免未来接 Halt 前无意放宽 button/IMU 的边沿语义。
    PWR_PeriphWakeUpCfg(DISABLE, RB_GPIO_EDGE_WAKE, Short_Delay);

    // USB configured 时项目语义必须保持 Active，
    // 当前不把 USB 作为 Suspend wake-source 主线的一部分。
    PWR_PeriphWakeUpCfg(DISABLE, RB_SLP_USB2_WAKE, Short_Delay);
    R8_USB2_WAKE_CTRL = 0u;
}

static vp_status_t enable_button_wake_sources(void) {
    for (vp_input_id_t input_id = VP_INPUT_LEFT; input_id <= VP_INPUT_ACTION;
         input_id++) {
        const vp_status_t status =
            configure_input_wake_source(input_id, VP_EXTI_EDGE_FALLING);
        if (status != VP_STATUS_OK) {
            return status;
        }
    }

    return VP_STATUS_OK;
}

static vp_status_t enable_encoder_wake_sources(void) {
    const vp_input_id_t inputs[] = {VP_INPUT_ENCODER_A, VP_INPUT_ENCODER_B};
    for (uint8_t i = 0u; i < (uint8_t)(sizeof(inputs) / sizeof(inputs[0])); i++) {
        const vp_status_t status =
            configure_input_wake_source(inputs[i], VP_EXTI_EDGE_BOTH);
        if (status != VP_STATUS_OK) {
            return status;
        }
    }

    return VP_STATUS_OK;
}

static vp_status_t enable_imu_wake_sources(void) {
    const vp_input_id_t inputs[] = {VP_INPUT_IMU_INT1, VP_INPUT_IMU_INT2};
    for (uint8_t i = 0u; i < (uint8_t)(sizeof(inputs) / sizeof(inputs[0])); i++) {
        BoardGpio gpio = {0};
        if (!board_input_id_to_gpio(inputs[i], &gpio) || !board_gpio_is_valid(gpio)) {
            continue;
        }

        const vp_status_t status =
            configure_input_wake_source(inputs[i], VP_EXTI_EDGE_RISING);
        if (status != VP_STATUS_OK) {
            return status;
        }
    }

    return VP_STATUS_OK;
}

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

vp_status_t c_vp_i2c_init(void) { return ImuPlatform_I2cInit(); }

vp_status_t c_vp_i2c_recover_bus(void) { return ImuPlatform_I2cRecoverBus(); }

vp_status_t c_vp_i2c_abort(void) { return LSM6DSV_AbortAsync(); }

vp_status_t c_vp_imu_config_active(void) {
    return LSM6DSV_ConfigActive() ? VP_STATUS_OK : VP_STATUS_IO_ERROR;
}

vp_status_t c_vp_imu_config_suspend(void) {
    return LSM6DSV_ConfigSuspend() ? VP_STATUS_OK : VP_STATUS_IO_ERROR;
}

vp_status_t c_vp_imu_config_sleep(void) {
    return LSM6DSV_ConfigSleep() ? VP_STATUS_OK : VP_STATUS_IO_ERROR;
}

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

vp_status_t c_vp_imu_read_wake_status(vp_bool_t* out_wake_event,
                                      vp_bool_t* out_sleep_change,
                                      uint8_t*   out_raw) {
    lsm6dsv_wake_status_t status = {0};

    if (out_wake_event == NULL || out_sleep_change == NULL || out_raw == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    if (!LSM6DSV_ReadWakeStatus(&status)) {
        *out_wake_event = 0u;
        *out_sleep_change = 0u;
        *out_raw = 0u;
        return VP_STATUS_IO_ERROR;
    }

    *out_wake_event = status.wake_event;
    *out_sleep_change = status.sleep_change;
    *out_raw = status.raw;
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
                                         const int8_t dy,
                                         const int8_t wheel) {
    mouse_report_t report = {
        .buttons = buttons,
        .dx = clamp_i8_to_hid_range(dx),
        .dy = clamp_i8_to_hid_range(dy),
        .wheel = wheel,
    };

    switch (route) {
        case VP_HID_ROUTE_BLE:
            if (!ble_mouse_route_ready()) {
                return VP_HID_SEND_NOT_CONNECTED;
            }
            return ble_send_mouse_report(&report);
        case VP_HID_ROUTE_USB:
            return usb_send_mouse_report(&report);
        case VP_HID_ROUTE_DONGLE_2G4:
            return dongle_send_mouse_report(&report);
        default:
            return VP_HID_SEND_FATAL;
    }
}

vp_hid_send_status_t c_vp_hid_send_vendor(const vp_hid_route_t route,
                                          const uint8_t* ptr,
                                          const uint16_t len) {
    switch (route) {
        case VP_HID_ROUTE_USB:
            return usb_send_vendor_report(ptr, len);
        case VP_HID_ROUTE_BLE:
        case VP_HID_ROUTE_DONGLE_2G4:
            return VP_HID_SEND_NOT_CONNECTED;
        default:
            return VP_HID_SEND_FATAL;
    }
}

vp_status_t c_vp_hid_route_enable(const vp_hid_route_t route,
                                  const vp_bool_t      enabled) {
    switch (route) {
        case VP_HID_ROUTE_BLE:
            return BleHidApp_SetAdvertisingEnabled(enabled ? TRUE : FALSE)
                       ? VP_STATUS_OK
                       : VP_STATUS_IO_ERROR;
        case VP_HID_ROUTE_USB:
            return VP_STATUS_OK;
        case VP_HID_ROUTE_DONGLE_2G4:
            return VP_STATUS_UNSUPPORTED;
        default:
            return VP_STATUS_INVALID_ARG;
    }
}

vp_status_t c_vp_hid_route_reset(const vp_hid_route_t route) {
    switch (route) {
        case VP_HID_ROUTE_BLE:
            return BleHidApp_Disconnect() ? VP_STATUS_OK : VP_STATUS_IO_ERROR;
        case VP_HID_ROUTE_USB:
            USBHS_HidDevice_ResetLinkState();
            return VP_STATUS_OK;
        case VP_HID_ROUTE_DONGLE_2G4:
            return VP_STATUS_UNSUPPORTED;
        default:
            return VP_STATUS_INVALID_ARG;
    }
}

vp_status_t c_vp_power_prepare_suspend(void) {
    if (c_vp_gpio_write(VP_OUTPUT_LASER, 0u) != VP_STATUS_OK) {
        VP_LOG_WARN("power", "laser force-off failed before suspend");
    }

    if (c_vp_imu_config_suspend() != VP_STATUS_OK) {
        VP_LOG_WARN("power", "suspend prepare failed;step=imu_profile");
        return VP_STATUS_IO_ERROR;
    }

    return VP_STATUS_OK;
}

vp_status_t c_vp_power_enter_suspend(void) {
    // 当前保持项目级 Suspend：进入前已收敛恢复源与 profile，
    // 这里暂不切到芯片 deep-sleep。
    return VP_STATUS_OK;
}

vp_status_t c_vp_power_prepare_sleep(void) {
    if (c_vp_gpio_write(VP_OUTPUT_LASER, 0u) != VP_STATUS_OK) {
        VP_LOG_WARN("power", "laser force-off failed before sleep");
    }

    if (c_vp_imu_config_sleep() != VP_STATUS_OK) {
        VP_LOG_WARN("power", "sleep prepare failed;step=imu_profile");
        return VP_STATUS_IO_ERROR;
    }

    if (!BleHidApp_SetAdvertisingEnabled(FALSE)) {
        VP_LOG_WARN("power", "sleep prepare failed;step=ble_advertising_off");
        return VP_STATUS_IO_ERROR;
    }

    return VP_STATUS_OK;
}

vp_status_t c_vp_power_enter_sleep(void) {
    // 当前仍保持项目级 Sleep：进入前先切 profile 并关闭 BLE advertising，
    // 这里暂不直接切到 CH585 deep-sleep。
    return VP_STATUS_OK;
}

vp_status_t c_vp_power_restore_from_sleep(void) {
    if (current_usb_state == VP_USB_STATE_CONFIGURED) {
        return VP_STATUS_OK;
    }

    if (!BleHidApp_SetAdvertisingEnabled(TRUE)) {
        VP_LOG_WARN("power", "sleep restore failed;step=ble_advertising_on");
        return VP_STATUS_IO_ERROR;
    }

    return VP_STATUS_OK;
}

vp_status_t c_vp_wake_source_enable(const vp_wake_source_t source,
                                    const vp_bool_t        enabled) {
    if (enabled) {
        vp_status_t status = VP_STATUS_OK;

        if ((source & VP_WAKE_SOURCE_BUTTON) != 0u) {
            status = enable_button_wake_sources();
        }
        if (status == VP_STATUS_OK && (source & VP_WAKE_SOURCE_ENCODER) != 0u) {
            status = enable_encoder_wake_sources();
        }
        if (status == VP_STATUS_OK && (source & VP_WAKE_SOURCE_IMU) != 0u) {
            status = enable_imu_wake_sources();
        }
        if (status != VP_STATUS_OK) {
            return status;
        }

        enabled_wake_sources |= source;
        sync_platform_wake_sources();
        return VP_STATUS_OK;
    }

    // 当前活动态与 Suspend 共用同一套 EXTI 分发路径；
    // disable 仅清理 bookkeeping，不主动 mask 这些输入，避免破坏活动态输入链路。
    enabled_wake_sources &= ~source;
    sync_platform_wake_sources();
    return VP_STATUS_OK;
}

vp_status_t c_vp_flash_config_region(vp_flash_region_t* out_info) {
    if (out_info == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    out_info->offset = 0u;
    out_info->length = VP_CONFIG_REGION_SIZE_BYTES;
    out_info->page_size = EEPROM_MIN_ER_SIZE;
    out_info->write_alignment = 4u;
    return VP_STATUS_OK;
}

vp_status_t c_vp_flash_read(const uint32_t offset, uint8_t* ptr,
                            const uint32_t len) {
    if (ptr == NULL && len != 0u) {
        return VP_STATUS_INVALID_ARG;
    }

    if (!flash_config_range_valid(offset, len)) {
        return VP_STATUS_INVALID_ARG;
    }

    if (len == 0u) {
        return VP_STATUS_OK;
    }

    EEPROM_READ(VP_CONFIG_REGION_BASE_ADDR + offset, ptr, len);
    return VP_STATUS_OK;
}

vp_status_t c_vp_flash_erase(const uint32_t offset, const uint32_t len) {
    if (len == 0u) {
        return VP_STATUS_OK;
    }

    if (!flash_config_range_valid(offset, len)) {
        return VP_STATUS_INVALID_ARG;
    }

    if ((offset % EEPROM_BLOCK_SIZE) != 0u || (len % EEPROM_BLOCK_SIZE) != 0u) {
        return VP_STATUS_INVALID_ARG;
    }

    EEPROM_ERASE(VP_CONFIG_REGION_BASE_ADDR + offset, len);
    return VP_STATUS_OK;
}

vp_status_t c_vp_flash_write(const uint32_t offset, const uint8_t* ptr,
                             const uint32_t len) {
    if (ptr == NULL && len != 0u) {
        return VP_STATUS_INVALID_ARG;
    }

    if (!flash_config_range_valid(offset, len)) {
        return VP_STATUS_INVALID_ARG;
    }

    if (len == 0u) {
        return VP_STATUS_OK;
    }

    EEPROM_WRITE(VP_CONFIG_REGION_BASE_ADDR + offset, (void*)ptr, len);
    return VP_STATUS_OK;
}

void c_vp_print(const char* ptr, const uint16_t len) {
    if (ptr == NULL || len == 0u) {
        return;
    }

    for (uint16_t i = 0; i < len; i++) {
        printf("%c", ptr[i]);
    }
}

vp_status_t c_vp_platform_reset(const uint32_t reason) {
    (void)reason;
    SYS_ResetExecute();
    return VP_STATUS_OK;
}

void c_vp_led_play(const vp_led_id_t led_id, const uint32_t* ptr, const uint16_t len,
                   const vp_bool_t is_loop) {
    LedPlatform_Play((uint8_t)led_id, (const uint8_t*)ptr, (uint16_t)(len * 4u), is_loop);
}

void c_vp_led_stop(void) {
    LedPlatform_Stop();
}

void c_vp_pwm_set_duty(const uint8_t pwm_id, const uint8_t duty) {
    PwmPlatform_SetDuty(pwm_id, duty);
}
