#include "imu_platform.h"

#include "CH58x_common.h"  // IWYU pragma: keep
#include "board_gpio.h"
#include "board_map.h"
#include "c_api.h"
#include "lsm6dsv.h"

static const vp_exti_edge_t VP_IMU_INT_EDGE = VP_EXTI_EDGE_RISING;

static void I2C_Hardware_Init(void) {
    board_gpio_digital_cfg(board_i2c_sda, ENABLE);
    board_gpio_digital_cfg(board_i2c_scl, ENABLE);
    GPIOPinRemap(ENABLE, RB_PIN_I2C);
    GPIOAGPPCfg(DISABLE, RB_PIN_USB2_EN);
    board_gpio_mode_cfg(board_i2c_sda, GPIO_ModeIN_PU);
    board_gpio_mode_cfg(board_i2c_scl, GPIO_ModeIN_PU);
    I2C_Init(I2C_Mode_I2C, 400000, I2C_DutyCycle_16_9, I2C_Ack_Enable,
             I2C_AckAddr_7bit, 0);
}

void ImuPlatform_InitGpio(void) {
    if (board_gpio_is_valid(board_imu_int1)) {
        board_gpio_digital_cfg(board_imu_int1, ENABLE);
        board_gpio_mode_cfg(board_imu_int1, GPIO_ModeIN_PU);
    }

    if (board_gpio_is_valid(board_imu_int2)) {
        board_gpio_digital_cfg(board_imu_int2, ENABLE);
        board_gpio_mode_cfg(board_imu_int2, GPIO_ModeIN_PU);
    }
}

void ImuPlatform_InitExti(void) {
    if (board_gpio_is_valid(board_imu_int1)) {
        (void)c_vp_exti_set_edge(VP_INPUT_IMU_INT1, VP_IMU_INT_EDGE);
    }

    if (board_gpio_is_valid(board_imu_int2)) {
        (void)c_vp_exti_set_edge(VP_INPUT_IMU_INT2, VP_IMU_INT_EDGE);
    }
}

void ImuPlatform_InitDevice(void) {
    I2C_Hardware_Init();
    if (!LSM6DSV_Init()) {
        VP_LOG_ERROR("main", "imu initialization failed");
    } else {
        VP_LOG_INFO("main", "imu initialization ok");
    }
}
