"""Offline C3 joins: exact commands/content identities, bounded cross-clock offset."""
import argparse
import json
from pathlib import Path
from measure_layer_move import stats


def unique(items):
    return items[0] if len(items) == 1 else None


def analyze_row(raw, core, row):
    if 'press_ns' not in row:
        return dict(phase=row.get('phase'), terminal=row.get('terminal'), timing_valid=False,
                    reason='no Press recorded; preparation/assertion failure retained', samples=0)
    qt, events = raw.get('trace', []), core.get('events', [])
    children, child_events = {}, {}
    for e in qt:
        children.setdefault(e.get('parent'), {})[e['name']] = e['value']
        child_events.setdefault(e.get('parent'), {})[e['name']] = e
    inputs = [dict(i) for i in row.get('inputs', [])]
    evidence = row.get('evidence', {})
    plans, ingress = evidence.get('planned_inputs', []), evidence.get('input_events', [])
    serial_protocol = len(plans) == len(inputs) and all(p['type'] == i['type'] for p, i in zip(plans, inputs))
    for index, inp in enumerate(inputs):
        # v2 called evidence.send exactly once per row input, in serial order;
        # later tools save that same concrete-event plan ID directly. Never join
        # arbitrary MouseMove events by time, source or device name.
        plan = inp.get('plan_id', plans[index]['id'] if serial_protocol else None)
        event = unique([e for e in ingress if e.get('plan_id') == plan and
                        e.get('provenance') == 'planned_exact_event' and e['type'] == inp['type']]) if plan is not None else None
        if event:
            inp['window_dispatch_ns'] = event['state']['ns']
    moves = [i for i in inputs if i['type'] == 5]
    press, end = row['press_ns'], row.get('hold_end_ns', 0)
    result = dict(phase=row['phase'], terminal=row['terminal'], tracking=raw['tracking'],
                  direct_w=raw['direct_w'], assets_ready=row['assets_ready'], uploads=row['uploads'],
                  samples=len(moves), held_pixels_and_association=row['held_pixels_and_association'],
                  timing_valid=row['terminal'] == 'committed',
                  qt_dropped=raw.get('trace_dropped'), core_dropped=core.get('dropped'),
                  adoptions_dropped=row.get('adoptions_dropped'))
    result['cadence_ms'] = dict(
        press_to_first_move=(moves[0]['dispatch_ns']-press)/1e6 if moves else None,
        dispatch_sync=stats([(i['end_ns']-i['dispatch_ns'])/1e6 for i in moves]),
        planned_to_dispatch=stats([(i['dispatch_ns']-i['planned_ns'])/1e6 for i in moves]),
        move_gaps=stats([(b['dispatch_ns']-a['dispatch_ns'])/1e6 for a, b in zip(moves, moves[1:])]),
        heartbeat=stats(row.get('heartbeat_ms', [])))
    result['cadence_ms']['planned_to_window_ingress'] = stats([
        (i['window_dispatch_ns']-i['planned_ns'])/1e6 for i in moves if 'window_dispatch_ns' in i])
    result['cadence_ms']['press_send_entry_to_first_window_move'] = (
        (moves[0]['window_dispatch_ns']-press)/1e6 if moves and 'window_dispatch_ns' in moves[0] else None)
    if not result['timing_valid'] or not end:
        result['reason'] = 'invalid/incomplete run retained; no latency success inference'
        return result
    generation = row['generation']
    scopes = [e for e in qt if e['name'] in ('move.begin', 'move.submit') and press <= e['start_ns'] < end]
    commands, bounds = [], []
    for scope in scopes:
        submit = unique([e for e in events if e['name'] == 'command.submit' and
                         e['generation'] == generation and e['sequence'] == scope['value']])
        if not submit:
            continue
        key = {k: submit[k] for k in ('session', 'generation', 'sequence')}
        command = {name: unique([e for e in events if e['name'] == name and all(e[k] == v for k, v in key.items())])
                   for name in ('command.submit', 'command.accept', 'command.applied')}
        if not all(command.values()):
            continue
        bounds.append((scope['start_ns']-submit['ns'], scope['start_ns']+scope['duration_us']*1000-submit['ns']))
        commands.append((scope, command, key))
    lo, hi = (max(b[0] for b in bounds), min(b[1] for b in bounds)) if bounds else (1, 0)
    result['clock'] = dict(status='bounded' if lo <= hi else 'unmeasured',
                          uncertainty_us=(hi-lo)/1000 if lo <= hi else None,
                          method='intersection of exact synchronous command.submit inside C++ FFI scopes; independent origins')
    timeline = []
    for scope, cmd, key in commands:
        seq, rev = key['sequence'], cmd['command.applied']['revision']
        worker = unique([e for e in qt if e['name'] == 'move.worker.metadata' and e['value'] == seq and
                         children.get(e['id'], {}).get('move.worker.metadata.generation') == generation and
                         children.get(e['id'], {}).get('move.worker.metadata.revision') == rev])
        gui = unique([e for e in qt if e['name'] == 'move.gui.metadataEntry' and e['value'] == seq])
        adopted, gui_content = [], []
        for a in row.get('adoptions', []):
            if not a['valid'] or a['generation'] != generation or not press <= a['ns'] <= end:
                continue
            if a['resident']:
                matches = [e for e in qt if e['name'] == 'move.gui.acceptResident' and e['value'] == rev and
                           children.get(e['id'], {}).get('move.gui.resident.generation') == generation and
                           children.get(e['id'], {}).get('move.gui.resident.source') == a['source'] and a['revision'] == rev]
            else:
                matches = [e for e in qt if e['name'] == 'move.gui.acceptFrame' and e['value'] == rev and
                           children.get(e['id'], {}).get('move.frame.generation') == generation and
                           children.get(e['id'], {}).get('move.frame.view') == 0 and
                           str(int(children.get(e['id'], {}).get('move.frame.imageKey', -1))) == a['image_key']]
            content = unique(matches)
            if content:
                adopted.append(a)
                gui_content.append(content)
        adoption, content = unique(adopted), unique(gui_content)
        emitted = child_events.get(worker['id'], {}).get('move.worker.metadata.emit') if worker else None
        item = dict(**key, kind=cmd['command.submit']['value'], revision=rev,
                    delta=([0, 0] if scope['name'] == 'move.begin' else
                           [children.get(scope['id'], {}).get('move.submit.dx'), children.get(scope['id'], {}).get('move.submit.dy')]),
                    qt_submit_after_press_ms=(scope['start_ns']-press)/1e6,
                    core_submit_to_accept_ms=(cmd['command.accept']['ns']-cmd['command.submit']['ns'])/1e6,
                    core_apply_ms=(cmd['command.applied']['ns']-cmd['command.accept']['ns'])/1e6,
                    worker_read_after_press_ms=(worker['start_ns']-press)/1e6 if worker else None,
                    worker_read_sync_ms=worker['duration_us']/1000 if worker else None,
                    gui_metadata_after_press_ms=(gui['start_ns']-press)/1e6 if gui else None,
                    worker_read_end_to_gui_ms=(gui['start_ns']-(worker['start_ns']+worker['duration_us']*1000))/1e6 if worker and gui else None,
                    worker_emit_to_gui_ms=(gui['start_ns']-emitted['start_ns'])/1e6 if emitted and gui else None,
                    gui_content_after_press_ms=(content['start_ns']-press)/1e6 if content else None,
                    node_adopt_after_press_ms=(adoption['ns']-press)/1e6 if adoption else None,
                    node_identity=adoption)
        if lo <= hi:
            item['core_after_press_ms'] = {n: [(e['ns']+lo-press)/1e6, (e['ns']+hi-press)/1e6] for n, e in cmd.items()}
        # Associate actual dispatched absolute values, not the nearest input in time.
        exact_inputs = [i for i in moves if i['delta'] == item['delta'] and i['dispatch_ns'] <= scope['start_ns']]
        inp = unique(exact_inputs)
        item['input_id'] = inp['id'] if inp else None
        ingress_ns = inp.get('window_dispatch_ns') if inp else None
        item['input_to_submit_ms'] = (scope['start_ns']-ingress_ns)/1e6 if ingress_ns is not None else None
        item['input_to_node_ms'] = (adoption['ns']-ingress_ns)/1e6 if ingress_ns is not None and adoption else None
        item['send_entry_to_node_ms'] = (adoption['ns']-inp['dispatch_ns'])/1e6 if inp and adoption else None
        timeline.append(item)
    result['commands'] = timeline
    # Durations within one render thread. An overlapping interval is evidence
    # of where a stall lies, not a proof that pipeline compilation caused all of it.
    render = sorted([e for e in qt if e['name'].startswith('c3.render.')], key=lambda e: e['start_ns'])
    spans = []
    for a, b in zip(render, render[1:]):
        if a['name'] == 'c3.render.beforeRender' and b['name'] == 'c3.render.afterRender' and a['thread'] == b['thread'] and a['start_ns'] < end and b['start_ns'] > press:
            spans.append(dict(start_after_press_ms=(a['start_ns']-press)/1e6,
                              end_after_press_ms=(b['start_ns']-press)/1e6,
                              duration_ms=(b['start_ns']-a['start_ns'])/1e6, thread=a['thread']))
    result['render_intervals_overlapping_hold'] = spans
    result['limits'] = 'node adoption is Qt sync, not GPU/physical presentation; core publication-ready time unmeasured; nested/overlapping intervals must not be added'
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('root', type=Path)
    p.add_argument('output', type=Path)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    rows = []
    for path in sorted(a.root.rglob('raw.json')):
        raw = json.loads(path.read_text(encoding='utf-8'))
        cp = path.with_name('core.json')
        core = json.loads(cp.read_text(encoding='utf-8')) if cp.exists() else {}
        for row in raw.get('rows', []):
            rows.append(dict(source=str(path.resolve()), **analyze_row(raw, core, row)))
    (a.output/'analysis.json').write_text(json.dumps(dict(rows=rows), indent=2), encoding='utf-8')
    print(f'{len(rows)} rows retained')


if __name__ == '__main__':
    main()
