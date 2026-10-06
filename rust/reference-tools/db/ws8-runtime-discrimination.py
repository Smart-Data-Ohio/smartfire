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


def check(name, changes, test_filter, must_fail, package="campfire_db"):
    original = {path: path.read_text() for path in changes}
    try:
        for path, mutate in changes.items():
            broken = mutate(original[path])
            assert broken != original[path], f"mutation did not change {path}"
            path.write_text(broken)
        run = subprocess.run(
            ["cargo", "test", "-j", "4", "-p", package, test_filter],
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



APP=ROOT/"rust/crates/campfire/src"
check("renderer-failure-propagation", {MODELS/"message.rs": lambda s: replace_body(s,"fn prepare_body(","Ok(body.to_string())").replace("rich_text.try_to_plain_text(conn, html, &names).map(|s| !s.trim().is_empty()).map_err(crate::Error::Other)","Ok::<bool,crate::Error>(!rich_text.to_plain_text(conn, html, &names).trim().is_empty())").replace("rich_text.try_to_plain_text(conn, &html, &names).map_err(crate::Error::Other)?","rich_text.to_plain_text(conn, &html, &names)").replace("rich_text.try_markdown_plain_text(conn, &html, &names).map_err(crate::Error::Other)?","rich_text.markdown_plain_text(conn, &html, &names)")}, "rich_text_failure_test", ["canonical_errors_abort_message_writes","plain_text_errors_abort_message_writes","mention_errors_abort_message_writes"])
check("write-flow-markdown", {APP/"rich_text.rs": lambda s: replace_body(s,"fn render_markdown(","BasicRichText.render_markdown(conn,source,room_id)")}, "runtime_scheduled_edit_and_forward_match_rails", ["runtime_scheduled_edit_and_forward_match_rails"], "campfire")
check("real-file-copier", {APP/"messaging.rs": lambda s: replace_body(s,"fn copy(","Err(campfire_db::Error::Other(\"copy scaffold\".into()))")}, "ws8_storage_copies_match_rails", ["ws8_storage_copies_match_rails_and_rollback_on_durable_enqueue_failure"], "campfire")
check("file-rollback", {APP/"messaging.rs": lambda s: s.replace("crate::active_storage::keep_after_commit(tx, staged);", "staged.keep();")}, "ws8_storage_copies_match_rails", ["ws8_storage_copies_match_rails_and_rollback_on_durable_enqueue_failure"], "campfire")
for label in ["Saved item reminder failed","Scheduled message failed","Poll closing failed"]:
    check("loop-"+label.split()[0].lower(), {APP/"jobs/periodic.rs": lambda s,label=label: s.replace(f'tracing::error!(id,%error,"{label}");','return Err(error.into());')}, "ws8_periodic_row_failures_continue_like_rails", ["ws8_periodic_row_failures_continue_like_rails"], "campfire")
print("WS8 runtime discrimination: 7 mutations detected; sources restored",flush=True)
