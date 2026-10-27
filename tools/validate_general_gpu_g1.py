"""Run G1 transport and default-display regression in isolated native windows."""
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
    sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    files = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', 'core', 'ui', 'tools'], cwd=root, text=True).splitlines()
    (out/'invocation.json').write_text(json.dumps(dict(
        head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        status=subprocess.check_output(['git', 'status', '--short'], cwd=root, text=True),
        sources={f: sha(root/f) for f in files if (root/f).is_file()},
        binaries={name: sha(root/'build/bin/Release'/name) for name in ['drawverse.exe', 'paint_ffi.dll', 'qt_composition_transport_tests.exe', 'qt_transform_tests.exe']}
    ), indent=2), encoding='utf-8')
    regressions = ['pendingMoveObservationRejectsUnrelatedState',
                   'pendingMoveGrabAssociationRejectsStaleAndInterruptedEvidence',
                   'movePreparationClearsOnceDuringOtherEdits',
                   'pickedMoveUsesLatestInputForActualTarget',
                   'residentCompletionWaitsForFinalPrimaryFrame',
                   'residentMoveUsesLatestInputBeforeBackendRoundTrip']
    results = []
    for backend in ['software', 'd3d11']:
        directory = out/backend
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
                   DRAWVERSE_G1_REPORT=str(directory/'transport.json'))
        env['PATH'] = str(root/'.tools/qt/6.8.3/msvc2022_64/bin')+os.pathsep+env['PATH']
        if backend == 'software':
            env['QT_QUICK_BACKEND'] = backend
        (directory/'environment.json').write_text(json.dumps({k: v for k, v in env.items() if k.startswith(('QT_', 'QSG_', 'DRAWVERSE_'))}, indent=2), encoding='utf-8')
        for executable, arguments in [('qt_composition_transport_tests.exe', []), ('qt_transform_tests.exe', regressions)]:
            stem = executable.removesuffix('.exe')
            with (directory/(stem+'.log')).open('w', encoding='utf-8') as log:
                try:
                    run = subprocess.run([str(root/'build/bin/Release'/executable), *arguments,
                                          '-o', str(directory/(stem+'.txt'))+',txt'], env=env,
                                         stdout=log, stderr=subprocess.STDOUT, timeout=120,
                                         creationflags=subprocess.CREATE_NO_WINDOW)
                    code = run.returncode
                except subprocess.TimeoutExpired:
                    code = 'timeout'
            result = dict(backend=backend, executable=executable, exit=code)
            results.append(result)
            print(result, flush=True)
        raw = json.loads((directory/'transport.json').read_text(encoding='utf-8')) if (directory/'transport.json').exists() else {}
        results.append(dict(backend=backend, transport_passed=bool(raw.get('passed'))))
    passed = all(r.get('exit', 0) == 0 and r.get('transport_passed', True) for r in results)
    (out/'results.json').write_text(json.dumps(dict(passed=passed, results=results), indent=2), encoding='utf-8')
    raise SystemExit(0 if passed else 1)


if __name__ == '__main__':
    main()
