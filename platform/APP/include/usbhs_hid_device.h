#ifndef VOIDPOINTER_USBHS_HID_DEVICE_H
#define VOIDPOINTER_USBHS_HID_DEVICE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void    usb_hid_init();
void    usb_hid_reset_link();
uint8_t usb_hid_send_mouse(const uint8_t* report, uint16_t len);
uint8_t usb_hid_send_vendor(const uint8_t* report, uint16_t len);

#ifdef __cplusplus
}
#endif

#endif
