#ifndef VOIDPOINTER_USBHS_HID_DEVICE_H
#define VOIDPOINTER_USBHS_HID_DEVICE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void USBHS_HidDevice_Init(void);
void USBHS_HidDevice_ResetLinkState(void);
uint8_t USBHS_HidDevice_SendMouseReport(const uint8_t* report, uint16_t len);
uint8_t USBHS_HidDevice_SendVendorReport(const uint8_t* report, uint16_t len);

#ifdef __cplusplus
}
#endif

#endif
