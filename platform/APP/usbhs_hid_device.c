#include "usbhs_hid_device.h"

#include "CH58x_common.h"
#include "c_api.h"
#include "main.h"
#include <stdint.h>
#include <string.h>

#define DEF_UEP_IN 0x80u
#define DEF_UEP0 0x00u
#define DEF_UEP1 0x01u
#define DEF_UEP2 0x02u
#define DEF_UEP3 0x03u

#define VP_USBHS_IRQ_ATTR __INTERRUPT __HIGH_CODE

#define USBHS_HID_EP0_SIZE 64u
#define USBHS_HID_MOUSE_EP_SIZE 8u
#define USBHS_HID_VENDOR_EP_SIZE 64u

#define USBHS_HID_MOUSE_INTERFACE 0u
#define USBHS_HID_VENDOR_INTERFACE 1u
#define USBHS_HID_INTERFACE_COUNT 2u

#define USBHS_HID_STRING_LANG 0u
#define USBHS_HID_STRING_MANUFACTURER 1u
#define USBHS_HID_STRING_PRODUCT 2u
#define USBHS_HID_STRING_SERIAL 3u

#define USBHS_HID_REQ_GET_REPORT 0x01u
#define USBHS_HID_REQ_GET_IDLE 0x02u
#define USBHS_HID_REQ_GET_PROTOCOL 0x03u
#define USBHS_HID_REQ_SET_REPORT 0x09u
#define USBHS_HID_REQ_SET_IDLE 0x0Au
#define USBHS_HID_REQ_SET_PROTOCOL 0x0Bu

#define USBHS_HID_REMOTE_WAKEUP 0x01u
#define USBHS_HID_BUS_SUSPENDED 0x02u

#define USBHS_HID_MOUSE_EP_IN_ADDR (DEF_UEP_IN | DEF_UEP1)
#define USBHS_HID_VENDOR_EP_OUT_ADDR (0x00u | DEF_UEP2)
#define USBHS_HID_VENDOR_EP_IN_ADDR (DEF_UEP_IN | DEF_UEP3)

typedef struct __attribute__((packed)) {
    uint8_t bmRequestType;
    uint8_t bRequest;
    uint16_t wValue;
    uint16_t wIndex;
    uint16_t wLength;
} usbhs_setup_req_t;

static const uint8_t g_usbhs_hid_device_desc[] = {
    0x12, 0x01, 0x10, 0x01, 0x00, 0x00, 0x00, USBHS_HID_EP0_SIZE,
    0x86, 0x1A, 0x11, 0xFE, 0x00, 0x01, USBHS_HID_STRING_MANUFACTURER,
    USBHS_HID_STRING_PRODUCT, USBHS_HID_STRING_SERIAL, 0x01,
};

static const uint8_t g_usbhs_hid_config_desc[] = {
    0x09, 0x02, 0x42u, 0x00, USBHS_HID_INTERFACE_COUNT, 0x01, 0x00, 0xA0, 0x32,

    0x09, 0x04, USBHS_HID_MOUSE_INTERFACE, 0x00, 0x01, 0x03, 0x01, 0x02, 0x00,
    0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x34, 0x00,
    0x07, 0x05, USBHS_HID_MOUSE_EP_IN_ADDR, 0x03, USBHS_HID_MOUSE_EP_SIZE, 0x00, 0x01,

    0x09, 0x04, USBHS_HID_VENDOR_INTERFACE, 0x00, 0x02, 0x03, 0x00, 0x00, 0x00,
    0x09, 0x21, 0x11, 0x01, 0x00, 0x01, 0x22, 0x22, 0x00,
    0x07, 0x05, USBHS_HID_VENDOR_EP_OUT_ADDR, 0x03, USBHS_HID_VENDOR_EP_SIZE, 0x00, 0x01,
    0x07, 0x05, USBHS_HID_VENDOR_EP_IN_ADDR, 0x03, USBHS_HID_VENDOR_EP_SIZE, 0x00, 0x01,
};

static const uint8_t g_usbhs_hid_mouse_report_desc[] = {
    0x05, 0x01, 0x09, 0x02, 0xA1, 0x01, 0x09, 0x01, 0xA1, 0x00,
    0x05, 0x09, 0x19, 0x01, 0x29, 0x03, 0x15, 0x00, 0x25, 0x01,
    0x75, 0x01, 0x95, 0x03, 0x81, 0x02, 0x75, 0x05, 0x95, 0x01,
    0x81, 0x01, 0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x09, 0x38,
    0x15, 0x81, 0x25, 0x7F, 0x75, 0x08, 0x95, 0x03, 0x81, 0x06,
    0xC0, 0xC0,
};

static const uint8_t g_usbhs_hid_vendor_report_desc[] = {
    0x06, 0x00, 0xFF,
    0x09, 0x01,
    0xA1, 0x01,
    0x09, 0x02,
    0x15, 0x00,
    0x26, 0xFF, 0x00,
    0x75, 0x08,
    0x95, 0x40,
    0x81, 0x02,
    0x09, 0x02,
    0x15, 0x00,
    0x26, 0xFF, 0x00,
    0x75, 0x08,
    0x95, 0x40,
    0x91, 0x02,
    0xC0,
};

static const uint8_t g_usbhs_hid_qualifier_desc[] = {
    0x0A, 0x06, 0x10, 0x01, 0x00, 0x00, 0x00, USBHS_HID_EP0_SIZE, 0x01, 0x00,
};

static const uint8_t g_usbhs_hid_lang_desc[] = {0x04, 0x03, 0x09, 0x04};
static const uint8_t g_usbhs_hid_manufacturer_desc[] = {
    0x12, 0x03, 'V', 0, 'o', 0, 'i', 0, 'd', 0, 'P', 0, 't', 0, 'r', 0,
};
static const uint8_t g_usbhs_hid_product_desc[] = {
    0x22, 0x03, 'V', 0, 'o', 0, 'i', 0, 'd', 0, ' ', 0, 'P', 0, 'o', 0,
    'i', 0, 'n', 0, 't', 0, 'e', 0, 'r', 0, ' ', 0, 'C', 0, 'o', 0,
    'm', 0,
};
static const uint8_t g_usbhs_hid_serial_desc[] = {
    0x12, 0x03, '0', 0, '0', 0, '0', 0, '0', 0, '0', 0, '0', 0, '1', 0,
};

static const uint8_t* g_ep0_desc_ptr = NULL;
static uint16_t g_ep0_desc_remaining = 0u;
static uint8_t g_dev_config = 0u;
static uint8_t g_dev_addr = 0u;
static uint8_t g_dev_sleep_status = 0u;
static uint8_t g_hid_idle[USBHS_HID_INTERFACE_COUNT] = {0u, 0u};
static uint8_t g_hid_protocol[USBHS_HID_INTERFACE_COUNT] = {1u, 1u};

__attribute__((aligned(4))) static uint8_t g_ep0_buf[USBHS_HID_EP0_SIZE];
__attribute__((aligned(4))) static uint8_t g_ep1_tx_buf[USBHS_HID_MOUSE_EP_SIZE];
__attribute__((aligned(4))) static uint8_t g_ep2_rx_buf[USBHS_HID_VENDOR_EP_SIZE];
__attribute__((aligned(4))) static uint8_t g_ep3_tx_buf[USBHS_HID_VENDOR_EP_SIZE];
static volatile uint8_t g_ep_busy[16];

static usbhs_setup_req_t* usbhs_setup_packet(void) {
    return (usbhs_setup_req_t*)g_ep0_buf;
}

static void usbhs_ep_init(void) {
    for (uint8_t i = 0; i < 16u; ++i) {
        g_ep_busy[i] = 0u;
    }

    R16_U2EP_TX_EN = RB_EP0_EN | RB_EP1_EN | RB_EP3_EN;
    R16_U2EP_RX_EN = RB_EP0_EN | RB_EP2_EN;

    R32_U2EP0_MAX_LEN = USBHS_HID_EP0_SIZE;
    R32_U2EP1_MAX_LEN = USBHS_HID_MOUSE_EP_SIZE;
    R32_U2EP2_MAX_LEN = USBHS_HID_VENDOR_EP_SIZE;
    R32_U2EP3_MAX_LEN = USBHS_HID_VENDOR_EP_SIZE;

    R32_U2EP0_DMA = (uintptr_t)g_ep0_buf;
    R32_U2EP1_TX_DMA = (uintptr_t)g_ep1_tx_buf;
    R32_U2EP2_RX_DMA = (uintptr_t)g_ep2_rx_buf;
    R32_U2EP3_TX_DMA = (uintptr_t)g_ep3_tx_buf;

    R8_U2EP0_TX_CTRL = USBHS_UEP_T_RES_NAK;
    R8_U2EP0_RX_CTRL = USBHS_UEP_R_RES_ACK;
    R8_U2EP1_TX_CTRL = USBHS_UEP_T_RES_NAK;
    R8_U2EP2_RX_CTRL = USBHS_UEP_R_RES_ACK;
    R8_U2EP3_TX_CTRL = USBHS_UEP_T_RES_NAK;
}

static uint8_t usbhs_interface_valid(uint16_t index) {
    return index < USBHS_HID_INTERFACE_COUNT ? 1u : 0u;
}

static void usbhs_notify_resumed_state(void) {
    if (g_dev_config != 0u) {
        Platform_NotifyUsbStateChanged(VP_USB_STATE_CONFIGURED);
    } else {
        Platform_NotifyUsbStateChanged(VP_USB_STATE_ATTACHED);
    }
}

static void usbhs_handle_get_descriptor(uint16_t value, uint16_t index, uint16_t req_len, uint8_t* out_err) {
    const uint8_t* desc = NULL;
    uint16_t desc_len = 0u;

    switch ((uint8_t)(value >> 8)) {
        case USB_DESCR_TYP_DEVICE:
            desc = g_usbhs_hid_device_desc;
            desc_len = sizeof(g_usbhs_hid_device_desc);
            break;
        case USB_DESCR_TYP_CONFIG:
            desc = g_usbhs_hid_config_desc;
            desc_len = sizeof(g_usbhs_hid_config_desc);
            break;
        case USB_DESCR_TYP_STRING:
            switch ((uint8_t)(value & 0xFFu)) {
                case USBHS_HID_STRING_LANG:
                    desc = g_usbhs_hid_lang_desc;
                    desc_len = sizeof(g_usbhs_hid_lang_desc);
                    break;
                case USBHS_HID_STRING_MANUFACTURER:
                    desc = g_usbhs_hid_manufacturer_desc;
                    desc_len = sizeof(g_usbhs_hid_manufacturer_desc);
                    break;
                case USBHS_HID_STRING_PRODUCT:
                    desc = g_usbhs_hid_product_desc;
                    desc_len = sizeof(g_usbhs_hid_product_desc);
                    break;
                case USBHS_HID_STRING_SERIAL:
                    desc = g_usbhs_hid_serial_desc;
                    desc_len = sizeof(g_usbhs_hid_serial_desc);
                    break;
                default:
                    *out_err = 0xFFu;
                    return;
            }
            break;
        case USB_DESCR_TYP_QUALIF:
            desc = g_usbhs_hid_qualifier_desc;
            desc_len = sizeof(g_usbhs_hid_qualifier_desc);
            break;
        case USB_DESCR_TYP_HID:
            if (index == USBHS_HID_MOUSE_INTERFACE) {
                desc = &g_usbhs_hid_config_desc[18];
                desc_len = 9u;
            } else if (index == USBHS_HID_VENDOR_INTERFACE) {
                desc = &g_usbhs_hid_config_desc[43];
                desc_len = 9u;
            } else {
                *out_err = 0xFFu;
                return;
            }
            break;
        case USB_DESCR_TYP_REPORT:
            if (index == USBHS_HID_MOUSE_INTERFACE) {
                desc = g_usbhs_hid_mouse_report_desc;
                desc_len = sizeof(g_usbhs_hid_mouse_report_desc);
            } else if (index == USBHS_HID_VENDOR_INTERFACE) {
                desc = g_usbhs_hid_vendor_report_desc;
                desc_len = sizeof(g_usbhs_hid_vendor_report_desc);
            } else {
                *out_err = 0xFFu;
                return;
            }
            break;
        default:
            *out_err = 0xFFu;
            return;
    }

    g_ep0_desc_ptr = desc;
    g_ep0_desc_remaining = req_len < desc_len ? req_len : desc_len;
    const uint16_t first_len =
        g_ep0_desc_remaining > USBHS_HID_EP0_SIZE ? USBHS_HID_EP0_SIZE : g_ep0_desc_remaining;
    if (first_len != 0u) {
        memcpy(g_ep0_buf, g_ep0_desc_ptr, first_len);
        g_ep0_desc_ptr += first_len;
        g_ep0_desc_remaining -= first_len;
    }
    R16_U2EP0_T_LEN = first_len;
}

static void usbhs_handle_setup(void) {
    usbhs_setup_req_t* req = usbhs_setup_packet();
    uint8_t err = 0u;
    uint16_t reply_len = 0u;
    const uint16_t interface_index = req->wIndex;

    g_ep0_desc_ptr = NULL;
    g_ep0_desc_remaining = 0u;

    if ((req->bmRequestType & USB_REQ_TYP_MASK) == USB_REQ_TYP_STANDARD) {
        switch (req->bRequest) {
            case USB_GET_DESCRIPTOR:
                usbhs_handle_get_descriptor(req->wValue, req->wIndex, req->wLength, &err);
                break;
            case USB_SET_ADDRESS:
                g_dev_addr = (uint8_t)(req->wValue & 0x7Fu);
                reply_len = 0u;
                R16_U2EP0_T_LEN = reply_len;
                break;
            case USB_GET_CONFIGURATION:
                g_ep0_buf[0] = g_dev_config;
                reply_len = req->wLength > 1u ? 1u : req->wLength;
                R16_U2EP0_T_LEN = reply_len;
                break;
            case USB_SET_CONFIGURATION:
                g_dev_config = (uint8_t)(req->wValue & 0xFFu);
                if (g_dev_config != 0u) {
                    Platform_NotifyUsbStateChanged(VP_USB_STATE_CONFIGURED);
                } else {
                    Platform_NotifyUsbStateChanged(VP_USB_STATE_ATTACHED);
                }
                reply_len = 0u;
                R16_U2EP0_T_LEN = reply_len;
                break;
            case USB_GET_INTERFACE:
                g_ep0_buf[0] = 0u;
                reply_len = req->wLength > 1u ? 1u : req->wLength;
                R16_U2EP0_T_LEN = reply_len;
                break;
            case USB_SET_INTERFACE:
                reply_len = 0u;
                R16_U2EP0_T_LEN = reply_len;
                break;
            case USB_GET_STATUS:
                g_ep0_buf[0] = (g_dev_sleep_status & USBHS_HID_REMOTE_WAKEUP) ? 0x02u : 0x00u;
                g_ep0_buf[1] = 0u;
                reply_len = req->wLength > 2u ? 2u : req->wLength;
                R16_U2EP0_T_LEN = reply_len;
                break;
            case USB_CLEAR_FEATURE:
                if ((req->bmRequestType & USB_REQ_RECIP_MASK) == USB_REQ_RECIP_DEVICE &&
                    (uint8_t)(req->wValue & 0xFFu) == USB_REQ_FEAT_REMOTE_WAKEUP) {
                    g_dev_sleep_status &= (uint8_t)~USBHS_HID_REMOTE_WAKEUP;
                    reply_len = 0u;
                    R16_U2EP0_T_LEN = reply_len;
                } else {
                    err = 0xFFu;
                }
                break;
            case USB_SET_FEATURE:
                if ((req->bmRequestType & USB_REQ_RECIP_MASK) == USB_REQ_RECIP_DEVICE &&
                    (uint8_t)(req->wValue & 0xFFu) == USB_REQ_FEAT_REMOTE_WAKEUP) {
                    g_dev_sleep_status |= USBHS_HID_REMOTE_WAKEUP;
                    reply_len = 0u;
                    R16_U2EP0_T_LEN = reply_len;
                } else {
                    err = 0xFFu;
                }
                break;
            default:
                err = 0xFFu;
                break;
        }
    } else if ((req->bmRequestType & USB_REQ_TYP_MASK) == USB_REQ_TYP_CLASS) {
        if (!usbhs_interface_valid(interface_index)) {
            err = 0xFFu;
        } else {
            switch (req->bRequest) {
                case USBHS_HID_REQ_GET_IDLE:
                    g_ep0_buf[0] = g_hid_idle[interface_index];
                    reply_len = req->wLength > 1u ? 1u : req->wLength;
                    R16_U2EP0_T_LEN = reply_len;
                    break;
                case USBHS_HID_REQ_SET_IDLE:
                    g_hid_idle[interface_index] = (uint8_t)(req->wValue >> 8);
                    reply_len = 0u;
                    R16_U2EP0_T_LEN = reply_len;
                    break;
                case USBHS_HID_REQ_GET_PROTOCOL:
                    g_ep0_buf[0] = g_hid_protocol[interface_index];
                    reply_len = req->wLength > 1u ? 1u : req->wLength;
                    R16_U2EP0_T_LEN = reply_len;
                    break;
                case USBHS_HID_REQ_SET_PROTOCOL:
                    g_hid_protocol[interface_index] = (uint8_t)(req->wValue & 0xFFu);
                    reply_len = 0u;
                    R16_U2EP0_T_LEN = reply_len;
                    break;
                case USBHS_HID_REQ_GET_REPORT:
                    memset(g_ep0_buf, 0, USBHS_HID_EP0_SIZE);
                    reply_len = req->wLength > USBHS_HID_EP0_SIZE ? USBHS_HID_EP0_SIZE : req->wLength;
                    R16_U2EP0_T_LEN = reply_len;
                    break;
                case USBHS_HID_REQ_SET_REPORT:
                    R8_U2EP0_RX_CTRL = USBHS_UEP_R_TOG_DATA1 | USBHS_UEP_R_RES_ACK;
                    return;
                default:
                    err = 0xFFu;
                    break;
            }
        }
    } else {
        err = 0xFFu;
    }

    if (err != 0u) {
        R8_U2EP0_TX_CTRL = USBHS_UEP_T_TOG_DATA1 | USBHS_UEP_T_RES_STALL;
        R8_U2EP0_RX_CTRL = USBHS_UEP_R_TOG_DATA1 | USBHS_UEP_R_RES_STALL;
        return;
    }

    R8_U2EP0_TX_CTRL = USBHS_UEP_T_TOG_DATA1 | USBHS_UEP_T_RES_ACK;
    if ((req->bmRequestType & USB_REQ_TYP_IN) == 0u) {
        R8_U2EP0_RX_CTRL = USBHS_UEP_R_TOG_DATA1 | USBHS_UEP_R_RES_ACK;
    }
}

static void usbhs_reset_link_state(void) {
    g_dev_config = 0u;
    g_dev_addr = 0u;
    g_dev_sleep_status = 0u;
    g_hid_idle[0] = 0u;
    g_hid_idle[1] = 0u;
    g_hid_protocol[0] = 1u;
    g_hid_protocol[1] = 1u;
    g_ep0_desc_ptr = NULL;
    g_ep0_desc_remaining = 0u;
    R8_USB2_DEV_AD = 0u;
    usbhs_ep_init();
}

void USBHS_HidDevice_Init(void) {
    usbhs_reset_link_state();

    R16_CLK_SYS_CFG |= (RB_CLK_SYS_MOD & 0x40) | RB_XROM_SCLK_SEL | RB_OSC32M_SEL;
    R8_USBHS_PLL_CTRL = USBHS_PLL_EN;
    R16_PIN_CONFIG |= RB_PIN_USB2_EN;

    R8_USB2_CTRL = USBHS_UD_RST_LINK | USBHS_UD_PHY_SUSPENDM;
    R8_USB2_INT_EN = USBHS_UDIE_BUS_RST | USBHS_UDIE_SUSPEND |
                     USBHS_UDIE_TRANSFER | USBHS_UDIE_LINK_RDY;
    R8_USB2_BASE_MODE = USBHS_UD_SPEED_HIGH;
    R8_USB2_CTRL = USBHS_UD_DEV_EN | USBHS_UD_DMA_EN | USBHS_UD_PHY_SUSPENDM;
    PFIC_EnableIRQ(USB2_DEVICE_IRQn);
}

void USBHS_HidDevice_ResetLinkState(void) {
    R8_USB2_CTRL = USBHS_UD_RST_LINK | USBHS_UD_PHY_SUSPENDM;
    usbhs_reset_link_state();
    R8_USB2_CTRL = USBHS_UD_DEV_EN | USBHS_UD_DMA_EN | USBHS_UD_PHY_SUSPENDM;
}

uint8_t USBHS_HidDevice_SendMouseReport(const uint8_t* report, uint16_t len) {
    if (report == NULL || len > USBHS_HID_MOUSE_EP_SIZE) {
        return 0u;
    }

    if (g_dev_config == 0u || g_ep_busy[DEF_UEP1] != 0u) {
        return 0u;
    }

    memcpy(g_ep1_tx_buf, report, len);
    g_ep_busy[DEF_UEP1] = 1u;
    R16_U2EP1_T_LEN = len;
    R8_U2EP1_TX_CTRL = (R8_U2EP1_TX_CTRL & (uint8_t)~USBHS_UEP_T_RES_MASK) | USBHS_UEP_T_RES_ACK;
    return 1u;
}

uint8_t USBHS_HidDevice_SendVendorReport(const uint8_t* report, uint16_t len) {
    if (report == NULL || len > USBHS_HID_VENDOR_EP_SIZE) {
        return 0u;
    }

    if (g_dev_config == 0u || g_ep_busy[DEF_UEP3] != 0u) {
        return 0u;
    }

    memcpy(g_ep3_tx_buf, report, len);
    g_ep_busy[DEF_UEP3] = 1u;
    R16_U2EP3_T_LEN = len;
    R8_U2EP3_TX_CTRL = (R8_U2EP3_TX_CTRL & (uint8_t)~USBHS_UEP_T_RES_MASK) | USBHS_UEP_T_RES_ACK;
    return 1u;
}

VP_USBHS_IRQ_ATTR
void USB2_DEVICE_IRQHandler(void) {
    const uint8_t intflag = R8_USB2_INT_FG;
    const uint8_t intst = R8_USB2_INT_ST;

    if (intflag & USBHS_UDIF_TRANSFER) {
        const uint8_t endp_num = intst & USBHS_UDIS_EP_ID_MASK;
        if ((intst & USBHS_UDIS_EP_DIR) == 0u) {
            if (endp_num == DEF_UEP0 && (R8_U2EP0_RX_CTRL & USBHS_UEP_R_SETUP_IS) != 0u) {
                usbhs_handle_setup();
            } else if (endp_num == DEF_UEP0) {
                R8_U2EP0_RX_CTRL = USBHS_UEP_R_RES_NAK;
                R16_U2EP0_T_LEN = 0u;
                R8_U2EP0_TX_CTRL = USBHS_UEP_T_TOG_DATA1 | USBHS_UEP_T_RES_ACK;
                R8_U2EP0_RX_CTRL &= (uint8_t)~USBHS_UEP_R_DONE;
            } else if (endp_num == DEF_UEP2) {
                if ((R8_U2EP2_RX_CTRL & USBHS_UEP_R_TOG_MATCH) != 0u) {
                    const uint16_t rx_len = R16_U2EP2_RX_LEN > USBHS_HID_VENDOR_EP_SIZE
                                                ? USBHS_HID_VENDOR_EP_SIZE
                                                : R16_U2EP2_RX_LEN;
                    vp_on_vendor_report_rx(VP_HID_ROUTE_USB, g_ep2_rx_buf, rx_len, c_vp_rtc_millis());
                    R8_U2EP2_RX_CTRL ^= USBHS_UEP_R_TOG_DATA1;
                    R8_U2EP2_RX_CTRL = (R8_U2EP2_RX_CTRL & (uint8_t)~USBHS_UEP_R_RES_MASK) | USBHS_UEP_R_RES_ACK;
                }
                R8_U2EP2_RX_CTRL &= (uint8_t)~USBHS_UEP_R_DONE;
            }
        } else {
            switch (endp_num) {
                case DEF_UEP0:
                    if (g_ep0_desc_remaining != 0u && g_ep0_desc_ptr != NULL) {
                        const uint16_t tx_len = g_ep0_desc_remaining > USBHS_HID_EP0_SIZE ? USBHS_HID_EP0_SIZE : g_ep0_desc_remaining;
                        memcpy(g_ep0_buf, g_ep0_desc_ptr, tx_len);
                        g_ep0_desc_ptr += tx_len;
                        g_ep0_desc_remaining -= tx_len;
                        R16_U2EP0_T_LEN = tx_len;
                        R8_U2EP0_TX_CTRL ^= USBHS_UEP_T_TOG_DATA1;
                        R8_U2EP0_TX_CTRL = (R8_U2EP0_TX_CTRL & (uint8_t)~USBHS_UEP_T_RES_MASK) | USBHS_UEP_T_RES_ACK;
                    } else {
                        if (g_dev_addr != 0u) {
                            R8_USB2_DEV_AD = g_dev_addr;
                            g_dev_addr = 0u;
                        }
                        R16_U2EP0_T_LEN = 0u;
                        R8_U2EP0_RX_CTRL = USBHS_UEP_R_TOG_DATA1 | USBHS_UEP_R_RES_ACK;
                    }
                    R8_U2EP0_TX_CTRL &= (uint8_t)~USBHS_UEP_T_DONE;
                    break;
                case DEF_UEP1:
                    R8_U2EP1_TX_CTRL = (R8_U2EP1_TX_CTRL & (uint8_t)~USBHS_UEP_T_RES_MASK) | USBHS_UEP_T_RES_NAK;
                    R8_U2EP1_TX_CTRL ^= USBHS_UEP_T_TOG_DATA1;
                    g_ep_busy[DEF_UEP1] = 0u;
                    R8_U2EP1_TX_CTRL &= (uint8_t)~USBHS_UEP_T_DONE;
                    break;
                case DEF_UEP3:
                    R8_U2EP3_TX_CTRL = (R8_U2EP3_TX_CTRL & (uint8_t)~USBHS_UEP_T_RES_MASK) | USBHS_UEP_T_RES_NAK;
                    R8_U2EP3_TX_CTRL ^= USBHS_UEP_T_TOG_DATA1;
                    g_ep_busy[DEF_UEP3] = 0u;
                    R8_U2EP3_TX_CTRL &= (uint8_t)~USBHS_UEP_T_DONE;
                    break;
                default:
                    break;
            }
        }
        R8_USB2_INT_FG = USBHS_UDIF_TRANSFER;
        return;
    }

    if (intflag & USBHS_UDIF_LINK_RDY) {
        Platform_NotifyUsbStateChanged(VP_USB_STATE_ATTACHED);
        R8_USB2_INT_FG = USBHS_UDIF_LINK_RDY;
        return;
    }

    if (intflag & USBHS_UDIF_SUSPEND) {
        if ((R8_USB2_MIS_ST & USBHS_UDMS_SUSPEND) != 0u) {
            g_dev_sleep_status |= USBHS_HID_BUS_SUSPENDED;
            Platform_NotifyUsbStateChanged(VP_USB_STATE_SUSPENDED);
        } else {
            g_dev_sleep_status &= (uint8_t)~USBHS_HID_BUS_SUSPENDED;
            usbhs_notify_resumed_state();
        }
        R8_USB2_INT_FG = USBHS_UDIF_SUSPEND;
        return;
    }

    if (intflag & USBHS_UDIF_BUS_RST) {
        g_dev_config = 0u;
        g_dev_addr = 0u;
        g_dev_sleep_status = 0u;
        g_hid_idle[0] = 0u;
        g_hid_idle[1] = 0u;
        g_hid_protocol[0] = 1u;
        g_hid_protocol[1] = 1u;
        g_ep0_desc_ptr = NULL;
        g_ep0_desc_remaining = 0u;
        R8_USB2_DEV_AD = 0u;
        usbhs_ep_init();
        Platform_NotifyUsbStateChanged(VP_USB_STATE_ATTACHED);
        R8_USB2_INT_FG = USBHS_UDIF_BUS_RST;
        return;
    }

    R8_USB2_INT_FG = intflag;
}
