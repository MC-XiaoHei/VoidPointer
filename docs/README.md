# 文档导航

本文档是 VoidPointer 项目所有设计文档的入口。按建议顺序阅读，可以快速理解全局设计。


## 建议阅读顺序

初次接触项目，推荐按以下顺序阅读：

| 步骤 | 文档 | 内容 |
|------|------|------|
| 1 | [DESIGN.md](DESIGN.md) | 产品目标、硬件背景和总体分层架构 |
| 2 | [SETUP.md](SETUP.md) | 本地开发环境、工具链和构建流程 |
| 3 | [POWER_STATE_MACHINE.md](POWER_STATE_MACHINE.md) | 电源状态机 |
| 4 | [ROUTE_STATE_MACHINE.md](ROUTE_STATE_MACHINE.md) | BLE/USB/2.4G 路由状态机 |
| 5 | [CONFIG_SPEC.md](CONFIG_SPEC.md) | 配置结构与存储规则 |
| 6 | [VENDOR_PROTOCOL.md](VENDOR_PROTOCOL.md) | WebHID 配置协议 |


## 文档分类

### 长期设计文档

这些文档描述项目的稳定事实，进入维护期后仍然应该保留。

| 文档 | 用途 |
|------|------|
| [DESIGN.md](DESIGN.md) | 产品目标、硬件选型和总体设计 |
| [SETUP.md](SETUP.md) | 本地开发环境和构建流程 |
| [POWER_STATE_MACHINE.md](POWER_STATE_MACHINE.md) | 电源状态机 |
| [ROUTE_STATE_MACHINE.md](ROUTE_STATE_MACHINE.md) | BLE、2.4G、USB 路由状态机 |
| [CONFIG_SPEC.md](CONFIG_SPEC.md) | 配置结构与存储规则 |
| [VENDOR_PROTOCOL.md](VENDOR_PROTOCOL.md) | Vendor/WebHID 协议 |
| [RESOURCE_PROFILE.md](RESOURCE_PROFILE.md) | IMU、I2C、HID 等资源与 Profile 依据 |
| [LIGHTING.md](LIGHTING.md) | 灯光模式、触发条件和播放参数 |
| [TEST_PLAN.md](TEST_PLAN.md) | 测试计划与验收标准 |
| [STYLE.md](STYLE.md) | 仓库级注释、文档和提交规范 |

### 开发期文档

这些文档服务于当前开发阶段，不承担长期规格说明职责。

| 文档 | 用途 |
|------|------|
| [TASKLIST.md](TASKLIST.md) | 当前实现状态、任务和完成标准 |
