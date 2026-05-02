#!/usr/bin/env python3
"""Cross-platform helper for local Zed/dev tasks."""

from __future__ import annotations

import argparse
import os
import shlex
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUILD_DIR = ROOT / "cmake-build-debug-mrs-risc-v-gcc12"
ELF = BUILD_DIR / "VoidPointer.elf"
COMPILE_COMMANDS_IN_BUILD = BUILD_DIR / "compile_commands.json"
COMPILE_COMMANDS_IN_ROOT = ROOT / "compile_commands.json"
PRESET = "mounriver-riscv-gcc12-debug"


def parse_env_value(value: str) -> str:
    value = value.strip()
    if (value.startswith('"') and value.endswith('"')) or (
        value.startswith("'") and value.endswith("'")
    ):
        return value[1:-1]
    return value


def load_env_local() -> None:
    env_file = ROOT / ".env.local"
    if not env_file.exists():
        return

    for raw_line in env_file.read_text(encoding="utf-8").splitlines():
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


def configure(_: argparse.Namespace) -> None:
    run(["cmake", "--preset", PRESET])


def build(_: argparse.Namespace) -> None:
    run(["cmake", "--build", "--preset", PRESET])


def refresh_clangd(_: argparse.Namespace) -> None:
    if not COMPILE_COMMANDS_IN_BUILD.exists():
        raise SystemExit(f"Missing {COMPILE_COMMANDS_IN_BUILD}. Run configure first.")
    shutil.copy2(COMPILE_COMMANDS_IN_BUILD, COMPILE_COMMANDS_IN_ROOT)
    print(f"Copied {COMPILE_COMMANDS_IN_BUILD} -> {COMPILE_COMMANDS_IN_ROOT}")


def configure_refresh(args: argparse.Namespace) -> None:
    configure(args)
    refresh_clangd(args)


def download(args: argparse.Namespace) -> None:
    load_env_local()

    firmware = Path(args.firmware) if args.firmware else ELF
    if not firmware.is_absolute():
        firmware = ROOT / firmware

    openocd_exe = os.environ.get("WCH_OPENOCD_EXE", "openocd")
    openocd_cfg = os.environ.get("WCH_OPENOCD_CFG", "wch-riscv.cfg")

    if not firmware.exists():
        raise SystemExit(f"Missing firmware ELF: {firmware}. Build first.")

    run(
        [
            openocd_exe,
            "-f",
            openocd_cfg,
            "-c",
            "init",
            "-c",
            "halt",
            "-c",
            f"program {firmware.as_posix()} verify reset exit",
        ]
    )


def build_download(args: argparse.Namespace) -> None:
    build(args)
    download(args)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command",
        choices=[
            "configure",
            "build",
            "refresh-clangd",
            "configure-refresh",
            "download",
            "build-download",
        ],
    )
    parser.add_argument(
        "--firmware",
        help="Firmware ELF to download. Defaults to cmake-build-debug-mrs-risc-v-gcc12/VoidPointer.elf.",
    )
    args = parser.parse_args()

    commands = {
        "configure": configure,
        "build": build,
        "refresh-clangd": refresh_clangd,
        "configure-refresh": configure_refresh,
        "download": download,
        "build-download": build_download,
    }
    commands[args.command](args)


if __name__ == "__main__":
    main()
