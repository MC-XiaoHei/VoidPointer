# VoidPointer FFI API 规格

本文档只描述目标状态下的 C/Rust FFI API。当前工程中已有的 demo/legacy API 不在本文档中作为目标 API 描述；需要迁移或替换的内容记录在 `dev/TASKLIST.md`。

---

## 1. API 设计原则

- C 是硬件执行层：提供 GPIO、EXTI、Timer、RTC、TMOS、I2C、IMU、HID、RF/USB、DataFlash、Power 等底层 API。
- Rust 是逻辑层：持有输入、motion、report、route、power、config 等业务状态机。
- Rust 调 C 的 API 以 `c_vp_*` 前缀命名。
- C 调 Rust 的 callback 以 `vp_*` 前缀命名。
- Rust 调 C 的声明源头是 `platform/Bind/c_api.h`，由 `bindgen` 自动生成 Rust bindings。
- C 调 Rust 的声明源头是 Rust exported functions，由 `cbindgen` 自动生成 `platform/Bind/rust_api.h`。
- 不手写 `rust_api.h`。
- 不在 Rust 中手写和 `c_api.h` 重复的 extern 声明。
- 所有跨 FFI struct 使用固定布局。
- 所有跨 FFI enum 使用固定底层整数表示。
- ISR-safe API 必须可在中断上下文调用；非 ISR-safe API 只能在 `vp_core_poll()` 或普通 task 上下文调用。

---

## 2. 通用 ABI 类型

### 2.1 基本约定

- timestamp 统一使用 RTC。
- 事件 callback 默认携带 `vp_timestamp_t`。
- bool 字段跨 struct 时使用 `uint8_t`，取值 `0/1`。
- buffer 使用 pointer + length。
- Rust 不保存 ISR callback 传入的裸指针。
- 所有多字节持久化字段使用 little-endian；运行期 FFI struct 使用目标 ABI 原生布局，但必须由工具生成声明。

### 2.2 通用类型

| 类型 | 建议 C 表示 | 说明 |
| --- | --- | --- |
| `vp_timestamp_t` | `uint32_t` | RTC millis。按 32-bit wrapping time 处理回绕。 |
| `vp_bool_t` | `uint8_t` | FFI struct 内 bool，`0=false`，`1=true`。 |
| `vp_status_t` | `uint8_t` enum | 通用返回状态。 |

### 2.3 通用状态码

| 值 | 名称 | 说明 |
| --- | --- | --- |
| `0` | `VP_STATUS_OK` | 成功。 |
| `1` | `VP_STATUS_BUSY` | 资源忙。 |
| `2` | `VP_STATUS_INVALID_ARG` | 参数非法。 |
| `3` | `VP_STATUS_NOT_READY` | 外设或协议栈未就绪。 |
| `4` | `VP_STATUS_IO_ERROR` | 底层 I/O 错误。 |
| `5` | `VP_STATUS_UNSUPPORTED` | 当前平台/模式不支持。 |

---

## 3. C → Rust exported callbacks

这些函数由 Rust 实现，通过 `cbindgen` 生成到 `platform/Bind/rust_api.h`，C 层在中断、协议栈事件或 TMOS task 中调用。

### 3.1 生命周期

| API | 上下文 | 说明 |
| --- | --- | --- |
| `vp_core_init()` | task | 初始化 Rust Runtime、加载配置、初始化状态机。 |
| `vp_core_poll()` | TMOS task | Rust bottom-half，处理 deferred work。 |

`vp_core_poll()` 由 C 的专用 TMOS event 调度。它不是硬件轮询函数，只处理 Rust 内部 pending work。

### 3.2 输入事件

| API | 上下文 | ISR-safe | 说明 |
| --- | --- | --- | --- |
| `vp_on_button_exti(button_id, level, timestamp)` | GPIO EXTI | 是 | 普通按键 EXTI 唤醒。 |
| `vp_on_mode_switch_exti(level, timestamp)` | GPIO EXTI | 是 | 物理模式开关 EXTI 唤醒。 |
| `vp_on_debounce_tick(timestamp)` | Timer ISR | 是 | 共享 1ms debounce tick。 |
| `vp_on_encoder_exti(a_level, b_level, timestamp)` | GPIO EXTI | 是 | 编码器 A/B 任意边沿。 |

#### `button_id`

| 值 | 名称 |
| --- | --- |
| `0` | Left |
| `1` | Right |
| `2` | Middle |
| `3` | Action |
| `4` | Laser |

#### `vp_input_id_t`

| 值 | 名称 | 说明 |
| --- | --- | --- |
| `0` | Left | 左键输入。 |
| `1` | Right | 右键输入。 |
| `2` | Middle | 中键输入。 |
| `3` | Action | Action 键输入。 |
| `4` | Laser | Laser 键输入。 |
| `5` | ModeSwitch | 物理模式开关输入。 |
| `6` | EncoderA | 编码器 A 相。 |
| `7` | EncoderB | 编码器 B 相。 |
| `8` | ImuInt1 | LSM6DSV INT1。是否实际连接由 PCB 决定。 |
| `9` | ImuInt2 | LSM6DSV INT2。是否实际连接由 PCB 决定。 |

USB attach/configured/suspend 状态通过 USB stack callback 上报，不作为 GPIO input id。

#### `vp_output_id_t`

| 值 | 名称 | 说明 |
| --- | --- | --- |
| `0` | Laser | 激光输出控制。 |

当前只定义 Laser。充电灯目前不走 MCU；后续如需 MCU 控制 LED/RF/power rail，再扩展该 enum。

### 3.3 IMU 事件

| API | 上下文 | ISR-safe | 说明 |
| --- | --- | --- | --- |
| `vp_on_imu_int(timestamp)` | GPIO EXTI | 是 | LSM6DSV INT 唤醒。Rust 决定是否请求 FIFO 读取。 |
| `vp_on_imu_sample(raw_x, raw_y, raw_z, timestamp)` | I2C completion/task | 是/短路径 | C 读取到最新 SFLP sample 后回调 Rust。 |
| `vp_on_imu_fifo_done(status, dropped_count, timestamp)` | I2C completion/task | 可选 | FIFO 读取批次完成通知。 |

`raw_x/raw_y/raw_z` 是 LSM6DSV SFLP game rotation 输出的 half-float bits，类型为 `uint16_t`。

### 3.4 连接与路由事件

| API | 上下文 | 说明 |
| --- | --- | --- |
| `vp_on_ble_connected(timestamp)` | protocol callback | BLE 已连接。 |
| `vp_on_ble_disconnected(reason, timestamp)` | protocol callback | BLE 已断开。 |
| `vp_on_dongle_connected(timestamp)` | protocol callback | 2.4G dongle 已连接。 |
| `vp_on_dongle_disconnected(reason, timestamp)` | protocol callback | 2.4G dongle 已断开。 |
| `vp_on_usb_state_changed(state, timestamp)` | USB callback | USB attach/configured/detach 状态变化。 |

### 3.5 HID 完成事件

如果底层 HID 发送是同步返回，则无需该 callback。如果底层发送是异步完成，则使用：

| API | 上下文 | 说明 |
| --- | --- | --- |
| `vp_on_hid_send_done(route, status, timestamp)` | protocol callback | HID 发送完成或失败。 |

### 3.6 Vendor/WebHID RX

| API | 上下文 | 说明 |
| --- | --- | --- |
| `vp_on_vendor_report_rx(route, ptr, len, timestamp)` | protocol callback | 收到 vendor report。Rust 复制/入队后在 `vp_core_poll()` 解析。 |

---

## 4. Rust → C imported APIs

这些函数由 C 在 `platform/Bind/c_api.h` 中声明和实现，由 `bindgen` 生成 Rust bindings。

---

## 5. GPIO / EXTI API

### 5.1 GPIO 输入

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_gpio_read(input_id)` | 是 | 读取单个输入电平。 |
| `c_vp_gpio_read_inputs(out_snapshot)` | 是 | 批量读取输入电平。 |

`input_id` 包括普通按键、ModeSwitch、编码器 A/B、IMU INT 等硬件输入。

### 5.2 GPIO 输出

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_gpio_write(output_id, level)` | 是 | 写 GPIO 输出，例如 Laser。 |

### 5.3 EXTI 控制

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_exti_mask(input_id)` | 是 | 屏蔽指定输入 EXTI。 |
| `c_vp_exti_unmask(input_id)` | 是 | 重新开启指定输入 EXTI。 |
| `c_vp_exti_clear_pending(input_id)` | 是 | 清除指定输入 pending。 |
| `c_vp_exti_set_edge(input_id, edge)` | 是 | 设置下一次语义转换；平台可按输入类型映射为边沿、电平触发或模拟实现。 |

### 5.4 EXTI edge 类型

#### `vp_exti_edge_t`

| 值 | 名称 | 说明 |
| --- | --- | --- |
| `0` | `Rising` | 上升沿。 |
| `1` | `Falling` | 下降沿。 |
| `2` | `Both` | 任意边沿。平台层负责映射或模拟。 |

CH585 WCH 标准外设 API 原生提供低电平/高电平/下降沿/上升沿触发，未直接提供双边沿；`Both` 是否可用由平台层按具体输入实现。普通低有效二态输入（按键/自锁开关）把 `Falling`/`Rising` 视作“下一次语义转换”而非强制硬件边沿：平台可映射为低电平/高电平触发，以避开机械触点上的 GPIOA 边沿锁存问题。编码器 A/B 需要 `Both`，当前由平台层按当前电平重配下一边沿模拟。

---

## 6. Timer / RTC / TMOS API

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_debounce_timer_start()` | 是 | 启动共享 debounce tick 来源。 |
| `c_vp_debounce_timer_stop()` | 是 | 停止共享 debounce tick 来源。 |
| `c_vp_rtc_tick()` | 是 | 读取 RTC tick。 |
| `c_vp_rtc_millis()` | 是 | 读取 RTC millis。 |
| `c_vp_rtc_micros()` | 是 | 读取 RTC micros，可选调试用。 |
| `c_vp_rtc_set_wake_after(ms)` | 否 | 配置 RTC 唤醒。 |
| `c_vp_request_core_poll()` | 是 | 合并/触发 TMOS event，调度 `vp_core_poll()`。 |

`c_vp_request_core_poll()` 必须只做置位或合并 event，不得直接调用 `vp_core_poll()`。

---

## 7. I2C / IMU API

### 7.1 I2C

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_i2c_init()` | 否 | 初始化 I2C master。 |
| `c_vp_i2c_recover_bus()` | 否 | I2C bus recovery。 |
| `c_vp_i2c_abort()` | 否 | 中止当前 I2C 事务。 |

### 7.2 IMU profile

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_imu_config_active()` | 否 | 配置 Active profile。 |
| `c_vp_imu_config_suspend()` | 否 | 配置 Suspend profile。 |
| `c_vp_imu_config_sleep()` | 否 | 配置 Sleep wake profile。 |

### 7.3 IMU FIFO

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_imu_read_fifo_async(max_samples)` | 否 | 请求异步读取 FIFO。完成后 C 回调 `vp_on_imu_sample()`。 |
| `c_vp_imu_read_whoami(out_id)` | 否 | 读取 WHO_AM_I，用于诊断/初始化。 |

IMU INT 到来时，C 不直接读 FIFO，而是调用 `vp_on_imu_int()`。Rust 在 `vp_core_poll()` 中根据状态请求 `c_vp_imu_read_fifo_async()`。

---

## 8. HID / Route API

### 8.1 类型

#### `vp_hid_route_t`

| 值 | 名称 |
| --- | --- |
| `0` | None |
| `1` | BLE |
| `2` | Dongle2G4 |
| `3` | USB |

#### `vp_hid_send_status_t`

| 值 | 名称 | 说明 |
| --- | --- | --- |
| `0` | Sent | 已发送或已入底层发送队列。 |
| `1` | RetryLater | 暂时不可发送。 |
| `2` | NotConnected | route 未连接。 |
| `3` | Fatal | 不可恢复错误。 |

### 8.2 Mouse report

目标 mouse report 字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `buttons` | `uint8_t` | bit0 left, bit1 right, bit2 middle。 |
| `dx` | `int8_t` | X delta。 |
| `dy` | `int8_t` | Y delta。 |
| `wheel` | `int8_t` | wheel delta。 |

### 8.3 API

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_hid_route_ready(route)` | 否 | 查询 route 是否可发送。 |
| `c_vp_hid_send_mouse(route, buttons, dx, dy, wheel)` | 否 | 发送 mouse report。 |
| `c_vp_hid_send_vendor(route, ptr, len)` | 否 | 发送 vendor report。`len` 使用 `uint16_t`。 |
| `c_vp_hid_route_enable(route, enabled)` | 否 | 开关 route。 |
| `c_vp_hid_route_reset(route)` | 否 | route 错误恢复。 |

HID report 内容只由 Rust 决定，C 不修改 buttons/dx/dy/wheel 语义。C 可做 HID 物理范围安全修正，例如避免发送 `-128`，但首选由 Rust report 层处理。

---

## 9. Power API

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_power_prepare_suspend()` | 否 | 准备进入 Suspend。 |
| `c_vp_power_enter_suspend()` | 否 | 执行 Suspend。 |
| `c_vp_power_prepare_sleep()` | 否 | 准备进入 Sleep。 |
| `c_vp_power_enter_sleep()` | 否 | 执行 Sleep。 |
| `c_vp_power_restore_from_sleep()` | 否 | wake 后恢复平台外设。 |
| `c_vp_wake_source_enable(source, enabled)` | 否 | 配置 wake source。 |

是否进入 `Active`/`Suspend`/`Sleep` 由 Rust 判断，C 只执行。

---

## 10. DataFlash API

### 10.1 Storage info

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_flash_config_region(out_info)` | 否 | 查询配置区起始、长度、page size、write alignment。 |

### 10.2 Read/write

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_flash_read(offset, ptr, len)` | 否 | 从 config region 读取。`offset` / `len` 使用 `uint32_t`。 |
| `c_vp_flash_erase(offset, len)` | 否 | 擦除 config region 内指定范围。`offset` / `len` 使用 `uint32_t`。 |
| `c_vp_flash_write(offset, ptr, len)` | 否 | 写入 config region。`offset` / `len` 使用 `uint32_t`。 |

C 只按 offset/len 操作，不解析配置 header/payload。

---

## 11. Debug / Diagnostics API

| API | ISR-safe | 说明 |
| --- | --- | --- |
| `c_vp_debug_print(ptr, len)` | 视实现而定 | Rust logger/panic 输出。ISR 中应避免长输出。 |
| `c_vp_platform_reset(reason)` | 否 | 可选：请求平台复位。 |

---

## 12. 调度约定

### 12.1 快速 callback

C ISR 或协议栈回调调用 Rust callback 后，Rust 可以：

- 更新短状态。
- 推入 SPSC event queue。
- 更新 latest-sample cache。
- 设置 dirty/pending flag。
- 调用 ISR-safe C API。
- 调用 `c_vp_request_core_poll()`。

Rust 不应在快速 callback 中：

- 写 DataFlash。
- 发送 HID report。
- 阻塞等待 I2C。
- 解析复杂 WebHID 命令。
- 执行 sleep enter。

### 12.2 `vp_core_poll()`

`vp_core_poll()` 必须运行在非 ISR 上下文，但当前稳定实现并不是“收到 TMOS event 就立刻在 TMOS task 内直接执行”。实际流程是：

1. Rust callback / C 平台逻辑调用 `c_vp_request_core_poll()`。
2. `RuntimeTask_RequestPoll()` 仅置位 `runtime_poll_request_pending`。
3. 主循环 `Main_Circulation()` 每轮执行两次 `RuntimeTask_Service()`，分别位于 `TMOS_SystemProcess()` 前后。
4. `RuntimeTask_Service()` 先补服务 GPIOA 锁存中断、推进 debounce 软时基，再在 `runtime_poll_request_pending != 0` 时调用 `vp_core_poll()`。
5. `RuntimeTask_RequestPollAfter(ms)` 在 `ms > 0` 时通过 TMOS timer 投递 `RUNTIME_CORE_POLL_EVT`；`RuntimeTask_ProcessEvent()` 收到事件后只把 `runtime_poll_request_pending` 置位，真正的 `vp_core_poll()` 仍由下一轮 `RuntimeTask_Service()` 执行。

因此，`vp_core_poll()` 的职责保持不变：

- event queue。
- HID report send/retry。
- IMU FIFO async read request。
- WebHID command parse。
- config save。
- power transition。

但它的**实际执行入口**是主循环侧的 runtime service，而不是 TMOS 事件处理函数直接调用。这是当前项目已验证的稳定调度结构。

如果 `vp_core_poll()` 处理后仍有 pending work，应再次调用 `c_vp_request_core_poll()`。

---

## 13. 生成工具要求

- `bindgen` 输入：`platform/Bind/c_api.h`。
- `cbindgen` 输出：`platform/Bind/rust_api.h`。
- 新增 Rust exported function 后必须确保 `cbindgen` 能生成稳定 C 声明。
- 新增 C API 后必须确保 `bindgen` 能生成 no_std 可用 bindings。
- CI/构建应保证 `rust_api.h` 与 Rust exported ABI 同步。

---

## 14. ABI implementation checklist

本节是 FFI 实现前检查项；后续以本文为 FFI/ABI 单一入口。

### 14.1 Fixed types

| 类型 | 底层类型 / 状态 |
| --- | --- |
| `vp_timestamp_t` | `uint32_t` RTC millis，已确认。 |
| `vp_bool_t` | `uint8_t`，`0=false`，`1=true`。 |
| `vp_status_t` | `uint8_t` enum。 |
| `vp_button_id_t` | `uint8_t` enum：Left / Right / Middle / Action / Laser。 |
| `vp_input_id_t` | `uint8_t` enum：button / mode / encoder / IMU inputs；USB 状态通过 USB stack callback。 |
| `vp_output_id_t` | `uint8_t` enum：当前只定义 Laser。 |
| `vp_exti_edge_t` | `uint8_t` enum：Rising / Falling / Both。 |
| `vp_hid_route_t` | `uint8_t` enum。 |
| `vp_hid_send_status_t` | `uint8_t` enum。 |
| `vp_usb_state_t` | `uint8_t` enum：Detached / Attached / Configured / Suspended / Error。 |
| `vp_wake_source_t` | diagnostics-only bitmask，不参与业务决策。 |
| `vp_power_state_t` | 不跨 FFI 暴露。 |
| `vp_flash_status_t` | 不使用，统一返回 `vp_status_t`。 |

### 14.2 Pointer and buffer ownership

- C → Rust callback 中的 `ptr,len` 只在 callback 期间有效。
- Rust 必须立即复制 vendor report，不保存裸指针。
- Rust → C flash write / HID vendor send 的 `ptr,len` 在函数返回前有效，C 不保存指针；如果底层异步，C 必须复制。
- `out_*` 参数由调用方分配，由被调用方写入。
- HID/vendor report length 使用 `uint16_t`。
- flash/config buffer length 使用 `uint32_t`。

### 14.3 Context rules

| 上下文 | 允许行为 |
| --- | --- |
| ISR | 只做短路径、不可阻塞、不可 malloc、不可 flash。 |
| protocol callback | 不阻塞，允许入队。 |
| TMOS task / `vp_core_poll()` | 允许 HID send、I2C request、config parse、power transition。 |
| init task | 允许初始化硬件和 Runtime。 |

### 14.4 Reentrancy rules

- `vp_core_poll()` 严格不可重入；如果执行期间再次请求 poll，只能合并 pending event，下次再运行。
- 不要求 C 层额外序列化所有 Rust callback。
- callback 设计原则是互相不共享业务状态；即使被中断打断，也不应破坏状态。
- 如果实现中确实存在共享底层结构，例如事件队列、latest sample cache、pending flags，则该结构本身必须 ISR-safe / reentrant-tolerant。
- ISR callback 只做极短路径。
- Runtime 长耗时操作不能持有全局 mutable borrow。
- 复杂状态变更集中在 `vp_core_poll()`。

### 14.5 ABI version

v1 不需要 ABI version 运行时校验。

原因：C/Rust FFI header 全部自动生成且一起编译，不存在独立分发造成的版本漂移问题。可以保留编译期注释或常量，但不要求运行时校验。
