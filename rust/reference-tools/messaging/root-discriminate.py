#!/usr/bin/env python3
"""Compile root-edit/actions regressions; require the named test to fail and restore sources."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
CONTROLLER = ROOT / "rust/crates/campfire/src/controllers/messages.rs"
PRESENTER = ROOT / "rust/crates/campfire/src/controllers/presenters.rs"
PAYLOAD = ROOT / "rust/crates/campfire/src/controllers/messages/payload.rs"
MODEL = ROOT / "rust/crates/db/src/models/message.rs"
SCRATCH = ROOT / ".scratch/root-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, CI="1", TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"),
           CARGO_BUILD_JOBS="4", CABLE_TEST_PORT_RANGE="52000-52049", MAIL_TEST_PORT_RANGE="52000-52049")


def check(name, source, old, new, test):
    original = source.read_text()
    assert old in original, f"missing mutation anchor: {name}"
    full_test = f"controllers::messages::root_tests::{test}"
    try:
        source.write_text(original.replace(old, new))
        run = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
                              "-p", "campfire", "--bin", "campfire", full_test, "--", "--exact", "--nocapture"],
                             cwd=ROOT / "rust", env=ENV, capture_output=True, text=True)
        output = run.stdout + run.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summary = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert run.returncode != 0 and summary, f"{name}: no compiled test failure"
        assert f"test {full_test} ... FAILED" in output, f"{name}: wrong failing test"
        print(f"{name}: {summary[0]}", flush=True)
    finally:
        source.write_text(original)


check("actions-membership", CONTROLLER, "let (_, room) = concerns::set_room(c).await?;",
      "let room_id = c.param_str(\"room_id\").and_then(cast_integer).ok_or(Error::NotFound)?;\n"
      "    let room = c.app().db.read(move |conn| Room::find(conn, room_id)).await.map_err(db_error)?;",
      "actions_require_alive_membership_and_exclude_thread_messages")
check("actions-root-scope", CONTROLLER, "Message::find_in(conn, Timeline::Room(room_id), id)",
      "Message::find_in_room(conn, room_id, id)", "actions_require_alive_membership_and_exclude_thread_messages")
check("actions-immutable", PAYLOAD, "!message.system_note && ", "",
      "actions_match_rails_for_two_members_without_shared_viewer_state")
check("actions-viewer", PAYLOAD, "SavedItem::find_by_user_and_message(p.conn, viewer.id, message.id)?",
      "SavedItem::find_by_user_and_message(p.conn, message.creator_id, message.id)?",
      "actions_match_rails_for_two_members_without_shared_viewer_state")
check("legacy-rendered-source", PRESENTER, "let body = self.rendered_body_html(message)?;",
      "let body = message.body_html(self.conn)?.unwrap_or_default();",
      "actions_match_rails_for_two_members_without_shared_viewer_state")
check("legacy-snapshot", CONTROLLER, "Presenter::new(tx.conn(), &app, host).rendered_body_html(&message)?",
      "message.body_html(tx.conn())?.unwrap_or_default()",
      "updates_match_rails_json_and_saved_rows_including_legacy_conversion")
check("markdown-clear", CONTROLLER, "clear_markdown_source: attributes.markdown_source.is_none()",
      "clear_markdown_source: false", "updates_match_rails_json_and_saved_rows_including_legacy_conversion")
check("update-job-atomicity", MODEL,
      "tx.emit_after_commit(Event::job(&crate::models::message_reference::QuoteCardsRefreshJob { source_message_id: self.id }));",
      "// Deliberately omit the transactional enqueue.",
      "update_rolls_back_text_and_drive_changes_when_atomic_job_insert_fails")
check("update-csrf", CONTROLLER,
      "pub async fn update(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default()).await?;",
      "pub async fn update(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default().skip_forgery_protection()).await?;",
      "edit_http_uses_markdown_source_and_update_requires_csrf")
check("edit-http-source", CONTROLLER, "editable_body_html: presenter.editable_markdown_source(&message)?",
      "editable_body_html: presenter.body_html(&message)?", "edit_http_uses_markdown_source_and_update_requires_csrf")
check("actions-bot-denial", CONTROLLER,
      "pub async fn actions(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default()).await?;",
      "pub async fn actions(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default().allow_bot_access()).await?;",
      "valid_bot_keys_are_forbidden_on_root_actions_and_updates")
check("update-bot-denial", CONTROLLER,
      "pub async fn update(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default()).await?;",
      "pub async fn update(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default().allow_bot_access()).await?;",
      "valid_bot_keys_are_forbidden_on_root_actions_and_updates")
check("json-wire-order", PAYLOAD,
      'json!({"file_id": id, "url": format!("https://drive.google.com/open?id={id}")})',
      'json!({"url": format!("https://drive.google.com/open?id={id}"), "file_id": id})',
      "updates_match_rails_json_and_saved_rows_including_legacy_conversion")
check("edit-form-bytes", ROOT / "rust/crates/views/templates/messages/edit.html",
      '.attr("maxlength", 50000)', '.attr("maxlength", 49999)', "edit_forms_and_actions_menu_match_rails_bytes")
print("WS8bm root discrimination: 14 compiled regressions detected; sources restored", flush=True)
