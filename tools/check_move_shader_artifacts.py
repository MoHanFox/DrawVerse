"""Verify actual qsb targets, optionally also the bytes linked into an executable.

qsb may leave HLSL in place after fxc fails. A successful qsb exit is insufficient.
This small build check uses the SDK tool's public dump interface, not QSB internals.
"""
import argparse
import hashlib
from pathlib import Path
import re
import subprocess


def verify_dump(dump, precompiled):
    targets = set(re.findall(r'(SPIR-V|GLSL|HLSL|DXBC|MSL) (\d+(?: es)?) \[Standard\]', dump))
    required = {('SPIR-V', '100'), ('GLSL', '300 es'), ('GLSL', '330'), ('MSL', '12'),
                ('DXBC' if precompiled else 'HLSL', '50')}
    if not required <= targets or (precompiled and ('HLSL', '50') in targets):
        raise ValueError(f'Unexpected shader targets: {targets}; required {required}')
    return targets


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('qsb', type=Path)
    p.add_argument('directory', type=Path)
    p.add_argument('--precompiled', action='store_true')
    p.add_argument('--executable', type=Path)
    a = p.parse_args()
    binary = a.executable.read_bytes() if a.executable else None
    for name in ('liquify.vert.qsb', 'liquify.frag.qsb'):
        path = a.directory/name
        result = subprocess.run([str(a.qsb), '-d', str(path)], capture_output=True, check=True)
        targets = verify_dump(result.stdout.decode('utf-8', errors='replace'), a.precompiled)
        data = path.read_bytes()
        if binary is not None and binary.count(data) != 1:
            raise ValueError(f'{name}: expected one exact embedded resource in {a.executable}')
        print(name, hashlib.sha256(data).hexdigest(), sorted(targets), 'embedded' if binary else '')


if __name__ == '__main__':
    main()
