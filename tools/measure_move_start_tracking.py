"""One opt-in C3 native probe process; reuse C1 environment, no focus retries."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('executable', type=Path)
    p.add_argument('output', type=Path)
    p.add_argument('--direct-w', action='store_true')
    p.add_argument('--tracking', action='store_true')
    p.add_argument('--trace', action='store_true')
    p.add_argument('--scene', type=int, choices=[1, 2], default=2)
    p.add_argument('--shader-pair', action='store_true', help='direct W1 then same-window W2 tracking')
    p.add_argument('--main-qml', action='store_true')
    p.add_argument('--production-cache', action='store_true', help='default automatic pipeline policy, fresh isolated cache')
    p.add_argument('--source-root', type=Path, help='actual source worktree of executable')
    a = p.parse_args()
    exe, out = a.executable.resolve(), a.output.resolve()
    if not exe.is_file():
        p.error('executable missing')
    try:
        out.mkdir(parents=True, exist_ok=False)
    except FileExistsError:
        p.error('output exists; retained unchanged')
    root = Path(__file__).resolve().parent.parent
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    source = a.source_root.resolve() if a.source_root else root
    files = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', 'ui', 'core', 'tools'], cwd=source, text=True).splitlines()
    (out/'invocation.json').write_text(json.dumps(dict(
        head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=source, text=True).strip(),
        executable=str(exe), executable_sha256=sha(exe),
        sources_sha256={f: sha(source/f) for f in files if (source/f).is_file()},
        source_root=str(source), tool_sha256=sha(Path(__file__)), shader_pair=a.shader_pair, main_qml=a.main_qml, production_cache=a.production_cache,
        direct_w=a.direct_w, tracking=a.tracking, trace=a.trace, scene=a.scene), indent=2), encoding='utf-8')
    env = os.environ.copy()
    for key, name in [('APPDATA', 'appdata'), ('LOCALAPPDATA', 'localappdata'), ('TEMP', 'tmp'), ('TMP', 'tmp')]:
        path = out/name
        path.mkdir(exist_ok=True)
        env[key] = str(path)
    for key in ['QT_QUICK_BACKEND', 'QSG_RHI_PREFER_SOFTWARE_RENDERER', 'DRAWVERSE_PENDING_MOVE_EVIDENCE', 'DRAWVERSE_MOVE_COMPLETION_TRACE']:
        env.pop(key, None)
    env.update(QT_QPA_PLATFORM='windows', QSG_RHI_BACKEND='d3d11', QSG_INFO='1',
               QSG_RHI_DISABLE_DISK_CACHE='1', QML_DISK_CACHE_PATH=str(out/'qml-cache'),
               DRAWVERSE_MOVE_REPORT=str(out/'raw.json'), DRAWVERSE_MOVE_SCENE=str(a.scene),
               DRAWVERSE_MOVE_TRACE=str(int(a.trace)), DRAWVERSE_C3_DIRECT_W=str(int(a.direct_w)),
               DRAWVERSE_C3_TRACKING=str(int(a.tracking)))
    env.update(DRAWVERSE_C3_SHADER_PAIR=str(int(a.shader_pair)), DRAWVERSE_C3_MAIN_QML=str(int(a.main_qml)))
    if a.production_cache:
        env.pop('QSG_RHI_DISABLE_DISK_CACHE', None)
        # Windows QStandardPaths uses Known Folders, not LOCALAPPDATA. Keep the
        # normal enabled policy but explicitly isolate this test window's file.
        env['DRAWVERSE_C3_PIPELINE_CACHE'] = str(out/'pipeline.cache')
    else:
        env.pop('DRAWVERSE_C3_PIPELINE_CACHE', None)
    if a.trace:
        env['DRAWVERSE_MOVE_COMPLETION_TRACE'] = str(out/'core.json')
    env['PATH'] = str(root/'.tools/qt/6.8.3/msvc2022_64/bin')+os.pathsep+env['PATH']
    (out/'environment.json').write_text(json.dumps({k: v for k, v in env.items() if k.startswith(('QT_', 'QSG_', 'QML_', 'DRAWVERSE_'))}, indent=2), encoding='utf-8')
    with (out/'qt.log').open('w', encoding='utf-8') as log:
        proc = subprocess.Popen([str(exe), '-o', str(out/'test.txt')+',txt'], env=env,
                                stdout=log, stderr=subprocess.STDOUT,
                                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
        try:
            code = proc.wait(timeout=240)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
            code = 'process-timeout'
    raw = json.loads((out/'raw.json').read_text(encoding='utf-8')) if (out/'raw.json').exists() else {}
    trace_problem = None
    if a.trace:
        core_path = out/'core.json'
        core = json.loads(core_path.read_text(encoding='utf-8')) if core_path.exists() else {}
        if not core.get('closed') or core.get('dropped') or raw.get('trace_dropped'):
            trace_problem = 'missing/unclosed/overflowed diagnostics; raw evidence retained'
    (out/'process.json').write_text(json.dumps(dict(exit=code, passed=raw.get('passed'),
        trace_problem=trace_problem, terminals=[r.get('terminal') for r in raw.get('rows', [])]), indent=2), encoding='utf-8')
    print(out.name, code, [r.get('terminal') for r in raw.get('rows', [])], flush=True)
    if code or not raw.get('passed') or trace_problem:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
