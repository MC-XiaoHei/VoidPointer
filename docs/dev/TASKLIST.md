# VoidPointer 固件实现 TaskList

本文是实现阶段主入口，按顺序拆分基础设施、FFI、平台层、Rust Core、典型流程和完成标准。当前代码与目标规格差距也集中记录在本文，避免把“已设计”误认为“已实现”。

## Current implementation status

| 模块 | 当前代码状态 | v1 目标 / 主要差距 |
| --- | --- | --- |
| Rust lifecycle | 当前仍有 legacy `init_core()` / `tick()` | 一次性替换为 `vp_core_init()` / `vp_core_poll()`，不保留 legacy wrapper；迁移到 TMOS bottom-half 语义。 |
| Input | 已有 Rust input/encoder 原型 | 接入 EXTI callback、debounce timer callback、mode switch policy。 |
| Motion / Attitude | 已有 attitude/motion 模块和 LSM6DSV 原型 | 接入 IMU INT + async FIFO + latest sample cache。 |
| Report / HID | 已有 report/hid 模块和 BLE HID 原型 | 扩展为 route-aware HID send、retry、USB/2.4G stub。 |
| Route | 未见独立 Rust route 模块 | v1 BLE + USB 可用；2.4G stub。 |
| Power | 未见独立 Rust power 模块 | 实现 `Active` / `Suspend` / `Sleep` 状态机、timeout、blocker、C power API 接入。 |
| Config | 未见完整 DataFlash config 实现 | 建立 `config` 骨架，双槽读写、CRC、migration、runtime apply。 |
| Vendor/WebHID | 未见完整命令层 | 实现 vendor report protocol、命令解析、响应与保存流程。 |
| FFI ABI | `platform/Bind/c_api.h` 仍是 legacy API | 目标化 `c_api.h`、`cbindgen`、`bindgen`。 |
| Platform C | 有 CH585/LSM6DSV/BLE 原型 | 拆成底层 API，不持有业务状态。 |

---

## 0. 基础设施任务

- [ ] 实现 ISR-safe 与 bottom-half 执行规则。
  - [ ] ISR-safe Rust callback 只做短状态更新、事件入队、调用 C 的轻量 API。
  - [ ] HID retry、I2C 读取请求、DataFlash 写入、WebHID 解析、电源切换放到 `vp_core_poll()`。
  - [ ] ISR 或 Rust 快速回调中如产生 deferred work，Rust 通过 C API 请求 TMOS event 调度 `vp_core_poll()`。
- [ ] 接入 RTC 作为统一事件时间戳。
  - [ ] Rust 统一使用 C 提供的 RTC timestamp。
  - [ ] C→Rust 事件回调均携带 RTC timestamp。
  - [ ] debounce、activity timeout、Suspend/Sleep timeout 均基于 RTC timestamp 计算。
- [ ] 实现 TMOS bottom-half 调度。
  - [ ] C 注册专用 TMOS task/event 用于调用 `vp_core_poll()`。
  - [ ] Rust 暴露 deferred work 标志或通过 C API 请求触发该 TMOS event。
  - [ ] 多次请求可合并，避免重复排队。
  - [ ] `vp_core_poll()` 每次执行到无 pending work 或达到单次预算后返回。
  - [ ] 主循环只运行 `TMOS_SystemProcess()`/协议栈，不直接轮询业务状态。
- [ ] 实现全局 Runtime 访问保护。
  - [ ] 确定使用短临界区或 ISR event queue。
  - [ ] 禁止在长耗时操作期间持有 Runtime 可变借用。

---

## 1. FFI ABI 设计

> 具体 ABI 框架与演进规则见 `../FFI_ABI.md`。

### 1.1 C → Rust exported callbacks

- [ ] 生命周期入口。
  - [ ] `vp_core_init()`：Rust Runtime 初始化、配置加载、初始状态同步。
  - [ ] `vp_core_poll()`：Rust bottom-half，由 TMOS event 调度；处理 deferred work、HID retry、I2C 读取请求、配置任务、电源决策。
- [ ] 输入入口。
  - [ ] `vp_on_button_exti(button_id, level, timestamp)`。
  - [ ] `vp_on_debounce_tick(timestamp)`。
  - [ ] `vp_on_encoder_exti(a_level, b_level, timestamp)`。
  - [ ] `vp_on_mode_switch_exti(level, timestamp)`。
- [ ] IMU 入口。
  - [ ] `vp_on_imu_int(timestamp)`。
  - [ ] `vp_on_imu_sample(raw_x, raw_y, raw_z, timestamp)`。
  - [ ] 可选：`vp_on_imu_fifo_done(status, dropped_count, timestamp)`。
- [ ] HID / 连接入口。
  - [ ] `vp_on_ble_connected(timestamp)`。
  - [ ] `vp_on_ble_disconnected(timestamp)`。
  - [ ] `vp_on_dongle_connected(timestamp)`。
  - [ ] `vp_on_dongle_disconnected(timestamp)`。
  - [ ] `vp_on_usb_state_changed(state, timestamp)`。
  - [ ] `vp_on_hid_send_done(route, status, timestamp)`，如果发送采用异步完成模型。
- [ ] WebHID / Vendor 入口。
  - [ ] `vp_on_vendor_report_rx(route, ptr, len, timestamp)`。

### 1.2 Rust → C imported hardware APIs

- [ ] GPIO / EXTI API。
  - [ ] 读取单个 GPIO 输入。
  - [ ] 批量读取按键 GPIO。
  - [ ] 写 GPIO 输出，例如 Laser。
  - [ ] mask/unmask 指定 EXTI。
  - [ ] 清 EXTI pending。
  - [ ] 配置 EXTI 边沿。
- [ ] Timer / RTC API。
  - [ ] 启动/停止 1ms debounce timer。
  - [ ] 请求 TMOS event 调度 `vp_core_poll()`。
  - [ ] 读取 RTC tick。
  - [ ] 读取 RTC millis。
  - [ ] 读取 RTC micros，可选。
  - [ ] 配置 RTC wake。
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

- [ ] 保留现有 `bindgen` / `cbindgen` 自动生成机制。
- [ ] 一次性替换 legacy `init_core()` / `tick()`；v1 不保留 legacy wrapper 过渡。
- [ ] 将 Rust exported `init_core()` 替换为 `vp_core_init()`。
- [ ] 将 Rust exported `tick()` 替换为 `vp_core_poll()`，并改为 TMOS event bottom-half 语义。
- [ ] 将 C API 命名迁移到 `c_vp_*` 前缀。
- [ ] 用事件化输入 callback 替代主路径中的同步 `c_get_input_status()`。
- [ ] 用 GPIO/EXTI/Timer API 替代输入业务中的直接 GPIO 快照轮询。
- [ ] 用 IMU INT + async FIFO + `vp_on_imu_sample()` 替代主路径中的同步 `c_read_sflp_game_rotation_raw()`。
- [ ] 将 BLE-only `c_send_ble_hid_mouse_report()` 扩展或替换为 route-aware `c_vp_hid_send_mouse(route, ...)`。
- [ ] 保留 RTC API 能力，并按目标 API 命名提供 `c_vp_rtc_tick/millis/micros()`。
- [ ] 为每个新增 Rust→C API 标注 ISR-safe 或 bottom-half only。
- [ ] 为每个新增 C→Rust callback 确认 `cbindgen` 生成声明稳定可用。

---

## 2. C 平台层任务

### 2.1 启动与初始化

- [ ] 整理 `main.c` 启动流程。
  - [ ] 系统时钟初始化。
  - [ ] 低功耗安全 GPIO 默认态。
  - [ ] Debug UART 可选初始化。
  - [ ] RTC/timebase 初始化。
  - [ ] I2C 初始化。
  - [ ] IMU 基础通信初始化。
  - [ ] BLE/USB/2.4G/HID 栈初始化。
  - [ ] 调用 `vp_core_init()`。
  - [ ] 根据 Rust 返回/调用结果配置 EXTI、IMU profile、HID route。
- [ ] C 主循环只运行 `TMOS_SystemProcess()` 和协议栈调度，不做业务轮询。

### 2.2 GPIO / EXTI

- [ ] 为 Left/Right/Middle/Action/Laser/ModeSwitch 配置 GPIO 输入。
- [ ] 为编码器 A/B 配置任意边沿输入。
  - [ ] 如平台存在可靠 both-edge 能力则直接映射。
  - [ ] CH585 StdPeriph 未暴露普通 GPIO IRQ both-edge 时，平台层用“读当前电平后重配下一边沿”模拟。
  - [ ] Rust 编码器状态机用 old/new 4-bit lookup 兜底非法跳变；必要时增加短周期采样兜底。
- [ ] 为普通按键配置 EXTI wake。
- [ ] 为 ModeSwitch 配置 EXTI wake。
- [ ] 按键 EXTI ISR 中只做：识别 pin、读 level、拿 timestamp、调用 Rust。
- [ ] 编码器 EXTI ISR 中只做：读 A/B level、拿 timestamp、调用 Rust。
- [ ] 提供 Rust 可调用的 EXTI mask/unmask/clear/edge 配置 API。

### 2.3 Debounce Timer

- [ ] 实现共享 1ms debounce timer。
- [ ] Timer ISR 调用 `vp_on_debounce_tick(timestamp)`。
- [ ] 普通按键与 ModeSwitch 复用同一个 debounce tick，差异由 Rust policy 处理。
- [ ] Timer 是否继续运行由 Rust 通过 C API 控制。
- [ ] C 不维护“哪个输入正在消抖”的业务状态。

### 2.4 I2C / IMU

> LSM6DSV 寄存器、SFLP/FIFO 参数和 I2C 依据见 `../RESOURCE_PROFILE.md`。

- [ ] CH585 I2C master 配置。
  - [ ] SDA/SCL 使用 PCB 实际引脚，当前规划为 `PB12/PB13`。
  - [ ] 400 kHz。
  - [ ] 7-bit address。
  - [ ] ACK enable。
  - [ ] 支持 bus idle 检查。
- [ ] 实现 I2C bus recovery。
  - [ ] SCL pulse。
  - [ ] STOP condition。
  - [ ] I2C peripheral reset。
- [ ] 实现 LSM6DSV 基础寄存器访问。
  - [ ] read register。
  - [ ] write register。
  - [ ] burst read。
  - [ ] WHO_AM_I 检查。
- [ ] 实现 LSM6DSV 三个 profile。
  - [ ] `Active` profile：按 `../RESOURCE_PROFILE.md`，SFLP enabled，120 Hz 起步，FIFO continuous/latest-sample。
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
  - [ ] C 可解析 FIFO tag，筛选 SFLP game rotation raw sample。
  - [ ] C 将 raw half-float x/y/z 回调给 Rust。

### 2.5 HID / 连接栈

> BLE HID mouse report、USB vendor HID endpoint/report 依据见 `../RESOURCE_PROFILE.md`。

- [ ] 封装 BLE HID mouse report 发送。
- [ ] 实现 USB HID mouse report 发送。
- [ ] 实现 USB Vendor HID report 收发。
- [ ] 2.4G dongle report 发送保留 stub，返回 `Unsupported` 或 `NotConnected`，待 dongle 协议栈选型后补齐。
- [ ] 将底层发送结果映射为统一状态。
  - [ ] `Sent`。
  - [ ] `RetryLater`。
  - [ ] `NotConnected`。
  - [ ] `Fatal`。
- [ ] BLE connected/disconnected 事件回调 Rust。
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

- [ ] 建立统一 `Runtime`。
  - [ ] `InputRuntime`。
  - [ ] `MotionRuntime`。
  - [ ] `ReportRuntime`。
  - [ ] `HidRouter`。
  - [ ] `PowerManager`。
  - [ ] `ConfigManager`。
  - [ ] `WebHidRuntime`。
- [ ] 现在建立 Rust `power` / `route` / `config` 模块骨架和基础类型；部分实现可先 stub，但模块边界和 public API 需与文档对齐。
- [ ] 定义事件队列与 latest-sample 缓存。
  - [ ] ISR 到 bottom-half 的非即时事件进入固定容量 SPSC event queue。
  - [ ] IMU 姿态使用 latest-sample cache，优先低延迟，允许丢弃旧样本。
  - [ ] 事件队列满时定义降级策略：可丢弃可合并事件，但不得丢失 button release 安全事件。
- [ ] 实现事件分发。
  - [ ] ISR callback 内更新短状态或入队。
  - [ ] `vp_core_poll()` 处理 deferred work。
- [ ] 实现 activity timestamp 更新。
- [ ] 实现统一 dirty flag。
  - [ ] input dirty。
  - [ ] motion dirty。
  - [ ] report dirty。
  - [ ] power dirty。
  - [ ] config dirty。

### 3.2 输入系统

- [ ] 定义 `ButtonId`。
  - [ ] Left。
  - [ ] Right。
  - [ ] Middle。
  - [ ] Action。
  - [ ] Laser。
- [ ] 定义 `SwitchId`。
  - [ ] ModeSwitch。
- [ ] 定义 debounce instance id。
  - [ ] 可区分普通 button 与 switch。
  - [ ] C 侧不解释 id 语义，只按 Rust 请求读取对应 GPIO。
- [ ] 定义 `InputSnapshot`。
  - [ ] left/right/middle/action/laser。
  - [ ] mode。
  - [ ] wheel pending。
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
- [ ] 实现 `RotaryEncoder`。
  - [ ] 开机读取初始 A/B 状态。
  - [ ] old/new 4-bit lookup。
  - [ ] `+1/-1/0` 微步。
  - [ ] `internal_phase >= 4` 发布 wheel +1。
  - [ ] `internal_phase <= -4` 发布 wheel -1。
  - [ ] 非法跳变计数。
- [ ] 实现输入事件到业务行为的映射。
  - [ ] Left/Right → HID buttons。
  - [ ] Middle → HID middle + motion trigger。
  - [ ] Action → motion trigger only。
  - [ ] Laser → GPIO output or configurable mapping。
  - [ ] Wheel → HID wheel。

### 3.3 Motion / 姿态映射

- [ ] 重构 `TiltMotionSolver`。
  - [ ] `new()` 不读取硬件、不 unwrap。
  - [ ] `calibrate(attitude)`。
  - [ ] `update(attitude)`。
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
- [ ] 实现非线性曲线。
  - [ ] Linear。
  - [ ] Quadratic 默认。
  - [ ] Cubic。
  - [ ] Exponential 或 piecewise 预留。
- [ ] 实现姿态数据转换。
  - [ ] SFLP raw half-float x/y/z → quaternion。
  - [ ] 计算 w。
  - [ ] quaternion → roll/pitch/yaw。
  - [ ] validity check。
- [ ] 实现 latest attitude cache。
  - [ ] active motion 使用最新 sample。
  - [ ] arming 时捕获最新 sample。
  - [ ] FIFO 多样本时优先低延迟，允许丢旧帧。

### 3.4 HID Report 聚合

- [ ] 定义统一 mouse report。
  - [ ] buttons。
  - [ ] dx。
  - [ ] dy。
  - [ ] wheel。
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
- [ ] motion speed 积分。
  - [ ] 按 report_hz 积分。
  - [ ] i8 clamp。
  - [ ] 避免发送 -128，必要时 clamp 到 -127。
  - [ ] 发送成功后 commit。
- [ ] HID send result 处理。
  - [ ] Sent：commit dx/dy/wheel/buttons。
  - [ ] RetryLater：保留 pending。
  - [ ] NotConnected：丢弃 motion pending，保留必要 release safety。
  - [ ] Fatal：route unhealthy，清 motion pending，尝试安全释放。

### 3.5 HID Route / 三模策略

- [ ] 定义 `HidRoute`。
  - [ ] BLE。
  - [ ] Dongle2G4。
  - [ ] USB。
  - [ ] None。
- [ ] 定义连接状态。
  - [ ] BLE connected/disconnected。
  - [ ] Dongle connected/disconnected。
  - [ ] USB detached/attached/configured。
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

- [ ] 定义电源状态。
  - [ ] `Active`。
  - [ ] `Suspend`。
  - [ ] `Sleep`。
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
