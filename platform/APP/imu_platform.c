#include "imu_platform.h"

#include "CH58x_common.h"  // IWYU pragma: keep
#include "vp_hal.h"
#include "board_map.h"
#include "c_api.h"
#include "lsm6dsv.h"

static const vp_exti_edge_t VP_IMU_INT_EDGE = VP_EXTI_EDGE_RISING;
static const uint8_t        VP_I2C_RECOVER_CLOCK_PULSES = 9u;

static void i2c_release_lines_to_pullup(void) {
    vp_gpio_mode_cfg(board_signal_get(BOARD_SIGNAL_I2C_SDA), GPIO_ModeIN_PU);
    vp_gpio_mode_cfg(board_signal_get(BOARD_SIGNAL_I2C_SCL), GPIO_ModeIN_PU);
}

static void i2c_drive_scl_low(void) {
    vp_gpio_reset(board_signal_get(BOARD_SIGNAL_I2C_SCL));
    vp_gpio_mode_cfg(board_signal_get(BOARD_SIGNAL_I2C_SCL),
                     GPIO_ModeOut_PP_5mA);
}

static void i2c_drive_sda_low(void) {
    vp_gpio_reset(board_signal_get(BOARD_SIGNAL_I2C_SDA));
    vp_gpio_mode_cfg(board_signal_get(BOARD_SIGNAL_I2C_SDA),
                     GPIO_ModeOut_PP_5mA);
}

static void imu_i2c_hw_init(void) {
    vp_gpio_digital_cfg(board_signal_get(BOARD_SIGNAL_I2C_SDA), ENABLE);
    vp_gpio_digital_cfg(board_signal_get(BOARD_SIGNAL_I2C_SCL), ENABLE);
    GPIOAGPPCfg(DISABLE, RB_PIN_USB2_EN);
    i2c_release_lines_to_pullup();
    I2C_Cmd(DISABLE);
    I2C_SoftwareResetCmd(ENABLE);
    I2C_SoftwareResetCmd(DISABLE);
    I2C_Init(I2C_Mode_I2C, 400000, I2C_DutyCycle_16_9, I2C_Ack_Enable,
             I2C_AckAddr_7bit, 0);
    I2C_Cmd(ENABLE);
}

void imu_init_pins(void) {
    if (vp_gpio_is_valid(board_signal_get(BOARD_SIGNAL_IMU_INT1))) {
        vp_gpio_digital_cfg(board_signal_get(BOARD_SIGNAL_IMU_INT1), ENABLE);
        vp_gpio_mode_cfg(board_signal_get(BOARD_SIGNAL_IMU_INT1),
                         GPIO_ModeIN_PU);
    }

    if (vp_gpio_is_valid(board_signal_get(BOARD_SIGNAL_IMU_INT2))) {
        vp_gpio_digital_cfg(board_signal_get(BOARD_SIGNAL_IMU_INT2), ENABLE);
        vp_gpio_mode_cfg(board_signal_get(BOARD_SIGNAL_IMU_INT2),
                         GPIO_ModeIN_PU);
    }
}

void imu_init_irq(void) {
    if (vp_gpio_is_valid(board_signal_get(BOARD_SIGNAL_IMU_INT1))) {
        (void)c_vp_exti_set_edge(VP_INPUT_IMU_INT1, VP_IMU_INT_EDGE);
    }

    if (vp_gpio_is_valid(board_signal_get(BOARD_SIGNAL_IMU_INT2))) {
        (void)c_vp_exti_set_edge(VP_INPUT_IMU_INT2, VP_IMU_INT_EDGE);
    }
}

vp_bool_t imu_i2c_is_idle(void) {
    const BoardGpio i2c_sda = board_signal_get(BOARD_SIGNAL_I2C_SDA);
    const BoardGpio i2c_scl = board_signal_get(BOARD_SIGNAL_I2C_SCL);

    if (!vp_gpio_is_valid(i2c_sda) || !vp_gpio_is_valid(i2c_scl)) {
        return 0u;
    }

    return (vp_gpio_read_level(i2c_sda) && vp_gpio_read_level(i2c_scl) &&
            I2C_GetFlagStatus(I2C_FLAG_BUSY) == RESET)
               ? 1u
               : 0u;
}

vp_status_t imu_i2c_init(void) {
    imu_i2c_hw_init();
    return imu_i2c_is_idle() ? VP_STATUS_OK : VP_STATUS_BUSY;
}

vp_status_t imu_i2c_recover(void) {
    const BoardGpio i2c_sda = board_signal_get(BOARD_SIGNAL_I2C_SDA);
    const BoardGpio i2c_scl = board_signal_get(BOARD_SIGNAL_I2C_SCL);

    if (!vp_gpio_is_valid(i2c_sda) || !vp_gpio_is_valid(i2c_scl)) {
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

    vp_gpio_digital_cfg(i2c_sda, ENABLE);
    vp_gpio_digital_cfg(i2c_scl, ENABLE);
    i2c_release_lines_to_pullup();
    mDelayuS(10);

    for (uint8_t i = 0; i < VP_I2C_RECOVER_CLOCK_PULSES; i++) {
        if (vp_gpio_read_level(i2c_sda)) {
            break;
        }

        i2c_drive_scl_low();
        mDelayuS(5);
        i2c_release_lines_to_pullup();
        mDelayuS(5);
    }

    if (!vp_gpio_read_level(i2c_sda)) {
        i2c_drive_sda_low();
        mDelayuS(5);
        i2c_drive_scl_low();
        mDelayuS(5);
        i2c_release_lines_to_pullup();
        mDelayuS(5);
    }

    i2c_release_lines_to_pullup();
    mDelayuS(10);

    imu_i2c_hw_init();
    lsm6dsv_reinit_async();
    if (!imu_i2c_is_idle()) {
        VP_LOG_WARN("imu", "i2c recover incomplete");
        return VP_STATUS_BUSY;
    }

    VP_LOG_INFO("imu", "i2c recovered");
    return VP_STATUS_OK;
}

void imu_init(void) {
    const vp_status_t i2c_status = imu_i2c_init();
    if (i2c_status != VP_STATUS_OK) {
        VP_LOG_ERROR("imu", "i2c init failed;status=%u", i2c_status);
        return;
    }

    if (!lsm6dsv_init()) {
        VP_LOG_ERROR("imu", "init failed");
    } else {
        VP_LOG_INFO("imu", "initialized");
    }
}
