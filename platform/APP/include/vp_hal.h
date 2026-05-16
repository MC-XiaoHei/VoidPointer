#ifndef VOIDPOINTER_VP_HAL_H
#define VOIDPOINTER_VP_HAL_H

#include <stdint.h>
#include "board_map.h"
#include "CH58x_common.h"
#include "c_api.h"

#ifdef __cplusplus
extern "C" {
#endif

// PWMX
void vp_pwm_init(BoardSignal sig, uint16_t cycle);
void vp_pwm_set_duty(BoardSignal sig, uint8_t duty);

// TMR PWM（用于 LED）
void vp_tmr_pwm_init(BoardSignal sig, uint32_t cycle);
void vp_tmr_pwm_enable(BoardSignal sig);
void vp_tmr_pwm_disable(BoardSignal sig);
void vp_tmr_pwm_dma_cfg(BoardSignal sig, uint32_t start_addr, uint32_t end_addr, uint8_t loop);
void vp_tmr_pwm_dma_stop(BoardSignal sig);
void vp_tmr_reset(BoardSignal sig);
void vp_tmr_pwm_set_polarity(BoardSignal sig, uint8_t active_low);
void vp_tmr_pwm_load_fifo(BoardSignal sig, uint32_t value);

// 外设常量（隔离 CH58x HAL 类型）
#define VP_PWMX_CYCLE_256   ((uint16_t)256)

// GPIO 运行时操作
vp_bool_t vp_gpio_is_valid(BoardGpio gpio);
vp_bool_t vp_gpio_read_level(BoardGpio gpio);
uint32_t  vp_gpio_read_port(BoardGpioGroup group);
void      vp_gpio_set(BoardGpio gpio);
void      vp_gpio_reset(BoardGpio gpio);
void      vp_gpio_mode_cfg(BoardGpio gpio, GPIOModeTypeDef mode);
void      vp_gpio_mode_cfg_mask(BoardGpioGroup group, uint32_t pins, GPIOModeTypeDef mode);
void      vp_gpio_digital_cfg(BoardGpio gpio, FunctionalState enable);
void      vp_gpio_digital_cfg_mask(BoardGpioGroup group, FunctionalState enable, uint32_t pins);
void      vp_gpio_it_mode_cfg(BoardGpio gpio, GPIOITModeTpDef mode);
void      vp_gpio_config_next_edge(BoardGpio gpio);
void      vp_gpio_clear_it_flag(BoardGpio gpio);
uint16_t  vp_gpio_read_it_flag_port(BoardGpioGroup group);
void      vp_gpio_clear_it_flag_port(BoardGpioGroup group, uint16_t flags);
uint16_t  vp_gpio_read_int_enable_port(BoardGpioGroup group);
void      vp_gpio_prepare_level_rearm(BoardGpio gpio);
vp_status_t vp_gpio_int_mask(BoardGpio gpio);
vp_status_t vp_gpio_int_unmask(BoardGpio gpio);
void      vp_gpio_irq_enable(BoardGpio gpio);
void      vp_gpio_irq_clear_pending(BoardGpioGroup group);

#ifdef __cplusplus
}
#endif

#endif