#!/usr/bin/env python3
"""Build the test driver against an existing foxdaw checkout without modifying it."""
import argparse, json, os, pathlib, shutil, subprocess
p = argparse.ArgumentParser()
p.add_argument('foxdaw', type=pathlib.Path)
a = p.parse_args()
repo = pathlib.Path(__file__).resolve().parents[2]
package = repo / 'target/editor-check-driver'
package.mkdir(parents=True, exist_ok=True)
host = a.foxdaw.resolve() / 'crates/foxdaw-vst3-host'
(package / 'Cargo.toml').write_text('''[package]
name = "foxplugs-editor-check"
version = "0.1.0"
edition = "2021"
[workspace]
[[bin]]
name = "foxplugs-editor-check"
path = "main.rs"
[dependencies]
eframe = { version = "0.33", default-features = false, features = ["default_fonts", "glow", "wayland", "x11"] }
libloading = "0.8"
libc = "0.2"
serde_json = "1"
foxdaw-vst3-host = { path = ''' + json.dumps(str(host)) + ''', features = ["native"] }
''')
shutil.copyfile(pathlib.Path(__file__).with_name('main.rs'), package / 'main.rs')
if not (package / 'Cargo.lock').exists():
    shutil.copyfile(a.foxdaw / 'Cargo.lock', package / 'Cargo.lock')
subprocess.run(['cargo', 'build', '--offline', '--manifest-path', str(package / 'Cargo.toml')],
               env={**os.environ, 'CARGO_TARGET_DIR': str(repo / 'target/gui-check')}, check=True)
