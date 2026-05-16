#ifndef VOIDPOINTER_LED_PLATFORM_H
#define VOIDPOINTER_LED_PLATFORM_H

#include <stdint.h>
#include "board_map.h"

#ifdef __cplusplus
extern "C" {
#endif

void led_init();
void led_play(BoardSignal sig, const uint8_t* data, uint16_t byte_len,
              uint8_t is_loop);
void led_stop();

#ifdef __cplusplus
}
#endif

#endif
