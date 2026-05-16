# VoidPointer 设计总览

产品目标、硬件背景和总体分层架构。实现状态和任务拆分见 `TASKLIST.md`。

## 1. 产品目标

VoidPointer 是一款集成空中鼠标、激光指示和便携存储能力的三模演示器

目标不是堆功能，而是在以下几件事上同时成立：

- 便携
- 低功耗
- 姿态控制稳定
- 有线与无线路由行为可预测
- 无需本地驱动即可完成配置

## 2. 硬件边界

### 2.1 核心器件

- 主控 MCU：`CH585`
- IMU：`LSM6DSV`
- USB Hub：`CH334`
- TF 读卡器：`GL823K`
- BMS：`ETA6005`
- 3.3V 稳压：`TPS63805`

### 2.2 USB 拓扑

内置 Hub 把 USB 分成两路：

- 一路给 TF 读卡器
- 一路给 `CH585`

这意味着设备既可以表现为 HID，也可以同时保留存储路径

### 2.3 电源边界

系统没有物理总电源开关，功耗完全依赖固件状态机管理

固件统一使用三个电源状态：

- `Active`
- `Suspend`
- `Sleep`

详细行为见 `POWER_STATE_MACHINE.md`

## 3. 软件分层

### 3.1 总体原则

- C 只负责硬件事实和平台动作
- Rust 只负责业务状态和决策
- C 不持有产品业务状态
- Rust 不直接碰寄存器或协议栈细节

### 3.2 C 层职责

C 层负责：

- GPIO 与 EXTI
- Timer、RTC、TMOS
- I2C 与 IMU 访问
- BLE、USB 和后续 2.4G 平台 glue
- HID 与 vendor report 的底层收发
- DataFlash 与低功耗平台动作

C 层只上报事实，不解释产品语义

### 3.3 Rust 层职责

Rust 层负责：

- 输入状态机
- 按键与模式开关消抖
- 编码器解析
- 姿态缓存与映射
- HID 报告聚合
- 路由策略
- 电源状态机
- 配置系统
- Vendor/WebHID 命令解析

Rust 根据状态机做出决策，再通过 FFI 调 C 执行动作

### 3.4 BLE glue 分层

当前 BLE app 继续保持三层边界，并统一使用 `ble_hid_app.*` 命名：

- `ble_hid_app.c`：TMOS task、profile bring-up、HID callback glue
- `ble_gap_policy.c`：连接句柄、广播开关、GAP 状态和断连策略
- `ble_hid_app_config.h`：默认参数、配对参数、连接参数和策略常量

这样做的目的，是把协议 glue、连接策略和调参常量分开维护

## 4. 事件与调度模型

### 4.1 事件原则

所有实体输入都优先走中断唤醒
IMU 中断只负责唤醒主循环，不直接承载姿态数据语义
主循环不靠 GPIO 轮询制造业务事件

### 4.2 输入模型

- 按键与模式开关使用低有效二态输入模型
- 平台层按 Rust 请求的语义转换重新 arm 下一次中断
- 编码器使用正交状态机，只在累计满一个物理刻度后发布滚轮事件

### 4.3 IMU 模型

- `LSM6DSV` 中断只负责唤醒
- FIFO 读取由 Rust 在 bottom-half 决定是否发起
- 姿态数据使用 latest-sample 缓存，不追求保留每一帧历史

### 4.4 调度模型

当前稳定方案是 `Main_Circulation()`、TMOS 和 runtime service 的混合调度

主循环固定执行：

- `RuntimeTask_Service()`
- `TMOS_SystemProcess()`
- `RuntimeTask_Service()`

其中 runtime service 只做三类轻量工作：

- 补服务已锁存但未再次派发的 GPIOA 中断
- 驱动 1ms debounce 软时基
- 在 poll pending 时调用 `vp_core_poll()`

TMOS 负责协议栈推进和延时唤醒，Rust bottom-half 的实际执行入口仍由主循环侧统一合并

## 5. 输入与姿态交互

### 5.1 Action 键

按下 `Action` 键时记录姿态基准
按住期间，设备相对基准姿态的角度偏移会映射为光标速度

关键约束是：

- 映射基于姿态偏移，不基于角速度
- 相同偏转角度应得到稳定的速度输出
- 松开按键后立即停止，不保留惯性

### 5.2 中键复用

按住中键时也可进入相同的姿态映射路径
用于兼容 CAD、3D 建模和大画幅设计软件的中键拖拽习惯

### 5.3 编码器与按键消抖

- 编码器依赖正交状态机和相位累计，不依赖延时屏蔽窗口
- 普通按键与自锁开关都使用二态输入加 debounce
- 平台层不靠扫描电平主动制造输入事件

## 6. 路由策略

支持三条路由：

- BLE
- 2.4G
- USB

当前默认采用 wired priority：

- USB 进入 `Configured` 后，鼠标报告只走 USB
- 同时关闭 BLE 广播并断开现有 BLE 连接
- USB 退出 `Configured` 后，再恢复 BLE 广播与无线待机能力

详细状态规则见 `ROUTE_STATE_MACHINE.md`

## 7. 配置与生态

配置通道目标是 WebHID，而不是本地驱动软件

核心约束如下：

- 用户不需要安装平台专用软件
- 配置应保存在设备侧 DataFlash
- 配置跟设备走，不跟主机走

协议细节见 `VENDOR_PROTOCOL.md`，存储规则见 `CONFIG_SPEC.md`

## 8. 文档边界

本文档只回答三类问题：

- 这个产品想做成什么
- 总体架构怎么分层
- 哪些约束必须长期保持

更细的状态机、ABI、资源参数和测试规则分别放在对应专项文档中，不在这里重复展开
