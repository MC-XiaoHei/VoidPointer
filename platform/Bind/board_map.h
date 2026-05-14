// 自动生成，勿手动编辑

#ifndef VOIDPOINTER_BOARD_MAP_H
#define VOIDPOINTER_BOARD_MAP_H

#include <stdint.h>
#include <stdbool.h>

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

typedef enum {
    BOARD_SIGNAL_BTN_LEFT,
    BOARD_SIGNAL_BTN_RIGHT,
    BOARD_SIGNAL_BTN_MIDDLE,
    BOARD_SIGNAL_BTN_ACTION,
    BOARD_SIGNAL_BTN_LASER,
    BOARD_SIGNAL_MODE_SWITCH,
    BOARD_SIGNAL_ENC_A,
    BOARD_SIGNAL_ENC_B,
    BOARD_SIGNAL_IMU_INT1,
    BOARD_SIGNAL_IMU_INT2,
    BOARD_SIGNAL_LED_STATUS,
    BOARD_SIGNAL_PWM_LASER,
    BOARD_SIGNAL_I2C_SDA,
    BOARD_SIGNAL_I2C_SCL,
    BOARD_SIGNAL_DEBUG_TX,
    BOARD_SIGNAL_DEBUG_RX,
    BOARD_SIGNAL_COUNT,
} BoardSignal;

BoardGpio board_signal_get(BoardSignal sig);
bool      board_signal_is_present(BoardSignal sig);

extern const BoardGpio BOARD_MAP_DEFAULT[BOARD_SIGNAL_COUNT];
extern BoardGpio       BOARD_MAP_CURRENT[BOARD_SIGNAL_COUNT];

void board_remap_reset(void);
void board_remap_apply(const BoardGpio mapping[BOARD_SIGNAL_COUNT]);
void board_gpio_init_all(void);
uint8_t board_signal_get_channel(BoardSignal sig);

#ifdef __cplusplus
}
#endif

#endif
