#ifndef VOIDPOINTER_IMU_PLATFORM_H
#define VOIDPOINTER_IMU_PLATFORM_H

#ifdef __cplusplus
extern "C" {
#endif

void ImuPlatform_InitGpio(void);
void ImuPlatform_InitExti(void);
void ImuPlatform_InitDevice(void);

#ifdef __cplusplus
}
#endif

#endif
