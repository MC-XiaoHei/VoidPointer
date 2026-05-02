#ifndef LSM6DSV_H
#define LSM6DSV_H

#include "c_api.h"
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LSM6DSV_I2C_ADDR               (0x6A << 1)
#define LSM6DSV_I2C_MAX_TIMEOUT        100

#define LSM6DSV_REG_FUNC_CFG_ACCESS    0x01
#define LSM6DSV_REG_EMB_FUNC_EN_A      0x04
#define LSM6DSV_REG_FIFO_CTRL4         0x0A
#define LSM6DSV_REG_WHO_AM_I           0x0F
#define LSM6DSV_REG_CTRL1              0x10
#define LSM6DSV_REG_CTRL2              0x11
#define LSM6DSV_REG_CTRL3              0x12
#define LSM6DSV_REG_CTRL6              0x15
#define LSM6DSV_REG_CTRL8              0x17
#define LSM6DSV_REG_FIFO_STATUS1       0x1B
#define LSM6DSV_REG_FIFO_STATUS2       0x1C
#define LSM6DSV_REG_FUNCTIONS_ENABLE   0x50
#define LSM6DSV_REG_EMB_FUNC_FIFO_EN_A 0x44
#define LSM6DSV_REG_SFLP_ODR           0x5E
#define LSM6DSV_REG_EMB_FUNC_INIT_A    0x66
#define LSM6DSV_REG_FIFO_DATA_OUT_TAG  0x78

#define LSM6DSV_WHOAMI_VALUE           0x70
#define LSM6DSV_FIFO_TAG_SFLP_GAME     0x13

bool LSM6DSV_Init(void);
bool LSM6DSV_ReadSFLPGameRotationRaw(sflp_game_rotation_raw_t* raw);

#ifdef __cplusplus
}
#endif

#endif
