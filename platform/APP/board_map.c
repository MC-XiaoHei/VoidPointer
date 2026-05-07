#include "board_map.h"
#include "CH58x_common.h"  // IWYU pragma: keep
#include "main.h"

const BoardGpio board_btn_left = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_9,
};

const BoardGpio board_btn_right = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_8,
};

const BoardGpio board_btn_middle = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_5,
};

const BoardGpio board_btn_action = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_10,
};

const BoardGpio board_btn_laser = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_7,
};

const BoardGpio board_mode_switch = {
    .group = BOARD_GPIO_GROUP_NONE,
    .pin = 0u,
};

const BoardGpio board_enc_a = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_4,
};

const BoardGpio board_enc_b = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_6,
};

const BoardGpio board_imu_int1 = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_11,
};

const BoardGpio board_imu_int2 = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_12,
};

const BoardGpio board_i2c_sda = {
    .group = BOARD_GPIO_GROUP_B,
    .pin = GPIO_Pin_20,
};

const BoardGpio board_i2c_scl = {
    .group = BOARD_GPIO_GROUP_B,
    .pin = GPIO_Pin_21,
};

const BoardGpio board_debug_tx = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_14,
};

const BoardGpio board_debug_rx = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_15,
};
