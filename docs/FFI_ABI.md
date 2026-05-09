# VoidPointer FFI 与 ABI 约束

本文档只描述长期有效的 FFI 边界规则
当前实现进度、迁移任务和临时差距不放在这里，统一看 `dev/TASKLIST.md`

## 1. 总体原则

- C 是平台执行层
- Rust 是业务决策层
- `platform/Bind/c_api.h` 是 Rust 调 C 的唯一声明源
- Rust 导出的函数由 `cbindgen` 生成 `platform/Bind/rust_api.h`
- 不手写和工具生成结果重复的声明
- 跨 FFI 的命名保持稳定前缀
  - Rust 调 C 使用 `c_vp_*`
  - C 调 Rust 使用 `vp_*`

## 2. ABI 规则

- timestamp 统一使用 RTC 毫秒
- `vp_timestamp_t` 固定为 `uint32_t`，并按 wrapping time 处理回绕
- bool 统一使用 `uint8_t`，取值 `0` 或 `1`
- HID 与 vendor report 长度统一使用 `uint16_t`
- flash 与 config buffer 长度统一使用 `uint32_t`
- buffer 统一使用指针加长度
- Rust 不保存 ISR callback 传入的裸指针
- 跨 FFI struct 使用固定布局
- 跨 FFI enum 使用固定底层整数表示
- 持久化多字节字段统一使用 little-endian
- v1 不做 ABI version 校验

## 3. 执行上下文规则

### 3.1 ISR-safe

ISR-safe API 必须满足两点：

- 可以在中断上下文调用
- 不做阻塞、分配或重逻辑

### 3.2 bottom-half only

非 ISR-safe API 只能在 `vp_core_poll()` 或普通 task 语境调用
不能在 GPIO、Timer、I2C 完成中断等硬中断路径直接执行

### 3.3 `vp_core_poll()` 约束

- `vp_core_poll()` 是 Rust bottom-half 入口
- 它由 C 侧事件调度，不是忙等轮询函数
- 它严格不可重入
- C 侧请求 poll 时只能置位或合并事件，不能直接在请求点调用 `vp_core_poll()`

## 4. C 与 Rust 的职责边界

### 4.1 C 负责什么

C 提供以下平台能力：

- GPIO 与 EXTI
- Timer、RTC、TMOS
- I2C 与 IMU 访问
- HID、USB、BLE 和后续 2.4G 平台动作
- Power 与 DataFlash 平台动作

C 只负责硬件事实和执行动作，不持有产品业务状态

### 4.2 Rust 负责什么

Rust 负责以下业务逻辑：

- 输入状态机
- 编码器与按键消抖
- 姿态处理与映射
- 报告聚合
- 路由策略
- 电源状态机
- 配置与 Vendor 命令解析

Rust 根据状态做决策，再通过 FFI 调 C 执行动作

额外约束：

- `vp_core_poll()` 严格不可重入
- callback 之间不共享业务状态
- 如确实需要跨 callback 共享底层结构，必须由底层自行保证 ISR-safe

## 5. 通用类型

| 类型 | C 表示 | 说明 |
| --- | --- | --- |
| `vp_timestamp_t` | `uint32_t` | RTC 毫秒，按 wrapping time 处理回绕 |
| `vp_bool_t` | `uint8_t` | FFI bool |
| `vp_status_t` | `uint8_t` enum | 通用状态码 |
| `vp_hid_send_status_t` | `uint8_t` enum | HID 发送结果，独立于 `vp_status_t` |

### `vp_status_t`

| 值 | 名称 | 说明 |
| --- | --- | --- |
| `0` | `VP_STATUS_OK` | 成功 |
| `1` | `VP_STATUS_BUSY` | 资源忙 |
| `2` | `VP_STATUS_INVALID_ARG` | 参数非法 |
| `3` | `VP_STATUS_NOT_READY` | 外设或协议栈未就绪 |
| `4` | `VP_STATUS_IO_ERROR` | 底层 I/O 错误 |
| `5` | `VP_STATUS_UNSUPPORTED` | 当前平台或模式不支持 |

## 6. C 调 Rust 的入口

这些函数由 Rust 实现，C 在中断、协议栈回调或 task 中调用

### 6.1 生命周期

| API | 上下文 | 说明 |
| --- | --- | --- |
| `vp_core_init()` | task | 初始化 Runtime 与初始状态 |
| `vp_core_poll()` | TMOS task | 执行 Rust bottom-half |

### 6.2 输入事件

| API | 典型上下文 | ISR-safe | 说明 |
| --- | --- | --- | --- |
| `vp_on_button_exti(button_id, level, timestamp)` | GPIO EXTI | 是 | 普通按键输入 |
| `vp_on_mode_switch_exti(level, timestamp)` | GPIO EXTI | 是 | 物理模式开关输入 |
| `vp_on_debounce_tick(timestamp)` | Timer ISR | 是 | 共享 debounce tick |
| `vp_on_encoder_exti(a_level, b_level, timestamp)` | GPIO EXTI | 是 | 编码器输入 |

### 6.3 IMU 事件

| API | 典型上下文 | 说明 |
| --- | --- | --- |
| `vp_on_imu_int(timestamp)` | GPIO EXTI | IMU 唤醒事实 |
| `vp_on_imu_sample(raw_x, raw_y, raw_z, timestamp)` | I2C completion 或 task | 上报最新姿态原始样本 |
| `vp_on_imu_fifo_done(status, dropped_count, timestamp)` | I2C completion 或 task | 上报一次 FIFO 读取结束 |

### 6.4 路由与连接事件

| API | 典型上下文 | 说明 |
| --- | --- | --- |
| `vp_on_ble_connected(timestamp)` | protocol callback | BLE 已连接 |
| `vp_on_ble_input_ready(timestamp)` | protocol callback | BLE 输入路径 ready |
| `vp_on_ble_disconnected(reason, timestamp)` | protocol callback | BLE 已断开 |
| `vp_on_dongle_connected(timestamp)` | protocol callback | 2.4G 已连接 |
| `vp_on_dongle_disconnected(reason, timestamp)` | protocol callback | 2.4G 已断开 |
| `vp_on_usb_state_changed(state, timestamp)` | USB callback | USB 状态变化 |
| `vp_on_hid_send_done(route, status, timestamp)` | protocol callback | 异步发送完成事件 |
| `vp_on_vendor_report_rx(route, ptr, len, timestamp)` | protocol callback | 收到 vendor report |

## 7. Rust 调 C 的接口类别

这些函数由 C 实现并声明在 `platform/Bind/c_api.h`

### 7.1 GPIO 与 EXTI

- `c_vp_gpio_read()`
- `c_vp_gpio_read_inputs()`
- `c_vp_gpio_write()`
- `c_vp_exti_mask()`
- `c_vp_exti_unmask()`
- `c_vp_exti_clear_pending()`
- `c_vp_exti_set_edge()`

关键约束：

- 普通低有效二态输入的 `Falling` 与 `Rising` 表示下一次语义转换
- 平台可以把它们映射为高低电平触发，而不是机械地映射成边沿
- `Both` 主要服务编码器，平台可以用重配下一边沿模拟

### 7.2 Timer、RTC 与调度

- `c_vp_debounce_timer_start()`
- `c_vp_debounce_timer_stop()`
- `c_vp_rtc_tick()`
- `c_vp_rtc_millis()`
- `c_vp_rtc_micros()`
- `c_vp_rtc_set_wake_after()`
- `c_vp_request_core_poll()`
- `c_vp_request_core_poll_after()`

关键约束：

- 请求 poll 只负责调度，不负责现场执行
- RTC 是跨模块统一时间源

### 7.3 I2C 与 IMU

- `c_vp_i2c_init()`
- `c_vp_i2c_recover_bus()`
- `c_vp_i2c_abort()`
- `c_vp_imu_config_active()`
- `c_vp_imu_config_suspend()`
- `c_vp_imu_config_sleep()`
- `c_vp_imu_read_fifo_async()`
- `c_vp_imu_read_whoami()`

关键约束：

- IMU 中断只上报事实，不在 ISR 内直接读 FIFO
- FIFO 读取是否发生，由 Rust 在 bottom-half 决定

### 7.4 HID 与 route

- `c_vp_hid_route_ready()`
- `c_vp_hid_send_mouse()`
- `c_vp_hid_send_vendor()`
- `c_vp_hid_route_enable()`
- `c_vp_hid_route_reset()`

#### `vp_hid_route_t`

| 值 | 名称 |
| --- | --- |
| `0` | `None` |
| `1` | `BLE` |
| `2` | `Dongle2G4` |
| `3` | `USB` |

#### `vp_hid_send_status_t`

| 值 | 名称 | 说明 |
| --- | --- | --- |
| `0` | `Sent` | 已发送或已进入底层队列 |
| `1` | `RetryLater` | 暂时不可发送 |
| `2` | `NotConnected` | route 不可用 |
| `3` | `Fatal` | 不可恢复错误 |

关键约束：

- `connected` 不等于 `route ready`
- 尤其是 BLE，链路存在不代表输入路径已经 secure 或 notify-ready
- route 不可用时，runtime 必须收敛本次发送尝试，不能靠脏状态自旋重试
- 只有底层明确返回 `RetryLater` 且存在合理恢复窗口时，才允许延时重试
- `Sent`：提交本次发送对应的 pending 状态，并清除 retry
- `RetryLater`：保留待发状态，设置短退避重试
- `NotConnected`：收敛本次尝试，等待 route 相关事件再次唤醒
- `Fatal`：按本次发送失败收敛，不做定时重试；后续是否 route reset 由更高层策略决定

### 7.5 Power 与 DataFlash

- `c_vp_power_prepare_suspend()`
- `c_vp_power_enter_suspend()`
- `c_vp_power_prepare_sleep()`
- `c_vp_power_enter_sleep()`
- `c_vp_power_restore_from_sleep()`
- `c_vp_wake_source_enable()`
- `c_vp_flash_config_region()`
- `c_vp_flash_read()`
- `c_vp_flash_erase()`
- `c_vp_flash_write()`

关键约束：

- 是否进入 `Active`、`Suspend`、`Sleep` 由 Rust 判断
- C 只执行平台动作，不替代状态决策

## 8. 常用枚举的语义约束

### `vp_button_id_t`

`vp_button_id_t` 固定表示这些实体按钮：

- `Left`
- `Right`
- `Middle`
- `Action`
- `Laser`

### `vp_input_id_t`

`vp_input_id_t` 只表示真实硬件输入，例如：

- 按键
- 模式开关
- 编码器 A/B
- IMU INT

USB 状态通过 USB callback 上报，不作为 GPIO input id

### `vp_output_id_t`

当前只定义 `Laser`
如需增加可控 LED 或其他 power rail，再扩展该枚举

### `vp_exti_edge_t`

`vp_exti_edge_t` 只暴露：

- `Rising`
- `Falling`
- `Both`

对普通低有效二态输入，`Rising` 与 `Falling` 表示下一次语义转换
平台可以把它们映射为高低电平触发，而不是机械地映射成边沿

### `vp_wake_source_t`

`vp_wake_source_t` 只用于平台唤醒源控制和 debug / diagnostics 表达，不承担业务状态建模职责

## 9. 文档边界

本文档只回答三类问题：

- FFI 的职责怎么分
- 哪些接口可以在什么上下文调用
- 哪些 ABI 约束不能随意改变

更细的状态机和平台参数分别放在 `POWER_STATE_MACHINE.md`、`ROUTE_STATE_MACHINE.md`、`RESOURCE_PROFILE.md` 和 `CH585_NOTES.md`
