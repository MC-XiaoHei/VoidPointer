/********************************** (C) COPYRIGHT *******************************
 * File Name          : hidmouse.c
 * Author             : WCH
 * Version            : V1.0
 * Date               : 2018/12/10
 * Description        : 蓝牙鼠标应用程序，初始化广播连接参数，然后广播，直至连接主机后，定时上传键值
 *********************************************************************************
 * Copyright (c) 2021 Nanjing Qinheng Microelectronics Co., Ltd.
 * Attention: This software (modified or not) and binary are used for
 * microcontroller manufactured by Nanjing Qinheng Microelectronics.
 *******************************************************************************/

/*********************************************************************
 * INCLUDES
 */

#include "CONFIG.h"  // IWYU pragma: keep
#include "battservice.h"
#include "hiddev.h"
#include "hidmouse.h"
#include "hidmouseservice.h"
#include "rust_api.h"

/*********************************************************************
 * MACROS
 */

// Selected HID mouse button values
#define MOUSE_BUTTON_NONE                 0x00

// HID mouse input report length
#define HID_MOUSE_IN_RPT_LEN              4

/*********************************************************************
 * CONSTANTS
 */
// Param update delay
#define START_PARAM_UPDATE_EVT_DELAY      12800
#define VP_BLE_BRINGUP_DISABLE_PARAM_UPDATE 0
#define VP_BLE_BRINGUP_REQUEST_SECURITY_ON_CONNECT FALSE

// Param update delay
#define START_PHY_UPDATE_DELAY            1600

// HID idle timeout in msec; set to zero to disable timeout
#define DEFAULT_HID_IDLE_TIMEOUT          60000

// Minimum connection interval (units of 1.25ms)
#define DEFAULT_DESIRED_MIN_CONN_INTERVAL 8

// Maximum connection interval (units of 1.25ms)
#define DEFAULT_DESIRED_MAX_CONN_INTERVAL 8

// Slave latency to use if parameter update request
#define DEFAULT_DESIRED_SLAVE_LATENCY     0

// Supervision timeout value (units of 10ms)
#define DEFAULT_DESIRED_CONN_TIMEOUT      500

// Default passcode
#define DEFAULT_PASSCODE                  0

// Default GAP pairing mode
#define DEFAULT_PAIRING_MODE              GAPBOND_PAIRING_MODE_WAIT_FOR_REQ

// Default MITM mode (TRUE to require passcode or OOB when pairing)
#define DEFAULT_MITM_MODE                 FALSE

// Default bonding mode, TRUE to bond
#define DEFAULT_BONDING_MODE              TRUE

// Default GAP bonding I/O capabilities
#define DEFAULT_IO_CAPABILITIES           GAPBOND_IO_CAP_NO_INPUT_NO_OUTPUT

// Battery level is critical when it is less than this %
#define DEFAULT_BATT_CRITICAL_LEVEL       6

/*********************************************************************
 * TYPEDEFS
 */

/*********************************************************************
 * GLOBAL VARIABLES
 */

// Task ID
static uint8_t hidEmuTaskId = INVALID_TASK_ID;

/*********************************************************************
 * EXTERNAL VARIABLES
 */

/*********************************************************************
 * EXTERNAL FUNCTIONS
 */

/*********************************************************************
 * LOCAL VARIABLES
 */

// GAP Profile - Name attribute for SCAN RSP data
static uint8_t scanRspData[] = {
    0x0D,  // length of this data
    GAP_ADTYPE_LOCAL_NAME_COMPLETE,  // AD Type = Complete local name
    'V', 'o', 'i', 'd', ' ', 'P', 'o', 'i', 'n', 't', 'e', 'r',
    // connection interval range
    0x05,  // length of this data
    GAP_ADTYPE_SLAVE_CONN_INTERVAL_RANGE,
    LO_UINT16(DEFAULT_DESIRED_MIN_CONN_INTERVAL),  // 100ms
    HI_UINT16(DEFAULT_DESIRED_MIN_CONN_INTERVAL),
    LO_UINT16(DEFAULT_DESIRED_MAX_CONN_INTERVAL),  // 1s
    HI_UINT16(DEFAULT_DESIRED_MAX_CONN_INTERVAL),

    // service UUIDs
    0x05,  // length of this data
    GAP_ADTYPE_16BIT_MORE, LO_UINT16(HID_SERV_UUID), HI_UINT16(HID_SERV_UUID),
    LO_UINT16(BATT_SERV_UUID), HI_UINT16(BATT_SERV_UUID),

    // Tx power level
    0x02,  // length of this data
    GAP_ADTYPE_POWER_LEVEL,
    0  // 0dBm
};

// Advertising data
static uint8_t advertData[] = {
    // flags
    0x02,  // length of this data
    GAP_ADTYPE_FLAGS,
    GAP_ADTYPE_FLAGS_LIMITED | GAP_ADTYPE_FLAGS_BREDR_NOT_SUPPORTED,

    // appearance
    0x03,  // length of this data
    GAP_ADTYPE_APPEARANCE, LO_UINT16(GAP_APPEARE_HID_MOUSE),
    HI_UINT16(GAP_APPEARE_HID_MOUSE)};

// Device name attribute value
static CONST uint8_t attDeviceName[GAP_DEVICE_NAME_LEN] = "Void Pointer";

// HID Dev configuration
static hidDevCfg_t hidEmuCfg = {
    DEFAULT_HID_IDLE_TIMEOUT,  // Idle timeout
    HID_FEATURE_FLAGS  // HID feature flags
};

static uint16_t         hidEmuConnHandle = GAP_CONNHANDLE_INIT;
static uint8_t          hidEmuAdvertisingAllowed = TRUE;
static gapRole_States_t hidEmuGapState = GAPROLE_INIT;
static uint8_t          hidEmuGapStarted = FALSE;

/*********************************************************************
 * LOCAL FUNCTIONS
 */

static void    hidEmu_ProcessTMOSMsg(tmos_event_hdr_t* pMsg);
static void    hidEmuSendMouseReport(uint8_t buttons, uint8_t X_data,
                                     uint8_t Y_data);
static uint8_t hidEmuRptCB(uint8_t id, uint8_t type, uint16_t uuid,
                           uint8_t oper, uint16_t* pLen, uint8_t* pData);
static void    hidEmuEvtCB(uint8_t evt);
static void    hidEmuStateCB(gapRole_States_t newState, gapRoleEvent_t* pEvent);
static void    hidEmuApplyAdvertisingPolicy(void);

/*********************************************************************
 * PROFILE CALLBACKS
 */

static hidDevCB_t hidEmuHidCBs = {hidEmuRptCB, hidEmuEvtCB, NULL,
                                  hidEmuStateCB};

/*********************************************************************
 * PUBLIC FUNCTIONS
 */

/*********************************************************************
 * @fn      HidEmu_Init
 *
 * @brief   Initialization function for the HidEmuKbd App Task.
 *          This is called during initialization and should contain
 *          any application specific initialization (ie. hardware
 *          initialization/setup, table initialization, power up
 *          notificaiton ... ).
 *
 * @param   task_id - the ID assigned by TMOS.  This ID should be
 *                    used to send messages and set timers.
 *
 * @return  none
 */
void HidEmu_Init() {
    hidEmuAdvertisingAllowed = TRUE;
    hidEmuConnHandle = GAP_CONNHANDLE_INIT;
    hidEmuGapState = GAPROLE_INIT;
    hidEmuGapStarted = FALSE;
    hidEmuTaskId = TMOS_ProcessEventRegister(HidEmu_ProcessEvent);

    // Setup the GAP Peripheral Role Profile
    {
        uint8_t initial_advertising_enable = TRUE;

        // Set the GAP Role Parameters
        GAPRole_SetParameter(GAPROLE_ADVERT_ENABLED, sizeof(uint8_t),
                             &initial_advertising_enable);

        GAPRole_SetParameter(GAPROLE_ADVERT_DATA, sizeof(advertData),
                             advertData);
        GAPRole_SetParameter(GAPROLE_SCAN_RSP_DATA, sizeof(scanRspData),
                             scanRspData);
    }

    // Set the GAP Characteristics
    GGS_SetParameter(GGS_DEVICE_NAME_ATT, GAP_DEVICE_NAME_LEN,
                     (void*)attDeviceName);

    // Setup the GAP Bond Manager
    {
        uint32_t passkey = DEFAULT_PASSCODE;
        uint8_t  pairMode = DEFAULT_PAIRING_MODE;
        uint8_t  mitm = DEFAULT_MITM_MODE;
        uint8_t  ioCap = DEFAULT_IO_CAPABILITIES;
        uint8_t  bonding = DEFAULT_BONDING_MODE;
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
        PRINT("BLE bond cfg: pairMode=%u mitm=%u ioCap=%u bonding=%u passkey=%lu\n",
              pairMode, mitm, ioCap, bonding, (unsigned long)passkey);
    }

    // Setup Battery Characteristic Values
    {
        uint8_t critical = DEFAULT_BATT_CRITICAL_LEVEL;
        Batt_SetParameter(BATT_PARAM_CRITICAL_LEVEL, sizeof(uint8_t), &critical);
    }

    // Set up HID keyboard service
    Hid_AddService();

    // Register for HID Dev callback
    HidDev_Register(&hidEmuCfg, &hidEmuHidCBs);

    // Setup a delayed profile startup
    tmos_set_event(hidEmuTaskId, START_DEVICE_EVT);
}

uint8_t HidEmu_SetAdvertisingEnabled(uint8_t enabled) {
    hidEmuAdvertisingAllowed = enabled ? TRUE : FALSE;
    PRINT("BLE advertising allowed=%u state=%u started=%u handle=%u\n",
          hidEmuAdvertisingAllowed, hidEmuGapState, hidEmuGapStarted,
          hidEmuConnHandle);
    hidEmuApplyAdvertisingPolicy();
    return SUCCESS;
}

uint8_t HidEmu_Disconnect(void) {
    if (hidEmuConnHandle == GAP_CONNHANDLE_INIT) {
        return SUCCESS;
    }

    return GAPRole_TerminateLink(hidEmuConnHandle);
}

/*********************************************************************
 * @fn      HidEmu_ProcessEvent
 *
 * @brief   HidEmuKbd Application Task event processor.  This function
 *          is called to process all events for the task.  Events
 *          include timers, messages and any other user defined events.
 *
 * @param   task_id  - The TMOS assigned task ID.
 * @param   events - events to process.  This is a bit map and can
 *                   contain more than one event.
 *
 * @return  events not processed
 */
uint16_t HidEmu_ProcessEvent(uint8_t task_id, uint16_t events) {
    if (events & SYS_EVENT_MSG) {
        uint8_t* pMsg;

        if ((pMsg = tmos_msg_receive(hidEmuTaskId)) != NULL) {
            hidEmu_ProcessTMOSMsg((tmos_event_hdr_t*)pMsg);

            // Release the TMOS message
            tmos_msg_deallocate(pMsg);
        }

        // return unprocessed events
        return (events ^ SYS_EVENT_MSG);
    }

    if (events & START_DEVICE_EVT) {
        return (events ^ START_DEVICE_EVT);
    }

    if (events & START_PARAM_UPDATE_EVT) {
#if !VP_BLE_BRINGUP_DISABLE_PARAM_UPDATE
        PRINT("ConnParamUpdate start\n");
        GAPRole_PeripheralConnParamUpdateReq(
            hidEmuConnHandle, DEFAULT_DESIRED_MIN_CONN_INTERVAL,
            DEFAULT_DESIRED_MAX_CONN_INTERVAL, DEFAULT_DESIRED_SLAVE_LATENCY,
            DEFAULT_DESIRED_CONN_TIMEOUT, hidEmuTaskId);
#endif

        return (events ^ START_PARAM_UPDATE_EVT);
    }

    if (events & START_PHY_UPDATE_EVT) {
        PRINT("Send Phy Update %x...\n",
              GAPRole_UpdatePHY(hidEmuConnHandle, 0, GAP_PHY_BIT_LE_2M,
                                GAP_PHY_BIT_LE_2M, 0));

        return (events ^ START_PHY_UPDATE_EVT);
    }

    return 0;
}

/*********************************************************************
 * @fn      hidEmu_ProcessTMOSMsg
 *
 * @brief   Process an incoming task message.
 *
 * @param   pMsg - message to process
 *
 * @return  none
 */
static void hidEmu_ProcessTMOSMsg(tmos_event_hdr_t* pMsg) {
    switch (pMsg->event) {
        default:
            break;
    }
}

static void hidEmuApplyAdvertisingPolicy(void) {
    if (!hidEmuGapStarted) {
        return;
    }

    if ((hidEmuGapState & GAPROLE_STATE_ADV_MASK) == GAPROLE_CONNECTED ||
        (hidEmuGapState & GAPROLE_STATE_ADV_MASK) == GAPROLE_CONNECTED_ADV) {
        return;
    }

    GAPRole_SetParameter(GAPROLE_ADVERT_ENABLED, sizeof(uint8_t),
                         &hidEmuAdvertisingAllowed);
}

/*********************************************************************
 * @fn      hidEmuSendMouseReport
 *
 * @brief   Build and send a HID mouse report.
 *
 * @param   buttons - Mouse button code
 *                    X_data - X axis move data
 *                    Y_data - Y axis move data
 *
 * @return  none
 */
static void hidEmuSendMouseReport(uint8_t buttons, uint8_t X_data,
                                  uint8_t Y_data) {
    uint8_t buf[HID_MOUSE_IN_RPT_LEN];

    buf[0] = buttons;  // Buttons
    buf[1] = X_data;  // X
    buf[2] = Y_data;  // Y
    buf[3] = 0;  // Wheel

    HidDev_Report(HID_RPT_ID_MOUSE_IN, HID_REPORT_TYPE_INPUT,
                  HID_MOUSE_IN_RPT_LEN, buf);
}

/*********************************************************************
 * @fn      hidEmuStateCB
 *
 * @brief   GAP state change callback.
 *
 * @param   newState - new state
 *
 * @return  none
 */
static void hidEmuStateCB(gapRole_States_t newState, gapRoleEvent_t* pEvent) {
    hidEmuGapState = newState;

    switch (newState & GAPROLE_STATE_ADV_MASK) {
        case GAPROLE_STARTED: {
            uint8_t ownAddr[6];
            hidEmuGapStarted = TRUE;
            GAPRole_GetParameter(GAPROLE_BD_ADDR, ownAddr);
            GAP_ConfigDeviceAddr(ADDRTYPE_STATIC, ownAddr);
            PRINT("Initialized..\n");
            hidEmuApplyAdvertisingPolicy();
        } break;

        case GAPROLE_ADVERTISING:
            PRINT("Advertising..\n");
            break;

        case GAPROLE_CONNECTED:
            if (pEvent->gap.opcode == GAP_LINK_ESTABLISHED_EVENT) {
                gapEstLinkReqEvent_t* event = (gapEstLinkReqEvent_t*)pEvent;

                hidEmuConnHandle = event->connectionHandle;
                PRINT("Connected..\n");
                vp_on_ble_connected(c_vp_rtc_millis());
#if VP_BLE_BRINGUP_REQUEST_SECURITY_ON_CONNECT
                PRINT("SecurityReq start: handle=%u\n", hidEmuConnHandle);
                bStatus_t securityReqStatus =
                    GAPBondMgr_PeriSecurityReq(hidEmuConnHandle);
                PRINT("SecurityReq done: handle=%u status=%u\n",
                      hidEmuConnHandle, securityReqStatus);
#endif
#if !VP_BLE_BRINGUP_DISABLE_PARAM_UPDATE
                tmos_start_task(hidEmuTaskId, START_PARAM_UPDATE_EVT,
                                START_PARAM_UPDATE_EVT_DELAY);
#endif
            }
            break;

        case GAPROLE_CONNECTED_ADV:
            break;

        case GAPROLE_WAITING:
            if (pEvent->gap.opcode == GAP_LINK_TERMINATED_EVENT) {
                PRINT("Disconnected.. reason=%x\n",
                      pEvent->linkTerminate.reason);
                hidEmuConnHandle = GAP_CONNHANDLE_INIT;
                vp_on_ble_disconnected(pEvent->linkTerminate.reason,
                                       c_vp_rtc_millis());
            }
            hidEmuApplyAdvertisingPolicy();
            break;

        case GAPROLE_ERROR:
            break;

        default:
            break;
    }
}

/*********************************************************************
 * @fn      hidEmuRptCB
 *
 * @brief   HID Dev report callback.
 *
 * @param   id - HID report ID.
 * @param   type - HID report type.
 * @param   uuid - attribute uuid.
 * @param   oper - operation:  read, write, etc.
 * @param   len - Length of report.
 * @param   pData - Report data.
 *
 * @return  GATT status code.
 */
static uint8_t hidEmuRptCB(uint8_t id, uint8_t type, uint16_t uuid,
                           uint8_t oper, uint16_t* pLen, uint8_t* pData) {
    uint8_t status = SUCCESS;

    // write
    if (oper == HID_DEV_OPER_WRITE) {
        if (uuid == REPORT_UUID) {
            if (type == HID_REPORT_TYPE_OUTPUT) {
                // keyboard output report (LEDs)
            } else if (type == HID_REPORT_TYPE_FEATURE) {
                // feature report
            }
        }
    } else if (oper == HID_DEV_OPER_ENABLE && uuid == GATT_CLIENT_CHAR_CFG_UUID &&
               id == HID_RPT_ID_MOUSE_IN && type == HID_REPORT_TYPE_INPUT &&
               HidDev_IsReportNotifyEnabled(id, type)) {
        vp_on_ble_input_ready(c_vp_rtc_millis());
    }

    return status;
}

/*********************************************************************
 * @fn      hidEmuEvtCB
 *
 * @brief   HID event callback.
 *
 * @param   evt - event code
 *
 * @return  none
 */
static void hidEmuEvtCB(uint8_t evt) {
    switch (evt) {
        default:
            break;
    }
}
