#include "CONFIG.h"
#include "HAL.h"
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

void I2C_Hardware_Init(void) {
    GPIOB_ModeCfg(GPIO_Pin_12 | GPIO_Pin_13, GPIO_ModeIN_PU);
    I2C_Init(I2C_Mode_I2C, 400000, I2C_DutyCycle_16_9, I2C_Ack_Enable,
             I2C_AckAddr_7bit, 0);
}

#define RUNTIME_TICK_EVT 0x0001
#define RUNTIME_PERIOD   13
static tmosTaskID runtime_task_id = 0xFF;

// ReSharper disable once CppParameterNeverUsed
uint16_t RuntimeTask_ProcessEvent(uint8_t task_id, uint16_t events) {
    if (events & RUNTIME_TICK_EVT) {
        tick();
        return events ^ RUNTIME_TICK_EVT;
    }
    return 0;
}

void RuntimeTask_Init(void) {
    runtime_task_id = TMOS_ProcessEventRegister(RuntimeTask_ProcessEvent);
    if (runtime_task_id == 0xFF) {
        PRINT("Runtime task register failed\n");
        return;
    }
    tmos_start_reload_task(runtime_task_id, RUNTIME_TICK_EVT, RUNTIME_PERIOD);
}

int main(void) {
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
    GPIOA_SetBits(GPIO_Pin_14);
    GPIOPinRemap(ENABLE, RB_PIN_UART0);
    GPIOA_ModeCfg(GPIO_Pin_15, GPIO_ModeIN_PU);
    GPIOA_ModeCfg(GPIO_Pin_14, GPIO_ModeOut_PP_5mA);
    UART0_DefInit();
#endif

    PRINT("%s\n", (const char *)VER_LIB);

    CH58x_BLEInit();
    HAL_Init();

    I2C_Hardware_Init();
    if (!LSM6DSV_Init()) PRINT("IMU init failed\n");

    GAPRole_PeripheralInit();
    HidDev_Init();
    HidEmu_Init();

    init_core();
    RuntimeTask_Init();

    Main_Circulation();
}