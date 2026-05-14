// 自动生成，勿手动编辑

#include "board_map.h"
#include "CH58x_common.h"

const BoardGpio BOARD_MAP_DEFAULT[BOARD_SIGNAL_COUNT] = {
    [BOARD_SIGNAL_BTN_LEFT] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_9 },
    [BOARD_SIGNAL_BTN_RIGHT] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_8 },
    [BOARD_SIGNAL_BTN_MIDDLE] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_5 },
    [BOARD_SIGNAL_BTN_ACTION] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_10 },
    [BOARD_SIGNAL_BTN_LASER] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_7 },
    [BOARD_SIGNAL_MODE_SWITCH] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_0 },
    [BOARD_SIGNAL_ENC_A] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_4 },
    [BOARD_SIGNAL_ENC_B] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_6 },
    [BOARD_SIGNAL_IMU_INT1] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_11 },
    [BOARD_SIGNAL_IMU_INT2] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_12 },
    [BOARD_SIGNAL_LED_STATUS] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_2 },
    [BOARD_SIGNAL_PWM_LASER] = { .group = BOARD_GPIO_GROUP_B, .pin = GPIO_Pin_4 },
    [BOARD_SIGNAL_I2C_SDA] = { .group = BOARD_GPIO_GROUP_B, .pin = GPIO_Pin_20 },
    [BOARD_SIGNAL_I2C_SCL] = { .group = BOARD_GPIO_GROUP_B, .pin = GPIO_Pin_21 },
    [BOARD_SIGNAL_DEBUG_TX] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_14 },
    [BOARD_SIGNAL_DEBUG_RX] = { .group = BOARD_GPIO_GROUP_A, .pin = GPIO_Pin_15 },
};

BoardGpio BOARD_MAP_CURRENT[BOARD_SIGNAL_COUNT];

BoardGpio board_signal_get(BoardSignal sig) {
    if (sig >= BOARD_SIGNAL_COUNT) {
        return (BoardGpio){ .group = BOARD_GPIO_GROUP_NONE, .pin = 0u };
    }
    return BOARD_MAP_CURRENT[sig];
}

bool board_signal_is_present(BoardSignal sig) {
    if (sig >= BOARD_SIGNAL_COUNT) {
        return false;
    }
    const BoardGpio g = BOARD_MAP_CURRENT[sig];
    return g.group != BOARD_GPIO_GROUP_NONE && g.pin != 0u;
}

void board_remap_reset(void) {
    for (int i = 0; i < BOARD_SIGNAL_COUNT; i++) {
        BOARD_MAP_CURRENT[i] = BOARD_MAP_DEFAULT[i];
    }
}

void board_remap_apply(const BoardGpio mapping[BOARD_SIGNAL_COUNT]) {
    for (int i = 0; i < BOARD_SIGNAL_COUNT; i++) {
        BOARD_MAP_CURRENT[i] = mapping[i];
    }
}

uint8_t board_signal_get_channel(BoardSignal sig) {
    switch (sig) {
        case BOARD_SIGNAL_LED_STATUS: return 3;
        case BOARD_SIGNAL_PWM_LASER: return 7;
        case BOARD_SIGNAL_DEBUG_TX: return 0;
        case BOARD_SIGNAL_DEBUG_RX: return 0;
        default: return 0;
    }
}

void board_gpio_init_all(void) {
    board_remap_reset();

    GPIOPinRemap(DISABLE, RB_PIN_TMR3);
    GPIOPinRemap(DISABLE, RB_PIN_PWMX);
    GPIOPinRemap(ENABLE, RB_PIN_I2C);
    GPIOPinRemap(ENABLE, RB_PIN_UART0);

    GPIOADigitalCfg(ENABLE, GPIO_Pin_9);
    GPIOA_ModeCfg(GPIO_Pin_9, GPIO_ModeIN_PU);
    GPIOADigitalCfg(ENABLE, GPIO_Pin_8);
    GPIOA_ModeCfg(GPIO_Pin_8, GPIO_ModeIN_PU);
    GPIOADigitalCfg(ENABLE, GPIO_Pin_5);
    GPIOA_ModeCfg(GPIO_Pin_5, GPIO_ModeIN_PU);
    GPIOADigitalCfg(ENABLE, GPIO_Pin_10);
    GPIOA_ModeCfg(GPIO_Pin_10, GPIO_ModeIN_PU);
    GPIOADigitalCfg(ENABLE, GPIO_Pin_7);
    GPIOA_ModeCfg(GPIO_Pin_7, GPIO_ModeIN_PU);
    GPIOA_ModeCfg(GPIO_Pin_0, GPIO_ModeOut_PP_5mA);
    GPIOA_ResetBits(GPIO_Pin_0);
    GPIOADigitalCfg(ENABLE, GPIO_Pin_4);
    GPIOA_ModeCfg(GPIO_Pin_4, GPIO_ModeIN_PU);
    GPIOADigitalCfg(ENABLE, GPIO_Pin_6);
    GPIOA_ModeCfg(GPIO_Pin_6, GPIO_ModeIN_PU);
    GPIOADigitalCfg(ENABLE, GPIO_Pin_11);
    GPIOA_ModeCfg(GPIO_Pin_11, GPIO_ModeIN_PU);
    GPIOADigitalCfg(ENABLE, GPIO_Pin_12);
    GPIOA_ModeCfg(GPIO_Pin_12, GPIO_ModeIN_PU);
    GPIOA_ModeCfg(GPIO_Pin_2, GPIO_ModeOut_PP_5mA);
    GPIOB_ModeCfg(GPIO_Pin_4, GPIO_ModeOut_PP_5mA);
    GPIOBDigitalCfg(ENABLE, GPIO_Pin_20);
    GPIOB_ModeCfg(GPIO_Pin_20, GPIO_ModeIN_PU);
    GPIOBDigitalCfg(ENABLE, GPIO_Pin_21);
    GPIOB_ModeCfg(GPIO_Pin_21, GPIO_ModeIN_PU);
    GPIOA_ModeCfg(GPIO_Pin_14, GPIO_ModeOut_PP_5mA);
    GPIOA_ModeCfg(GPIO_Pin_15, GPIO_ModeIN_PU);
}
