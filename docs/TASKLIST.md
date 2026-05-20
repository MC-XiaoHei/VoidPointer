# 固件实现 TaskList

各子系统实现状态。长期规格见 `docs/`。

## 子系统实现状态

### Runtime / FFI — ✅ 已完成

- [x] 主入口 `vp_core_init()` / `vp_core_poll()`
- [x] C 注册 TMOS task/event 调度 `vp_core_poll()`
- [x] 即时 poll 与 delayed poll 都走 TMOS event；fallback polling bridge 已移除
- [x] 基础 ISR event queue
- [x] HID retry / Power state request / IMU FIFO read 已建立 RuntimeCommand 边界
- [x] `bindgen` / `cbindgen` 生成链可用
- [x] `c_vp_*` 前缀迁移完成；旧 `tick()` 已清理
- [x] Rust→C API 调用上下文在 `c_api.h` 标注（ISR-safe / bottom-half only）
- [x] 长耗时路径已拆分，避免持有 Runtime 可变借用跨越慢调用

### Input — ✅ 已完成

- [x] 按键 low-active EXTI + debounce 主链路
- [x] 编码器 EXTI + Rust lookup + wheel 事件主链路
- [x] `vp_on_button_exti()` / `vp_on_debounce_tick()` / `vp_on_encoder_exti()` ABI
- [x] EXTI mask / unmask / clear pending / edge API
- [x] 共享 debounce timer 启停控制
- [x] `InputSnapshot` 覆盖 left/right/middle/action/laser/wheel
- [x] `DebouncedTwoStateInput` 可复用核心 + `InputManager` policy
- [x] ModeSwitch：PB11 物理引脚已定义，EXTI 全链路已通，触发 `ModeSwitchExti` 事件并播放 `MODE_BLE`/`MODE_2G4` LED 动画
- [x] Laser 按键接入：加入 debounce 管线，`input.laser` 稳定后在 `poll_input_and_hid` 中调用 `pwm::set_laser_duty()`

### IMU / I2C — 🟡 进行中

**已完成**

- [x] CH585 I2C master 硬件初始化（400 kHz、7-bit address、ACK enable）
- [x] SDA/SCL 映射（PB20/PB21）
- [x] LSM6DSV 基础寄存器访问：read / write / burst read、WHO_AM_I
- [x] Active profile 工作实现
- [x] `LSM6DSV_ConfigSuspend()` / `LSM6DSV_ConfigSleep()`
- [x] 通用 `c_vp_i2c_init()` / `c_vp_i2c_recover_bus()` API
- [x] bus idle 检查；I2C bus recovery（SCL pulse / STOP / peripheral reset）
- [x] `vp_on_imu_int()` / `vp_on_imu_sample()` / `vp_on_imu_fifo_done()` 事件链
- [x] Rust 请求异步 FIFO 读取；I2C IRQ 驱动 FIFO 读取状态机
- [x] FIFO tag 解析 + `SFLP game rotation raw` 筛选
- [x] latest sample cache + 姿态更新主链路
- [x] IMU wake source（INT 只负责唤醒，FIFO 读取由 Rust bottom-half 发起）

**未完成**

- [ ] 将 active/suspend/sleep profile 整理成稳定的 Rust 端 profile API/table

### Motion / Attitude — ✅ 已完成

- [x] `SFLP raw half-float` → quaternion → roll/pitch/yaw
- [x] `TiltMotionSolver::new()` / `calibrate()` / `update()`
- [x] Active 态 latest sample 驱动 motion；按固定节拍聚合输出
- [x] Quadratic 默认曲线
- [x] validity check：`AttitudeData::is_valid()` 检查 NaN/Inf/模长
- [x] MotionSession：Idle→Calibrating→Active 状态机，封装触发检测、校零、样本去重和 validity check
- [x] 参数配置化：`Runtime::new()` 从 ConfigManager 加载；WebHID 改配置后自动 `sync_motion_config()`
- [x] Middle/Action 触发策略：`MotionConfig.middle_triggers_motion` 开关（默认 true）

### HID / Report / Route — 🟡 进行中

**已完成**

- [x] 统一 mouse report 结构：buttons / dx / dy / wheel
- [x] BLE / USB mouse report 发送
- [x] USB vendor report 收发
- [x] BLE connected / disconnected 回调 Rust
- [x] USB state changed 回调 Rust
- [x] `HidRoute` / `UsbState` / route ready 基础判断
- [x] 路由策略：USB configured 优先且独占，否则无线固定 BLE
- [x] USB configured 时关闭 BLE 广播并断开现有连接
- [x] 2.4G stub 统一返回 `route ready = false`，mouse/vendor send 返回 `NotConnected`
- [x] route not-ready 时收敛本次发送，等 route 事件唤醒
- [x] route 不可用时丢弃未发送 motion 累积，清理 wheel 暂存，重置 button sync 基线
- [x] mouse 发送条件：motion delta / wheel / button 变化 / retry / report dirty
- [x] `Sent / RetryLater / NotConnected / Fatal` 收敛规则并在 runtime 显式处理
- [x] 统一 `ReportRuntime`：合并 `MouseReportRuntime` 与 `ReportState`，封装 motion/wheel/button 集成接口

**未完成**

- [ ] route error recovery
- [ ] `usb_mouse_policy` 配置化

### Power — 🟡 进行中

**已完成**

- [x] Rust `PowerManager` 基础骨架（Active / Suspend / Sleep 状态）
- [x] Power state request 走 RuntimeCommand 边界
- [x] Suspend 最小闭环：Rust 评估并进入 Suspend，平台 `prepare/enter suspend` 已实现
- [x] Suspend wake source：button / encoder / IMU 通过 EXTI 接入，wake 后恢复 Active IMU profile
- [x] Sleep 最小闭环：平台 `prepare/enter/restore sleep` 项目级路径已通
  - prepare：关激光、IMU sleep profile、关 BLE advertising
  - enter：暂不切芯片 deep-sleep（`return VP_STATUS_OK`）
  - restore：USB configured 时跳过，否则重开 BLE advertising
- [x] wake 后清理：旧 attitude / IMU sample / motion cache、report 累积、wheel 暂存、button sync 基线
- [x] runtime 将 requeue 中的 vendor 待发包视为低功耗 blocker

**未完成**

- [ ] Suspend 真实 deep low-power entry（`LowPower_Halt_WFE()`）
  - 需先证明 BLE connected 下进低功耗不断链
- [ ] wake 后更完整的外设恢复
- [ ] 低功耗与 IMU profile、RF、route 状态联动
- [ ] 完整 sleep 电流优化：接口与状态路径已落地，电流目标与阈值调优后续进行。

### Config / Storage — ✅ 已完成

- [x] `DeviceConfig` 结构（power / motion / report 子配置）
- [x] 默认配置与业务校验
- [x] 双槽 DataFlash（SlotHeader + payload + CRC32）
- [x] postcard 序列化 + serde
- [x] 全链路验证（magic → version → CRC → 反序列化 → 业务校验）
- [x] 写入状态机（begin / chunk / commit / abort）
- [x] WebHID 配置命令：GetConfigInfo / ReadConfig / WriteConfig / SaveConfig / RestoreDefaults
- [x] 自动生成 C stubs 的测试框架
- [x] 纯逻辑模块 100% 覆盖率
- [x] 版本迁移（migration）
- [x] 配置写入运行时通用 `apply_to` 机制

### LED / PWM — ✅ 已完成

- [x] `LedProfile<N>` 编译期定义（loop / once）
- [x] `LedSequenceBuilder`：`Segment::Level`（固定亮度）、`Segment::Fade`（匀加速淡入淡出）
- [x] `once_profile!` / `loop_profile!` 宏
- [x] 单元测试覆盖 builder 边界条件
- [x] `pwm::set_laser_duty()` FFI
- [x] C 侧 `PwmPlatform_Init()`
- [x] Rust FFI 绑定 `LedPlatform_Play()` / `LedPlatform_Stop()`
- [x] Rust LedManager 运行时播放器：持续态（充电/低电量）与瞬态（连接/断开/模式切换）调度，已接入 `Runtime::process_once()` 每轮 poll 驱动
- [x] 预置 Profile：`CONNECTED`、`DISCONNECTED`、`CHARGING`、`LOW_BATTERY`、`MODE_BLE`、`MODE_2G4`
- [x] 事件联动：BleConnected / BleDisconnected / UsbStateChanged / ModeSwitchExti 触发瞬态播放
- [x] LED / PWM 与电源状态联动（低功耗时停止动画，醒来后由 led_manager.poll 恢复）

### Vendor / WebHID — 🟡 进行中

**已完成**

- [x] `VendorRuntime` RX queue（SPSC，容量 4）
- [x] 单包协议解析骨架（magic/version/flags/seq/cmd/status/offset/total_len/payload_len）
- [x] 以下命令已接入：

| ID       | Command           |
| -------- | ----------------- |
| `0x0000` | Ping              |
| `0x0001` | GetProtocolInfo   |
| `0x0002` | GetDeviceInfo     |
| `0x0100` | GetConfigInfo     |
| `0x0101` | ReadConfig        |
| `0x0102` | WriteConfigBegin  |
| `0x0103` | WriteConfigChunk  |
| `0x0104` | WriteConfigCommit |
| `0x0105` | WriteConfigAbort  |
| `0x0106` | SaveConfig        |
| `0x0107` | RestoreDefaults   |
| `0x0201` | GetRouteState     |
| `0x0202` | GetPowerState     |
| `0x0300` | GetDiagnostics    |

- [x] 配置写会话状态机
- [x] C 层仅收发 raw vendor report

**未完成**

- [ ] 多包分片：协议已预留 `CUSTOM_FLAG_FRAGMENT`、`offset`/`total_len`，但解析时返回 `FragmentNotSupported`
- [ ] BLE Custom GATT / USB Custom HID transport backend

## 暂缓项

- [ ] 2.4G 协议栈：route ready / mouse send / vendor send 真实发送实现。待硬件开始后推进。
