#include "imu_platform.h"

#include "CH58x_common.h"  // IWYU pragma: keep
#include "board_gpio.h"
#include "board_map.h"
#include "c_api.h"
#include "lsm6dsv.h"

static const vp_exti_edge_t VP_IMU_INT_EDGE = VP_EXTI_EDGE_RISING;
static const uint8_t        VP_I2C_RECOVER_CLOCK_PULSES = 9u;

static void i2c_release_lines_to_pullup(void) {
    board_gpio_mode_cfg(board_i2c_sda, GPIO_ModeIN_PU);
    board_gpio_mode_cfg(board_i2c_scl, GPIO_ModeIN_PU);
}

static void i2c_drive_scl_low(void) {
    board_gpio_reset(board_i2c_scl);
    board_gpio_mode_cfg(board_i2c_scl, GPIO_ModeOut_PP_5mA);
}

static void i2c_drive_sda_low(void) {
    board_gpio_reset(board_i2c_sda);
    board_gpio_mode_cfg(board_i2c_sda, GPIO_ModeOut_PP_5mA);
}

static void I2C_Hardware_Init(void) {
    board_gpio_digital_cfg(board_i2c_sda, ENABLE);
    board_gpio_digital_cfg(board_i2c_scl, ENABLE);
    GPIOPinRemap(ENABLE, RB_PIN_I2C);
    GPIOAGPPCfg(DISABLE, RB_PIN_USB2_EN);
    i2c_release_lines_to_pullup();
    I2C_Cmd(DISABLE);
    I2C_SoftwareResetCmd(ENABLE);
    I2C_SoftwareResetCmd(DISABLE);
    I2C_Init(I2C_Mode_I2C, 400000, I2C_DutyCycle_16_9, I2C_Ack_Enable,
             I2C_AckAddr_7bit, 0);
    I2C_Cmd(ENABLE);
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

vp_bool_t ImuPlatform_I2cBusIdle(void) {
    if (!board_gpio_is_valid(board_i2c_sda) || !board_gpio_is_valid(board_i2c_scl)) {
        return 0u;
    }

    return (board_gpio_read_level(board_i2c_sda) &&
            board_gpio_read_level(board_i2c_scl) &&
            I2C_GetFlagStatus(I2C_FLAG_BUSY) == RESET)
               ? 1u
               : 0u;
}

vp_status_t ImuPlatform_I2cInit(void) {
    I2C_Hardware_Init();
    return ImuPlatform_I2cBusIdle() ? VP_STATUS_OK : VP_STATUS_BUSY;
}

vp_status_t ImuPlatform_I2cRecoverBus(void) {
    if (!board_gpio_is_valid(board_i2c_sda) || !board_gpio_is_valid(board_i2c_scl)) {
        return VP_STATUS_INVALID_ARG;
    }

    I2C_Cmd(DISABLE);
    I2C_ITConfig(I2C_IT_BUF, DISABLE);
    I2C_ITConfig(I2C_IT_EVT, DISABLE);
    I2C_ITConfig(I2C_IT_ERR, DISABLE);
    I2C_GenerateSTART(DISABLE);
    I2C_GenerateSTOP(ENABLE);
    I2C_AcknowledgeConfig(ENABLE);
    I2C_SoftwareResetCmd(ENABLE);
    mDelayuS(10);
    I2C_SoftwareResetCmd(DISABLE);

    board_gpio_digital_cfg(board_i2c_sda, ENABLE);
    board_gpio_digital_cfg(board_i2c_scl, ENABLE);
    i2c_release_lines_to_pullup();
    mDelayuS(10);

    for (uint8_t i = 0; i < VP_I2C_RECOVER_CLOCK_PULSES; i++) {
        if (board_gpio_read_level(board_i2c_sda)) {
            break;
        }

        i2c_drive_scl_low();
        mDelayuS(5);
        i2c_release_lines_to_pullup();
        mDelayuS(5);
    }

    if (!board_gpio_read_level(board_i2c_sda)) {
        i2c_drive_sda_low();
        mDelayuS(5);
        i2c_drive_scl_low();
        mDelayuS(5);
        i2c_release_lines_to_pullup();
        mDelayuS(5);
    }

    i2c_release_lines_to_pullup();
    mDelayuS(10);

    I2C_Hardware_Init();
    LSM6DSV_ReinitAsync();
    if (!ImuPlatform_I2cBusIdle()) {
        VP_LOG_WARN("imu", "i2c recover incomplete");
        return VP_STATUS_BUSY;
    }

    VP_LOG_INFO("imu", "i2c recovered");
    return VP_STATUS_OK;
}

void ImuPlatform_InitDevice(void) {
    const vp_status_t i2c_status = ImuPlatform_I2cInit();
    if (i2c_status != VP_STATUS_OK) {
        VP_LOG_ERROR("imu", "i2c init failed;status=%u", i2c_status);
        return;
    }

    if (!LSM6DSV_Init()) {
        VP_LOG_ERROR("imu", "init failed");
    } else {
        VP_LOG_INFO("imu", "initialized");
    }
}
