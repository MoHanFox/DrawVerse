import unittest
from measure_layer_move import audit_row


def event(i, name, start, duration=0, value=0, parent=0):
    return dict(id=i, name=name, start_ns=start, duration_us=duration, value=value, parent=parent)


def fixture():
    return dict(phase='immediate', prepare_start_ns=0, release_ns=100000,
                release_to_full_frame_ms=.1, completed=1,
                inputs=[dict(type=2, start_ns=10000, sync_us=1),
                        dict(type=5, start_ns=20000, sync_us=1),
                        dict(type=5, start_ns=60000, sync_us=1),
                        dict(type=3, start_ns=100000, sync_us=1)])


class AuditTests(unittest.TestCase):
    def test_drag_and_completion_are_disjoint(self):
        r=audit_row(dict(trace=[event(1,'copy',30000,10),event(2,'copy',120000,10)]),fixture())
        self.assertEqual(r['phases']['sustained_drag']['event_ids'],[1])
        self.assertEqual(r['phases']['completion']['event_ids'],[2])
        self.assertEqual(r['cadence']['sustained_move_total_ms'],.04)

    def test_crossing_span_is_not_split_or_double_counted(self):
        r=audit_row(dict(trace=[event(1,'cross',90000,30)]),fixture())
        self.assertEqual(len(r['cross_boundary_spans']),1)
        self.assertFalse(any(p['event_ids'] for p in r['phases'].values()))

    def test_request_source_and_frame_identity(self):
        row=fixture();row.update(prepare_request=8,press_source=80,
                                final_frame_identity=dict(view=0,generation=3,request=9,revision=4,frame=5))
        t=[event(1,'move.prepare.request',1,value=7),event(2,'move.prepare.request',2,value=8),
           event(3,'move.gui.acceptPreparation',5,value=7),event(4,'move.gui.preparation.source',5,value=70,parent=3),
           event(5,'move.gui.acceptPreparation',6,value=8),event(6,'move.gui.preparation.source',6,value=80,parent=5)]
        for ident,view in [(10,1),(20,0)]:
            t.append(event(ident,'move.gui.acceptFrame',150000,value=4))
            for j,(name,value) in enumerate([('view',view),('generation',3),('request',9),('id',5)]):
                t.append(event(ident+j+1,'move.frame.'+name,150000,value=value,parent=ident))
        r=audit_row(dict(trace=t),row)
        self.assertEqual(r['preparation']['request'],8)
        self.assertEqual(r['final_frame']['event_id'],20)

    def test_incomplete_does_not_invent_success_or_end(self):
        row=fixture();row.update(completed=0,release_to_full_frame_ms=-1,terminal='unexpected_cancel')
        r=audit_row(dict(trace=[]),row)
        self.assertIsNone(r['phases']['completion']['end_ns'])
        self.assertEqual(r['terminal'],'unexpected_cancel')
        self.assertEqual(r['final_frame']['status'],'unmeasured')


if __name__=='__main__':unittest.main()
