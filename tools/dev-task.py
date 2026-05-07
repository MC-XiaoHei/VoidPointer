#!/usr/bin/env python3
"""Cross-platform helper for local Zed/dev tasks."""

from __future__ import annotations

import argparse
from datetime import datetime
import json
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUILD_DIR = ROOT / "cmake-build-debug-mrs-risc-v-gcc12"
ELF = BUILD_DIR / "VoidPointer.elf"
COMPILE_COMMANDS_IN_BUILD = BUILD_DIR / "compile_commands.json"
COMPILE_COMMANDS_IN_ROOT = ROOT / "compile_commands.json"
PRESET = "mounriver-riscv-gcc12-debug"
CMAKE_USER_PRESETS = ROOT / "CMakeUserPresets.json"
ENV_LOCAL = ROOT / ".env.local"
MOUNRIVER_PLACEHOLDER = "C:/Path/To/MounRiver_Studio2"


def openocd_scripts_dir(openocd_exe: Path) -> Path | None:
    # MounRiver layout:
    #   OpenOCD/OpenOCD/bin/openocd.exe
    #   OpenOCD/OpenOCD/share/openocd/scripts
    candidate = openocd_exe.resolve().parent.parent / "share" / "openocd" / "scripts"
    return candidate if candidate.is_dir() else None


def host_exe(name: str) -> str:
    return f"{name}.exe" if os.name == "nt" else name


TOOLCHAIN_PROGRAMS = {
    "CMAKE_C_COMPILER": host_exe("riscv-wch-elf-gcc"),
    "CMAKE_CXX_COMPILER": host_exe("riscv-wch-elf-g++"),
    "CMAKE_ASM_COMPILER": host_exe("riscv-wch-elf-gcc"),
    "CMAKE_OBJCOPY": host_exe("riscv-wch-elf-objcopy"),
    "CMAKE_OBJDUMP": host_exe("riscv-wch-elf-objdump"),
    "CMAKE_SIZE": host_exe("riscv-wch-elf-size"),
    "CMAKE_AR": host_exe("riscv-wch-elf-ar"),
}


def parse_env_value(value: str) -> str:
    value = value.strip()
    if (value.startswith('"') and value.endswith('"')) or (
        value.startswith("'") and value.endswith("'")
    ):
        return value[1:-1]
    return value


def load_env_local() -> None:
    if not ENV_LOCAL.exists():
        return

    for raw_line in ENV_LOCAL.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line[len("export ") :].strip()
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        if not key:
            continue
        os.environ[key] = parse_env_value(value)


def run(args: list[str]) -> None:
    print("+ " + " ".join(shlex.quote(arg) for arg in args), flush=True)
    completed = subprocess.run(args, cwd=ROOT)
    if completed.returncode != 0:
        raise SystemExit(completed.returncode)


def probe(args: list[str], timeout: int = 10) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )


def prompt_path(prompt: str, default: Path | None = None) -> Path:
    suffix = f" [{default}]" if default else ""
    value = input(f"{prompt}{suffix}: ").strip().strip('"')
    if not value and default is not None:
        return default
    return Path(value).expanduser()


def prompt_yes_no(prompt: str, default: bool = True) -> bool:
    default_text = "Y/n" if default else "y/N"
    while True:
        value = input(f"{prompt} [{default_text}]: ").strip().lower()
        if not value:
            return default
        if value in {"y", "yes"}:
            return True
        if value in {"n", "no"}:
            return False
        print("Please answer y or n.")


def candidate_toolchain_bin(path: Path) -> Path:
    if path.is_file():
        return path.parent
    if (path / TOOLCHAIN_PROGRAMS["CMAKE_C_COMPILER"]).is_file():
        return path
    return (
        path
        / "resources"
        / "app"
        / "resources"
        / "win32"
        / "components"
        / "WCH"
        / "Toolchain"
        / "RISC-V Embedded GCC12"
        / "bin"
    )


def candidate_openocd_paths(path: Path) -> tuple[Path, Path]:
    if path.is_file():
        return path, path.with_name("wch-riscv.cfg")
    openocd_bin = (
        path
        / "resources"
        / "app"
        / "resources"
        / "win32"
        / "components"
        / "WCH"
        / "OpenOCD"
        / "OpenOCD"
        / "bin"
    )
    return openocd_bin / host_exe("openocd"), openocd_bin / "wch-riscv.cfg"


def validate_toolchain_bin(toolchain_bin: Path) -> dict[str, Path]:
    resolved: dict[str, Path] = {}
    missing: list[Path] = []
    for key, filename in TOOLCHAIN_PROGRAMS.items():
        executable = toolchain_bin / filename
        if executable.is_file():
            resolved[key] = executable.resolve()
        else:
            missing.append(executable)

    if missing:
        missing_text = "\n".join(f"- {path}" for path in missing)
        raise ValueError(f"Missing required toolchain executable(s):\n{missing_text}")

    gcc = resolved["CMAKE_C_COMPILER"]
    try:
        dumpmachine = probe([str(gcc), "-dumpmachine"])
    except OSError as exc:
        raise ValueError(f"Failed to run {gcc}: {exc}") from exc
    except subprocess.TimeoutExpired as exc:
        raise ValueError(f"Timed out while running {gcc} -dumpmachine") from exc

    machine = (dumpmachine.stdout + dumpmachine.stderr).strip().lower()
    if dumpmachine.returncode != 0 or "riscv" not in machine:
        raise ValueError(
            f"{gcc} does not look like a RISC-V GCC. `-dumpmachine` returned: {machine!r}"
        )

    return resolved


def validate_openocd(openocd_exe: Path, openocd_cfg: Path) -> tuple[Path, Path]:
    if not openocd_exe.is_file():
        raise ValueError(f"OpenOCD executable does not exist: {openocd_exe}")
    if not openocd_cfg.is_file():
        raise ValueError(f"OpenOCD config does not exist: {openocd_cfg}")

    try:
        version = probe([str(openocd_exe), "--version"])
    except OSError as exc:
        raise ValueError(f"Failed to run {openocd_exe}: {exc}") from exc
    except subprocess.TimeoutExpired as exc:
        raise ValueError(f"Timed out while running {openocd_exe} --version") from exc

    if version.returncode != 0:
        output = (version.stdout + version.stderr).strip()
        raise ValueError(f"OpenOCD --version failed: {output}")

    return openocd_exe.resolve(), openocd_cfg.resolve()


def candidate_ninja_paths(mounriver_root: Path | None) -> list[Path]:
    candidates: list[Path] = []
    path_ninja = shutil.which("ninja")
    if path_ninja is not None:
        candidates.append(Path(path_ninja))
    if mounriver_root is not None:
        candidates.extend(
            [
                mounriver_root / "ninja.exe",
                mounriver_root
                / "resources"
                / "app"
                / "resources"
                / "win32"
                / "components"
                / "Ninja"
                / "ninja.exe",
                mounriver_root
                / "resources"
                / "app"
                / "resources"
                / "win32"
                / "components"
                / "ninja"
                / "ninja.exe",
                mounriver_root
                / "resources"
                / "app"
                / "resources"
                / "win32"
                / "ninja.exe",
            ]
        )
    return candidates


def validate_ninja(ninja: Path) -> Path:
    if not ninja.is_file():
        raise ValueError(f"Ninja executable does not exist: {ninja}")
    try:
        completed = probe([str(ninja), "--version"])
    except OSError as exc:
        raise ValueError(f"Failed to run {ninja}: {exc}") from exc
    except subprocess.TimeoutExpired as exc:
        raise ValueError(f"Timed out while running {ninja} --version") from exc
    if completed.returncode != 0:
        output = (completed.stdout + completed.stderr).strip()
        raise ValueError(f"Ninja validation failed: {output}")
    return ninja.resolve()


def collect_ninja_path(mounriver_root: Path | None) -> Path:
    for candidate in candidate_ninja_paths(mounriver_root):
        if candidate.is_file():
            ninja = validate_ninja(candidate)
            print(f"Validated ninja: {ninja}")
            return ninja

    print("\nNinja setup")
    while True:
        entered = prompt_path("Ninja executable")
        try:
            ninja = validate_ninja(entered)
            print(f"Validated ninja: {ninja}")
            return ninja
        except ValueError as exc:
            print(f"\nNinja validation failed:\n{exc}\n")
            if not prompt_yes_no("Try another Ninja path?", True):
                raise SystemExit(1)


def validate_host_command(command: str, args: list[str]) -> Path:
    executable = shutil.which(command)
    if executable is None:
        raise ValueError(f"Required command not found in PATH: {command}")
    try:
        completed = probe([executable, *args])
    except OSError as exc:
        raise ValueError(f"Failed to run {command}: {exc}") from exc
    except subprocess.TimeoutExpired as exc:
        raise ValueError(f"Timed out while running {command}") from exc
    if completed.returncode != 0:
        output = (completed.stdout + completed.stderr).strip()
        raise ValueError(f"{command} validation failed: {output}")
    return Path(executable).resolve()


def validate_host_tools() -> None:
    print("\nHost build tool setup")
    for command, args in (("cmake", ["--version"]), ("cargo", ["--version"])):
        executable = validate_host_command(command, args)
        print(f"Validated {command}: {executable}")

    rustup = shutil.which("rustup")
    if rustup is None:
        print(
            "Warning: rustup was not found in PATH; cannot verify the Rust target installation."
        )
        return

    completed = probe([rustup, "target", "list", "--installed"])
    installed_targets = completed.stdout.splitlines()
    if (
        completed.returncode != 0
        or "riscv32imc-unknown-none-elf" not in installed_targets
    ):
        raise ValueError(
            "Missing Rust target riscv32imc-unknown-none-elf. Run: rustup target add riscv32imc-unknown-none-elf"
        )
    print("Validated Rust target: riscv32imc-unknown-none-elf")


def collect_toolchain_paths() -> tuple[dict[str, Path], Path | None]:
    common_default = Path("C:/MounRiver/MounRiver_Studio2") if os.name == "nt" else None
    print("\nMounRiver / WCH RISC-V GCC setup")
    print(
        "Enter the MounRiver Studio 2 directory, the toolchain bin directory, or riscv-wch-elf-gcc itself."
    )

    while True:
        entered = prompt_path("MounRiver/toolchain path", common_default)
        toolchain_bin = candidate_toolchain_bin(entered)
        try:
            tools = validate_toolchain_bin(toolchain_bin)
            mounriver_root = (
                entered if entered.is_dir() and entered != toolchain_bin else None
            )
            print(f"Validated RISC-V GCC toolchain: {toolchain_bin}")
            return tools, mounriver_root
        except ValueError as exc:
            print(f"\nToolchain validation failed:\n{exc}\n")
            if not prompt_yes_no("Try another toolchain path?", True):
                raise SystemExit(1)


def collect_openocd_paths(mounriver_root: Path | None) -> tuple[Path, Path]:
    print("\nWCH OpenOCD setup")
    default_exe: Path | None = None
    default_cfg: Path | None = None
    if mounriver_root is not None:
        default_exe, default_cfg = candidate_openocd_paths(mounriver_root)

    while True:
        openocd_exe = prompt_path("OpenOCD executable", default_exe)
        openocd_cfg = prompt_path("WCH OpenOCD config", default_cfg)
        try:
            validated = validate_openocd(openocd_exe, openocd_cfg)
            print(f"Validated OpenOCD: {validated[0]}")
            return validated
        except ValueError as exc:
            print(f"\nOpenOCD validation failed:\n{exc}\n")
            if not prompt_yes_no("Try another OpenOCD path?", True):
                raise SystemExit(1)


def path_for_config(path: Path) -> str:
    return path.as_posix()


def write_cmake_user_presets(
    tools: dict[str, Path], ninja: Path, overwrite: bool
) -> None:
    if CMAKE_USER_PRESETS.exists() and not overwrite:
        print(f"Keeping existing {CMAKE_USER_PRESETS.relative_to(ROOT)}")
        return

    cache_variables = {
        "CMAKE_BUILD_TYPE": "Debug",
        "CMAKE_EXPORT_COMPILE_COMMANDS": "ON",
        "CMAKE_MAKE_PROGRAM": path_for_config(ninja),
        **{key: path_for_config(path) for key, path in tools.items()},
    }
    data = {
        "version": 6,
        "cmakeMinimumRequired": {"major": 3, "minor": 22, "patch": 0},
        "configurePresets": [
            {
                "name": PRESET,
                "displayName": "MounRiver WCH RISC-V GCC12 Debug",
                "generator": "Ninja",
                "binaryDir": "${sourceDir}/cmake-build-debug-mrs-risc-v-gcc12",
                "cacheVariables": cache_variables,
            }
        ],
        "buildPresets": [{"name": PRESET, "configurePreset": PRESET}],
        "vendor": {
            "voidpointer": {
                "debugger": path_for_config(
                    tools["CMAKE_C_COMPILER"].with_name(host_exe("riscv-wch-elf-gdb"))
                )
            }
        },
    }
    CMAKE_USER_PRESETS.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {CMAKE_USER_PRESETS.relative_to(ROOT)}")


def write_env_local(openocd_exe: Path, openocd_cfg: Path, overwrite: bool) -> None:
    if ENV_LOCAL.exists() and not overwrite:
        print(f"Keeping existing {ENV_LOCAL.relative_to(ROOT)}")
        return

    scripts_dir = openocd_scripts_dir(openocd_exe)
    scripts_line = (
        f'export WCH_OPENOCD_SCRIPTS="{path_for_config(scripts_dir)}"\n'
        if scripts_dir is not None
        else ""
    )
    content = (
        "# Local machine configuration for VoidPointer.\n"
        "# Generated by `python tools/dev-task.py init`.\n"
        "# `.env.local` is ignored by git.\n\n"
        f'export WCH_OPENOCD_EXE="{path_for_config(openocd_exe)}"\n'
        f'export WCH_OPENOCD_CFG="{path_for_config(openocd_cfg)}"\n'
        f"{scripts_line}"
    )
    ENV_LOCAL.write_text(content, encoding="utf-8")
    print(f"Wrote {ENV_LOCAL.relative_to(ROOT)}")


def is_initialized() -> bool:
    if not CMAKE_USER_PRESETS.exists() or not ENV_LOCAL.exists():
        return False
    presets_text = CMAKE_USER_PRESETS.read_text(encoding="utf-8")
    env_text = ENV_LOCAL.read_text(encoding="utf-8")
    return (
        MOUNRIVER_PLACEHOLDER not in presets_text
        and MOUNRIVER_PLACEHOLDER not in env_text
    )


def init(args: argparse.Namespace) -> None:
    if is_initialized() and not prompt_yes_no(
        "Local environment is already initialized. Reconfigure it?", False
    ):
        print("Initialization unchanged.")
        return

    tools, mounriver_root = collect_toolchain_paths()

    try:
        validate_host_tools()
    except ValueError as exc:
        raise SystemExit(f"Host build tool validation failed:\n{exc}") from exc

    ninja = collect_ninja_path(mounriver_root)
    openocd_exe, openocd_cfg = collect_openocd_paths(mounriver_root)

    write_cmake_user_presets(tools, ninja, True)
    write_env_local(openocd_exe, openocd_cfg, True)

    print("\nInitialization complete. Running initial refresh...")
    refresh(args)


def ensure_initialized_for_configure() -> None:
    if not CMAKE_USER_PRESETS.exists():
        raise SystemExit(
            "Missing CMakeUserPresets.json. Run `python tools/dev-task.py init` first."
        )
    text = CMAKE_USER_PRESETS.read_text(encoding="utf-8")
    if MOUNRIVER_PLACEHOLDER in text:
        raise SystemExit(
            "CMakeUserPresets.json still contains template paths. Run `python tools/dev-task.py init`."
        )


def ensure_initialized_for_download() -> None:
    if not ENV_LOCAL.exists():
        raise SystemExit(
            "Missing .env.local. Run `python tools/dev-task.py init` first."
        )
    text = ENV_LOCAL.read_text(encoding="utf-8")
    if MOUNRIVER_PLACEHOLDER in text:
        raise SystemExit(
            ".env.local still contains template paths. Run `python tools/dev-task.py init`."
        )


def configure(_: argparse.Namespace) -> None:
    ensure_initialized_for_configure()
    run(["cmake", "--preset", PRESET])


def build(_: argparse.Namespace) -> None:
    ensure_initialized_for_configure()
    run(["cmake", "--build", "--preset", PRESET])


def refresh_clangd(_: argparse.Namespace) -> None:
    if not COMPILE_COMMANDS_IN_BUILD.exists():
        raise SystemExit(f"Missing {COMPILE_COMMANDS_IN_BUILD}. Run configure first.")
    shutil.copy2(COMPILE_COMMANDS_IN_BUILD, COMPILE_COMMANDS_IN_ROOT)
    print(f"Copied {COMPILE_COMMANDS_IN_BUILD} -> {COMPILE_COMMANDS_IN_ROOT}")


def refresh(args: argparse.Namespace) -> None:
    configure(args)
    refresh_clangd(args)


def format_sources(_: argparse.Namespace) -> None:
    clang_format = shutil.which("clang-format")
    if clang_format is None:
        raise SystemExit(
            "Missing clang-format in PATH. Install LLVM/Clang and ensure `clang-format` is available."
        )

    c_like_files = sorted(
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "platform").rglob("*")
        if path.is_file() and path.suffix.lower() in {".c", ".h", ".cpp", ".hpp"}
    )
    if c_like_files:
        run([clang_format, "-i", *c_like_files])
    else:
        print("No C/C++ sources found under platform/")

    run(["cargo", "fmt", "--manifest-path", "core/Cargo.toml"])


def download_only(args: argparse.Namespace) -> None:
    ensure_initialized_for_download()
    load_env_local()

    firmware = Path(args.firmware) if args.firmware else ELF
    if not firmware.is_absolute():
        firmware = ROOT / firmware

    openocd_exe = Path(os.environ.get("WCH_OPENOCD_EXE", "openocd"))
    openocd_cfg = Path(os.environ.get("WCH_OPENOCD_CFG", "wch-riscv.cfg"))
    openocd_scripts = os.environ.get("WCH_OPENOCD_SCRIPTS")

    if not firmware.exists():
        raise SystemExit(f"Missing firmware ELF: {firmware}. Build first.")

    command = [str(openocd_exe)]
    scripts_dir = Path(openocd_scripts) if openocd_scripts else openocd_scripts_dir(openocd_exe)
    if scripts_dir is not None:
        command.extend(["-s", str(scripts_dir)])

    command.extend(
        [
            "-f",
            str(openocd_cfg),
            "-c",
            "tcl_port disabled",
            "-c",
            "gdb_port disabled",
            "-c",
            f'program "{firmware.as_posix()}"',
            "-c",
            "reset",
            "-c",
            "shutdown",
        ]
    )
    run(command)


def download(args: argparse.Namespace) -> None:
    build(args)
    download_only(args)


def serial_monitor(_: argparse.Namespace) -> None:
    load_env_local()
    port = os.environ.get("VP_SERIAL_PORT", "COM5")
    baud = int(os.environ.get("VP_SERIAL_BAUD", "115200"))

    try:
        import serial  # type: ignore
    except ImportError as exc:
        raise SystemExit(
            "pyserial is required for the serial monitor. Install it with: python -m pip install pyserial"
        ) from exc

    print(f"+ timestamped serial monitor on {port} {baud},8,N,1", flush=True)
    print("--- Quit: Ctrl+C ---", flush=True)

    try:
        with serial.Serial(port, baudrate=baud, timeout=0.1) as ser:
            buffer = bytearray()
            while True:
                chunk = ser.read(4096)
                if not chunk:
                    continue
                buffer.extend(chunk)

                while True:
                    newline_index = buffer.find(b"\n")
                    if newline_index < 0:
                        break

                    line = bytes(buffer[: newline_index + 1])
                    del buffer[: newline_index + 1]

                    timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                    text = line.decode("utf-8", errors="replace").rstrip("\r\n")
                    print(f"[{timestamp}] {text}", flush=True)
    except KeyboardInterrupt:
        print("\n--- Serial monitor stopped ---", flush=True)
    except Exception as exc:
        raise SystemExit(f"Serial monitor failed: {exc}") from exc


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command",
        choices=[
            "init",
            "refresh",
            "build",
            "format",
            "download",
            "download-only",
            "serial",
        ],
    )
    parser.add_argument(
        "-f",
        "--firmware",
        help="Firmware ELF to download. Defaults to cmake-build-debug-mrs-risc-v-gcc12/VoidPointer.elf.",
    )

    args = parser.parse_args()

    commands = {
        "init": init,
        "refresh": refresh,
        "build": build,
        "format": format_sources,
        "download": download,
        "download-only": download_only,
        "serial": serial_monitor,
    }
    commands[args.command](args)


if __name__ == "__main__":
    main()
