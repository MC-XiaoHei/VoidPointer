#ifndef VOIDPOINTER_IMU_PLATFORM_H
#define VOIDPOINTER_IMU_PLATFORM_H

#include "c_api.h"

#ifdef __cplusplus
extern "C" {
#endif

void        ImuPlatform_InitGpio(void);
void        ImuPlatform_InitExti(void);
void        ImuPlatform_InitDevice(void);
vp_status_t ImuPlatform_I2cInit(void);
vp_status_t ImuPlatform_I2cRecoverBus(void);
vp_bool_t   ImuPlatform_I2cBusIdle(void);

#ifdef __cplusplus
}
#endif

#endif
