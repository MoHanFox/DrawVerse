"""C1 native visible layer-move investigation. No cross-machine performance gate."""
import argparse
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess


def stats(values):
    values = sorted(values)
    return dict(n=len(values), **{key: values[max(0, math.ceil(len(values)*p)-1)] if values else None
                                for key, p in [('p50', .5), ('p95', .95), ('max', 1)]})


def summarize(data, scene, run):
    trace = data['trace']
    result = []
    for row in data['rows']:
        lo = row['inputs'][0]['start_ns']
        hi = row['release_ns'] + max(0, row['release_to_full_frame_ms'])*1e6
        events = [e for e in trace if lo <= e['start_ns'] <= hi]
        item = dict(scene=scene, run=run, phase=row['phase'], trace=data['trace_enabled'],
                    attempts=row['attempts'], completed=row['completed'], timeouts=row['timeouts'],
                    passed=data['passed'], observation_dropped=row['observation_dropped'], heartbeat_dropped=row['heartbeat_dropped'], press_us=row['inputs'][0]['sync_us'],
                    release_us=row['inputs'][-1]['sync_us'],
                    press_assets_ready=row['press_assets_ready'], press_upload_batches=row['press_upload_batches'],
                    release_to_metadata_ms=row['release_to_metadata_ms'],
                    release_to_full_frame_ms=row['release_to_full_frame_ms'],
                    history_delta=row['history_delta'], resident_sources=len(row['resident_sources']))
        for field in ['move_sync_us', 'actual_send_gap_ms', 'heartbeat_ms']:
            item.update({field+'_'+k: v for k, v in row[field].items()})
        for name in ['move.submit', 'move.worker.readResident', 'move.worker.copyFrame',
                     'move.render.adoptGeometry', 'move.render.createTextures',
                     'move.render.uploadBatch', 'move.render.replaceFrameTexture']:
            item.update({name+'_'+k: v for k, v in stats([e['duration_us'] for e in events if e['name'] == name]).items()})
        item['changed_deltas'] = sum(e['name'] == 'move.changedDelta.x' for e in events)
        preparations = [e for e in trace if e['name'] == 'move.gui.acceptPreparation'
                        and row['prepare_start_ns'] <= e['start_ns'] < lo]
        item['prepare_request_to_gui_ms'] = ((preparations[0]['start_ns']-row['prepare_start_ns'])/1e6
                                            if preparations else None)
        uploads = [e for e in trace if e['name'] == 'move.render.uploadBatch'
                   and e['value'] == row['press_source'] and row['prepare_start_ns'] <= e['start_ns'] < lo]
        item['prepare_request_to_upload_recorded_ms'] = ((uploads[0]['start_ns']+uploads[0]['duration_us']*1000-row['prepare_start_ns'])/1e6
                                                        if uploads else None)

        # Join a submitted sequence to the first GUI-observed completed_sequence.
        # This interval includes core queue/work AND worker polling + GUI delivery.
        metadata = sorted((e for e in trace if e['name'] == 'move.gui.metadata'), key=lambda e: e['start_ns'])
        observed = []
        for e in events:
            if e['name'] != 'move.submit' or not e['value']:
                continue
            match = next((m for m in metadata if m['start_ns'] >= e['start_ns'] and m['value'] >= e['value']), None)
            if match:
                observed.append((match['start_ns']-e['start_ns'])/1e6)
        item.update({'submit_to_gui_completed_ms_'+k: v for k, v in stats(observed).items()})
        # Immutable image size, host bytes only, no GPU residency/compression claim.
        item['created_source_host_bytes'] = sum(e['value'] for e in events if e['name'] == 'move.texture.hostBytes')
        item['copied_frame_bytes'] = sum(e['value'] for e in events if e['name'] == 'move.copyFrame.bytes')
        item['submit_failures'] = sum(e['name'] == 'move.submit.status' and e['value'] != 0 for e in events)
        result.append(item)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--scenes', type=int, choices=range(1, 5), nargs='+', default=[1, 2, 3, 4])
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument('--trace', action='store_true')
    args = parser.parse_args()
    if args.repeats < 1:
        parser.error('--repeats must be positive')
    executable, output = args.executable.resolve(), args.output.resolve()
    if not executable.is_file():
        parser.error('executable missing')
    try:
        output.mkdir(parents=True, exist_ok=False)
    except FileExistsError:
        parser.error('output exists; retained without modification')
    root = Path(__file__).resolve().parent.parent
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    diff = subprocess.check_output(['git', 'diff', 'HEAD', '--', 'ui', 'tools/measure_layer_move.py'], cwd=root)
    metadata = dict(product_baseline='b948dccbc2fd767af3a999ee9a03e108455c44cb', tool_revision=revision,
                    tracked_diff_sha256=hashlib.sha256(diff).hexdigest(), executable_sha256=digest(executable),
                    tool_sha256=digest(Path(__file__)), repeats=args.repeats, scenes=args.scenes, trace=args.trace)
    metadata['runtime_dlls_sha256'] = {p.name: digest(p) for p in executable.parent.glob('*paint*.dll')}
    (output/'invocation.json').write_text(json.dumps(metadata, indent=2), encoding='utf-8')
    summaries, failures = [], []
    for scene in args.scenes:
        for run in range(1, args.repeats+1):
            directory = output/f's{scene}-run-{run}'
            directory.mkdir()
            env = os.environ.copy()
            for key, name in [('APPDATA', 'appdata'), ('LOCALAPPDATA', 'localappdata'), ('TEMP', 'tmp'), ('TMP', 'tmp')]:
                path = directory/name
                path.mkdir(exist_ok=True)
                env[key] = str(path)
            env.pop('QT_QUICK_BACKEND', None)
            env.pop('QSG_RHI_PREFER_SOFTWARE_RENDERER', None)
            env.update(QT_QPA_PLATFORM='windows', QSG_RHI_BACKEND='d3d11', QSG_INFO='1',
                       QSG_RHI_DISABLE_DISK_CACHE='1', QML_DISK_CACHE_PATH=str(directory/'qml-cache'),
                       DRAWVERSE_MOVE_REPORT=str(directory/'raw.json'), DRAWVERSE_MOVE_SCENE=str(scene),
                       DRAWVERSE_MOVE_TRACE=str(int(args.trace)))
            env['PATH'] = str(root/'.tools/qt/6.8.3/msvc2022_64/bin')+os.pathsep+env['PATH']
            with (directory/'qt.log').open('w', encoding='utf-8') as log:
                process = subprocess.Popen([str(executable), '-o', str(directory/'test.txt')+',txt'], env=env,
                                           stdout=log, stderr=subprocess.STDOUT,
                                           creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
                try:
                    code = process.wait(timeout=360)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                    code = 'process-timeout'
            if (directory/'raw.json').is_file():
                data = json.loads((directory/'raw.json').read_text(encoding='utf-8'))
                summaries.extend(summarize(data, scene, run))
                if code or not data['passed'] or data['trace_dropped']:
                    failures.append(dict(scene=scene, run=run, exit=code, passed=data['passed'], dropped=data['trace_dropped']))
            else:
                failures.append(dict(scene=scene, run=run, exit=code, error='no raw report'))
            print(f'S{scene} run {run}: exit {code}', flush=True)
            (output/'summary.json').write_text(json.dumps(dict(runs=summaries, failures=failures), indent=2), encoding='utf-8')
    if summaries:
        with (output/'summary.csv').open('w', newline='', encoding='utf-8') as file:
            writer = csv.DictWriter(file, fieldnames=list(summaries[0]))
            writer.writeheader()
            writer.writerows(summaries)
    if failures:
        raise SystemExit('Failures retained; inspect raw reports and native logs')


if __name__ == '__main__':
    main()
