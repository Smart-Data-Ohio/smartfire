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
        run = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
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
print("WS8bm continuation discrimination: 10 compiled regressions detected; sources restored", flush=True)
