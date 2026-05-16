#include "CONFIG.h"  // IWYU pragma: keep
#include "ble_gap_policy.h"
#include "ble_hid_app.h"
#include "ble_hid_app_config.h"
#include "c_api.h"

#include "hiddev.h"
#include "hidmouseservice.h"
#include "rust_api.h"

static uint8_t          ble_task_id = INVALID_TASK_ID;
static uint16_t         conn_handle = GAP_CONNHANDLE_INIT;
static uint8_t          ble_advert_allowed = TRUE;
static gapRole_States_t ble_gap_state = GAPROLE_INIT;
static uint8_t          ble_gap_started = FALSE;

void ble_init(uint8_t task_id) {
    ble_task_id = task_id;
    ble_advert_allowed = TRUE;
    conn_handle = GAP_CONNHANDLE_INIT;
    ble_gap_state = GAPROLE_INIT;
    ble_gap_started = FALSE;
}

uint8_t ble_set_advertising(uint8_t enabled) {
    ble_advert_allowed = enabled ? TRUE : FALSE;
    VP_LOG_DEBUG(
        "ble_gap",
        "advertising policy changed;allowed=%u,state=%u,started=%u,handle=%u",
        ble_advert_allowed, ble_gap_state,
        ble_gap_started, conn_handle);
    ble_advert_apply();
    return SUCCESS;
}

uint8_t ble_disconnect() {
    if (conn_handle == GAP_CONNHANDLE_INIT) {
        return SUCCESS;
    }

    return GAPRole_TerminateLink(conn_handle);
}

uint8_t ble_is_connected() {
    return conn_handle != GAP_CONNHANDLE_INIT ? TRUE : FALSE;
}

uint16_t ble_conn_handle() {
    return conn_handle;
}

void ble_advert_apply() {
    if (!ble_gap_started) {
        return;
    }

    if ((ble_gap_state & GAPROLE_STATE_ADV_MASK) == GAPROLE_CONNECTED ||
        (ble_gap_state & GAPROLE_STATE_ADV_MASK) ==
            GAPROLE_CONNECTED_ADV) {
        return;
    }

    GAPRole_SetParameter(GAPROLE_ADVERT_ENABLED, sizeof(uint8_t),
                         &ble_advert_allowed);
}

void ble_on_state_change(gapRole_States_t newState,
                                 gapRoleEvent_t*  pEvent) {
    ble_gap_state = newState;

    switch (newState & GAPROLE_STATE_ADV_MASK) {
        case GAPROLE_STARTED: {
            uint8_t ownAddr[6];
            ble_gap_started = TRUE;
            GAPRole_GetParameter(GAPROLE_BD_ADDR, ownAddr);
            GAP_ConfigDeviceAddr(ADDRTYPE_STATIC, ownAddr);
            VP_LOG_INFO("ble_gap", "initialized");
            ble_advert_apply();
        } break;

        case GAPROLE_ADVERTISING:
            VP_LOG_DEBUG("ble_gap", "gap state changed;state=advertising");
            break;

        case GAPROLE_CONNECTED:
            if (pEvent->gap.opcode == GAP_LINK_ESTABLISHED_EVENT) {
                gapEstLinkReqEvent_t* event = (gapEstLinkReqEvent_t*)pEvent;

                conn_handle = event->connectionHandle;
                VP_LOG_INFO("ble_gap", "connected;handle=%u",
                            conn_handle);
                vp_on_ble_connected(c_vp_rtc_millis());
#if BLE_GAP_POLICY_PARAM_UPDATE_ENABLED
                tmos_start_task(ble_task_id, START_PARAM_UPDATE_EVT,
                                BLE_GAP_POLICY_PARAM_UPDATE_DELAY_MS);
#endif
            }
            break;

        case GAPROLE_CONNECTED_ADV:
            break;

        case GAPROLE_WAITING:
            if (pEvent->gap.opcode == GAP_LINK_TERMINATED_EVENT) {
#if BLE_GAP_POLICY_PARAM_UPDATE_ENABLED
                tmos_stop_task(ble_task_id, START_PARAM_UPDATE_EVT);
#endif
                VP_LOG_INFO("ble_gap", "disconnected;reason=0x%02x",
                            pEvent->linkTerminate.reason);
                conn_handle = GAP_CONNHANDLE_INIT;
                vp_on_ble_disconnected(pEvent->linkTerminate.reason,
                                       c_vp_rtc_millis());
            }
            ble_advert_apply();
            break;

        case GAPROLE_ERROR:
            conn_handle = GAP_CONNHANDLE_INIT;
            VP_LOG_ERROR("ble_gap", "gap state failed");
            break;

        default:
            break;
    }
}

void ble_on_notify_enabled(uint8_t id, uint8_t type,
                                            uint16_t uuid) {
    if (uuid == GATT_CLIENT_CHAR_CFG_UUID && id == HID_RPT_ID_MOUSE_IN &&
        type == HID_REPORT_TYPE_INPUT) {
        vp_on_ble_input_ready(c_vp_rtc_millis());
    }
}
