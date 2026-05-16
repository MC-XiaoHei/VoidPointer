# Route State Machine

BLE / 2.4G / USB 路由状态机和切换规则。实现进度见 `TASKLIST.md`。

## 1. Route 类型

| Route | 说明 |
| --- | --- |
| `None` | 无可用 HID route。 |
| `BLE` | 蓝牙 HID。 |
| `Dongle2G4` | 2.4G dongle HID。 |
| `USB` | USB HID / Vendor HID。 |

v1 实现范围见 `TASKLIST.md`。


## 2. 物理模式开关

当前硬件为两段式物理模式开关：

| 档位 | 语义 |
| --- | --- |
| BLE | 选择 BLE 无线模式。 |
| 2.4G | 选择 2.4G dongle 模式。 |

物理开关决定默认无线 route。USB 插入时的 mouse 行为由 `HidConfig.usb_mouse_policy` 决定。

## 3. USB mouse policy

`CONFIG_SPEC.md` 使用 `usb_mouse_policy` 描述 USB configured 时的 mouse report 策略：

| 值 | 名称 | 行为 |
| --- | --- | --- |
| `0` | `Disabled` | USB 插入时禁用全部 mouse report，包括 motion、按键、滚轮。 |
| `1` | `MotionDisabled` | USB 插入时仅禁用 `dx/dy`，保留 buttons/wheel。 |
| `2` | `Enabled` | USB 插入时 mouse report 全部开启，并默认走 USB route。 |

`wired_behavior` 废弃。项目尚未进入正式配置发布阶段，因此不需要考虑旧配置迁移。

## 4. Route 优先级

### 4.1 Mouse HID route

| 条件 | Mouse report 目标 |
| --- | --- |
| USB configured 且 `usb_mouse_policy = Disabled` | 不发送 mouse report。 |
| USB configured 且 `usb_mouse_policy = MotionDisabled` | `dx/dy = 0`；buttons/wheel 继续发送，目标 route 按当前 active mouse route 决定。 |
| USB configured 且 `usb_mouse_policy = Enabled` | USB。 |
| USB 未 configured | 由物理开关选择 BLE 或 2.4G。 |

补充规则：

- `USB configured` 时采用 wired priority。
- 当前实现下，进入 `Configured` 后会关闭 BLE 广播并断开现有 BLE 连接。

### 4.2 Vendor/WebHID route

结论：v1 当前只有 USB route 完整支持 Vendor/WebHID 收发。

- USBHS Vendor HID：已接入。
- BLE Vendor / custom HID：接口保留，但当前发送侧仍按 `NotConnected` 收口。
- 2.4G Vendor / custom channel：保留 stub，当前发送侧按 `NotConnected` 收口。

当前回复规则：

1. 优先沿收到请求的同一路径回复。
2. 如果该路径当前不可用，则退回 `preferred_custom_route()`。
3. 如果最终 route 仍不可用，本次发送尝试收敛，等待 route 相关事件再次唤醒。

Vendor/config 会话不作为特殊 power blocker；如果设备满足普通 idle 条件，允许进入 `Suspend`。收到新的 vendor report 时再唤醒处理。

### 5.1 BLE connected 与 input-ready

BLE route 需要把链路连接和输入路径可发送分开建模：

- `connected` 表示链路存在
- `input-ready` 表示 HID notify / security / profile 路径已经可发送

因此 `connected` 不等于 route ready。只有进入 input-ready 后，mouse report 和 vendor reply 才能把 BLE 当成真正可发送的 route。

### 5.2 BLE 运行规则

建议状态：

| 状态 | 说明 |
| --- | --- |
| `Disabled` | BLE route 未启用。 |
| `Advertising` | 正在广播，未连接。 |
| `Connected` | 已连接，可发送 HID。 |
| `Error` | 协议栈错误或需要 reset。 |

BLE 断开后：

- `Suspend` 阶段可以继续 advertising。
- 进入 `Sleep` 后停止 advertising。
- BLE 断开不会导致设备立即 `Sleep`；从断开时刻开始计算单独的 sleep gate，例如断开后 1 分钟再允许进入 `Sleep`。

## 6. 2.4G 状态

建议状态：

| 状态 | 说明 |
| --- | --- |
| `Disabled` | 2.4G route 未启用。 |
| `Pairing` | 正在配对/搜索 dongle。 |
| `Connected` | 已连接 dongle，可发送 HID。 |
| `Error` | 需要 route reset。 |

2.4G 断开后：

- `Suspend` 阶段可以继续搜索/配对。
- 进入 `Sleep` 后停止搜索/配对。
- 2.4G 断开不会导致设备立即 `Sleep`；从断开时刻开始计算单独的 sleep gate，例如断开后 1 分钟再允许进入 `Sleep`。

## 7. USB 状态

建议状态：

| 状态 | 说明 |
| --- | --- |
| `Detached` | 未插入 USB host。 |
| `Attached` | 检测到插入，尚未 configured。 |
| `Configured` | USB 设备枚举完成。 |
| `Suspended` | USB bus suspend。 |
| `Error` | USB endpoint/stall 等错误。 |

USB configured 时，系统保持 `Active`；当前设计认为插入使用场景不需要为省电进入 `Suspend` / `Sleep`。

CH585 USBHS 事件映射策略：

| 低层事件/条件 | v1 `vp_usb_state_t` 映射 | 说明 |
| --- | --- | --- |
| 启动后未观察到 link-ready，且平台没有明确 USB host/board-level attached 信号 | `Detached` | 保守初始状态。 |
| `USBHS_UDIF_LINK_RDY` 或 `R8_USB2_MIS_ST` 显示 ready/connected | `Attached` | 表示 USB link-ready 候选；尚未 configured。 |
| `USB_SET_CONFIGURATION` 成功，`USBHS_DevConfig` / `USBHS_DevEnumStatus` 置位 | `Configured` | USB HID/vendor endpoint 可服务。 |
| `USBHS_UDIF_SUSPEND` 且 `RB_UMS_SUSPEND` 置位 | `Suspended` | USB bus suspend；不等同项目级 `Suspend`。 |
| `USBHS_UDIF_SUSPEND` 但 `RB_UMS_SUSPEND` 清除 | 回到 `Configured` 或 `Attached` | 如果此前 configured 仍有效则回 `Configured`，否则回 `Attached`。 |
| `USBHS_UDIF_BUS_RST` | 清 configured，回 `Attached`，并 reinit endpoints | bus reset 不是 physical detach；不直接上报 `Detached`。 |
| endpoint stall / controller error / descriptor 状态异常 | `Error` | 平台可 reset USBHS 后重新进入 `Attached`。 |
| 明确 VBUS lost、board-level detach、或后续验证到可靠 link-lost 信号 | `Detached` | 当前 WCH 示例未给出明确 detach callback；只有出现可靠依据时才上报。 |

原则：不要把 `BUS_RST` 或普通 suspend 当作物理拔出。`Detached` 只在启动无 link-ready，或平台有明确 VBUS/link-lost/板级 detach 信号时上报。

## 8. Route 切换行为

已有配置：

| 字段 | 默认 | 语义 |
| --- | --- | --- |
| `clear_motion_on_disconnect` | true | route 断开时清 motion pending。 |
| `sync_buttons_on_reconnect` | true | route 恢复后同步当前 button state。 |

建议规则：

- route 断开时停止 motion 输出，必要时发送 zero report 取决于底层可用性。
- route 重连后同步当前 buttons，避免 host 端卡键。
- mode switch 改变时清空 motion baseline 和 pending delta。
- USB behavior 改变时重新评估 primary route。

## 9. 与 Power 的关系

| Route 情况 | Power 策略 |
| --- | --- |
| BLE/2.4G connected | 静置后进入 `Suspend`，保持连接。 |
| BLE/2.4G disconnected | 从断连时刻重新计算 `disconnect_sleep_timeout_ms`，超时后允许 `Sleep`。 |
| USB configured | 保持 `Active`，禁止 `Suspend` / `Sleep`。 |
| Vendor/config active | 不做特殊 blocker；按普通 idle 规则允许 `Suspend`。 |


