"""Forward MSVC diagnostics and /showIncludes to Ninja as UTF-8."""
import subprocess
import sys
import argparse


NINJA_INCLUDE_PREFIX = b"drawverse include: "


def normalize_line(line: bytes) -> bytes:
    try:
        line.decode("utf-8")
        return line
    except UnicodeDecodeError:
        # MSVC without a console uses the Windows ANSI code page (not OEM).
        return line.decode("mbcs").encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--include-prefix")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.command:
        parser.error("A compiler command is required")
    prefix = args.include_prefix.encode("utf-8") if args.include_prefix else None
    try:
        with subprocess.Popen(args.command, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT) as process:
            for line in process.stdout:
                line = normalize_line(line)
                if prefix and line.startswith(prefix):
                    line = NINJA_INCLUDE_PREFIX + line[len(prefix):]
                sys.stdout.buffer.write(line)
                sys.stdout.buffer.flush()
            return process.wait()
    except (OSError, UnicodeError) as error:
        print(f"MSVC UTF-8 launcher failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
