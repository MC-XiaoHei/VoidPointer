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
extern void GPIOA_ServicePendingInterrupts(void);

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

static tmosTaskID runtime_task_id = 0xFF;
static volatile uint8_t runtime_poll_request_pending = 0u;
static volatile uint8_t runtime_debounce_timer_running = 0u;
static uint32_t runtime_debounce_next_ms = 0u;

static void ServiceLatchedGpioAInterrupts(void) {
    const uint16_t pending_flags = (uint16_t)(GPIOA_ReadITFlagPort() & R16_PA_INT_EN);
    if (pending_flags == 0u || runtime_debounce_timer_running) {
        return;
    }

    GPIOA_ServicePendingInterrupts();
    runtime_poll_request_pending = 1u;
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

static void ServiceRequestedRuntimePoll(void) {
    if (!runtime_poll_request_pending) {
        return;
    }

    runtime_poll_request_pending = 0u;
    vp_core_poll();
}

static void RuntimeTask_Service(void) {
    if (runtime_task_id == 0xFF) {
        return;
    }

    ServiceLatchedGpioAInterrupts();
    ServiceDebounceTimer();
    ServiceRequestedRuntimePoll();
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
    runtime_debounce_timer_running = 1u;
    runtime_debounce_next_ms = c_vp_rtc_millis() + 1u;
}

void RuntimeTask_StopDebounceTimer(void) {
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
