#include "led_platform.h"
#include "board_map.h"
#include "board_gpio.h"
#include "CH58x_common.h"
#include "c_api.h"

static uint8_t current_led = 0xFFu;

static void led_init_inner(const BoardGpio gpio) {
    board_gpio_mode_cfg(gpio, GPIO_ModeOut_PP_5mA);
}

void LedPlatform_Init(void) {
    led_init_inner(board_led_status);
    GPIOPinRemap(DISABLE, RB_PIN_TMR3);
    TMR3_PWMCycleCfg(VP_LED_PWM_CYCLE);
    TMR3_PWMInit(High_Level, PWM_Times_1);
}

void LedPlatform_Play(const uint8_t led_id, const uint8_t* data,
                       const uint16_t byte_len, const uint8_t is_loop) {
    if (data == NULL || byte_len == 0u) {
        return;
    }

    if (current_led != 0xFFu) {
        LedPlatform_Stop();
    }

    uint32_t start_addr = (uint32_t)(uintptr_t)data;
    uint32_t end_addr = start_addr + byte_len;
    DMAModeTypeDef dma_mode = is_loop ? Mode_LOOP : Mode_Single;

    switch (led_id) {
        case VP_LED_ID_STATUS:
            TMR3_DMACfg(ENABLE, start_addr, end_addr, dma_mode);
            TMR3_PWMEnable();
            TMR3_Enable();
            current_led = led_id;
            break;
        default:
            break;
    }
}

void LedPlatform_Stop(void) {
    TMR3_PWMDisable();
    TMR3_Disable();
    TMR3_DMACfg(DISABLE, 0u, 0u, Mode_Single);
    R8_TMR3_CTRL_MOD = RB_TMR_ALL_CLEAR;
    current_led = 0xFFu;
}
