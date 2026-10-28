"""Runner admission tests: never launch a GUI or replace an existing experiment."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


RUNNER = Path(__file__).with_name('measure_brush_interaction.py')


class OutputAdmissionTests(unittest.TestCase):
    def invoke(self, output, *arguments):
        return subprocess.run([sys.executable, str(RUNNER), sys.executable,
                               str(output), *arguments], capture_output=True, text=True)

    def test_invalid_parameters_do_not_create_output(self):
        with tempfile.TemporaryDirectory() as directory:
            for index, arguments in enumerate([('--repeats', '0'), ('--repeats', '-1'),
                              ('--repeats', '1.5'), ('--count', '0'),
                              ('--count', '1'), ('--count', '-1')]):
                with self.subTest(arguments=arguments):
                    output = Path(directory)/f'must-not-exist-{index}'
                    result = self.invoke(output, *arguments)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn('error:', result.stderr)
                    self.assertFalse(output.exists())

    def test_existing_output_is_untouched(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)/'experiment'
            (output/'run-1').mkdir(parents=True)
            (output/'invocation.json').write_bytes(b'original invocation\n')
            (output/'run-1'/'evidence.bin').write_bytes(b'original evidence\x00')
            def snapshot():
                return {str(path.relative_to(output)): (path.stat().st_mtime_ns,
                        path.read_bytes() if path.is_file() else None)
                        for path in [output, *output.rglob('*')]}
            before = snapshot()
            result = self.invoke(output, '--repeats', '1', '--count', '2')
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(before, snapshot())
            self.assertIn('already exists', result.stderr)


if __name__ == '__main__':
    unittest.main()
