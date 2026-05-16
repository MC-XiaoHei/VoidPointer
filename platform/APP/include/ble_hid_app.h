#ifndef BLE_HID_APP_H
#define BLE_HID_APP_H

#ifdef __cplusplus
extern "C" {
#endif

// 这些事件位由 ble_hid_app glue 自己持有，外部只负责触发，不解释内部时序
#define START_DEVICE_EVT       0x0001
#define START_REPORT_EVT       0x0002
#define START_PARAM_UPDATE_EVT 0x0004
#define START_PHY_UPDATE_EVT   0x0008

extern void     ble_hid_init();
extern uint16_t ble_hid_process_event(uint8_t task_id, uint16_t events);
extern uint8_t  ble_hid_set_advertising(uint8_t enabled);
extern uint8_t  ble_hid_disconnect();
extern uint8_t  ble_hid_is_connected();

#ifdef __cplusplus
}
#endif

#endif
