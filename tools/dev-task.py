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
from enum import Enum
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COMPILE_COMMANDS_LINK = ROOT / "compile_commands.json"
CMAKE_USER_PRESETS = ROOT / "CMakeUserPresets.json"
ENV_LOCAL = ROOT / ".env.local"
PLACEHOLDER_PATH = "C:/Path/To/MounRiver_Studio2"

# MounRiver Studio 2 embedded directory layout
_MOUNRIVER_WIN32 = Path("resources/app/resources/win32")

TOOLCHAIN_PROGRAMS = {
    "CMAKE_C_COMPILER": "riscv-wch-elf-gcc",
    "CMAKE_CXX_COMPILER": "riscv-wch-elf-g++",
    "CMAKE_ASM_COMPILER": "riscv-wch-elf-gcc",
    "CMAKE_OBJCOPY": "riscv-wch-elf-objcopy",
    "CMAKE_OBJDUMP": "riscv-wch-elf-objdump",
    "CMAKE_SIZE": "riscv-wch-elf-size",
    "CMAKE_AR": "riscv-wch-elf-ar",
}


class Profile(Enum):
    DEV = "dev"
    RELEASE = "release"

    def preset(self) -> str:
        suffix = "-debug" if self == Profile.DEV else "-release"
        return f"mounriver-riscv-gcc12{suffix}"

    def build_dir(self) -> Path:
        dir_name = (
            "cmake-build-debug-mrs-risc-v-gcc12"
            if self == Profile.DEV
            else "cmake-build-release-mrs-risc-v-gcc12"
        )
        return ROOT / dir_name

    def elf(self) -> Path:
        return self.build_dir() / "VoidPointer.elf"

    def compile_commands(self) -> Path:
        return self.build_dir() / "compile_commands.json"

    def cmake_build_type(self) -> str:
        return "Debug" if self == Profile.DEV else "Release"

    def display_name(self) -> str:
        tag = "Debug" if self == Profile.DEV else "Release"
        return f"MounRiver WCH RISC-V GCC12 {tag}"

    @staticmethod
    def from_env() -> Profile:
        load_env_local()
        raw = os.environ.get("VP_PROFILE", "dev")
        try:
            return Profile(raw)
        except ValueError:
            return Profile.DEV

    @staticmethod
    def resolve(profile_arg: str | None) -> Profile:
        if profile_arg is not None:
            return Profile(profile_arg)
        return Profile.from_env()


def host_exe(name: str) -> str:
    return f"{name}.exe" if os.name == "nt" else name


def openocd_scripts_dir(openocd_exe: Path) -> Path | None:
    candidate = openocd_exe.resolve().parent.parent / "share" / "openocd" / "scripts"
    return candidate if candidate.is_dir() else None


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
        if key:
            os.environ[key] = parse_env_value(value)


# ---------------------------------------------------------------------------
# Execution helpers
# ---------------------------------------------------------------------------

def run(args: list[str]) -> None:
    print("+ " + " ".join(shlex.quote(arg) for arg in args), flush=True)
    completed = subprocess.run(args, cwd=ROOT)
    if completed.returncode != 0:
        raise SystemExit(completed.returncode)


def probe(args: list[str], timeout: int = 10) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout
    )


def run_cargo(args: list[str]) -> None:
    cargo_args = ["cargo", *args]
    print("+ " + " ".join(shlex.quote(arg) for arg in cargo_args), flush=True)
    completed = subprocess.run(cargo_args, cwd=ROOT / "core")
    if completed.returncode != 0:
        raise SystemExit(completed.returncode)


# ---------------------------------------------------------------------------
# Interactive prompts
# ---------------------------------------------------------------------------

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


# ---------------------------------------------------------------------------
# Path candidates (MounRiver Studio 2 layout helpers)
# ---------------------------------------------------------------------------

def _mounriver_path(segments: list[str]) -> Path:
    return _MOUNRIVER_WIN32.joinpath(*segments)


def candidate_toolchain_bin(path: Path) -> Path:
    if path.is_file():
        return path.parent
    gcc = host_exe(TOOLCHAIN_PROGRAMS["CMAKE_C_COMPILER"])
    if (path / gcc).is_file():
        return path
    return ROOT / _mounriver_path(["components", "WCH", "Toolchain", "RISC-V Embedded GCC12", "bin"])


def candidate_openocd_paths(path: Path) -> tuple[Path, Path]:
    if path.is_file():
        return path, path.with_name("wch-riscv.cfg")
    openocd_bin = ROOT / _mounriver_path(["components", "WCH", "OpenOCD", "OpenOCD", "bin"])
    return openocd_bin / host_exe("openocd"), openocd_bin / "wch-riscv.cfg"


def candidate_ninja_paths(mounriver_root: Path | None) -> list[Path]:
    candidates: list[Path] = []
    path_ninja = shutil.which("ninja")
    if path_ninja is not None:
        candidates.append(Path(path_ninja))
    if mounriver_root is not None:
        base = mounriver_root / _mounriver_path([])
        candidates.extend(
            [
                mounriver_root / "ninja.exe",
                base / "Ninja" / "ninja.exe",
                base / "ninja" / "ninja.exe",
                base / "ninja.exe",
            ]
        )
    return candidates


# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------

def _run_and_check(
    args: list[str],
    error_prefix: str,
    timeout: int = 10,
    check_returncode: bool = True,
    check_output: str | None = None,
) -> subprocess.CompletedProcess[str]:
    try:
        completed = probe(args, timeout=timeout)
    except OSError as exc:
        raise ValueError(f"{error_prefix}: {exc}") from exc
    except subprocess.TimeoutExpired as exc:
        raise ValueError(f"{error_prefix}: timed out") from exc
    if check_returncode and completed.returncode != 0:
        output = (completed.stdout + completed.stderr).strip()
        raise ValueError(f"{error_prefix}: {output}")
    if check_output is not None and check_output not in (completed.stdout + completed.stderr).lower():
        raise ValueError(f"{error_prefix}: unexpected output: {completed.stdout + completed.stderr}")
    return completed


def validate_toolchain_bin(toolchain_bin: Path) -> dict[str, Path]:
    resolved: dict[str, Path] = {}
    missing: list[Path] = []

    for key, filename in TOOLCHAIN_PROGRAMS.items():
        executable = toolchain_bin / host_exe(filename)
        if executable.is_file():
            resolved[key] = executable.resolve()
        else:
            missing.append(executable)

    if missing:
        lines = "\n".join(f"- {p}" for p in missing)
        raise ValueError(f"Missing toolchain executable(s):\n{lines}")

    gcc = resolved["CMAKE_C_COMPILER"]
    completed = _run_and_check([str(gcc), "-dumpmachine"], f"Failed to run {gcc}")
    machine = (completed.stdout + completed.stderr).strip().lower()
    if "riscv" not in machine:
        raise ValueError(f"{gcc} is not a RISC-V GCC. -dumpmachine returned: {machine!r}")

    return resolved


def validate_openocd(openocd_exe: Path, openocd_cfg: Path) -> tuple[Path, Path]:
    if not openocd_exe.is_file():
        raise ValueError(f"OpenOCD executable not found: {openocd_exe}")
    if not openocd_cfg.is_file():
        raise ValueError(f"OpenOCD config not found: {openocd_cfg}")

    _run_and_check([str(openocd_exe), "--version"], f"Failed to run {openocd_exe}")
    return openocd_exe.resolve(), openocd_cfg.resolve()


def validate_ninja(ninja: Path) -> Path:
    if not ninja.is_file():
        raise ValueError(f"Ninja executable not found: {ninja}")
    _run_and_check([str(ninja), "--version"], f"Failed to run {ninja}")
    return ninja.resolve()


def validate_host_command(command: str, check_args: list[str]) -> Path:
    executable = shutil.which(command)
    if executable is None:
        raise ValueError(f"Command not found in PATH: {command}")
    _run_and_check([executable, *check_args], f"Failed to run {command}")
    return Path(executable).resolve()


def validate_host_tools() -> None:
    print("\nHost build tool setup")
    for command, check_args in (("cmake", ["--version"]), ("cargo", ["--version"])):
        executable = validate_host_command(command, check_args)
        print(f"Validated {command}: {executable}")

    rustup = shutil.which("rustup")
    if rustup is None:
        print("Warning: rustup not found in PATH; cannot verify Rust target.")
        return

    completed = probe([rustup, "target", "list", "--installed"])
    if completed.returncode != 0 or "riscv32imc-unknown-none-elf" not in completed.stdout.splitlines():
        raise ValueError(
            "Missing Rust target riscv32imc-unknown-none-elf.\n"
            "Run: rustup target add riscv32imc-unknown-none-elf"
        )
    print("Validated Rust target: riscv32imc-unknown-none-elf")


# ---------------------------------------------------------------------------
# Interactive collection
# ---------------------------------------------------------------------------

def _retry_collect(
    prompt_msg: str,
    validate_fn,
    default_path: Path | None = None,
):
    while True:
        entered = prompt_path(prompt_msg, default_path)
        try:
            result = validate_fn(entered)
            return result
        except ValueError as exc:
            print(f"\nValidation failed:\n{exc}\n")
            if not prompt_yes_no("Try again?", True):
                raise SystemExit(1)


def collect_ninja_path(mounriver_root: Path | None) -> Path:
    for candidate in candidate_ninja_paths(mounriver_root):
        if candidate.is_file():
            ninja = validate_ninja(candidate)
            print(f"Validated ninja: {ninja}")
            return ninja

    print("\nNinja setup")
    return _retry_collect(
        "Ninja executable",
        lambda p: (n := validate_ninja(p)) or print(f"Validated ninja: {n}") or n,
    )


def collect_toolchain_paths() -> tuple[dict[str, Path], Path | None]:
    common_default = Path("C:/MounRiver/MounRiver_Studio2") if os.name == "nt" else None
    print("\nMounRiver / WCH RISC-V GCC setup")
    print("Enter the MounRiver Studio 2 directory, the toolchain bin directory, or riscv-wch-elf-gcc itself.")

    while True:
        entered = prompt_path("MounRiver/toolchain path", common_default)
        toolchain_bin = candidate_toolchain_bin(entered)
        try:
            tools = validate_toolchain_bin(toolchain_bin)
            mounriver_root = entered if entered.is_dir() and entered != toolchain_bin else None
            print(f"Validated RISC-V GCC toolchain: {toolchain_bin}")
            return tools, mounriver_root
        except ValueError as exc:
            print(f"\nToolchain validation failed:\n{exc}\n")
            if not prompt_yes_no("Try again?", True):
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
            exe, cfg = validate_openocd(openocd_exe, openocd_cfg)
            print(f"Validated OpenOCD: {exe}")
            return exe, cfg
        except ValueError as exc:
            print(f"\nOpenOCD validation failed:\n{exc}\n")
            if not prompt_yes_no("Try again?", True):
                raise SystemExit(1)


# ---------------------------------------------------------------------------
# Config file writers
# ---------------------------------------------------------------------------

def _path_for_cmake(path: Path) -> str:
    return path.as_posix()


def write_cmake_user_presets(tools: dict[str, Path], ninja: Path, overwrite: bool) -> None:
    if CMAKE_USER_PRESETS.exists() and not overwrite:
        print(f"Keeping existing {CMAKE_USER_PRESETS.relative_to(ROOT)}")
        return

    configure_presets = []
    build_presets = []

    for profile in Profile:
        pname = profile.preset()
        cache_vars = {
            "CMAKE_BUILD_TYPE": profile.cmake_build_type(),
            "CMAKE_EXPORT_COMPILE_COMMANDS": "ON",
            "CMAKE_MAKE_PROGRAM": _path_for_cmake(ninja),
            **{key: _path_for_cmake(path) for key, path in tools.items()},
        }
        configure_presets.append(
            {
                "name": pname,
                "displayName": profile.display_name(),
                "generator": "Ninja",
                "binaryDir": "${sourceDir}/" + profile.build_dir().relative_to(ROOT).as_posix(),
                "cacheVariables": cache_vars,
            }
        )
        build_presets.append({"name": pname, "configurePreset": pname})

    data = {
        "version": 6,
        "cmakeMinimumRequired": {"major": 3, "minor": 22, "patch": 0},
        "configurePresets": configure_presets,
        "buildPresets": build_presets,
        "vendor": {
            "voidpointer": {
                "debugger": _path_for_cmake(
                    tools["CMAKE_C_COMPILER"].with_name(host_exe("riscv-wch-elf-gdb"))
                )
            }
        },
    }
    CMAKE_USER_PRESETS.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {CMAKE_USER_PRESETS.relative_to(ROOT)}")


def write_env_local(
    openocd_exe: Path,
    openocd_cfg: Path,
    overwrite: bool,
    profile: Profile = Profile.DEV,
) -> None:
    if ENV_LOCAL.exists() and not overwrite:
        print(f"Keeping existing {ENV_LOCAL.relative_to(ROOT)}")
        return

    scripts_dir = openocd_scripts_dir(openocd_exe)
    parts = [
        "# Local machine configuration for VoidPointer.",
        "# Generated by `python tools/dev-task.py init`.",
        "# `.env.local` is ignored by git.",
        "",
        f'export VP_PROFILE="{profile.value}"',
        f'export WCH_OPENOCD_EXE="{_path_for_cmake(openocd_exe)}"',
        f'export WCH_OPENOCD_CFG="{_path_for_cmake(openocd_cfg)}"',
    ]
    if scripts_dir is not None:
        parts.append(f'export WCH_OPENOCD_SCRIPTS="{_path_for_cmake(scripts_dir)}"')
    parts.append("")

    ENV_LOCAL.write_text("\n".join(parts), encoding="utf-8")
    print(f"Wrote {ENV_LOCAL.relative_to(ROOT)}")


# ---------------------------------------------------------------------------
# State checks
# ---------------------------------------------------------------------------

def is_initialized() -> bool:
    if not CMAKE_USER_PRESETS.exists() or not ENV_LOCAL.exists():
        return False
    presets_text = CMAKE_USER_PRESETS.read_text(encoding="utf-8")
    env_text = ENV_LOCAL.read_text(encoding="utf-8")
    return PLACEHOLDER_PATH not in presets_text and PLACEHOLDER_PATH not in env_text


def ensure_initialized_for_configure() -> None:
    if not CMAKE_USER_PRESETS.exists():
        raise SystemExit("Missing CMakeUserPresets.json. Run `python tools/dev-task.py init` first.")
    if PLACEHOLDER_PATH in CMAKE_USER_PRESETS.read_text(encoding="utf-8"):
        raise SystemExit(
            "CMakeUserPresets.json still contains template paths. Run `python tools/dev-task.py init`."
        )


def ensure_initialized_for_download() -> None:
    if not ENV_LOCAL.exists():
        raise SystemExit("Missing .env.local. Run `python tools/dev-task.py init` first.")
    if PLACEHOLDER_PATH in ENV_LOCAL.read_text(encoding="utf-8"):
        raise SystemExit(
            ".env.local still contains template paths. Run `python tools/dev-task.py init`."
        )


def _ensure_preset_exists(profile: Profile) -> None:
    if not CMAKE_USER_PRESETS.exists():
        raise SystemExit("Missing CMakeUserPresets.json. Run init first.")

    data = json.loads(CMAKE_USER_PRESETS.read_text(encoding="utf-8"))
    existing = data.get("configurePresets", [])
    if any(p.get("name") == profile.preset() for p in existing):
        return

    try:
        cache = data["configurePresets"][0]["cacheVariables"]
    except (KeyError, IndexError):
        raise SystemExit(
            "Cannot parse CMakeUserPresets.json. "
            "Run `python tools/dev-task.py init` to regenerate."
        )

    tools: dict[str, Path] = {}
    for key in TOOLCHAIN_PROGRAMS:
        val = cache.get(key)
        if not val:
            raise SystemExit(
                f"Missing {key} in CMakeUserPresets.json. "
                "Run `python tools/dev-task.py init` to regenerate."
            )
        tools[key] = Path(val)

    ninja_val = cache.get("CMAKE_MAKE_PROGRAM")
    if not ninja_val:
        raise SystemExit(
            "Missing CMAKE_MAKE_PROGRAM in CMakeUserPresets.json. "
            "Run `python tools/dev-task.py init` to regenerate."
        )

    write_cmake_user_presets(tools, Path(ninja_val), overwrite=True)
    print(f"Added missing preset '{profile.preset()}' to CMakeUserPresets.json.")


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

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

    print("\nInitialization complete. Running initial refresh for dev...")
    args.profile = "dev"
    refresh(args)


def configure(args: argparse.Namespace) -> None:
    ensure_initialized_for_configure()
    profile = Profile.resolve(args.profile)
    _ensure_preset_exists(profile)
    run(["cmake", "--preset", profile.preset()])


def build(args: argparse.Namespace) -> None:
    ensure_initialized_for_configure()
    profile = Profile.resolve(args.profile)
    _ensure_preset_exists(profile)

    if not (profile.build_dir() / "CMakeCache.txt").exists():
        run(["cmake", "--preset", profile.preset()])

    run(["cmake", "--build", "--preset", profile.preset()])


def refresh_clangd(args: argparse.Namespace) -> None:
    profile = Profile.resolve(args.profile)
    cc = profile.compile_commands()
    if not cc.exists():
        raise SystemExit(f"Missing {cc}. Run configure first.")
    shutil.copy2(cc, COMPILE_COMMANDS_LINK)
    print(f"Copied {cc} -> {COMPILE_COMMANDS_LINK}")


def refresh(args: argparse.Namespace) -> None:
    configure(args)
    refresh_clangd(args)


def format_sources(_: argparse.Namespace) -> None:
    clang_format = shutil.which("clang-format")
    if clang_format is None:
        raise SystemExit("clang-format not found in PATH. Install LLVM/Clang.")

    c_like_files = sorted(
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "platform").rglob("*")
        if path.is_file() and path.suffix.lower() in (".c", ".h", ".cpp", ".hpp")
    )

    if c_like_files:
        run([clang_format, "-i", *c_like_files])
    else:
        print("No C/C++ sources found under platform/")

    run(["cargo", "fmt", "--manifest-path", "core/Cargo.toml"])


def download_only(args: argparse.Namespace) -> None:
    ensure_initialized_for_download()
    load_env_local()

    profile = Profile.resolve(args.profile)
    firmware = Path(args.firmware) if args.firmware else profile.elf()
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
        import serial  # type: ignore[import-untyped]
    except ImportError as exc:
        raise SystemExit(
            "pyserial is required. Install with: python -m pip install pyserial"
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


def test(_: argparse.Namespace) -> None:
    run_cargo(["test", "--lib", "--target", "x86_64-pc-windows-msvc"])


def coverage(args: argparse.Namespace) -> None:
    cmd = ["+nightly", "llvm-cov", "--html", "--target", "x86_64-pc-windows-msvc"]
    if args.open_html:
        cmd.append("--open")
    run_cargo(cmd)


def switch_profile(args: argparse.Namespace) -> None:
    if args.profile is None:
        print(f"Active profile: {Profile.from_env().value}")
        return

    target = Profile(args.profile)
    load_env_local()

    if not ENV_LOCAL.exists():
        raise SystemExit("Run `python tools/dev-task.py init` first.")

    lines = ENV_LOCAL.read_text(encoding="utf-8").splitlines()
    new_lines: list[str] = []
    found = False
    for line in lines:
        if line.startswith("export VP_PROFILE="):
            new_lines.append(f'export VP_PROFILE="{target.value}"')
            found = True
        else:
            new_lines.append(line)
    if not found:
        new_lines.append(f'export VP_PROFILE="{target.value}"')

    ENV_LOCAL.write_text("\n".join(new_lines) + "\n", encoding="utf-8")
    print(f"Switched to {target.value} profile.")
    print()
    refresh(args)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command",
        choices=[
            "init",
            "profile",
            "refresh",
            "build",
            "format",
            "download",
            "download-only",
            "serial",
            "test",
            "coverage",
        ],
    )
    parser.add_argument(
        "-p",
        "--profile",
        choices=["dev", "release"],
        default=None,
        help="Build profile (overrides VP_PROFILE in .env.local)",
    )
    parser.add_argument(
        "-f",
        "--firmware",
        help="Firmware ELF path. Defaults to the active profile's build directory.",
    )
    parser.add_argument(
        "--open",
        "-o",
        action="store_true",
        dest="open_html",
        help="Open HTML coverage report in browser.",
    )

    args = parser.parse_args()

    commands = {
        "init": init,
        "profile": switch_profile,
        "refresh": refresh,
        "build": build,
        "format": format_sources,
        "download": download,
        "download-only": download_only,
        "serial": serial_monitor,
        "test": test,
        "coverage": coverage,
    }
    commands[args.command](args)


if __name__ == "__main__":
    main()
