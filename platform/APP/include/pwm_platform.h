#ifndef VOIDPOINTER_PWM_PLATFORM_H
#define VOIDPOINTER_PWM_PLATFORM_H

#include <stdint.h>
#include "board_map.h"

#ifdef __cplusplus
extern "C" {
#endif

void pwm_init();
void pwm_set_duty(BoardSignal sig, uint8_t duty);

#ifdef __cplusplus
}
#endif

#endif
