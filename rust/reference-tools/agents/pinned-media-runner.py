#!/usr/bin/env python3
"""Run byte-exact media tests with the Rails image's libvips/ffmpeg builds.

All other tests run natively. The application logo case and storage vectors keep
all their byte assertions; only their media runtime matches CI and the oracle.
"""
from pathlib import Path
import os
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
binary = Path(sys.argv[1]).resolve()
arguments = sys.argv[2:]
logo = 'controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers'


def pinned_args(test_arguments):
    scratch = root / '.scratch'
    temporary = scratch / 'pinned-media'
    temporary.mkdir(parents=True, exist_ok=True)
    image = os.environ.get('PARITY_IMAGE', 'ws11ui-reference:d7c7de92')
    owner = os.environ.get('PARITY_OWNER', 'ws11ui')
    print(f'WS11 media runner: byte-exact media tests execute in {image}', flush=True)
    args = ['docker', 'run', '--rm', '--network', 'none', '--name',
            owner + '-fresh-pinned-media-' + str(os.getpid()), '--cpus', '2',
            '--user', f'{os.getuid()}:{os.getgid()}', '--entrypoint', str(binary),
            '-e', 'CI=1', '-e', 'TMPDIR=' + str(temporary),
            '-v', str(root) + ':' + str(root) + ':ro',
            '-v', str(scratch) + ':' + str(scratch) + ':rw',
            '--workdir', str(Path.cwd())]
    for name in ['CABLE_TEST_PORT_RANGE', 'INTEGRATION_TEST_PORT_RANGE',
                 'MAIL_TEST_PORT_RANGE', 'GITHUB_TEST_PORT_RANGE']:
        if name in os.environ:
            args.extend(['-e', name])
    if not binary.is_relative_to(root):
        # A fresh source clone may use an external shared compiler cache.
        args.extend(['-v', str(binary.parent) + ':' + str(binary.parent) + ':ro'])
    return args + [image, *test_arguments]


if Path.cwd() == root / 'rust/crates/storage' and binary.name.startswith('vectors-'):
    os.execvp('docker', pinned_args(arguments))
if (Path.cwd() == root / 'rust/crates/campfire'
        and binary.name.startswith('campfire-') and '--list' not in arguments):
    selected = subprocess.run([str(binary), *arguments, '--list'],
                              check=True, capture_output=True, text=True).stdout
    if logo + ': test' in selected.splitlines():
        native = subprocess.run([str(binary), *arguments, '--skip', logo])
        # Sequential: never add another worker to the native eight-thread run.
        media = subprocess.run(pinned_args([logo, '--exact', '--test-threads=8']))
        sys.exit(native.returncode or media.returncode)
os.execv(str(binary), [str(binary), *arguments])
