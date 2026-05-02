#ifndef VOIDPOINTER_MAIN_H
#define VOIDPOINTER_MAIN_H

#include <stdint.h>

#define ENC_A      GPIO_Pin_4
#define MIDDLE_BTN GPIO_Pin_5
#define ENC_B      GPIO_Pin_6
#define LIGHT_BTN  GPIO_Pin_7
#define RIGHT_BTN  GPIO_Pin_8
#define LEFT_BTN   GPIO_Pin_9
#define ACTION_BTN GPIO_Pin_10

#define I2C_SDA    GPIO_Pin_12
#define I2C_SCL    GPIO_Pin_13

#define DEBUG_TX   GPIO_Pin_14
#define DEBUG_RX   GPIO_Pin_15

void RuntimeTask_RequestPoll(void);
void RuntimeTask_RequestPollAfter(uint32_t ms);
void RuntimeTask_StartDebounceTimer(void);
void RuntimeTask_StopDebounceTimer(void);

#endif
