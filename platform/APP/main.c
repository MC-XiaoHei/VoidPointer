#include "CONFIG.h"
#include "HAL.h"
#include "main.h"
#include "hiddev.h"
#include "ble_hid_app.h"
#include "lsm6dsv.h"
#include "rust_api.h"
#include "c_api.h"
#include "usbhs_hid_device.h"
#include "board_map.h"
#include "vp_hal.h"
#include "board_input.h"
#include "imu_platform.h"
#include "led_platform.h"
#include "pwm_platform.h"

#include <math.h>

__attribute__((aligned(4))) uint32_t MEM_BUF[BLE_MEMHEAP_SIZE / 4];

#if (defined(BLE_MAC)) && (BLE_MAC == TRUE)
const uint8_t MacAddr[6] = {0x4F, 0x9D, 0x2A, 0x8B, 0xC1, 0x7E};
#endif

static void     core_service(void);
static uint16_t core_process_event(uint8_t task_id, uint16_t events);

#define CORE_POLL_EVT 0x0001

__HIGH_CODE
__attribute__((noinline)) void main_loop() {
    while (1) {
        core_service();
        TMOS_SystemProcess();
        core_service();
    }
}

static tmosTaskID       runtime_task_id = 0xFF;
static volatile uint8_t runtime_poll_request_pending = 0u;
static volatile uint8_t runtime_debounce_timer_running = 0u;
static uint32_t         runtime_debounce_next_ms = 0u;

static void service_latched_irqs(void) {
    if (runtime_debounce_timer_running) {
        return;
    }

    if (board_input_service_pending_all()) {
        runtime_poll_request_pending = 1u;
    }
}

static void service_debounce(void) {
    if (!runtime_debounce_timer_running) {
        return;
    }

    const uint32_t now = c_vp_rtc_millis();
    if ((uint32_t)(now - runtime_debounce_next_ms) >= 0x80000000u) {
        return;
    }

    runtime_debounce_next_ms = now + 1u;
    vp_on_debounce_tick(now);
}

static void core_service(void) {
    if (runtime_poll_request_pending) {
        runtime_poll_request_pending = 0u;
        vp_core_poll();
    }

    service_latched_irqs();
    service_debounce();
}

static uint16_t core_process_event(uint8_t task_id, uint16_t events) {
    (void)task_id;
    if (events & CORE_POLL_EVT) {
        runtime_poll_request_pending = 1u;
        return events & (uint16_t)(~CORE_POLL_EVT);
    }

    return 0u;
}

void core_request_poll() { runtime_poll_request_pending = 1u; }

void core_request_poll_after(const uint32_t ms) {
    if (runtime_task_id == 0xFF) {
        return;
    }

    if (ms == 0u) {
        runtime_poll_request_pending = 1u;
        return;
    }

    tmos_start_task(runtime_task_id, CORE_POLL_EVT,
                    MS1_TO_SYSTEM_TIME(ms));
}

void debounce_start() {
    if (runtime_task_id == 0xFF) {
        return;
    }
    runtime_debounce_timer_running = 1u;
    runtime_debounce_next_ms = c_vp_rtc_millis() + 1u;
}

void debounce_stop() {
    runtime_debounce_timer_running = 0u;
}

void core_init() {
    runtime_task_id = TMOS_ProcessEventRegister(core_process_event);
    if (runtime_task_id == 0xFF) {
        VP_LOG_ERROR("runtime", "task registration failed");
        return;
    }
    core_request_poll();
}

void input_init_pins() {
    board_gpio_init_all();

    imu_init_pins();
}

void input_init_irq() {
    (void)c_vp_exti_set_edge(VP_INPUT_LEFT, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_RIGHT, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_MIDDLE, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_ACTION, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_ENCODER_A, VP_EXTI_EDGE_BOTH);
    (void)c_vp_exti_set_edge(VP_INPUT_ENCODER_B, VP_EXTI_EDGE_BOTH);

    imu_init_irq();
}

int main() {
#if (defined(DCDC_ENABLE)) && (DCDC_ENABLE == TRUE)
    PWR_DCDCCfg(ENABLE);
#endif

    HSECFG_Capacitance(HSECap_12p);
    LSECFG_Capacitance(LSECap_13p);
    SetSysClock(SYSCLK_FREQ);

#if (defined(HAL_SLEEP)) && (HAL_SLEEP == TRUE)
    vp_gpio_mode_cfg_mask(BOARD_GPIO_GROUP_A, GPIO_Pin_All, GPIO_ModeIN_PU);
    vp_gpio_mode_cfg_mask(BOARD_GPIO_GROUP_B, GPIO_Pin_All, GPIO_ModeIN_PU);
#endif

#ifdef DEBUG
    UART0_DefInit();
#endif

    CH58x_BLEInit();
    HAL_Init();
    input_init_pins();
    input_init_irq();

    imu_init();

    GAPRole_PeripheralInit();
    HidDev_Init();
    ble_hid_init();

    usb_hid_init();
    led_init();
    pwm_init();

    vp_core_init();
    core_init();
    vp_input_enable();

    main_loop();
}
