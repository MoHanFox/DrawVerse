"""Summarize the opt-in sharedSettings probe without adding nested span times."""
import argparse
import json
from pathlib import Path


def summarize(report):
    names = ('snapshotObject.rebuild', 'snapshotObject.reuse', 'encodeSnapshotObject',
             'synchronizeBrushCatalog', 'preferences.broadcast', 'preferences.receiver.restore',
             'preferences.receiver.selectionAttempt', 'preferences.restore.full',
             'colorWheel.paint', 'colorWheel.rebuildTriangle', 'worker.saveBrushPreferences')
    result = []
    for row in report['rows']:
        start, end = row['start_ns'], row['end_ns']
        events = [e for e in report['trace'] if start <= e['start_ns'] < end]
        spans = {}
        for name in names:
            values = [e['duration_us'] / 1000 for e in events if e['name'] == name]
            spans[name] = {'n': len(values), 'max_ms': max(values, default=0)}
        active = [b['gap_ms'] for b in row.get('beats', []) if b['ns'] <= row['release_ns']]
        tail = [b['gap_ms'] for b in row.get('beats', []) if b['ns'] > row['release_ns']]
        result.append({k: row[k] for k in ('documents', 'field', 'interval_ms', 'attempts',
                                         'dispatch_us', 'persist_requests', 'receiver_rebuilds',
                                         'changed', 'saved', 'receivers_correct', 'input_invalid')} |
                      {'active_heartbeat_max_ms': max(active, default=0),
                       'tail_heartbeat_max_ms': max(tail, default=0), 'spans': spans})
    return {'passed': report['passed'], 'preset_count': report['preset_count'],
            'normal_startup': report.get('normal_startup', False),
            'isolation': report['isolation'], 'renderer': report['renderer'],
            'dpr': report['dpr'], 'trace_dropped': report['trace_dropped'], 'rows': result}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    data = summarize(json.loads(args.report.read_text(encoding='utf-8')))
    with args.output.open('x', encoding='utf-8') as file:
        json.dump(data, file, indent=2)
    print('docs field interval dispatch_max_ms active_gap_ms tail_gap_ms snapshot_max_ms broadcast_max_ms rebuilds')
    for row in data['rows']:
        print(row['documents'], row['field'], row['interval_ms'],
              *(round(v, 3) for v in (row['dispatch_us']['max'] / 1000,
                row['active_heartbeat_max_ms'], row['tail_heartbeat_max_ms'],
                row['spans']['snapshotObject.rebuild']['max_ms'],
                row['spans']['preferences.broadcast']['max_ms'])), row['receiver_rebuilds'])
    if not data['passed'] or data['trace_dropped']:
        raise SystemExit('Incomplete probe; raw evidence retained')


if __name__ == '__main__':
    main()
