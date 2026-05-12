# Vendor/WebHID 协议实现状态

## 单包解析

已按完整 header 解析 `magic/version/flags/seq/cmd/status/offset/total_len/payload_len`。

当前仅接受：
- `fragment` 标志未置位
- `offset == 0`
- `total_len == payload_len`
- 单包总长度不超过当前 `CustomReport` 容量

## 已接入的命令

| ID | Command | 状态 |
| --- | --- | --- |
| `0x0000` | `Ping` | ✅ |
| `0x0001` | `GetProtocolInfo` | ✅ |
| `0x0002` | `GetDeviceInfo` | ✅ |
| `0x0100` | `GetConfigInfo` | ✅ |
| `0x0101` | `ReadConfig` | ✅ |
| `0x0102` | `WriteConfigBegin` | ✅ |
| `0x0103` | `WriteConfigChunk` | ✅ |
| `0x0104` | `WriteConfigCommit` | ✅ |
| `0x0105` | `WriteConfigAbort` | ✅ |
| `0x0106` | `SaveConfig` | ✅ |
| `0x0107` | `RestoreDefaults` | ✅ |
| `0x0201` | `GetRouteState` | ✅ |
| `0x0202` | `GetPowerState` | ✅ |
| `0x0300` | `GetDiagnostics` | ✅ |

## 未完成

- 多包分片重组（`crc16`）
- BLE Custom GATT / USB Custom HID transport backend 的进一步完善
