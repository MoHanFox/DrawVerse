"""MSVC diagnostics and real Ninja dependencies from an IDE-style process."""
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
LAUNCHER = ROOT / "tools/msvc_utf8.py"
spec = importlib.util.spec_from_file_location("msvc_utf8", LAUNCHER)
msvc_utf8 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(msvc_utf8)


class CompilerOutputTests(unittest.TestCase):
    def test_utf8_is_preserved(self):
        output = "注意: 包含文件:  C:/中文/header.h\r\n".encode("utf-8")
        self.assertEqual(msvc_utf8.normalize_line(output), output)

    @unittest.skipUnless(os.name == "nt", "Windows ANSI encoding")
    def test_ansi_is_converted(self):
        # Use a character supported by the active ANSI code page.
        for message in ("注意: 包含文件:  C:/中文/header.h\r\n", "café\r\n"):
            try:
                output = message.encode("mbcs")
            except UnicodeEncodeError:
                continue
            if output != message.encode("utf-8"):
                self.assertEqual(msvc_utf8.normalize_line(output), message.encode("utf-8"))
                return
        self.skipTest("Active ANSI code page is UTF-8 or lacks these characters")

    def test_stderr_and_failure_are_forwarded(self):
        output = "compiler error: 中文\n".encode("utf-8")
        command = "import sys; sys.stderr.buffer.write(" + repr(output) + "); sys.exit(7)"
        result = subprocess.run([sys.executable, str(LAUNCHER), sys.executable, "-c", command],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.assertEqual(result.returncode, 7)
        self.assertEqual(result.stdout, output)
        self.assertEqual(result.stderr, b"")

    def test_include_prefix_is_ascii_and_paths_remain_utf8(self):
        prefix = "注意: 包含文件:  "
        output = (prefix + " C:/中文/header.h\r\nordinary diagnostic\n").encode("utf-8")
        command = "import sys; sys.stdout.buffer.write(" + repr(output) + ")"
        result = subprocess.run([sys.executable, str(LAUNCHER), "--include-prefix", prefix,
                                 sys.executable, "-c", command], stdout=subprocess.PIPE, check=True)
        self.assertEqual(result.stdout,
                         b"drawverse include:  " + "C:/中文/header.h\r\nordinary diagnostic\n".encode("utf-8"))


@unittest.skipUnless(os.name == "nt", "MSVC Ninja integration requires Windows")
class MsvcNinjaTests(unittest.TestCase):
    def test_detached_configure_build_and_header_rebuild(self):
        cmake = shutil.which(os.environ.get("DRAWVERSE_CMAKE", "cmake"))
        ninja = shutil.which("ninja")
        compiler = shutil.which("cl")
        if not all((cmake, ninja, compiler)):
            self.skipTest("Requires CMake, Ninja and an activated MSVC environment")
        with tempfile.TemporaryDirectory(prefix="drawverse ninja 中文 ") as directory:
            source = Path(directory)
            build = source / "build"
            header = source / "shared.h"
            marker = source / "compiler launcher.py"
            log = source / "launcher.log"
            marker.write_text(
                "import pathlib, subprocess, sys\n"
                "with pathlib.Path(__file__).with_name('launcher.log').open('a') as stream:\n"
                "    stream.write('compile\\n')\n"
                "sys.exit(subprocess.call(sys.argv[1:]))\n", encoding="utf-8")
            module = (ROOT / "cmake/MsvcNinjaDependencies.cmake").as_posix()
            (source / "CMakeLists.txt").write_text(
                "cmake_minimum_required(VERSION 3.24)\n"
                "project(MsvcDependencies LANGUAGES C CXX)\n"
                "find_package(Python3 3.9 REQUIRED COMPONENTS Interpreter)\n"
                f'include("{module}")\n'
                "add_library(c_probe OBJECT sample.c)\n"
                "add_library(cpp_probe OBJECT sample.cpp)\n", encoding="utf-8")
            header.write_text("#define SHARED_VALUE 1\n", encoding="utf-8")
            for filename in ("sample.c", "sample.cpp"):
                (source / filename).write_text(
                    '#include "shared.h"\n#include <stddef.h>\n'
                    'int sample(void) { return SHARED_VALUE + (int)sizeof(size_t); }\n',
                    encoding="utf-8")
            env = os.environ.copy()
            env["VSLANG"] = "2052"

            def run(*args, detached=True):
                result = subprocess.run(args, env=env, stdout=subprocess.PIPE,
                                        stderr=subprocess.STDOUT,
                                        creationflags=subprocess.DETACHED_PROCESS if detached else 0)
                self.assertEqual(result.returncode, 0, result.stdout.decode("utf-8", errors="replace"))
                return result.stdout

            user_launcher = f"{Path(sys.executable).as_posix()};{marker.as_posix()}"
            configure = [cmake, "-S", str(source), "-B", str(build), "-G", "Ninja",
                         f"-DCMAKE_MAKE_PROGRAM={ninja}", f"-DCMAKE_C_COMPILER={compiler}",
                         f"-DCMAKE_CXX_COMPILER={compiler}", f"-DPython3_EXECUTABLE={sys.executable}",
                         f"-DCMAKE_C_COMPILER_LAUNCHER={user_launcher}",
                         f"-DCMAKE_CXX_COMPILER_LAUNCHER={user_launcher}"]
            for _ in range(2):
                run(*configure)
                rules = build / "CMakeFiles/rules.ninja"
                self.assertTrue(rules.is_file(), "Configure succeeded but rules.ninja is missing")
                rules.read_text(encoding="utf-8")
            run(cmake, "--build", str(build))
            self.assertEqual(log.read_text().splitlines(), ["compile", "compile"])
            objects = [build / "CMakeFiles/c_probe.dir/sample.c.obj",
                       build / "CMakeFiles/cpp_probe.dir/sample.cpp.obj"]
            self.assertEqual(len(objects), 2)
            before = [obj.stat().st_mtime_ns for obj in objects]
            dependencies = run(ninja, "-C", str(build), "-t", "deps")
            self.assertIn(b"shared.h", dependencies)
            header.write_text("#define SHARED_VALUE 2\n", encoding="utf-8")
            run(cmake, "--build", str(build), detached=False)
            self.assertTrue(all(obj.stat().st_mtime_ns > old for obj, old in zip(objects, before)))
            self.assertEqual(len(log.read_text().splitlines()), 4)
            run(cmake, "--build", str(build))
            self.assertEqual(len(log.read_text().splitlines()), 4)


if __name__ == "__main__":
    unittest.main()
