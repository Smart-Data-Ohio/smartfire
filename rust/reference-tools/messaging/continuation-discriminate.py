#!/usr/bin/env python3
"""Compile paging, publisher, thread-membership and collection regressions; restore sources."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / ".scratch/continuation-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, CI="1", TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"),
           CARGO_BUILD_JOBS="4", CABLE_TEST_PORT_RANGE="52000-52049", MAIL_TEST_PORT_RANGE="52000-52049")


def check(name, path, old, new, test):
    source = ROOT / path
    original = source.read_text()
    assert old in original, f"missing mutation anchor: {name}"
    try:
        source.write_text(original.replace(old, new))
        run = subprocess.run(["cargo", "test", "--locked", "-j", "4",
                              "-p", "campfire", "--bin", "campfire", test, "--", "--exact", "--nocapture"],
                             cwd=ROOT / "rust", env=ENV, capture_output=True, text=True)
        output = run.stdout + run.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summary = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert run.returncode != 0 and summary, f"{name}: no compiled test failure"
        assert f"test {test} ... FAILED" in output, f"{name}: wrong failing test"
        print(f"{name}: {summary[0]}", flush=True)
    finally:
        source.write_text(original)


controller = "rust/crates/campfire/src/controllers/messages.rs"
paging = "controllers::messages::paging_tests::"
threads = "rust/crates/campfire/src/controllers/channel_threads.rs"
thread_tests = "controllers::channel_threads::tests::"
check("root-page-scope", controller, "Message::find_in(conn, timeline, id)",
      "match timeline { Timeline::Room(room_id) => Message::find_in_room(conn, room_id, id), _ => Message::find_in(conn, timeline, id) }",
      paging + "page_anchors_require_alive_membership_and_a_root_message")
check("validator-pin-set", "rust/crates/campfire/src/controllers/messages/freshness.rs",
      'pairs.join(", ")', 'String::new()', paging + "validators_observe_related_rows_and_older_unpins_without_message_touches")
check("publisher-rendered-message", "rust/crates/campfire/src/controllers/messages/rendered.rs",
      "Ok(Some(views::uncached_message(ctx, &view)))", "Ok(Some(String::new()))",
      "channels::tests::hub_test::message_parity::domain_message_descriptions_render_through_the_guard_and_publisher")
check("thread-cross-room", threads, "if thread.room_id != room_id", "if false && thread.room_id != room_id",
      thread_tests + "membership_actions_require_alive_parent_membership_and_thread_scope")
check("thread-join-csrf", threads,
      "pub async fn join(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default()).await?;",
      "pub async fn join(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default().skip_forgery_protection()).await?;",
      thread_tests + "thread_membership_writes_reject_bot_keys_and_forged_csrf_without_rows")
check("collection-streaming", "rust/crates/campfire/src/controllers/presenters/message_cache.rs",
      "streaming: message.streaming", "streaming: false",
      "controllers::messages::collection_tests::collection_cache_tracks_pin_thread_stream_edit_drive_and_reaction_states")
check("collection-null-room", "rust/crates/views/src/fragment_cache/keys.rs",
      '.to_owned() + "nil]"', '.to_owned() + "empty]"',
      "controllers::messages::collection_tests::nullable_quote_name_digest_matches_ruby_inspect_and_comparison_errors")
check("room-unread-marker", "rust/crates/campfire/src/controllers/presenters/room_list.rs",
      "records.iter().position(|record| record.id == id)", "records.iter().position(|record| record.id == -id)",
      "controllers::messages::room_list_tests::room_list_places_unread_outside_shared_fragments_and_matches_rails_around_pages")
check("forward-note-bytes", "rust/crates/campfire/src/controllers/presenters.rs",
      "forward_note: message.forward_note.clone()", "forward_note: None",
      "controllers::messages::state_tests::complete_message_states_match_rails_on_cache_misses_and_hits")
check("agent-step-order", "rust/crates/campfire/src/controllers/presenters.rs",
      "ORDER BY position, id", "ORDER BY position DESC, id DESC",
      "channels::tests::hub_test::message_parity::all_owned_message_states_publish_the_actual_rails_append_replace_remove_bytes")
nested = "rust/crates/campfire/src/controllers/channel_thread_messages.rs"
nested_tests = "controllers::channel_thread_messages::tests::"
check("nested-thread-room", nested, "if thread.room_id != room_id", "if false && thread.room_id != room_id",
      nested_tests + "nested_reads_require_alive_membership_and_both_thread_and_message_scope")
check("nested-message-scope", nested, "Message::find_in(conn, Timeline::Thread(thread_id), id)",
      "{ let _ = thread_id; Message::find(conn, id) }",
      nested_tests + "nested_reads_require_alive_membership_and_both_thread_and_message_scope")
check("nested-edit-author", nested, "messages::ensure_can_edit(c, &message)?;", "// removed author gate",
      "controllers::channel_thread_messages::write_tests::nested_writes_enforce_author_admin_notes_scope_and_csrf_before_changes")
check("nested-edit-stream", nested, "messages::rendered::broadcast_thread_edit(c, &room, &message, drive_given)",
      "messages::rendered::broadcast_edit(c, &room, &message, drive_given)",
      "channels::tests::hub_test::message_parity::thread_writes_publish_rails_bytes_on_the_correct_streams_without_retry_frames")
check("bodyless-presentation", "rust/crates/campfire/src/controllers/presenters.rs",
      "if missing_body {", "if false && missing_body {",
      "controllers::messages::state_tests::complete_message_states_match_rails_on_cache_misses_and_hits")
check("thread-pages-bot", threads,
      "pub async fn index(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default()).await?;",
      "pub async fn index(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default().allow_bot_access()).await?;",
      thread_tests + "thread_pages_deny_bot_keys_without_joining_the_browser")
check("thread-list-stale", threads,
      "(board || thread.auto_archive_at() > now)", "(board || true || thread.auto_archive_at() > now)",
      "controllers::channel_threads::page_tests::thread_state_lists_and_standalone_reads_match_rails_bytes")
check("work-event-note", "rust/crates/campfire/src/controllers/messages/payload.rs",
      'event["note"] = note.into();', 'let _ = note;',
      "controllers::channel_threads::page_tests::thread_state_lists_and_standalone_reads_match_rails_bytes")
print("WS8bm continuation discrimination: 18 compiled regressions detected; sources restored", flush=True)
