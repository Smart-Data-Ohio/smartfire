#!/usr/bin/env python3
"""Require named compiled tests to reject regressions; always restore each source."""
import os
from pathlib import Path
import re
import subprocess
import sys

FILTER = sys.argv[1] if len(sys.argv) > 1 else ""
COUNT = 0

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / ".scratch/features-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, CI="1", TMPDIR=str(ROOT / ".scratch/tmp"), CARGO_TARGET_DIR=str(ROOT / "rust/target"),
           CABLE_TEST_PORT_RANGE="52500-52549", MAIL_TEST_PORT_RANGE="52550-52599")


def check(name, relative, old, new, test):
    global COUNT
    if FILTER and not name.startswith(FILTER):
        return
    COUNT += 1
    source = ROOT / relative
    original = source.read_text()
    assert old in original, f"missing mutation anchor: {name}"
    full_test = test if test.startswith(("channels::", "controllers::")) else f"controllers::message_features::tests::{test}"
    try:
        source.write_text(original.replace(old, new, 1))
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


check("membership", "rust/crates/campfire/src/controllers/message_features.rs",
      "let (_, room) = concerns::set_room(c).await?;",
      'let id = c.param_str("room_id").and_then(cast_integer).ok_or(Error::NotFound)?;\n'
      '    let room = c.app().db.read(move |conn| Room::find(conn, id)).await.map_err(db_error)?;',
      "poll_membership_and_human_gates")
check("pin-reachability", "rust/crates/campfire/src/controllers/message_features.rs",
      "Message::find_reachable(conn, user_id, id)",
      "{ let _ = user_id; Message::find(conn, id) }", "pins_require_reachable_messages_and_rooms")
check("csrf", "rust/crates/campfire/src/controllers/rooms/polls.rs",
      "before_actions(c, Before::default()).await?;", "before_actions(c, Before::default().skip_forgery_protection()).await?;",
      "poll_and_pin_writes_require_csrf_and_reject_bot_keys")
check("anonymous-voters", "rust/crates/views/src/messages/parts.rs",
      "if self.anonymous {", "if false {", "poll_partials_match_rails_open_voted_anonymous_closed_and_error_bytes")
check("pin-list-bytes", "rust/crates/views/templates/rooms/pins/_list.html",
      'class="pins-panel__excerpt"', 'class="pins-panel__excerpt-broken"', "pin_partials_match_rails_list_count_badge_frame_and_empty_bytes")
check("broadcast-origin", "rust/crates/campfire/src/channels/message_features.rs",
      ".with(|value| value.borrow().clone())", ".with(|_| None::<String>)", "poll_and_pin_frames_reach_a_real_websocket_without_session_values")
check("dst-gap", "rust/crates/db/src/slash_commands/time_parser.rs",
      "dt.checked_add(Span::new().hours(1))", "dt.checked_add(Span::new().minutes(30))", "builder_dates_match_rails_zones_and_dst_gap_fold")
check("pin-cap", "rust/crates/db/src/models/message_pin.rs",
      "MAX_PER_ROOM: i64 = 50", "MAX_PER_ROOM: i64 = 51", "pins_enforce_the_cap_but_repinning_a_full_room_succeeds")
check("poll-json-order", "rust/crates/db/src/models/poll.rs",
      '"id": self.id,\n            "message_id": self.message_id,',
      '"message_id": self.message_id,\n            "id": self.id,', "poll_http_matches_real_rails_responses_and_transactional_create_errors")
check("pin-note", "rust/crates/db/src/models/message_pin.rs",
      "if let Some(note) = pin.post_pin_note(tx, message)? {", "if let Some(note) = None::<Message> {",
      "pin_requests_match_rails_and_keep_the_note_quiet_and_idempotent")
check("atomic-job", "rust/crates/campfire/src/jobs.rs",
      "let id = self.queue.enqueue(tx, &request)?;", "let id = 0;",
      "poll_creation_rolls_back_when_the_durable_job_insert_is_rejected")
check("turbo-vote", "rust/crates/campfire/src/controllers/rooms/polls.rs",
      "campfire_cable::turbo::Action::Replace", "campfire_cable::turbo::Action::Append",
      "ballots_replace_and_retract_in_turbo_and_reject_foreign_rooms_and_odd_shapes")
check("sti-pin-list", "rust/crates/campfire/src/controllers/rooms/pins.rs",
      "campfire_db::broadcasts::room_param_key(room.room_type)", '"rooms_closed".to_owned()',
      "boards_reject_root_polls_and_pin_lists_keep_their_sti_dom_identity")
check("request-zone", "rust/crates/campfire/src/controllers/message_features.rs",
      "campfire_views::time::Zone::for_user(name.as_deref())", '{ let _ = name; campfire_views::time::Zone::utc() }',
      "a_zone_local_multiple_anonymous_poll_shows_only_each_viewers_ballot")
check("pin-order", "rust/crates/db/src/models/message_pin.rs",
      '\"message_pins\".\"created_at\" DESC, \"message_pins\".\"id\" DESC',
      '\"message_pins\".\"created_at\" ASC, \"message_pins\".\"id\" ASC',
      "pins_list_orders_newest_first_and_keeps_thread_jump_links")
check("origin-restoration", "rust/crates/campfire/src/channels/message_features.rs",
      "ORIGIN.with(|origin| origin.replace(self.0.take()));", "let _ = &self.0;",
      "channels::message_features::tests::origin_is_present_for_commit_callbacks_and_restored_after_errors_and_panics")
check("saved-json", "rust/crates/campfire/src/controllers/saved_items.rs",
      'format!("{}.json", campfire_routes::saved_item(item.id))', 'format!("{}", campfire_routes::saved_item(item.id))',
      "controllers::message_features::saved_tests::saved_http_matches_rails_exact_json_and_no_store_headers")
check("saved-partials", "rust/crates/views/templates/saved_items/_item.html", 'class="saved-item__body"', 'class="saved-item__body-broken"',
      "controllers::message_features::saved_tests::saved_partials_match_rails_for_reminders_statuses_zones_and_empty_page")
check("saved-csrf", "rust/crates/campfire/src/controllers/saved_items.rs", "before_actions(c, Before::default()).await?;", "before_actions(c, Before::default().skip_forgery_protection()).await?;",
      "controllers::message_features::saved_tests::saved_mutations_require_csrf_and_turbo_redirects_keep_the_filter")
check("saved-reminder-job", "rust/crates/campfire/src/jobs.rs", "let id = self.queue.enqueue(tx, &request)?;", "let id = 0;",
      "controllers::message_features::saved_tests::reminder_dispatch_rolls_back_failed_jobs_and_refires_the_same_inbox_item")
check('scheduled-race-edit', 'rust/crates/campfire/src/controllers/scheduled_messages.rs', 'let mut row = ScheduledMessage::find(tx.conn(), initial.id)?;', 'let mut row = initial;', 'controllers::message_features::scheduled_tests::a_send_between_lookup_and_lock_refuses_the_edit')
check('scheduled-race-cancel', 'rust/crates/campfire/src/controllers/scheduled_messages.rs', 'let row = ScheduledMessage::find(tx.conn(), initial.id)?;', 'let row = initial;', 'controllers::message_features::scheduled_tests::a_send_between_lookup_and_lock_refuses_the_cancel')
check('scheduled-zone', 'rust/crates/campfire/src/controllers/scheduled_messages.rs', 'let zone = features::user_zone(c).await?;', 'let zone = { let _ = features::user_zone(c).await?; campfire_views::time::Zone::utc() };', 'controllers::message_features::scheduled_tests::scheduled_http_matches_rails_json_offsets_and_claim_outcomes')
check('scheduled-partials', 'rust/crates/views/templates/scheduled_messages/_item.html', 'class="scheduled-message__meta"', 'class="scheduled-message__meta-broken"', 'controllers::message_features::scheduled_tests::scheduled_row_partials_match_rails_and_empty_page_bytes')
check('scheduled-csrf', 'rust/crates/campfire/src/controllers/scheduled_messages.rs', 'before_actions(c, Before::default()).await?;', 'before_actions(c, Before::default().skip_forgery_protection()).await?;', 'controllers::message_features::scheduled_tests::scheduled_mutations_require_csrf_and_busy_claims_accept_no_parameters')
check('scheduled-job', 'rust/crates/campfire/src/jobs.rs', 'let id = self.queue.enqueue(tx, &request)?;', 'let id = 0;', 'controllers::message_features::scheduled_tests::scheduled_send_rolls_back_claim_post_and_history_when_job_insert_fails')
check('scheduled-socket', 'rust/crates/campfire/src/channels/message_features.rs', 'if !pin_note && !scheduled {', 'if !pin_note {', 'controllers::message_features::scheduled_tests::scheduled_send_reaches_a_real_websocket_with_one_token_free_message_frame')
check('scheduled-composer', 'rust/crates/views/templates/scheduled_messages/_composer_button.html', 'data-controller="schedule-send"', 'data-controller="broken"', 'controllers::message_features::scheduled_tests::scheduled_composer_controls_match_rails_for_room_and_thread')
check('search-membership', 'rust/crates/db/src/models/search_query.rs', 'mem.room_id=rooms.id AND mem.user_id=?)', 'mem.room_id=rooms.id AND mem.user_id=? OR rooms.id=messages.room_id)', 'controllers::searches::ports::unreachable_messages_are_not_found')
check('search-csrf', 'rust/crates/campfire/src/controllers/searches.rs', 'pub async fn create(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default()).await?;', 'pub async fn create(c: &mut Ctx) -> Result {\n    before_actions(c, Before::default().skip_forgery_protection()).await?;', 'controllers::searches::ports::search_mutations_require_csrf')
check('search-tuple', 'rust/crates/db/src/models/search_query.rs', '(messages.created_at,messages.id)<(?,?)', '(messages.created_at,messages.id)<=(?,?)', 'controllers::searches::ports::results_page_through_load_older_results_with_same_timestamp')
check('search-zone', 'rust/crates/db/src/models/search_query.rs', 'local_datetime(date.to_datetime(Time::MIN), zone)', 'local_datetime(date.to_datetime(Time::MIN), &TimeZone::UTC)', 'controllers::searches::ports::on_narrows_results_to_that_day_in_the_users_zone_including_dst_and_missing_days')
check('search-missing-day', 'rust/crates/db/src/models/search_query.rs', 'let resolved_date = start.jiff().to_zoned(zone.clone()).date();', 'let resolved_date = date;', 'controllers::searches::ports::on_narrows_results_to_that_day_in_the_users_zone_including_dst_and_missing_days')
check('search-chips', 'rust/crates/views/templates/searches/_filters.html', 'class="search-filter-chip__label"', 'class="search-filter-chip__label-broken"', 'controllers::searches::ports::search_parser_chips_and_empty_page_match_pinned_rails_bytes')
check('search-header-limit', 'rust/crates/db/src/models/search.rs', 'SELECT * FROM searches WHERE user_id=? ORDER BY updated_at DESC LIMIT 10', 'SELECT * FROM searches WHERE user_id=? ORDER BY updated_at DESC LIMIT 12', 'controllers::searches::ports::the_header_renders_at_most_ten_recents')
check('search-fts', 'rust/crates/db/src/models/search_query.rs', 'format!("\\"{w}\\"")', 'w.to_string()', 'controllers::searches::ports::a_boolean_looking_query_does_not_exclude_terms')
check('search-section-time', 'rust/crates/views/templates/searches/_sections.html', '"time",h::attrs()', '"datetime",h::attrs()', 'controllers::searches::ports::search_sections_load_older_and_older_stream_match_pinned_rails_bytes')
check('preload-lazy', 'rust/crates/campfire/src/controllers/searches.rs', 'let p = p.preload_search(messages)?;', 'let mut p = p.preload_search(messages)?; let data = p.search_preloads.take(); let _ = p.messages(messages)?; p.search_preloads = data;', 'controllers::searches::ports::full_message_preloads_keep_queries_constant')
check('preload-pin', 'rust/crates/campfire/src/controllers/searches/preloads.rs', 'pinned: self.records.pinned.contains(&m.id),', 'pinned: false,', 'controllers::searches::ports::preloaded_complete_messages_match_rails_and_lazy_presenter')
print(f"WS8bm2 discrimination: {COUNT} compiled regressions detected; sources restored", flush=True)
