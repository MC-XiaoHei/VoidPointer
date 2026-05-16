#include "CONFIG.h"  // IWYU pragma: keep
#include "battservice.h"
#include "hiddev.h"
#include "ble_gap_policy.h"
#include "ble_hid_app.h"
#include "ble_hid_app_config.h"
#include "hidmouseservice.h"
#include "c_api.h"

static uint8_t ble_hid_task_id = INVALID_TASK_ID;

static uint8_t scan_rsp_data[] = {0x0D,
                                GAP_ADTYPE_LOCAL_NAME_COMPLETE,
                                'V',
                                'o',
                                'i',
                                'd',
                                ' ',
                                'P',
                                'o',
                                'i',
                                'n',
                                't',
                                'e',
                                'r',
                                0x05,
                                GAP_ADTYPE_SLAVE_CONN_INTERVAL_RANGE,
                                LO_UINT16(BLE_GAP_POLICY_CONN_INTERVAL_MIN),
                                HI_UINT16(BLE_GAP_POLICY_CONN_INTERVAL_MIN),
                                LO_UINT16(BLE_GAP_POLICY_CONN_INTERVAL_MAX),
                                HI_UINT16(BLE_GAP_POLICY_CONN_INTERVAL_MAX),
                                0x05,
                                GAP_ADTYPE_16BIT_MORE,
                                LO_UINT16(HID_SERV_UUID),
                                HI_UINT16(HID_SERV_UUID),
                                LO_UINT16(BATT_SERV_UUID),
                                HI_UINT16(BATT_SERV_UUID),
                                0x02,
                                GAP_ADTYPE_POWER_LEVEL,
                                0};

static uint8_t advert_data[] = {
    0x02,
    GAP_ADTYPE_FLAGS,
    GAP_ADTYPE_FLAGS_LIMITED | GAP_ADTYPE_FLAGS_BREDR_NOT_SUPPORTED,
    0x03,
    GAP_ADTYPE_APPEARANCE,
    LO_UINT16(GAP_APPEARE_HID_MOUSE),
    HI_UINT16(GAP_APPEARE_HID_MOUSE)};

static CONST uint8_t device_name[GAP_DEVICE_NAME_LEN] = "Void Pointer";

static hidDevCfg_t hid_dev_cfg = {BLE_HID_APP_DEFAULT_HID_IDLE_TIMEOUT,
                                   HID_FEATURE_FLAGS};

static void    init_advertising_params(void);
static void    init_bonding_params(void);
static void    init_battery_params(void);
static void    process_tmos_msg(tmos_event_hdr_t* pMsg);
static uint8_t hid_report_cb(uint8_t id, uint8_t type, uint16_t uuid,
                              uint8_t oper, uint16_t* pLen, uint8_t* pData);
static void    hid_event_cb(uint8_t evt);
static void gap_state_cb(gapRole_States_t newState, gapRoleEvent_t* pEvent);

static hidDevCB_t hid_dev_callbacks = {hid_report_cb, hid_event_cb, NULL,
                                     gap_state_cb};

void ble_hid_init(void) {
    ble_hid_task_id = TMOS_ProcessEventRegister(ble_hid_process_event);
    ble_init(ble_hid_task_id);

    init_advertising_params();
    GGS_SetParameter(GGS_DEVICE_NAME_ATT, GAP_DEVICE_NAME_LEN,
                     (void*)device_name);
    init_bonding_params();
    init_battery_params();

    Hid_AddService();
    HidDev_Register(&hid_dev_cfg, &hid_dev_callbacks);

    tmos_set_event(ble_hid_task_id, START_DEVICE_EVT);
}

uint8_t ble_hid_set_advertising(uint8_t enabled) {
    return ble_set_advertising(enabled);
}

uint8_t ble_hid_disconnect(void) { return ble_disconnect(); }

uint8_t ble_hid_is_connected(void) { return ble_is_connected(); }

uint16_t ble_hid_process_event(uint8_t task_id, uint16_t events) {
    (void)task_id;

    if (events & SYS_EVENT_MSG) {
        uint8_t* pMsg = tmos_msg_receive(ble_hid_task_id);
        if (pMsg != NULL) {
            process_tmos_msg((tmos_event_hdr_t*)pMsg);
            tmos_msg_deallocate(pMsg);
        }
        return events ^ SYS_EVENT_MSG;
    }

    if (events & START_DEVICE_EVT) {
        return events ^ START_DEVICE_EVT;
    }

    if (events & START_PARAM_UPDATE_EVT) {
        VP_LOG_DEBUG("ble_hid", "conn param update requested");
        GAPRole_PeripheralConnParamUpdateReq(
            ble_conn_handle(),
            BLE_GAP_POLICY_CONN_INTERVAL_MIN, BLE_GAP_POLICY_CONN_INTERVAL_MAX,
            BLE_GAP_POLICY_CONN_LATENCY, BLE_GAP_POLICY_CONN_TIMEOUT,
            ble_hid_task_id);
        return events ^ START_PARAM_UPDATE_EVT;
    }

    if (events & START_PHY_UPDATE_EVT) {
        VP_LOG_DEBUG("ble_hid", "phy update requested;status=0x%02x",
            GAPRole_UpdatePHY(ble_conn_handle(), 0,
                              GAP_PHY_BIT_LE_2M, GAP_PHY_BIT_LE_2M, 0));
        return events ^ START_PHY_UPDATE_EVT;
    }

    return 0;
}

static void init_advertising_params(void) {
    uint8_t enable = BLE_HID_APP_DEFAULT_ADVERTISING_ENABLED;

    GAPRole_SetParameter(GAPROLE_ADVERT_ENABLED, sizeof(enable), &enable);
    GAPRole_SetParameter(GAPROLE_ADVERT_DATA, sizeof(advert_data), advert_data);
    GAPRole_SetParameter(GAPROLE_SCAN_RSP_DATA, sizeof(scan_rsp_data),
                         scan_rsp_data);
}

static void init_bonding_params(void) {
    uint32_t passkey = BLE_HID_APP_DEFAULT_PASSCODE;
    uint8_t  mode    = BLE_HID_APP_DEFAULT_PAIRING_MODE;
    uint8_t  mitm    = BLE_HID_APP_DEFAULT_MITM_MODE;
    uint8_t  io      = BLE_HID_APP_DEFAULT_IO_CAPABILITIES;
    uint8_t  bond    = BLE_HID_APP_DEFAULT_BONDING_MODE;

    GAPBondMgr_SetParameter(GAPBOND_PERI_DEFAULT_PASSCODE, sizeof(passkey),
                            &passkey);
    GAPBondMgr_SetParameter(GAPBOND_PERI_PAIRING_MODE, sizeof(mode), &mode);
    GAPBondMgr_SetParameter(GAPBOND_PERI_MITM_PROTECTION, sizeof(mitm), &mitm);
    GAPBondMgr_SetParameter(GAPBOND_PERI_IO_CAPABILITIES, sizeof(io), &io);
    GAPBondMgr_SetParameter(GAPBOND_PERI_BONDING_ENABLED, sizeof(bond), &bond);
}

static void init_battery_params(void) {
    uint8_t level = BLE_HID_APP_DEFAULT_BATT_CRITICAL_LEVEL;
    Batt_SetParameter(BATT_PARAM_CRITICAL_LEVEL, sizeof(level), &level);
}

static void process_tmos_msg(tmos_event_hdr_t* pMsg) {
    switch (pMsg->event) {
        default:
            break;
    }
}

static uint8_t hid_report_cb(uint8_t id, uint8_t type, uint16_t uuid,
                              uint8_t oper, uint16_t* pLen, uint8_t* pData) {
    uint8_t status = SUCCESS;

    if (oper == HID_DEV_OPER_WRITE) {
        status = Hid_SetParameter(id, type, uuid, *pLen, pData);
    } else if (oper == HID_DEV_OPER_READ) {
        status = Hid_GetParameter(id, type, uuid, pLen, pData);
    } else if (oper == HID_DEV_OPER_ENABLE) {
        ble_on_notify_enabled(id, type, uuid);
    }

    return status;
}

static void hid_event_cb(uint8_t evt) {
    switch (evt) {
        case HID_DEV_SUSPEND_EVT:
            VP_LOG_INFO("ble_hid", "hid state changed;state=suspended");
            break;
        case HID_DEV_EXIT_SUSPEND_EVT:
            VP_LOG_INFO("ble_hid", "hid state changed;state=resumed");
            break;
        default:
            break;
    }
}

static void gap_state_cb(gapRole_States_t newState,
                             gapRoleEvent_t*  pEvent) {
    ble_on_state_change(newState, pEvent);
}
