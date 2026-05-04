# 文档导航

这份导航只回答三件事：先看什么、长期文档放哪里、开发期信息放哪里

## 第一次进入仓库时先看什么

1. `DESIGN.md`：产品目标、硬件背景和整体约束
2. `SETUP.md`：本地环境、工具链和构建方式
3. `FFI_ABI.md`：C 与 Rust 的边界、调度约束和调用规则
4. `POWER_STATE_MACHINE.md` 与 `ROUTE_STATE_MACHINE.md`：运行期最重要的两个状态机
5. `CONFIG_SPEC.md` 与 `VENDOR_PROTOCOL.md`：配置存储和配置通道协议

## 长期文档

长期文档描述稳定事实，项目进入维护期后仍然应该保留

| 文档 | 用途 |
| --- | --- |
| `DESIGN.md` | 产品目标、硬件选型和总体设计 |
| `SETUP.md` | 本地开发环境和构建流程 |
| `FFI_ABI.md` | C/Rust FFI 边界和调用约束 |
| `POWER_STATE_MACHINE.md` | 电源状态机 |
| `ROUTE_STATE_MACHINE.md` | BLE、2.4G、USB 路由状态机 |
| `CONFIG_SPEC.md` | 配置结构与存储规则 |
| `VENDOR_PROTOCOL.md` | Vendor/WebHID 协议 |
| `RESOURCE_PROFILE.md` | IMU、I2C、HID 等资源与 profile 依据 |
| `CH585_NOTES.md` | 芯片相关依据、实测点和注意事项 |
| `TEST_PLAN.md` | 验收范围和测试方法 |
| `STYLE.md` | 仓库级注释、文档和维护风格规范 |

## 开发期文档

开发期文档只服务当前实现，不承担长期规格说明职责

| 文档 | 用途 |
| --- | --- |
| `dev/DECISIONS.md` | 已确认但还没完全回填到长期文档的结论 |
| `dev/TASKLIST.md` | 当前实现任务、差距和完成标准 |
| `dev/OPEN_QUESTIONS.md` | 仍然需要拍板的问题 |

## 为什么要分成 `docs/` 和 `docs/dev/`

这样分不是为了看起来整齐，而是为了把“项目事实”和“开发过程”拆开

- `docs/` 面向项目本身，记录长期有效的设计、接口、状态机、规格和验证依据
- `docs/dev/` 面向当前开发，记录任务、阶段性决策、待确认问题和协作约定
- 稳定结论应该从 `docs/dev/` 回填到 `docs/`
- 如果一份文档在项目进入维护期后仍然应该保留，它更可能属于 `docs/`
- 如果一份文档主要回答“现在做到哪了”或“这周怎么协作”，它更可能属于 `docs/dev/`

## 文档维护规则

- 稳定结论写进长期文档，不长期停留在 `dev/`
- `dev/TASKLIST.md` 只管任务，不代替规格说明
- `dev/OPEN_QUESTIONS.md` 只保留真正阻塞推进的问题
- 同一件事只保留一个主入口，其他文档只放链接和摘要
- 代码注释只解释局部约束，跨模块规则统一回填到文档
