#!/usr/bin/env python3
"""Offline C2-A attribution; exact tickets, independent clocks and bounded calibration."""
import argparse
import json
from pathlib import Path

IDENTITY = ('view', 'generation', 'request', 'revision', 'frame')

def identity(event, target):
    return all(k in event and k in target and event[k] == target[k] for k in IDENTITY)

def scope_for(trace, name, prefix, target):
    for event in trace:
        if event['name'] != name or event['value'] != target['revision']:
            continue
        fields = {child['name'][len(prefix):]: child['value'] for child in trace
                  if child.get('parent') == event['id'] and child['name'].startswith(prefix)}
        fields['revision'] = event['value']
        if 'id' in fields: fields['frame'] = fields.pop('id')
        if identity(fields, target):
            return event
    return None

def analyze_row(raw, core, row):
    out = dict(phase=row['phase'], terminal=row.get('terminal'), attempts=row.get('attempts'),
               release_to_gui_ms=row.get('release_to_full_frame_ms'),
               core_dropped=core.get('dropped'), qt_dropped=raw.get('trace_dropped'), status='unmeasured')
    if row.get('terminal') != 'committed':
        out['reason'] = 'not a successful commit; inputs/terminal retained in raw'
        return out
    frame, receipt = row.get('final_frame_identity', {}), row['terminal_receipt']
    events, qt = core['events'], raw['trace']
    publications = [e for e in events if e['name'] == 'frame.published' and identity(e, frame)]
    if len(publications) != 1:
        out['reason'] = 'no unique final frame publication (restamp is not a render task)'
        return out
    pub = publications[0]
    task = [e for e in events if e['task'] == pub['task'] and e['session'] == pub['session']]
    by_name = {e['name']: e for e in task if e['name'] != 'task.concurrent'}
    required = ('task.snapshot_begin', 'task.snapshot_end', 'task.enqueue', 'task.start', 'task.end', 'task.result')
    if any(k not in by_name for k in required) or by_name['task.enqueue']['value'] != 0 or pub['sequence'] < receipt['sequence']:
        out['reason'] = 'missing stages or not a post-commit normal task'
        return out
    commands = {e['name']: e for e in events if e['name'].startswith('command.') and
                e['session'] == pub['session'] and e['generation'] == receipt['generation'] and e['sequence'] == receipt['sequence']}
    if any(k not in commands for k in ('command.submit','command.queued','command.accept','command.applied')):
        out['reason'] = 'commit command identity missing'
        return out
    if commands['command.submit']['value'] != 5 or 'command.error' in commands:
        out['reason'] = 'not a successful COMMIT'
        return out
    def delta(a, b): return (b['ns']-a['ns'])/1e6
    out.update(status='measured', frame=frame, task=pub['task'], sequence=receipt['sequence'],
        core_ms=dict(command_submit_to_accept=delta(commands['command.submit'],commands['command.accept']),
                     command_apply=delta(commands['command.accept'],commands['command.applied']),
                     apply_to_snapshot=delta(commands['command.applied'],by_name['task.snapshot_begin']),
                     snapshot=delta(by_name['task.snapshot_begin'],by_name['task.snapshot_end']),
                     queue=delta(by_name['task.enqueue'],by_name['task.start']),
                     execute=delta(by_name['task.start'],by_name['task.end']),
                     end_to_actor=delta(by_name['task.end'],by_name['task.result']),
                     actor_to_publish=delta(by_name['task.result'],pub)),
        renderer={k[len('renderer.'):]: e['value'] for k,e in by_name.items() if k.startswith('renderer.')},
        storage={k[len('storage.'):]: e['value'] for k,e in by_name.items() if k.startswith('storage.')},
        concurrent_task_ids=[e['value'] for e in task if e['name']=='task.concurrent'])
    out['concurrent_tasks'] = []
    for task_id in out['concurrent_task_ids']:
        other = [e for e in events if e['task']==task_id and e['session']==pub['session']]
        starts = [e for e in other if e['name']=='task.start']
        ends = [e for e in other if e['name']=='task.end']
        overlap = None
        if len(starts)==1 and len(ends)==1:
            overlap = max(0,min(ends[0]['ns'],by_name['task.end']['ns'])-max(starts[0]['ns'],by_name['task.start']['ns']))/1e6
        out['concurrent_tasks'].append(dict(task=task_id,view=other[0]['view'] if other else None,
            execution_overlap_ms=overlap,meaning='observed overlap; not isolated contention cost'))
    # Only count obsolete-view waits after this commit, in this session/view, for this exact revision.
    out['old_view_wait_observations'] = sum(e['name']=='task.wait_old_view' and e['session']==pub['session'] and
        e['view']==pub['view'] and e['value']==receipt['revision'] and e['ns']>=commands['command.accept']['ns'] for e in events)
    out['core_events_relative_submit_ms'] = {k: delta(commands['command.submit'],e) for k,e in commands.items()}
    out['core_events_relative_submit_ms'].update({k: delta(commands['command.submit'],e) for k,e in by_name.items() if not k.startswith(('renderer.','storage.'))})
    out['core_events_relative_submit_ms']['frame.published'] = delta(commands['command.submit'],pub)
    info = next((e for e in events if e['name']=='frame.info' and e['session']==pub['session'] and identity(e,frame) and e['ns']>=pub['ns']),None)
    copied = next((e for e in events if e['name']=='frame.copied' and e['session']==pub['session'] and identity(e,frame) and e['ns']>=pub['ns']),None)
    if info: out['core_ms']['publish_to_first_frame_info'] = delta(pub,info)
    if info and copied: out['core_ms']['first_info_to_copied'] = delta(info,copied)
    gui = scope_for(qt,'move.gui.acceptFrame','move.frame.',frame)
    copy = scope_for(qt,'move.worker.copyFrame','move.copyFrame.',frame)
    submit = next((e for e in qt if e['name']=='move.submit' and e['value']==receipt['sequence'] and
        any(c['parent']==e['id'] and c['name']=='move.submit.kind' and c['value']==5 for c in qt)),None)
    if not submit or not gui or not copy or not copied:
        out['cross_clock'] = dict(status='unmeasured',reason='exact synchronous anchors missing')
        return out
    # Rust command.submit is INSIDE the exact synchronous C ABI call, itself INSIDE this C++ scope.
    # Rust frame.copied is INSIDE read_frame, INSIDE the exact C++ copy scope. No equal origins assumed.
    bounds = [(submit['start_ns']-commands['command.submit']['ns'],
               submit['start_ns']+submit['duration_us']*1000-commands['command.submit']['ns']),
              (copy['start_ns']-copied['ns'],copy['start_ns']+copy['duration_us']*1000-copied['ns'])]
    lo, hi = max(b[0] for b in bounds), min(b[1] for b in bounds)
    if lo > hi:
        out['cross_clock'] = dict(status='inconsistent',reason='containment brackets do not overlap; no absolute timeline asserted',bounds=bounds)
        return out
    out['cross_clock'] = dict(status='bounded',method='intersection of exact COMMIT and exact read_frame synchronous containment brackets; stable clock rates over one completion',
        offset_ns=[lo,hi], uncertainty_us=(hi-lo)/1000,
        rust_events_after_release_ms={name:[(e['ns']+lo-row['release_ns'])/1e6,(e['ns']+hi-row['release_ns'])/1e6]
            for name,e in {**commands,**{k:e for k,e in by_name.items() if not k.startswith(('renderer.','storage.'))},'frame.published':pub,'frame.copied':copied}.items()},
        copied_to_gui_ms=[(gui['start_ns']-copied['ns']-hi)/1e6,(gui['start_ns']-copied['ns']-lo)/1e6])
    out['qt_ms'] = dict(release_to_commit_scope=(submit['start_ns']-row['release_ns'])/1e6,
                       copy_sync=copy['duration_us']/1000,
                       gui_entry_after_release=(gui['start_ns']-row['release_ns'])/1e6)
    return out

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=False)
    results=[]
    lifecycles=[]
    for path in sorted(args.root.rglob('raw.json')):
        core=path.with_name('core.json')
        raw=json.loads(path.read_text(encoding='utf-8'))
        if core.exists():
            core_data=json.loads(core.read_text(encoding='utf-8'))
            tasks={}
            for e in core_data['events']:
                if e['task']:
                    tasks.setdefault((e['session'],e['task']),[]).append(e)
            for (session,task),events in tasks.items():
                names={e['name'] for e in events}
                lifecycles.append(dict(source=str(path.resolve()),session=session,task=task,
                    view=events[0]['view'],sequence=events[0]['sequence'],
                    stages=sorted(names),unfinished='task.enqueue' in names and not names.intersection(
                        {'frame.published','task.cancelled','task.discarded','task.obsolete','task.enqueue_failed'})))
        for row in raw['rows']:
            result=analyze_row(raw,json.loads(core.read_text(encoding='utf-8')),row) if core.exists() else dict(phase=row['phase'],status='unmeasured',reason='diagnostics disabled or process failed')
            results.append(dict(source=str(path.resolve()),scene=raw['scenario'],**result))
    failures=[]
    for path in args.root.rglob('summary.json'):
        failures.extend(dict(source=str(path.resolve()),**f) for f in json.loads(path.read_text(encoding='utf-8')).get('failures',[]))
    (args.output/'attribution.json').write_text(json.dumps(dict(rows=results,failures=failures,task_lifecycles=lifecycles),indent=2),encoding='utf-8')
    print(f'{len(results)} rows; {len(failures)} process/scenario failures retained')
if __name__=='__main__': main()
