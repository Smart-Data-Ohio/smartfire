#!/usr/bin/env python3
"""Prove selected WS8 regressions fail real tests; restore sources after every mutation.

Run from the WS8 worktree with no other build/edit process active. Logs and temp databases use
the worktree's .scratch. A compilation failure never counts as detecting a regression.
"""
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / ".scratch" / "discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"))
MODELS = ROOT / "rust/crates/db/src/models"


def replace_body(source, marker, body):
    start = source.index("{", source.index(marker))
    depth = 1
    end = start + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[:start] + "{\n" + body + "\n}" + source[end:]


def check(name, changes, test_filter, must_fail):
    original = {path: path.read_text() for path in changes}
    try:
        for path, mutate in changes.items():
            broken = mutate(original[path])
            assert broken != original[path], f"mutation did not change {path}"
            path.write_text(broken)
        run = subprocess.run(
            ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "-j", "4", "-p", "campfire_db", test_filter],
            cwd=ROOT / "rust", env=ENV, capture_output=True, text=True,
        )
        output = run.stdout + run.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        failed = re.findall(r"^test (\S+) \.\.\. FAILED$", output, re.M)
        summary = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert run.returncode != 0 and summary, f"{name}: did not produce failing tests (see log)"
        for suffix in must_fail:
            assert any(test.endswith("::" + suffix) for test in failed), f"{name}: {suffix} did not fail"
        print(f"{name}: detected ({len(failed)} failing tests)", flush=True)
        print(summary[0], flush=True)
    finally:
        for path, source in original.items():
            path.write_text(source)


def broken_keyword(source):
    source = replace_body(source, "pub fn normalize(", "phrase.to_string()")
    source = replace_body(source, "fn validate(", "Ok(())")
    return replace_body(source, "pub fn matching_user_ids(", "Ok(Vec::new())")


check("keyword-rules", {MODELS / "keyword_alert.rs": broken_keyword}, "keyword_alert_test", [
    "normalizes_whitespace", "rejects_blank_and_overlong_phrases", "rejects_case_insensitive_duplicates_per_user",
    "caps_each_user_at_twenty_phrases", "matches_case_insensitively", "matches_on_word_boundaries_only",
    "matches_multi_word_phrases", "treats_phrases_literally", "returns_every_user_with_a_match",
    "overlapping_phrases_across_users_all_match", "nested_phrases_match_the_same_user_once",
    "repeated_phrases_match_every_holder_once", "ignores_blank_phrases_and_text",
    "matches_line_breaks_without_matching_inside_unicode_words",
])

check("scheduled-claim", {MODELS / "scheduled_message.rs": lambda s: s.replace(
    "sent_at IS NULL AND dropped_at IS NULL", "(sent_at IS NULL OR 1 = 1) AND (dropped_at IS NULL OR 1 = 1)"
).replace("claimed_at IS NULL OR claimed_at < ?", "claimed_at IS NULL OR claimed_at < ? OR 1 = 1")},
    "scheduled_message_test", ["each_row_sends_exactly_once_across_repeated_runs",
        "a_live_claim_blocks_dispatch_and_a_stale_claim_is_recovered", "concurrent_dispatchers_post_once_on_real_sqlite"])

check("scheduled-validation", {MODELS / "scheduled_message.rs": lambda s: replace_body(s, "pub fn validate(", "Ok(Errors::default())")},
    "scheduled_message_test", ["rejects_past_times_blank_bodies_and_overlong_sources", "threads_and_replies_must_belong_to_the_scheduled_conversation"])

check("scheduled-access", {MODELS / "scheduled_message.rs": lambda s: replace_body(s, "pub fn sendable(", "Ok(true)")},
    "scheduled_message_test", ["unusable_rows_are_not_sendable", "rows_whose_author_lost_access_drop_with_an_inbox_item"])

check("scheduled-thread-drop", {MODELS / "scheduled_message.rs": lambda s: replace_body(s, "pub(crate) fn drop_for_thread(", "Ok(())")},
    "scheduled_message_test", ["deleting_a_thread_drops_pending_rows_with_an_inbox_item", "a_row_dropped_after_a_claim_posts_nothing"])

check("category-validation", {
    MODELS / "room_category.rs": lambda s: replace_body(s, "fn validate(", "Ok(())"),
    MODELS / "membership.rs": lambda s: replace_body(s, "fn validate_organization(", "Ok(())"),
}, "room_category_test", ["categories_validate_required_user_name_and_character_limit", "assigning_and_unassigning_categories_validates_the_member_owner"])

check("category-nullify", {MODELS / "room_category.rs": lambda s: s.replace(
    "UPDATE memberships SET room_category_id = NULL WHERE room_category_id = ?",
    "UPDATE memberships SET room_category_id = room_category_id WHERE room_category_id = ?")},
    "room_category_test", ["deleting_a_category_nullifies_memberships_without_touching_them"])

check("favorite-writes", {MODELS / "membership.rs": lambda s: replace_body(s, "fn set_favorite_position(", "Ok(())")},
    "room_category_test", ["favorites_append_idempotently_and_unfavorite_without_reordering_others", "favorites_move_to_absolute_positions_and_clamp_both_ends"])

check("transaction-rollback", {ROOT / "rust/crates/db/src/database.rs": lambda s: s.replace(
    'let _ = conn.execute_batch("ROLLBACK TRANSACTION");', 'let _ = conn.execute_batch("COMMIT TRANSACTION");')},
    "category_and_favorite_writes_rollback_with_the_transaction", ["category_and_favorite_writes_rollback_with_the_transaction"])

def incomplete_golden(source):
    cases = json.loads(source)
    return json.dumps({name: case for name, case in cases.items() if not name.endswith("/pin")}, indent=2) + "\n"

check("save-touch-golden", {ROOT / "rust/crates/db/src/tests/message_save_touches.json": incomplete_golden},
    "message_saves_touch_what_rails_touches", ["message_saves_touch_what_rails_touches"])

print("WS8 discrimination: 10 mutations detected; sources restored", flush=True)
