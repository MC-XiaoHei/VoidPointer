# VoidPointer 固件实现 TaskList

本文是实现阶段主入口，按顺序拆分基础设施、FFI、平台层、Rust Core、典型流程和完成标准。当前代码与目标规格差距也集中记录在本文，避免把“已设计”误认为“已实现”。

## Current implementation status

| 模块 | 当前代码状态 | v1 目标 / 主要差距 |
| --- | --- | --- |
| Rust lifecycle | 已切到 `vp_core_init()` / `vp_core_poll()`；`c_vp_request_core_poll()` 已接到 TMOS event；临时 debug polling bridge 已移除；`core/src/utils/runtime.rs` 仍残留 legacy `tick()` 原型但未接入当前 Runtime | 仍需完善 deferred work 处理并清理 legacy 原型。 |
| Input | 已有 Rust input/encoder 原型；新 Runtime 已通过低有效二态输入 + 电平 EXTI + debounce 接入按钮，通过 GPIOA 待处理服务补偿 CH585 PFIC 未再次派发的待处理中断标志；encoder EXTI 接入 wheel 到 BLE HID | mode switch GPIO 映射与 route policy、IMU INT。 |
| Motion / Attitude | 已有 attitude/motion 模块和 LSM6DSV 原型 | 接入 IMU INT + async FIFO + latest sample cache。 |
| Report / HID | 已有 report/hid 模块和 BLE HID 原型；新 Runtime 已打通最小 GPIO snapshot → BLE mouse buttons/wheel report，`RetryLater` 保留 pending | 扩展为 route-aware HID send、USB/2.4G stub、motion dx/dy 聚合、异步完成处理。 |
| Route | 已有独立 Rust route 模块骨架，维护 BLE/dongle connected 与 USB state | 仍需 route selection、`usb_mouse_policy`、Vendor route 优先级、USB/2.4G 发送实现。 |
| Power | 已有独立 Rust power 模块骨架和简化 `Active` / `Suspend` / `Sleep` 转换；Power transition 已改为 RuntimeCommand 边界，不在 PowerManager 内直接调用 C API；power timeout 已安排 delayed poll 重新评估 | blocker、断连时间门控、平台 power API、wake/restore、配置参数仍需补齐。 |
| Config | 已有 `ConfigManager` dirty flag 骨架 | 仍需 `DeviceConfig`、双槽读写、CRC、migration、runtime apply。 |
| Vendor/WebHID | 已有 `VendorRuntime` pending RX 骨架；新增固定容量 vendor RX payload queue，callback 短路径复制最多 64B payload 后由 bottom-half drain | 仍需 vendor report protocol、命令解析、响应与保存流程；USBHS 512B payload/分片策略未完成。 |
| FFI ABI | `platform/Bind/c_api.h` 已目标化，`bindgen`/`cbindgen` 已接入并生成 `rust_api.h` | 大量 C API 仍是 `UNSUPPORTED`，callback 多数只置位 pending/dirty。 |
| Platform C | 有 CH585/LSM6DSV/BLE 原型 | 拆成底层 API，不持有业务状态。 |

---

## 0. 基础设施任务

- [ ] 实现 ISR-safe 与 bottom-half 执行规则。（已建立固定容量 ISR event queue，C→Rust 快速 callback 不再直接借用 Runtime；具体业务事件仍需逐步接入真实 EXTI/I2C/Vendor 数据）
  - [x] ISR-safe Rust callback 只做短状态更新、事件入队、调用 C 的轻量 API。
  - [ ] HID retry、I2C 读取请求、DataFlash 写入、WebHID 解析、电源切换放到 `vp_core_poll()`。（HID retry、Power transition、IMU FIFO read request 已走 poll/RuntimeCommand；HID RetryLater 现在使用延迟 TMOS poll 而非立即 tight retry；Vendor RX payload 已固定缓冲并在 poll 中 drain；DataFlash/WebHID 协议解析未完成）
  - [x] ISR 或 Rust 快速回调中如产生 deferred work，Rust 通过 C API 请求 TMOS event 调度 `vp_core_poll()`。
- [ ] 接入 RTC 作为统一事件时间戳。（Rust runtime 与已接入 BLE 事件使用 RTC millis；EXTI/debounce/IMU/USB 平台事件待接入时仍需逐项验证）
  - [x] Rust 统一使用 C 提供的 RTC timestamp。
  - [x] C→Rust 事件回调均携带 RTC timestamp。（ABI 均携带 timestamp；当前已接入 BLE connected/disconnected、GPIO EXTI 与 debounce timer tick；IMU/USB 平台事件待接入时仍需逐项验证）
  - [ ] debounce、activity timeout、Suspend/Sleep timeout 均基于 RTC timestamp 计算。（activity 与 power timeout 已基于 RTC millis；power timeout 已可安排 delayed poll 重新评估；debounce tick 已携带 RTC timestamp，完整 release/mode-switch 策略仍需继续完善）
- [ ] 实现 TMOS bottom-half 调度。
  - [x] C 注册专用 TMOS task/event 用于调用 `vp_core_poll()`。
  - [x] Rust 暴露 deferred work 标志或通过 C API 请求触发该 TMOS event。
  - [x] 多次请求可合并，避免重复排队。（即时 core poll 与 delayed core poll 均走专用 TMOS event；临时 debug fallback poll 已移除）
  - [x] `vp_core_poll()` 每次执行到无 pending work 或达到单次预算后返回。
  - [x] Suspend/Sleep timeout 使用 delayed TMOS poll 明确调度下一次 power eval。
  - [x] 主循环只运行 `TMOS_SystemProcess()`/协议栈，不直接轮询业务状态。
- [ ] 实现全局 Runtime 访问保护。
  - [x] 确定使用短临界区或 ISR event queue。
  - [ ] 禁止在长耗时操作期间持有 Runtime 可变借用。（已为 HID mouse send、Power transition、IMU FIFO read request 建立 RuntimeCommand 边界，先释放 Runtime 借用再调用 C API；Flash/DataFlash 等长耗时调用仍需继续拆分）

---

## 1. FFI ABI 设计

> 具体 ABI 框架与演进规则见 `../FFI_ABI.md`。

### 1.1 C → Rust exported callbacks

- [ ] 生命周期入口。
  - [ ] `vp_core_init()`：Rust Runtime 初始化、配置加载、初始状态同步。（入口函数、Runtime 初始化、初始 input snapshot/encoder sync 已完成；配置加载未完成）
  - [ ] `vp_core_poll()`：Rust bottom-half，由 TMOS event 调度；处理 deferred work、HID retry、I2C 读取请求、配置任务、电源决策。（入口函数、TMOS event 调度、固定容量 event queue、HID/Power/IMU FIFO RuntimeCommand 边界、pass budget、电源/config/vendor 骨架、最小 BLE HID retry pending 已完成；DataFlash/WebHID 等未完成；临时 debug polling bridge 已移除）
- [ ] 输入入口。
  - [x] `vp_on_button_exti(button_id, level, timestamp)`。（ABI/callback 入队路径已完成；C GPIOA 低有效二态输入电平 EXTI 已接入）
  - [x] `vp_on_debounce_tick(timestamp)`。（ABI/callback 入队路径已完成；debounce tick 由 main runtime service 基于 RTC millis 驱动）
  - [x] `vp_on_encoder_exti(a_level, b_level, timestamp)`。（ABI/callback 入队路径已完成；C GPIOA EXTI 已接入）
  - [ ] `vp_on_mode_switch_exti(level, timestamp)`。（ABI/callback 入队路径已完成；C EXTI 未接入）
- [ ] IMU 入口。
  - [ ] `vp_on_imu_int(timestamp)`。（ABI/callback 入队路径已完成；C IMU INT 未接入）
  - [ ] `vp_on_imu_sample(raw_x, raw_y, raw_z, timestamp)`。（ABI/callback 入队路径已完成；async FIFO 未接入）
  - [ ] 可选：`vp_on_imu_fifo_done(status, dropped_count, timestamp)`。（ABI/callback 入队路径已完成；async FIFO 未接入）
- [ ] HID / 连接入口。
  - [x] `vp_on_ble_connected(timestamp)`。
  - [x] `vp_on_ble_disconnected(reason, timestamp)`。
  - [ ] `vp_on_dongle_connected(timestamp)`。（ABI/callback 入队路径已完成；2.4G 协议栈未接入）
  - [ ] `vp_on_dongle_disconnected(timestamp)`。（ABI/callback 入队路径已完成；2.4G 协议栈未接入）
  - [ ] `vp_on_usb_state_changed(state, timestamp)`。（ABI/callback 入队路径已完成；USB stack 未接入）
  - [ ] `vp_on_hid_send_done(route, status, timestamp)`，如果发送采用异步完成模型。（ABI/callback 入队路径已完成；当前 BLE send 为同步返回）
- [ ] WebHID / Vendor 入口。
  - [ ] `vp_on_vendor_report_rx(route, ptr, len, timestamp)`。（payload 固定缓冲复制与 callback 入队路径已完成；协议解析未完成，当前只支持最多 64B 单包）

### 1.2 Rust → C imported hardware APIs

- [ ] GPIO / EXTI API。
  - [x] 读取单个 GPIO 输入。
  - [x] 批量读取按键 GPIO。
  - [ ] 写 GPIO 输出，例如 Laser。
  - [x] mask/unmask 指定 EXTI。（当前支持已映射 GPIOA 输入）
  - [x] 清 EXTI pending。（当前支持已映射 GPIOA 输入）
  - [x] 配置 EXTI 边沿/语义转换。（Encoder A/B 的 Both 使用按当前电平重配下一边沿模拟；低有效二态输入将 Falling/Rising 语义映射为低电平/高电平触发）
- [ ] Timer / RTC API。
  - [x] 启动/停止 1ms debounce timer。
  - [x] 请求 TMOS event 调度 `vp_core_poll()`。
  - [x] 请求延迟 TMOS poll，用于 BLE HID RetryLater 等退避重试。
  - [x] 读取 RTC tick。
  - [x] 读取 RTC millis。
  - [x] 读取 RTC micros，可选。
  - [x] 配置 RTC wake。
- [ ] I2C / IMU API。
  - [ ] 按 `../RESOURCE_PROFILE.md` 中的 LSM6DSV/I2C profile 实现目标接口。
  - [ ] I2C 初始化。
  - [ ] I2C bus recover。
  - [ ] IMU active profile 配置。
  - [ ] IMU suspend profile 配置。
  - [ ] IMU sleep wake profile 配置。
  - [ ] 异步读取 IMU FIFO。
  - [ ] 中止 I2C 事务。
- [ ] HID API。
  - [ ] 按 `../RESOURCE_PROFILE.md` 中的 BLE/USB HID profile 实现目标 mouse/vendor report 接口。
  - [ ] 查询 route ready。
  - [ ] 发送 mouse report。
  - [ ] 发送 vendor report。
  - [ ] BLE/2.4G/USB route enable/disable。
- [ ] Power API。
  - [ ] 准备进入 `Suspend`。
  - [ ] 执行进入 `Suspend`。
  - [ ] 准备进入 `Sleep`。
  - [ ] 执行进入 `Sleep`。
  - [ ] sleep resume 后恢复外设。
  - [ ] 配置 wake source。
- [ ] DataFlash API。
  - [ ] flash read。
  - [ ] flash page erase。
  - [ ] flash aligned write。
  - [ ] 查询 page size / config region。
- [ ] Debug API。
  - [ ] UART print。
  - [ ] 可选：panic reason 输出。

### 1.3 现有 FFI 框架迁移任务

- [x] 保留现有 `bindgen` / `cbindgen` 自动生成机制。
- [ ] 一次性替换 legacy `init_core()` / `tick()`；v1 不保留 legacy wrapper 过渡。（主入口已替换；`core/src/utils/runtime.rs` 仍残留未接入的 legacy tick 原型，待清理）
- [x] 将 Rust exported `init_core()` 替换为 `vp_core_init()`。
- [ ] 将 Rust exported `tick()` 替换为 `vp_core_poll()`，并改为 TMOS event bottom-half 语义。（export 已替换；`c_vp_request_core_poll()` 已接入 event；临时 1-tick delayed polling bridge 已移除）
- [x] 将 C API 命名迁移到 `c_vp_*` 前缀。
- [ ] 用事件化输入 callback 替代主路径中的同步 `c_get_input_status()`。（按键与 encoder 主路径已走 EXTI/debounce；事件处理中的 GPIO 读取仍用于 debounce 采样、初始 sync 与 encoder 状态兜底同步）
- [ ] 用 GPIO/EXTI/Timer API 替代输入业务中的直接 GPIO 快照轮询。（已使用目标 `c_vp_gpio_read()`；button EXTI/debounce 与 encoder EXTI 已接入；ModeSwitch 未完成）
- [ ] 用 IMU INT + async FIFO + `vp_on_imu_sample()` 替代主路径中的同步 `c_read_sflp_game_rotation_raw()`。（callback/event queue 与 `c_vp_imu_read_fifo_async()` RuntimeCommand 请求路径已完成；C 异步 I2C/FIFO 状态机未完成）
- [ ] 将 BLE-only `c_send_ble_hid_mouse_report()` 扩展或替换为 route-aware `c_vp_hid_send_mouse(route, ...)`。
- [x] 保留 RTC API 能力，并按目标 API 命名提供 `c_vp_rtc_tick/millis/micros()`。
- [ ] 为每个新增 Rust→C API 标注 ISR-safe 或 bottom-half only。
- [x] 为每个新增 C→Rust callback 确认 `cbindgen` 生成声明稳定可用。

---

## 2. C 平台层任务

### 2.1 启动与初始化

- [ ] 整理 `main.c` 启动流程。
  - [x] 系统时钟初始化。
  - [ ] 低功耗安全 GPIO 默认态。
  - [x] Debug UART 可选初始化。
  - [x] RTC/timebase 初始化。
  - [x] I2C 初始化。
  - [x] IMU 基础通信初始化。
  - [x] BLE/USB/2.4G/HID 栈初始化。（BLE/HID 已初始化；USB/2.4G 仍未接入）
  - [x] 调用 `vp_core_init()`。
  - [ ] 根据 Rust 返回/调用结果配置 EXTI、IMU profile、HID route。

- [x] C 主循环只运行 `TMOS_SystemProcess()`、协议栈调度和硬件 pending bottom-half，不做业务轮询。（Runtime 通过即时/延迟 TMOS event 调度；GPIOA pending service 只消费 `R16_PA_INT_IF & R16_PA_INT_EN` 这个硬件中断事实；临时 debug fallback poll 已移除）

### 2.2 GPIO / EXTI

- [x] 为 Left/Right/Middle/Action/Laser/ModeSwitch 配置 GPIO 输入。（当前 `InputGPIO_Init()` 覆盖按键/编码器/Laser 引脚；ModeSwitch 实际 GPIO 映射仍需确认，`c_vp_gpio_read()` 对未映射输入返回 0）
- [x] 为编码器 A/B 配置任意边沿输入。（CH585 StdPeriph 无原生 both-edge，当前用“读当前电平后重配下一边沿”模拟；Rust encoder lookup 兜底非法跳变）
  - [x] CH585 StdPeriph 未暴露普通 GPIO IRQ both-edge 时，平台层用“读当前电平后重配下一边沿”模拟。
  - [x] Rust 编码器状态机用 old/new 4-bit lookup 兜底非法跳变；必要时增加短周期采样兜底。
- [x] 为普通按键配置 EXTI wake。（低有效二态输入：Rust 请求 Falling/Rising 语义，CH585 平台映射为低电平/高电平触发，避免机械按键下降沿锁存一次性问题；低功耗 wake source 尚未配置）
- [ ] 为 ModeSwitch 配置 EXTI wake。
- [x] 按键/二态输入 EXTI service 中只做：识别 pin、读低有效电平、拿 timestamp、屏蔽对应 EXTI、调用 Rust。（屏蔽是为避免 debounce 处理前的弹跳中断风暴；下一相反电平由 Rust debounce policy 在稳定态确认后重新 arm）
- [x] 编码器 EXTI ISR 中只做：读 A/B level、拿 timestamp、调用 Rust。
- [x] 提供 Rust 可调用的 EXTI mask/unmask/clear/edge 配置 API。（当前支持已映射 GPIOA 输入；低有效二态输入将语义 edge 映射为电平触发；ModeSwitch/IMU INT 映射待确认）

### 2.3 Debounce Timer

- [x] 实现共享 debounce tick。（当前由 main runtime service 基于 RTC millis 产生 1ms tick；保留 TMR0 IRQ 原型但实机调试路径不依赖 TMR0）
- [x] debounce tick 调用 `vp_on_debounce_tick(timestamp)`。
- [x] 普通按键与 ModeSwitch 复用同一个 debounce tick，差异由上层事件语义处理。（普通按键已作为二态输入接入；ModeSwitch 可复用同一状态机，但 GPIO 映射仍未知）
- [x] Timer 是否继续运行由 Rust 通过 C API 控制。
- [x] C 不维护输入业务状态；仅维护 GPIOA/Timer 硬件服务状态。

### 2.4 I2C / IMU

> LSM6DSV 寄存器、SFLP/FIFO 参数和 I2C 依据见 `../RESOURCE_PROFILE.md`。

- [x] CH585 I2C master 配置。
  - [x] SDA/SCL 使用 PCB 实际引脚，当前规划为 `PB12/PB13`。
  - [x] 400 kHz。
  - [x] 7-bit address。
  - [x] ACK enable。
  - [ ] 支持 bus idle 检查。
- [ ] 实现 I2C bus recovery。
  - [ ] SCL pulse。
  - [ ] STOP condition。
  - [ ] I2C peripheral reset。
- [x] 实现 LSM6DSV 基础寄存器访问。
  - [x] read register。
  - [x] write register。
  - [x] burst read。
  - [x] WHO_AM_I 检查。
- [ ] 实现 LSM6DSV 三个 profile。
  - [ ] `Active` profile：按 `../RESOURCE_PROFILE.md`，SFLP enabled，120 Hz 起步，FIFO continuous/latest-sample。（当前已有一组原型初始化寄存器值，仍需整理为 profile table/API）
  - [ ] `Suspend` profile：按 `../RESOURCE_PROFILE.md`，默认不依赖 SFLP 角度检测，使用 activity/inactivity 或 wake-up interrupt，低 ODR/gyro sleep。
  - [ ] `Sleep` profile：按 `../RESOURCE_PROFILE.md`，SFLP disabled，gyro power-down，accel wake-up/significant motion。
- [ ] 实现 IMU wake 配置。
  - [ ] 默认使用 LSM6DSV accelerometer-based wake-up/activity-inactivity/significant-motion interrupt。
  - [ ] SFLP 角度检测只作为可选二级确认策略，不作为默认唤醒路径。
  - [ ] 配置 `FUNCTIONS_ENABLE`、`INACTIVITY_DUR`、`WAKE_UP_THS`、`WAKE_UP_DUR`、`MD1_CFG/MD2_CFG`。
  - [ ] wake 后读取 `WAKE_UP_SRC` 或相关 status，回调 Rust 进行 power state 决策。
- [ ] 实现 IMU INT ISR。
  - [ ] 清 pending。
  - [ ] 调 `vp_on_imu_int(timestamp)`。
  - [ ] 不在 ISR 中阻塞读取 FIFO。
- [ ] 实现异步 FIFO 读取。
  - [ ] Rust 请求后启动 I2C 状态机。
  - [ ] I2C IRQ 驱动读取。
  - [x] C 可解析 FIFO tag，筛选 SFLP game rotation raw sample。（同步 polling 原型已实现）
  - [ ] C 将 raw half-float x/y/z 回调给 Rust。

### 2.5 HID / 连接栈

> BLE HID mouse report、USB vendor HID endpoint/report 依据见 `../RESOURCE_PROFILE.md`。

- [x] 封装 BLE HID mouse report 发送。
- [ ] 实现 USB HID mouse report 发送。
- [ ] 实现 USB Vendor HID report 收发。
- [ ] 2.4G dongle report 发送保留 stub，返回 `Unsupported` 或 `NotConnected`，待 dongle 协议栈选型后补齐。
- [ ] 将底层发送结果映射为统一状态。（BLE route 已映射，USB/2.4G 仍待实现）
  - [x] `Sent`。
  - [x] `RetryLater`。
  - [x] `NotConnected`。
  - [x] `Fatal`。
- [x] BLE connected/disconnected 事件回调 Rust。
- [ ] USB attach/detach/configured 事件回调 Rust。
- [ ] 2.4G dongle connected/disconnected 事件回调 Rust。
- [ ] Vendor report RX 只传 raw bytes 给 Rust。


### 2.6 Power

- [ ] 实现 `Suspend` 平台准备和进入。
  - [ ] RF 保持连接。
  - [ ] IMU 低功耗 profile。
  - [ ] 按键/编码器/IMU/USB wake source 保持。
- [ ] 实现 `Sleep` 平台准备和进入。
  - [ ] RF 关闭。
  - [ ] IMU 极低功耗 wake profile。
  - [ ] 保留必要 GPIO/IMU/USB wake source。
  - [ ] 具体 power plan、IMU threshold/duration 和目标电流允许后续按实测调优，但接口和状态路径 v1 需要完整。
- [ ] 实现 wake 后外设恢复。
- [ ] C 不判断是否进入 `Suspend`/`Sleep`，只执行 Rust 的请求。

### 2.7 DataFlash

- [ ] 提供 config storage region。
- [ ] 提供 page erase / aligned write / read API。
- [ ] C 不解析配置内容。
- [ ] 处理写入期间中断和低功耗互斥。

---

## 3. Rust Core 任务

### 3.1 Runtime 总状态机

- [x] 建立统一 `Runtime`。
  - [ ] `InputRuntime`。
  - [ ] `MotionRuntime`。
  - [ ] `ReportRuntime`。
  - [x] `HidRouter`。
  - [x] `PowerManager`。
  - [x] `ConfigManager`。
  - [x] `WebHidRuntime`。
- [x] 现在建立 Rust `power` / `route` / `config` 模块骨架和基础类型；部分实现可先 stub，但模块边界和 public API 需与文档对齐。
- [ ] 定义事件队列与 latest-sample 缓存。
  - [ ] ISR 到 bottom-half 的非即时事件进入固定容量 SPSC event queue。
  - [ ] IMU 姿态使用 latest-sample cache，优先低延迟，允许丢弃旧样本。
  - [ ] 事件队列满时定义降级策略：可丢弃可合并事件，但不得丢失 button release 安全事件。
- [ ] 实现事件分发。
  - [ ] ISR callback 内更新短状态或入队。（目前多数 callback 只置 pending/dirty）
  - [ ] `vp_core_poll()` 处理 deferred work。（已有 pass budget 和部分 placeholder，业务 work 仍待补齐）
- [x] 实现 activity timestamp 更新。
- [x] 实现统一 dirty flag。
  - [x] input dirty。
  - [x] motion dirty。
  - [x] report dirty。
  - [x] power dirty。
  - [x] config dirty。

### 3.2 输入系统

- [x] 定义 `ButtonId`。（FFI 层 enum 已定义；Rust 侧业务 enum 仍可后续补充）
  - [x] Left。
  - [x] Right。
  - [x] Middle。
  - [x] Action。
  - [x] Laser。
- [ ] 定义 `SwitchId`。
  - [ ] ModeSwitch。
- [ ] 定义 debounce instance id。
  - [ ] 可区分普通 button 与 switch。
  - [ ] C 侧不解释 id 语义，只按 Rust 请求读取对应 GPIO。
- [ ] 定义 `InputSnapshot`。
  - [x] left/right/middle/action/laser。（旧 `InputStatus` 原型已覆盖）
  - [ ] mode。
  - [x] wheel pending。（旧 `InputStatus` 原型已覆盖）
- [ ] 定义 `InputEvent`。
  - [ ] ButtonPressed。
  - [ ] ButtonReleased。
  - [ ] Wheel。
  - [ ] ModeChanged。
- [ ] 实现 Rust 侧通用 debounce core。
  - [ ] 提供共享采样窗口、history/settle 计数、当前稳定值、候选值、debouncing 状态。
  - [ ] C 侧只负责 EXTI 唤醒、读取 GPIO、启动/停止 timer，不区分 button/switch 业务语义。
  - [ ] debounce core 通过 policy/child state machine 复用到普通按键和模式开关，避免重复实现。
- [ ] 实现 `ButtonDebouncePolicy`。
  - [ ] 面向普通瞬时按键：Left、Right、Middle、Action、Laser。
  - [ ] 连续低电平确认 pressed。
  - [ ] 连续高电平确认 released。
  - [ ] EXTI self-mask 由 Rust 调 C 完成。
  - [ ] 普通按键确认 released 后，Rust 调 C 清 pending 并重新开启 EXTI。
- [ ] 实现 `SwitchDebouncePolicy`。
  - [ ] 面向物理模式开关，复用 debounce core，但不复用 button pressed/released 语义。
  - [ ] switch EXTI 触发后 Rust 调 C 临时 mask switch EXTI。
  - [ ] 经过配置的稳定等待时间或连续稳定采样后确认最终档位。
  - [ ] 确认档位后发布 `ModeChanged` 事件。
  - [ ] switch 切换确认后一段时间即可清 pending 并重新开启 EXTI，不需要等待“released”。
  - [ ] 切换过程中的来回抖动只保留最终稳定档位。
- [ ] 实现共享 debounce tick 处理。
  - [ ] Rust 判断哪些 debounce instance 正在运行。
  - [ ] Rust 调 C 读取对应 GPIO level。
  - [ ] Rust 将采样值交给 debounce core，再由对应 policy 解释为 ButtonPressed/ButtonReleased/ModeChanged。
  - [ ] 无 active debounce instance 时停止 timer。
- [x] 实现 `RotaryEncoder`。（旧 input 原型中已实现，尚未接入新事件链路）
  - [ ] 开机读取初始 A/B 状态。
  - [x] old/new 4-bit lookup。
  - [x] `+1/-1/0` 微步。
  - [x] `internal_phase >= 4` 发布 wheel +1。
  - [x] `internal_phase <= -4` 发布 wheel -1。
  - [ ] 非法跳变计数。
- [ ] 实现输入事件到业务行为的映射。
  - [ ] Left/Right → HID buttons。
  - [ ] Middle → HID middle + motion trigger。
  - [ ] Action → motion trigger only。
  - [ ] Laser → GPIO output or configurable mapping。
  - [ ] Wheel → HID wheel。


### 3.3 Motion / 姿态映射

- [ ] 重构 `TiltMotionSolver`。（算法原型已存在；仍需去掉 `new()` 读硬件/unwrap，并接入新 Runtime）
  - [ ] `new()` 不读取硬件、不 unwrap。
  - [x] `calibrate(attitude)`。
  - [x] `update(attitude)`。
  - [ ] `reset_filter()`。
  - [ ] 支持 configurable axis mapping。
- [ ] 实现 `MotionSession`。
  - [ ] `Idle`。
  - [ ] `Arming { trigger }`。
  - [ ] `Active { trigger }`。
  - [ ] `BlockedByPolicy`，用于有线防误触等策略临时禁止 motion。
- [ ] 定义 `MotionTrigger`。
  - [ ] Action。
  - [ ] Middle。
- [ ] 实现 trigger priority。
  - [ ] Middle 优先。
  - [ ] Action 不进入 HID button。
  - [ ] Middle 同时保持 HID middle pressed。
- [ ] 实现 motion arming。
  - [ ] Trigger pressed 后请求 IMU active。
  - [ ] 下一帧有效 attitude 捕获为 center。
  - [ ] 进入 Active。
- [ ] 实现 motion stop。
  - [ ] Trigger released 后停止。
  - [ ] 清 filter。
  - [ ] 清 motion pending。
  - [ ] 松手即停，无惯性。
- [ ] 实现非线性曲线。（当前 `TiltMotionSolver` 已实现 Quadratic 默认；Linear/Cubic 等配置化仍待补）
  - [ ] Linear。
  - [x] Quadratic 默认。
  - [ ] Cubic。
  - [ ] Exponential 或 piecewise 预留。
- [ ] 实现姿态数据转换。（基础转换已实现；validity check 仍待补）
  - [x] SFLP raw half-float x/y/z → quaternion。
  - [x] 计算 w。
  - [x] quaternion → roll/pitch/yaw。
  - [ ] validity check。
- [ ] 实现 latest attitude cache。
  - [ ] active motion 使用最新 sample。
  - [ ] arming 时捕获最新 sample。
  - [ ] FIFO 多样本时优先低延迟，允许丢旧帧。

### 3.4 HID Report 聚合

- [x] 定义统一 mouse report。
  - [x] buttons。
  - [x] dx。
  - [x] dy。
  - [x] wheel。
- [ ] 实现 `ReportRuntime`。
  - [ ] pending dx/dy。
  - [ ] fractional accum x/y。
  - [ ] pending wheel。
  - [ ] current buttons。
  - [ ] last sent buttons。
  - [ ] dirty flag。
- [ ] 发送条件。
  - [ ] dx/dy pending 非零。
  - [ ] wheel pending 非零。
  - [ ] buttons 与 last sent 不同。
  - [ ] 需要安全 release frame。
  - [ ] route 恢复后同步状态。
- [ ] motion speed 积分。（旧 `ReportState` 原型已实现，尚未接入新 Runtime/ReportRuntime）
  - [x] 按 report_hz 积分。
  - [x] i8 clamp。
  - [x] 避免发送 -128，必要时 clamp 到 -127。
  - [x] 发送成功后 commit。
- [ ] HID send result 处理。
  - [ ] Sent：commit dx/dy/wheel/buttons。
  - [ ] RetryLater：保留 pending。
  - [ ] NotConnected：丢弃 motion pending，保留必要 release safety。
  - [ ] Fatal：route unhealthy，清 motion pending，尝试安全释放。

### 3.5 HID Route / 三模策略

- [x] 定义 `HidRoute`。
  - [x] BLE。
  - [x] Dongle2G4。
  - [x] USB。
  - [x] None。
- [x] 定义连接状态。
  - [x] BLE connected/disconnected。
  - [x] Dongle connected/disconnected。
  - [x] USB detached/attached/configured。
- [ ] 实现 route selection。
  - [ ] Mode switch = BLE → 优先 BLE。
  - [ ] Mode switch = 2.4G → 优先 Dongle。
  - [ ] USB 行为由配置决定。
- [ ] 实现 USB mouse policy。
  - [ ] 默认 `usb_mouse_policy = Disabled` 时 USB configured 禁用全部 mouse report。
  - [ ] `MotionDisabled` 时仅禁用 `dx/dy`，保留 buttons/wheel。
  - [ ] `Enabled` 时 mouse report 全部开启，并默认走 USB。
  - [ ] 允许 WebHID 配置。
- [ ] 实现 route ready 判断。
- [ ] 实现 route error recovery。

### 3.6 PowerManager

- [x] 定义电源状态。
  - [x] `Active`。
  - [x] `Suspend`。
  - [x] `Sleep`。
- [ ] 定义状态语义。
  - [ ] `Active`：全功能运行，输入/IMU/HID 活跃。
  - [ ] `Suspend`：有连接或需要快速恢复，RF 保持，IMU 低功耗，按键/编码器/IMU 可唤醒。
  - [ ] `Sleep`：无连接且静置，RF 关闭，IMU/GPIO 极低功耗唤醒。
- [ ] 实现进入 `Suspend` 条件。
  - [ ] USB 未处于 configured 状态。
  - [ ] 无按键按下。
  - [ ] 无 motion active。
  - [ ] 无 HID pending。
  - [ ] 无 DataFlash erase/write 进行中。
  - [ ] 有 BLE/2.4G 连接或需要保持快速恢复。
  - [ ] 静置超过 suspend timeout。
  - [ ] Vendor/config 会话不作为特殊 blocker。
- [ ] 实现进入 `Sleep` 条件。
  - [ ] USB detached。
  - [ ] 无 BLE/2.4G 连接，2.4G v1 stub 视为无连接。
  - [ ] 无按键按下。
  - [ ] 无 motion active。
  - [ ] 无 HID pending。
  - [ ] 无 DataFlash erase/write 进行中。
  - [ ] config 不 dirty；如 dirty，先保存，保存完成后再允许 Sleep。
  - [ ] 激光关闭；若仍开启，进入前关闭并记录诊断。
  - [ ] 从无线断连时刻计算，超过 `disconnect_sleep_timeout_ms`。
- [ ] 实现 wake 处理。
  - [ ] 按键 wake → Active。
  - [ ] 编码器 wake → Active。
  - [ ] IMU wake → 根据连接和运动策略进入 Active 或 Suspend。
  - [ ] USB wake/state change → 根据 `usb_mouse_policy` 和 USB state mapping 进入 Active/config。
- [ ] 实现 Rust 调 C 执行状态切换。
  - [ ] 切 IMU profile。
  - [ ] 开关 RF。
  - [ ] 配置 wake source。
  - [ ] 执行 sleep。

### 3.7 Config 系统

> 配置存储格式、双槽策略、字段建议与保存流程见 `../CONFIG_SPEC.md`。

- [ ] 定义 `DeviceConfig`。
  - [ ] motion config。
  - [ ] input config。
  - [ ] HID config。
  - [ ] power config。
  - [ ] route config。
  - [ ] WebHID/vendor config。
- [ ] 定义 motion config。
  - [ ] deadzone x/y。
  - [ ] max angle。
  - [ ] sensitivity x/y。
  - [ ] curve kind。
  - [ ] smoothing。
  - [ ] invert x/y。
  - [ ] swap xy。
  - [ ] axis mapping。
- [ ] 定义 input config。
  - [ ] debounce samples。
  - [ ] encoder detent steps。
  - [ ] wheel invert。
  - [ ] button remap。
  - [ ] laser behavior。
- [ ] 定义 HID config。
  - [ ] report_hz。
  - [ ] `usb_mouse_policy`：Disabled / MotionDisabled / Enabled。
  - [ ] BLE name。
  - [ ] report options。
- [ ] 定义 power config。
  - [ ] suspend timeout。
  - [ ] `disconnect_sleep_timeout_ms`。
  - [ ] IMU active/suspend/sleep profile selector。
- [ ] 实现配置序列化。
  - [ ] magic。
  - [ ] version。
  - [ ] length。
  - [ ] sequence。
  - [ ] CRC。
  - [ ] payload。
- [ ] 实现默认配置 fallback。
- [ ] 实现配置校验。
- [ ] 实现配置版本迁移预留。
- [ ] 实现 DataFlash 双槽或 sequence-based 保存策略。

### 3.8 WebHID / Vendor 命令

- [ ] 定义 vendor report 协议。
- [ ] 命令：get device info。
- [ ] 命令：get current config。
- [ ] 命令：set runtime config。
- [ ] 命令：save config。
- [ ] 命令：reset config。
- [ ] 命令：start calibration。
- [ ] 命令：get sensor status。
- [ ] 命令：get battery/power status。
- [ ] 命令：get firmware version。
- [ ] Rust 解析命令并生成 response。
- [ ] C 只收发 raw vendor report。
- [ ] 配置写入期间阻止进入 `Suspend`/`Sleep`；config dirty 未保存时允许 `Suspend` 但禁止 `Sleep`。

### 3.9 Logging / Diagnostics

- [ ] Rust panic 输出到 C debug print。
- [ ] 记录关键错误计数。
  - [ ] encoder invalid transition。
  - [ ] I2C timeout。
  - [ ] IMU sample dropped。
  - [ ] HID retry/fatal。
  - [ ] config CRC failure。
- [ ] 提供 WebHID debug status 查询。

---

## 4. 典型流程验收任务

### 4.1 滚轮流程

- [ ] 编码器 A/B 任意边沿唤醒 C。
- [ ] C 读取 A/B 并调用 Rust。
- [ ] Rust 查表解析微步。
- [ ] Rust 相位满 4 后生成 wheel event。
- [ ] Rust 标记 report dirty。
- [ ] bottom-half 发送 wheel report。
- [ ] 发送成功后 commit wheel。
- [ ] 边缘抖动不会误触发 wheel step。

### 4.2 Middle 空鼠流程

- [ ] Middle EXTI 唤醒 C。
- [ ] C 调 Rust button exti callback。
- [ ] Rust 关闭该按钮 EXTI。
- [ ] Rust 启动 debounce timer。
- [ ] 连续 8ms low 后 Rust 确认 MiddlePressed。
- [ ] Rust 设置 HID middle pressed。
- [ ] Rust motion session 进入 `Arming(Middle)`。
- [ ] Rust 请求 IMU active。
- [ ] 下一帧 attitude 捕获为 center。
- [ ] Middle 按住期间姿态偏移产生 dx/dy。
- [ ] Middle 松开后 Rust 停止 motion，清 pending，发送 middle release。
- [ ] 松开后光标立即停止。

### 4.3 Action 空鼠流程

- [ ] Action 稳定按下后只触发 motion，不进入 HID button。
- [ ] 按下瞬间捕获姿态基准。
- [ ] 按住期间角度偏移决定鼠标速度。
- [ ] 松开后 motion stop、filter reset、pending clear。
- [ ] 不产生惯性漂移。

### 4.4 普通按键流程

- [ ] Left/Right 经过 Rust debounce。
- [ ] pressed/released 都能独立触发 HID report。
- [ ] 即使 dx/dy 为 0，button change 也必须发送。
- [ ] HID retry 不导致主机卡键。

### 4.5 IMU 流程

- [ ] IMU INT 唤醒 C。
- [ ] C 调 Rust。
- [ ] Rust 判断需要读取 FIFO。
- [ ] Rust 请求 C async FIFO read。
- [ ] C I2C IRQ 完成读取。
- [ ] C 回调 Rust raw sample。
- [ ] Rust 转姿态并驱动 motion。
- [ ] FIFO 多样本时优先使用最新样本降低延迟。

### 4.6 电源流程

- [ ] `Active` 下正常响应输入、IMU、HID。
- [ ] 静置且有连接时进入 `Suspend`。
- [ ] `Suspend` 保持 RF 连接，按键/编码器/IMU 可快速唤醒。
- [ ] 无连接且静置时进入 `Sleep`。
- [ ] `Sleep` 关闭 RF，仅保留必要 wake source。
- [ ] 任意有效输入唤醒后回到 `Active`。
- [ ] 配置写入、HID pending、Laser on 时禁止进入低功耗。

### 4.7 三模/有线流程

- [ ] BLE mode switch 选择 BLE route。
- [ ] 2.4G mode switch 选择 dongle route。
- [ ] USB 插入后根据 `usb_mouse_policy` 决定 mouse report 行为。
- [ ] `usb_mouse_policy` 默认禁用 USB 插入时的 mouse report，防止插线误触。
- [ ] WebHID 配置通道在 USB 下可用。
- [ ] route disconnected 时不累计过期 motion。

---

## 5. 文件拆分建议

### 5.1 Rust

- [ ] `core/src/ffi/`：C ABI types、imports、exports。
- [ ] `core/src/runtime/`：总 Runtime 与事件分发。
- [ ] `core/src/input/`：debounce core、button policy、switch policy、encoder、snapshot、events。
- [ ] `core/src/attitude/`：SFLP raw 转姿态、latest cache。
- [ ] `core/src/motion/`：MotionSession、TiltMotionSolver、曲线、轴映射。
- [ ] `core/src/report/`：HID mouse report 聚合与 commit。
- [ ] `core/src/hid/`：route、sender trait、send status。
- [ ] `core/src/power/`：Active/Suspend/Sleep 策略。
- [ ] `core/src/config/`：配置结构、默认值、校验、序列化。
- [ ] `core/src/storage/`：DataFlash 配置布局策略。
- [ ] `core/src/webhid/`：vendor command parser/response。
- [ ] `core/src/diagnostics/`：错误计数、debug status。

### 5.2 C

- [ ] `platform/APP/main.c`：启动和主循环。
- [ ] `platform/HAL/hal_gpio.c`：GPIO/EXTI。
- [ ] `platform/HAL/hal_timer.c`：debounce timer、RTC time。
- [ ] `platform/HAL/hal_i2c.c`：I2C master async driver。
- [ ] `platform/APP/imu_lsm6dsv.c`：LSM6DSV profile、FIFO parse。
- [ ] `platform/APP/hid_route.c`：BLE/USB/2.4G HID send glue。
- [ ] `platform/APP/storage_flash.c`：DataFlash API。
- [ ] `platform/APP/power_platform.c`：Suspend/Sleep 执行。
- [ ] `platform/Bind/c_api.c`：Rust imports/exports glue，不写业务逻辑。

---

## 6. 完成标准

- [ ] C 层没有产品业务判断。
- [ ] Rust 层拥有唯一业务状态机。
- [ ] 所有按键稳定事件由 Rust debounce 后产生。
- [ ] 滚轮方向与步进完全由 Rust encoder state machine 产生。
- [ ] Action/Middle 空鼠逻辑完全由 Rust 控制。
- [ ] Middle 同时具备 HID middle button 和 motion trigger 语义。
- [ ] HID report 由 Rust 聚合，C 只发送。
- [ ] IMU 读取由 Rust 策略触发，C 只执行 I2C 和 FIFO raw sample 回调。
- [ ] `Active`/`Suspend`/`Sleep` 转换完全由 Rust 判断。
- [ ] 配置格式与 WebHID 命令完全由 Rust 管理。
- [ ] DataFlash 只作为 C 提供的块设备式 API 使用。
- [ ] USB 插入防误触策略由 Rust 中的 `usb_mouse_policy` 配置控制。
- [ ] 无 HID pending、无按键按下、无配置写入时才允许低功耗。
