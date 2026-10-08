"""Repository verification. Requires a configured compiler/linker environment."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cargo", default=os.environ.get("DRAWVERSE_CARGO", "cargo"))
    parser.add_argument("--cmake", default=os.environ.get("DRAWVERSE_CMAKE", "cmake"))
    parser.add_argument("--qt-prefix", help="Qt 6.5+ SDK prefix (or use CMAKE_PREFIX_PATH)")
    parser.add_argument("--core-only", action="store_true", help="Verify Rust/C ABI only; explicitly omit UI checks")
    args = parser.parse_args()
    cargo = shutil.which(args.cargo)
    if cargo is None:
        print("Cargo not found. Activate Rust + platform linker, or pass --cargo PATH.", file=sys.stderr)
        return 2
    root = Path(__file__).resolve().parents[1]
    cmake = shutil.which(args.cmake)
    if cmake is None:
        print("CMake not found; native C11/C++20 ABI checks cannot run.", file=sys.stderr)
        return 2
    checks = [
        ["fmt", "--all", "--", "--check"],
        ["clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"],
        ["test", "--workspace", "--all-targets", "--locked"],
        ["test", "--workspace", "--doc", "--locked"],
        ["build", "--workspace", "--release", "--locked"],
        ["run", "-p", "paint-api-gen", "--locked", "--", "--check"],
    ]
    for check in checks:
        print("+ cargo " + " ".join(check), flush=True)
        result = subprocess.run([cargo, *check], cwd=root / "core", check=False)
        if result.returncode:
            return result.returncode
    qt_prefix = args.qt_prefix
    if not qt_prefix and not os.environ.get("CMAKE_PREFIX_PATH"):
        candidates = sorted((root / ".tools" / "qt").glob("*/msvc2022_64/lib/cmake/Qt6/Qt6Config.cmake"))
        if candidates:
            qt_prefix = str(candidates[-1].parents[3])
    with_ui = not args.core_only
    abi_build = root / "build" / ("verify-ui" if with_ui else "abi")
    configure = [cmake, "-S", str(root), "-B", str(abi_build),
                 "-DCMAKE_BUILD_TYPE=Release", f"-DDRAWVERSE_CARGO_EXECUTABLE={cargo}",
                 f"-DDRAWVERSE_BUILD_UI={'ON' if with_ui else 'OFF'}"]
    if qt_prefix:
        configure.append(f"-DCMAKE_PREFIX_PATH={qt_prefix}")
    # Leave an existing build tree's generator intact. Prefer portable Ninja when available.
    if not (abi_build / "CMakeCache.txt").exists() and shutil.which("ninja"):
        configure.extend(["-G", "Ninja"])
    ctest = Path(cmake).with_name("ctest.exe" if os.name == "nt" else "ctest")
    if not ctest.is_file():
        print("CTest not found alongside CMake.", file=sys.stderr)
        return 2
    for command in [configure, [cmake, "--build", str(abi_build), "--config", "Release"],
                    [str(ctest), "--test-dir", str(abi_build), "-C", "Release",
                     "--tests-regex", "^(abi_|qt_ui_)" if with_ui else "^abi_",
                     "--no-tests=error", "--output-on-failure"]]:
        print("+ " + " ".join(command), flush=True)
        result = subprocess.run(command, cwd=root, check=False)
        if result.returncode:
            return result.returncode
    print("Rust, generated header, C11/C++20 static/shared ABI checks passed. " +
          ("Qt UI checks passed." if with_ui else "UI explicitly omitted (--core-only).") +
          " Python plugins remain unimplemented.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
