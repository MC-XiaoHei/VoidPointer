# VoidPointer 固件实现 TaskList

本文只保留**当前实现状态、主线任务、暂缓项和验收口径**。
长期规格看 `docs/`；已拍板但尚未完全沉淀的结论看 `DECISIONS.md`；真正需要负责人拍板的问题看 `OPEN_QUESTIONS.md`。

---

## 1. 当前状态总览

| 模块 | 状态 | 说明 |
| --- | --- | --- |
| Runtime / FFI | Done | `vp_core_init/poll`、RuntimeCommand 边界、事件队列、bindgen/cbindgen 全链路已接通；长耗时已通过 command→execute→apply 模式隔离；Motion 相关 5 个离散字段已合并为 `MotionSession`。 |
| Input | Partial | 按键 EXTI + debounce、编码器 EXTI + wheel 已跑通；Mode switch 当前板无硬件，暂不实现。 |
| IMU / I2C | Partial | CH585 I2C + LSM6DSV 基础通信、WHO_AM_I、active/suspend/sleep profile、通用 I2C API、bus idle / recovery、异步 FIFO 读取主链路已完成；profile API 整理与完整低功耗闭环仍需继续收敛。 |
| Motion / Attitude | Done | validity check、MotionSession、config 参数配置化、Middle/Action 触发策略均已实现 |
| HID / Report | Partial | BLE/USB mouse report 已接入，wheel/buttons/motion 已能出报告；USB vendor 收发已接通；mouse 发送条件已收敛为 motion/wheel/button/retry/dirty 五类触发，并已抽出最小 `MouseReportRuntime`；统一 `ReportRuntime` 结构和无线 vendor backend 仍待补齐；当前 route 不可用时会丢弃未发送的 motion/wheel 暂存，并重置 button sync 基线，避免恢复后回放旧输入或沿用过期按钮发送状态。 |
| Route | Partial | 当前策略为“USB configured 优先且独占，否则无线固定 BLE”；2.4G 仍为 stub。 |
| Power | Partial | Rust `PowerManager` 已接通，Suspend 最小闭环已进入实现；button/encoder/IMU 的 Suspend wake source 已接通最小实现，并可在 wake 后恢复 Active IMU profile；`Sleep` 的 `prepare/enter/restore` 已补成项目级最小闭环，但尚未映射到真实 deep low-power；当前 `Sleep restore` 已避免在 `USB configured` 时误重新打开 BLE advertising，并且 runtime 已把 requeue 中的 vendor 待发包视为低功耗 blocker；恢复回 `Active` 时还会清掉旧 attitude / IMU sample / motion cache，重置 report 累积状态，并清理旧 wheel 暂存 / button sync 基线，避免 wake 后误用陈旧姿态、残余累计量或过期 mouse transport 状态；待确认重点仍是 **BLE connected 下能否映射到真实平台浅低功耗且不断链**，以及更完整的 deep low-power enter/restore。 |
| Config | Done | 双槽 DataFlash 持久化、SlotHeader + CRC32 + postcard 序列化、全链路校验、写入状态机、WebHID 配置命令、主机测试框架与 100% 纯逻辑覆盖率。 |
| Vendor / WebHID | Partial | 单包 RX queue、协议解析、基础查询命令、配置读写会话已接通；多包分片和完整 transport backend 未完成。 |
| 2.4G | Not started | 目前仅保留 route / HID stub。 |

---

## 2. 当前主线任务

只保留近期真正要推进的事项。

### P0：先把文档与代码状态对齐
- [x] 清理 `TASKLIST.md` 中过时或重复描述。
- [x] 清理旧 `tick()` 残留，避免 Runtime 双语义并存。
- [x] 为 Rust→C API 明确标注 `ISR-safe` 或 `bottom-half only` 约束。

### P1：补齐 IMU / I2C 的缺口
- [x] 实现通用 `c_vp_i2c_init()` API，避免文档与实际初始化路径分裂。
- [x] 实现 `c_vp_i2c_recover_bus()`。
- [x] 增加 bus idle 检查。
- [x] 实现 `LSM6DSV` suspend profile。
- [x] 实现 `LSM6DSV` sleep profile。
- [x] 完成 IMU wake source 接入，并收敛为“INT 只负责唤醒、FIFO 读取由 Rust bottom-half 发起”的模型。

### P2：收敛 Report / Route / 2.4G stub 边界
- [x] 明确并统一 2.4G stub 的 `route ready` / `send` / `vendor` 返回语义。
- [x] 收敛 `ReportState` 与发送条件，形成更清晰的统一 report runtime。
- [x] 明确 HID send result 在 `Sent / RetryLater / NotConnected / Fatal` 下的收敛行为。

### P3：推进 Power 平台接口
- [x] 补齐 `prepare/enter suspend` 平台 API。
- [x] 补齐 `prepare/enter sleep` 平台 API。
- [ ] 评估并接入更真实的 `Suspend` deep low-power entry（如 `LowPower_Halt_WFE()`）以及对应恢复约束。
  - 当前审计结论：暂不直接启用 `LowPower_Halt_WFE()`；首先要证明 **BLE connected 场景下进入该低功耗后仍能保持连接**。USBHS halt 后恢复、TMOS/runtime deep-sleep 恢复边界随后再补齐。
- [x] 补齐 `Suspend` 最小 wake source 配置，并在 resume 后恢复 Active IMU profile。
- [ ] 将更完整的 power profile / RF / route 恢复真正连上。
  - 当前已补齐五条确定性收口：`Sleep restore` 不会在 `USB configured` 时误开 BLE advertising；requeue 中的 vendor 待发包会阻止进入低功耗；恢复回 `Active` 时会清掉旧 attitude / IMU sample / motion cache；同时会重置 report 累积状态，避免 wake 后误用陈旧姿态或残余累计量；并会清理旧 wheel 暂存 / button sync 基线，避免 wake 或 route 恢复后回放过期 mouse transport 状态。其余恢复联动继续按审计推进。

### P4：Config 持久化
- [x] 定义 `DeviceConfig`。
- [x] 定义默认配置和基础校验。
- [x] 提供 DataFlash read / erase / aligned write API。
- [x] 实现双槽保存流程。
- [x] postcard 序列化 + CRC32 校验。
- [x] WebHID 配置命令：GetConfigInfo / ReadConfig / WriteConfig / SaveConfig / RestoreDefaults。
- [x] 主机测试框架，纯逻辑 100% 覆盖率。

---

## 3. 暂缓项 / 非当前板能力

这些不是当前主线开发任务。

- [ ] **Mode switch**：当前板无物理硬件，暂不接入。
  - 当前代码策略：无线 route 暂时固定 BLE。
  - 后续如果新板加入 mode switch，再恢复“BLE / 2.4G 由 mode switch 或 policy 决定”。
- [ ] **2.4G 真正发送实现**：待协议栈/方案启动后推进。
- [ ] **完整 sleep 电流优化**：接口与状态路径先落地，电流目标与阈值调优后续进行。

---

## 4. 分系统状态与差距

## 4.1 Runtime / FFI

### 已完成
- [x] 主入口切到 `vp_core_init()` / `vp_core_poll()`。
- [x] C 注册专用 TMOS task/event 调度 `vp_core_poll()`。
- [x] 即时 poll 与 delayed poll 都走 TMOS event。
- [x] fallback polling bridge 已移除。
- [x] 基础 ISR event queue 已建立。
- [x] `HID retry`、`Power state request`、`IMU FIFO read request` 已建立 RuntimeCommand 边界。
- [x] `bindgen` / `cbindgen` 生成链保持可用。
- [x] `c_vp_*` 前缀迁移已完成。
- [x] 旧 `tick()` 残留已清理。
- [x] Rust→C API 调用上下文已在 `c_api.h` 标注。
- [x] 继续拆分长耗时路径，避免持有 Runtime 可变借用跨越慢调用。
- [x] 将“当前事实”更多沉淀到长期文档，减少 TaskList 负担。

## 4.2 Input

### 已完成
- [x] 按键 low-active EXTI + debounce 主链路。
- [x] 编码器 EXTI + Rust lookup + wheel 事件主链路。
- [x] `vp_on_button_exti()` / `vp_on_debounce_tick()` / `vp_on_encoder_exti()` ABI 与回调路径。
- [x] EXTI mask / unmask / clear pending / edge API。
- [x] 共享 debounce timer 启停控制。
- [x] `InputSnapshot` 已覆盖 left/right/middle/action/laser/wheel。
- [x] 输入事件到业务行为的文档化与代码收敛仍需继续整理。——边界清晰，无结构性缺口。
- [x] 将 debounce core 明确整理为“可复用核心 + button policy”结构。——结构已完备，`DebouncedTwoStateInput` 为通用核心，`InputManager` 承载 policy。

### 未完成
- [ ] `Laser` 输出语义与输入语义进一步理顺。——Laser 按键事件当前在 Rust 侧被丢弃（`button_id_to_input_id` 未映射），需要接入 debounce 管线 + `pwm::set_laser_duty()` 控制。

### 暂缓
- [ ] `ModeSwitch` 输入链路：当前板无硬件，不做。

## 4.3 IMU / I2C

### 已完成
- [x] CH585 I2C master 硬件初始化。
- [x] SDA/SCL 实际映射接入（当前 `PB20/PB21`）。
- [x] 400 kHz。
- [x] 7-bit address。
- [x] ACK enable。
- [x] LSM6DSV 基础寄存器访问：read / write / burst read。
- [x] `WHO_AM_I` 检查与地址探测（`0x6A / 0x6B`）。
- [x] Active profile 工作实现。
- [x] `LSM6DSV_ConfigSuspend()`。
- [x] `LSM6DSV_ConfigSleep()`。
- [x] 通用 `c_vp_i2c_init()` API。
- [x] 通用 `c_vp_i2c_recover_bus()` API。
- [x] bus idle 检查。
- [x] I2C bus recovery：SCL pulse / STOP condition / peripheral reset。
- [x] `vp_on_imu_int()` / `vp_on_imu_sample()` / `vp_on_imu_fifo_done()` 事件链。
- [x] Rust 请求异步 FIFO 读取。
- [x] I2C IRQ 驱动 FIFO 读取状态机。
- [x] FIFO tag 解析并筛选 `SFLP game rotation raw`。
- [x] latest sample cache 与姿态更新主链路。
- [x] IMU wake source 已接入，并对齐为“INT 只负责唤醒”的语义。

### 未完成
- [ ] 将当前 active/suspend/sleep profile 整理成更稳定的 profile API / table。

## 4.4 Motion / Attitude

### 已完成
- [x] `SFLP raw half-float x/y/z` → quaternion → roll/pitch/yaw 基础转换。
- [x] `TiltMotionSolver::new()` / `calibrate()` / `update()` 基础能力。
- [x] active 态 latest sample 驱动 motion 主链路。
- [x] motion report 按固定节拍聚合并输出。
- [x] Quadratic 默认曲线已可用。
- [x] **validity check**：`AttitudeData::is_valid()` 检查 NaN/Inf/模长异常，在 session 和 runtime 两级生效。
- [x] **MotionSession**：抽取 Idle→Calibrating→Active 状态机，封装触发检测、校零、样本去重和 validity check。
- [x] **motion 参数配置化**：`Runtime::new()` 从 `ConfigManager` 加载配置；WebHID 改配置后自动 `sync_motion_config()`。
- [x] **Middle/Action 触发策略**：`MotionConfig.middle_triggers_motion` 开关，默认 `true`，可配为 `false`。
- [x] 总体状态已从 `Partial` 升级为 `Done`。

## 4.5 HID / Report / Route

### 已完成
- [x] 统一 mouse report 结构：buttons / dx / dy / wheel。
- [x] BLE mouse report 发送。
- [x] USB mouse report 发送。
- [x] USB vendor report 收发。
- [x] BLE connected / disconnected 回调 Rust。
- [x] USB state changed 回调 Rust。
- [x] `HidRoute` / `UsbState` / route ready 基础判断。
- [x] 当前最小路由策略：USB configured 优先且独占，否则无线固定 BLE。
- [x] USB configured 时关闭 BLE 广播并断开现有 BLE 连接。
- [x] 2.4G stub 已统一为 `route ready = false`，mouse/vendor send 返回 `NotConnected`。
- [x] route not-ready 时 mouse/vendor 发送都收敛本次尝试，等待 route 事件再次唤醒。
- [x] route 不可用时丢弃未发送的 motion 累积，避免恢复后回放旧移动。
- [x] route 不可用或 wake 恢复时清理旧 wheel 暂存，并重置 button sync 基线，避免恢复后回放旧滚轮或沿用断连前按钮发送状态。
- [x] mouse 发送条件已收敛为 motion delta、wheel、button 变化、retry、report dirty 五类触发。
- [x] `Sent / RetryLater / NotConnected / Fatal` 已有明确收敛规则并在 runtime 中显式处理。
- [x] 已抽出最小 `MouseReportRuntime`，承接 wheel/button/send-decision/send-commit 逻辑。

### 未完成
- [ ] 更独立、清晰的 `ReportRuntime` 结构。
- [ ] 异步 `hid_send_done` 模型真正接入（如果后续发送模型需要）。
- [ ] 2.4G route ready / mouse send / vendor send 真实实现。
- [ ] BLE / 2.4G vendor transport backend 的真实发送实现。
- [ ] route error recovery。
- [ ] `usb_mouse_policy` 配置化。

## 4.6 Power

### 已完成
- [x] Rust `PowerManager` 基础骨架。
- [x] `Active / Suspend / Sleep` 状态类型。
- [x] Power state request 已走 RuntimeCommand 边界。
- [x] Suspend 最小闭环已接通：Rust 可评估并进入 `Suspend`，平台 `prepare/enter suspend` 已有最小实现。
- [x] `Suspend` 最小 wake source 已接通：button / encoder / IMU 已通过平台 EXTI 配置接入，wake 后会恢复 Active IMU profile。

### 未完成
- [x] 平台 `prepare/enter/restore sleep` 项目级最小路径。
- [ ] `Suspend` 的真实 deep low-power entry 及其 BLE/USB/runtime 恢复约束。
  - 当前阻塞点：BLE library 只通过 `cfg.idleCB = CH58x_LowPower` 接入自身低功耗；`ble_gap_policy` 当前只维护连接句柄、广播和断连事件，没有项目级 suspend retention contract；因此在未证明 connected 下不断链之前，不宜把 `Suspend` 直接映射到 `LowPower_Halt_WFE()`。
- [ ] wake 后更完整的外设恢复。
- [ ] 低功耗与 IMU profile、RF、route 状态联动。

## 4.7 Config / Storage

### 已完成
- [x] `DeviceConfig` 结构（power / motion / report 子配置）。
- [x] 默认配置与业务校验。
- [x] 双槽 DataFlash 存储（SlotHeader + payload + CRC32）。
- [x] postcard 序列化 + serde。
- [x] 全链路有效性验证（magic → version → CRC → 反序列化 → 业务校验）。
- [x] 写入状态机（begin / chunk / commit / abort）。
- [x] WebHID 配置命令：GetConfigInfo / ReadConfig / WriteConfig / SaveConfig / RestoreDefaults。
- [x] 自动生成 C 函数 stubs 的测试框架。
- [x] 纯逻辑模块 100% 覆盖率。

### 未完成
- [ ] 版本迁移（migration）。
- [ ] 配置写入运行时 apply（apply_to runtime 子系统）。

## 4.8 Vendor / WebHID

### 已完成
- [x] `VendorRuntime` RX queue。
- [x] 单包协议解析骨架。
- [x] 基础命令：`Ping` / `GetProtocolInfo` / `GetDeviceInfo` / `GetConfigInfo` / `GetRouteState` / `GetPowerState` / `GetDiagnostics`。
- [x] 配置读写命令：`ReadConfig` / `WriteConfigBegin` / `WriteConfigChunk` / `WriteConfigCommit` / `WriteConfigAbort` / `SaveConfig` / `RestoreDefaults`。
- [x] 配置写会话状态机。
- [x] C 层仅收发 raw vendor report。

### 未完成
- [ ] 多包分片。
- [ ] BLE Custom GATT / USB Custom HID transport backend 的进一步完善。

---

## 5. 典型流程验收口径

只保留当前仍有判断价值的验收路径。

- [ ] **滚轮流程**：编码器边沿 → Rust 解码 → wheel event → report send → commit 正常，抖动不误触发。
- [ ] **普通按键流程**：Left/Right/Middle/Action 经 Rust debounce 后稳定地产生 pressed/released，button change 即使 `dx/dy=0` 也会发出。
- [ ] **mouse report 收敛流程**：仅在 motion delta、wheel、button 变化、retry 或 report dirty 存在时尝试发送；否则不重发空 report。
- [ ] **HID send result 收敛流程**：`Sent` 提交 pending，`RetryLater` 短退避重试，`NotConnected` 等 route 事件，`Fatal` 不做定时重试。
- [ ] **Middle / Action motion 流程**：trigger → IMU sample → attitude → motion dx/dy → release stop 无惯性。
- [ ] **IMU 流程**：IMU INT → Rust bottom-half 决策 → async FIFO read → sample callback → attitude update → motion。
- [ ] **有线/无线流程**：USB configured 时只走 USB；USB 退出后无线恢复 BLE；route 不可用时不累计失效 motion，也不靠定时自旋重试 vendor 发送。
- [ ] **低功耗流程**：在补齐平台 API 后验证 `Active / Suspend / Sleep / wake` 闭环。

---

## 6. 完成标准

- [ ] C 层不持有产品业务状态，只提供平台 API / glue / 中断转发。
- [ ] Rust 层拥有唯一业务状态机。
- [ ] 输入稳定事件由 Rust debounce 后产生。
- [ ] 滚轮步进完全由 Rust encoder 状态机产生。
- [ ] HID report 由 Rust 聚合，C 只发送。
- [ ] IMU 读取由 Rust 策略触发，C 只执行 I2C / FIFO / raw sample 回调。
- [ ] Route / Power / Config 决策由 Rust 控制。
- [ ] DataFlash 只作为 C 提供的块设备式 API 使用。
- [ ] 当前板级约束（无 mode switch、无 2.4G 栈）在代码和文档中都有明确说明。
