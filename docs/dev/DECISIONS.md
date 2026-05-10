# 已确认决策

这里记录已经拍板、但还没有完全沉淀进长期文档的决策

使用规则很简单：

- 这里记录结论，不记录讨论过程
- 结论稳定后回填到 `docs/` 对应长期文档
- 还没拍板的问题不要写在这里，放到 `OPEN_QUESTIONS.md`

## 当前仍保留在这里的过渡性结论

以下内容仍然更像实现阶段的过渡约定，暂时保留在这里：

- 当前可以继续保留 `power`、`route`、`config` 的模块骨架与部分 stub
- 某些 v1 范围虽然已经写入长期文档，但实现仍可能分阶段落地，具体差距继续看 `TASKLIST.md`
- `Suspend` 的设计语义已经明确为“**保持无线连接的浅休眠**”；在没有明确证据证明 **BLE connected** 场景下可通过某个 CH585 low-power 入口 **不断链** 之前，不得把项目 `Suspend` 直接映射到 `LowPower_Halt_WFE()` 或其他可能断链的芯片级低功耗模式

## 已回填说明

以下主题已经有明确长期入口，不再在这里展开重复列表：

- 文档分层与维护规则见 `../README.md` 与 `../STYLE.md`
- Power 规则见 `../POWER_STATE_MACHINE.md`
- Route 与 USB 规则见 `../ROUTE_STATE_MACHINE.md`
- FFI 与输入枚举规则见 `../FFI_ABI.md`
- Vendor/WebHID 协议规则见 `../VENDOR_PROTOCOL.md`
- IMU/profile 规则见 `../RESOURCE_PROFILE.md`
- BLE app 分层见 `../DESIGN.md`
