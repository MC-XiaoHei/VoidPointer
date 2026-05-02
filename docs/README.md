# VoidPointer 文档索引

本目录维护 VoidPointer 固件项目的长期规格文档、开发期文档和本地开发环境说明。

文档按生命周期分为两类：

- **长期文档**：项目完成后仍应保留，用于说明产品设计、固件架构、接口约定、开发环境和验证方法。
- **开发期文档**：用于实现阶段的任务跟踪、待决问题和临时决策记录；开发完成并回填到长期文档后可以删除或归档。

## 推荐阅读顺序

首次了解项目时建议按以下顺序阅读：

1. `DESIGN.md`：产品目标、总体背景和设计约束。
2. `SETUP.md`：本地工具链、CMake preset、Zed/clangd 和下载流程。
3. `FFI_ABI.md`：C/Rust FFI ABI、调度约定、所有权和上下文规则。
4. `POWER_STATE_MACHINE.md` / `ROUTE_STATE_MACHINE.md`：核心运行状态机。
5. `CONFIG_SPEC.md` / `VENDOR_PROTOCOL.md`：配置存储和 Vendor/WebHID 协议。
6. `RESOURCE_PROFILE.md` / `CH585_NOTES.md`：硬件依据、profile 起点和芯片注意事项。
7. `TEST_PLAN.md`：验收测试计划。
8. `dev/TASKLIST.md` / `dev/OPEN_QUESTIONS.md`：当前开发状态和待确认问题。

## 长期文档

这些文档描述项目最终应保留的信息。

| 文档 | 状态 | 说明 |
| --- | --- | --- |
| `DESIGN.md` | Reference | 产品白皮书、总体背景和设计目标。 |
| `SETUP.md` | Active | 本地开发环境、工具链、Zed task 和 clangd 配置说明。 |
| `FFI_ABI.md` | Active | 目标 C/Rust FFI ABI、上下文规则、所有权和 checklist。 |
| `POWER_STATE_MACHINE.md` | Draft | `Active` / `Suspend` / `Sleep` 电源状态机。 |
| `ROUTE_STATE_MACHINE.md` | Draft | BLE / 2.4G / USB route、USB state mapping 和 `usb_mouse_policy`。 |
| `CONFIG_SPEC.md` | Draft | 配置结构、DataFlash 双槽存储和保存流程。 |
| `VENDOR_PROTOCOL.md` | Draft | USBHS / BLE / 2.4G Vendor/WebHID 配置协议。 |
| `RESOURCE_PROFILE.md` | Draft | LSM6DSV、I2C、BLE HID、USB HID 等资源依据和 profile 起点。 |
| `CH585_NOTES.md` | Draft | CH585 USBHS、GPIO/EXTI、低功耗、RTC、DataFlash 的硬件依据和验证项。 |
| `TEST_PLAN.md` | Draft | 输入、motion、HID、power、config 的验收测试计划。 |

## 开发期文档

这些文档位于 `dev/`，用于开发阶段协作。开发完成后，应将仍然有效的信息回填到长期文档，再删除或归档对应开发期文档。

| 文档 | 状态 | 说明 |
| --- | --- | --- |
| `dev/DECISIONS.md` | Active | 已确认结论和决策记录。结论稳定后应同步到对应长期文档。 |
| `dev/TASKLIST.md` | Active | 实现任务、当前代码差距、文件拆分建议和完成标准。 |
| `dev/OPEN_QUESTIONS.md` | Active | 当前仍需项目负责人拍板的问题；为空时表示暂无策略 blocker。 |

## 文档维护规则

- 长期文档应描述稳定设计和接口，不记录临时讨论过程。
- 开发期问题先写入 `dev/OPEN_QUESTIONS.md`，确认后同步到 `dev/DECISIONS.md`。
- 已确认并稳定的设计结论，应回填到对应长期文档，例如 `FFI_ABI.md`、`CONFIG_SPEC.md` 或状态机文档。
- `dev/TASKLIST.md` 只维护开发任务，不应替代规格文档。
- 硬件依据和实测 TBD 放在 `CH585_NOTES.md` 或 `RESOURCE_PROFILE.md`，不要在实现代码中凭空假设。
- 新增文档时先判断生命周期：长期保留放在 `docs/`，开发期临时协作放在 `docs/dev/`。
