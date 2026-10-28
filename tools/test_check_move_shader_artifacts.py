import unittest
from check_move_shader_artifacts import verify_dump


class ArtifactTests(unittest.TestCase):
    def test_only_expected_d3d_target_changes(self):
        common = 'SPIR-V 100 [Standard]\nGLSL 300 es [Standard]\nGLSL 330 [Standard]\nMSL 12 [Standard]\n'
        verify_dump(common+'HLSL 50 [Standard]', False)
        verify_dump(common+'DXBC 50 [Standard]', True)
        for invalid in (common+'HLSL 50 [Standard]', common+'DXBC 50 [Batchable]', 'DXBC 50 [Standard]'):
            with self.assertRaises(ValueError):
                verify_dump(invalid, True)


if __name__ == '__main__':
    unittest.main()
