# CH585 Hardware Notes

本文档整理 `resources/main/` 中 CH585 资料对 VoidPointer 当前硬件/FFI 设计的依据。除非明确列出冲突，本文件不修改现有设计结论；资料不足处标记为 `TBD`。

## Source index

| 来源 | 路径 | 本文使用内容 |
| --- | --- | --- |
| CH585 datasheet | `resources/main/CH585DS1.PDF` | USBFS/USBHS、Flash/DataFlash、低功耗、RTC、GPIO/PMU/USBHS 总体能力；PDF 文本抽取可读性有限 |
| CH585 schematic | `resources/main/CH585/PUB/CH585SCH.pdf` | USB Type-C、BOOT/RST、示例板连接；未作为功能寄存器结论主依据 |
| GPIO driver header | `resources/main/CH585/EXAM/SRC/StdPeriphDriver/inc/CH58x_gpio.h` | GPIO mode、GPIO interrupt trigger enum、读电平/中断标志 API |
| GPIO driver source | `resources/main/CH585/EXAM/SRC/StdPeriphDriver/CH58x_gpio.c` | GPIOA/GPIOB 中断触发寄存器配置实现 |
| SFR header | `resources/main/CH585/EXAM/SRC/StdPeriphDriver/inc/CH585SFR.h` | Sleep/wake/RTC/USBHS/PMU/Flash register definitions |
| PWR driver | `resources/main/CH585/EXAM/SRC/StdPeriphDriver/inc/CH58x_pwr.h`, `resources/main/CH585/EXAM/SRC/StdPeriphDriver/CH58x_pwr.c` | Idle/Halt/Sleep/Shutdown API、wake source API、sleep save/restore notes |
| Flash/ISP API | `resources/main/CH585/EXAM/SRC/StdPeriphDriver/inc/ISP585.h`, `resources/main/CH585/EXAM/SRC/StdPeriphDriver/inc/CH58x_flash.h`, `resources/main/CH585/EXAM/SRC/StdPeriphDriver/CH58x_flash.c`, `resources/main/CH585/EXAM/FLASH/src/Main.c` | DataFlash/Flash size、page/block size、read/erase/write examples |
| RTC/BLE HAL | `resources/main/CH585/EXAM/BLE/HAL/RTC.c`, `resources/main/CH585/EXAM/BLE/HAL/include/RTC.h` | RTC tick source, trigger wake, TMOS clock integration |
| USBHS Compatibility HID | `resources/main/CH585/EXAM/USB/USBHS/DEVICE/CompatibilityHID/User/usb_desc.h`, `usb_desc.c`, `ch585_usbhs_device.h`, `ch585_usbhs_device.c`, `usbd_compatibility_hid.h`, `usbd_compatibility_hid.c`, `main.c` | USBHS device init, descriptor, endpoint, HID report, IN/OUT service, suspend/reset/configured handling |

## USBHS capability

### Conclusions

| 项目 | 依据 | 项目结论 |
| --- | --- | --- |
| CH585 有 USBHS，速率 480 Mbps，支持 Host/Device | `CH585DS1.PDF` 中 USBHS 描述为 `480Mbps USB 2.0 PHY ... Host Device`；`EXAM/USB/USBHS/DEVICE/...` 下存在多个 device 示例 | USB 配置通道使用 USBHS 可行 |
| USBHS device mode 初始化 | `ch585_usbhs_device.c` 的 `USBHS_Device_Init(ENABLE)` 配置 `R8_USBHS_PLL_CTRL`、`RB_PIN_USB2_EN`、`R8_USB2_INT_EN`、`R8_USB2_BASE_MODE = USBHS_UD_SPEED_HIGH`、`R8_USB2_CTRL = USBHS_UD_DEV_EN ...` | 平台层应使用 USBHS device controller；示例直接证明 device mode 可用 |
| USBHS HS interrupt/bulk max packet size | `usb_desc.h` 定义 `DEF_USBD_HS_PACK_SIZE 512`，`DEF_USBD_HS_ISO_PACK_SIZE 1024`；`usb_desc.c` HS endpoint `wMaxPacketSize = 0x0200` | USBHS Vendor HID physical payload 目标为 512 bytes |
| USB FS fallback packet size | `usb_desc.h` 定义 `DEF_USBD_FS_PACK_SIZE 64`；`usb_desc.c` FS endpoint `wMaxPacketSize = 0x0040` | USB FS fallback 为 64 bytes |
| Compatibility HID endpoints | `usb_desc.c` HS/FS config descriptor：EP1 OUT `0x01` interrupt，EP2 IN `0x82` interrupt；HS `512`，FS `64`；`ch585_usbhs_device.c` 仅启用 EP1 RX、EP2 TX | Vendor HID 可按 EP1 OUT + EP2 IN 建模 |
| Compatibility HID report descriptor | `usb_desc.c`：HS report 使用 vendor page `0xFF00`，Input/Output report size 32 bits、count 128，即 512 bytes；FS report size 8 bits、count 64，即 64 bytes | Vendor HID report 与 512/64 physical payload 对齐 |
| OUT 收包方式 | `ch585_usbhs_device.c` 的 EP1 OUT 中断读取 `R16_U2EP1_RX_LEN` 到 `USB_RecLen`，NAK EP1；`usbd_compatibility_hid.c` 的 `UART2_Tx_Service()` 从 `USBHS_EP1_Rx_Buf` 读取长度头和 payload，处理后重新 ACK EP1 | 平台层 OUT 路径应在 EP1 OUT IRQ 中锁住 buffer/长度，bottom-half 消费后重新 ACK |
| IN 发包方式 | `usbd_compatibility_hid.c` 的 `UART2_Rx_Service()` 填 `USBHS_EP2_Tx_Buf`，设置 `R16_U2EP2_T_LEN` 和 TX ACK；EP2 IN 完成中断清 busy/NAK | 平台层 IN 路径应维护 endpoint busy，IN 完成后才允许下一包 |
| configured 事件位置 | `ch585_usbhs_device.c` 处理 `USB_SET_CONFIGURATION` 时设置 `USBHS_DevConfig` 和 `USBHS_DevEnumStatus = 0x01`；`main.c` 仅在 `USBHS_DevEnumStatus` 为真时跑 service | 可将 `USB_SET_CONFIGURATION` 映射为 configured/active 事件 |
| attach/link ready 位置 | `USBHS_Device_Init()` 启用 `USBHS_UDIE_LINK_RDY`；`USB2_DEVICE_IRQHandler()` 处理 `USBHS_UDIF_LINK_RDY` 并清 flag；SFR 中 `USBHS_UDMS_READY` 表示 connected state | 可将 LINK_RDY/MIS_ST ready 作为 attach/link-ready 低层事件；是否等同 user-visible attach 需平台验证 |
| suspend/resume 位置 | `USB2_DEVICE_IRQHandler()` 处理 `USBHS_UDIF_SUSPEND`；若 `R8_USB2_MIS_ST & RB_UMS_SUSPEND` 则置 `USBHS_DevSleepStatus |= 0x02`，否则清除该位 | suspend/resume 可在该分支映射；示例没有单独回调函数，只在 IRQ 中处理状态位 |
| reset/detach-like 位置 | `USB2_DEVICE_IRQHandler()` 处理 `USBHS_UDIF_BUS_RST`，清 `USBHS_DevConfig`、地址、sleep、enum，并重新初始化 endpoints | bus reset 明确清 configured；物理 detach 检测策略 `TBD` |

### USBHS event/API table

| 事件/API/寄存器 | 示例位置 | 含义 | VoidPointer 映射 |
| --- | --- | --- | --- |
| `USBHS_Device_Init(ENABLE)` | `ch585_usbhs_device.c`, `main.c` | 启动 USBHS device controller，并使能 `USB2_DEVICE_IRQn` | USBHS platform init |
| `USBHS_UDIE_LINK_RDY` / `USBHS_UDIF_LINK_RDY` | `CH585SFR.h`, `ch585_usbhs_device.c` | USB connect/link ready interrupt | attach/link-ready candidate |
| `USB_SET_CONFIGURATION` | `ch585_usbhs_device.c` | host set config，置 `USBHS_DevEnumStatus` | route/configured = USB active |
| `USBHS_UDIF_SUSPEND` | `CH585SFR.h`, `ch585_usbhs_device.c` | USB suspend interrupt/status | USB bus suspend/resume input；项目 policy 不等价于进入系统 `Suspend` |
| `USBHS_UDIF_BUS_RST` | `CH585SFR.h`, `ch585_usbhs_device.c` | USB bus reset | clear configured; reinit endpoints |
| `R8_USB2_WAKE_CTRL`, `USBHS_UD_UD_REMOTE_WKUP` | `CH585SFR.h` | USBHS remote wake register | 若需要 remote wake，后续补充实现细节 |

### Project USB state mapping decision

基于上述来源，VoidPointer v1 采用保守 USB state mapping：

| CH585 low-level event/condition | `vp_usb_state_t` | 依据/备注 |
| --- | --- | --- |
| no link-ready observed at startup and no board-level attached signal | `Detached` | 保守初始状态；不是来自明确 detach callback |
| `USBHS_UDIF_LINK_RDY` or `USBHS_UDMS_READY` / ready status | `Attached` | `ch585_usbhs_device.c`, `CH585SFR.h`; link-ready candidate |
| `USB_SET_CONFIGURATION` sets `USBHS_DevConfig` / `USBHS_DevEnumStatus` | `Configured` | `ch585_usbhs_device.c` setup request handling |
| `USBHS_UDIF_SUSPEND` with `RB_UMS_SUSPEND` set | `Suspended` | `ch585_usbhs_device.c` suspend branch |
| suspend interrupt with `RB_UMS_SUSPEND` clear | previous configured state if still valid, otherwise `Attached` | resume-like input; example clears sleep flag |
| `USBHS_UDIF_BUS_RST` | clear configured, reinit endpoints, state becomes `Attached` | bus reset is not treated as physical detach |
| endpoint/controller fatal error | `Error` | platform error mapping |
| explicit VBUS lost, board-level detach, or verified reliable link-lost signal | `Detached` | current WCH example does not provide this; future board/platform hook |

原则：`BUS_RST` 和 USB bus suspend 不直接等同物理拔出。`Detached` 只在启动无 link-ready 或有明确 VBUS/link-lost/板级信号时上报。

### Project alignment

- USB 配置通道继续使用 USBHS。
- Vendor HID HS physical payload 以 512 bytes 为目标；FS fallback 为 64 bytes。
- USB configured 时，项目电源策略保持 `Active`，不进入项目级 `Suspend` / `Sleep`。
- USB bus suspend 是 USB 总线事件；是否驱动项目级低功耗由上层 policy 决定，不自动等价。

## GPIO / EXTI capability

### Trigger modes verified from StdPeriph API

| Trigger | API enum | Driver implementation | 结论 |
| --- | --- | --- | --- |
| low level | `GPIO_ITMode_LowLevel` | `GPIOA_ITModeCfg` / `GPIOB_ITModeCfg`: clear `R16_Px_INT_MODE`, clear output bit | 原生支持低电平触发 |
| high level | `GPIO_ITMode_HighLevel` | clear `R16_Px_INT_MODE`, set output bit | 原生支持高电平触发 |
| falling edge | `GPIO_ITMode_FallEdge` | set `R16_Px_INT_MODE`, clear output bit | 原生支持下降沿触发 |
| rising edge | `GPIO_ITMode_RiseEdge` | set `R16_Px_INT_MODE`, set output bit | 原生支持上升沿触发 |
| both edge | 无对应 `GPIOITModeTpDef` enum | `CH58x_gpio.h` enum 只有四项；`CH58x_gpio.c` switch 无 both-edge case | 标准 GPIO IT API 未原生暴露 both-edge |

### Related GPIO API

| API/寄存器 | 来源 | 用途 |
| --- | --- | --- |
| `GPIOA_ModeCfg`, `GPIOB_ModeCfg` | `CH58x_gpio.h`, `CH58x_gpio.c` | 输入/输出/上下拉配置 |
| `GPIOA_ReadPortPin`, `GPIOB_ReadPortPin` | `CH58x_gpio.h` | ISR 中读取当前电平 |
| `GPIOA_ITModeCfg`, `GPIOB_ITModeCfg` | `CH58x_gpio.h`, `CH58x_gpio.c` | 配置单一触发模式并使能中断 |
| `GPIOA_ReadITFlagBit`, `GPIOB_ReadITFlagBit` | `CH58x_gpio.h` | 读取中断标志 |
| `GPIOA_ClearITFlagBit`, `GPIOB_ClearITFlagBit` | `CH58x_gpio.h` | 清中断标志 |
| `RB_GPIO_EDGE_WAKE` | `CH585SFR.h`, `CH58x_pwr.c` | 低功耗 GPIO wake 可配置“无论上升还是下降都唤醒”；这是 wake 控制，不等同于普通 GPIO IRQ both-edge API |

### VoidPointer 低有效二态输入实现

实机调试发现：CH585 GPIOA 下降沿中断可以锁存第一次机械按键转换，但后续即使 `R16_PA_INT_IF` 与 `R16_PA_INT_EN` 已表示存在待处理中断，PFIC 也可能不再派发 GPIOA IRQ。因此 v1 对低有效按键和自锁开关采用以下平台策略：

| Rust 语义请求 | CH585 GPIO 模式 | 含义 |
| --- | --- | --- |
| `VP_EXTI_EDGE_FALLING` | `GPIO_ITMode_LowLevel` | 当前稳定态为未激活/高电平；输入变为激活/低电平时唤醒 |
| `VP_EXTI_EDGE_RISING` | `GPIO_ITMode_HighLevel` | 当前稳定态为激活/低电平；输入变为未激活/高电平时唤醒 |

GPIOA service 在调用 Rust 前会先屏蔽触发的二态输入。Rust debounce 确认稳定态、发布状态变化后，再请求相反语义转换。由于 PFIC 可能不会再次派发已经待处理的 GPIOA IRQ，runtime 主循环会检查并服务已经锁存的 `R16_PA_INT_IF & R16_PA_INT_EN`。这不是 GPIO 扫描：事件来源仍然是硬件中断标志。

### Both-edge simulation options

标准 API 没有原生 both-edge。平台层若需要对 `vp_exti_edge_t::Both` 支持，可模拟：

| 方案 | 做法 | 风险/备注 |
| --- | --- | --- |
| ISR 中切换 rising/falling | 当前配置 rising，触发后读电平并改为 falling；或反向 | 简单，但在 ISR 重配窗口内可能漏掉快速反向边沿 |
| 根据当前电平重配下一边沿 | 每次 ISR 读取 `GPIOx_ReadPortPin`：当前高则配置 falling，当前低则配置 rising | 比固定切换更稳，但仍存在读电平到重配之间的竞态 |
| 编码器 A/B 双相表兜底 | 任意边沿后读取 A/B 当前状态，使用旧状态+新状态查表 | 如果某一相在重配窗口内漏边沿，可能出现 `00->11` 等跳变，方向判断可被抵消或判无效，但高速滚动可能丢步 |
| 定时采样兜底 | 对编码器或关键按键加入短周期 sampling / debounce timer | 能降低漏边沿影响；代价是功耗/调度开销增加 |

### FFI design relation

| FFI item | CH585 mapping | 结论 |
| --- | --- | --- |
| `vp_exti_edge_t::Rising` | `GPIO_ITMode_RiseEdge` | 原生映射 |
| `vp_exti_edge_t::Falling` | `GPIO_ITMode_FallEdge` | 原生映射 |
| `vp_exti_edge_t::Both` | 平台层模拟，或低功耗 wake 使用 `RB_GPIO_EDGE_WAKE` 但普通 EXTI 仍需模拟 | FFI 只暴露 `Rising` / `Falling` / `Both` 合理；`Both` 由平台层映射或模拟 |

## Low power / wake

### Modes and API

| 项目 | 来源 | 结论 |
| --- | --- | --- |
| CH585 支持 Idle / Halt / Sleep / Shutdown | `CH58x_pwr.h` 声明 `LowPower_Idle`, `LowPower_Halt`, `LowPower_Sleep`, `LowPower_Shutdown`；`EXAM/PM/src/Main.c` 演示四种模式 | 可映射到项目 `Active` 下的 idle wait、项目 `Suspend`/`Sleep` 的平台实现候选 |
| Idle | `CH58x_pwr.c` `LowPower_Idle()` 关闭 Flash 后 `SLEEPDEEP=0` + `__WFI()` | 低延迟等待，不等同项目 `Sleep` |
| Halt | `CH58x_pwr.c` `LowPower_Halt()` 保存 clock/flash 配置，切 4 MHz 内部时钟，`SLEEPDEEP=1`，唤醒后恢复 clock/flash | 可作为项目 `Suspend` 的硬件候选，但需验证 BLE/USB 保持策略 |
| Sleep | `CH58x_pwr.c` `LowPower_Sleep(rm)` 使用 `R16_POWER_PLAN` 选择保留 RAM/EXTEND/XROM，注释提示唤醒后 Flash 稳定需延时 | 项目 `Sleep` 的硬件候选；需要严格保存/恢复外设 |
| Shutdown | `CH58x_pwr.c` `LowPower_Shutdown(rm)` 唤醒后软件复位；`EXAM/PM/src/Main.c` 注释也说明唤醒后会执行复位 | 一般不直接对应项目无缝 `Sleep`，除非设计接受 wake reset |

### Wake sources

| Wake source | API/寄存器 | 来源 | 结论 |
| --- | --- | --- | --- |
| GPIO wake | `PWR_PeriphWakeUpCfg(ENABLE, RB_SLP_GPIO_WAKE, ...)`; `EXAM/PM/src/Main.c` 用 PA5 falling edge 唤醒；`RB_GPIO_EDGE_WAKE` 表示 GPIO 任意边沿唤醒 | `CH58x_pwr.c`, `CH585SFR.h`, `EXAM/PM/src/Main.c` | 支持 GPIO wake；普通 GPIO IRQ both-edge 与 wake both-edge 分开看 |
| USBFS wake | `RB_SLP_USB_WAKE` | `CH585SFR.h`, `CH58x_pwr.c` | 支持 USBFS wake |
| USBHS wake | `RB_SLP_USB2_WAKE`, `R8_USB2_WAKE_CTRL` | `CH585SFR.h`, `CH58x_pwr.c` | 支持 USBHS wake/remote wake 相关寄存器；完整流程 `TBD` |
| RTC wake | `RB_SLP_RTC_WAKE`, `R32_RTC_TRIG`, `RB_RTC_TRIG_EN`; PM 示例配置 RTC trigger 后 WFE | `CH585SFR.h`, `CH58x_pwr.c`, `EXAM/PM/src/Main.c`, `RTC.c` | 支持 RTC wake |
| BLE/RF wake | datasheet/SFR 显示 BLE/RF/EXTEND 电源域；`LowPower_Sleep` 的 `RB_PWR_EXTEND` 注释为 USB/BLE retention | `CH585DS1.PDF`, `CH585SFR.h`, `CH58x_pwr.h/c` | BLE/RF low power/wake 由 BLE stack/HAL 管理，具体回调/约束 `TBD` |
| Battery wake/monitor | `RB_SLP_BAT_WAKE`, `PowerMonitor()` | `CH585SFR.h`, `CH58x_pwr.h/c` | 可用于低电压事件；项目暂未作为主 wake source |

### Save/restore notes

| 场景 | 来源 | 需要处理 |
| --- | --- | --- |
| 进入 Halt | `CH58x_pwr.c` | 保存 `R16_CLK_SYS_CFG`, `R8_FLASH_CFG`, `R8_FLASH_SCK`；关闭 Flash；切 4 MHz；唤醒后恢复 clock/flash；示例外层调用 `HSECFG_Current(HSE_RCur_100)` |
| 进入 Sleep | `CH58x_pwr.c` | 保存 clock/flash/HFCK 配置；配置 `R16_POWER_PLAN`; 关闭电压监控；唤醒后恢复 clock/flash/HFCK；如果保留 `RB_PWR_EXTEND` 且使用 USBHS，注释要求唤醒后复位所有高速 USB 寄存器 |
| PM Sleep with RTC | `EXAM/PM/src/Main.c` | 进入前切内部时钟，设置 `R8_SLP_WAKE_CTRL` 为 RTC wake，设置 `R32_RTC_TRIG`，关 Flash，WFE，醒后清 RTC flag、恢复 wake ctrl、恢复 HSE/system clock |
| Shutdown | `CH58x_pwr.c`, `EXAM/PM/src/Main.c` | 唤醒后复位；若项目需要保持 Rust 状态，不应使用 Shutdown 作为普通 `Sleep` |

### Project power-state alignment

| Project state | CH585 implementation note | 结论 |
| --- | --- | --- |
| `Active` | 全功能运行；可用 `LowPower_Idle()` 做短等待但状态仍为 Active | USB configured、配置会话、HID 活跃时保持 `Active` |
| `Suspend` | 候选为 Halt 或浅 Sleep，同时保持 BLE/2.4G 连接和 IMU wake；具体 BLE/RF retention 需要栈约束 | `TBD`: 需要确认 BLE connected 下允许的最低功耗调用 |
| `Sleep` | 候选为 Sleep，关闭 RF/USB route，仅保留 GPIO/RTC/IMU wake 所需电源域 | USB detached 且无线断开后才允许进入 |
| USB configured | USBHS active/configured | 保持 `Active`，不进入项目 `Suspend` / `Sleep` |

## RTC / timebase

| 项目 | 来源 | 结论 |
| --- | --- | --- |
| RTC clock source | `RTC.h` 中 `FREQ_RTC` 为 32000 或 32768；`RTC.c` 根据 `CLK_OSC32K` 配置内部 32K 或外部 32K | RTC/32K 可作为低功耗 timebase |
| RTC counter | `CH585SFR.h` 定义 `R32_RTC_CNT_32K`、`R16_RTC_CNT_32K`、`R16_RTC_CNT_2S`、`R32_RTC_CNT_DAY`；`RTC.c` 的 `SYS_GetClockValue()` 双读 `R32_RTC_CNT_32K` 保证稳定 | 可读取 32K tick 并换算 millis |
| RTC millis helper | `RTC.h` 定义 `RTC_TO_MS`, `MS_TO_RTC`, `CLK_PER_MS` 等宏 | 适合作为 `vp_timestamp_t = uint32_t RTC millis` 来源；32-bit millis 约 49.7 天回绕，需上层按 wrapping time 处理 |
| Sleep 中运行 | `CH585SFR.h` PMU/RTC wake registers；`EXAM/PM/src/Main.c` Sleep 中配置 RTC trigger WFE 唤醒 | RTC 可作为 sleep wake source，说明 sleep 中可继续提供触发；具体在所有 power plan 下的保持条件 `TBD` |
| RTC wake 配置 | `RTC.c` `RTC_SetTignTime()` 写 `R32_RTC_TRIG`；PM 示例设置 `RB_SLP_RTC_WAKE`、`RB_RTC_TRIG_EN`、`R32_RTC_TRIG` 并清 `RB_RTC_TRIG_CLR` | 平台层可封装 `set_rtc_wake_at/after` |

## DataFlash / Flash

### Memory capability

| 项目 | 来源 | 结论 |
| --- | --- | --- |
| Flash/DataFlash 容量 | `CH585DS1.PDF`：512KB FlashROM，其中 448KB CodeFlash、32KB DataFlash、24KB BootLoader、8KB InfoFlash；地址图显示 DataFlash `0x00070000-0x00077FFF` | DataFlash 可作为配置存储区；容量 32KB |
| DataFlash API | `ISP585.h` 定义 `EEPROM_READ`, `EEPROM_ERASE`, `EEPROM_WRITE`；`EXAM/FLASH/src/Main.c` 演示读 500 bytes、擦除、写 500 bytes | 官方 API/示例存在 |
| DataFlash page/write | `ISP585.h` 注释：DataFlash 支持 byte and page writing，最小 write/read 1 byte，256 bytes/page，`EEPROM_WRITE` 支持 1 byte or more，multiple of 256 best | 写入可按 byte length，但项目宜按 256-byte page 对齐优化 |
| DataFlash erase | `ISP585.h` 注释：erase block 为 256/4096 bytes；`EEPROM_MIN_ER_SIZE = 256`, `EEPROM_BLOCK_SIZE = 4096`；但 `EEPROM_ERASE` inline 对 chip id `0x08` 要求 length 必须为 4096 的倍数 | CH585 实际 DataFlash erase granularity 需要按 chip id 验证；保守按 4KB erase block 设计 |
| FlashROM write/erase | `ISP585.h` 注释：FlashROM 最小写/校验 4 bytes，256 bytes/page，4KB erase block；`EXAM/FLASH/src/Main.c` 注释 CodeFlash 写入必须是 4 字节整数倍 | CodeFlash 不应作为普通配置双槽首选 |
| Buffer alignment | `ISP585.h` API 注释：Buffer must in RAM and be aligned to 4 bytes | C FFI DataFlash API 应保证 RAM buffer 4-byte aligned |

### Relation to dual-slot config storage

| 设计点 | CH585 依据 | 建议/结论 |
| --- | --- | --- |
| 双槽配置 | 32KB DataFlash 足够容纳两个 slot + metadata | 继续使用双槽；slot 大小需由平台常量暴露给 Rust |
| erase unit | `EEPROM_BLOCK_SIZE=4096`，`EEPROM_MIN_ER_SIZE=256` 但 CH585 inline 可能强制 4096 | 建议初版按 4KB erase block 对齐 slot；`TBD` 实机确认是否可 256-byte erase |
| write unit | 最小 1 byte，256-byte page best | FFI 可暴露 min write 1、preferred write 256；Rust 仍可按 page/chunk 写 |
| atomicity | 官方 API 仅提供 read/erase/write，未找到断电原子保证 | 双槽 + sequence + CRC 仍必要 |

## Open questions / TBD

| 问题 | 当前状态 | 需要回填到哪里 |
| --- | --- | --- |
| USBHS 物理 detach 判断 | 项目 v1 映射已定：不把 `BUS_RST`/suspend 当 detach；仅启动无 link-ready 或有明确 VBUS/link-lost/板级信号时上报 `Detached`。仍需后续实机验证是否有可靠 link-lost source | USB platform implementation notes |
| USBHS remote wake 完整流程 | 找到 `R8_USB2_WAKE_CTRL` / `USBHS_UD_UD_REMOTE_WKUP` 和 remote-wakeup feature 处理，但没有完整项目化流程 | Power/USB platform notes |
| BLE/RF connected 状态下最低可用 low-power API | 找到 `RB_PWR_EXTEND` 为 USB/BLE retention、BLE HAL RTC timebase，但未系统整理 BLE stack sleep contract | `POWER_STATE_MACHINE.md` 或 CH585 platform plan |
| 普通 GPIO IRQ both-edge 是否存在更底层寄存器原生模式 | StdPeriph API 未暴露；SFR 中只确认 wake 任意边沿 `RB_GPIO_EDGE_WAKE` | 若后续查到寄存器级 both-edge，可更新 FFI mapping |
| DataFlash erase granularity 对 CH585 实物的最终限制 | `ISP585.h` 同时给出 256/4096 erase，但 inline 对某 chip id 强制 4096 | `CONFIG_SPEC.md` 的 slot/page 设计 |
| RTC 在不同 `R16_POWER_PLAN` 组合下的保持条件 | RTC wake 示例存在；具体 power domain 保持矩阵需进一步从 datasheet/实测确认 | power platform notes |
| `CH585SCH.pdf` 中项目板 USB/按键/电源连接与 VoidPointer 最终 PCB 的对应关系 | 仅粗看了示例板原理图，未逐页整理 | board bring-up notes |

## Possible conflicts / backfill points

| 现有设计点 | 资料结论 | 是否冲突 | 回填建议 |
| --- | --- | --- | --- |
| 编码器 A/B 配置为双边沿外部中断 | StdPeriph GPIO IT API 没有 both-edge enum，只能 rising/falling/level；wake 有 any-edge 但普通 EXTI 未确认 | 潜在实现风险，不是设计冲突 | 在 FFI/platform 文档中说明 `Both` 由平台模拟，编码器可能需要定时采样兜底 |
| USBHS 512 bytes / FS 64 bytes | Compatibility HID 示例完全对齐 | 无冲突 | 可在 `VENDOR_PROTOCOL.md` 回填物理层依据 |
| USB configured 保持 Active | USBHS 示例 configured 后进入持续 service loop；低功耗 Sleep 会要求 USBHS 寄存器复位 | 无冲突，反而支持 | 在 `POWER_STATE_MACHINE.md` 强调 configured 不进低功耗 |
| DataFlash 双槽 | CH585 DataFlash 32KB，官方 EEPROM API 可读写擦 | 无冲突 | 在 `CONFIG_SPEC.md` 回填 CH585 page/block/alignment 约束，尤其 4KB erase 保守策略 |
| `vp_timestamp_t = uint32_t RTC millis` | RTC 32K counter + conversion helper 存在，sleep wake 示例存在 | 无冲突 | 在 FFI/timebase 文档补充 wrapping millis 语义 |
