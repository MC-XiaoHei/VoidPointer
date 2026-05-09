# Test Plan

本文定义 VoidPointer 固件的测试与验收计划。当前是 Draft。

## 1. 输入测试

### 1.1 按键消抖

| 用例 | 验收标准 |
| --- | --- |
| 单击 Left/Right/Middle/Action/Laser | 每次只产生一次 press/release，重复点击不丢失。 |
| 高频点击 | 无丢失、无多触发，延迟符合 debounce 配置。 |
| 长按 | press 只触发一次；持有期间不产生中断风暴；release 正常。 |
| 抖动边沿模拟 | EXTI/电平中断首次触发后屏蔽，debounce 裁决稳定状态，并按稳定态重新 arm 相反电平。 |
| GPIOA pending service | 当 `R16_PA_INT_IF & R16_PA_INT_EN` 已置位但 PFIC 未派发时，runtime service 能复用 GPIOA handler 处理事件。 |

### 1.2 ModeSwitch

| 用例 | 验收标准 |
| --- | --- |
| BLE ↔ 2.4G 切换 | 复用二态输入 debounce，只发布最终稳定档位。 |
| 切换过程抖动 | 不产生多次 route switch，稳定后重新 arm 相反电平。 |
| 切换时 motion active | 清 motion baseline/pending，避免跳变。 |

### 1.3 编码器

| 用例 | 验收标准 |
| --- | --- |
| 慢速单格滚动 | 每个 detent 产生一次 wheel。 |
| 正反边缘抖动 | 微步在 `internal_phase` 中抵消，不乱跳。 |
| 高速滚动 | 不因延时屏蔽窗口丢步。 |

## 2. Motion / IMU 测试

| 用例 | 验收标准 |
| --- | --- |
| Action 按下 | 记录当前姿态为 baseline。 |
| Action 按住倾斜 | 角度偏移映射为光标速度。 |
| Action 松开 | 立即停止 motion，并按配置发送 zero report。 |
| Middle motion | 配置启用时行为同 Action。 |
| 小角度微操 | deadzone 与低速曲线符合配置。 |
| 大角度移动 | 达到最大速度限制，无溢出。 |
| FIFO 多样本 | 使用 latest-sample 策略，低延迟优先；FIFO 读取由 Rust bottom-half 发起。 |
| IMU wake | Suspend/Sleep 下由 accel wake interrupt 唤醒，不依赖 SFLP；IMU INT 不直接承载姿态数据语义。 |

## 3. HID / Route 测试

| 用例 | 验收标准 |
| --- | --- |
| BLE connected report | report 格式正确，按钮/dx/dy/wheel 正确。 |
| HID RetryLater | Rust 保留 pending 并重试，不丢关键状态。 |
| Disconnect | 清 motion pending，避免重连后跳变。 |
| Reconnect | 按配置同步当前 button state。 |
| USB 策略：全部禁用 | USB 插入时不发送 mouse report。 |
| USB 策略：仅禁用移动 | USB 插入时 `dx/dy = 0`，buttons/wheel 仍按 route 策略发送。 |
| USB 策略：全部开启 | USB 插入时 mouse report 完整启用。 |
| Vendor over USB | WebHID/vendor command 可通过 USB 收发。 |
| Vendor over BLE | WebHID/vendor command 可通过 BLE 收发。 |
| Vendor over 2.4G | WebHID/vendor command 可通过 2.4G 收发。 |

## 4. Power 测试

| 用例 | 验收标准 |
| --- | --- |
| Active → Suspend | 有连接静置超过 timeout 后进入，RF 保持连接。 |
| USB configured idle | 保持 `Active`，不进入 `Suspend` / `Sleep`。 |
| Suspend wake by button | 秒唤醒，按键事件不丢。 |
| Suspend wake by IMU | 拿起/移动后由 IMU wake interrupt 唤醒并恢复 Active。 |
| Suspend disconnect grace | 断连后从断连时刻重新计算 `disconnect_sleep_timeout_ms`，不立即 Sleep。 |
| Active → Sleep | 无连接、USB detached、静置超过 timeout 后进入，RF 关闭。 |
| Sleep wake by button | 外设恢复，Rust 收到 wake 事件。 |
| Sleep wake by IMU | 由低功耗 accel wake interrupt 唤醒；wake 后姿态数据仍由 bottom-half FIFO 路径获取。 |
| Flash write blocker | 写入期间禁止 Suspend/Sleep。 |
| Config dirty | 进入 Sleep 前必须先保存。 |
| Laser stuck-on | 进入低功耗前自动关闭 Laser，并记录/诊断为异常或硬件 bug。 |
| Vendor/config idle | Vendor/config 会话不特殊阻止 Suspend。 |

## 5. Config / DataFlash 测试

| 用例 | 验收标准 |
| --- | --- |
| 空 flash 启动 | 加载默认配置。 |
| 单槽有效 | 选择有效槽。 |
| 双槽有效 | 选择 sequence 更新的槽。 |
| CRC 损坏 | 丢弃损坏槽。 |
| 保存断电 | 旧槽仍可恢复。 |
| migration | 旧版本可迁移到当前版本。 |
| runtime apply | 可立即应用项立即生效，需要延后项按规则延后。 |

## 6. Panic / 异常测试

| 用例 | 验收标准 |
| --- | --- |
| IMU WHO_AM_I 失败 | 进入诊断/降级路径，不崩溃。 |
| I2C bus stuck | 执行 bus recovery。 |
| HID fatal | route reset 或降级。 |
| Rust panic | 输出 panic reason，后续策略待确认。 |

## 7. 功耗验收

功耗目标需要补充实测或目标值：

| 状态 | 目标电流 | 状态 |
| --- | --- | --- |
| `Active` | TBD | 待硬件实测。 |
| `Suspend` | TBD | 待硬件实测。 |
| `Sleep` | TBD | 待硬件实测。 |
