# VoidPointer Decisions

本文集中记录已经由项目负责人确认的产品策略、架构策略和 v1 实现策略。需要项目负责人继续拍板的问题只放在 `OPEN_QUESTIONS.md`；硬件资料待验证项放在 `../CH585_NOTES.md`。

## 1. 文档与实现边界

- 所有长期设计文档集中放在 `docs/`，开发期文档放在 `docs/dev/`。
- 文档优先；除非明确要求，不开始写代码。
- 当前未要求实现代码，后续实现按 `TASKLIST.md` 执行。
- `TASKLIST.md` 是实现顺序和任务拆分的主入口。
- `../TEST_PLAN.md` 是验收主入口。
- `../CH585_NOTES.md` 只记录硬件依据、TBD 和实测验证项；不要用猜测覆盖已有设计结论。

## 2. v1 实现范围

- v1 route：BLE + USB 进入可用状态。
- v1 2.4G：保留 `Dongle2G4` route 类型、状态、API 入口和错误映射；因为 CH585 2.4G dongle 协议栈暂无确定方案，真实链路 v1 不要求可用。
- v1 power：`Active` / `Suspend` / `Sleep` 三态都实现。
- `Sleep` 不是整体 stub，但具体 power plan、IMU threshold/duration 和目标电流允许后续实测调优。
- v1 不需要独立 shipping/storage mode，只做普通 `Sleep`。
- 目标 ABI 一次性替换 legacy `init_core()` / `tick()`；不保留 legacy wrapper。
- 现在建立 Rust `power` / `route` / `config` 模块骨架和基础类型；部分功能可先 stub。

## 3. Power decisions

- 电源状态保留三态：`Active` / `Suspend` / `Sleep`。
- 这不是三级睡眠，而是一个工作态 + 两级低功耗态。
- `Suspend` 用于有连接静置且保持无线连接。
- `Sleep` 用于无连接静置且 RF 关闭。
- USB configured 时保持 `Active`，不进入项目级 `Suspend` / `Sleep`。
- USB attached/configured 时禁止进入 `Sleep`；只有 USB detached 时才允许。
- `suspend_timeout_ms` 和 `disconnect_sleep_timeout_ms` 是两个独立门控。
- `disconnect_sleep_timeout_ms` 默认 `60000`。
- `Suspend` 中无线断开后，从断连时刻重新计算 `disconnect_sleep_timeout_ms`。
- `Suspend` 阶段 BLE 可以 advertising，2.4G 可以搜索/配对；进入 `Sleep` 后停止 advertising / 搜索。
- 不把 `WakeEvaluate` 作为正式 `PowerState`；Sleep wake 后先恢复到 `Active`，再由 `vp_core_poll()` 重新评估。
- 配置 dirty 时不能进入 `Sleep`；必须先保存，保存完成后才允许。
- Vendor/config 会话不作为特殊 power blocker；允许按普通 idle 规则进入 `Suspend`。
- Laser 如果在应进入低功耗时仍开启，视作异常/硬件 bug；进入低功耗前关闭 Laser。
- v1 功耗目标电流先标为 `TBD / measurement required`，等待板级实测后确定。

## 4. Route / USB decisions

- USB 插入时的 mouse 行为使用 `usb_mouse_policy`，废弃 `wired_behavior`。
- `usb_mouse_policy` 默认 `Disabled`。
- `usb_mouse_policy` 支持：`Disabled`、`MotionDisabled`、`Enabled`。
- `usb_mouse_policy = Enabled` 时 mouse report 默认走 USB。
- 项目尚未进入正式配置发布阶段，不需要考虑旧 `wired_behavior` 配置迁移。
- Vendor/WebHID 支持 USB、BLE、2.4G 三种 route。
- 多个 Vendor/WebHID route 同时可用时，优先级为 USB > 当前物理模式开关对应无线 route > 另一个无线 route。
- USB 配置通道使用 USBHS 口；USBHS Vendor HID 物理包按 512 bytes，USB FS fallback 64 bytes，BLE/2.4G 按各自 route MTU 分片。
- `vp_usb_state_t` 固定为 `Detached` / `Attached` / `Configured` / `Suspended` / `Error`。
- v1 USB state mapping：
  - `LINK_RDY` / ready status 映射为 `Attached` 候选。
  - `USB_SET_CONFIGURATION` 映射 `Configured`。
  - USBHS suspend 映射 `Suspended` / resume 输入。
  - `BUS_RST` 只清 configured 并回 `Attached` / reinit endpoints，不直接当作 physical detach。
  - `Detached` 只在启动无 link-ready 或平台有明确 VBUS/link-lost/board-level detach 信号时上报。

## 5. Input / GPIO decisions

- `vp_button_id_t` 固定为 `Left` / `Right` / `Middle` / `Action` / `Laser`。
- `vp_input_id_t` 只定义 button / mode / encoder / IMU input；USB 状态通过 USB stack callback，不作为 GPIO input。
- `vp_input_id_t` 预留 `ImuInt1` / `ImuInt2` 两个输入，实际是否都连接由 PCB 决定。
- `vp_output_id_t` 当前只定义 `Laser`；充电灯目前不走 MCU。
- `vp_exti_edge_t` FFI 只暴露 `Rising` / `Falling` / `Both`。
- CH585 WCH GPIO interrupt API 原生提供 low-level/high-level/fall-edge/rise-edge，未直接提供普通 GPIO IRQ both-edge；`Both` 由平台层模拟或按具体输入验证。
- 编码器 A/B 按“任意边沿输入”实现：如有可靠 both-edge 能力则直接映射；否则在平台层通过读当前电平后重配下一边沿模拟，并由 Rust 正交状态机兜底非法跳变，必要时增加短周期采样兜底。

## 6. FFI / ABI decisions

- `vp_timestamp_t` 使用 `uint32_t` RTC millis，按 wrapping time 处理回绕。
- `vp_core_poll()` 严格不可重入。
- 不要求 C 层额外处理 Rust callback 嵌套；设计上 callback 之间不共享业务状态。如有共享底层结构，该结构必须自身 ISR-safe。
- FFI length 使用混合固定宽度：HID/vendor report 用 `uint16_t`，flash/config buffer 用 `uint32_t`。
- v1 不需要 ABI version 校验，因为 C/Rust FFI 自动生成且一起编译。
- HID send status 保留独立 `vp_hid_send_status_t`。
- `vp_wake_source_t` 只做 debug/diagnostics，不参与业务决策。
- `vp_power_state_t` 不跨 FFI 暴露；power state 只在 Rust 内部。
- 不使用独立 `vp_flash_status_t`；DataFlash / I2C / IMU 等底层 API 统一返回 `vp_status_t`。

## 7. Vendor/WebHID decisions

- Vendor/WebHID frame header 保持完整，优先可调试性。
- Vendor command id 使用 `u16`。
- Vendor response status 使用独立枚举，不复用 `vp_status_t`。
- Vendor command id 按功能范围分段：
  - `0x0000..0x00FF`：基础协议 / device info / ping。
  - `0x0100..0x01FF`：Config 读写。
  - `0x0200..0x02FF`：Runtime apply / route / power。
  - `0x0300..0x03FF`：Diagnostics。
  - `0x8000..0xFFFF`：Vendor experimental / debug。
- Vendor frame header 中 `offset` 使用 byte offset。
- Vendor protocol v1 不支持乱序分片；必须顺序发送。header 保留 byte offset 语义，为后续乱序/重传预留。
- Vendor/WebHID CRC16 只在多包分片时启用。

## 8. IMU / profile decisions

- LSM6DSV v1 profile：Active profile 先固定默认起点。
- Suspend / Sleep profile 标为 tuning required，后续按误触发、唤醒延迟和功耗实测调优。
- Suspend/Sleep 默认唤醒不依赖 SFLP 角度检测，而使用 accelerometer-based wake-up / activity-inactivity / significant-motion 等低功耗中断。
