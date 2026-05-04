# Vendor/WebHID Protocol

本文定义 VoidPointer Vendor/WebHID 配置协议草案。当前是 Draft，用于统一 USBHS / BLE / 2.4G 三种配置通道的逻辑协议。

## 1. 目标

- 同一套逻辑命令可运行在 USBHS Vendor HID、BLE Vendor / custom HID、2.4G Vendor / custom channel 上。
- USBHS 作为首选高吞吐配置通道。
- BLE / 2.4G 根据各自 MTU 或私有链路能力进行分片。
- 协议层不依赖具体 transport 的物理包大小。

## 2. Transport 能力

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
| Framing | 分片、重组、序号、长度校验。 |
| Command | 解析 command id、request/response、错误码。 |
| Config service | 读取/写入配置、保存、恢复默认、diagnostics。 |

## 4. Packet framing 草案

为了兼容不同 transport，建议所有 route 使用相同逻辑 frame header：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `magic` | `u8` | 固定魔数，建议 `0xA5`。 |
| `version` | `u8` | 协议版本，初版 `1`。 |
| `flags` | `u8` | bit flags，例如 request/response/fragment。 |
| `seq` | `u8` | 请求序号，用于匹配 response。 |
| `cmd` | `u16` | command id。 |
| `status` | `u16` | response status；request 中填 0。 |
| `offset` | `u16` | 当前分片在完整逻辑 payload 中的 byte offset。v1 虽然使用 byte offset 字段，但仍要求顺序分片。 |
| `total_len` | `u32` | 完整 payload 长度。 |
| `payload_len` | `u16` | 本包 payload 长度。 |
| `payload` | bytes | 本包数据。 |
| `crc16` | `u16` | 多包分片时启用，覆盖完整逻辑 payload 或分片上下文；单包命令可不启用。 |

CRC16 策略：只在多包分片时启用。单包命令依赖 transport 校验、`magic`、`version`、`seq`、`payload_len` 和 command 语义检查。

分片顺序策略：v1 不支持乱序分片；多包 payload 必须按 `offset = 0, previous_offset + previous_payload_len, ...` 顺序发送。设备收到非预期 `offset` 时返回 `BadSequence`。`offset` 仍使用 byte offset，而不是 chunk index，以便后续版本支持乱序或重传时无需修改 header 语义。

### 4.1 当前代码实现状态

当前 Rust runtime 已实现 **单包子集**：

- 已按完整 header 解析 `magic/version/flags/seq/cmd/status/offset/total_len/payload_len`。
- 当前仅接受：
  - `fragment` 标志未置位
  - `offset == 0`
  - `total_len == payload_len`
  - 单包总长度不超过当前 `CustomReport` 容量
- 当前已接入的命令：
  - `Ping` (`0x0000`)
  - `GetProtocolInfo` (`0x0001`)
  - `GetDeviceInfo` (`0x0002`)
  - `GetConfigInfo` (`0x0100`)
  - `GetRouteState` (`0x0201`)
  - `GetPowerState` (`0x0202`)
  - `GetDiagnostics` (`0x0300`)
- 当前 runtime 已能在 `vp_core_poll()` 中完成：
  - vendor RX 入队后解析
  - 统一协议响应生成
  - 通过 route-aware `SendVendor` 命令回发响应

这意味着协议层已经从 transport 中抽离；后续 USB Custom HID、BLE Custom GATT、2.4G custom channel 只需要承载同一份 frame。

## 5. Command id allocation

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
| `0x0100` | `GetConfigInfo` | host → device | 查询配置版本、大小、CRC。 |
| `0x0101` | `ReadConfig` | host → device | 分片读取当前配置。 |
| `0x0102` | `WriteConfigBegin` | host → device | 开始写配置，声明总长度/CRC。 |
| `0x0103` | `WriteConfigChunk` | host → device | 写入配置分片。 |
| `0x0104` | `WriteConfigCommit` | host → device | 校验并应用配置，标记 save pending。 |
| `0x0105` | `WriteConfigAbort` | host → device | 放弃本次写入。 |
| `0x0106` | `SaveConfig` | host → device | 请求保存到 DataFlash。 |
| `0x0107` | `RestoreDefaults` | host → device | 恢复默认配置。 |
| `0x0200` | `ApplyRuntimeConfig` | host → device | 仅应用 runtime config，不立即保存。 |
| `0x0201` | `GetRouteState` | host → device | 查询 BLE / 2.4G / USB route 状态。 |
| `0x0202` | `GetPowerState` | host → device | 查询 power diagnostics 状态；不暴露 Rust 内部 `vp_power_state_t` ABI。 |
| `0x0300` | `GetDiagnostics` | host → device | 查询 debug counters / wake diagnostics。 |

## 6. Vendor response status

Vendor/WebHID 使用独立 response status，不复用底层 `vp_status_t`。原因是 Vendor 协议需要表达分片、序号、CRC 等协议层错误。

| Status | 说明 |
| --- | --- |
| `Ok` | 成功。 |
| `InvalidCommand` | 不支持的 command。 |
| `InvalidArgument` | 参数非法。 |
| `BadLength` | 长度非法。 |
| `BadSequence` | 分片序号或状态不匹配。 |
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

## 8. Confirmed decisions

- Frame header 保持完整，优先可调试性。
- `cmd` 使用 `u16`。
- Vendor response status 使用独立枚举。
- CRC16 只在多包分片时启用。
- command id 按功能范围分段：基础协议 `0x0000..0x00FF`，Config `0x0100..0x01FF`，Runtime/Route/Power `0x0200..0x02FF`，Diagnostics `0x0300..0x03FF`，experimental/debug `0x8000..0xFFFF`。
- frame header 中 `offset` 使用 byte offset。
- v1 不支持乱序分片；必须顺序发送。header 保留 byte offset 语义，为后续乱序/重传预留。
- CH585 USBHS 硬件依据见 `CH585_NOTES.md`：Compatibility HID 示例验证 HS 512-byte、FS 64-byte report/endpoint 能力。

## 9. Open questions

- BLE/2.4G 的最大单包 payload 待 route 实现确认。
- 多包分片重组、`crc16`、配置写会话状态机仍待实现。
- `GetDeviceInfo` 的返回结构当前还是临时占位，后续应改为固定字段布局。
- `GetConfigInfo` 当前已返回 config version / dirty flag / payload size / CRC 占位字段；待真实 `DeviceConfig` 与序列化/CRC 接入后再填充真实长度与 CRC。
- `GetDiagnostics` 当前覆盖 protocol/vendor queue/event queue 的基础计数，后续可继续扩展 IMU / HID / config 错误统计。
