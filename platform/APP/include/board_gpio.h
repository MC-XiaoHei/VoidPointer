#ifndef VOIDPOINTER_BOARD_GPIO_H
#define VOIDPOINTER_BOARD_GPIO_H

#include "board_map.h"
#include "CH58x_common.h"  // IWYU pragma: keep
#include "c_api.h"

#ifdef __cplusplus
extern "C" {
#endif

vp_bool_t board_gpio_is_valid(BoardGpio gpio);
vp_bool_t board_gpio_read_level(BoardGpio gpio);
uint32_t  board_gpio_read_port(BoardGpioGroup group);
void      board_gpio_set(BoardGpio gpio);
void      board_gpio_reset(BoardGpio gpio);
void      board_gpio_mode_cfg(BoardGpio gpio, GPIOModeTypeDef mode);
void      board_gpio_mode_cfg_mask(BoardGpioGroup group, uint32_t pins,
                                   GPIOModeTypeDef mode);
void      board_gpio_digital_cfg(BoardGpio gpio, FunctionalState enable);
void board_gpio_digital_cfg_mask(BoardGpioGroup group, FunctionalState enable,
                                 uint32_t pins);
void board_gpio_it_mode_cfg(BoardGpio gpio, GPIOITModeTpDef mode);
void board_gpio_config_next_edge(BoardGpio gpio);
void board_gpio_clear_it_flag(BoardGpio gpio);
uint16_t    board_gpio_read_it_flag_port(BoardGpioGroup group);
void        board_gpio_clear_it_flag_port(BoardGpioGroup group, uint16_t flags);
uint16_t    board_gpio_read_int_enable_port(BoardGpioGroup group);
void        board_gpio_prepare_level_rearm(BoardGpio gpio);
vp_status_t board_gpio_int_mask(BoardGpio gpio);
vp_status_t board_gpio_int_unmask(BoardGpio gpio);
void        board_gpio_irq_enable(BoardGpio gpio);
void        board_gpio_irq_clear_pending(BoardGpioGroup group);

#ifdef __cplusplus
}
#endif

#endif
