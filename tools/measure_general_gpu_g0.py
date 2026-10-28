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
    parser.add_argument('--composition', action='store_true', help='G3 explicit general compositor; preserve the v4 fixture/input')
    parser.add_argument('--warp', action='store_true', help='additional v5 transform mode; pair separately from ordinary transform')
    parser.add_argument('--native-driver', action='store_true', help='v5 native event loop; pair OFF/ON runs; preserve all v4 records')
    parser.add_argument('--timing-only', action='store_true', help='v6: no captures or pixel reads; use independent correctness validation')
    parser.add_argument('--repeat', type=int, choices=range(1, 4), default=1)
    args = parser.parse_args()
    if args.timing_only and not args.native_driver:
        parser.error('--timing-only requires --native-driver for the v6 timing boundary')
    if args.warp and (not args.native_driver or args.operation != 'transform'):
        parser.error('--warp requires --native-driver --operation transform')
    root = Path(__file__).resolve().parents[1]
    exe, out = args.executable.resolve(), args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out/'working-tree.patch').write_bytes(subprocess.check_output(['git', 'diff', '--binary', 'HEAD', '--', 'core', 'ui', 'tools'], cwd=root))
    if args.repeat > 1:
        for index in range(1, args.repeat+1):
            command = [os.sys.executable, str(Path(__file__).resolve()), str(exe), str(out/f'run-{index}'),
                       '--operation', args.operation, '--size', str(args.size)]
            command += ['--warp'] if args.warp else []
            command += ['--native-driver'] if args.native_driver else []
            command += ['--timing-only'] if args.timing_only else []
            command += ['--trace'] if args.trace else []
            command += ['--composition'] if args.composition else []
            command += ['--software'] if args.software else []
            subprocess.run(command, check=True)
        return
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    files = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', 'ui', 'core', 'tools'], cwd=root, text=True).splitlines()
    invocation = dict(head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
                      branch=subprocess.check_output(['git', 'branch', '--show-current'], cwd=root, text=True).strip(),
                      diff=subprocess.check_output(['git', 'status', '--short'], cwd=root, text=True),
                      executable=str(exe), executable_sha256=sha(exe), operation=args.operation,
                      size=args.size, software=args.software, trace=args.trace, composition=args.composition, native_driver=args.native_driver, warp=args.warp, timing_only=args.timing_only,
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
    if args.trace:
        # The existing Rust recorder is bounded in memory and exports only after
        # worker shutdown. Its clock origin differs from the Qt recorder;
        # correlate sequence IDs, never subtract timestamps across clocks.
        env['DRAWVERSE_MOVE_COMPLETION_TRACE'] = str(out/'core-trace.json')
    if args.warp:
        env['DRAWVERSE_G0_WARP'] = '1'
    if args.native_driver:
        env['DRAWVERSE_G0_DRIVER'] = 'native-v5'
    # Freeze the paired reference even when the product enables GPU by default.
    env['DRAWVERSE_GPU_COMPOSITION'] = '1' if args.composition else '0'
    env['DRAWVERSE_G0_CAPTURE'] = '0' if args.timing_only else '1'
    if not args.timing_only:
        env['DRAWVERSE_CPU_DISPLAY_REFERENCE'] = '1'
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
    core_trace_path = out/'core-trace.json'
    core_trace = json.loads(core_trace_path.read_text(encoding='utf-8')) if core_trace_path.exists() else {}
    passed = code == 0 and raw.get('passed') and not any(raw.get(k, 0) for k in ['trace_dropped', 'adoptions_dropped', 'frames_dropped', 'states_dropped', 'unplanned_input_count'])
    if args.trace and args.operation != 'probe':
        passed = passed and core_trace.get('closed') and core_trace.get('dropped') == 0
    (out/'process.json').write_text(json.dumps(dict(exit=code, passed=bool(passed), core_trace_dropped=core_trace.get('dropped')), indent=2), encoding='utf-8')
    print(out, code, bool(passed), flush=True)
    raise SystemExit(0 if passed else 1)


if __name__ == '__main__':
    main()
