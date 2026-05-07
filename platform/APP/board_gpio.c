#include "board_gpio.h"

#include "CH58x_common.h"  // IWYU pragma: keep

vp_bool_t board_gpio_is_valid(const BoardGpio gpio) {
    return (gpio.group == BOARD_GPIO_GROUP_A ||
            gpio.group == BOARD_GPIO_GROUP_B) &&
                   gpio.pin != 0u
               ? 1u
               : 0u;
}

vp_bool_t board_gpio_read_level(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            return GPIOA_ReadPortPin(gpio.pin) ? 1u : 0u;
        case BOARD_GPIO_GROUP_B:
            return GPIOB_ReadPortPin(gpio.pin) ? 1u : 0u;
        default:
            return 0u;
    }
}

uint32_t board_gpio_read_port(const BoardGpioGroup group) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            return GPIOA_ReadPort();
        case BOARD_GPIO_GROUP_B:
            return GPIOB_ReadPort();
        default:
            return 0u;
    }
}

void board_gpio_set(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            GPIOA_SetBits(gpio.pin);
            break;
        case BOARD_GPIO_GROUP_B:
            GPIOB_SetBits(gpio.pin);
            break;
        default:
            break;
    }
}

void board_gpio_reset(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            GPIOA_ResetBits(gpio.pin);
            break;
        case BOARD_GPIO_GROUP_B:
            GPIOB_ResetBits(gpio.pin);
            break;
        default:
            break;
    }
}

void board_gpio_mode_cfg(const BoardGpio gpio, const GPIOModeTypeDef mode) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            GPIOA_ModeCfg(gpio.pin, mode);
            break;
        case BOARD_GPIO_GROUP_B:
            GPIOB_ModeCfg(gpio.pin, mode);
            break;
        default:
            break;
    }
}

void board_gpio_mode_cfg_mask(const BoardGpioGroup group, const uint32_t pins,
                              const GPIOModeTypeDef mode) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            GPIOA_ModeCfg(pins, mode);
            break;
        case BOARD_GPIO_GROUP_B:
            GPIOB_ModeCfg(pins, mode);
            break;
        default:
            break;
    }
}

void board_gpio_digital_cfg(const BoardGpio       gpio,
                            const FunctionalState enable) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            GPIOADigitalCfg(enable, gpio.pin);
            break;
        case BOARD_GPIO_GROUP_B:
            GPIOBDigitalCfg(enable, gpio.pin);
            break;
        default:
            break;
    }
}

void board_gpio_digital_cfg_mask(const BoardGpioGroup  group,
                                 const FunctionalState enable,
                                 const uint32_t        pins) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            GPIOADigitalCfg(enable, pins);
            break;
        case BOARD_GPIO_GROUP_B:
            GPIOBDigitalCfg(enable, pins);
            break;
        default:
            break;
    }
}

void board_gpio_it_mode_cfg(const BoardGpio gpio, const GPIOITModeTpDef mode) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            GPIOA_ITModeCfg(gpio.pin, mode);
            break;
        case BOARD_GPIO_GROUP_B:
            GPIOB_ITModeCfg(gpio.pin, mode);
            break;
        default:
            break;
    }
}

void board_gpio_config_next_edge(const BoardGpio gpio) {
    board_gpio_it_mode_cfg(gpio, board_gpio_read_level(gpio)
                                     ? GPIO_ITMode_FallEdge
                                     : GPIO_ITMode_RiseEdge);
}

void board_gpio_clear_it_flag(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            GPIOA_ClearITFlagBit(gpio.pin);
            break;
        case BOARD_GPIO_GROUP_B:
            GPIOB_ClearITFlagBit(gpio.pin);
            break;
        default:
            break;
    }
}

uint16_t board_gpio_read_it_flag_port(const BoardGpioGroup group) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            return GPIOA_ReadITFlagPort();
        case BOARD_GPIO_GROUP_B:
            return GPIOB_ReadITFlagPort();
        default:
            return 0u;
    }
}

void board_gpio_clear_it_flag_port(const BoardGpioGroup group,
                                   const uint16_t       flags) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            GPIOA_ClearITFlagBit(flags);
            break;
        case BOARD_GPIO_GROUP_B:
            GPIOB_ClearITFlagBit(flags);
            break;
        default:
            break;
    }
}

uint16_t board_gpio_read_int_enable_port(const BoardGpioGroup group) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            return R16_PA_INT_EN;
        case BOARD_GPIO_GROUP_B:
            return R16_PB_INT_EN;
        default:
            return 0u;
    }
}

void board_gpio_prepare_level_rearm(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            R16_PA_INT_EN &= (uint16_t)(~gpio.pin);
            R16_PA_INT_MODE &= (uint16_t)(~gpio.pin);
            R32_PA_CLR |= gpio.pin;
            board_gpio_clear_it_flag_port(
                BOARD_GPIO_GROUP_A,
                board_gpio_read_it_flag_port(BOARD_GPIO_GROUP_A));
            board_gpio_clear_it_flag(gpio);
            break;
        case BOARD_GPIO_GROUP_B:
            board_gpio_clear_it_flag_port(
                BOARD_GPIO_GROUP_B,
                board_gpio_read_it_flag_port(BOARD_GPIO_GROUP_B));
            board_gpio_clear_it_flag(gpio);
            break;
        default:
            break;
    }
}

vp_status_t board_gpio_int_mask(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            R16_PA_INT_EN &= (uint16_t)(~gpio.pin);
            return VP_STATUS_OK;
        case BOARD_GPIO_GROUP_B:
            R16_PB_INT_EN &= (uint16_t)(~gpio.pin);
            return VP_STATUS_OK;
        default:
            return VP_STATUS_INVALID_ARG;
    }
}

vp_status_t board_gpio_int_unmask(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            R16_PA_INT_EN |= (uint16_t)gpio.pin;
            return VP_STATUS_OK;
        case BOARD_GPIO_GROUP_B:
            R16_PB_INT_EN |= (uint16_t)gpio.pin;
            return VP_STATUS_OK;
        default:
            return VP_STATUS_INVALID_ARG;
    }
}

void board_gpio_irq_enable(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            PFIC_EnableIRQ(GPIO_A_IRQn);
            break;
        case BOARD_GPIO_GROUP_B:
            PFIC_EnableIRQ(GPIO_B_IRQn);
            break;
        default:
            break;
    }
}

void board_gpio_irq_clear_pending(const BoardGpioGroup group) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            PFIC_ClearPendingIRQ(GPIO_A_IRQn);
            break;
        case BOARD_GPIO_GROUP_B:
            PFIC_ClearPendingIRQ(GPIO_B_IRQn);
            break;
        default:
            break;
    }
}
