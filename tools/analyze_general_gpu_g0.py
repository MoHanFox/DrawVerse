"""Correlate G0 content adoptions with bounded input records, after the run."""
import argparse
import json
import math
from pathlib import Path


def stats(values):
    values = sorted(values)
    if not values:
        return dict(n=0)
    return dict(n=len(values), p50=values[math.ceil(len(values)*.5)-1],
                p95=values[math.ceil(len(values)*.95)-1], max=values[-1])


def move_begin_diagnostics(data, row, core):
    """Correlate by sequence, keeping Rust/Qt clock differences local."""
    begins = [e for e in data.get('trace', []) if e['name'] == 'move.begin' and
              row['enter_ns'] <= e['start_ns'] <= row['hold_ns']]
    if len(begins) != 1:
        return None
    begin = begins[0]
    sequence = int(begin['value'])
    emitted = next((e for e in data['trace'] if e['name'] == 'move.worker.metadata.emit' and
                    e['value'] >= sequence and begin['start_ns'] <= e['start_ns'] <= row['hold_ns']), None)
    result = dict(sequence=sequence, qt_begin_us=begin['duration_us'],
                  qt_begin_to_metadata_emit_ms=(emitted['start_ns']-begin['start_ns'])/1e6 if emitted else None,
                  core_trace_dropped=core.get('dropped'))
    submitted = [e for e in core.get('events', []) if e['name'] == 'command.submit' and e['sequence'] == sequence]
    if len(submitted) == 1 and core.get('dropped') == 0:
        submit = submitted[0]
        events = {e['name']: e for e in core['events'] if
                  e['sequence'] == sequence and e['session'] == submit['session']}
        for field, start, end in [
            ('core_queue_us', 'command.submit', 'command.accept'),
            ('core_apply_us', 'command.accept', 'command.applied'),
            ('core_apply_to_publication_us', 'command.applied', 'metadata.published'),
            ('core_submit_to_publication_us', 'command.submit', 'metadata.published'),
        ]:
            result[field] = (events[end]['ns']-events[start]['ns'])/1000 if start in events and end in events else None
    return result


def analyze(path):
    data = json.loads(path.read_text(encoding='utf-8'))
    invocation = json.loads((path.parent/'invocation.json').read_text(encoding='utf-8'))
    process = json.loads((path.parent/'process.json').read_text(encoding='utf-8'))
    result = dict(path=str(path), head=invocation['head'], executable_sha256=invocation['executable_sha256'],
                  passed=data.get('passed', False) and process.get('passed', False),
                  operation=data.get('operation'), size=data.get('size'),
                  fixture_version=data.get('fixture_version'), warp=data.get('warp', False),
                  captures=data.get('captures', True),
                  pixel_correctness_checked=data.get('pixel_correctness_checked', True),
                  trace=data.get('trace_enabled'), api=data.get('api'), dpr=data.get('dpr'), rows=[])
    if not result['passed'] or 'adoptions' not in data:
        return result
    core_path = path.parent/'core-trace.json'
    core = json.loads(core_path.read_text(encoding='utf-8')) if core_path.exists() else {}
    for row in data['rows']:
        inputs = [i for i in row['inputs'] if i['event_type'] == 5]
        candidates = [a for a in data['adoptions'] if a['valid'] and
                      inputs[0]['t0_ns'] <= a['ns'] <= row['hold_ns'] and
                      a['generation'] == row['held_identity']['generation']]
        latencies, matches = [], []
        for event in inputs:
            def matches_input(a):
                if a['ns'] < event['t0_ns']:
                    return False
                if row.get('composition_expected'):
                    return (a.get('composition') and a['source'] == row['held_identity']['source'] and
                            a['device_epoch'] == row['held_identity']['device_epoch'] and
                            ((abs(a['x0']-event['dx']) < .01 and abs(a['y0']-event['dy']) < .01) if data['operation']=='transform' else
                             (a['dx'] == event['dx'] and a['dy'] == event['dy'])))
                if row['resident_expected']:
                    if not a['resident'] or a['source'] != row['held_identity']['source']:
                        return False
                    if data['operation'] == 'move':
                        return a['dx'] == event['dx'] and a['dy'] == event['dy']
                    return abs(a['x0']-event['dx']) < .01 and abs(a['y0']-event['dy']) < .01
                # The unprepared immediate move is a CPU path. Correlate its
                # authoritative offset and completed UPDATE with the adopted
                # exact frame revision, never merely with the current getter.
                if len(inputs) == 1:
                    # The held identity was frozen after completedSequence and
                    # current-frame checks, then validated by the stable grab.
                    # Match the exact adopted image, not any revision >= floor.
                    return not a['resident'] and a['image_key'] == row['held_identity']['image_key']
                revisions = [s['revision'] for s in row['states'] if
                             s['move']['deltaX'] == event['dx'] and s['move']['deltaY'] == event['dy'] and
                             s['move']['completedSequence'] >= s.get('sequence', float('inf')) and
                             s['move']['submittedKind'] == 4 and not s['move']['dirty']]
                return bool(revisions) and not a['resident'] and a['revision'] >= min(revisions)
            adopted = next((a for a in candidates if matches_input(a)), None)
            if adopted:
                latency = (adopted['ns']-event['t0_ns'])/1e6
                latencies.append(latency)
                frames = [f for f in data['submitted_frames'] if f['epoch'] == adopted['epoch'] and
                          f['adoption_ns'] == adopted['ns'] and f['ns'] >= adopted['ns']]
                matches.append(dict(input=event['id'], input_ns=event['t0_ns'], adopted_ns=adopted['ns'],
                                    geometry=adopted['geometry'], source=adopted['source'],
                                    revision=adopted['revision'], latency_ms=latency,
                                    frame_ns=frames[0]['ns'] if frames else None))
        first = min(matches, key=lambda m: m['adopted_ns']) if matches else None
        result['rows'].append(dict(phase=row['phase'], resident=row['resident_expected'],
                                  move_begin_diagnostics=move_begin_diagnostics(data, row, core),
                                  composition=row.get('composition_expected', False),
                                  preparation_ms=(row['ready_ns']-row['enter_ns'])/1e6,
                                  first_content_ms=(first['adopted_ns']-inputs[0]['t0_ns'])/1e6 if first else None,
                                  first_content_input=first['input'] if first else None,
                                  first_frame_submit_ms=(first['frame_ns']-inputs[0]['t0_ns'])/1e6 if first and first['frame_ns'] else None,
                                  matched_frame_submit=stats([(m['frame_ns']-m['input_ns'])/1e6 for m in matches if m['frame_ns']]),
                                  target_confirmation_ms=(row['target_gui_confirmed_ns']-inputs[0]['t0_ns'])/1e6 if row.get('target_gui_confirmed_ns') else None,
                                  enter_to_content_ms=(first['adopted_ns']-row['enter_ns'])/1e6 if first else None,
                                  matched=stats(latencies), input_count=len(inputs), matches=matches,
                                  actual_gap_ms=stats([(b['t0_ns']-a['t0_ns'])/1e6 for a, b in zip(inputs, inputs[1:])]),
                                  dispatch_lateness_ms=stats([(i['t0_ns']-i['planned_ns'])/1e6 for i in inputs]),
                                  release_to_final_node_ms=(row['final_adopted_ns']-row['release_ns'])/1e6))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    result = [analyze(p) for p in sorted(args.root.rglob('raw.json'))]
    args.output.write_text(json.dumps(result, indent=2), encoding='utf-8')
    for run in result:
        print(Path(run['path']).parent.name, 'pass' if run['passed'] else 'FAIL',
              [(r['phase'], r['first_content_ms'], r['matched']) for r in run['rows']])


if __name__ == '__main__':
    main()
