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


def analyze(path):
    data = json.loads(path.read_text(encoding='utf-8'))
    invocation = json.loads((path.parent/'invocation.json').read_text(encoding='utf-8'))
    process = json.loads((path.parent/'process.json').read_text(encoding='utf-8'))
    result = dict(path=str(path), head=invocation['head'], executable_sha256=invocation['executable_sha256'],
                  passed=data.get('passed', False) and process.get('passed', False),
                  operation=data.get('operation'), size=data.get('size'),
                  fixture_version=data.get('fixture_version'),
                  trace=data.get('trace_enabled'), api=data.get('api'), dpr=data.get('dpr'), rows=[])
    if not result['passed'] or 'adoptions' not in data:
        return result
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
                                  preparation_ms=(row['ready_ns']-row['enter_ns'])/1e6,
                                  first_content_ms=(first['adopted_ns']-inputs[0]['t0_ns'])/1e6 if first else None,
                                  first_content_input=first['input'] if first else None,
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
