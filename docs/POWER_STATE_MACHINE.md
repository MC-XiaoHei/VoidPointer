# Power State Machine

电源状态机：状态语义、转换条件和职责边界。实现进度见 `TASKLIST.md`。

## 1. 状态模型

VoidPointer 采用三状态电源模型：

| 状态 | 类型 | 用户语义 | 目标 |
| --- | --- | --- | --- |
| `Active` | 工作态 | 正在操作或需要即时响应 | 全功能运行、最低交互延迟。 |
| `Suspend` | 连接保持浅休眠 | 有连接但静置 | 保持 BLE/2.4G 连接，拿起秒唤醒。 |
| `Sleep` | 断连深休眠 | 无连接且静置 | RF 关闭，保留必要 wake source，最低待机功耗。 |

注意：这不是“三级睡眠”。更准确地说是“一个工作态 + 两级低功耗态”。

v1 实现范围、目标电流和 shipping mode 等开发期信息见 `TASKLIST.md`。


## 2. 状态行为

| 行为项 | `Active` | `Suspend` | `Sleep` |
| --- | --- | --- | --- |
| BLE / 2.4G RF | 按 route 策略启用 | 保持已连接 route，不以断链重连伪装 `Suspend` | 关闭或不保持连接 |
| USB | 正常处理 attach/configured/vendor | USB configured 时不进入 `Suspend`；`Suspend` 不以 USB bus suspend/resume 作为核心语义 | USB attached/configured 时禁止进入；仅 USB detached 时允许 |
| IMU | Active profile，SFLP/FIFO 用于 motion 轮询读取 | Suspend profile，低功耗 accel wake interrupt | Sleep profile，SFLP off，gyro power-down，accel wake interrupt |
| HID mouse report | 可发送 | 通常不主动发送，wake 后恢复 | 不发送 |
| Vendor/WebHID | 可处理 | 不做特殊 blocker；允许按普通 idle 规则进入 `Suspend` | 不处理，wake 后处理 |
| DataFlash save | 可在 `vp_core_poll()` 执行 | dirty 可存在，但进入 `Sleep` 前必须保存 | 禁止写入；未保存配置时禁止进入 |
| Laser | 可用 | 如果进入低功耗前 Laser 仍开启，视作异常/硬件 bug，平台应关闭 Laser | 如果进入低功耗前 Laser 仍开启，视作异常/硬件 bug，平台应关闭 Laser |
| 唤醒源 | 全部事件 | GPIO / encoder / IMU / route event；是否需要额外 USB 相关平台事件仅作为实现细节处理，不作为 `Suspend` 主语义 | GPIO / IMU / USB attach / route event，具体待确认 |

路由与首帧同步补充语义：

- route 不可用或未 ready 时，runtime 会丢弃本次未发送的 motion / wheel 输出，避免 route 恢复后回放旧输入。
- wake 回 `Active` 时，runtime 也会重置 mouse transport 的暂存状态，并把 button sync 基线重新置空，允许恢复后按当前按钮状态重新同步首帧。

## 3. 进入条件

### 3.1 `Active` → `Suspend`

满足全部条件时允许：

- USB 未处于 configured 状态。USB configured 时保持 `Active`，因为插入使用场景不需要为了省电进入 `Suspend`。
- 当前存在需要保持的 BLE 或 2.4G 连接，或当前策略允许无线浅休眠。
- 无实体输入活动。
- 无 motion active / arming。
- 无 HID report pending / retry pending。
- 无 DataFlash 写入正在进行。
- 距离最近活动时间超过 `suspend_timeout_ms`。

Vendor/WebHID config 会话不作为特殊 blocker；如果满足普通 idle 条件，允许进入 `Suspend`。但这不改变一条更底层的执行约束：**已经排队等待发送的 vendor report** 仍然算 pending work，必须先收敛，不能直接跨进低功耗。

### 3.2 `Active` → `Sleep`

满足全部条件时允许：

- USB detached。只要 USB attached/configured，就禁止 `Sleep`，并保持 `Active` 或按 USB 自身事件处理。
- 无 BLE / 2.4G 连接，或当前策略允许断开后深睡。
- 无实体输入活动。
- 无 motion active / arming。
- 无 HID report pending / retry pending。
- 无 DataFlash 写入正在进行。
- 配置没有 dirty；如果 dirty，必须先保存，保存完成后才允许 `Sleep`。
- 距离无线断连时刻超过 `disconnect_sleep_timeout_ms`。
- 如果进入低功耗前 Laser 仍开启，平台应关闭 Laser；该情况视作异常/硬件 bug，而不是用户正常活动。

### 3.3 `Suspend` → `Sleep`

当处于 `Suspend` 且无线连接断开后，从断连时刻重新开始计算 `disconnect_sleep_timeout_ms`。这样可以给用户留出反应时间，避免误断开后立即深睡。

示例策略：

- 静置 `suspend_timeout_ms = 30000` 后进入 `Suspend`。
- 如果在 `Suspend` 阶段断链，允许继续 BLE advertising / 2.4G 搜索。
- 断链后再等待独立 sleep gate，例如 `60000 ms`，才允许进入 `Sleep`。
- 进入 `Sleep` 后停止 BLE advertising / 2.4G 搜索。

## 4. 退出条件

### 4.1 `Suspend` → `Active`

任意条件满足：

- 按键 EXTI。
- 编码器 EXTI。
- ModeSwitch EXTI。
- IMU wake interrupt。
- USB state changed。
- BLE/2.4G route event。
- HID send completion / retry window 到达。
- Vendor/WebHID RX。

当前实现进度和阻塞点见 `TASKLIST.md`。

### 4.2 `Sleep` → `Active`

`Sleep` 唤醒后平台层需要先恢复必要外设，然后通知 Rust。`WakeEvaluate` 不作为正式 `PowerState`；唤醒后先回到 `Active`，再由 `vp_core_poll()` 重新评估是否进入 `Suspend` 或 `Sleep`。

当前实现进度见 `TASKLIST.md`。

## 5. 禁止进入低功耗的条件

以下任一条件存在时禁止进入 `Suspend` / `Sleep`：

- ISR 或快速 callback 当前正在执行。
- `vp_core_poll()` 有必须立即处理的 pending work。
- DataFlash erase/write 进行中。
- HID mouse/vendor report pending。
- I2C FIFO 读取正在进行。
- motion active / arming。
- 任一按键处于 pressed 状态，尤其 Action/Middle。
- USB configured：禁止进入 `Suspend` / `Sleep`，保持 `Active`。

以下条件允许 `Suspend` 但禁止 `Sleep`：

- 配置 dirty 但未保存。进入 `Sleep` 前必须先保存。
- BLE/2.4G 连接仍需保持。

Laser 特例：如果系统判定应进入低功耗但 Laser 仍开启，认为这是异常或硬件 bug，进入低功耗前应关闭 Laser。

## 6. timeout 与时间基准

- 时间戳统一使用 RTC millis。
- `vp_timestamp_t` 固定为 `uint32_t` RTC millis。
- `suspend_timeout_ms` 默认 `30000`。
- `disconnect_sleep_timeout_ms` 默认 `60000`，语义为断连后进入 `Sleep` 的门控时间。
- `suspend_timeout_ms` 与 `disconnect_sleep_timeout_ms` 是两个独立门控：`suspend_timeout_ms` 从最后活动时间计算；`disconnect_sleep_timeout_ms` 在 BLE/2.4G 断连后从断连时刻计算。
- activity timestamp 应由 Rust 统一维护。

## 7. C/Rust 职责

| 层 | 职责 |
| --- | --- |
| C | 执行 `prepare/enter/restore`，配置 wake source，恢复外设。 |
| Rust | 判断能否进入状态、选择目标状态、维护 timeout、维护业务 blocker。 |

C 不判断业务策略。是否进入 `Active` / `Suspend` / `Sleep` 由 Rust 决定。


