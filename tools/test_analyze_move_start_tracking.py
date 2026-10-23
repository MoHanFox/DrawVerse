import copy
import unittest
from analyze_move_start_tracking import analyze_row


def fixture():
    # C++ origin is deliberately different from Rust. BEGIN metadata delivery
    # is slow although the core applied it quickly; UPDATE has a nonzero delta.
    offset = 1_000_000_000
    def q(i, name, ns, value, duration=0, parent=0):
        return dict(id=i, name=name, start_ns=offset+ns, value=value,
                    duration_us=duration, parent=parent, thread='render')
    qt = [q(1, 'move.begin', 1000, 8, 2), q(2, 'move.submit', 10_000_000, 9, 2),
          q(3, 'move.submit.dx', 10_000_100, 384, parent=2), q(4, 'move.submit.dy', 10_000_100, 0, parent=2),
          q(5, 'move.worker.metadata', 100_000, 8, 1),
          q(6, 'move.worker.metadata.generation', 100_100, 3, parent=5),
          q(7, 'move.worker.metadata.revision', 100_100, 3, parent=5),
          q(8, 'move.gui.metadataEntry', 9_000_000, 8),
          q(9, 'move.gui.acceptResident', 11_000_000, 4),
          q(10, 'move.gui.resident.source', 11_000_100, 7, parent=9),
          q(11, 'move.gui.resident.generation', 11_000_100, 3, parent=9)]
    core = dict(events=[])
    for seq, revision, submit, kind in [(8, 2, 1500, 1), (9, 3, 10_000_500, 4)]:
        for name, ns in [('command.submit', submit), ('command.accept', submit+100), ('command.applied', submit+200)]:
            core['events'].append(dict(name=name, ns=ns, session=1, sequence=seq,
                generation=3, revision=revision+(name=='command.applied'), value=kind))
    raw = dict(trace=qt, tracking=False, direct_w=True)
    row = dict(phase='W', terminal='committed', press_ns=offset, hold_end_ns=offset+20_000_000,
        generation=3, assets_ready=True, uploads=1, held_pixels_and_association=True,
        inputs=[dict(type=5, id=1, planned_ns=offset+2000, dispatch_ns=offset+3000,
                     end_ns=offset+3100, window_dispatch_ns=offset+3000, delta=[384, 0])],
        adoptions=[dict(ns=offset+12_000_000, valid=True, generation=3, source=7, revision=4,
                        resident=True, image_key='0')])
    return raw, core, row


class AttributionTests(unittest.TestCase):
    def test_exact_sequence_and_independent_clock(self):
        raw, core, row = fixture()
        result = analyze_row(raw, core, row)
        self.assertEqual(result['clock']['status'], 'bounded')
        self.assertAlmostEqual(result['commands'][0]['core_apply_ms'], .0001)
        update = result['commands'][1]
        self.assertEqual(update['input_id'], 1)
        self.assertAlmostEqual(update['input_to_node_ms'], 11.997)
        self.assertGreater(result['commands'][0]['worker_read_end_to_gui_ms'], 8)
        self.assertIsNone(result['commands'][0]['worker_emit_to_gui_ms'])

    def test_old_source_or_generation_cannot_prove_adoption(self):
        for field in ['source', 'generation', 'revision']:
            raw, core, row = fixture()
            row['adoptions'][0][field] = 99
            result = analyze_row(raw, core, row)
            self.assertIsNone(result['commands'][1]['node_adopt_after_press_ms'])

    def test_repeated_input_value_is_not_nearest_time_guess(self):
        raw, core, row = fixture()
        row['inputs'].append(copy.deepcopy(row['inputs'][0]))
        result = analyze_row(raw, core, row)
        self.assertIsNone(result['commands'][1]['input_id'])

    def test_invalid_and_incomplete_are_retained(self):
        for terminal in ['invalid_input_or_lifecycle', 'unexpected_cancel', 'timeout', 'product_error']:
            raw, core, row = fixture()
            row['terminal'] = terminal
            result = analyze_row(raw, core, row)
            self.assertFalse(result['timing_valid'])
            self.assertEqual(result['samples'], 1)
            self.assertNotIn('commands', result)

    def test_no_clock_bracket_no_absolute_core_timeline(self):
        raw, core, row = fixture()
        for e in core['events']:
            if e['sequence'] == 9:
                e['ns'] += 1_000_000
        result = analyze_row(raw, core, row)
        self.assertEqual(result['clock']['status'], 'unmeasured')
        self.assertNotIn('core_after_press_ms', result['commands'][0])

    def test_generator_delay_and_synchronous_cost_are_separate(self):
        raw, core, row = fixture()
        row['inputs'][0]['dispatch_ns'] += 5_000_000
        row['inputs'][0]['end_ns'] += 5_000_000
        c = analyze_row(raw, core, row)['cadence_ms']
        self.assertAlmostEqual(c['planned_to_dispatch']['p50'], 5.001)
        self.assertAlmostEqual(c['dispatch_sync']['p50'], .0001)

    def test_failure_before_press_is_not_a_zero_latency(self):
        result = analyze_row({}, {}, dict(phase='W', terminal='assertion_or_preparation_incomplete'))
        self.assertFalse(result['timing_valid'])
        self.assertEqual(result['samples'], 0)

    def test_exact_event_ingress_not_other_mouse_events_inside_send(self):
        raw, core, row = fixture()
        row['evidence'] = dict(planned_inputs=[dict(id=3, type=5)], input_events=[
            dict(plan_id=3, type=5, provenance='planned_exact_event', state=dict(ns=1_000_007_000)),
            dict(type=5, provenance='unplanned_window_ingress', state=dict(ns=1_000_008_000))])
        result = analyze_row(raw, core, row)
        self.assertAlmostEqual(result['commands'][1]['input_to_node_ms'], 11.993)
        self.assertAlmostEqual(result['cadence_ms']['planned_to_window_ingress']['p50'], .005)


if __name__ == '__main__':
    unittest.main()
