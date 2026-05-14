# VoidPointer

**2026 年全国大学生嵌入式竞赛参赛作品**

---

## 项目简介

VoidPointer 是一款面向教师和演讲者的演示笔，集成空中鼠标、激光笔和便携存储三项能力，满足翻页、激光指示和携带文件等使用场景。

硬件方面，主控采用 CH585，IMU 采用 LSM6DSV。

固件方面，使用 Rust 与 C 共同开发，C 处理硬件与协议栈边界，Rust 实现核心业务逻辑。

> 目前本仓库仅用于维护固件源码，硬件原理图与 PCB 暂未开源，后续补充。

---

## 快速开始

```bash
# 安装 Rust 目标
rustup target add riscv32imc-unknown-none-elf

# 初始化工具链配置
python tools/dev-task.py init

# 构建固件
python tools/dev-task.py build

# 下载到设备
python tools/dev-task.py download
```

## 仓库结构

```
VoidPointer/
  ├── core/       # 业务层（负责业务逻辑实现，由 Rust 编写）
  ├── platform/   # 平台层（负责 BLE，USB，IMU 等平台相关工作，由 C 编写）
  ├── docs/       # 设计文档
  └── tools/      # 开发脚本
```

---

## 许可

MIT © 2026 MC_XiaoHei
