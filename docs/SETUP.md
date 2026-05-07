# 本地开发环境配置指南

本文档说明 VoidPointer 固件项目的本地开发环境、工具链配置、编辑器索引配置和固件下载流程。文档目标是让新的开发者在不提交个人路径配置的前提下，稳定完成配置、构建和下载。

## 1. 适用范围

本文档适用于以下场景：

- 使用 MounRiver / WCH RISC-V GCC 工具链构建 CH585 固件。
- 使用 CMake preset 管理本地交叉编译配置。
- 使用 Zed、VS Code、Neovim 等 clangd 编辑器进行 C/C++ 索引和诊断。
- 使用 OpenOCD 将构建产物下载到目标板。

## 2. 必要工具

| 工具                | 用途               | 要求                          |
| ------------------- | ------------------ | ----------------------------- |
| CMake               | 配置和生成构建系统 | 3.22 或更新版本               |
| Ninja               | CMake 生成器       | 需要可在命令行调用            |
| Python              | 跨平台开发任务入口 | Python 3                      |
| Rust                | 构建 Rust core     | stable 工具链即可             |
| Rust target         | RISC-V no-std 目标 | `riscv32imc-unknown-none-elf` |
| MounRiver / WCH GCC | C/C++/ASM 交叉编译 | RISC-V Embedded GCC12         |
| clangd              | 编辑器索引和诊断   | 可选，但推荐                  |

安装 Rust 目标平台：

```/dev/null/command.sh#L1
rustup target add riscv32imc-unknown-none-elf
```

检查 Python：

```/dev/null/command.sh#L1
python --version
```

如果 Linux 发行版只有 `python3` 而没有 `python`，可以安装兼容包，例如 `python-is-python3`；也可以在本机把 `.zed/tasks.json` 中的 `command` 改成 `python3`。

检查 clangd：

```/dev/null/command.sh#L1
clangd --version
```

## 3. 本地文件约定

仓库会提交模板文件，但不会提交每台机器不同的本地路径。

| 文件                                            | 是否提交 | 说明                                    |
| ----------------------------------------------- | -------- | --------------------------------------- |
| `tools/templates/CMakeUserPresets.example.json` | 是       | CMake 本地 preset 模板。                |
| `tools/templates/env.local.example`             | 是       | OpenOCD 本地环境变量模板。              |
| `CMakeUserPresets.json`                         | 否       | 本机 CMake preset，包含工具链绝对路径。 |
| `.env.local`                                    | 否       | 本机下载配置，包含 OpenOCD 绝对路径。   |
| `compile_commands.json`                         | 否       | 给 clangd 使用的本地编译数据库。        |
| `cmake-build**/`                                | 否       | CMake 构建目录。                        |

## 4. 创建本地 CMake preset

首次克隆后运行交互式初始化：

```/dev/null/command.sh#L1
python tools/dev-task.py init
```

该命令会先询问 MounRiver / WCH RISC-V GCC 工具链位置，再校验本机 CMake、Cargo、Rust target、Ninja、OpenOCD 可执行文件和 WCH OpenOCD 配置文件。若 `ninja` 不在 `PATH`，会询问 Ninja 可执行文件路径，并写入 `CMakeUserPresets.json`。校验通过后生成 `CMakeUserPresets.json` 和 `.env.local`。

如果本机环境已经初始化过，再次运行 `init` 会先询问是否确认重新配置。

初始化会自动运行一次 CMake configure，并刷新 clangd 用的 `compile_commands.json`。后续只有 CMake 配置或源文件列表变化时，才需要运行 `refresh`。

也可以手动复制模板：

```/dev/null/command.sh#L1
cp tools/templates/CMakeUserPresets.example.json CMakeUserPresets.json
```

Windows CMD：

```/dev/null/command.txt#L1
copy tools\templates\CMakeUserPresets.example.json CMakeUserPresets.json
```

然后编辑 `CMakeUserPresets.json`，将 `C:/Path/To/MounRiver_Studio2` 替换为本机 MounRiver Studio 2 安装路径。

该文件应保留在本地，不应提交。

## 5. 创建本地下载配置

复制 OpenOCD 环境模板：

```/dev/null/command.sh#L1
cp tools/templates/env.local.example .env.local
```

Windows CMD：

```/dev/null/command.txt#L1
copy tools\templates\env.local.example .env.local
```

然后编辑 `.env.local`，设置：

- `WCH_OPENOCD_EXE`：OpenOCD 可执行文件路径。
- `WCH_OPENOCD_CFG`：WCH OpenOCD 配置文件路径。

示例：

```/dev/null/.env.local#L1-2
export WCH_OPENOCD_EXE="C:/MounRiver/MounRiver_Studio2/resources/app/resources/win32/components/WCH/OpenOCD/OpenOCD/bin/openocd.exe"
export WCH_OPENOCD_CFG="C:/MounRiver/MounRiver_Studio2/resources/app/resources/win32/components/WCH/OpenOCD/OpenOCD/bin/wch-riscv.cfg"
```

`download` / `build-download` 会读取 `.env.local`。如果该文件不存在或仍包含模板路径，请先运行 `python tools/dev-task.py init`。

## 6. 配置和构建

刷新 CMake 配置和 clangd 编译数据库：

```/dev/null/command.sh#L1
python tools/dev-task.py refresh
```

`refresh` 会执行 CMake configure，并把构建目录里的 `compile_commands.json` 复制到项目根目录。

等价的底层 CMake 命令：

```/dev/null/command.sh#L1
cmake --preset mounriver-riscv-gcc12-debug
```

构建固件：

```/dev/null/command.sh#L1
cmake --build --preset mounriver-riscv-gcc12-debug
```

默认构建产物目录：

`cmake-build-debug-mrs-risc-v-gcc12/`

主要产物包括：

- `VoidPointer.elf`
- `VoidPointer.hex`
- `VoidPointer.map`
- `VoidPointer.lst`

## 7. clangd 配置

项目提交了一份通用 `.clangd`。它用于：

- 提供相对路径形式的备用 include path。
- 移除 clangd 常见无法解析的 WCH/GCC/RISC-V 专用参数。

真实构建仍然以 CMake 和 WCH GCC 为准，`.clangd` 只影响编辑器索引、补全和诊断。

为了让 clangd 获得最准确的编译参数，配置 CMake 后建议将编译数据库复制到项目根目录：

```/dev/null/command.sh#L1
python tools/dev-task.py refresh-clangd
```

该命令会复制：

`cmake-build-debug-mrs-risc-v-gcc12/compile_commands.json`

到：

`compile_commands.json`

如果 clangd 仍然无法 query WCH 交叉编译器，可以在编辑器的用户配置中添加 clangd `--query-driver`。以 Zed 为例：

```/dev/null/zed-settings.json#L1-12
{
  "lsp": {
    "clangd": {
      "binary": {
        "path": "clangd",
        "arguments": [
          "--query-driver=C:/Path/To/MounRiver_Studio2/resources/app/resources/win32/components/WCH/Toolchain/RISC-V Embedded GCC12/bin/riscv-wch-elf-*.exe"
        ]
      }
    }
  }
}
```

用户级编辑器配置不应提交到仓库。

## 8. Zed 任务

项目提交了 `.zed/tasks.json`，Zed 中可通过 `task: spawn` 运行任务。

Zed 当前更偏“命令面板 / task picker”工作流，不提供 CLion 那种完整图形化 Run Configuration 工具栏。如需一键运行，可以给 task 绑定快捷键。

所有 Zed task 都调用跨平台入口 `tools/dev-task.py`：

| Zed task                          | 等价命令                                 | 说明                                                                                                                                                        |
| --------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `init local environment`          | `python tools/dev-task.py init`          | 交互式初始化本机工具链/OpenOCD 配置，校验 CMake/Cargo/Rust target/Ninja/可执行文件，生成 `CMakeUserPresets.json` / `.env.local`，并自动执行一次 `refresh`。 |
| `refresh project config`          | `python tools/dev-task.py refresh`       | 重新执行 CMake configure，并刷新根目录 `compile_commands.json`。                                                                                            |
| `build firmware`                  | `python tools/dev-task.py build`         | 构建固件。                                                                                                                                                  |
| `format source code`              | `python tools/dev-task.py format`        | 统一格式化源码：对 `platform/` 下的 C/C++ 文件调用 `clang-format`，对 `core/` 调用 `cargo fmt`。                                                          |
| `download firmware`               | `python tools/dev-task.py download`      | 构建后下载固件；日常最常用。                                                                                                                                |
| `download firmware without build` | `python tools/dev-task.py download-only` | 只下载已有固件，不重新构建。                                                                                                                                |

推荐日常使用：

- 首次克隆后运行一次 `init local environment`。
- 后续 CMake 配置、源文件列表或 include 路径变化后运行 `refresh project config`。
- 日常下载运行 `download firmware`。
- 只想重复烧录已有产物时运行 `download firmware without build`。
- 仅检查编译时运行 `build firmware`。

## 9. 固件下载

下载流程由 `tools/dev-task.py` 统一处理。脚本会：

1. 读取 `.env.local`。
2. 获取 `WCH_OPENOCD_EXE` 和 `WCH_OPENOCD_CFG`。
3. 检查 `cmake-build-debug-mrs-risc-v-gcc12/VoidPointer.elf` 是否存在。
4. 调用 OpenOCD 执行 `program ... reset shutdown`。

构建并下载：

```/dev/null/command.sh#L1
python tools/dev-task.py download
```

只下载已有构建产物，不重新构建：

```/dev/null/command.sh#L1
python tools/dev-task.py download-only
```
