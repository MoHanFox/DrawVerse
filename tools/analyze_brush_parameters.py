"""Summarize only the opt-in parameter rows; preserve raw evidence and clock boundaries."""
import argparse
import json
from pathlib import Path
from measure_layer_move import stats


def analyze(data):
    events = data['trace']
    rows = []
    for row in data['rows']:
        inside = [e for e in events if row['start_ns'] <= e['start_ns'] and
                  e['start_ns']+e['duration_us']*1000 <= row['end_ns']]
        def named(name):
            return [e for e in inside if e['name'] == name]
        requests = named('catalog.request.preparation')
        completions = [e for e in events if e['name'] == 'catalog.complete.preparation']
        delays, unmatched = [], []
        for request in requests:
            matched = [e for e in completions if e['value'] == request['value']]
            if len(matched) == 1:
                delays.append((matched[0]['start_ns']-request['start_ns'])/1e6)
            else:
                unmatched.append(request['value'])
        rows.append(dict(scene=row['scene'], attempts=row['attempts'], changes=row['actual_changes'],
            completed=row['completed'], failures=row['failures'], timeouts=row['timeouts'], final_valid=row['final_valid'],
            first_sync_us=row['samples'][0]['sync_us'] if row['samples'] else None,
            warm_sync_us=row['warm_sync_us'], actual_gaps_ms=row['actual_input_gap_ms'], heartbeat_ms=row['heartbeat_ms'],
            release_sync_us=row['release_sync_us'], tail_observation_ms=row['post_input_quiescence_observation_ms'],
            notifications={k:row[k] for k in ('settingsChanged','brushChanged','persistRequested','activeColorsChanged')},
            expected_final=row['expected_final'], model_final=row['model_final'], saved_final=row['saved_final'],
            catalog_serialized_rows=sum(e['value'] for e in named('catalogSnapshot.serialized_rows')),
            catalog_reused_rows=sum(e['value'] for e in named('catalogSnapshot.preference_rows_reused')),
            spans_ms={name:stats([e['duration_us']/1000 for e in named(name)]) for name in
                      ('snapshotObject.rebuild','catalogSnapshot.rebuild','catalogSnapshot.presetRows',
                       'synchronizeBrushCatalog','refreshBrushCursor','worker.saveBrushPreferences')},
            preparation_requests=len(requests), preparation_observation_ms=stats(delays), unmatched_preparations=unmatched,
            cross_boundary_spans=sum(e['start_ns'] < row['end_ns'] and e['start_ns']+e['duration_us']*1000 > row['start_ns'] and e not in inside for e in events)))
    return dict(environment=data.get('environment'), passed=data['passed'],
        invalid_input_or_lifecycle=data['invalid_input_or_lifecycle'], trace_dropped=data['trace_dropped'],
        rows=rows, limits='GUI heartbeat and completion waits are proxies, not display latency; spans may nest/overlap and must not be added; async preparation joined by unique ticket in this single-client process')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('root', type=Path)
    p.add_argument('output', type=Path)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    runs = {str(path.relative_to(a.root)): analyze(json.loads(path.read_text(encoding='utf-8')))
            for path in a.root.rglob('raw.json')}
    (a.output/'analysis.json').write_text(json.dumps(runs, indent=2), encoding='utf-8')
    print(len(runs), 'processes retained')


if __name__ == '__main__':
    main()
