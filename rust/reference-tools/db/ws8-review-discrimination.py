#!/usr/bin/env python3
"""Astra regressions must fail real named tests; always restore the implementation.

Run alone from this worktree. Cargo uses four jobs and all temporary output stays
in .scratch. A build error does not count as a detected regression.
"""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / ".scratch" / "discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"))
MODELS = ROOT / "rust/crates/db/src/models"


def check(name, path, old, new, test, package="campfire_db"):
    original = path.read_text()
    assert original.count(old) == 1, f"{name}: ambiguous mutation"
    try:
        path.write_text(original.replace(old, new))
        run = subprocess.run(
            ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "-j", "4", "-p", package, test],
            cwd=ROOT / "rust", env=ENV, capture_output=True, text=True,
        )
        output = run.stdout + run.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        failed = re.findall(r"^test (\S+) \.\.\. FAILED$", output, re.M)
        summary = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert run.returncode and summary, f"{name}: no failing tests (see log)"
        assert any(t.endswith("::" + test) for t in failed), f"{name}: named regression did not fail"
        print(f"{name}: detected ({len(failed)} failing tests)", flush=True)
        print(summary[0], flush=True)
    finally:
        path.write_text(original)


check("review-audit-fold", MODELS / "audit_log.rs", "if secret_key(key)", "if SECRET.is_match(key)", "review_unicode_secret_redaction_matches_rails_on_record")
check("review-audit-boundaries", MODELS / "audit_log.rs", "if boundaries.binary_search(&matched.start()).is_ok()\n            && boundaries.binary_search(&matched.end()).is_ok()", "if true", "review_unicode_secret_redaction_matches_rails_on_record")
check("review-search-phrase", MODELS / "search_query.rs", 'Regex::new(r"\\w+")', 'Regex::new(r"[\\p{L}\\p{M}\\p{N}_]+")', "review_unicode_connector_phrases_match_rails_in_sqlite")
check("review-direct-order", MODELS / "direct_room.rs", "Ok(display_ordered_members(for_user, &list))", "{ let mut list = list; list.sort_by_key(|u| u.name.to_lowercase()); Ok(display_ordered_members(for_user, &list)) }", "review_direct_default_and_preloaded_order_match_rails")
check("review-create-canonical", MODELS / "message.rs", "Ok(Some(Self::prepare_body(tx, &rendered, true)?))", "Ok(Some(rendered))", "runtime_review_markdown_create_and_edit_store_canonical_rails_html", "campfire")
check("review-edit-canonical", MODELS / "message.rs", "let body = body.as_deref().map(|body| Self::prepare_body(tx, body, markdown_source.is_some())).transpose()?;", "let body = body;", "runtime_review_markdown_create_and_edit_store_canonical_rails_html", "campfire")
check("review-cable-room", MODELS / "channel_thread.rs", 'serde_json::json!({ "threadId": thread.id, "roomId": thread.room_id })', 'serde_json::json!({ "threadId": thread.id })', "ws8_thread_unread_broadcasts_match_real_rails_callbacks", "campfire")
print("WS8 review discrimination: 7 mutations detected; sources restored", flush=True)
