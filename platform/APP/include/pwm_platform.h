#ifndef VOIDPOINTER_PWM_PLATFORM_H
#define VOIDPOINTER_PWM_PLATFORM_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void PwmPlatform_Init(void);
void PwmPlatform_SetDuty(uint8_t pwm_id, uint8_t duty);

#ifdef __cplusplus
}
#endif

#endif
