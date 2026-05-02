#include "CONFIG.h"
#include "HAL.h"
#include "main.h"
#include "hiddev.h"
#include "hidmouse.h"
#include "lsm6dsv.h"
#include "rust_api.h"

#include <math.h>

__attribute__((aligned(4))) uint32_t MEM_BUF[BLE_MEMHEAP_SIZE / 4];

#if (defined(BLE_MAC)) && (BLE_MAC == TRUE)
const uint8_t MacAddr[6] = {0x4F, 0x9D, 0x2A, 0x8B, 0xC1, 0x7E};
#endif

__HIGH_CODE
__attribute__((noinline)) void Main_Circulation() {
    // ReSharper disable once CppDFAEndlessLoop
    while (1) {
        TMOS_SystemProcess();
    }
}

void I2C_Hardware_Init() {
    GPIOB_ModeCfg(I2C_SDA | I2C_SCL, GPIO_ModeIN_PU);
    I2C_Init(I2C_Mode_I2C, 400000, I2C_DutyCycle_16_9, I2C_Ack_Enable,
             I2C_AckAddr_7bit, 0);
}

#define RUNTIME_TICK_EVT 0x0001
#define RUNTIME_PERIOD   1
static tmosTaskID runtime_task_id = 0xFF;

// ReSharper disable once CppParameterNeverUsed
uint16_t RuntimeTask_ProcessEvent(uint8_t task_id, uint16_t events) {
    if (events & RUNTIME_TICK_EVT) {
        vp_core_poll();
        // ReSharper disable once CppRedundantParentheses
        return events & (~RUNTIME_TICK_EVT);
    }
    return 0;
}

void RuntimeTask_Init() {
    runtime_task_id = TMOS_ProcessEventRegister(RuntimeTask_ProcessEvent);
    if (runtime_task_id == 0xFF) {
        PRINT("Runtime task register failed\n");
        return;
    }
    tmos_start_reload_task(runtime_task_id, RUNTIME_TICK_EVT, RUNTIME_PERIOD);
}

void InputGPIO_Init() {
    const uint32_t target_pins = LIGHT_BTN | RIGHT_BTN | LEFT_BTN | ACTION_BTN |
                                 ENC_A | MIDDLE_BTN | ENC_B;
    GPIOADigitalCfg(ENABLE, target_pins);
    GPIOA_ModeCfg(target_pins, GPIO_ModeIN_Floating);
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

    Main_Circulation();
}
