#include "lsm6dsv.h"
#include "CH58x_common.h"  // IWYU pragma: keep
#include "rust_api.h"

typedef enum {
    LSM6DSV_ASYNC_IDLE = 0,
    LSM6DSV_ASYNC_READ_STATUS1_TX,
    LSM6DSV_ASYNC_READ_STATUS1_RX,
    LSM6DSV_ASYNC_READ_STATUS2_TX,
    LSM6DSV_ASYNC_READ_STATUS2_RX,
    LSM6DSV_ASYNC_READ_FIFO_WORD_TX,
    LSM6DSV_ASYNC_READ_FIFO_WORD_RX,
} lsm6dsv_async_phase_t;

typedef struct {
    volatile vp_bool_t             busy;
    volatile vp_bool_t             read_in_progress;
    volatile vp_bool_t             nack_sent;
    volatile vp_status_t           request_status;
    volatile lsm6dsv_async_phase_t phase;

    uint8_t          tx_buf[1];
    uint8_t          rx_buf[7];
    volatile uint8_t tx_index;
    volatile uint8_t tx_len;
    volatile uint8_t rx_index;
    volatile uint8_t rx_len;
    volatile uint8_t addr_rw;

    uint16_t requested_max_samples;
    uint16_t fifo_total_words;
    uint16_t fifo_words_remaining;
    uint16_t fifo_words_to_drop;
    uint16_t dropped_count;

    uint8_t status1;
    uint8_t status2;

    sflp_game_rotation_raw_t latest_raw;
    vp_bool_t                latest_raw_valid;
} lsm6dsv_async_ctx_t;

static lsm6dsv_async_ctx_t g_lsm6dsv_async = {0};
static uint8_t             g_lsm6dsv_i2c_addr = LSM6DSV_I2C_ADDR;

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

static bool lsm6dsv_write_reg_addr(const uint8_t addr, const uint8_t reg,
                                   const uint8_t value) {
    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(addr, I2C_Direction_Transmitter);
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

static bool lsm6dsv_write_reg(const uint8_t reg, const uint8_t value) {
    return lsm6dsv_write_reg_addr(g_lsm6dsv_i2c_addr, reg, value);
}

static bool lsm6dsv_read_reg_addr(const uint8_t addr, const uint8_t reg,
                                  uint8_t* value) {
    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(addr, I2C_Direction_Transmitter);
    if (!i2c_wait_event(I2C_EVENT_MASTER_TRANSMITTER_MODE_SELECTED))
        return false;

    I2C_SendData(reg);
    if (!i2c_wait_event(I2C_EVENT_MASTER_BYTE_TRANSMITTED)) return false;

    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(addr, I2C_Direction_Receiver);
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

static bool lsm6dsv_read_reg(const uint8_t reg, uint8_t* value) {
    return lsm6dsv_read_reg_addr(g_lsm6dsv_i2c_addr, reg, value);
}

static bool lsm6dsv_read_regs(const uint8_t reg, uint8_t* buf,
                              const uint16_t len) {
    if (len == 0) return false;

    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(g_lsm6dsv_i2c_addr, I2C_Direction_Transmitter);
    if (!i2c_wait_event(I2C_EVENT_MASTER_TRANSMITTER_MODE_SELECTED))
        return false;

    I2C_SendData(reg);
    if (!i2c_wait_event(I2C_EVENT_MASTER_BYTE_TRANSMITTED)) return false;

    I2C_GenerateSTART(ENABLE);
    if (!i2c_wait_event(I2C_EVENT_MASTER_MODE_SELECT)) return false;

    I2C_Send7bitAddress(g_lsm6dsv_i2c_addr, I2C_Direction_Receiver);
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

static bool lsm6dsv_probe_address(const uint8_t addr) {
    uint8_t id = 0;
    if (!lsm6dsv_read_reg_addr(addr, LSM6DSV_REG_WHO_AM_I, &id)) {
        return false;
    }

    if (id != LSM6DSV_WHOAMI_VALUE) {
        return false;
    }

    g_lsm6dsv_i2c_addr = addr;
    return true;
}

static bool lsm6dsv_check_id(void) {
    uint8_t id = 0;
    if (!lsm6dsv_read_reg(LSM6DSV_REG_WHO_AM_I, &id)) {
        VP_LOG_ERROR("imu", "read whoami failed");
        return false;
    }
    if (id != LSM6DSV_WHOAMI_VALUE) {
        VP_LOG_ERROR("imu", "unexpected whoami;value=0x%02X,expected=0x%02X",
                     id, LSM6DSV_WHOAMI_VALUE);
        return false;
    }
    return true;
}

static bool lsm6dsv_apply_active_profile(void) {
#define VP_IMU_WRITE_OR_FAIL(reg, value, name)                               \
    do {                                                                     \
        if (!lsm6dsv_write_reg((reg), (value))) {                            \
            VP_LOG_ERROR(                                                    \
                "imu",                                                       \
                "active profile write failed;step=%s,reg=0x%02X,val=0x%02X", \
                (name), (reg), (value));                                     \
            return false;                                                    \
        }                                                                    \
    } while (0)

    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_CTRL3, 0x44, "ctrl3");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_CTRL8, 0x00, "ctrl8");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_CTRL6, 0x04, "ctrl6");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_CTRL1, 0x06, "ctrl1");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_CTRL2, 0x06, "ctrl2");

    mDelaymS(50);

    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_FUNCTIONS_ENABLE, 0x40,
                         "functions_enable");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_FUNC_CFG_ACCESS, 0x80,
                         "func_cfg_access_on");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_EMB_FUNC_EN_A, 0x02, "emb_func_en_a");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_EMB_FUNC_FIFO_EN_A, 0x02,
                         "emb_func_fifo_en_a");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_SFLP_ODR, 0x5B, "sflp_odr");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_EMB_FUNC_INIT_A, 0x02, "emb_func_init_a");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_FUNC_CFG_ACCESS, 0x00,
                         "func_cfg_access_off");
    VP_IMU_WRITE_OR_FAIL(LSM6DSV_REG_FIFO_CTRL4, 0x06, "fifo_ctrl4");

#undef VP_IMU_WRITE_OR_FAIL
    return true;
}

static void lsm6dsv_async_reset_io_state(void) {
    g_lsm6dsv_async.tx_index = 0u;
    g_lsm6dsv_async.tx_len = 0u;
    g_lsm6dsv_async.rx_index = 0u;
    g_lsm6dsv_async.rx_len = 0u;
    g_lsm6dsv_async.addr_rw = 0u;
    g_lsm6dsv_async.read_in_progress = 0u;
    g_lsm6dsv_async.nack_sent = 0u;
}

static void lsm6dsv_async_finish(const vp_status_t status) {
    const vp_timestamp_t timestamp = c_vp_rtc_millis();

    I2C_AcknowledgeConfig(ENABLE);
    I2C_GenerateSTOP(ENABLE);
    lsm6dsv_async_reset_io_state();

    g_lsm6dsv_async.phase = LSM6DSV_ASYNC_IDLE;
    g_lsm6dsv_async.busy = 0u;
    g_lsm6dsv_async.request_status = status;

    if (status == VP_STATUS_OK && g_lsm6dsv_async.latest_raw_valid) {
        vp_on_imu_sample(g_lsm6dsv_async.latest_raw.x,
                         g_lsm6dsv_async.latest_raw.y,
                         g_lsm6dsv_async.latest_raw.z, timestamp);
    }

    vp_on_imu_fifo_done(status, g_lsm6dsv_async.dropped_count, timestamp);
}

static void lsm6dsv_async_prepare_write_then_read(
    const uint8_t reg, const uint8_t rx_len,
    const lsm6dsv_async_phase_t next_phase) {
    g_lsm6dsv_async.tx_buf[0] = reg;
    g_lsm6dsv_async.tx_index = 0u;
    g_lsm6dsv_async.tx_len = 1u;
    g_lsm6dsv_async.rx_index = 0u;
    g_lsm6dsv_async.rx_len = rx_len;
    g_lsm6dsv_async.read_in_progress = 0u;
    g_lsm6dsv_async.nack_sent = 0u;
    g_lsm6dsv_async.phase = next_phase;
    g_lsm6dsv_async.addr_rw = LSM6DSV_I2C_ADDR;

    I2C_GenerateSTOP(DISABLE);
    I2C_GenerateSTART(ENABLE);
}

static void lsm6dsv_async_begin_next_phase(void) {
    switch (g_lsm6dsv_async.phase) {
        case LSM6DSV_ASYNC_READ_STATUS1_TX:
            lsm6dsv_async_prepare_write_then_read(
                LSM6DSV_REG_FIFO_STATUS1, 1u, LSM6DSV_ASYNC_READ_STATUS1_RX);
            break;
        case LSM6DSV_ASYNC_READ_STATUS2_TX:
            lsm6dsv_async_prepare_write_then_read(
                LSM6DSV_REG_FIFO_STATUS2, 1u, LSM6DSV_ASYNC_READ_STATUS2_RX);
            break;
        case LSM6DSV_ASYNC_READ_FIFO_WORD_TX:
            lsm6dsv_async_prepare_write_then_read(
                LSM6DSV_REG_FIFO_DATA_OUT_TAG, 7u,
                LSM6DSV_ASYNC_READ_FIFO_WORD_RX);
            break;
        default:
            lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
            break;
    }
}

static void lsm6dsv_async_consume_fifo_word(void) {
    const uint8_t tag = g_lsm6dsv_async.rx_buf[0] >> 3;

    if (g_lsm6dsv_async.fifo_words_to_drop > 0u) {
        g_lsm6dsv_async.fifo_words_to_drop--;
        g_lsm6dsv_async.dropped_count++;
    } else if (tag == LSM6DSV_FIFO_TAG_SFLP_GAME) {
        g_lsm6dsv_async.latest_raw.x =
            (uint16_t)(((uint16_t)g_lsm6dsv_async.rx_buf[2] << 8) |
                       g_lsm6dsv_async.rx_buf[1]);
        g_lsm6dsv_async.latest_raw.y =
            (uint16_t)(((uint16_t)g_lsm6dsv_async.rx_buf[4] << 8) |
                       g_lsm6dsv_async.rx_buf[3]);
        g_lsm6dsv_async.latest_raw.z =
            (uint16_t)(((uint16_t)g_lsm6dsv_async.rx_buf[6] << 8) |
                       g_lsm6dsv_async.rx_buf[5]);
        g_lsm6dsv_async.latest_raw_valid = 1u;
    }

    if (g_lsm6dsv_async.fifo_words_remaining > 0u) {
        g_lsm6dsv_async.fifo_words_remaining--;
    }

    if (g_lsm6dsv_async.fifo_words_remaining == 0u) {
        lsm6dsv_async_finish(g_lsm6dsv_async.latest_raw_valid
                                 ? VP_STATUS_OK
                                 : VP_STATUS_NOT_READY);
        return;
    }

    g_lsm6dsv_async.phase = LSM6DSV_ASYNC_READ_FIFO_WORD_TX;
    lsm6dsv_async_begin_next_phase();
}

void LSM6DSV_AsyncInit(void) {
    lsm6dsv_async_reset_io_state();
    g_lsm6dsv_async.busy = 0u;
    g_lsm6dsv_async.phase = LSM6DSV_ASYNC_IDLE;
    g_lsm6dsv_async.request_status = VP_STATUS_OK;
    I2C_ITConfig(I2C_IT_BUF, ENABLE);
    I2C_ITConfig(I2C_IT_EVT, ENABLE);
    I2C_ITConfig(I2C_IT_ERR, ENABLE);
    PFIC_EnableIRQ(I2C_IRQn);
}

vp_bool_t LSM6DSV_IsAsyncBusy(void) { return g_lsm6dsv_async.busy; }

vp_status_t LSM6DSV_AbortAsync(void) {
    if (!g_lsm6dsv_async.busy) {
        return VP_STATUS_OK;
    }

    lsm6dsv_async_finish(VP_STATUS_BUSY);
    return VP_STATUS_OK;
}

vp_status_t LSM6DSV_StartAsyncFifoRead(const uint16_t max_samples) {
    if (g_lsm6dsv_async.busy) {
        return VP_STATUS_BUSY;
    }

    if (I2C_GetFlagStatus(I2C_FLAG_BUSY) != RESET) {
        return VP_STATUS_BUSY;
    }

    g_lsm6dsv_async.busy = 1u;
    g_lsm6dsv_async.request_status = VP_STATUS_BUSY;
    g_lsm6dsv_async.requested_max_samples = max_samples;
    g_lsm6dsv_async.fifo_total_words = 0u;
    g_lsm6dsv_async.fifo_words_remaining = 0u;
    g_lsm6dsv_async.fifo_words_to_drop = 0u;
    g_lsm6dsv_async.dropped_count = 0u;
    g_lsm6dsv_async.status1 = 0u;
    g_lsm6dsv_async.status2 = 0u;
    g_lsm6dsv_async.latest_raw_valid = 0u;

    g_lsm6dsv_async.phase = LSM6DSV_ASYNC_READ_STATUS1_TX;
    lsm6dsv_async_begin_next_phase();
    return VP_STATUS_OK;
}

bool LSM6DSV_Init(void) {
    if (!lsm6dsv_probe_address((0x6A << 1)) &&
        !lsm6dsv_probe_address((0x6B << 1))) {
        VP_LOG_ERROR("imu", "no imu ack on 0x6A/0x6B");
        return false;
    }

    if (!lsm6dsv_write_reg(LSM6DSV_REG_CTRL3, 0x01)) {
        VP_LOG_ERROR("imu", "soft reset write failed;addr=0x%02X",
                     g_lsm6dsv_i2c_addr >> 1);
        return false;
    }
    mDelaymS(50);

    if (!lsm6dsv_check_id()) {
        return false;
    }
    if (!lsm6dsv_apply_active_profile()) {
        return false;
    }

    LSM6DSV_AsyncInit();
    mDelaymS(100);
    return true;
}

bool LSM6DSV_ConfigActive(void) {
    if (g_lsm6dsv_async.busy) {
        return false;
    }

    return lsm6dsv_apply_active_profile();
}

bool LSM6DSV_ConfigSuspend(void) { return false; }

bool LSM6DSV_ConfigSleep(void) { return false; }

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

    uint16_t sample_count = (uint16_t)((((uint16_t)s2) & 0x01u) << 8) | s1;
    if (sample_count == 0u) return false;

    uint16_t drop_count = 0u;
    if (max_samples != 0u && sample_count > max_samples) {
        drop_count = (uint16_t)(sample_count - max_samples);
        if (out_dropped_count != 0) {
            *out_dropped_count = drop_count;
        }
    }

    while (sample_count--) {
        if (!lsm6dsv_read_regs(LSM6DSV_REG_FIFO_DATA_OUT_TAG, data, 7)) {
            return false;
        }

        const uint8_t tag = data[0] >> 3;
        if (drop_count > 0u) {
            drop_count--;
            continue;
        }

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

__INTERRUPT
__HIGH_CODE
void I2C_IRQHandler(void) {
    uint32_t event = I2C_GetLastEvent();

    if (!g_lsm6dsv_async.busy) {
        if (event & RB_I2C_AF) {
            I2C_ClearFlag(I2C_FLAG_AF);
        }
        if (event & RB_I2C_BERR) {
            I2C_ClearFlag(RB_I2C_BERR);
        }
        if (event & RB_I2C_ARLO) {
            I2C_ClearFlag(RB_I2C_ARLO);
        }
        if (event & RB_I2C_OVR) {
            I2C_ClearFlag(RB_I2C_OVR);
        }
        if (event & RB_I2C_PECERR) {
            I2C_ClearFlag(RB_I2C_PECERR);
        }
        if (event & RB_I2C_TIMEOUT) {
            I2C_ClearFlag(RB_I2C_TIMEOUT);
        }
        if (event & RB_I2C_SMBALERT) {
            I2C_ClearFlag(RB_I2C_SMBALERT);
        }
        return;
    }

    if (event & RB_I2C_SB) {
        I2C_SendData(g_lsm6dsv_async.addr_rw);
        return;
    }

    if (event & (RB_I2C_MSL << 16)) {
        if (event & (RB_I2C_TRA << 16)) {
            if (event & RB_I2C_AF) {
                I2C_ClearFlag(I2C_FLAG_AF);
                lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
                return;
            }

            if (event &
                (RB_I2C_ADDR | RB_I2C_BTF | RB_I2C_TxE | (RB_I2C_TRA << 16))) {
                if (!g_lsm6dsv_async.read_in_progress) {
                    if (g_lsm6dsv_async.tx_index < g_lsm6dsv_async.tx_len) {
                        I2C_SendData(
                            g_lsm6dsv_async.tx_buf[g_lsm6dsv_async.tx_index++]);
                    } else {
                        g_lsm6dsv_async.read_in_progress = 1u;
                        I2C_GenerateSTART(ENABLE);
                    }
                }
            }
        } else {
            if (event & RB_I2C_ADDR) {
                if (g_lsm6dsv_async.rx_len > 1u) {
                    I2C_AcknowledgeConfig(ENABLE);
                } else {
                    I2C_AcknowledgeConfig(DISABLE);
                    g_lsm6dsv_async.nack_sent = 1u;
                }
            }

            if (event & RB_I2C_RxNE) {
                if (g_lsm6dsv_async.rx_index < g_lsm6dsv_async.rx_len) {
                    g_lsm6dsv_async.rx_buf[g_lsm6dsv_async.rx_index++] =
                        I2C_ReceiveData();
                }

                if (g_lsm6dsv_async.rx_index < g_lsm6dsv_async.rx_len) {
                    if ((g_lsm6dsv_async.rx_len - g_lsm6dsv_async.rx_index) ==
                        1u) {
                        I2C_AcknowledgeConfig(DISABLE);
                    } else {
                        I2C_AcknowledgeConfig(ENABLE);
                    }
                } else {
                    I2C_GenerateSTOP(ENABLE);

                    switch (g_lsm6dsv_async.phase) {
                        case LSM6DSV_ASYNC_READ_STATUS1_RX:
                            g_lsm6dsv_async.status1 = g_lsm6dsv_async.rx_buf[0];
                            g_lsm6dsv_async.phase =
                                LSM6DSV_ASYNC_READ_STATUS2_TX;
                            lsm6dsv_async_begin_next_phase();
                            return;
                        case LSM6DSV_ASYNC_READ_STATUS2_RX: {
                            g_lsm6dsv_async.status2 = g_lsm6dsv_async.rx_buf[0];
                            g_lsm6dsv_async.fifo_total_words =
                                (uint16_t)((((uint16_t)
                                                 g_lsm6dsv_async.status2) &
                                            0x01u)
                                           << 8) |
                                g_lsm6dsv_async.status1;
                            g_lsm6dsv_async.fifo_words_remaining =
                                g_lsm6dsv_async.fifo_total_words;

                            if (g_lsm6dsv_async.fifo_total_words == 0u) {
                                lsm6dsv_async_finish(VP_STATUS_NOT_READY);
                                return;
                            }

                            if (g_lsm6dsv_async.requested_max_samples != 0u &&
                                g_lsm6dsv_async.fifo_total_words >
                                    g_lsm6dsv_async.requested_max_samples) {
                                g_lsm6dsv_async.fifo_words_to_drop =
                                    (uint16_t)(g_lsm6dsv_async
                                                   .fifo_total_words -
                                               g_lsm6dsv_async
                                                   .requested_max_samples);
                            } else {
                                g_lsm6dsv_async.fifo_words_to_drop = 0u;
                            }

                            g_lsm6dsv_async.phase =
                                LSM6DSV_ASYNC_READ_FIFO_WORD_TX;
                            lsm6dsv_async_begin_next_phase();
                            return;
                        }
                        case LSM6DSV_ASYNC_READ_FIFO_WORD_RX:
                            lsm6dsv_async_consume_fifo_word();
                            return;
                        default:
                            lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
                            return;
                    }
                }
            }

            if (event & RB_I2C_AF) {
                I2C_ClearFlag(I2C_FLAG_AF);
                lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
                return;
            }
        }
    }

    if (event & RB_I2C_BERR) {
        I2C_ClearFlag(RB_I2C_BERR);
        lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
        return;
    }
    if (event & RB_I2C_ARLO) {
        I2C_ClearFlag(RB_I2C_ARLO);
        lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
        return;
    }
    if (event & RB_I2C_OVR) {
        I2C_ClearFlag(RB_I2C_OVR);
        lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
        return;
    }
    if (event & RB_I2C_PECERR) {
        I2C_ClearFlag(RB_I2C_PECERR);
        lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
        return;
    }
    if (event & RB_I2C_TIMEOUT) {
        I2C_ClearFlag(RB_I2C_TIMEOUT);
        lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
        return;
    }
    if (event & RB_I2C_SMBALERT) {
        I2C_ClearFlag(RB_I2C_SMBALERT);
        lsm6dsv_async_finish(VP_STATUS_IO_ERROR);
        return;
    }
}
