"""G5 correctness, recovery and bounded resource pressure; no latency claims."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--probe', action='store_true', help='Minimum directed cases only; not the full G5 gate')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out/'working-tree.patch').write_bytes(subprocess.check_output(['git', 'diff', '--binary', 'HEAD', '--', 'core', 'ui', 'tools'], cwd=root))
    files = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', 'core', 'ui', 'tools'], cwd=root, text=True).splitlines()
    sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    (out/'invocation.json').write_text(json.dumps(dict(
        head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        status=subprocess.check_output(['git', 'status', '--short'], cwd=root, text=True),
        sources={f: sha(root/f) for f in files if (root/f).is_file()},
        binaries={name: sha(root/'build/bin/Release'/name) for name in ['drawverse.exe', 'paint_ffi.dll', 'qt_composition_stability_tests.exe']},
        scope='minimum directed probe' if args.probe else 'G5 frozen correctness/recovery/payload quota/DPI matrix',
    ), indent=2), encoding='utf-8')
    cases = [('software', 1, 1), ('d3d11', 1, 1)]
    if not args.probe:
        cases += [('d3d11', 2, 1), ('d3d11', 3, 1), ('d3d11', 1, 2), ('d3d11', 1, 3), ('d3d11', 1, 0)]
    results = []
    for backend, repeat, dpr in cases:
        directory = out/f'{backend}-{repeat}-dpr{dpr if dpr else "native"}'
        directory.mkdir()
        env = os.environ.copy()
        for key in list(env):
            if key.startswith(('DRAWVERSE_', 'QT_QUICK_BACKEND', 'QSG_RHI_', 'QT_SCALE_FACTOR', 'QT_SCREEN_SCALE_FACTORS', 'QT_ENABLE_HIGHDPI_SCALING')):
                env.pop(key)
        for key in ['APPDATA', 'LOCALAPPDATA', 'TMP', 'TEMP']:
            path = directory/key
            path.mkdir()
            env[key] = str(path)
        env.update(QT_QPA_PLATFORM='windows', QSG_RHI_BACKEND='d3d11', QSG_INFO='1',
                   QT_FORCE_STDERR_LOGGING='1', QSG_RHI_DISABLE_DISK_CACHE='1',
                   DRAWVERSE_G5_REPORT_DIR=str(directory))
        if dpr:
            env.update(QT_ENABLE_HIGHDPI_SCALING='0', QT_SCALE_FACTOR=str(dpr), DRAWVERSE_G5_EXPECTED_DPR=str(dpr))
        env['PATH'] = str(root/'.tools/qt/6.8.3/msvc2022_64/bin')+os.pathsep+env['PATH']
        if backend == 'software':
            env['QT_QUICK_BACKEND'] = backend
        (directory/'environment.json').write_text(json.dumps({k: v for k, v in env.items() if k.startswith(('QT_', 'QSG_', 'DRAWVERSE_'))}, indent=2), encoding='utf-8')
        functions = []
        if args.probe:
            functions = ['readbackRejectsOldVersionAndPacket', 'defaultPolicy:default', 'gpuDoesNotWaitForCpuDisplayReferences', 'blendHoverUsesCpuAndRestores',
                         'windowsAndDocumentsRejectOldCallbacks', 'budgetRecovery:group', 'faultRecovery:device', 'faultDuringMove']
        elif dpr != 1:
            functions = ['defaultPolicy', 'gpuDoesNotWaitForCpuDisplayReferences', 'blendHoverUsesCpuAndRestores',
                         'windowsAndDocumentsRejectOldCallbacks', 'faultRecovery:device', 'faultDuringMove']
        with (directory/'process.log').open('w', encoding='utf-8') as log:
            try:
                code = subprocess.run([str(root/'build/bin/Release/qt_composition_stability_tests.exe'), *functions,
                                       '-o', str(directory/'test.txt')+',txt'], env=env, stdout=log,
                                      stderr=subprocess.STDOUT, timeout=600,
                                      creationflags=subprocess.CREATE_NO_WINDOW).returncode
            except subprocess.TimeoutExpired:
                code = 'timeout'
        result = dict(backend=backend, repeat=repeat, dpr=dpr if dpr else 'native', exit=code)
        results.append(result)
        print(result, flush=True)
    passed = all(r['exit'] == 0 for r in results)
    (out/'results.json').write_text(json.dumps(dict(passed=passed, results=results), indent=2), encoding='utf-8')
    raise SystemExit(0 if passed else 1)


if __name__ == '__main__':
    main()
