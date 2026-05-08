/********************************** (C) COPYRIGHT *******************************
 * File Name          : ble_hid_app.c
 * Author             : WCH
 * Version            : V1.0
 * Date               : 2018/12/10
 * Description        : BLE HID app TMOS glue，负责任务注册与 HID callback 接入
 *********************************************************************************
 * Copyright (c) 2021 Nanjing Qinheng Microelectronics Co., Ltd.
 * Attention: This software (modified or not) and binary are used for
 * microcontroller manufactured by Nanjing Qinheng Microelectronics.
 *******************************************************************************/

#include "CONFIG.h"  // IWYU pragma: keep
#include "battservice.h"
#include "hiddev.h"
#include "ble_gap_policy.h"
#include "ble_hid_app.h"
#include "ble_hid_app_config.h"
#include "hidmouseservice.h"
#include "c_api.h"

static uint8_t bleHidAppTaskId = INVALID_TASK_ID;

static uint8_t scanRspData[] = {0x0D,
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

static uint8_t advertData[] = {
    0x02,
    GAP_ADTYPE_FLAGS,
    GAP_ADTYPE_FLAGS_LIMITED | GAP_ADTYPE_FLAGS_BREDR_NOT_SUPPORTED,
    0x03,
    GAP_ADTYPE_APPEARANCE,
    LO_UINT16(GAP_APPEARE_HID_MOUSE),
    HI_UINT16(GAP_APPEARE_HID_MOUSE)};

static CONST uint8_t attDeviceName[GAP_DEVICE_NAME_LEN] = "Void Pointer";

static hidDevCfg_t bleHidAppCfg = {BLE_HID_APP_DEFAULT_HID_IDLE_TIMEOUT,
                                   HID_FEATURE_FLAGS};

static void    bleHidApp_ConfigureGapRole(void);
static void    bleHidApp_ConfigureBondManager(void);
static void    bleHidApp_ConfigureBatteryService(void);
static void    bleHidApp_ProcessTMOSMsg(tmos_event_hdr_t* pMsg);
static uint8_t bleHidAppRptCB(uint8_t id, uint8_t type, uint16_t uuid,
                              uint8_t oper, uint16_t* pLen, uint8_t* pData);
static void    bleHidAppEvtCB(uint8_t evt);
static void bleHidAppStateCB(gapRole_States_t newState, gapRoleEvent_t* pEvent);

static hidDevCB_t bleHidAppHidCBs = {bleHidAppRptCB, bleHidAppEvtCB, NULL,
                                     bleHidAppStateCB};

void BleHidApp_Init() {
    bleHidAppTaskId = TMOS_ProcessEventRegister(BleHidApp_ProcessEvent);
    BleGapPolicy_Init(bleHidAppTaskId);

    bleHidApp_ConfigureGapRole();
    GGS_SetParameter(GGS_DEVICE_NAME_ATT, GAP_DEVICE_NAME_LEN,
                     (void*)attDeviceName);
    bleHidApp_ConfigureBondManager();
    bleHidApp_ConfigureBatteryService();

    Hid_AddService();
    HidDev_Register(&bleHidAppCfg, &bleHidAppHidCBs);
    tmos_set_event(bleHidAppTaskId, START_DEVICE_EVT);
}

uint8_t BleHidApp_SetAdvertisingEnabled(uint8_t enabled) {
    return BleGapPolicy_SetAdvertisingEnabled(enabled);
}

uint8_t BleHidApp_Disconnect(void) { return BleGapPolicy_Disconnect(); }

uint8_t BleHidApp_IsConnected(void) { return BleGapPolicy_IsConnected(); }

uint16_t BleHidApp_ProcessEvent(uint8_t task_id, uint16_t events) {
    (void)task_id;

    if (events & SYS_EVENT_MSG) {
        uint8_t* pMsg;

        if ((pMsg = tmos_msg_receive(bleHidAppTaskId)) != NULL) {
            bleHidApp_ProcessTMOSMsg((tmos_event_hdr_t*)pMsg);
            tmos_msg_deallocate(pMsg);
        }

        return (events ^ SYS_EVENT_MSG);
    }

    if (events & START_DEVICE_EVT) {
        return (events ^ START_DEVICE_EVT);
    }

    if (events & START_PARAM_UPDATE_EVT) {
        VP_LOG_DEBUG("ble_hid", "conn param update requested");
        GAPRole_PeripheralConnParamUpdateReq(
            BleGapPolicy_GetConnectionHandle(),
            BLE_GAP_POLICY_CONN_INTERVAL_MIN, BLE_GAP_POLICY_CONN_INTERVAL_MAX,
            BLE_GAP_POLICY_CONN_LATENCY, BLE_GAP_POLICY_CONN_TIMEOUT,
            bleHidAppTaskId);
        return (events ^ START_PARAM_UPDATE_EVT);
    }

    if (events & START_PHY_UPDATE_EVT) {
        VP_LOG_DEBUG(
            "ble_hid", "phy update requested;status=0x%02x",
            GAPRole_UpdatePHY(BleGapPolicy_GetConnectionHandle(), 0,
                              GAP_PHY_BIT_LE_2M, GAP_PHY_BIT_LE_2M, 0));
        return (events ^ START_PHY_UPDATE_EVT);
    }

    return 0;
}

static void bleHidApp_ConfigureGapRole(void) {
    uint8_t initialAdvertisingEnable = BLE_HID_APP_DEFAULT_ADVERTISING_ENABLED;

    GAPRole_SetParameter(GAPROLE_ADVERT_ENABLED, sizeof(uint8_t),
                         &initialAdvertisingEnable);
    GAPRole_SetParameter(GAPROLE_ADVERT_DATA, sizeof(advertData), advertData);
    GAPRole_SetParameter(GAPROLE_SCAN_RSP_DATA, sizeof(scanRspData),
                         scanRspData);
}

static void bleHidApp_ConfigureBondManager(void) {
    uint32_t passkey = BLE_HID_APP_DEFAULT_PASSCODE;
    uint8_t  pairMode = BLE_HID_APP_DEFAULT_PAIRING_MODE;
    uint8_t  mitm = BLE_HID_APP_DEFAULT_MITM_MODE;
    uint8_t  ioCap = BLE_HID_APP_DEFAULT_IO_CAPABILITIES;
    uint8_t  bonding = BLE_HID_APP_DEFAULT_BONDING_MODE;

    GAPBondMgr_SetParameter(GAPBOND_PERI_DEFAULT_PASSCODE, sizeof(uint32_t),
                            &passkey);
    GAPBondMgr_SetParameter(GAPBOND_PERI_PAIRING_MODE, sizeof(uint8_t),
                            &pairMode);
    GAPBondMgr_SetParameter(GAPBOND_PERI_MITM_PROTECTION, sizeof(uint8_t),
                            &mitm);
    GAPBondMgr_SetParameter(GAPBOND_PERI_IO_CAPABILITIES, sizeof(uint8_t),
                            &ioCap);
    GAPBondMgr_SetParameter(GAPBOND_PERI_BONDING_ENABLED, sizeof(uint8_t),
                            &bonding);
}

static void bleHidApp_ConfigureBatteryService(void) {
    uint8_t critical = BLE_HID_APP_DEFAULT_BATT_CRITICAL_LEVEL;
    Batt_SetParameter(BATT_PARAM_CRITICAL_LEVEL, sizeof(uint8_t), &critical);
}

static void bleHidApp_ProcessTMOSMsg(tmos_event_hdr_t* pMsg) {
    switch (pMsg->event) {
        default:
            break;
    }
}

static uint8_t bleHidAppRptCB(uint8_t id, uint8_t type, uint16_t uuid,
                              uint8_t oper, uint16_t* pLen, uint8_t* pData) {
    uint8_t status = SUCCESS;

    if (oper == HID_DEV_OPER_WRITE) {
        status = Hid_SetParameter(id, type, uuid, *pLen, pData);
    } else if (oper == HID_DEV_OPER_READ) {
        status = Hid_GetParameter(id, type, uuid, pLen, pData);
    } else if (oper == HID_DEV_OPER_ENABLE) {
        BleGapPolicy_HandleReportNotifyEnabled(id, type, uuid);
    }

    return status;
}

static void bleHidAppEvtCB(uint8_t evt) {
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

static void bleHidAppStateCB(gapRole_States_t newState,
                             gapRoleEvent_t*  pEvent) {
    BleGapPolicy_HandleGapState(newState, pEvent);
}
