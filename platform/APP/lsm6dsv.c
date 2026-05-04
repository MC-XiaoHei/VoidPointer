#include "lsm6dsv.h"
#include "CH58x_common.h"  // IWYU pragma: keep

static bool i2c_wait_event(const uint32_t event) {
    uint32_t timeout = LSM6DSV_I2C_MAX_TIMEOUT;

    while (!I2C_CheckEvent(event)) {
        if (I2C_GetFlagStatus(I2C_FLAG_AF)) {
            I2C_ClearFlag(I2C_FLAG_AF);
            I2C_GenerateSTOP(ENABLE);
            return false;
        }
        if (--timeout == 0) {
            I2C_GenerateSTOP(ENABLE);
            return false;
        }
    }

    return true;
}

static bool lsm6dsv_write_reg(const uint8_t reg, const uint8_t value) {
    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(LSM6DSV_I2C_ADDR, I2C_Direction_Transmitter);
    if (!i2c_wait_event(I2C_EVENT_MASTER_TRANSMITTER_MODE_SELECTED))
        return false;

    I2C_SendData(reg);
    if (!i2c_wait_event(I2C_EVENT_MASTER_BYTE_TRANSMITTED)) return false;

    I2C_SendData(value);
    if (!i2c_wait_event(I2C_EVENT_MASTER_BYTE_TRANSMITTED)) return false;

    I2C_GenerateSTOP(ENABLE);

    mDelaymS(5);
    return true;
}

static bool lsm6dsv_read_reg(const uint8_t reg, uint8_t* value) {
    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(LSM6DSV_I2C_ADDR, I2C_Direction_Transmitter);
    if (!i2c_wait_event(I2C_EVENT_MASTER_TRANSMITTER_MODE_SELECTED))
        return false;

    I2C_SendData(reg);
    if (!i2c_wait_event(I2C_EVENT_MASTER_BYTE_TRANSMITTED)) return false;

    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(LSM6DSV_I2C_ADDR, I2C_Direction_Receiver);
    if (!i2c_wait_event(I2C_EVENT_MASTER_RECEIVER_MODE_SELECTED)) return false;

    I2C_AcknowledgeConfig(DISABLE);
    I2C_GenerateSTOP(ENABLE);

    if (!i2c_wait_event(I2C_EVENT_MASTER_BYTE_RECEIVED)) {
        I2C_AcknowledgeConfig(ENABLE);
        return false;
    }

    *value = I2C_ReceiveData();
    I2C_AcknowledgeConfig(ENABLE);
    return true;
}

static bool lsm6dsv_read_regs(const uint8_t reg, uint8_t* buf,
                              const uint16_t len) {
    if (len == 0) return false;

    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(LSM6DSV_I2C_ADDR, I2C_Direction_Transmitter);
    if (!i2c_wait_event(I2C_EVENT_MASTER_TRANSMITTER_MODE_SELECTED))
        return false;

    I2C_SendData(reg);
    if (!i2c_wait_event(I2C_EVENT_MASTER_BYTE_TRANSMITTED)) return false;

    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(LSM6DSV_I2C_ADDR, I2C_Direction_Receiver);
    if (!i2c_wait_event(I2C_EVENT_MASTER_RECEIVER_MODE_SELECTED)) return false;

    for (uint16_t i = 0; i < len; i++) {
        if (i == (len - 1)) {
            I2C_AcknowledgeConfig(DISABLE);
            I2C_GenerateSTOP(ENABLE);
        }

        if (!i2c_wait_event(I2C_EVENT_MASTER_BYTE_RECEIVED)) {
            I2C_AcknowledgeConfig(ENABLE);
            return false;
        }

        buf[i] = I2C_ReceiveData();
    }

    I2C_AcknowledgeConfig(ENABLE);
    return true;
}

static bool lsm6dsv_check_id(void) {
    uint8_t id = 0;
    if (!lsm6dsv_read_reg(LSM6DSV_REG_WHO_AM_I, &id)) return false;
    return id == LSM6DSV_WHOAMI_VALUE;
}

bool LSM6DSV_Init(void) {
    if (!lsm6dsv_write_reg(LSM6DSV_REG_CTRL3, 0x01)) return false;
    mDelaymS(50);

    if (!lsm6dsv_check_id()) return false;

    if (!lsm6dsv_write_reg(LSM6DSV_REG_CTRL3, 0x44)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_CTRL8, 0x00)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_CTRL6, 0x04)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_CTRL1, 0x06)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_CTRL2, 0x06)) return false;

    mDelaymS(50);

    if (!lsm6dsv_write_reg(LSM6DSV_REG_FUNCTIONS_ENABLE, 0x40)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_FUNC_CFG_ACCESS, 0x80)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_EMB_FUNC_EN_A, 0x02)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_EMB_FUNC_FIFO_EN_A, 0x02)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_SFLP_ODR, 0x5B)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_EMB_FUNC_INIT_A, 0x02)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_FUNC_CFG_ACCESS, 0x00)) return false;
    if (!lsm6dsv_write_reg(LSM6DSV_REG_FIFO_CTRL4, 0x06)) return false;

    mDelaymS(100);
    return true;
}

bool LSM6DSV_ReadWhoAmI(uint8_t* out_id) {
    if (out_id == 0) {
        return false;
    }

    return lsm6dsv_read_reg(LSM6DSV_REG_WHO_AM_I, out_id);
}

bool LSM6DSV_ReadLatestSFLPGameRotationRaw(sflp_game_rotation_raw_t* raw,
                                           const uint16_t max_samples,
                                           uint16_t*      out_dropped_count) {
    if (raw == 0) return false;

    uint8_t s1 = 0;
    uint8_t s2 = 0;
    uint8_t data[7];
    bool    found = false;

    if (out_dropped_count != 0) {
        *out_dropped_count = 0u;
    }

    if (!lsm6dsv_read_reg(LSM6DSV_REG_FIFO_STATUS1, &s1)) return false;
    if (!lsm6dsv_read_reg(LSM6DSV_REG_FIFO_STATUS2, &s2)) return false;

    uint16_t sample_count = ((s2 & 0x01) << 8) | s1;
    if (sample_count == 0u) return false;

    uint16_t read_limit = sample_count;
    if (max_samples != 0u && read_limit > max_samples) {
        read_limit = max_samples;
        if (out_dropped_count != 0) {
            *out_dropped_count = (uint16_t)(sample_count - max_samples);
        }
    }

    while (read_limit--) {
        if (!lsm6dsv_read_regs(LSM6DSV_REG_FIFO_DATA_OUT_TAG, data, 7)) {
            return false;
        }

        const uint8_t tag = data[0] >> 3;
        if (tag == LSM6DSV_FIFO_TAG_SFLP_GAME) {
            raw->x = ((uint16_t)data[2] << 8) | data[1];
            raw->y = ((uint16_t)data[4] << 8) | data[3];
            raw->z = ((uint16_t)data[6] << 8) | data[5];
            found = true;
        }
    }

    return found;
}

bool LSM6DSV_ReadSFLPGameRotationRaw(sflp_game_rotation_raw_t* raw) {
    return LSM6DSV_ReadLatestSFLPGameRotationRaw(raw, 0u, 0);
}
