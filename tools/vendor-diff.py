#!/usr/bin/env python3
"""Compare vendored backports with exact original commits in a Cargo checkout cache."""
import argparse, difflib, pathlib, subprocess
p = argparse.ArgumentParser()
p.add_argument('--cache', type=pathlib.Path, default=pathlib.Path.home()/'.cargo/git/checkouts')
a = p.parse_args()
root = pathlib.Path(__file__).resolve().parents[1]
for vendor, checkout, rev in [
    ('vizia', 'vizia-41d4f3e0958d0ccf/e91b36f', 'e91b36f2ce213eb8eefdffbb6dbc462523f2c02e'),
    ('vizia-plug', 'vizia-plug-7db7431eee744c88/d106863', 'd106863cecb020b492acf00254fb5a61d4cda55d'),
    ('baseview', 'baseview-33f6ff6866b5c0bf/70e4e05', '70e4e05ba1de837970b5b5457a99893aae0b6cfa'),
]:
    source = a.cache / checkout
    tracked = subprocess.check_output(['git','-C',str(source),'ls-tree','-rz','--name-only',rev]).decode().split('\0')
    for name in filter(None, tracked):
        original = subprocess.check_output(['git','-C',str(source),'show',f'{rev}:{name}'])
        path = root/'vendor'/vendor/name
        current = path.read_bytes() if path.exists() else b''
        if original == current: continue
        try:
            print(''.join(difflib.unified_diff(original.decode().splitlines(True),current.decode().splitlines(True),
                fromfile=f'a/{vendor}/{name}',tofile=f'b/{vendor}/{name}')),end='')
        except UnicodeDecodeError:
            print(f'Binary difference: {vendor}/{name}')
    for path in sorted((root/'vendor'/vendor).rglob('*')):
        if path.is_file() and str(path.relative_to(root/'vendor'/vendor)) not in tracked:
            print(f'Added: {path.relative_to(root)}')
