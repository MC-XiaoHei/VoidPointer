/********************************** (C) COPYRIGHT *******************************
 * File Name          : ble_gap_policy.c
 * Description        : BLE GAP / advertising / connection policy state
 *******************************************************************************/

#include "CONFIG.h"  // IWYU pragma: keep
#include "ble_gap_policy.h"
#include "ble_hid_app.h"
#include "ble_hid_app_config.h"
#include "c_api.h"

#include "hiddev.h"
#include "hidmouseservice.h"
#include "rust_api.h"

static uint8_t          bleGapPolicyTaskId = INVALID_TASK_ID;
static uint16_t         bleGapPolicyConnHandle = GAP_CONNHANDLE_INIT;
static uint8_t          bleGapPolicyAdvertisingAllowed = TRUE;
static gapRole_States_t bleGapPolicyGapState = GAPROLE_INIT;
static uint8_t          bleGapPolicyGapStarted = FALSE;

void BleGapPolicy_Init(uint8_t task_id) {
    bleGapPolicyTaskId = task_id;
    bleGapPolicyAdvertisingAllowed = TRUE;
    bleGapPolicyConnHandle = GAP_CONNHANDLE_INIT;
    bleGapPolicyGapState = GAPROLE_INIT;
    bleGapPolicyGapStarted = FALSE;
}

uint8_t BleGapPolicy_SetAdvertisingEnabled(uint8_t enabled) {
    bleGapPolicyAdvertisingAllowed = enabled ? TRUE : FALSE;
    VP_LOG_DEBUG(
        "ble_gap",
        "advertising policy updated;allowed=%u,state=%u,started=%u,handle=%u",
        bleGapPolicyAdvertisingAllowed, bleGapPolicyGapState,
        bleGapPolicyGapStarted, bleGapPolicyConnHandle);
    BleGapPolicy_ApplyAdvertising();
    return SUCCESS;
}

uint8_t BleGapPolicy_Disconnect(void) {
    if (bleGapPolicyConnHandle == GAP_CONNHANDLE_INIT) {
        return SUCCESS;
    }

    return GAPRole_TerminateLink(bleGapPolicyConnHandle);
}

uint8_t BleGapPolicy_IsConnected(void) {
    return bleGapPolicyConnHandle != GAP_CONNHANDLE_INIT ? TRUE : FALSE;
}

uint16_t BleGapPolicy_GetConnectionHandle(void) {
    return bleGapPolicyConnHandle;
}

void BleGapPolicy_ApplyAdvertising(void) {
    if (!bleGapPolicyGapStarted) {
        return;
    }

    if ((bleGapPolicyGapState & GAPROLE_STATE_ADV_MASK) == GAPROLE_CONNECTED ||
        (bleGapPolicyGapState & GAPROLE_STATE_ADV_MASK) ==
            GAPROLE_CONNECTED_ADV) {
        return;
    }

    GAPRole_SetParameter(GAPROLE_ADVERT_ENABLED, sizeof(uint8_t),
                         &bleGapPolicyAdvertisingAllowed);
}

void BleGapPolicy_HandleGapState(gapRole_States_t newState,
                                 gapRoleEvent_t*  pEvent) {
    bleGapPolicyGapState = newState;

    switch (newState & GAPROLE_STATE_ADV_MASK) {
        case GAPROLE_STARTED: {
            uint8_t ownAddr[6];
            bleGapPolicyGapStarted = TRUE;
            GAPRole_GetParameter(GAPROLE_BD_ADDR, ownAddr);
            GAP_ConfigDeviceAddr(ADDRTYPE_STATIC, ownAddr);
            VP_LOG_INFO("ble_gap", "gap started");
            BleGapPolicy_ApplyAdvertising();
        } break;

        case GAPROLE_ADVERTISING:
            VP_LOG_DEBUG("ble_gap", "gap state changed;state=advertising");
            break;

        case GAPROLE_CONNECTED:
            if (pEvent->gap.opcode == GAP_LINK_ESTABLISHED_EVENT) {
                gapEstLinkReqEvent_t* event = (gapEstLinkReqEvent_t*)pEvent;

                bleGapPolicyConnHandle = event->connectionHandle;
                VP_LOG_INFO("ble_gap", "connected;handle=%u",
                            bleGapPolicyConnHandle);
                vp_on_ble_connected(c_vp_rtc_millis());
#if BLE_GAP_POLICY_PARAM_UPDATE_ENABLED
                tmos_start_task(bleGapPolicyTaskId, START_PARAM_UPDATE_EVT,
                                BLE_GAP_POLICY_PARAM_UPDATE_DELAY_MS);
#endif
            }
            break;

        case GAPROLE_CONNECTED_ADV:
            break;

        case GAPROLE_WAITING:
            if (pEvent->gap.opcode == GAP_LINK_TERMINATED_EVENT) {
#if BLE_GAP_POLICY_PARAM_UPDATE_ENABLED
                tmos_stop_task(bleGapPolicyTaskId, START_PARAM_UPDATE_EVT);
#endif
                VP_LOG_INFO("ble_gap", "disconnected;reason=0x%02x",
                            pEvent->linkTerminate.reason);
                bleGapPolicyConnHandle = GAP_CONNHANDLE_INIT;
                vp_on_ble_disconnected(pEvent->linkTerminate.reason,
                                       c_vp_rtc_millis());
            }
            BleGapPolicy_ApplyAdvertising();
            break;

        case GAPROLE_ERROR:
            bleGapPolicyConnHandle = GAP_CONNHANDLE_INIT;
            VP_LOG_ERROR("ble_gap", "gap role error");
            break;

        default:
            break;
    }
}

void BleGapPolicy_HandleReportNotifyEnabled(uint8_t id, uint8_t type,
                                            uint16_t uuid) {
    if (uuid == GATT_CLIENT_CHAR_CFG_UUID && id == HID_RPT_ID_MOUSE_IN &&
        type == HID_REPORT_TYPE_INPUT) {
        vp_on_ble_input_ready(c_vp_rtc_millis());
    }
}
