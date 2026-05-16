#ifndef VOIDPOINTER_MAIN_H
#define VOIDPOINTER_MAIN_H

#include <stdint.h>

void core_request_poll();
void core_request_poll_after(uint32_t ms);
void debounce_start();
void debounce_stop();

#endif
