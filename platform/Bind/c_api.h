#ifndef VOIDPOINTER_C_API_H
#define VOIDPOINTER_C_API_H

#include <stdint.h>
#include <stdbool.h>

typedef struct {
    uint16_t x;
    uint16_t y;
    uint16_t z;
} sflp_game_rotation_raw_t;

bool read_sflp_game_rotation_raw(sflp_game_rotation_raw_t *raw);

#endif
