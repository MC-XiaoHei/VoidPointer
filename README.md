# VoidPointer

VoidPointer 是一个面向 CH585 平台的混合固件项目，使用 C 处理硬件与协议栈边界，使用 Rust 承担输入、姿态、路由、电源和配置等核心业务逻辑

## 仓库结构

- `core/`：Rust 核心逻辑
- `platform/`：CH585 平台层、BLE/USB glue、FFI 绑定
- `docs/`：长期文档、文档导航和仓库维护规范
- `tools/`：辅助脚本与工具

## 建议阅读顺序

1. `docs/README.md`
2. `docs/DESIGN.md`
3. `docs/SETUP.md`
4. `docs/FFI_ABI.md`
5. `docs/POWER_STATE_MACHINE.md` 与 `docs/ROUTE_STATE_MACHINE.md`
6. `docs/STYLE.md`

## 代码约定

- 注释优先解释约束、边界和为什么这样做
- 代码中的长期事实应回填到 `docs/`，不要散落在临时笔记里
- 开发期决策、任务和待确认问题统一放在 `docs/dev/`
- 注释与文档风格约定见 `docs/STYLE.md`
- Git 提交消息使用简短的英文，尽可能让提交短小
