/********************************** (C) COPYRIGHT *******************************
 * File Name          : ble_gap_policy.h
 * Description        : BLE GAP / advertising / connection policy state
 *******************************************************************************/

#ifndef BLE_GAP_POLICY_H
#define BLE_GAP_POLICY_H

#ifdef __cplusplus
extern "C" {
#endif

#include "CONFIG.h"

void     BleGapPolicy_Init(uint8_t task_id);
uint8_t  BleGapPolicy_SetAdvertisingEnabled(uint8_t enabled);
uint8_t  BleGapPolicy_Disconnect(void);
uint8_t  BleGapPolicy_IsConnected(void);
uint16_t BleGapPolicy_GetConnectionHandle(void);
void     BleGapPolicy_HandleGapState(gapRole_States_t newState,
                                     gapRoleEvent_t*  pEvent);
void     BleGapPolicy_HandleReportNotifyEnabled(uint8_t id, uint8_t type,
                                                uint16_t uuid);
void     BleGapPolicy_ApplyAdvertising(void);

#ifdef __cplusplus
}
#endif

#endif
