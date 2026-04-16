#include "c_api.h"

#include <lsm6dsv.h>

bool read_sflp_game_rotation_raw(sflp_game_rotation_raw_t *raw) {
    return LSM6DSV_ReadSFLPGameRotationRaw(raw);
}