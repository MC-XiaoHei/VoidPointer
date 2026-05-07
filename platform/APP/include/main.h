#ifndef VOIDPOINTER_MAIN_H
#define VOIDPOINTER_MAIN_H

#include <stdint.h>

void RuntimeTask_RequestPoll(void);
void RuntimeTask_RequestPollAfter(uint32_t ms);
void RuntimeTask_StartDebounceTimer(void);
void RuntimeTask_StopDebounceTimer(void);
void Platform_NotifyUsbStateChanged(uint8_t state);

#endif
