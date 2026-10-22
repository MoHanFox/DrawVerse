"""Summarize paired A0/A1/A2 raw records without adding nested span durations."""
import argparse
import csv
import json
import math
from pathlib import Path
import re


def stats(values):
    values = sorted(values)
    return {'n': len(values), **({key: values[math.ceil(len(values)*fraction)-1]
            for key, fraction in [('p50', .5), ('p95', .95), ('max', 1)]} if values else {})}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    rows, evidence, tool_hashes = [], [], None
    names = ('snapshotObject', 'snapshotObject.rebuild', 'snapshotObject.reuse',
             'catalogSnapshot.rebuild', 'catalogSnapshot.object_reuse', 'catalogSnapshot.bytes_reuse',
             'synchronizeBrushCatalog', 'catalog.request.preparation', 'encodeSnapshotObject',
             'worker.saveBrushPreferences', 'preferences.queue_us')
    for invocation in sorted(args.directory.glob('*/invocation.json')):
        metadata = json.loads(invocation.read_text(encoding='utf-8'))
        if tool_hashes is None:
            tool_hashes = metadata['tool_files_sha256']
        if metadata['tool_files_sha256'] != tool_hashes:
            raise SystemExit(f'Measurement sources differ: {invocation}')
        for raw in sorted(invocation.parent.glob('run-*/raw.json')):
            data = json.loads(raw.read_text(encoding='utf-8'))
            log = (raw.parent/'test.txt').read_text(encoding='utf-8')
            adapter = re.search(r"Adapter \d+: '([^']+)' \(vendor (\S+) device (\S+) flags (\S+)\).*?using this adapter", log, re.S)
            evidence.append({'run': invocation.parent.name, 'invocation': metadata,
                             'adapter': adapter.groups() if adapter else None,
                             'renderer': data['renderer'], 'dpr': data['dpr'],
                             'logical_dpi': data['logical_dpi'], 'colors_persisted': data['colors_persisted'],
                             'expected_final_colors': data['expected_final_colors'], 'saved_final_colors': data['saved_final_colors']})
            if not data['colors_persisted'] or data['error'] or data['trace_dropped']:
                raise SystemExit(f'Invalid run retained: {raw}')
            for index, row in enumerate(data['rows'], 1):
                events = [event for event in data['trace'] if index*1000 <= event['operation'] < (index+1)*1000]
                spans = {name: stats([event['duration_us'] for event in events if event['name'] == name]) for name in names}
                retained = [event['value'] for event in events if event['name'] == 'colors.preference_cache_retained']
                item = {'run': invocation.parent.name, 'presets': data['preset_count'], 'trace': data['trace_enabled'],
                        'scene': row['scene'], 'attempts': row['attempts'], 'completed': row['completed'],
                        'actual_changes': row['actual_changes'], 'failures': row['failures'], 'timeouts': row['timeouts'],
                        'first_sync_us': row['samples'][0]['sync_us'], 'sync_us': row['warm_sync_us'],
                        'heartbeat_ms': row['heartbeat_ms'], 'input_gap_ms': row['actual_input_gap_ms'],
                        'tail_observation_ms': row['post_input_quiescence_observation_ms'],
                        'retained_color_cache': sum(retained), 'color_cache_observations': len(retained), 'spans_us': spans}
                rows.append(item)
    if not rows:
        raise SystemExit('No raw results found')
    result = {'rows': rows, 'evidence': evidence, 'notes': 'Spans are nested/parallel: do not sum. Heartbeat and tail observation are not display latency.'}
    (args.directory/'analysis.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    flat = []
    for row in rows:
        flat.append({key: json.dumps(value, separators=(',', ':')) if isinstance(value, dict) else value for key, value in row.items()})
    with (args.directory/'analysis.csv').open('w', newline='', encoding='utf-8') as file:
        writer = csv.DictWriter(file, fieldnames=list(flat[0])); writer.writeheader(); writer.writerows(flat)


if __name__ == '__main__':
    main()
