#!/usr/bin/env python3
"""Run byte-exact media tests with the Rails image's libvips/ffmpeg builds.

All other tests run natively. The application logo, video corpora and storage
vectors keep every byte assertion; only their media runtime matches the oracle.
"""

from pin_identity import PIN, PIN_FULL, PIN_IMAGE
from pathlib import Path
import os
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
binary = Path(sys.argv[1]).resolve()
arguments = sys.argv[2:]
pinned_application_tests = (
    'controllers::messages::attachment_processing_tests::attachment_processing_rows_html_and_broadcast_bytes_match_fresh_rails',
    'controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers',
    'controllers::agent_review_r3_tests::pr192_r3_fresh_video_retains_preview_and_variant_files',
    'controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect',
    'controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy',
    'controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect',
    'controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy',
)


def pinned_args(test_arguments):
    scratch = Path(os.environ.get('PINNED_MEDIA_SCRATCH', root / '.scratch'))
    temporary = scratch / 'pinned-media'
    temporary.mkdir(parents=True, exist_ok=True)
    image = os.environ.get('PARITY_IMAGE', PIN_IMAGE)
    owner = os.environ.get('PARITY_OWNER', 'ws11ui')
    print(f'WS11 media runner: byte-exact media tests execute in {image}', flush=True, file=sys.stderr)
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
    media = [test for test in pinned_application_tests
             if test + ': test' in selected.splitlines()]
    if media:
        native_arguments = [str(binary), *arguments]
        for test in media:
            native_arguments.extend(['--skip', test])
        native = subprocess.run(native_arguments)
        # Sequential: never add another worker to the native run.
        pinned = subprocess.run(pinned_args([*media, '--exact',
                                            '--test-threads=' + os.environ.get('RUST_TEST_THREADS', '4')]))
        sys.exit(native.returncode or pinned.returncode)
os.execv(str(binary), [str(binary), *arguments])
