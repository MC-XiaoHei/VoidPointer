# VoidPointer 项目白皮书

## 1. 项目概述

VoidPointer 是一款集成了高精度空中鼠标、激光指示、便携存储于一体的多功能“三模”演示器。采用变形尾插设计与非线性映射算法，为用户提供极致的便携性与精准的操控体验。

## 2. 硬件架构方案

### 2.1 核心控制与传感

- **主控 MCU:** **CH585**，负责三模通信、按键处理及姿态解算。
- **IMU:** **LSM6DSV**，高精度六轴传感器，支持低功耗运行模式。

### 2.2 USB 拓扑与存储

- 内置 **CH334 USB Hub**。
- 一路连接 **GL823K** TF 卡读卡器（仅供用户使用）。
- 一路连接 **CH585 MCU**。

### 2.3 电源与功耗管理

- **BMS:** **ETA6005** (NVDC 架构)，配备 800mAh 电池，充电电流 400mA。
- **稳压:** **TPS63805** Buck-Boost 稳压至 3.3V VCC。
- **三级电源状态策略:** 系统不设物理电源总开关，完全依靠智能休眠管理功耗。固件电源状态统一命名为 `Active` / `Suspend` / `Sleep`：
   - **Active:** 设备处于运动状态、按键操作、HID 发送、配置会话或其他需要即时响应的场景，全功能运行。
   - **Suspend:** 设备静置一定时间后进入。IMU 使用 accelerometer-based wake-up / activity-inactivity / stationary-motion 等低功耗中断检测本体移动，同时**保持无线连接** (BLE/2.4G)，确保用户拿起时“秒唤醒”零延迟响应。Suspend 默认不依赖 SFLP 角度检测，SFLP 仅在空鼠 active/arming 时启用。
   - **Sleep:** **仅在无无线连接（断开连接）且设备静置时进入**。此时射频功能关闭，IMU 关闭 SFLP 与 gyro，仅保留低功耗 accelerometer wake-up/significant-motion 检测，实现类似漏电级别的极低功耗。

### 2.4 UI 与状态指示

- **充电状态灯 (双色 LED):** 充电中红+绿（黄），充满电（仅绿）。
- **物理模式开关:** 两段式物理拨动开关，用于硬切换 BLE (蓝牙) 与 2.4G 模式。

## 3. 软件架构设计 (C/Rust 混合驱动)

VoidPointer 在固件层面采用了面向极致低功耗的异步、无锁架构：

- **混合编译与自动化 FFI:** 采用 `corrosion-rs` 框架管理 C (CMake) 与 Rust 的混合编译。FFI 边界完全由工具链自动化生成，确保内存布局与跨语言调用的绝对安全。
- **分层设计:**
   - **硬件层 (C 语言):** 只负责被硬件中断/协议栈事件唤醒、采样最小硬件事实，并提供 GPIO、EXTI、Timer、I2C、IMU、HID、RF/USB、DataFlash、Sleep 等底层硬件 API。C 层不持有产品业务状态，不解释按键/滚轮/电源/连接策略。
   - **逻辑层 (Rust 语言):** 负责所有产品逻辑，包括输入状态机、按键消抖、滚轮解析、IMU 姿态解算、非线性映射算法、HID 报告聚合、连接路由、电源状态机、配置系统与 WebHID 命令解析。Rust 根据状态机决策后，再通过 C API 执行具体硬件动作。
   - **BLE app glue 分层:** 当前 BLE 外围已进一步拆成 `ble_hid_app.c`、`ble_gap_policy.c` 和 `ble_hid_app_config.h` 三层：`ble_hid_app.c` 负责 TMOS task/event glue、HID callback 注册与 profile bring-up，`ble_gap_policy.c` 集中维护 BLE connection handle、advertising allow、GAP state 与断连/复播策略，`ble_hid_app_config.h` 负责集中定义默认参数、pairing/bonding 参数、连接参数与策略时序常量，避免把连接策略和调参常量继续散落在多个 C 文件里。
   - **事件驱动边界:** C 层在 EXTI、Timer、IMU INT、I2C 完成、BLE/USB/2.4G 栈事件等唤醒点调用 Rust 导出的事件入口；Rust 更新状态机后，可反向调用 C API 关闭/开启中断、启动定时器、请求 I2C 读取、发送 HID 报告或进入低功耗。
- **异步数据流:**
   - **中断驱动 IMU 数据流：** LSM6DSV 通过 INT 引脚触发 CH585 GPIO 中断；Rust 根据当前 motion/power 状态决定是否请求 C 启动 I2C FIFO 异步读取。Active motion 使用 SFLP FIFO 姿态数据；Suspend/Sleep 默认使用 LSM6DSV accelerometer-based wake-up/activity-inactivity/significant-motion 中断作为唤醒源，不依赖 SFLP 角度检测。I2C 完成后 C 将最新 SFLP 姿态原始样本回调给 Rust，Rust 使用 latest-sample 缓存驱动姿态映射。
   - **中断驱动输入系统：** 所有实体输入均以硬件中断作为唤醒源，主循环不做按键轮询；中断链路只负责极速采样、推进状态机或启动定时裁决，最终由 Rust 逻辑层发布稳定输入快照与离散事件。事件时间戳统一使用 CH585 RTC tick/millis，精度满足消抖、活动超时与 `Suspend`/`Sleep` 判定需求。
   - **滚轮编码器状态机：** A/B 两相配置为双边沿外部中断。任意跳变立即读取当前 A/B 状态，将“旧状态 + 新状态”拼接为 4-bit 索引查表，得到本次微观跳变方向（`+1` / `-1` / `0`）。方向结果仅累计到 `internal_phase` 内部相位缓冲池；只有相位达到 `+4` 或 `-4`，即跨越一个完整物理刻度时，才向上层发布 `wheel +1/-1` 事件，并扣除对应相位。卡在物理刻度边缘的高频抖动会在迟滞区间内相互抵消，不会触发滚轮乱跳。
   - **实体按键与模式开关消抖：** 左/右/中/Action/Laser 等瞬时微动按键，以及物理模式开关，统一建模为低有效二态输入。Rust 维护稳定态“激活/未激活”，并按当前稳定态请求下一次相反语义转换：稳定高电平时请求 `Falling`，平台映射为 CH585 `LowLevel`；稳定低电平时请求 `Rising`，平台映射为 CH585 `HighLevel`。GPIOA ISR 或 GPIOA 待处理服务只根据硬件 `IF & EN` 识别输入、读取当前低有效电平、屏蔽该 pin，并把事件入队；1ms 消抖下半部确认稳定态后发布按下/抬起或开关开/关，并重新 arm 下一相反电平。该实现仍以 GPIOA 中断标志为事件源，主循环不通过 GPIO 扫描制造输入事件；主循环仅补服务 CH585 上已锁存但 PFIC 未再次派发的 GPIOA 待处理中断标志。
   - **事件队列与最新样本缓存:** ISR 到 Rust bottom-half 的异步事件使用固定容量 SPSC 事件队列传递，避免在中断上下文执行重逻辑；IMU 姿态数据采用 latest-sample 缓存策略，FIFO 中多帧样本以低延迟为优先，允许丢弃旧帧并仅保留最新有效姿态。
- **调度模型:** 当前稳定方案不是“纯 TMOS task 单点驱动”，而是 **`Main_Circulation()` + TMOS + runtime service 的混合调度**。主循环固定执行 `RuntimeTask_Service()` → `TMOS_SystemProcess()` → `RuntimeTask_Service()`。其中 `RuntimeTask_Service()` 只做三类轻量工作：补服务已锁存但未再次派发的 GPIOA 中断、驱动 1ms debounce 软时基、以及在 `runtime_poll_request_pending` 置位时调用 `vp_core_poll()`。`RuntimeTask_RequestPoll()` 默认只置位 `runtime_poll_request_pending`；`RuntimeTask_RequestPollAfter(ms)` 在 `ms>0` 时通过 TMOS timer 触发 `RUNTIME_CORE_POLL_EVT`，再把 pending 位置位。也就是说，TMOS 负责协议栈推进和延时唤醒，`vp_core_poll()` 的实际执行入口仍由主循环侧的 `RuntimeTask_Service()` 合并调度。这套结构是为兼容 CH585/TMOS/BLE 栈时序、GPIO 锁存补服务和 debounce 软时基后反复收敛下来的稳定方案。
- **路由与重试约束:** 连接状态与报告路径就绪状态必须分离建模。以 BLE 为例，`connected` 只代表链路存在，不代表 HID 输入路径已经 secure/notify-ready；只有 `input-ready` 才允许 BLE 参与 HID 路由选择。此外，当当前 route 不存在或尚未 ready 时，runtime 必须主动收敛本次 report 尝试，不能让 `dirty.report` 持续驱动 `vp_core_poll()` 自旋重调度，否则会干扰 BLE bonded reconnect 的安全恢复时序。对于 vendor pending tx，则要与 mouse report 分开处理：前者保留待发包并用 backoff retry，后者在 route not-ready 时直接收敛等待下一次真实路由事件。

## 4. 交互逻辑

- **Action 键 (核心姿态映射):**
   - **记录基准:** 按下瞬间记录当前空间姿态作为基准零点。
   - **非线性映射:** 按住 Action 键并倾斜设备，设备偏离基准点的**角度偏移量**将被直接映射为鼠标光标的**移动速度**。
      - *核心逻辑:* 该映射完全基于绝对的角度偏移，与摆动速率（角速度）无关。无论用户挥动多快或多慢，只要保持在一个相同的偏转角度，光标的移动速度即是恒定的。
      - 偏转角度越大，经由非线性加速曲线换算后的光标速度越快（适合跨屏移动）；角度越小，速度越慢（适合精准微操）。
   - **松开即停:** 松开 Action 键光标立即停止，无惯性漂移。
- **中键增强逻辑:** 按住中键同样触发上述空间姿态映射逻辑，高度兼容特定专业软件（如 CAD、3D 建模软件、大画幅设计工具）的中键平移与拖拽操作习惯。
- **按键配置:** 实体左/右/中键 + 机械编码器滚轮 + 独立 Laser 激光键。
- **输入消抖策略:**
   - **滚轮编码器:** 采用“正交状态机 + 迟滞相位缓冲池”机制。A/B 两相的每一次有效格雷码跳变都被记录为微步，正反向抖动会在 `internal_phase` 中自然抵消；仅当累计满一个完整物理刻度时才结算为滚轮事件。该方案不设置延时屏蔽窗口，既能抑制边缘抖动导致的网页上下乱跳，也能避免高速滚动时丢步。
   - **实体按键与模式开关:** 普通微动按键和自锁拨动开关统一使用“二态输入 + 电平 EXTI + debounce”机制。平台层不依赖 CH585 下降沿锁存；Rust 按稳定态重新 arm 相反电平，因此普通微动按键是会自动回到未激活态的二态输入，自锁开关是保持激活/未激活态的二态输入。只有连续稳定采样达到阈值时才发布状态变化。

## 5. 连接特性

- **三模支持:** 2.4G、BLE、有线。
- **有线防误触 / 路由优先级:** 当前默认策略为 **wired priority**。USB 进入 `Configured` 后，空中鼠标报告仅走 USB，并主动关闭 BLE 广播、断开现有 BLE 连接；USB 退出 `Configured` 后，再恢复 BLE 广播与无线待机能力。后续如需改为“插线但保留无线待命”，应通过明确的 route policy 配置实现，而不是让多条路由在未收敛状态下并存。

## 6. 工业设计与配件

- **变形尾插:**
   - 闭合态：USB-C 母口（有线连接/充电）。
   - 分离态：USB-A 公口（直连内部 Hub，直插电脑变为有线鼠标/读卡器）。
- **超微型 2.4G 接收器 (Dongle):**
   - **核心方案:** 采用 **CH592D** 主控搭配 **TLV70033DDCR** (LDO) 供电。
   - **极致体积:** 采用 0.8mm 厚度的 PCB 直接作为 USB 金手指，结合成熟的商业微型外壳，实现插入电脑后近乎隐形的极限尺寸。

## 7. 配置与驱动生态

为了提供极致的跨平台体验，VoidPointer 摒弃了传统的本地驱动软件，采用全免驱的轻量化设计方案：

- **WebHID 网页端配置:** 采用现代浏览器的 WebHID API 技术开发可视化配置后台。用户无需下载、安装任何软件，只需打开专属网页，授权连接后即可对鼠标进行全方位自定义设置（如：修改非线性加速曲线、自定义按键映射、调整休眠等待时间、设定有线防误触逻辑等）。
- **板载内存 (DataFlash 存储):** 充分利用 CH585 MCU 内部的 DataFlash 作为独立的用户配置存储区。用户在 WebHID 页面中修改的所有参数和宏设定，均会直接写入设备本体。无论更换何种操作系统或电脑，配置“跟鼠走”，实现真正的即插即用体验。
