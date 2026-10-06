#!/usr/bin/env python3
"""Prove the four domain security tests reject deliberately broken implementations.

Run from rust/. Restores each source in finally, even when a check fails. No
HTTP-parity claim: event controllers are a separate, still required slice.
"""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
domain = root / "crates/db/src/models/calendar_event.rs"
attendance = domain.parent / "calendar_event/attendance.rs"
recurrence = domain.parent / "calendar_event/recurrence.rs"
changes = domain.parent / "calendar_event/changes.rs"
mutations = [
    (domain, " AND m.user_id=? AND r.deleted_at IS NULL", " AND ? IS NOT NULL AND r.deleted_at IS NULL", "ws14e_security_nonmember"),
    (domain, "u.id == self.organizer_id || u.is_administrator()", "u.id != 0 || u.is_administrator()", "ws14e_security_only_organizer"),
    (attendance, "!member(tx.conn(), event.room_id, user_id)?", "false", "ws14e_security_rsvp"),
    (domain, "JOIN memberships m ON m.user_id=u.id AND m.room_id=? WHERE a.event_id=?", "JOIN memberships m ON m.room_id=? WHERE a.event_id=?", "ws14e_security_reminder"),
    (recurrence, "first.hour(),", "first.hour() + 1,", "pinned_rails_recurrence_vectors"),
    (changes, "UPDATE events SET series_id=NULL WHERE id=?", "UPDATE events SET series_id=series_id WHERE id=?", "event_scoped_operations_match_rails_vectors"),
]
for path, old, new, test in mutations:
    source = path.read_text()
    assert source.count(old) == 1, f"mutation target drifted: {test}"
    try:
        path.write_text(source.replace(old, new))
        result = subprocess.run(["cargo", "test", "--locked", "-j", "4", "-p", "campfire_db", test], cwd=root, capture_output=True, text=True)
        output = result.stdout + result.stderr
        lines = [line for line in output.splitlines() if line.startswith("test result:")]
        assert result.returncode != 0 and any("FAILED." in line for line in lines), output
        print(test + ": " + lines[-1], flush=True)
    finally:
        path.write_text(source)
