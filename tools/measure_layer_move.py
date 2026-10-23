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


def audit_row(data, row):
    """No nearest-time identity guesses. Old fields retain their original meaning."""
    trace = data.get('trace', [])
    children = {}
    for event in trace:
        children.setdefault(event.get('parent'), []).append(event)
    points = lambda e: {c['name']: c['value'] for c in children.get(e['id'], [])}
    inputs = row.get('inputs', [])
    press = next((i for i in inputs if i['type'] == 2), None)
    moves = [i for i in inputs if i['type'] == 5]
    release = next((i for i in inputs if i['type'] == 3), None)
    start = press['start_ns'] if press else None
    first = moves[0]['start_ns'] if moves else None
    rel = release['start_ns'] if release else None
    end = row.get('terminal_ns')
    if not end and rel and row.get('completed') and row.get('release_to_full_frame_ms', -1) >= 0:
        end = rel + row['release_to_full_frame_ms']*1e6
    boundaries = {'preparation': (row.get('prepare_start_ns'), start),
                  'operation_start': (start, first), 'sustained_drag': (first, rel or end),
                  'completion': (rel, end)}
    phase_events = {name: [] for name in boundaries}
    cross, unassigned = [], []
    low = row.get('prepare_start_ns', start)
    # For incomplete historical rows there is no justified terminal timestamp.
    # Restrict to available input span; don't invent a 90s observation endpoint.
    high = end or (inputs[-1]['start_ns']+inputs[-1]['sync_us']*1000 if inputs else None)
    for e in trace:
        a, b = e['start_ns'], e['start_ns']+e['duration_us']*1000
        if low is None or high is None or b < low or a > high:
            continue
        contained = [name for name, (l, h) in boundaries.items()
                     if l is not None and h is not None and l <= a < h and b <= h]
        overlapping = [name for name, (l, h) in boundaries.items()
                       if l is not None and h is not None and a < h and b > l]
        if contained:
            phase_events[contained[0]].append(e)
        elif len(overlapping) > 1 or overlapping and (a < low or b > high):
            if len(overlapping)==1:overlapping.append("outside_observation")
            cross.append(dict(id=e['id'], name=e['name'], phases=overlapping, duration_us=e['duration_us']))
        else:
            unassigned.append(e['id'])
    phases = {}
    for name, events in phase_events.items():
        l, h = boundaries[name]
        phases[name] = dict(start_ns=l, end_ns=h, elapsed_ms=(h-l)/1e6 if l is not None and h is not None else None,
                            spans={n: stats([e['duration_us'] for e in events if e['name'] == n])
                                   for n in sorted({e['name'] for e in events})}, event_ids=[e['id'] for e in events])
    # Prefer explicit request; historical resident rows can recover it from an
    # unambiguous source child of acceptPreparation. Source 0 is never an identity.
    request, source = row.get('prepare_request'), row.get('press_source')
    accepted = [e for e in trace if e['name'] == 'move.gui.acceptPreparation'
                and (request is not None and e['value'] == request or
                     request is None and source and points(e).get('move.gui.preparation.source') == source)]
    preparation = dict(status='unmeasured', reason='missing or ambiguous request/source', request=request, source=source)
    if len(accepted) == 1:
        accept = accepted[0]
        request = accept['value']
        requested = [e for e in trace if e['name'] == 'move.prepare.request' and e['value'] == request]
        if len(requested) == 1:
            preparation = dict(status='matched', request=request, source=points(accept).get('move.gui.preparation.source'),
                               request_to_gui_ms=(accept['start_ns']-requested[0]['start_ns'])/1e6)
    # Full identity is required for an exact final-frame join. In C1 v1 the
    # generation/frame fields were absent; a nearby revision is insufficient.
    target = row.get('final_frame_identity')
    frame = dict(status='unmeasured', reason='missing final frame identity')
    if target and all(k in target for k in ('view', 'generation', 'request', 'revision', 'frame')):
        matched = []
        for e in trace:
            if e['name'] != 'move.gui.acceptFrame':
                continue
            p = points(e)
            identity = {'view': p.get('move.frame.view'), 'generation': p.get('move.frame.generation'),
                        'request': p.get('move.frame.request'), 'frame': p.get('move.frame.id'), 'revision': e['value']}
            if identity == target:
                matched.append(e)
        frame = dict(status='matched' if len(matched) == 1 else 'unmeasured', identity=target,
                     event_id=matched[0]['id'] if len(matched) == 1 else None)
    origin = press.get('end_ns', start+press['sync_us']*1000) if press else None
    period = row.get('target_interval_ms', 8)*1e6
    lateness = [(m['start_ns']-(origin+(i+1)*period))/1e6 for i, m in enumerate(moves)] if origin else []
    gaps = [(b['start_ns']-a['start_ns'])/1e6 for a, b in zip(moves, moves[1:])]
    cadence = dict(kind='GUI-event-loop-driven trajectory; not external constant 125Hz', moves_sent=len(moves),
                   press_to_first_move_ms=(first-start)/1e6 if first is not None and start is not None else None,
                   sustained_move_total_ms=(moves[-1]['start_ns']-first)/1e6 if moves else None,
                   actual_gap_ms=stats(gaps), relative_schedule_lateness_ms=stats(lateness),
                   schedule_origin='Press dispatch return; historical return reconstructed from recorded sync_us',
                   final_lateness_ms=lateness[-1] if lateness else None)
    return dict(schema=2, terminal=row.get('terminal', 'committed' if row.get('completed') else 'historical_incomplete'),
                phases=phases, cross_boundary_spans=cross, unassigned_event_ids=unassigned,
                preparation=preparation, final_frame=frame, cadence=cadence,
                legacy_total_interval=dict(start_ns=start, end_ns=(row.get('release_ns', 0)+max(0, row.get('release_to_full_frame_ms', -1))*1e6),
                                           meaning='v1 Press through full-frame observation; incomplete v1 ends at Release'))


def reanalyze(roots, output):
    output.mkdir(parents=True, exist_ok=False)
    rows = []
    for root in roots:
        for path in sorted(root.rglob('raw.json')):
            data = json.loads(path.read_text(encoding='utf-8'))
            for row in data.get('rows', []):
                rows.append(dict(source=str(path.resolve()), source_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                                 scene=data.get('scenario'), phase=row['phase'], audit=audit_row(data, row)))
    (output/'audit.json').write_text(json.dumps(dict(schema=2, rows=rows), indent=2), encoding='utf-8')
    print(f'Offline audit: {len(rows)} rows, original files unchanged', flush=True)


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
                    release_us=next((i['sync_us'] for i in row['inputs'] if i['type']==3), None),
                    terminal=row.get('terminal', 'committed' if row['completed'] else 'historical_incomplete'),
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
        item['legacy_total_interval_meaning'] = 'Press through full-frame receipt; v1 fields unchanged'
        item['audit'] = audit_row(data, row)
        item['legacy_total_status'] = 'available' if hi >= lo else 'unmeasured_no_release'
        if hi < lo:
            # New early-abort rows have no Release; zero isn't an observed count.
            for key in list(item):
                if key.startswith('move.') or key in ('changed_deltas','created_source_host_bytes','copied_frame_bytes','submit_failures'):
                    item[key] = None

        result.append(item)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path, nargs='?')
    parser.add_argument('output', type=Path, nargs='?')
    parser.add_argument('--reanalyze', type=Path, nargs='+')
    parser.add_argument('--audit-output', type=Path)
    parser.add_argument('--scenes', type=int, choices=range(1, 5), nargs='+', default=[1, 2, 3, 4])
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument('--trace', action='store_true')
    parser.add_argument('--core-trace', action='store_true', help='bounded Rust task trace, exported at core shutdown')
    args = parser.parse_args()
    if args.reanalyze:
        if not args.audit_output:
            parser.error('--audit-output is required for offline reanalysis')
        reanalyze(args.reanalyze, args.audit_output)
        return
    if not args.executable or not args.output:
        parser.error('executable and output required')
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
    diff = subprocess.check_output(['git', 'diff', 'HEAD', '--', 'ui', 'core', 'tools/measure_layer_move.py'], cwd=root)
    metadata = dict(product_baseline='b948dccbc2fd767af3a999ee9a03e108455c44cb', tool_revision=revision,
                    tracked_diff_sha256=hashlib.sha256(diff).hexdigest(), executable_sha256=digest(executable),
                    tool_sha256=digest(Path(__file__)), repeats=args.repeats, scenes=args.scenes, trace=args.trace, core_trace=args.core_trace)
    source_files = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', 'core', 'ui', 'tools'], cwd=root, text=True).splitlines()
    metadata['sources_sha256'] = {p: digest(root/p) for p in source_files if (root/p).is_file()}
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
            env.pop('DRAWVERSE_MOVE_COMPLETION_TRACE', None)
            if args.core_trace:
                env['DRAWVERSE_MOVE_COMPLETION_TRACE'] = str(directory/'core.json')
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
            if args.core_trace and not code:
                core_file = directory/'core.json'
                if not core_file.is_file():
                    failures.append(dict(scene=scene, run=run, error='core trace missing'))
                else:
                    core = json.loads(core_file.read_text(encoding='utf-8'))
                    if core.get('dropped') or not core.get('closed'):
                        failures.append(dict(scene=scene, run=run, error='core trace incomplete', dropped=core.get('dropped')))
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
