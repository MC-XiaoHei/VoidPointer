#include "CONFIG.h"
#include "HAL.h"
#include "main.h"
#include "hiddev.h"
#include "hidmouse.h"
#include "lsm6dsv.h"
#include "rust_api.h"
#include "c_api.h"

#include <math.h>

__attribute__((aligned(4))) uint32_t MEM_BUF[BLE_MEMHEAP_SIZE / 4];

#if (defined(BLE_MAC)) && (BLE_MAC == TRUE)
const uint8_t MacAddr[6] = {0x4F, 0x9D, 0x2A, 0x8B, 0xC1, 0x7E};
#endif

static void RuntimeTask_Service(void);
extern void GPIOA_IRQHandler(void);

__HIGH_CODE
__attribute__((noinline)) void Main_Circulation() {
    // ReSharper disable once CppDFAEndlessLoop
    while (1) {
        RuntimeTask_Service();
        TMOS_SystemProcess();
        RuntimeTask_Service();
    }
}

void I2C_Hardware_Init() {
    GPIOB_ModeCfg(I2C_SDA | I2C_SCL, GPIO_ModeIN_PU);
    I2C_Init(I2C_Mode_I2C, 400000, I2C_DutyCycle_16_9, I2C_Ack_Enable,
             I2C_AckAddr_7bit, 0);
}

#define RUNTIME_CORE_POLL_EVT 0x0001
#define INPUT_SCAN_EVT        0x0001

#ifndef VP_DEBUG_INPUT_SCAN
#define VP_DEBUG_INPUT_SCAN 0
#endif

#ifndef VP_DEBUG_DEBOUNCE_SERVICE
#define VP_DEBUG_DEBOUNCE_SERVICE 0
#endif

#ifndef VP_DEBUG_GPIOA_PENDING_SERVICE
#define VP_DEBUG_GPIOA_PENDING_SERVICE 0
#endif

static tmosTaskID runtime_task_id = 0xFF;
static volatile uint8_t runtime_poll_request_pending = 0u;
static volatile uint8_t runtime_debounce_timer_running = 0u;
static uint32_t runtime_debounce_next_ms = 0u;
static uint16_t runtime_button_poll_last = 0u;
static uint8_t runtime_button_poll_initialized = 0u;
static tmosTaskID input_scan_task_id = 0xFF;

static uint16_t RuntimeTask_ReadButtonPollSnapshot(void) {
    const uint32_t port = GPIOA_ReadPort();
    uint16_t snapshot = 0u;

    if ((port & LEFT_BTN) == 0u) snapshot |= (1u << VP_BUTTON_LEFT);
    if ((port & RIGHT_BTN) == 0u) snapshot |= (1u << VP_BUTTON_RIGHT);
    if ((port & MIDDLE_BTN) == 0u) snapshot |= (1u << VP_BUTTON_MIDDLE);
    if ((port & ACTION_BTN) == 0u) snapshot |= (1u << VP_BUTTON_ACTION);

    return snapshot;
}

static void RuntimeTask_ServiceButtonPoll(void) {
    const uint16_t snapshot = RuntimeTask_ReadButtonPollSnapshot();
    if (!runtime_button_poll_initialized) {
        runtime_button_poll_last = snapshot;
        runtime_button_poll_initialized = 1u;
        return;
    }

    const uint16_t pressed = (uint16_t)(snapshot & (uint16_t)(~runtime_button_poll_last));
    runtime_button_poll_last = snapshot;

    if (pressed & (1u << VP_BUTTON_LEFT)) {
        vp_on_button_exti(VP_BUTTON_LEFT, 1u, c_vp_rtc_millis());
    }
    if (pressed & (1u << VP_BUTTON_RIGHT)) {
        vp_on_button_exti(VP_BUTTON_RIGHT, 1u, c_vp_rtc_millis());
    }
    if (pressed & (1u << VP_BUTTON_MIDDLE)) {
        vp_on_button_exti(VP_BUTTON_MIDDLE, 1u, c_vp_rtc_millis());
    }
    if (pressed & (1u << VP_BUTTON_ACTION)) {
        vp_on_button_exti(VP_BUTTON_ACTION, 1u, c_vp_rtc_millis());
    }
}

static void RuntimeTask_Service(void) {
    if (runtime_task_id == 0xFF) {
        return;
    }

    const uint16_t gpioa_pending_flags = (uint16_t)(GPIOA_ReadITFlagPort() & R16_PA_INT_EN);
    if (gpioa_pending_flags != 0u && !runtime_debounce_timer_running) {
#if VP_DEBUG_GPIOA_PENDING_SERVICE
        static uint8_t gpioa_pending_report_count = 0u;
        if (gpioa_pending_report_count < 8u) {
            PRINT("GPIOA pending service flags:%04x PA:%04lx IF:%04x EN:%04x MODE:%04x\n",
                  gpioa_pending_flags, GPIOA_ReadPort(), GPIOA_ReadITFlagPort(),
                  R16_PA_INT_EN, R16_PA_INT_MODE);
            gpioa_pending_report_count++;
        }
#endif
        GPIOA_IRQHandler();
        runtime_poll_request_pending = 1u;
    }

    if (runtime_debounce_timer_running) {
        const uint32_t now = c_vp_rtc_millis();
        if ((uint32_t)(now - runtime_debounce_next_ms) < 0x80000000u) {
            static uint8_t debug_debounce_tick_count = 0u;
            runtime_debounce_next_ms = now + 1u;
#if VP_DEBUG_DEBOUNCE_SERVICE
            if (debug_debounce_tick_count < 32u) {
                PRINT("Debounce service tick:%u now:%lu PA:%04lx IF:%04x EN:%04x MODE:%04x\n",
                      debug_debounce_tick_count, now, GPIOA_ReadPort(),
                      GPIOA_ReadITFlagPort(), R16_PA_INT_EN, R16_PA_INT_MODE);
                debug_debounce_tick_count++;
            }
#endif
            vp_on_debounce_tick(now);
        }
    }

    if (runtime_poll_request_pending) {
        runtime_poll_request_pending = 0u;
        vp_core_poll();
    }
}

// ReSharper disable once CppParameterNeverUsed
uint16_t RuntimeTask_ProcessEvent(uint8_t task_id, uint16_t events) {
    (void)task_id;
    if (events & RUNTIME_CORE_POLL_EVT) {
        runtime_poll_request_pending = 1u;
        return events & (uint16_t)(~RUNTIME_CORE_POLL_EVT);
    }

    return 0;
}

void RuntimeTask_RequestPoll(void) {
    if (runtime_task_id != 0xFF) {
        runtime_poll_request_pending = 1u;
    }
}

void RuntimeTask_RequestPollAfter(uint32_t ms) {
    if (runtime_task_id == 0xFF) {
        return;
    }

    if (ms == 0u) {
        runtime_poll_request_pending = 1u;
        return;
    }

    tmos_start_task(runtime_task_id, RUNTIME_CORE_POLL_EVT, MS1_TO_SYSTEM_TIME(ms));
}

void RuntimeTask_StartDebounceTimer(void) {
    if (runtime_task_id == 0xFF) {
        PRINT("Runtime debounce start ignored: no task\n");
        return;
    }
#if VP_DEBUG_DEBOUNCE_SERVICE
    PRINT("Runtime debounce start PA:%04lx IF:%04x EN:%04x MODE:%04x\n",
          GPIOA_ReadPort(), GPIOA_ReadITFlagPort(), R16_PA_INT_EN, R16_PA_INT_MODE);
#endif
    runtime_debounce_timer_running = 1u;
    runtime_debounce_next_ms = c_vp_rtc_millis() + 1u;
}

void RuntimeTask_StopDebounceTimer(void) {
#if VP_DEBUG_DEBOUNCE_SERVICE
    PRINT("Runtime debounce stop PA:%04lx IF:%04x EN:%04x MODE:%04x\n",
          GPIOA_ReadPort(), GPIOA_ReadITFlagPort(), R16_PA_INT_EN, R16_PA_INT_MODE);
#endif
    runtime_debounce_timer_running = 0u;
}

void RuntimeTask_Init() {
    runtime_task_id = TMOS_ProcessEventRegister(RuntimeTask_ProcessEvent);
    if (runtime_task_id == 0xFF) {
        PRINT("Runtime task register failed\n");
        return;
    }
    RuntimeTask_RequestPoll();
}

#if VP_DEBUG_INPUT_SCAN
static uint16_t DebugInputScan_ReadRawActiveLow(void) {
    const uint32_t port = GPIOA_ReadPort();
    uint16_t       snapshot = 0u;

    if ((port & LEFT_BTN) == 0u) snapshot |= (1u << VP_INPUT_LEFT);
    if ((port & RIGHT_BTN) == 0u) snapshot |= (1u << VP_INPUT_RIGHT);
    if ((port & MIDDLE_BTN) == 0u) snapshot |= (1u << VP_INPUT_MIDDLE);
    if ((port & ACTION_BTN) == 0u) snapshot |= (1u << VP_INPUT_ACTION);
    if ((port & ENC_A) == 0u) snapshot |= (1u << VP_INPUT_ENCODER_A);
    if ((port & ENC_B) == 0u) snapshot |= (1u << VP_INPUT_ENCODER_B);

    return snapshot;
}

static void DebugInputScan_Print(const char* reason, const uint16_t snapshot) {
    PRINT("Input scan %s raw:%04x L:%u R:%u M:%u A:%u EA:%u EB:%u PA:%04lx IF:%04x EN:%04x\n",
          reason, snapshot,
          (snapshot & (1u << VP_INPUT_LEFT)) != 0u,
          (snapshot & (1u << VP_INPUT_RIGHT)) != 0u,
          (snapshot & (1u << VP_INPUT_MIDDLE)) != 0u,
          (snapshot & (1u << VP_INPUT_ACTION)) != 0u,
          (snapshot & (1u << VP_INPUT_ENCODER_A)) != 0u,
          (snapshot & (1u << VP_INPUT_ENCODER_B)) != 0u,
          GPIOA_ReadPort(), GPIOA_ReadITFlagPort(), R16_PA_INT_EN);
}

static uint16_t InputScanTask_ProcessEvent(uint8_t task_id, uint16_t events) {
    (void)task_id;

    if (events & INPUT_SCAN_EVT) {
        static uint16_t last_snapshot = 0xFFFFu;
        static uint8_t  heartbeat = 0u;

        const uint16_t snapshot = DebugInputScan_ReadRawActiveLow();
        if (snapshot != last_snapshot) {
            DebugInputScan_Print("changed", snapshot);
            last_snapshot = snapshot;
        } else if (heartbeat++ >= 20u) {
            DebugInputScan_Print("heartbeat", snapshot);
            heartbeat = 0u;
        }

        tmos_start_task(input_scan_task_id, INPUT_SCAN_EVT, MS1_TO_SYSTEM_TIME(50));
        return events & (uint16_t)(~INPUT_SCAN_EVT);
    }

    return 0;
}

static void DebugInputScan_Init(void) {
    input_scan_task_id = TMOS_ProcessEventRegister(InputScanTask_ProcessEvent);
    if (input_scan_task_id == 0xFF) {
        PRINT("Input scan task register failed\n");
        return;
    }

    DebugInputScan_Print("initial", DebugInputScan_ReadRawActiveLow());
    tmos_start_task(input_scan_task_id, INPUT_SCAN_EVT, MS1_TO_SYSTEM_TIME(50));
}
#else
static void DebugInputScan_Init(void) {}
#endif

void InputGPIO_Init() {
    const uint32_t target_pins = RIGHT_BTN | LEFT_BTN | ACTION_BTN | ENC_A |
                                 MIDDLE_BTN | ENC_B;
    GPIOADigitalCfg(ENABLE, target_pins);
    GPIOA_ModeCfg(target_pins, GPIO_ModeIN_PU);
}

void InputEXTI_Init() {
    (void)c_vp_exti_set_edge(VP_INPUT_LEFT, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_RIGHT, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_MIDDLE, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_ACTION, VP_EXTI_EDGE_FALLING);
    (void)c_vp_exti_set_edge(VP_INPUT_ENCODER_A, VP_EXTI_EDGE_BOTH);
    (void)c_vp_exti_set_edge(VP_INPUT_ENCODER_B, VP_EXTI_EDGE_BOTH);
}

int main() {
#if (defined(DCDC_ENABLE)) && (DCDC_ENABLE == TRUE)
    PWR_DCDCCfg(ENABLE);
#endif

    HSECFG_Capacitance(HSECap_12p);
    LSECFG_Capacitance(LSECap_13p);
    SetSysClock(SYSCLK_FREQ);

#if (defined(HAL_SLEEP)) && (HAL_SLEEP == TRUE)
    GPIOA_ModeCfg(GPIO_Pin_All, GPIO_ModeIN_PU);
    GPIOB_ModeCfg(GPIO_Pin_All, GPIO_ModeIN_PU);
#endif

#ifdef DEBUG
    GPIOA_SetBits(DEBUG_TX);
    GPIOPinRemap(ENABLE, RB_PIN_UART0);
    GPIOA_ModeCfg(DEBUG_RX, GPIO_ModeIN_PU);
    GPIOA_ModeCfg(DEBUG_TX, GPIO_ModeOut_PP_5mA);
    UART0_DefInit();
#endif

    GPIOA_ModeCfg(GPIO_Pin_0, GPIO_ModeOut_PP_5mA);
    GPIOA_ModeCfg(GPIO_Pin_1, GPIO_ModeOut_PP_5mA);

    GPIOA_ResetBits(GPIO_Pin_0);
    GPIOA_ResetBits(GPIO_Pin_1);

    PRINT("%s\n", (const char*)VER_LIB);

    CH58x_BLEInit();
    HAL_Init();
    InputGPIO_Init();
    DebugInputScan_Init();

    I2C_Hardware_Init();
    if (!LSM6DSV_Init()) PRINT("IMU init failed\n");

    GAPRole_PeripheralInit();
    HidDev_Init();
    HidEmu_Init();

    vp_core_init();
    RuntimeTask_Init();
    InputEXTI_Init();

    Main_Circulation();
}
