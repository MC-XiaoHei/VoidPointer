# VoidPointer 配置存储规格

本文档定义 VoidPointer 固件配置系统的存储格式、配置分块、默认值策略与 DataFlash 保存流程。WebHID 命令协议暂不在本文档展开。

---

## 1. 设计目标

配置系统需要满足：

- 配置跟随设备本体，不依赖主机驱动。
- Rust 负责配置结构、校验、默认值、版本迁移和解释。
- C 只提供 DataFlash read/erase/write 等块设备式 API，不解析配置内容。
- 断电或写入失败时，设备仍能回退到上一份有效配置或默认配置。
- 后续 WebHID 可读写配置，但配置持久化格式不依赖 WebHID 协议。
- 配置格式可演进，支持版本号和迁移。

---

## 2. 职责分工

### 2.1 Rust 负责

- 定义 `DeviceConfig`。
- 提供默认配置。
- 校验配置合法性。
- 序列化/反序列化配置 payload。
- 计算和校验 CRC。
- 选择有效配置槽。
- 执行版本迁移。
- 决定何时保存配置。
- 决定配置是否立即应用到 runtime。

### 2.2 C 负责

- 提供 DataFlash 区域读写能力。
- 提供 page erase。
- 提供 aligned write。
- 提供 config storage 起始地址、长度、page size。
- 保证 flash 操作期间的底层互斥与中断安全。

C 不负责：

- 解析配置字段。
- 校验配置含义。
- 计算业务默认值。
- 决定配置迁移。
- 判断 WebHID 命令是否合法。

---

## 3. 存储布局

推荐使用双槽保存策略。

```text
Config Region
├── Slot A
│   ├── Header
│   └── Payload
└── Slot B
    ├── Header
    └── Payload
```

### 3.1 双槽目标

- 保存新配置时不覆盖唯一有效配置。
- 启动时选择 sequence 最大且 CRC 有效的槽。
- 写入失败时仍可回退到旧槽。
- 支持简单 wear leveling。

### 3.2 Slot 大小

Slot 大小由 C 通过 DataFlash API 暴露，Rust 根据：

- config region size
- page size
- slot count

计算每个 slot 可用容量。

建议：

- slot count 固定为 `2`。
- 每个 slot 至少容纳 header + 当前 payload + 预留扩展空间。
- payload 超过 slot 容量时拒绝保存。

### 3.3 CH585 DataFlash 约束

CH585 资料依据见 `CH585_NOTES.md`：

- DataFlash 容量为 32KB，可容纳双槽配置和 metadata。
- 官方 EEPROM/DataFlash API 支持 read / erase / write。
- DataFlash page 为 256 bytes；写入最小可按 byte length，但 256-byte page 对齐更优。
- `EEPROM_MIN_ER_SIZE` 为 256，`EEPROM_BLOCK_SIZE` 为 4096；同时 `EEPROM_ERASE` inline 对部分 chip id 要求 erase length 为 4096 的倍数。
- 初版配置 slot 建议保守按 4KB erase block 对齐；实机确认 256-byte erase 可用后再放宽。
- ISP/EEPROM API 要求 RAM buffer 4-byte aligned；C 平台层应在 FFI 边界保证或复制到 aligned buffer。

---

## 4. 配置 Header

配置 header 建议使用固定 C ABI 友好的布局。

字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `magic` | `u32` | 固定 magic，用于识别 VoidPointer 配置。 |
| `header_version` | `u16` | header 格式版本。 |
| `config_version` | `u16` | payload 配置结构版本。 |
| `total_len` | `u32` | header + payload 总长度。 |
| `payload_len` | `u32` | payload 长度。 |
| `sequence` | `u32` | 单调递增保存序号。 |
| `payload_crc32` | `u32` | payload CRC32。 |
| `header_crc32` | `u32` | header 自身 CRC32，计算时该字段置 0。 |

建议 magic：

- ASCII 语义可取 `VPFG`。
- 实际数值实现时固定为 little-endian `0x47465056`。

### 4.1 Header 校验顺序

启动扫描 slot 时按以下顺序：

1. 校验 `magic`。
2. 校验 `header_version` 是否支持。
3. 校验 `total_len` 是否在 slot 容量内。
4. 校验 `payload_len` 是否合理。
5. 校验 `header_crc32`。
6. 校验 `payload_crc32`。
7. 校验 `config_version` 是否支持或可迁移。
8. 反序列化 payload。
9. 进行业务字段校验。

---

## 5. Payload 编码

为了避免 Rust 结构体内存布局直接成为永久存储格式，payload 不建议直接 dump `DeviceConfig` 内存。

推荐使用手写二进制 TLV 或固定小端字段编码。

### 5.1 推荐方案：版本化固定字段编码

当前版本可以采用固定字段顺序的小端二进制编码：

- 字段顺序由 `config_version` 固定。
- 所有整数使用 little-endian。
- 浮点参数可使用 `f32` little-endian。
- bool 使用 `u8`，`0 = false`，`1 = true`。
- enum 使用 `u8` 或 `u16`，必须保留 unknown 检查。

优点：

- 实现简单。
- `no_std` 下容易处理。
- 不依赖 serde/heap。
- CRC 结果稳定。

### 5.2 预留 TLV 方案

如果后续配置扩展频繁，可迁移到 TLV：

| 字段 | 类型 |
| --- | --- |
| tag | `u16` |
| len | `u16` |
| value | `[u8; len]` |

当前阶段不强制 TLV，避免复杂度过早上升。

---

## 6. DeviceConfig 分块

`DeviceConfig` 逻辑上分为以下块。

```text
DeviceConfig
├── MotionConfig
├── InputConfig
├── HidConfig
├── PowerConfig
├── RouteConfig
└── DiagnosticsConfig / Reserved
```

---

## 7. MotionConfig

### 7.1 字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `deadzone_x_rad` | `f32` | X 轴死区，单位 rad。 |
| `deadzone_y_rad` | `f32` | Y 轴死区，单位 rad。 |
| `max_angle_rad` | `f32` | 达到最大速度的角度。 |
| `sensitivity_x` | `f32` | X 轴速度系数。 |
| `sensitivity_y` | `f32` | Y 轴速度系数。 |
| `deadzone_speed` | `f32` | 小速度归零阈值。 |
| `smoothing_alpha` | `f32` | 速度滤波系数。 |
| `curve_kind` | `u8` | 非线性曲线类型。 |
| `axis_x` | `u8` | X 光标映射的姿态轴。 |
| `axis_y` | `u8` | Y 光标映射的姿态轴。 |
| `invert_x` | `u8` | X 轴反向。 |
| `invert_y` | `u8` | Y 轴反向。 |
| `swap_xy` | `u8` | 是否交换 X/Y。 |
| `max_speed_x` | `f32` | X 最大速度，可选。 |
| `max_speed_y` | `f32` | Y 最大速度，可选。 |

### 7.2 curve_kind

| 值 | 含义 |
| --- | --- |
| `0` | Linear |
| `1` | Quadratic，默认 |
| `2` | Cubic |
| `3` | Exponential，预留 |
| `4` | Piecewise，预留 |

### 7.3 axis

| 值 | 含义 |
| --- | --- |
| `0` | Roll |
| `1` | Pitch |
| `2` | Yaw |

### 7.4 默认值建议

初始可沿用当前 Rust 默认值：

| 字段 | 默认值 |
| --- | --- |
| `deadzone_x_rad` | `0.05` |
| `deadzone_y_rad` | `0.05` |
| `deadzone_speed` | `0.1` |
| `max_angle_rad` | `1.0` |
| `sensitivity_x` | `12000.0` |
| `sensitivity_y` | `12000.0` |
| `invert_x` | `false` |
| `invert_y` | `false` |
| `swap_xy` | `false` |
| `smoothing_alpha` | `0.2` |
| `curve_kind` | `Quadratic` |

后续通过实测调整。

### 7.5 校验规则

- `deadzone_x_rad >= 0`。
- `deadzone_y_rad >= 0`。
- `max_angle_rad > deadzone_x_rad` 且 `max_angle_rad > deadzone_y_rad`。
- `smoothing_alpha` 范围 `[0.0, 1.0]`。
- sensitivity 必须为有限正数。
- enum 值必须在支持范围内。
- f32 必须是 finite，不允许 NaN/Inf。

---

## 8. InputConfig

### 8.1 字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `button_debounce_samples` | `u8` | 普通按键连续稳定采样数。 |
| `switch_settle_ms` | `u16` | mode switch 切换后的稳定窗口。 |
| `switch_stable_samples` | `u8` | mode switch 连续稳定采样数。 |
| `encoder_detent_steps` | `u8` | 一个滚轮刻度对应的微步数，默认 4。 |
| `wheel_invert` | `u8` | wheel 方向反转。 |
| `middle_motion_enable` | `u8` | 中键是否触发 motion。 |
| `action_motion_enable` | `u8` | Action 是否触发 motion。 |
| `laser_mode` | `u8` | Laser 行为。 |

### 8.2 laser_mode

| 值 | 含义 |
| --- | --- |
| `0` | 按住点亮，默认。 |
| `1` | toggle，预留。 |
| `2` | disabled。 |
| `3` | remap to HID/vendor action，预留。 |

### 8.3 默认值建议

| 字段 | 默认值 |
| --- | --- |
| `button_debounce_samples` | `8` |
| `switch_settle_ms` | `20` |
| `switch_stable_samples` | `8` |
| `encoder_detent_steps` | `4` |
| `wheel_invert` | `false` |
| `middle_motion_enable` | `true` |
| `action_motion_enable` | `true` |
| `laser_mode` | `0` |

### 8.4 校验规则

- `button_debounce_samples` 范围建议 `[3, 32]`。
- `switch_settle_ms` 范围建议 `[5, 200]`。
- `switch_stable_samples` 范围建议 `[1, 32]`。
- `encoder_detent_steps` 范围建议 `[1, 8]`，默认 4。
- bool 字段只能是 `0` 或 `1`。
- `laser_mode` 必须是支持值。

---

## 9. HidConfig

### 9.1 字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `report_hz` | `u16` | HID report 目标频率。 |
| `usb_mouse_policy` | `u8` | USB 插入且 configured 时的 mouse report 策略。 |
| `ble_name_index` | `u8` | BLE 名称配置索引或预留。 |
| `send_zero_on_motion_stop` | `u8` | motion stop 后是否发送零移动同步帧。 |
| `clamp_negative_128` | `u8` | 是否避免发送 HID delta -128。 |

### 9.2 usb_mouse_policy

| 值 | 名称 | 含义 |
| --- | --- | --- |
| `0` | `Disabled` | USB 插入时禁用全部 mouse report，包括 motion、按键、滚轮。 |
| `1` | `MotionDisabled` | USB 插入时仅禁用 `dx/dy` 移动，保留 buttons/wheel。 |
| `2` | `Enabled` | USB 插入时 mouse report 全部开启，并默认走 USB route。 |

`wired_behavior` 废弃；新配置使用 `usb_mouse_policy`。项目尚未进入正式配置发布阶段，因此不需要考虑旧 `wired_behavior` 配置迁移。

### 9.3 默认值建议

| 字段 | 默认值 |
| --- | --- |
| `report_hz` | `1000` |
| `usb_mouse_policy` | `Disabled` |
| `send_zero_on_motion_stop` | `true` |
| `clamp_negative_128` | `true` |

### 9.4 校验规则

- `report_hz` 范围建议 `[125, 1000]`。
- `usb_mouse_policy` 必须是支持值。
- bool 字段只能是 `0` 或 `1`。

---

## 10. PowerConfig

### 10.1 字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `suspend_timeout_ms` | `u32` | 静置后进入 `Suspend` 的时间。 |
| `disconnect_sleep_timeout_ms` | `u32` | 无连接后进入 `Sleep` 的门控时间。从断连时刻开始计算，不沿用进入 Suspend 前的 idle 时间。 |
| `imu_active_profile` | `u8` | Active IMU profile id。 |
| `imu_suspend_profile` | `u8` | Suspend IMU profile id。 |
| `imu_sleep_profile` | `u8` | Sleep IMU profile id。 |
| `allow_sleep_on_usb_detached_only` | `u8` | 是否仅 USB detached 时允许 Sleep。 |

### 10.2 默认值建议

| 字段 | 默认值 |
| --- | --- |
| `suspend_timeout_ms` | `30000` |
| `disconnect_sleep_timeout_ms` | `60000` |
| `imu_active_profile` | `0` |
| `imu_suspend_profile` | `0` |
| `imu_sleep_profile` | `0` |
| `allow_sleep_on_usb_detached_only` | `true` |

### 10.3 校验规则

- `suspend_timeout_ms` 与 `disconnect_sleep_timeout_ms` 是两个独立门控：`suspend_timeout_ms` 从最后活动时间计算；`disconnect_sleep_timeout_ms` 在无线断连后从断连时间计算。
- `suspend_timeout_ms` 范围建议 `[1000, 3600000]`。
- `disconnect_sleep_timeout_ms` 范围建议 `[5000, 86400000]`。
- profile id 必须是固件支持的 profile。
- bool 字段只能是 `0` 或 `1`。

---

## 11. RouteConfig

### 11.1 字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `default_ble_advertise` | `u8` | BLE 模式下断开后是否在非 Sleep 阶段广播。Sleep 阶段不广播。 |
| `default_dongle_pairing` | `u8` | 2.4G 模式下断开后是否在非 Sleep 阶段搜索/配对。Sleep 阶段不搜索。 |
| `clear_motion_on_disconnect` | `u8` | route 断开时是否清 motion pending。 |
| `sync_buttons_on_reconnect` | `u8` | route 恢复后是否同步当前 button state。 |

### 11.2 默认值建议

| 字段 | 默认值 |
| --- | --- |
| `default_ble_advertise` | `true` |
| `default_dongle_pairing` | `true` |
| `clear_motion_on_disconnect` | `true` |
| `sync_buttons_on_reconnect` | `true` |

### 11.3 校验规则

- bool 字段只能是 `0` 或 `1`。

---

## 12. Reserved / DiagnosticsConfig

建议在 payload 末尾保留：

- `reserved_u8[16]` 或版本化 reserved 字段。
- diagnostics config，例如是否启用 debug counters 暴露。

保留区必须在保存时写入确定值，建议全 0，避免 CRC 不稳定。

---

## 13. 启动加载流程

启动时 Rust 执行：

1. 通过 C API 查询 config storage region。
2. 读取 Slot A header。
3. 读取 Slot B header。
4. 分别校验 header 和 payload CRC。
5. 丢弃无效 slot。
6. 如果两个 slot 都有效，选择 `sequence` 较大的 slot。
7. 反序列化 payload。
8. 如果 `config_version` 低于当前版本，执行 migration。
9. 执行业务校验。
10. 如果成功，应用配置。
11. 如果失败，加载默认配置。
12. 如果发生迁移，可标记 `config_save_pending`，由 `vp_core_poll()` 后续保存新版本。

### 13.1 sequence 回绕

`sequence` 是 `u32`。比较时应考虑回绕。

推荐规则：

- 如果两个 sequence 相差小于 `0x80000000`，较大者更新。
- 如果差值超过该范围，按回绕规则判断。

也可以简化为：设备保存次数远低于 `u32` 上限，初版直接比较大小，后续再增强。

---

## 14. 保存流程

保存配置必须在 `vp_core_poll()` 中执行，不在 ISR 中执行。

流程：

1. Rust runtime config 被修改。
2. 标记 `config_dirty = true`。
3. WebHID 或内部逻辑请求保存。
4. `vp_core_poll()` 检查当前是否允许写 flash。
   - 不在 ISR。
   - 没有正在进行的 flash 写。
   - 电源状态保持 Active。
   - 禁止进入 `Suspend`/`Sleep`。
5. Rust 序列化 payload 到固定 buffer。
6. Rust 计算 payload CRC。
7. Rust 生成 header。
8. 选择 inactive slot 或 sequence 较旧的 slot。
9. 调 C erase slot 所在 page。
10. 调 C write header + payload。
11. 读回 header + payload 校验。
12. 成功后更新 active slot metadata。
13. 失败则保留旧 slot，报告错误计数。

---

## 15. Runtime apply 规则

配置分为两类应用方式。

### 15.1 可立即应用

- motion sensitivity。
- deadzone。
- curve kind。
- axis mapping。
- debounce 参数。
- wheel invert。
- usb mouse policy。
- power timeout。

### 15.2 需要延后或重启子系统

- BLE name。
- HID descriptor 相关选项。
- IMU profile register set。
- 2.4G pairing 参数。

对于需要延后的配置：

- Runtime 可先保存配置。
- 标记 `restart_required` 或 `subsystem_restart_required`。
- 通过 WebHID status 告知主机。

---

## 16. 错误处理

### 16.1 启动无有效配置

- 使用默认配置。
- 设置 diagnostics flag：`CONFIG_LOAD_DEFAULTED`。
- 不立即写 flash，除非用户保存或迁移策略要求。

### 16.2 CRC 失败

- 丢弃该 slot。
- 如果另一个 slot 有效，使用另一个。
- 如果都失败，使用默认配置。

### 16.3 保存失败

- 保留旧配置槽。
- Runtime 继续使用当前内存配置。
- 增加 flash write error counter。
- WebHID status 可返回错误。

### 16.4 配置字段非法

- 拒绝该配置。
- 不应用。
- 不保存。
- 返回 invalid config error。

---

## 17. 与电源状态的关系

- 配置写入期间禁止进入 `Suspend`/`Sleep`。
- 配置 dirty 但未保存时，可以进入 `Suspend`，但禁止进入 `Sleep`；进入 `Sleep` 前必须先保存，保存完成后才允许。
- 如果电池极低且配置保存未完成，应优先保证旧配置槽不被破坏。
- WebHID 配置会话不作为特殊 power blocker；如果满足普通 idle 条件，可以按普通规则进入 `Suspend`。

---

## 18. 与 WebHID 的关系

WebHID 只负责传输配置命令，不定义底层 flash 格式。

推荐行为：

- `set runtime config`：只修改内存配置，立即应用可热更新字段。
- `save config`：触发本文档定义的保存流程。
- `reset config`：恢复默认配置，可选择只 runtime apply 或同时保存。
- `get config`：返回当前 runtime config，而不直接 dump flash slot。
- `get config storage status`：返回 active slot、sequence、CRC 状态、dirty 状态。

WebHID 协议包格式后续单独设计。

---

## 19. 初版实现建议

初版可以按以下最小集合实现：

1. 双槽 header + payload。
2. 固定字段小端 payload。
3. CRC32。
4. 默认配置 fallback。
5. 无 migration 或只支持当前版本。
6. `set runtime config` 与 `save config` 分离。
7. 配置写入只在 `vp_core_poll()` 中进行。

这样已经可以满足项目当前需求，并为后续 WebHID 和配置版本演进留出空间。
