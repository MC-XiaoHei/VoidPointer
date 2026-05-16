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
#define LSM6DSV_REG_EMB_FUNC_FIFO_EN_A 0x44
#define LSM6DSV_REG_WAKE_UP_SRC        0x45
#define LSM6DSV_REG_FUNCTIONS_ENABLE   0x50
#define LSM6DSV_REG_INACTIVITY_DUR     0x54
#define LSM6DSV_REG_INACTIVITY_THS     0x55
#define LSM6DSV_REG_WAKE_UP_THS        0x5B
#define LSM6DSV_REG_WAKE_UP_DUR        0x5C
/* 0x5E 在不同 bank 下复用，写 SFLP_ODR 前必须先切到 embedded function bank */
#define LSM6DSV_REG_SFLP_ODR           0x5E
#define LSM6DSV_REG_MD1_CFG            0x5E
#define LSM6DSV_REG_MD2_CFG            0x5F
#define LSM6DSV_REG_EMB_FUNC_INIT_A    0x66
#define LSM6DSV_REG_FIFO_DATA_OUT_TAG  0x78

#define LSM6DSV_WHOAMI_VALUE           0x70
#define LSM6DSV_FIFO_TAG_SFLP_GAME     0x13

typedef struct {
    vp_bool_t wake_event;
    vp_bool_t sleep_change;
    uint8_t   raw;
} lsm6dsv_wake_status_t;

bool lsm6dsv_init();
bool lsm6dsv_set_active();
bool lsm6dsv_set_suspend();
bool lsm6dsv_set_sleep();
bool lsm6dsv_read_id(uint8_t* out_id);
bool lsm6dsv_read_wake_status(lsm6dsv_wake_status_t* out_status);
bool lsm6dsv_read_latest_rotation(sflp_game_rotation_raw_t* raw,
                                  uint16_t                  max_samples,
                                  uint16_t*                 out_dropped_count);
bool lsm6dsv_read_rotation(sflp_game_rotation_raw_t* raw);

void        lsm6dsv_async_init();
void        lsm6dsv_reinit_async();
vp_status_t lsm6dsv_start_async_read(uint16_t max_samples);
vp_status_t lsm6dsv_abort_async();
vp_bool_t   lsm6dsv_is_async_busy();

#ifdef __cplusplus
}
#endif

#endif
