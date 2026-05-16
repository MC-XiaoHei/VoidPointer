#include "pwm_platform.h"
#include "board_map.h"
#include "vp_hal.h"

void pwm_init(void) {
    vp_pwm_init(BOARD_SIGNAL_LASER_LED, VP_PWMX_CYCLE_256);
    pwm_set_duty(BOARD_SIGNAL_LASER_LED, 0u);
}

void pwm_set_duty(const BoardSignal sig, const uint8_t duty) {
    vp_pwm_set_duty(sig, duty);
}