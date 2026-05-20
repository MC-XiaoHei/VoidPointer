# Vendor/WebHID 协议

配置通道的逻辑协议：帧格式、命令 ID 分配、响应状态。当前是 Draft。实现状态见 `TASKLIST.md`。

## 1. 目标

- 同一套逻辑命令可运行在 USBHS Vendor HID、BLE Vendor / custom HID、2.4G Vendor / custom channel 上。
- USBHS 作为首选高吞吐配置通道。
- BLE / 2.4G 根据各自 MTU 或私有链路能力进行传输。
- 大 payload 命令（ReadConfig / WriteConfig）使用显式 offset 分片，不在传输层做隐式重组。

## 2. 传输能力

| Route | 物理包建议 | 来源 / 说明 |
| --- | --- | --- |
| USBHS Vendor HID | 512 bytes | `USBHS/DEVICE/CompatibilityHID` 示例中 HS endpoint `wMaxPacketSize = 0x0200`，HS HID report descriptor 为 512-byte input/output。 |
| USB FS fallback | 64 bytes | 同示例 FS endpoint/report 为 64 bytes。 |
| BLE Vendor | route MTU 决定 | 需要按实际 BLE GATT/HID vendor 实现确定。 |
| 2.4G Vendor | route MTU 决定 | 需要按 2.4G 私有协议确定。 |

当前硬件使用 USBHS 口，因此 USB 配置通道按 USBHS 能力设计；仍保留 FS fallback 认知，避免后续平台差异。

Vendor route 优先级规则见 `ROUTE_STATE_MACHINE.md`，其长期结论是：USB > 当前物理模式开关对应无线 route > 另一个无线 route。

## 3. 分层

| 层 | 职责 |
| --- | --- |
| Transport | USBHS/BLE/2.4G 收发物理 report 或 packet。 |
| Framing | Frame header 解析、校验。 |
| Command | 解析 command id、request/response、错误码。 |
| Config service | 读取/写入配置、保存、恢复默认、diagnostics。 |

## 4. 帧格式

所有 route 使用相同逻辑 frame header：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `magic` | `u8` | 固定 `0xA5`。 |
| `version` | `u8` | 协议版本，初版 `1`。 |
| `flags` | `u8` | bit flags，例如 request/response。`FLAG_FRAGMENT` 保留未用。 |
| `seq` | `u8` | 请求序号，用于匹配 response。 |
| `cmd` | `u16` | command id。 |
| `status` | `u16` | response status；request 中填 0。 |
| `offset` | `u16` | ReadConfigChunk / WriteConfigChunk 的 byte offset。单包命令填 0。 |
| `total_len` | `u32` | 完整 payload 长度。单包命令填本包 payload_len。 |
| `payload_len` | `u16` | 本包 payload 长度。 |
| `payload` | bytes | 本包数据。 |

大 payload 命令（ReadConfig / WriteConfig）使用显式 offset 分片：
- 每个请求携带本次读/写的 byte offset
- 设备按 offset 返回/写入对应切片
- 上位机自行拼接或拆分，设备不做隐式重组
- `offset` 使用 byte offset 而非 chunk index，为后续支持乱序或重传预留

## 5. 命令 ID 分配

Vendor command id 使用 `u16`，按功能范围分段：

| 范围 | 用途 |
| --- | --- |
| `0x0000..0x00FF` | 基础协议 / device info / ping。 |
| `0x0100..0x01FF` | Config 读写。 |
| `0x0200..0x02FF` | Runtime apply / route / power。 |
| `0x0300..0x03FF` | Diagnostics。 |
| `0x8000..0xFFFF` | Vendor experimental / debug。 |

初版 command id：

| ID | Command | 方向 | 说明 |
| --- | --- | --- | --- |
| `0x0000` | `Ping` | host → device | 连通性测试。 |
| `0x0001` | `GetProtocolInfo` | host → device | 查询协议版本、能力、最大 payload。 |
| `0x0002` | `GetDeviceInfo` | host → device | 查询设备型号、固件版本、硬件版本。 |
| `0x0100` | `GetConfigInfo` | host → device | 查询配置版本、dirty 标志、总长度、CRC。 |
| `0x0101` | `ReadConfigChunk` | host → device | 按 offset 读取配置分片。请求 `[offset: u32]`，响应 `[offset: u32, data]`。 |
| `0x0102` | `WriteConfigBegin` | host → device | 开始写配置，声明总长度/CRC。 |
| `0x0103` | `WriteConfigChunk` | host → device | 写入配置分片。请求 `[offset: u32, data]`。 |
| `0x0104` | `WriteConfigCommit` | host → device | 校验并应用配置，标记 save pending。 |
| `0x0105` | `WriteConfigAbort` | host → device | 放弃本次写入。 |
| `0x0106` | `SaveConfig` | host → device | 请求保存到 DataFlash。 |
| `0x0107` | `RestoreDefaults` | host → device | 恢复默认配置。 |
| `0x0201` | `GetRouteState` | host → device | 查询 BLE / 2.4G / USB route 状态。 |
| `0x0202` | `GetPowerState` | host → device | 查询 power diagnostics 状态；不暴露 Rust 内部 `vp_power_state_t` ABI。 |
| `0x0300` | `GetDiagnostics` | host → device | 查询 debug counters / wake diagnostics。 |

## 6. 响应状态

Vendor/WebHID 使用独立 response status，不复用底层 `vp_status_t`。

| Status | 说明 |
| --- | --- |
| `Ok` | 成功。 |
| `InvalidCommand` | 不支持的 command。 |
| `InvalidArgument` | 参数非法。 |
| `BadLength` | 长度非法。 |
| `BadSequence` | 写入状态机序列不匹配。 |
| `CrcMismatch` | CRC 不匹配。 |
| `Busy` | 设备忙，可稍后重试。 |
| `NotReady` | 当前状态不允许。 |
| `StorageError` | DataFlash 读写失败。 |
| `InternalError` | 内部错误。 |

## 7. Power 关系

- Vendor/config 会话不作为特殊 power blocker。
- 如果满足普通 idle 条件，允许进入 `Suspend`。
- 收到新的 vendor packet 时唤醒并处理。
- 进入 `Sleep` 前如果 config dirty，必须先保存。

## 8. 已确认决策

- Frame header 保持完整，优先可调试性。
- `cmd` 使用 `u16`。
- Vendor response status 使用独立枚举。
- command id 按功能范围分段。
- 大 payload 命令使用显式 offset 分片，不做传输层隐式重组。
- `offset` 使用 byte offset，为后续乱序/重传预留。
- CH585 USBHS Compatibility HID 示例验证 HS 512-byte、FS 64-byte report/endpoint 能力。

## 9. 未决问题

- BLE/2.4G 的最大单包 payload 待 route 实现确认。
- `GetDeviceInfo` 的返回结构当前还是临时占位，后续应改为固定字段布局。
- `GetDiagnostics` 当前覆盖 protocol/vendor queue/event queue 的基础计数，后续可继续扩展 IMU / HID / config 错误统计。
