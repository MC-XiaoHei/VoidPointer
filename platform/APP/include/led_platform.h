#ifndef VOIDPOINTER_LED_PLATFORM_H
#define VOIDPOINTER_LED_PLATFORM_H

#include <stdint.h>
#include "board_map.h"

#ifdef __cplusplus
extern "C" {
#endif

void LedPlatform_Init(void);
void LedPlatform_Play(BoardSignal sig, const uint8_t* data, uint16_t byte_len,
                     uint8_t is_loop);
void LedPlatform_Stop(void);

#ifdef __cplusplus
}
#endif

#endif
