"""Run the opt-in native brush interaction baseline (no performance pass threshold)."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--presets', type=int, choices=(0, 256), default=0)
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument('--trace', action='store_true')
    parser.add_argument('--a2-only', action='store_true')
    parser.add_argument('--count', type=int, default=51, help='First input plus 50 normal samples by default')
    args = parser.parse_args()
    if args.repeats < 1:
        parser.error('--repeats must be a positive integer')
    if args.count < 2:
        parser.error('--count must be at least 2 (first and normal samples)')
    executable, output = args.executable.resolve(), args.output.resolve()
    if not executable.is_file():
        parser.error(f'executable does not exist: {executable}')
    try:
        output.mkdir(parents=True, exist_ok=False)
    except FileExistsError:
        parser.error(f'output directory already exists; all evidence retained: {output}')
    summary = []
    root = Path(__file__).resolve().parent.parent
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    diff = subprocess.check_output(['git', 'diff', 'HEAD', '--', 'ui', 'tools/measure_brush_interaction.py'], cwd=root)
    metadata = {'head': head, 'tracked_diff_sha256': hashlib.sha256(diff).hexdigest(),
                'executable_sha256': hashlib.sha256(executable.read_bytes()).hexdigest(), 'arguments': vars(args).copy()}
    metadata['arguments'] = {key: str(value) if isinstance(value, Path) else value for key, value in metadata['arguments'].items()}
    (output/'invocation.json').write_text(json.dumps(metadata, indent=2), encoding='utf-8')
    failures = []
    for run in range(1, args.repeats+1):
        directory = output/f'run-{run}'
        directory.mkdir(exist_ok=False)
        env = os.environ.copy()
        for key, name in [('APPDATA', 'appdata'), ('LOCALAPPDATA', 'localappdata'), ('TEMP', 'tmp'), ('TMP', 'tmp')]:
            path = directory/name
            path.mkdir(exist_ok=True)
            env[key] = str(path)
        env['QSG_RHI_DISABLE_DISK_CACHE'] = '1'
        env['QML_DISK_CACHE_PATH'] = str(directory/'qml-cache')
        env.pop('QT_QUICK_BACKEND', None)
        env.pop('QSG_RHI_PREFER_SOFTWARE_RENDERER', None)
        env.update(QT_QPA_PLATFORM='windows', QSG_RHI_BACKEND='d3d11', QSG_INFO='1',
                   DRAWVERSE_INTERACTION_REPORT=str(directory/'raw.json'),
                   DRAWVERSE_INTERACTION_PRESETS=str(args.presets), DRAWVERSE_INTERACTION_COUNT=str(args.count),
                   DRAWVERSE_INTERACTION_TRACE=str(int(args.trace)), DRAWVERSE_INTERACTION_A2_ONLY=str(int(args.a2_only)))
        # Reuse the installed SDK DLLs; the Qt native test window remains visible.
        env['PATH'] = str(root/'.tools/qt/6.8.3/msvc2022_64/bin')+os.pathsep+env['PATH']
        with (directory/'qt.log').open('w', encoding='utf-8') as log:
            process = subprocess.Popen([str(executable), '-o', str(directory/'test.txt')+',txt'], env=env,
                                       stdout=log, stderr=subprocess.STDOUT,
                                       creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            try:
                code = process.wait(timeout=600)
            except subprocess.TimeoutExpired:
                process.kill(); process.wait(); code = 'process-timeout'
        raw = directory/'raw.json'
        if not raw.is_file():
            failures.append({'run': run, 'exit': code, 'error': 'No report; inspect test.txt and qt.log'})
            print(f'run {run}: no report, exit {code}', flush=True)
            continue
        data = json.loads(raw.read_text(encoding='utf-8'))
        if code or data.get('error') or data['trace_dropped']:
            failures.append({'run': run, 'exit': code, 'error': data.get('error'), 'trace_dropped': data['trace_dropped']})
        for row in data['rows']:
            item = {'run': run, 'presets': data['preset_count'], 'scene': row['scene'],
                    'trace': data['trace_enabled'], 'attempts': row['attempts'], 'completed': row['completed'], 'actual_changes': row['actual_changes'],
                    'failures': row['failures'], 'timeouts': row['timeouts'],
                    'first_sync_us': row['samples'][0]['sync_us'],
                    **{f'warm_sync_{key}': value for key, value in row['warm_sync_us'].items()},
                    **{f'heartbeat_{key}': value for key, value in row['heartbeat_ms'].items()},
                    'actual_gap_p50_ms': row['actual_input_gap_ms']['p50'],
                    'tail_observation_ms': row['post_input_quiescence_observation_ms'],
                    **{key: row[key] for key in ('activeColorsChanged', 'settingsChanged', 'brushChanged', 'persistRequested')}}
            summary.append(item)
        print(f'run {run}: exit {code}, presets {data["preset_count"]}, rows {len(data["rows"])}', flush=True)
    (output/'summary.json').write_text(json.dumps({'runs': summary, 'failures': failures}, indent=2), encoding='utf-8')
    if summary:
        with (output/'summary.csv').open('w', newline='', encoding='utf-8') as file:
            writer = csv.DictWriter(file, fieldnames=list(summary[0])); writer.writeheader(); writer.writerows(summary)
    if failures or any(row['failures'] or row['timeouts'] for row in summary):
        raise SystemExit('Incomplete or failed measurements are retained; inspect reports')


if __name__ == '__main__':
    main()
