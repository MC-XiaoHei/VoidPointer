#ifndef BLE_GAP_POLICY_H
#define BLE_GAP_POLICY_H

#ifdef __cplusplus
extern "C" {
#endif

#include "CONFIG.h"

void     ble_init(uint8_t task_id);
uint8_t  ble_set_advertising(uint8_t enabled);
uint8_t  ble_disconnect();
uint8_t  ble_is_connected();
uint16_t ble_conn_handle();
void     ble_on_state_change(gapRole_States_t newState, gapRoleEvent_t* pEvent);
void     ble_on_notify_enabled(uint8_t id, uint8_t type, uint16_t uuid);
void     ble_advert_apply();

#ifdef __cplusplus
}
#endif

#endif
