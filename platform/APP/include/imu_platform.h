#ifndef VOIDPOINTER_IMU_PLATFORM_H
#define VOIDPOINTER_IMU_PLATFORM_H

#include "c_api.h"

#ifdef __cplusplus
extern "C" {
#endif

void        imu_init_pins();
void        imu_init_irq();
void        imu_init();
vp_status_t imu_i2c_init();
vp_status_t imu_i2c_recover();
vp_bool_t   imu_i2c_is_idle();

#ifdef __cplusplus
}
#endif

#endif
