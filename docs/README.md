# 文档导航

本文档是 VoidPointer 项目所有设计文档的入口。按建议顺序阅读，可以快速理解全局设计。

---

## 建议阅读顺序

初次接触项目，推荐按以下顺序阅读：

| 步骤 | 文档 | 内容 |
|------|------|------|
| 1 | [DESIGN.md](DESIGN.md) | 产品目标、硬件背景和总体分层架构 |
| 2 | [SETUP.md](SETUP.md) | 本地开发环境、工具链和构建流程 |
| 3 | [FFI_ABI.md](FFI_ABI.md) | C 与 Rust 的边界、调度约束和调用规则 |
| 4 | [POWER_STATE_MACHINE.md](POWER_STATE_MACHINE.md) | 电源状态机（运行期最重要的状态机之一） |
| 5 | [ROUTE_STATE_MACHINE.md](ROUTE_STATE_MACHINE.md) | BLE/USB/2.4G 路由状态机（运行期最重要的状态机之二） |
| 6 | [CONFIG_SPEC.md](CONFIG_SPEC.md) | 配置结构与存储规则 |
| 7 | [VENDOR_PROTOCOL.md](VENDOR_PROTOCOL.md) | WebHID 配置协议 |

---

## 文档分类

### 长期设计文档

这些文档描述项目的稳定事实，进入维护期后仍然应该保留。

| 文档 | 用途 |
|------|------|
| [DESIGN.md](DESIGN.md) | 产品目标、硬件选型和总体设计 |
| [SETUP.md](SETUP.md) | 本地开发环境和构建流程 |
| [FFI_ABI.md](FFI_ABI.md) | C/Rust FFI 边界和调用约束 |
| [POWER_STATE_MACHINE.md](POWER_STATE_MACHINE.md) | 电源状态机 |
| [ROUTE_STATE_MACHINE.md](ROUTE_STATE_MACHINE.md) | BLE、2.4G、USB 路由状态机 |
| [CONFIG_SPEC.md](CONFIG_SPEC.md) | 配置结构与存储规则 |
| [VENDOR_PROTOCOL.md](VENDOR_PROTOCOL.md) | Vendor/WebHID 协议 |
| [RESOURCE_PROFILE.md](RESOURCE_PROFILE.md) | IMU、I2C、HID 等资源与 Profile 依据 |
| [CH585_NOTES.md](CH585_NOTES.md) | 芯片相关实测点和注意事项 |
| [TEST_PLAN.md](TEST_PLAN.md) | 测试计划与验收标准 |
| [STYLE.md](STYLE.md) | 仓库级注释、文档和提交规范 |

### 开发期文档

这些文档服务于当前开发阶段，不承担长期规格说明职责。

| 文档 | 用途 |
|------|------|
| [dev/DECISIONS.md](dev/DECISIONS.md) | 已确认但尚未完全回填到长期文档的结论 |
| [dev/TASKLIST.md](dev/TASKLIST.md) | 当前实现状态、任务和完成标准 |
| [dev/OPEN_QUESTIONS.md](dev/OPEN_QUESTIONS.md) | 仍待拍板的问题 |
| [dev/PROTOCOL_IMPL_STATUS.md](dev/PROTOCOL_IMPL_STATUS.md) | Vendor 协议实现状态 |

### 为什么分成 `docs/` 和 `docs/dev/`

这样分不是为了看起来整齐，而是为了把**项目事实**和**开发过程**拆开：

- `docs/` —— 记录长期有效的设计、接口、状态机和验证依据
- `docs/dev/` —— 记录任务、阶段性决策和待确认问题
- 稳定结论应及时从 `docs/dev/` 回填到 `docs/`

---

## 文档维护约定

- 同一件事只保留一个主入口，其他文档只放链接和摘要
- 代码注释只解释局部约束，跨模块规则统一回填到文档
- 如果一份文档在项目进入维护期后仍然应该保留，它属于 `docs/`
- 如果一份文档主要回答"现在做到哪了"，它属于 `docs/dev/`
