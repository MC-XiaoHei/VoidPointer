// led_platform.c —— LED 驱动（通过 vp_hal 路由到具体 TMR 外设）
#include "led_platform.h"
#include "vp_hal.h"

static uint8_t current_led_active = 0u;

void LedPlatform_Init(void) {
    vp_tmr_pwm_init(BOARD_SIGNAL_LED_STATUS, VP_LED_PWM_CYCLE);
}

void LedPlatform_Play(const BoardSignal sig, const uint8_t* data,
                       const uint16_t byte_len, const uint8_t is_loop) {
    if (data == NULL || byte_len == 0u) {
        return;
    }

    if (current_led_active) {
        LedPlatform_Stop();
    }

    uint32_t start_addr = (uint32_t)(uintptr_t)data;
    uint32_t end_addr = start_addr + byte_len;

    vp_tmr_pwm_dma_cfg(sig, start_addr, end_addr, is_loop);
    vp_tmr_pwm_enable(sig);
    current_led_active = 1u;
}

void LedPlatform_Stop(void) {
    if (!current_led_active) {
        return;
    }
    vp_tmr_pwm_disable(BOARD_SIGNAL_LED_STATUS);
    vp_tmr_pwm_dma_stop(BOARD_SIGNAL_LED_STATUS);
    vp_tmr_reset(BOARD_SIGNAL_LED_STATUS);
    current_led_active = 0u;
}