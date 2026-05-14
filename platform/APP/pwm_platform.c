// pwm_platform.c —— PWM 驱动（通过 vp_hal 路由到具体 PWM 通道）
#include "pwm_platform.h"
#include "board_map.h"
#include "vp_hal.h"

void PwmPlatform_Init(void) {
    vp_pwm_init(BOARD_SIGNAL_PWM_LASER, VP_PWMX_CYCLE_256);
}

void PwmPlatform_SetDuty(const BoardSignal sig, const uint8_t duty) {
    vp_pwm_set_duty(sig, duty);
}