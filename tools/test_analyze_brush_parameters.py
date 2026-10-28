import unittest
from analyze_brush_parameters import analyze


def fixture():
    row = dict(scene='radius-discrete', start_ns=0, end_ns=1_000_000_000, attempts=2,
               actual_changes=2, completed=2, failures=0, timeouts=0, final_valid=True,
               samples=[dict(sync_us=10)], warm_sync_us={}, actual_input_gap_ms={}, heartbeat_ms={},
               release_sync_us=0, post_input_quiescence_observation_ms=80, expected_final=12,
               model_final=12, saved_final=12, settingsChanged=2, brushChanged=2,
               persistRequested=2, activeColorsChanged=0)
    def event(name, thread, ns=1, duration=0, value=0):
        return dict(name=name, thread=thread, start_ns=ns, duration_us=duration, value=value)
    data = dict(trace=[event('input.dispatch','gui'),
        event('catalogSnapshot.rebuild','gui',duration=200),
        event('catalogSnapshot.rebuild','worker',duration=100_000),
        event('catalogSnapshot.serialized_rows','gui',value=256),
        event('catalogSnapshot.serialized_rows','worker',value=4),
        event('catalog.request.preparation','gui',ns=100,value=7),
        event('catalog.complete.preparation','worker',ns=10_000_100,value=7)],
        rows=[row], passed=True, invalid_input_or_lifecycle=False, trace_dropped=0)
    return data


class AnalysisTests(unittest.TestCase):
    def test_gui_and_overlapping_background_work_are_separate(self):
        result=analyze(fixture());row=result['rows'][0]
        self.assertEqual(result['gui_thread'],'gui')
        self.assertEqual(row['gui_spans_ms']['catalogSnapshot.rebuild']['p50'],.2)
        self.assertEqual(row['gui_catalog_serialized_rows'],256)
        self.assertEqual(row['non_gui_catalog_serialized_rows'],4)
        self.assertEqual(row['preparation_observation_ms']['p50'],10)

    def test_duplicate_receipt_is_unmeasured_not_nearest(self):
        data=fixture();data['trace'].append(data['trace'][-1].copy())
        row=analyze(data)['rows'][0]
        self.assertEqual(row['unmatched_preparations'],[7])
        self.assertEqual(row['preparation_observation_ms']['n'],0)


if __name__=='__main__':
    unittest.main()
