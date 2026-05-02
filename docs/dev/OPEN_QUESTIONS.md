# Open Questions

本文只记录需要项目负责人继续拍板的问题。已确认结论集中维护在 `DECISIONS.md`；硬件资料待验证项集中维护在 `../CH585_NOTES.md`；实现顺序集中维护在 `TASKLIST.md`。

## 当前需要项目负责人拍板的问题

暂无。

## 当前不需要拍板但需要后续验证的事项

这些不是产品策略 blocker，后续在实现或实测阶段处理，并回填到对应文档：

| 项目 | 记录位置 | 说明 |
| --- | --- | --- |
| USBHS remote wake 完整流程 | `../CH585_NOTES.md` | 已找到相关寄存器线索，但未整理完整项目化流程。 |
| BLE/RF connected 状态下最低可用 low-power API | `../CH585_NOTES.md` / `../POWER_STATE_MACHINE.md` | 需要结合 BLE stack/HAL 和实测确认。 |
| DataFlash erase granularity | `../CH585_NOTES.md` / `../CONFIG_SPEC.md` | 初版保守按 4KB erase block 设计；是否可 256-byte erase 需实机确认。 |
| RTC 在不同 power plan 下的保持条件 | `../CH585_NOTES.md` | RTC wake 示例存在，但 power-domain 保持矩阵需验证。 |
| 普通 GPIO IRQ 寄存器级 both-edge 能力 | `../CH585_NOTES.md` / `TASKLIST.md` | StdPeriph 未暴露；编码器 v1 用平台模拟 + Rust 正交状态机兜底。 |

## 已关闭的问题

已关闭问题不在本文重复列出。需要查确认结论时看 `DECISIONS.md`。
