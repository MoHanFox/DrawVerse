"""G0 native Qt probes. Each invocation uses a fresh evidence directory/process.

No UI automation or pixel access in Python; the native executable owns both.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--operation', choices=['probe', 'move', 'transform'], default='probe')
    parser.add_argument('--size', type=int, choices=[1024, 4096], default=1024)
    parser.add_argument('--software', action='store_true')
    parser.add_argument('--trace', action='store_true')
    parser.add_argument('--repeat', type=int, choices=range(1, 4), default=1)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    exe, out = args.executable.resolve(), args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    if args.repeat > 1:
        for index in range(1, args.repeat+1):
            command = [os.sys.executable, str(Path(__file__).resolve()), str(exe), str(out/f'run-{index}'),
                       '--operation', args.operation, '--size', str(args.size)]
            command += ['--trace'] if args.trace else []
            command += ['--software'] if args.software else []
            subprocess.run(command, check=True)
        return
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    files = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', 'ui', 'core', 'tools'], cwd=root, text=True).splitlines()
    invocation = dict(head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
                      branch=subprocess.check_output(['git', 'branch', '--show-current'], cwd=root, text=True).strip(),
                      diff=subprocess.check_output(['git', 'status', '--short'], cwd=root, text=True),
                      executable=str(exe), executable_sha256=sha(exe), operation=args.operation,
                      size=args.size, software=args.software, trace=args.trace,
                      sources_sha256={f: sha(root/f) for f in files if (root/f).is_file()})
    (out/'invocation.json').write_text(json.dumps(invocation, indent=2), encoding='utf-8')
    env = os.environ.copy()
    for key, directory in [('APPDATA', 'appdata'), ('LOCALAPPDATA', 'localappdata'), ('TMP', 'tmp'), ('TEMP', 'tmp')]:
        path = out/directory
        path.mkdir(exist_ok=True)
        env[key] = str(path)
    for key in list(env):
        if key.startswith(('DRAWVERSE_', 'QT_QUICK_BACKEND', 'QSG_RHI_', 'QT_SCALE_FACTOR')):
            env.pop(key)
    env.update(QT_QPA_PLATFORM='windows', QSG_INFO='1', QSG_RHI_BACKEND='d3d11',
               DRAWVERSE_G0_REPORT=str(out/'raw.json'), DRAWVERSE_G0_SIZE=str(args.size),
               DRAWVERSE_G0_OPERATION=args.operation, DRAWVERSE_G0_TRACE=str(int(args.trace)),
               QML_DISK_CACHE_PATH=str(out/'qml-cache'), QSG_RHI_DISABLE_DISK_CACHE='1')
    if args.software:
        env['QT_QUICK_BACKEND'] = 'software'
    env['PATH'] = str(root/'.tools/qt/6.8.3/msvc2022_64/bin')+os.pathsep+env['PATH']
    (out/'environment.json').write_text(json.dumps({k: v for k, v in env.items() if k.startswith(('QT_', 'QSG_', 'QML_', 'DRAWVERSE_'))}, indent=2), encoding='utf-8')
    with (out/'qt.log').open('w', encoding='utf-8') as log:
        try:
            result = subprocess.run([str(exe), '-o', str(out/'test.txt')+',txt'], env=env, stdout=log,
                                    stderr=subprocess.STDOUT, timeout=240,
                                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            code = result.returncode
        except subprocess.TimeoutExpired:
            code = 'timeout'
    raw = json.loads((out/'raw.json').read_text(encoding='utf-8')) if (out/'raw.json').exists() else {}
    passed = code == 0 and raw.get('passed') and not any(raw.get(k, 0) for k in ['trace_dropped', 'adoptions_dropped', 'frames_dropped', 'states_dropped', 'unplanned_input_count'])
    (out/'process.json').write_text(json.dumps(dict(exit=code, passed=bool(passed)), indent=2), encoding='utf-8')
    print(out, code, bool(passed), flush=True)
    raise SystemExit(0 if passed else 1)


if __name__ == '__main__':
    main()
