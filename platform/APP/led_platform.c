#include "led_platform.h"
#include "vp_hal.h"
#include "board_map.h"

static uint8_t current_led_active = 0u;

void led_init(void) {
    BoardGpio gpio = board_signal_get(BOARD_SIGNAL_STATUS_LED);
    vp_gpio_set(gpio);
}

void led_play(const BoardSignal sig, const uint8_t* data,
                       const uint16_t byte_len, const uint8_t is_loop) {
    if (data == NULL || byte_len == 0u) {
        return;
    }

    if (current_led_active) {
        led_stop();
    }

    BoardGpio gpio = board_signal_get(sig);
    vp_gpio_mode_cfg(gpio, GPIO_ModeOut_PP_5mA);
    vp_gpio_reset(gpio);

    vp_tmr_reset(sig);
    vp_tmr_pwm_init(sig, VP_LED_PWM_CYCLE);

    vp_tmr_pwm_set_polarity(sig, board_signal_get_polarity(sig));

    const uint32_t* frames = (const uint32_t*)data;
    uint32_t len_words = byte_len / 4u;

    // DMA 启动前先写入第一帧，避免起始瞬间 FIFO=0 导致闪灭
    if (len_words > 0u) {
        vp_tmr_pwm_load_fifo(sig, frames[0]);
    }

    vp_tmr_pwm_dma_cfg(sig, (uint32_t)(uintptr_t)data,
                       (uint32_t)(uintptr_t)(data + byte_len), is_loop);

    vp_tmr_pwm_enable(sig);

    current_led_active = 1u;
}

void led_stop(void) {
    if (!current_led_active) {
        return;
    }
    // 先置高 GPIO，再停 TMR，避免 TMR 停止后 GPIO 拉低导致的闪光
    BoardGpio gpio = board_signal_get(BOARD_SIGNAL_STATUS_LED);
    vp_gpio_set(gpio);
    vp_tmr_pwm_dma_stop(BOARD_SIGNAL_STATUS_LED);
    vp_tmr_pwm_disable(BOARD_SIGNAL_STATUS_LED);
    current_led_active = 0u;
}
