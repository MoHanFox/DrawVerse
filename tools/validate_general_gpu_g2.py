"""G2/G3 correctness in isolated native windows; this is not a latency benchmark."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    root = Path(__file__).resolve().parents[1]
    out = Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    files = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', 'core', 'ui', 'tools'], cwd=root, text=True).splitlines()
    sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    (out/'invocation.json').write_text(json.dumps(dict(
        head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        status=subprocess.check_output(['git', 'status', '--short'], cwd=root, text=True),
        sources={f: sha(root/f) for f in files if (root/f).is_file()},
        binaries={name: sha(root/'build/bin/Release'/name) for name in ['drawverse.exe', 'paint_ffi.dll', 'qt_gpu_composition_tests.exe']},
        purpose='correctness, captures permitted; no latency conclusions'
    ), indent=2), encoding='utf-8')
    results = []
    for backend, repeat in [('software', 1), ('d3d11', 1), ('d3d11', 2), ('d3d11', 3)]:
        directory = out/f'{backend}-{repeat}'
        directory.mkdir()
        env = os.environ.copy()
        for key in list(env):
            if key.startswith(('DRAWVERSE_', 'QT_QUICK_BACKEND', 'QSG_RHI_', 'QT_SCALE_FACTOR')):
                env.pop(key)
        for key in ['APPDATA', 'LOCALAPPDATA', 'TMP', 'TEMP']:
            path = directory/key
            path.mkdir()
            env[key] = str(path)
        env.update(QT_QPA_PLATFORM='windows', QSG_RHI_BACKEND='d3d11', QSG_INFO='1',
                   QT_FORCE_STDERR_LOGGING='1', QSG_RHI_DISABLE_DISK_CACHE='1',
                   DRAWVERSE_G2_REPORT_DIR=str(directory))
        env['PATH'] = str(root/'.tools/qt/6.8.3/msvc2022_64/bin')+os.pathsep+env['PATH']
        if backend == 'software':
            env['QT_QUICK_BACKEND'] = backend
        (directory/'environment.json').write_text(json.dumps({k: v for k, v in env.items() if k.startswith(('QT_', 'QSG_', 'DRAWVERSE_'))}, indent=2), encoding='utf-8')
        with (directory/'process.log').open('w', encoding='utf-8') as log:
            try:
                code = subprocess.run([str(root/'build/bin/Release/qt_gpu_composition_tests.exe'),
                                       '-o', str(directory/'test.txt')+',txt'], env=env,
                                      stdout=log, stderr=subprocess.STDOUT, timeout=180,
                                      creationflags=subprocess.CREATE_NO_WINDOW).returncode
            except subprocess.TimeoutExpired:
                code = 'timeout'
        result = dict(backend=backend, repeat=repeat, exit=code)
        results.append(result)
        print(result, flush=True)
    passed = all(r['exit'] == 0 for r in results)
    (out/'results.json').write_text(json.dumps(dict(passed=passed, results=results), indent=2), encoding='utf-8')
    raise SystemExit(0 if passed else 1)


if __name__ == '__main__':
    main()
