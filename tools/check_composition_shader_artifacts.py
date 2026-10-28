"""Check the actual composition QSB targets, double uniform layout and embedded resources."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from check_move_shader_artifacts import verify_dump


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('qsb', type=Path)
    p.add_argument('directory', type=Path)
    p.add_argument('--precompiled', action='store_true')
    p.add_argument('--executable', type=Path)
    a = p.parse_args()
    binary = a.executable.read_bytes() if a.executable else None
    for name in ['composition.vert', 'composition.frag', 'composition_transform.vert', 'composition_precise.frag', 'composition_warp.frag']:
        path = a.directory/(name+'.qsb')
        dump = subprocess.check_output([str(a.qsb), '-d', str(path)]).decode('utf-8', errors='replace')
        if name in ['composition_precise.frag', 'composition_warp.frag']:
            targets = set(re.findall(r'(SPIR-V|GLSL|HLSL|DXBC|MSL) (\d+(?: es)?) \[Standard\]', dump))
            expected = {('SPIR-V', '100'), ('DXBC' if a.precompiled else 'HLSL', '50')}
            if targets != expected:
                raise ValueError(f'{name}: {targets}, expected {expected}')
            reflection, _ = json.JSONDecoder().raw_decode(dump.split('Reflection info: ', 1)[1])
            block = reflection['uniformBlocks'][0]
            fields = {m['name']: (m['offset'], m['type']) for m in block['members']}
            if block['size'] != 224 or any(fields.get('inverse'+str(i)) != (128+i*32, 'dvec4') for i in range(3)):
                raise ValueError(f'{name}: unexpected double uniform layout {block}')
        else:
            targets = verify_dump(dump, a.precompiled)
        data = path.read_bytes()
        if binary is not None and binary.count(data) != 1:
            raise ValueError(f'{name}: expected one exact embedded resource in {a.executable}')
        print(name, hashlib.sha256(data).hexdigest(), sorted(targets), 'embedded' if binary else '')


if __name__ == '__main__':
    main()
