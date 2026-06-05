#include "board_input.h"

#include "CH58x_common.h"  // IWYU pragma: keep
#include "vp_hal.h"
#include "c_api.h"
#include "rust_api.h"

static uint16_t board_input_exti_both_sim_mask_a = 0u;
static uint16_t board_input_exti_both_sim_mask_b = 0u;

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
    (void)input_id;
    return 0u;
}

static vp_bool_t board_input_is_imu_int(const vp_input_id_t input_id) {
    return input_id == VP_INPUT_IMU_INT1 || input_id == VP_INPUT_IMU_INT2;
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
        // 编码器已移除，保留避免 C API 符号缺失
        vp_gpio_config_next_edge(gpio);
        vp_on_encoder_exti(0u, 0u, timestamp);
        return 1u;
    }

    if (board_input_is_imu_int(input_id)) {
        (void)c_vp_exti_mask(input_id);
        vp_on_imu_int(timestamp);
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
        [VP_INPUT_CONTEXT] = BOARD_SIGNAL_CONTEXT_BUTTON,
        [VP_INPUT_ACTION] = BOARD_SIGNAL_ACT_BUTTON,
        [VP_INPUT_UP] = BOARD_SIGNAL_UP_BUTTON,
        [VP_INPUT_DOWN] = BOARD_SIGNAL_DOWN_BUTTON,
        [VP_INPUT_PRIMARY] = BOARD_SIGNAL_LEFT_BUTTON,
        [VP_INPUT_SECONDARY] = BOARD_SIGNAL_RIGHT_BUTTON,
        [VP_INPUT_MODE_SWITCH] = BOARD_SIGNAL_MODE_SWITCH,
        [VP_INPUT_PROFILE_SWITCH] = BOARD_SIGNAL_PROFILE_SWITCH,
        [VP_INPUT_IMU_INT1] = BOARD_SIGNAL_IMU_INT1,
        [VP_INPUT_IMU_INT2] = BOARD_SIGNAL_IMU_INT2,
    };

    if (input_id >= sizeof(id_map) / sizeof(id_map[0])) {
        return 0u;
    }

    *out_gpio = board_signal_get(id_map[input_id]);
    return board_signal_is_present(id_map[input_id]);
}

vp_status_t board_input_exti_unmask(const vp_input_id_t input_id,
                                    const BoardGpio     gpio) {
    (void)input_id;
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
        vp_gpio_clear_it_flag_port(gpio.group,
                                   vp_gpio_read_it_flag_port(gpio.group));
        vp_gpio_clear_it_flag(gpio);
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

    for (uint8_t input = 0u; input <= VP_INPUT_IMU_INT2; input++) {
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
