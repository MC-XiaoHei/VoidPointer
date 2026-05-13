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

// TODO
// 当前板子没有物理 mode switch，所以这里保持空映射。
// 如果后续硬件版本加入该开关，需要在这里补真实 GPIO，
// 并同步接通 InputGPIO_Init/InputEXTI_Init、Rust switch debounce、route policy。
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

const BoardGpio board_led_status = {
    .group = BOARD_GPIO_GROUP_A,
    .pin = GPIO_Pin_2,
};

const BoardGpio board_pwm_laser = {
    .group = BOARD_GPIO_GROUP_B,
    .pin = GPIO_Pin_4,
};
