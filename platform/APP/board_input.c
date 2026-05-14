#include "board_input.h"

#include "CH58x_common.h"  // IWYU pragma: keep
#include "vp_hal.h"
#include "c_api.h"
#include "rust_api.h"

static uint16_t board_input_exti_both_sim_mask_a = 0u;
static uint16_t board_input_exti_both_sim_mask_b = 0u;

static vp_bool_t active_low_gpio_level(const BoardGpio gpio) {
    return vp_gpio_read_level(gpio) ? 0u : 1u;
}

static uint16_t* board_input_exti_both_sim_mask_ptr(
    const BoardGpioGroup group) {
    switch (group) {
        case BOARD_GPIO_GROUP_A:
            return &board_input_exti_both_sim_mask_a;
        case BOARD_GPIO_GROUP_B:
            return &board_input_exti_both_sim_mask_b;
        default:
            return NULL;
    }
}

static vp_bool_t board_input_is_encoder(const vp_input_id_t input_id) {
    return input_id == VP_INPUT_ENCODER_A || input_id == VP_INPUT_ENCODER_B;
}

static vp_bool_t board_input_is_imu_int(const vp_input_id_t input_id) {
    return input_id == VP_INPUT_IMU_INT1 || input_id == VP_INPUT_IMU_INT2;
}

static vp_bool_t board_input_id_to_button_id(const vp_input_id_t input_id,
                                             vp_button_id_t* out_button_id) {
    if (out_button_id == NULL) {
        return 0u;
    }

    switch (input_id) {
        case VP_INPUT_LEFT:
            *out_button_id = VP_BUTTON_LEFT;
            return 1u;
        case VP_INPUT_RIGHT:
            *out_button_id = VP_BUTTON_RIGHT;
            return 1u;
        case VP_INPUT_MIDDLE:
            *out_button_id = VP_BUTTON_MIDDLE;
            return 1u;
        case VP_INPUT_ACTION:
            *out_button_id = VP_BUTTON_ACTION;
            return 1u;
        case VP_INPUT_LASER:
            *out_button_id = VP_BUTTON_LASER;
            return 1u;
        default:
            *out_button_id = 0u;
            return 0u;
    }
}

static vp_status_t board_input_map_exti_edge_to_mode(
    const vp_exti_edge_t edge, GPIOITModeTpDef* out_mode) {
    if (out_mode == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    switch (edge) {
        case VP_EXTI_EDGE_RISING:
            *out_mode = GPIO_ITMode_RiseEdge;
            return VP_STATUS_OK;
        case VP_EXTI_EDGE_FALLING:
            *out_mode = GPIO_ITMode_FallEdge;
            return VP_STATUS_OK;
        case VP_EXTI_EDGE_BOTH:
            return VP_STATUS_UNSUPPORTED;
        default:
            return VP_STATUS_INVALID_ARG;
    }
}

static vp_bool_t board_input_dispatch_one(const BoardGpio      gpio,
                                          const uint16_t       active_flags,
                                          const vp_input_id_t  input_id,
                                          const vp_timestamp_t timestamp) {
    if (!vp_gpio_is_valid(gpio) || (active_flags & gpio.pin) == 0u) {
        return 0u;
    }

    vp_gpio_clear_it_flag(gpio);

    if (board_input_is_encoder(input_id)) {
        const vp_bool_t a_level = active_low_gpio_level(board_signal_get(BOARD_SIGNAL_ENC_A));
        const vp_bool_t b_level = active_low_gpio_level(board_signal_get(BOARD_SIGNAL_ENC_B));
        vp_gpio_config_next_edge(gpio);
        vp_on_encoder_exti(a_level, b_level, timestamp);
        return 1u;
    }

    if (board_input_is_imu_int(input_id)) {
        (void)c_vp_exti_mask(input_id);
        vp_on_imu_int(timestamp);
        return 1u;
    }

    vp_button_id_t button_id = 0u;
    if (board_input_id_to_button_id(input_id, &button_id)) {
        const vp_bool_t level = active_low_gpio_level(gpio);
        (void)c_vp_exti_mask(input_id);
        vp_on_button_exti(button_id, level, timestamp);
        return 1u;
    }

    return 0u;
}

vp_bool_t board_input_id_to_gpio(const vp_input_id_t input_id,
                                 BoardGpio*          out_gpio) {
    if (out_gpio == NULL) {
        return 0u;
    }

    static const BoardSignal id_map[] = {
        [VP_INPUT_LEFT]        = BOARD_SIGNAL_BTN_LEFT,
        [VP_INPUT_RIGHT]       = BOARD_SIGNAL_BTN_RIGHT,
        [VP_INPUT_MIDDLE]      = BOARD_SIGNAL_BTN_MIDDLE,
        [VP_INPUT_ACTION]      = BOARD_SIGNAL_BTN_ACTION,
        [VP_INPUT_LASER]       = BOARD_SIGNAL_BTN_LASER,
        [VP_INPUT_MODE_SWITCH] = BOARD_SIGNAL_MODE_SWITCH,
        [VP_INPUT_ENCODER_A]   = BOARD_SIGNAL_ENC_A,
        [VP_INPUT_ENCODER_B]   = BOARD_SIGNAL_ENC_B,
        [VP_INPUT_IMU_INT1]    = BOARD_SIGNAL_IMU_INT1,
        [VP_INPUT_IMU_INT2]    = BOARD_SIGNAL_IMU_INT2,
    };

    if (input_id >= sizeof(id_map) / sizeof(id_map[0])) {
        return 0u;
    }

    *out_gpio = board_signal_get(id_map[input_id]);
    return board_signal_is_present(id_map[input_id]);
}

vp_status_t board_input_exti_unmask(const vp_input_id_t input_id,
                                    const BoardGpio     gpio) {
    const uint16_t* both_sim_mask =
        board_input_exti_both_sim_mask_ptr(gpio.group);
    if (both_sim_mask == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    if ((*both_sim_mask & gpio.pin) != 0u) {
        vp_gpio_clear_it_flag_port(gpio.group,
                                      vp_gpio_read_it_flag_port(gpio.group));
        vp_gpio_config_next_edge(gpio);
    } else {
        vp_button_id_t button_id = 0u;
        if (board_input_id_to_button_id(input_id, &button_id)) {
            vp_gpio_prepare_level_rearm(gpio);
        } else {
            vp_gpio_clear_it_flag_port(
                gpio.group, vp_gpio_read_it_flag_port(gpio.group));
            vp_gpio_clear_it_flag(gpio);
        }
    }

    (void)vp_gpio_int_unmask(gpio);
    vp_gpio_irq_enable(gpio);
    return VP_STATUS_OK;
}

vp_status_t board_input_exti_set_edge(const vp_input_id_t  input_id,
                                      const BoardGpio      gpio,
                                      const vp_exti_edge_t edge) {
    uint16_t* both_sim_mask = board_input_exti_both_sim_mask_ptr(gpio.group);
    if (both_sim_mask == NULL) {
        return VP_STATUS_INVALID_ARG;
    }

    if (edge == VP_EXTI_EDGE_BOTH) {
        if (!board_input_is_encoder(input_id)) {
            return VP_STATUS_UNSUPPORTED;
        }
        *both_sim_mask |= (uint16_t)gpio.pin;
        vp_gpio_config_next_edge(gpio);
        vp_gpio_irq_enable(gpio);
        return VP_STATUS_OK;
    }

    GPIOITModeTpDef   mode;
    const vp_status_t status = board_input_map_exti_edge_to_mode(edge, &mode);
    if (status != VP_STATUS_OK) {
        return status;
    }

    vp_button_id_t button_id = 0u;
    if (board_input_id_to_button_id(input_id, &button_id)) {
        if (edge == VP_EXTI_EDGE_FALLING) {
            mode = GPIO_ITMode_LowLevel;
        } else if (edge == VP_EXTI_EDGE_RISING) {
            mode = GPIO_ITMode_HighLevel;
        }
    }

    *both_sim_mask &= (uint16_t)(~gpio.pin);
    vp_gpio_it_mode_cfg(gpio, mode);
    vp_gpio_irq_enable(gpio);
    return VP_STATUS_OK;
}

vp_bool_t board_input_service_pending_group(const BoardGpioGroup group) {
    const uint16_t flags = vp_gpio_read_it_flag_port(group);
    const uint16_t active_flags =
        (uint16_t)(flags & vp_gpio_read_int_enable_port(group));

    if (active_flags == 0u) {
        if (flags != 0u) {
            vp_gpio_clear_it_flag_port(group, flags);
            vp_gpio_irq_clear_pending(group);
        }
        return 0u;
    }

    const vp_timestamp_t timestamp = c_vp_rtc_millis();
    vp_bool_t            handled = 0u;

    for (uint8_t input = VP_INPUT_LEFT; input <= VP_INPUT_IMU_INT2; input++) {
        BoardGpio gpio = {0};
        if (!board_input_id_to_gpio((vp_input_id_t)input, &gpio) ||
            gpio.group != group) {
            continue;
        }

        if (board_input_dispatch_one(gpio, active_flags, (vp_input_id_t)input,
                                     timestamp)) {
            handled = 1u;
        }
    }

    vp_gpio_irq_clear_pending(group);
    return handled;
}

vp_bool_t board_input_service_pending_all(void) {
    const vp_bool_t handled_a =
        board_input_service_pending_group(BOARD_GPIO_GROUP_A);
    const vp_bool_t handled_b =
        board_input_service_pending_group(BOARD_GPIO_GROUP_B);
    return handled_a || handled_b ? 1u : 0u;
}

__INTERRUPT
__HIGH_CODE
void GPIOA_IRQHandler(void) {
    (void)board_input_service_pending_group(BOARD_GPIO_GROUP_A);
}

__INTERRUPT
__HIGH_CODE
void GPIOB_IRQHandler(void) {
    (void)board_input_service_pending_group(BOARD_GPIO_GROUP_B);
}
