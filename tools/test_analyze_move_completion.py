import unittest
from analyze_move_completion import analyze_row


def fixture():
    frame=dict(view=0,generation=3,request=7,revision=20,frame=11)
    context=dict(session=1,task=2,sequence=9,**frame)
    def e(name,ns,value=0): return dict(context,name=name,ns=ns,value=value)
    events=[e(n,t,v) for n,t,v in [
        ('command.submit',1000,5),('command.queued',1100,5),('command.accept',2000,5),('command.applied',2500,5),
        ('task.snapshot_begin',2600,0),('task.snapshot_end',2700,0),('task.enqueue',2800,0),
        ('task.start',3000,0),('task.end',8000,0),('task.result',8100,0),('frame.published',8200,0),
        ('frame.info',8300,0),('frame.copied',8400,0)]]
    offset=1_000_000_000
    def q(i,n,t,d=0,v=0,parent=0): return dict(id=i,name=n,start_ns=offset+t,duration_us=d,value=v,parent=parent)
    qt=[q(1,'move.submit',900,.2,9),q(2,'move.submit.kind',901,v=5,parent=1),
        q(3,'move.worker.copyFrame',8300,.3,20),q(4,'move.gui.acceptFrame',8700,.1,20)]
    i=5
    for parent,prefix in [(3,'move.copyFrame.'),(4,'move.frame.')]:
        for k in ('view','generation','request','frame'):
            qt.append(q(i,prefix+('id' if k=='frame' else k),8500,v=frame[k],parent=parent));i+=1
    row=dict(phase='immediate',terminal='committed',attempts=120,release_ns=offset+500,
             release_to_full_frame_ms=.0083,final_frame_identity=frame,
             terminal_receipt=dict(sequence=9,generation=3,revision=20,kind=5))
    return dict(trace=qt,trace_dropped=0),dict(events=events,dropped=0),row


class AttributionTests(unittest.TestCase):
    def test_exact_task_and_synchronous_clock_brackets(self):
        raw,core,row=fixture();result=analyze_row(raw,core,row)
        self.assertEqual(result['status'],'measured')
        self.assertEqual(result['core_ms']['queue'],.0002)
        self.assertEqual(result['core_ms']['execute'],.005)
        self.assertEqual(result['cross_clock']['status'],'bounded')
        self.assertAlmostEqual(result['cross_clock']['uncertainty_us'],.2)
        self.assertEqual(result['cross_clock']['offset_ns'],[999999900,1000000100])

    def test_non_target_frame_never_supplies_completion(self):
        raw,core,row=fixture();core['events'].append(dict(core['events'][10],frame=12,ns=8199))
        self.assertEqual(analyze_row(raw,core,row)['frame']['frame'],11)
        row['final_frame_identity']['frame']=99
        self.assertEqual(analyze_row(raw,core,row)['status'],'unmeasured')

    def test_cancel_and_inconsistent_clock_are_not_successes(self):
        raw,core,row=fixture();row['terminal']='unexpected_cancel'
        self.assertEqual(analyze_row(raw,core,row)['status'],'unmeasured')
        row['terminal']='committed';core['events'][-1]['ns']+=1000
        self.assertEqual(analyze_row(raw,core,row)['cross_clock']['status'],'inconsistent')

if __name__=='__main__': unittest.main()
