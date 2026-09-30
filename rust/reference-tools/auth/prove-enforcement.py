#!/usr/bin/env python3
"""Deliberate runtime defects against real seeded HTTP and DB/cookie cable gates."""
from pathlib import Path
import os, re, subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT.parent / '.scratch/enforcement-mutations'
OUT.mkdir(parents=True, exist_ok=True)
cases = [
    ('global-gate-omitted', 'crates/campfire/src/concerns.rs',
     'pub async fn enforce_two_factor_for_restored_session(c: &mut Ctx) -> Result<()> {',
     'pub async fn enforce_two_factor_for_restored_session(c: &mut Ctx) -> Result<()> { return Ok(());',
     'app::enforcement_tests::unenrolled_password'),
    ('late-restore-gate-omitted', 'crates/campfire/src/concerns.rs',
     'resume_session(c, session, user).await?;\n    enforce_two_factor_for_restored_session(c).await?;',
     'resume_session(c, session, user).await?;',
     'app::enforcement_tests::late_restores'),
    ('stale-session-survives', 'crates/campfire/src/concerns.rs',
     'if enabled {\n        terminate_current_session(c).await?;',
     'if enabled {', 'app::enforcement_tests::stale_enrolled'),
    ('storage-2fa-bypassed', 'crates/campfire/src/active_storage.rs',
     'if session.two_factor_verified()', 'if true',
     'app::enforcement_tests::storage_metadata_rejects'),
    ('cable-2fa-bypassed', 'crates/campfire/src/channels/connection.rs',
     'if user.requires_two_factor() && !session.two_factor_verified()', 'if false',
     'channels::connection::tests::human_verification'),
    ('idle-boundary-inclusive', 'crates/campfire/src/concerns.rs',
     'session.last_active_at < now.ago(idle_timeout)',
     'session.last_active_at <= now.ago(idle_timeout)',
     'app::enforcement_tests::admin_idle'),
]
for name, path, needle, replacement, test in cases:
    source = ROOT / path
    original = source.read_text()
    pattern = r'\s*'.join(re.escape(p) for p in needle.split())
    matched = re.search(pattern, original)
    if matched is None:
        raise RuntimeError(f'{name}: mutation anchor disappeared')
    try:
        source.write_text(original[:matched.start()] + replacement + original[matched.end():])
        result = subprocess.run(
            ['cargo', 'test', '--locked', '-j', '4', '-p', 'campfire', test], cwd=ROOT,
            env=dict(os.environ, CI='1', TMPDIR=str(ROOT.parent / '.scratch/tmp')),
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
        )
        (OUT / (name + '.log')).write_text(result.stdout)
        summaries = [s for s in result.stdout.splitlines() if s.startswith('test result:')]
        if result.returncode == 0 or not any('FAILED' in s for s in summaries):
            raise RuntimeError(f'{name}: no real failing test; see {OUT}')
        print(name + ': ' + summaries[-1], flush=True)
    finally:
        source.write_text(original)
print(f'WS9 enforcement security gates: {len(cases)} deliberate defects rejected', flush=True)
