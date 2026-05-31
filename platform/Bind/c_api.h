#ifndef VOIDPOINTER_C_API_H
#define VOIDPOINTER_C_API_H

#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef uint32_t vp_timestamp_t;
typedef uint8_t  vp_bool_t;

typedef struct {
    uint16_t x;
    uint16_t y;
    uint16_t z;
} sflp_game_rotation_raw_t;

typedef uint8_t vp_status_t;
enum {
    VP_STATUS_OK = 0,
    VP_STATUS_BUSY = 1,
    VP_STATUS_INVALID_ARG = 2,
    VP_STATUS_NOT_READY = 3,
    VP_STATUS_IO_ERROR = 4,
    VP_STATUS_UNSUPPORTED = 5,
};

typedef uint8_t vp_button_id_t;
enum {
    VP_BUTTON_CONTEXT = 0,
    VP_BUTTON_ACTION = 1,
    VP_BUTTON_UP = 2,
    VP_BUTTON_DOWN = 3,
    VP_BUTTON_PRIMARY = 4,
    VP_BUTTON_SECONDARY = 5,
};

typedef uint8_t vp_input_id_t;
enum {
    VP_INPUT_CONTEXT = 0,
    VP_INPUT_ACTION = 1,
    VP_INPUT_UP = 2,
    VP_INPUT_DOWN = 3,
    VP_INPUT_PRIMARY = 4,
    VP_INPUT_SECONDARY = 5,
    VP_INPUT_MODE_SWITCH = 6,
    VP_INPUT_PROFILE_SWITCH = 7,
    VP_INPUT_IMU_INT1 = 8,
    VP_INPUT_IMU_INT2 = 9,
};

typedef uint8_t vp_output_id_t;
enum {
    VP_OUTPUT_LASER = 0,
};

typedef uint8_t vp_exti_edge_t;
enum {
    VP_EXTI_EDGE_RISING = 0,
    VP_EXTI_EDGE_FALLING = 1,
    VP_EXTI_EDGE_BOTH = 2,
};

typedef uint8_t vp_hid_route_t;
enum {
    VP_HID_ROUTE_NONE = 0,
    VP_HID_ROUTE_BLE = 1,
    VP_HID_ROUTE_DONGLE_2G4 = 2,
    VP_HID_ROUTE_USB = 3,
};

typedef uint8_t vp_hid_send_status_t;
enum {
    VP_HID_SEND_SENT = 0,
    VP_HID_SEND_RETRY_LATER = 1,
    VP_HID_SEND_NOT_CONNECTED = 2,
    VP_HID_SEND_FATAL = 3,
};

typedef uint8_t vp_usb_state_t;
enum {
    VP_USB_STATE_DETACHED = 0,
    VP_USB_STATE_ATTACHED = 1,
    VP_USB_STATE_CONFIGURED = 2,
    VP_USB_STATE_SUSPENDED = 3,
    VP_USB_STATE_ERROR = 4,
};

typedef uint32_t vp_wake_source_t;

#define VP_WAKE_SOURCE_BUTTON  (1u << 0)
#define VP_WAKE_SOURCE_ENCODER (1u << 1)
#define VP_WAKE_SOURCE_IMU     (1u << 2)
#define VP_WAKE_SOURCE_USB     (1u << 3)

typedef struct {
    uint32_t offset;
    uint32_t length;
    uint32_t page_size;
    uint32_t write_alignment;
} vp_flash_region_t;

/* Rust core 导出的回调入口 */
/* 生命周期 API，只能在 bottom-half 调用 */
void vp_core_init(void);
void vp_core_poll(void);

/* 路由与连接事件，可在 ISR 调用 */
void vp_on_ble_connected(vp_timestamp_t timestamp);
void vp_on_ble_input_ready(vp_timestamp_t timestamp);
void vp_on_ble_disconnected(uint8_t reason, vp_timestamp_t timestamp);
void vp_on_dongle_connected(vp_timestamp_t timestamp);
void vp_on_dongle_disconnected(uint8_t reason, vp_timestamp_t timestamp);
void vp_on_usb_state_changed(vp_usb_state_t state, vp_timestamp_t timestamp);
void on_usb_state_change(vp_usb_state_t state);

/* 输入事件，可在 ISR 调用 */
void vp_on_button_exti(vp_button_id_t button_id, vp_bool_t level,
                       vp_timestamp_t timestamp);
void vp_on_mode_switch_exti(vp_bool_t level, vp_timestamp_t timestamp);
void vp_on_debounce_tick(vp_timestamp_t timestamp);
void vp_on_encoder_exti(vp_bool_t a_level, vp_bool_t b_level,
                        vp_timestamp_t timestamp);

/* IMU 事件，可在 ISR 调用 */
void vp_on_imu_int(vp_timestamp_t timestamp);
void vp_on_imu_sample(uint16_t raw_x, uint16_t raw_y, uint16_t raw_z,
                      vp_timestamp_t timestamp);
void vp_on_imu_fifo_done(vp_status_t status, uint16_t dropped_count,
                         vp_timestamp_t timestamp);

/* 异步回调，可在 ISR 调用 */
void vp_on_hid_send_done(vp_hid_route_t route, vp_hid_send_status_t status,
                         vp_timestamp_t timestamp);
void vp_on_vendor_report_rx(vp_hid_route_t route, const uint8_t* ptr,
                            uint16_t len, vp_timestamp_t timestamp);

/* GPIO 与 EXTI API，可在 ISR 调用 */
vp_bool_t   c_vp_gpio_read(vp_input_id_t input_id);
vp_status_t c_vp_gpio_read_inputs(uint16_t* out_snapshot);
vp_status_t c_vp_gpio_write(vp_output_id_t output_id, vp_bool_t level);
vp_status_t c_vp_exti_mask(vp_input_id_t input_id);
vp_status_t c_vp_exti_unmask(vp_input_id_t input_id);
vp_status_t c_vp_exti_clear_pending(vp_input_id_t input_id);
vp_status_t c_vp_exti_set_edge(vp_input_id_t input_id, vp_exti_edge_t edge);

/* Timer、RTC 与 TMOS API */
/* timer/rtc 读取可在 ISR 调用，poll 请求 API 只负责调度 */
vp_status_t    c_vp_debounce_timer_start(void);
vp_status_t    c_vp_debounce_timer_stop(void);
uint32_t       c_vp_rtc_tick(void);
vp_timestamp_t c_vp_rtc_millis(void);
uint32_t       c_vp_rtc_micros(void);
vp_status_t    c_vp_rtc_set_wake_after(uint32_t ms);
void           c_vp_request_core_poll(void);
void           c_vp_request_core_poll_after(uint32_t ms);

/* I2C 与 IMU API，只能在 bottom-half 调用 */
vp_status_t c_vp_i2c_init(void);
vp_status_t c_vp_i2c_recover_bus(void);
vp_status_t c_vp_i2c_abort(void);
vp_status_t c_vp_imu_config_active(void);
vp_status_t c_vp_imu_config_suspend(void);
vp_status_t c_vp_imu_config_sleep(void);
vp_status_t c_vp_imu_read_fifo_async(uint16_t max_samples);
vp_status_t c_vp_imu_read_whoami(uint8_t* out_id);
vp_status_t c_vp_imu_read_wake_status(vp_bool_t* out_wake_event,
                                      vp_bool_t* out_sleep_change,
                                      uint8_t*   out_raw);

/* HID 与 route API，只能在 bottom-half 调用 */
vp_bool_t            c_vp_hid_route_ready(vp_hid_route_t route);
vp_hid_send_status_t c_vp_hid_send_mouse(vp_hid_route_t route, uint8_t buttons,
                                         int8_t dx, int8_t dy, int8_t wheel);
vp_hid_send_status_t c_vp_hid_send_vendor(vp_hid_route_t route,
                                          const uint8_t* ptr, uint16_t len);
vp_status_t c_vp_hid_route_enable(vp_hid_route_t route, vp_bool_t enabled);
vp_status_t c_vp_hid_route_reset(vp_hid_route_t route);

/* Power API，只能在 bottom-half 调用 */
vp_status_t c_vp_power_prepare_suspend(void);
vp_status_t c_vp_power_enter_suspend(void);
vp_status_t c_vp_power_prepare_sleep(void);
vp_status_t c_vp_power_enter_sleep(void);
vp_status_t c_vp_power_restore_from_sleep(void);
vp_status_t c_vp_wake_source_enable(vp_wake_source_t source, vp_bool_t enabled);

/* DataFlash 配置存储 API，只能在 bottom-half 调用 */
vp_status_t c_vp_flash_config_region(vp_flash_region_t* out_info);
vp_status_t c_vp_flash_read(uint32_t offset, uint8_t* ptr, uint32_t len);
vp_status_t c_vp_flash_erase(uint32_t offset, uint32_t len);
vp_status_t c_vp_flash_write(uint32_t offset, const uint8_t* ptr, uint32_t len);

/* print 与 diagnostics API，只能在 bottom-half 调用 */
void        c_vp_print(const char* ptr, uint16_t len);
vp_status_t c_vp_platform_reset(uint32_t reason);

// TMR3 时钟 = SYSCLK_FREQ(60MHz)，VP_LED_PWM_CYCLE = SYSCLK / 100, 每帧 10ms
#define VP_LED_PWM_CYCLE 600000u

/* LED 控制 API，只能在 bottom-half 调用 */
void c_vp_led_play(uint8_t led_sig, const uint32_t* ptr, uint16_t len,
                   vp_bool_t is_loop);
void c_vp_led_stop(void);

void c_vp_pwm_set_duty(uint8_t pwm_sig, uint8_t duty);

#ifdef __cplusplus
}
#endif

#define VP_LOG_ERROR(module, fmt, ...) \
    printf("[ERROR] [C:%s] " fmt "\n", module, ##__VA_ARGS__)
#define VP_LOG_WARN(module, fmt, ...) \
    printf("[WARN] [C:%s] " fmt "\n", module, ##__VA_ARGS__)
#define VP_LOG_INFO(module, fmt, ...) \
    printf("[INFO] [C:%s] " fmt "\n", module, ##__VA_ARGS__)
#define VP_LOG_DEBUG(module, fmt, ...) \
    printf("[DEBUG] [C:%s] " fmt "\n", module, ##__VA_ARGS__)

#endif
