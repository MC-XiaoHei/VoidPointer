#include "vp_hal.h"
#include "CH58x_common.h"
#include "CH58x_pwm.h"
#include "CH58x_timer.h"

// PWMX

void vp_pwm_init(BoardSignal sig, uint16_t cycle) {
    uint8_t ch = board_signal_get_channel(sig);
    PWMX_CLKCfg(4u);
    PWMX_CycleCfg(cycle);
    PWMX_ACTOUT(ch, 0u, board_signal_get_polarity(sig) ? Low_Level : High_Level,
                ENABLE);
}

void vp_pwm_set_duty(BoardSignal sig, uint8_t duty) {
    switch (board_signal_get_channel(sig)) {
        case 4:
            PWM4_ActDataWidth(duty);
            break;
        case 5:
            PWM5_ActDataWidth(duty);
            break;
        case 6:
            PWM6_ActDataWidth(duty);
            break;
        case 7:
            PWM7_ActDataWidth(duty);
            break;
        case 8:
            PWM8_ActDataWidth(duty);
            break;
        case 9:
            PWM9_ActDataWidth(duty);
            break;
        default:
            break;
    }
}

// TMR PWM

static void tmr_route_pwm_cycle(uint8_t ch, uint32_t cycle) {
    switch (ch) {
        case 0:
            TMR0_PWMCycleCfg(cycle);
            break;
        case 1:
            TMR1_PWMCycleCfg(cycle);
            break;
        case 2:
            TMR2_PWMCycleCfg(cycle);
            break;
        case 3:
            TMR3_PWMCycleCfg(cycle);
            break;
        default:
            break;
    }
}

static void tmr_route_pwm_init(uint8_t ch) {
    switch (ch) {
        case 0:
            TMR0_PWMInit(High_Level, PWM_Times_1);
            break;
        case 1:
            TMR1_PWMInit(High_Level, PWM_Times_1);
            break;
        case 2:
            TMR2_PWMInit(High_Level, PWM_Times_1);
            break;
        case 3:
            TMR3_PWMInit(High_Level, PWM_Times_1);
            break;
        default:
            break;
    }
}

static void tmr_route_enable(uint8_t ch) {
    switch (ch) {
        case 0:
            TMR0_Enable();
            break;
        case 1:
            TMR1_Enable();
            break;
        case 2:
            TMR2_Enable();
            break;
        case 3:
            TMR3_Enable();
            break;
        default:
            break;
    }
}

static void tmr_route_disable(uint8_t ch) {
    switch (ch) {
        case 0:
            TMR0_Disable();
            break;
        case 1:
            TMR1_Disable();
            break;
        case 2:
            TMR2_Disable();
            break;
        case 3:
            TMR3_Disable();
            break;
        default:
            break;
    }
}

static void tmr_route_pwm_enable(uint8_t ch) {
    switch (ch) {
        case 0:
            TMR0_PWMEnable();
            break;
        case 1:
            TMR1_PWMEnable();
            break;
        case 2:
            TMR2_PWMEnable();
            break;
        case 3:
            TMR3_PWMEnable();
            break;
        default:
            break;
    }
}

static void tmr_route_pwm_disable(uint8_t ch) {
    switch (ch) {
        case 0:
            TMR0_PWMDisable();
            break;
        case 1:
            TMR1_PWMDisable();
            break;
        case 2:
            TMR2_PWMDisable();
            break;
        case 3:
            TMR3_PWMDisable();
            break;
        default:
            break;
    }
}

static void tmr_route_dma_cfg(uint8_t ch, uint8_t s, uint32_t start_addr,
                              uint32_t end_addr, DMAModeTypeDef mode) {
    switch (ch) {
        case 0:
            TMR0_DMACfg(s, start_addr, end_addr, mode);
            break;
        case 1:
            TMR1_DMACfg(s, start_addr, end_addr, mode);
            break;
        case 2:
            TMR2_DMACfg(s, start_addr, end_addr, mode);
            break;
        case 3:
            TMR3_DMACfg(s, start_addr, end_addr, mode);
            break;
        default:
            break;
    }
}

void vp_tmr_pwm_init(BoardSignal sig, uint32_t cycle) {
    uint8_t ch = board_signal_get_channel(sig);
    tmr_route_pwm_cycle(ch, cycle);
    tmr_route_pwm_init(ch);
}

void vp_tmr_pwm_enable(BoardSignal sig) {
    uint8_t ch = board_signal_get_channel(sig);
    tmr_route_pwm_enable(ch);
    tmr_route_enable(ch);
}

void vp_tmr_pwm_disable(BoardSignal sig) {
    uint8_t ch = board_signal_get_channel(sig);
    tmr_route_pwm_disable(ch);
    tmr_route_disable(ch);
}

void vp_tmr_pwm_dma_cfg(BoardSignal sig, uint32_t start_addr, uint32_t end_addr,
                        uint8_t loop) {
    uint8_t ch = board_signal_get_channel(sig);
    tmr_route_dma_cfg(ch, ENABLE, start_addr, end_addr,
                      loop ? Mode_LOOP : Mode_Single);
}

void vp_tmr_pwm_dma_stop(BoardSignal sig) {
    uint8_t ch = board_signal_get_channel(sig);
    tmr_route_dma_cfg(ch, DISABLE, 0u, 0u, Mode_Single);
}

void vp_tmr_reset(BoardSignal sig) {
    switch (board_signal_get_channel(sig)) {
        case 0:
            R8_TMR0_CTRL_MOD = RB_TMR_ALL_CLEAR;
            break;
        case 1:
            R8_TMR1_CTRL_MOD = RB_TMR_ALL_CLEAR;
            break;
        case 2:
            R8_TMR2_CTRL_MOD = RB_TMR_ALL_CLEAR;
            break;
        case 3:
            R8_TMR3_CTRL_MOD = RB_TMR_ALL_CLEAR;
            break;
        default:
            break;
    }
}

void vp_tmr_pwm_set_polarity(BoardSignal sig, uint8_t active_low) {
    uint8_t ch = board_signal_get_channel(sig);
    switch (ch) {
        case 0:
            R8_TMR0_CTRL_MOD = RB_TMR_ALL_CLEAR;
            R8_TMR0_CTRL_MOD = (active_low << 4) | (PWM_Times_1 << 6);
            break;
        case 1:
            R8_TMR1_CTRL_MOD = RB_TMR_ALL_CLEAR;
            R8_TMR1_CTRL_MOD = (active_low << 4) | (PWM_Times_1 << 6);
            break;
        case 2:
            R8_TMR2_CTRL_MOD = RB_TMR_ALL_CLEAR;
            R8_TMR2_CTRL_MOD = (active_low << 4) | (PWM_Times_1 << 6);
            break;
        case 3:
            R8_TMR3_CTRL_MOD = RB_TMR_ALL_CLEAR;
            R8_TMR3_CTRL_MOD = (active_low << 4) | (PWM_Times_1 << 6);
            break;
        default:
            break;
    }
}

void vp_tmr_pwm_load_fifo(BoardSignal sig, uint32_t value) {
    switch (board_signal_get_channel(sig)) {
        case 0:
            TMR0_PWMActDataWidth(value);
            break;
        case 1:
            TMR1_PWMActDataWidth(value);
            break;
        case 2:
            TMR2_PWMActDataWidth(value);
            break;
        case 3:
            TMR3_PWMActDataWidth(value);
            break;
        default:
            break;
    }
}

// GPIO 运行时操作

vp_bool_t vp_gpio_is_valid(const BoardGpio gpio) {
    return (gpio.group == BOARD_GPIO_GROUP_A ||
            gpio.group == BOARD_GPIO_GROUP_B) &&
                   gpio.pin != 0u
               ? 1u
               : 0u;
}

vp_bool_t vp_gpio_read_level(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            return GPIOA_ReadPortPin(gpio.pin) ? 1u : 0u;
        case BOARD_GPIO_GROUP_B:
            return GPIOB_ReadPortPin(gpio.pin) ? 1u : 0u;
        default:
            return 0u;
    }
}

uint32_t vp_gpio_read_port(const BoardGpioGroup group) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            return GPIOA_ReadPort();
        case BOARD_GPIO_GROUP_B:
            return GPIOB_ReadPort();
        default:
            return 0u;
    }
}

void vp_gpio_set(const BoardGpio gpio) {
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

void vp_gpio_reset(const BoardGpio gpio) {
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

void vp_gpio_mode_cfg(const BoardGpio gpio, const GPIOModeTypeDef mode) {
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

void vp_gpio_mode_cfg_mask(const BoardGpioGroup group, const uint32_t pins,
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

void vp_gpio_digital_cfg(const BoardGpio gpio, const FunctionalState enable) {
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

void vp_gpio_digital_cfg_mask(const BoardGpioGroup  group,
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

void vp_gpio_it_mode_cfg(const BoardGpio gpio, const GPIOITModeTpDef mode) {
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

void vp_gpio_config_next_edge(const BoardGpio gpio) {
    vp_gpio_it_mode_cfg(gpio, vp_gpio_read_level(gpio) ? GPIO_ITMode_FallEdge
                                                       : GPIO_ITMode_RiseEdge);
}

void vp_gpio_clear_it_flag(const BoardGpio gpio) {
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

uint16_t vp_gpio_read_it_flag_port(const BoardGpioGroup group) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            return GPIOA_ReadITFlagPort();
        case BOARD_GPIO_GROUP_B:
            return GPIOB_ReadITFlagPort();
        default:
            return 0u;
    }
}

void vp_gpio_clear_it_flag_port(const BoardGpioGroup group,
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

uint16_t vp_gpio_read_int_enable_port(const BoardGpioGroup group) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            return R16_PA_INT_EN;
        case BOARD_GPIO_GROUP_B:
            return R16_PB_INT_EN;
        default:
            return 0u;
    }
}

void vp_gpio_prepare_level_rearm(const BoardGpio gpio) {
    switch (gpio.group) {
        case BOARD_GPIO_GROUP_A:
            R16_PA_INT_EN &= (uint16_t)(~gpio.pin);
            R16_PA_INT_MODE &= (uint16_t)(~gpio.pin);
            R32_PA_CLR |= gpio.pin;
            vp_gpio_clear_it_flag_port(
                BOARD_GPIO_GROUP_A,
                vp_gpio_read_it_flag_port(BOARD_GPIO_GROUP_A));
            vp_gpio_clear_it_flag(gpio);
            break;
        case BOARD_GPIO_GROUP_B:
            vp_gpio_clear_it_flag_port(
                BOARD_GPIO_GROUP_B,
                vp_gpio_read_it_flag_port(BOARD_GPIO_GROUP_B));
            vp_gpio_clear_it_flag(gpio);
            break;
        default:
            break;
    }
}

vp_status_t vp_gpio_int_mask(const BoardGpio gpio) {
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

vp_status_t vp_gpio_int_unmask(const BoardGpio gpio) {
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

void vp_gpio_irq_enable(const BoardGpio gpio) {
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

void vp_gpio_irq_clear_pending(const BoardGpioGroup group) {
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