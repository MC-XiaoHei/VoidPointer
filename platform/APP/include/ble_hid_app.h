/********************************** (C) COPYRIGHT *******************************
 * File Name          : ble_hid_app.h
 * Author             : WCH
 * Version            : V1.0
 * Date               : 2018/12/10
 * Description        : BLE HID app TMOS glue 对外接口
 *********************************************************************************
 * Copyright (c) 2021 Nanjing Qinheng Microelectronics Co., Ltd.
 * Attention: This software (modified or not) and binary are used for
 * microcontroller manufactured by Nanjing Qinheng Microelectronics.
 *******************************************************************************/

#ifndef BLE_HID_APP_H
#define BLE_HID_APP_H

#ifdef __cplusplus
extern "C" {
#endif

// TMOS events owned by ble_hid_app glue.
#define START_DEVICE_EVT       0x0001
#define START_REPORT_EVT       0x0002
#define START_PARAM_UPDATE_EVT 0x0004
#define START_PHY_UPDATE_EVT   0x0008

extern void     BleHidApp_Init(void);
extern uint16_t BleHidApp_ProcessEvent(uint8_t task_id, uint16_t events);
extern uint8_t  BleHidApp_SetAdvertisingEnabled(uint8_t enabled);
extern uint8_t  BleHidApp_Disconnect(void);
extern uint8_t  BleHidApp_IsConnected(void);

#ifdef __cplusplus
}
#endif

#endif
