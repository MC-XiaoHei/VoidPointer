#ifndef VOIDPOINTER_BOARD_INPUT_H
#define VOIDPOINTER_BOARD_INPUT_H

#include "board_map.h"
#include "c_api.h"

#ifdef __cplusplus
extern "C" {
#endif

vp_bool_t   board_input_id_to_gpio(vp_input_id_t input_id, BoardGpio* out_gpio);
vp_status_t board_input_exti_set_edge(vp_input_id_t input_id, BoardGpio gpio,
                                      vp_exti_edge_t edge);
vp_status_t board_input_exti_unmask(vp_input_id_t input_id, BoardGpio gpio);
vp_bool_t   board_input_service_pending_group(BoardGpioGroup group);
vp_bool_t   board_input_service_pending_all(void);

#ifdef __cplusplus
}
#endif

#endif
