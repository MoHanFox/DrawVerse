"""Exercise the real Git checkout rules, including Windows autocrlf behavior."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class CheckoutTests(unittest.TestCase):
    def test_autocrlf_checkout_keeps_source_lf_and_binary_bytes(self):
        git = shutil.which("git")
        self.assertIsNotNone(git, "Git is required for repository checkout verification")
        root = Path(__file__).resolve().parents[2]
        source = "// 中文 source\nsecond line\n".encode("utf-8")
        text_files = ("core.rs", "paint_api.h", "Main.qml", "check.py", "README.md", "setup.ps1")
        binary = b"\x89PNG\r\n\x1a\n\x00\xfforiginal\r\n"
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)

            def run(*args):
                return subprocess.run([git, *args], cwd=work, check=True,
                                      stdout=subprocess.PIPE, stderr=subprocess.PIPE)

            run("init", "--quiet")
            run("config", "core.autocrlf", "true")
            (work / ".gitattributes").write_bytes((root / ".gitattributes").read_bytes())
            for name in text_files:
                (work / name).write_bytes(source)
            for name in ("image.png", "archive.zip"):
                (work / name).write_bytes(binary)
            run("add", ".")
            # Restore tracked files through checkout, without commits or user Git identity.
            for name in (*text_files, "image.png", "archive.zip"):
                (work / name).unlink()
            run("checkout-index", "--all", "--force")
            for name in text_files:
                self.assertEqual((work / name).read_bytes(), source, name)
            for name in ("image.png", "archive.zip"):
                self.assertEqual((work / name).read_bytes(), binary, name)


if __name__ == "__main__":
    unittest.main()
