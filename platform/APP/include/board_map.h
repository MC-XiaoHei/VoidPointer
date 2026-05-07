#ifndef VOIDPOINTER_BOARD_MAP_H
#define VOIDPOINTER_BOARD_MAP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    BOARD_GPIO_GROUP_NONE = 0,
    BOARD_GPIO_GROUP_A = 1,
    BOARD_GPIO_GROUP_B = 2,
} BoardGpioGroup;

typedef struct {
    BoardGpioGroup group;
    uint32_t       pin;
} BoardGpio;

extern const BoardGpio board_btn_left;
extern const BoardGpio board_btn_right;
extern const BoardGpio board_btn_middle;
extern const BoardGpio board_btn_action;
extern const BoardGpio board_btn_laser;
extern const BoardGpio board_mode_switch;
extern const BoardGpio board_enc_a;
extern const BoardGpio board_enc_b;
extern const BoardGpio board_imu_int1;
extern const BoardGpio board_imu_int2;
extern const BoardGpio board_i2c_sda;
extern const BoardGpio board_i2c_scl;
extern const BoardGpio board_debug_tx;
extern const BoardGpio board_debug_rx;

#ifdef __cplusplus
}
#endif

#endif
