#include "pwm_platform.h"
#include "board_map.h"
#include "board_gpio.h"
#include "CH58x_common.h"
#include "c_api.h"

void PwmPlatform_Init(void) {
    board_gpio_mode_cfg(board_pwm_laser, GPIO_ModeOut_PP_5mA);
    PWMX_CycleCfg(PWMX_Cycle_256);
    PWMX_ACTOUT(CH_PWM7, 0u, High_Level, ENABLE);
}

void PwmPlatform_SetDuty(const uint8_t pwm_id, const uint8_t duty) {
    (void)pwm_id;
    PWM7_ActDataWidth(duty);
}
