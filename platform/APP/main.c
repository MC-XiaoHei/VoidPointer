/********************************** (C) COPYRIGHT *******************************
 * File Name          : main.c
 * Author             : WCH
 * Version            : V1.0
 * Date               : 2018/12/10
 * Description        : 主程序，完成系统初始化后进入 TMOS/BLE 主循环
 *********************************************************************************
 * Copyright (c) 2021 Nanjing Qinheng Microelectronics Co., Ltd.
 * Attention: This software (modified or not) and binary are used for
 * microcontroller manufactured by Nanjing Qinheng Microelectronics.
 *******************************************************************************/

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
#include "board_gpio.h"
#include "board_input.h"
#include "imu_platform.h"

#include <math.h>

__attribute__((aligned(4))) uint32_t MEM_BUF[BLE_MEMHEAP_SIZE / 4];

#if (defined(BLE_MAC)) && (BLE_MAC == TRUE)
const uint8_t MacAddr[6] = {0x4F, 0x9D, 0x2A, 0x8B, 0xC1, 0x7E};
#endif

static void     RuntimeTask_Service(void);
static uint16_t RuntimeTask_ProcessEvent(uint8_t task_id, uint16_t events);

#define RUNTIME_CORE_POLL_EVT 0x0001

__HIGH_CODE
__attribute__((noinline)) void Main_Circulation() {
    while (1) {
        RuntimeTask_Service();
        TMOS_SystemProcess();
        RuntimeTask_Service();
    }
}

static tmosTaskID       runtime_task_id = 0xFF;
static volatile uint8_t runtime_poll_request_pending = 0u;
static volatile uint8_t runtime_debounce_timer_running = 0u;
static uint32_t         runtime_debounce_next_ms = 0u;

static void ServiceLatchedInputInterrupts(void) {
    if (runtime_debounce_timer_running) {
        return;
    }

    if (board_input_service_pending_all()) {
        runtime_poll_request_pending = 1u;
    }
}

static void ServiceDebounceTimer(void) {
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

static void RuntimeTask_Service(void) {
    if (runtime_poll_request_pending) {
        runtime_poll_request_pending = 0u;
        vp_core_poll();
    }

    ServiceLatchedInputInterrupts();
    ServiceDebounceTimer();
}

static uint16_t RuntimeTask_ProcessEvent(uint8_t task_id, uint16_t events) {
    (void)task_id;
    if (events & RUNTIME_CORE_POLL_EVT) {
        runtime_poll_request_pending = 1u;
        return events & (uint16_t)(~RUNTIME_CORE_POLL_EVT);
    }

    return 0u;
}

void RuntimeTask_RequestPoll(void) { runtime_poll_request_pending = 1u; }

void RuntimeTask_RequestPollAfter(const uint32_t ms) {
    if (runtime_task_id == 0xFF) {
        return;
    }

    if (ms == 0u) {
        runtime_poll_request_pending = 1u;
        return;
    }

    tmos_start_task(runtime_task_id, RUNTIME_CORE_POLL_EVT,
                    MS1_TO_SYSTEM_TIME(ms));
}

void RuntimeTask_StartDebounceTimer(void) {
    if (runtime_task_id == 0xFF) {
        return;
    }
    runtime_debounce_timer_running = 1u;
    runtime_debounce_next_ms = c_vp_rtc_millis() + 1u;
}

void RuntimeTask_StopDebounceTimer(void) {
    runtime_debounce_timer_running = 0u;
}

void RuntimeTask_Init() {
    runtime_task_id = TMOS_ProcessEventRegister(RuntimeTask_ProcessEvent);
    if (runtime_task_id == 0xFF) {
        VP_LOG_ERROR("runtime", "task registration failed");
        return;
    }
    RuntimeTask_RequestPoll();
}

void InputGPIO_Init() {
    board_gpio_digital_cfg(board_btn_right, ENABLE);
    board_gpio_digital_cfg(board_btn_left, ENABLE);
    board_gpio_digital_cfg(board_btn_action, ENABLE);
    board_gpio_digital_cfg(board_enc_a, ENABLE);
    board_gpio_digital_cfg(board_btn_middle, ENABLE);
    board_gpio_digital_cfg(board_enc_b, ENABLE);

    board_gpio_mode_cfg(board_btn_right, GPIO_ModeIN_PU);
    board_gpio_mode_cfg(board_btn_left, GPIO_ModeIN_PU);
    board_gpio_mode_cfg(board_btn_action, GPIO_ModeIN_PU);
    board_gpio_mode_cfg(board_enc_a, GPIO_ModeIN_PU);
    board_gpio_mode_cfg(board_btn_middle, GPIO_ModeIN_PU);
    board_gpio_mode_cfg(board_enc_b, GPIO_ModeIN_PU);

    ImuPlatform_InitGpio();
}

void InputEXTI_Init() {
    (void)c_vp_exti_set_edge(VP_INPUT_LEFT, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_RIGHT, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_MIDDLE, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_ACTION, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_ENCODER_A, VP_EXTI_EDGE_BOTH);
    (void)c_vp_exti_set_edge(VP_INPUT_ENCODER_B, VP_EXTI_EDGE_BOTH);

    ImuPlatform_InitExti();
}

int main() {
#if (defined(DCDC_ENABLE)) && (DCDC_ENABLE == TRUE)
    PWR_DCDCCfg(ENABLE);
#endif

    HSECFG_Capacitance(HSECap_12p);
    LSECFG_Capacitance(LSECap_13p);
    SetSysClock(SYSCLK_FREQ);

#if (defined(HAL_SLEEP)) && (HAL_SLEEP == TRUE)
    board_gpio_mode_cfg_mask(BOARD_GPIO_GROUP_A, GPIO_Pin_All, GPIO_ModeIN_PU);
    board_gpio_mode_cfg_mask(BOARD_GPIO_GROUP_B, GPIO_Pin_All, GPIO_ModeIN_PU);
#endif

#ifdef DEBUG
    board_gpio_set(board_debug_tx);
    board_gpio_mode_cfg(board_debug_tx, GPIO_ModeOut_PP_5mA);
    GPIOPinRemap(ENABLE, RB_PIN_UART0);
    board_gpio_mode_cfg(board_debug_rx, GPIO_ModeIN_PU);
    UART0_DefInit();
#endif

    board_gpio_mode_cfg(
        (BoardGpio){.group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_0},
        GPIO_ModeOut_PP_5mA);
    board_gpio_mode_cfg(
        (BoardGpio){.group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_1},
        GPIO_ModeOut_PP_5mA);

    board_gpio_reset(
        (BoardGpio){.group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_0});
    board_gpio_reset(
        (BoardGpio){.group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_1});

    CH58x_BLEInit();
    HAL_Init();
    InputGPIO_Init();
    InputEXTI_Init();

    ImuPlatform_InitDevice();

    GAPRole_PeripheralInit();
    HidDev_Init();
    BleHidApp_Init();

    USBHS_HidDevice_Init();

    vp_core_init();
    RuntimeTask_Init();
    vp_input_enable();

    Main_Circulation();
}
