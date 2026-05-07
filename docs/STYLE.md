# 注释、文档与提交风格

这份文档只服务当前仓库协作，用来约束注释怎么写、文档怎么放、什么时候该把内容从 `docs/dev/` 回填到 `docs/`，以及 commit message 怎么写

## 一句话原则

- 代码注释解释约束、边界和原因，不解释显而易见的动作
- `docs/` 记录长期事实
- `docs/dev/` 记录当前开发活动
- 稳定结论要从 `docs/dev/` 回填到 `docs/`

## 注释风格

- 优先使用中文
- 尽量短
- 尽量不要句中换行
- 尽量不要尾随句号
- 优先解释为什么这样做、哪里不能乱动、什么情况下会出问题
- 能从命名和类型直接看出来的事实尽量不再写

### 更适合保留的注释

- 中断与主循环的职责边界
- 所有权、可重入性和调用上下文限制
- 状态机为什么这样收敛
- 为什么这里要退避、缓存、延迟或丢弃一次尝试
- 芯片、协议栈或硬件行为带来的特殊约束

### 应尽量避免的注释

- 逐行翻译代码在做什么
- 重复函数名或变量名已经表达清楚的事实
- 把开发过程、试错记录写进源码注释
- 本该进入长期文档的跨模块规则只留在局部注释里

## 日志风格

### 一句话原则

- 日志先服务 bring-up、联调和状态机排错，不拿来代替注释和文档
- 能靠返回值、状态机和类型表达清楚的正常流程，不要到处加日志
- 日志格式在 Rust 和 C 两侧保持统一，可直接混合阅读
- 先判断这条日志是否值得长期保留，再决定它的级别和文案

### 日志格式

Rust 统一使用 `log` crate，C 统一使用仓库内日志宏

统一输出格式：

- Rust：`[LEVEL] [module] message`
- C：`[LEVEL] [C:module] message`

约定如下：

- `LEVEL` 只使用 `ERROR`、`WARN`、`INFO`、`DEBUG`
- Rust 不直接绕过 `core/src/utils/logger.rs` 自己拼串
- Rust 模块名默认来自 `module_path()`，但会做可读压缩：去掉 crate 前缀，尾部 `::mod` 折叠掉，根模块显示为 `core`
- C 不直接新增裸 `PRINT(...)`，优先使用 `VP_LOG_ERROR`、`VP_LOG_WARN`、`VP_LOG_INFO`、`VP_LOG_DEBUG`
- C 侧 `module` 使用稳定短名，例如 `main`、`runtime`、`ble_gap`、`ble_hid`
- message 优先使用稳定短句；带参数时使用 `message;key=value,key2=value2` 风格，尽量短，优先小写英文，避免感叹号和无意义省略号
- message 只描述事实和关键信息，不写情绪化措辞
- 一条日志里优先带上最关键的状态字段，例如 `reason=%u`、`handle=%u`、`wired_active=%u`

### 级别语义

#### `ERROR`

用于：

- 明显错误
- 调用时序错误
- 不变量被破坏
- panic
- 会影响功能正确性的关键失败

不应用于：

- 正常但少见的状态
- 纯 bring-up 观察信息
- 仍在预期内的状态切换

#### `WARN`

用于：

- 非预期但系统还能继续运行的情况
- 降级、回退、重试、丢弃
- unsupported stub 被调用
- 某功能暂不可用但系统整体还能收敛

典型场景：

- queue 满导致样本或报文被丢弃
- route fallback 到次优链路
- 某 power/imu/flash 能力当前未实现

#### `INFO`

用于：

- 低频且关键的生命周期里程碑
- 默认日志阈值下值得长期保留的重要事件
- 初始化完成、连接建立、关键模式切换等摘要信息

典型场景：

- `core initialized`
- `connected;handle=...`
- `route changed;route=usb`

#### `DEBUG`

用于：

- 状态变化细节
- 输入变化
- 运行时决策细节
- 只在调试时有价值、长期看会嫌多的信息

典型场景：

- `usb state changed;state=...,wired_active=...`
- `button state changed;button=action,pressed=...`
- `hid send decision;route=...,ready=...`

### 什么时候该打日志

更适合保留：

- 初始化完成或失败
- route、USB、BLE、power 等关键状态切换
- 会影响后续收敛路径的异常分支
- unsupported stub 被调用
- 硬件 bring-up 阶段的重要条件变化
- 数据被丢弃、忽略、回退、重试等会影响诊断判断的事件
- API 在错误时机被调用

应尽量避免：

- 高频路径逐次打印，例如每帧姿态、每次轮询、每个 debounce tick
- ISR、主循环、定时器回调里的流水账式日志
- 能从上层已有日志稳定推出的重复日志
- 只有“到过这里”价值、没有状态信息的日志
- 未来长期不会用于定位问题的临时调试输出

### 高频路径规则

- 高频路径默认不要打 `INFO`
- 高频路径只在稳定状态变化、首次异常、显著回退或限频摘要时打日志
- 不打印每个样本、每个 queue push/pop、每次 poll、每次 EXTI 原始电平
- 如果日志只是为了证明“代码走到了这里”，通常不该长期保留

### 写日志前的判断问题

在新增日志前，先问自己：

1. 这条日志在长期运行里值得保留吗
2. 如果它每秒出现 100 次，我还愿不愿意看
3. 出问题时我会不会真的靠它定位
4. 它是不是只是为了证明代码走到了这里

如果答案更偏第 4 条，这条日志通常不该进入正式代码

### 文案约定

优先使用稳定短句，带参数时追加 `;key=value` 列表：

- 初始化完成：`core initialized`
- 状态切换：`usb state changed;state=...,wired_active=...`、`connected;handle=...`
- 调用时序错误：`api misuse;func=vp_core_poll,reason=before_init`
- panic：`panic;info=...`
- unsupported stub：`feature unavailable;feature=power_enter_sleep`

如果某条规则会被多个模块长期复用，应优先先改成统一日志文案，再继续扩散到新代码

## Git commit message 风格

### 一句话原则

- commit message 必须短、准、可扫读
- 不允许使用 scope
- 只靠 `type: summary` 表达清楚改动主题
- 一个 commit 只做一件事；如果写不清，优先拆 commit，不是补 scope

### 格式

统一格式：

- `<type>: <summary>`

例如：

- `feat: 增加按键边沿检测`
- `fix: 恢复 ble 重连流程`
- `refactor: 拆分板级输入层`
- `docs: 重写 ffi abi 说明`
- `chore: 对齐 c_api 格式与 clangd 配置`

不允许：

- `feat(input): 增加按键边沿检测`
- `fix(ble): 恢复重连流程`
- `refactor(core): 整理代码`

### type 约定

只使用下面几类：

- `feat`：新增功能、能力或对外行为
- `fix`：修复错误、修正异常行为
- `refactor`：重构、整理结构，不改变预期对外行为
- `docs`：文档、注释、说明文字调整为主
- `chore`：构建、脚本、配置、格式化、仓库维护杂项
- `test`：测试相关

如果一个提交同时像 `feat` 又像 `refactor`，优先按主要目的写；如果主要目的也说不清，通常说明这个 commit 该拆

### summary 规则

- 用动词开头
- 直接说改了什么，不说空话
- 优先写结果和对象，不写主观评价
- 尽量控制在一行内
- 不写无信息量词语，例如 `better`、`cleanup`、`update`、`misc`、`wip`

更好的写法：

- `feat: 实现 usb 与 ble 路由切换`
- `fix: 避免重连后重复初始化 hid`
- `refactor: 拆分 board map、gpio 与 input 辅助层`
- `docs: 澄清运行时事件流`
- `chore: 对齐 clangd 配置与代码格式`

应尽量避免：

- `refactor: 重构一下`
- `docs: 更新文档`
- `fix: 修复 bug`
- `chore: 顺手整理`
- `feat: 临时提交`
- `refactor: 优化注释`
- `fix: 修复了一些问题`

### 拆分规则

- 不要把功能、重命名、格式化、文档混进同一个 commit
- 不要为了省事把一串连续修补都堆成一个含糊提交
- 如果一个 commit 无法用一句具体的话说清主题，优先拆开
- 如果后一个 commit 只是修前一个 commit 的遗漏，合并前优先 squash

### 语言规则

- commit message 的 `type` 固定使用英文小写：`feat`、`fix`、`refactor`、`docs`、`chore`、`test`
- commit message 的 `summary` 强制使用中文
- 不要在 `summary` 里写中英混合句；术语、模块名、协议名、文件名等确实没有自然中文说法时，可保留必要英文名词
- 同一段历史里应保持语言一致，不要一部分提交写英文 summary、一部分提交写中文 summary

当前仓库统一采用：

- `type` 使用英文
- `summary` 使用中文
- 不使用 scope

采用这个约定的原因：

- 团队内部扫读和讨论更直接
- `feat`、`fix` 等前缀仍与常见工具和协作习惯兼容
- 项目里的术语、模块名、文件名仍可在需要时保留必要英文，不会影响表达


### 放进 `docs/` 的内容

满足下面任意一点，就更应该放进 `docs/`：

- 项目进入维护期后仍然应该保留
- 会被多个模块长期引用
- 属于规格、契约、状态机、设计边界或测试依据
- 新成员第一次理解项目时就需要知道

典型例子：

- `DESIGN.md`
- `FFI_ABI.md`
- `POWER_STATE_MACHINE.md`
- `ROUTE_STATE_MACHINE.md`
- `CONFIG_SPEC.md`
- `VENDOR_PROTOCOL.md`
- `TEST_PLAN.md`

### 放进 `docs/dev/` 的内容

满足下面任意一点，就更应该放进 `docs/dev/`：

- 只对当前开发阶段有意义
- 主要回答现在做到哪了、还差什么、谁来拍板
- 是阶段性协作规则，而不是产品规格本身
- 将来大概率会删除，或被回填进长期文档

典型例子：

- `TASKLIST.md`
- `DECISIONS.md`
- `OPEN_QUESTIONS.md`
- 本文

## 回填规则

出现下面这些情况时，应该把内容从 `docs/dev/` 回填到 `docs/`：

- 一个问题已经拍板，而且后续实现和测试都会依赖这个结论
- 同一条规则开始在多个代码文件里反复出现
- 某个阶段性决定已经不再是“阶段性”，而是系统边界的一部分
- 你发现新人如果不先知道这条规则，就很难读懂代码或文档

## 写文档时的判断问题

在新建文档前，先问自己四个问题：

1. 这份内容半年后还有没有价值
2. 它是在描述系统事实，还是描述当前推进过程
3. 它是否会被实现、测试、重构反复引用
4. 如果只能给这件事保留一个主入口，最合理的位置在哪里

如果答案更偏“长期事实”，放 `docs/`

如果答案更偏“当前推进”，放 `docs/dev/`
