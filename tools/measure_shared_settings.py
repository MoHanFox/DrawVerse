"""Run shared-settings investigation in a new isolated directory (native Windows)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--settings', type=Path, help='Copy an INI and sibling brush-resources')
    parser.add_argument('--adopt-initial', action='store_true', help='Historical benchmark startup control')
    parser.add_argument('--documents', type=int, choices=(1, 2, 3), default=3)
    parser.add_argument('--backend', choices=('d3d11','software'), default='d3d11')
    args = parser.parse_args()
    exe, output = args.executable.resolve(), args.output.resolve()
    if not exe.is_file():
        parser.error('executable does not exist')
    source = args.settings.resolve() if args.settings else None
    if source and not source.is_file():
        parser.error('settings do not exist')
    try:
        output.mkdir(parents=True, exist_ok=False)
    except FileExistsError:
        parser.error('output already exists; refusing to overwrite evidence')
    if source:
        shutil.copyfile(source, output/'preferences.ini')
        if (source.parent/'brush-resources').is_dir():
            shutil.copytree(source.parent/'brush-resources', output/'brush-resources')
    root = Path(__file__).resolve().parents[1]
    env = {key.upper(): value for key, value in os.environ.items()}
    for name in ('TEMP', 'TMP', 'APPDATA', 'LOCALAPPDATA'):
        directory = output/name.lower()
        directory.mkdir(exist_ok=True)
        env[name] = str(directory)
    env.update(QT_QPA_PLATFORM='windows', QSG_RHI_BACKEND='d3d11', QSG_INFO='1',
               QSG_RHI_DISABLE_DISK_CACHE='1', QML_DISK_CACHE_PATH=str(output/'qml-cache'),
               DRAWVERSE_SHARED_REPORT=str(output/'report.json'),
               DRAWVERSE_SHARED_MAX_DOCUMENTS=str(args.documents),
               DRAWVERSE_SHARED_NORMAL_STARTUP=str(int(not args.adopt_initial)))
    env.pop('QT_QUICK_BACKEND', None)
    if args.backend == 'software':
        env['QT_QUICK_BACKEND'] = 'software'
    env.pop('QSG_RHI_PREFER_SOFTWARE_RENDERER', None)
    env['PATH'] = str(root/'.tools/qt/6.8.3/msvc2022_64/bin')+os.pathsep+env['PATH']
    files = ('ui/tests/SharedSettingsProbe.h', 'ui/tests/BrushParameterProbe.h',
             'ui/src/platform/InteractionTrace.h', 'tools/measure_shared_settings.py')
    metadata = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
                'executable_sha256': hashlib.sha256(exe.read_bytes()).hexdigest(),
                'source_settings_sha256': hashlib.sha256(source.read_bytes()).hexdigest() if source else None,
                'normal_startup': not args.adopt_initial,
                'backend': args.backend,
                'tool_files_sha256': {name: hashlib.sha256((root/name).read_bytes()).hexdigest() for name in files}}
    (output/'invocation.json').write_text(json.dumps(metadata, indent=2), encoding='utf-8')
    with (output/'qt.log').open('w', encoding='utf-8') as log:
        result = subprocess.run([str(exe), 'sharedSettings', '-o', str(output/'test.txt')+',txt'],
                                cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=300,
                                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
    print(output, 'exit', result.returncode, flush=True)
    if result.returncode:
        raise SystemExit(result.returncode)
    report = json.loads((output/'report.json').read_text(encoding='utf-8'))
    if not report['passed'] or report['trace_dropped']:
        raise SystemExit('Incomplete report; evidence retained')


if __name__ == '__main__':
    main()
