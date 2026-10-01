#!/usr/bin/env python3
"""Compile each omitted provider seam; the full HTTP byte assertion must reject it."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
source = root / 'rust/crates/campfire/src/controllers/presenters.rs'
original = source.read_bytes()
scratch = root / '.scratch'
scratch.mkdir(exist_ok=True)
env = dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_BUILD_JOBS='2',
           CARGO_PROFILE_TEST_DEBUG='0', CARGO_PROFILE_DEV_DEBUG='0',
           CABLE_TEST_PORT_RANGE='52100-52149', MAIL_TEST_PORT_RANGE='52100-52149')
needle = 'let mut components = link_embeds::components(self, message)?;'
assert original.decode().count(needle) == 1
try:
    for field in ['fizzy_cards', 'link_embed_cards', 'linkedin_cards']:
        source.write_text(original.decode().replace(needle, f'{needle}\n                components.{field}.clear();'))
        run = subprocess.run([
            'mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j2',
            '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire', '--bin', 'campfire',
            'native_room_page_provider_cards_match_rails_bytes', '--', '--test-threads=4', '--nocapture',
        ], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f'native-provider-{field}-discrimination.log').write_text(run.stdout)
        summaries = [line for line in run.stdout.splitlines() if line.startswith('test result:')]
        assert run.returncode == 101 and len(summaries) == 1 and '0 passed; 1 failed;' in summaries[0], run.stdout[-6000:]
        print(f'{field}: {summaries[0]}', flush=True)
        source.write_bytes(original)
finally:
    source.write_bytes(original)
print('Native provider discrimination: three independently omitted owner card seams rejected; source restored')
