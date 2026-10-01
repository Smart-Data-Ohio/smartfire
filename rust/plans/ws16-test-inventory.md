# WS16 Rails test inventory — partial

Owner for every deferred or partial row: **WS16 continuation**. No tests are reassigned to other workstreams.

Covered is a behavior mapping, not a claim that the original Rails test was run against Rust. The executable Rust coverage is in `db/src/tests/slack*_test.rs`, `campfire/src/integrations/slack/client/tests.rs`, and the 481 generated converter vectors. Runner/protocol, actual durable worker execution, mappers, quiet Message save/undo and workspace/personal Rails row differentials now have executable coverage. OAuth, connection and setup routes now have Rails-generated transport/HTTP/view goldens and runtime security tests. All fifteen admin/personal run actions have real HTTP session/CSRF/row/audit/queue comparisons and 83 complete template body goldens. The combined setup/sudo/preview/plan/import/progress/undo interaction also executes over HTTP. Remaining fault/large-history cases are listed below.

## test/controllers/accounts/slack_import_runs_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| run pages are admin-only | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| run list shows every run newest first | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting a dry run needs a connected account | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting a dry run is blocked while another run is active | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting a dry run creates a workspace dry run with the options | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting a dry run ignores unparseable dates | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting a dry run defaults to including private channels | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| run page shows status, counts, timestamps, and issues with pagination | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| run page polls while active and stops when finished | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| status frame renders the run without polling itself | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| status frame marks a finished run so polling stops | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| run page shows queued-behind while another run is active | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| administrators can view personal runs from the admin page | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| plan renders conversations, targets, and samples | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| plan escapes Slack text and sample markdown | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| plan needs a completed dry run | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| test import starts with checked conversations and a recent default | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| test import date bounds are full timestamps covering their days | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| full import starts with no date bounds | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting an import keeps only conversations in the dry run | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting an import with only unknown conversations is rejected | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting an import drops room targets outside alive open and closed rooms | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting an import with nothing checked is rejected | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting an import is blocked without a connection or with an active run | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| catch-up repeats a full import's conversations and targets | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| catch-up refuses a date-bounded test import | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| catch-up needs a completed import | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| only a completed full import offers catch-up | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| cancel and undo act on workspace runs | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| administrators may cancel and undo personal runs | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| undo is blocked with a reason while another run is active | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| undo is blocked with a reason while a later import covers the same conversations | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| cancel and undo refuse finished and non-undoable runs | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |

## test/controllers/accounts/slack_imports_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| setup is admin-only | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| setup shows the manifest with the callback URL and user scopes | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| the client secret is never rendered back | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| saving credentials requires sudo | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| saving credentials creates the workspace and records who configured it | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| a blank secret keeps the stored one | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| invalid credentials re-render with errors | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| removing credentials requires sudo | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| removing credentials clears them and every connection but keeps runs | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| removing credentials is blocked while a run is active | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |

## test/controllers/slack/connections_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| disconnect revokes remotely and destroys the connection | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| disconnect redirects a member to the personal page | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| disconnect is blocked while one of the member's runs is active | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| another member's active run does not block disconnect | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| disconnect without a connection still redirects | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| disconnect requires sudo | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |

## test/controllers/slack/imports_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| personal page needs workspace setup first | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| personal page explains the scope and offers connect | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| personal page lists only the member's own runs | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| members can view their own personal runs | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| other members' runs 404 on the personal page | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| personal status frame renders the member's own run | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| workspace runs 404 on the personal page | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| administrators use the admin pages for other members' runs | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting a preview needs workspace setup and a connection | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| starting a preview creates a personal dry run | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| one active run per member | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| another member's active run queues the preview behind it | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| personal import starts from the preview's checked conversations | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| personal import never passes room targets or date bounds | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| personal import needs a completed preview with checked conversations | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| members can cancel and undo their own runs | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| undo is blocked with a reason while another run is active | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| undo is blocked with a reason while a later import covers the same conversations | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |
| run page loads later runs' stats once across the undo checks | partial | One blocked-reason scan feeds the view; the original SQL-count regression still needs instrumentation. |
| personal run page shows the plan with skip checkboxes | covered | 109 real Rails HTTP action goldens with signed sessions, CSRF, role/scope, rows, audits and queue; 83 complete template body goldens; actual HTTP ordering/pagination and combined workflow. |

## test/controllers/slack/oauth_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| start redirects to Slack with user scopes and no bot scope | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| start pins the team once it is known | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| start without configured credentials redirects back | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| start requires sudo | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback success upserts the connection and sets the team | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback success without a team.info answer still connects | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback clears a previous disconnected reason | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback with a state mismatch is rejected | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback state cannot be replayed | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback state is bound to the user who started it | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback with access_denied stores nothing | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback from a different team is rejected | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback missing a required scope is rejected with the missing ones | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback refuses a Slack account linked to another member | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback rescues a duplicate connection raced in after the check | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback fills a missing team name from team.info | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| a failed exchange returns to the page the flow started from | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback with a failed exchange stores nothing | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| callback returns to the personal page when the flow started there | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| an off-allowlist return_to falls back to the default page | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| a member's first connection does not name the workspace team | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |
| the code, token, and secret parameters are filtered from request logs | covered | slack OAuth TLS/state golden tests; 37 real Rails HTTP callback/disconnect/setup/remove scenarios; 11 complete setup body goldens; replay/unique conflict/CSRF/sudo/role/request filtering checks. |

## test/jobs/slack_import/run_lifecycle_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| users map by email, placeholders fill in, guests stay deactivated | covered | slack_users_preview_import_repeat_match_pinned_rails_fixture_rows |
| mentions of mapped users outside the channel render as tokens, unknown ids fall back | deferred | WS16 continuation: implement and exercise the original behavior. |
| placeholder Google-link eligibility follows the allowed domains | covered | slack_users_humans_receive_open_rooms_bots_guests_and_unknown_authors_do_not plus Rails user vector |
| channels merge into same-name rooms without touching memberships | covered | slack_conversations_only_public_workspace_channels_auto_merge |
| room targets force new, skip and explicit rooms, and reject bad ids | covered | slack_conversations_explicit_merge_leaves_memberships_and_invalid_target_reports |
| room target id must be an alive Open or Closed room | covered | slack_conversations_explicit_merge_leaves_memberships_and_invalid_target_reports |
| workspace runs never auto-merge a private channel by name | covered | slack_conversations_only_public_workspace_channels_auto_merge |
| workspace runs merge a private channel only into its room target | covered | slack_conversations_explicit_merge_leaves_memberships_and_invalid_target_reports |
| personal runs never merge a private channel into an existing room | covered | slack_conversations_only_public_workspace_channels_auto_merge |
| a conversation whose mapped room was deleted is skipped with an issue | deferred | WS16 continuation: implement and exercise the original behavior. |
| workspace runs never auto-merge a large group DM by name | deferred | WS16 continuation: implement and exercise the original behavior. |
| workspace runs merge a large group DM only into its room target | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal runs never auto-merge a large group DM by name | deferred | WS16 continuation: implement and exercise the original behavior. |
| dry runs preview a targeted large group DM as a merge | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal runs ignore room target ids | deferred | WS16 continuation: implement and exercise the original behavior. |
| room setup is atomic: a crash while recording memberships leaves nothing behind | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal run imports DMs, group DMs and private channels | covered | slack_sequence_personal_matches_rails_import_undo_reimport_database_rows: every field in 89 tables |
| personal run skips self DMs and Slackbot DMs | covered | slack_conversations_directs_reuse_member_sets_skip_self_and_slackbot |
| personal run dedupes a DM another member already imported | covered | slack_conversations_directs_reuse_member_sets_skip_self_and_slackbot |
| date bounds keep every row in range and are sent to Slack | partial | Client query tests and 18 Rails boundary vectors cover this seam; exact original full bounded fixture scenario remains. |
| cancel stops the run at the next step boundary | covered | slack_sql_store_rejects_cancelled_or_replaced_lease_before_domain_writes |
| a cancel observed mid-step stops the next page from being written | covered | slack_runner_cancelled_during_fetch_does_not_commit_page_or_progress |
| HTTP 429 reschedules the run after Retry-After | covered | slack_job_retry_after_commits_heartbeat_and_delayed_job_atomically |
| auth errors fail the run and flag the connection | covered | slack_job_cancel_during_auth_error_preserves_cancel_and_disconnects plus client error vectors |
| scope errors fail with the missing scope and leave the connection | partial | Client vector plus shared job failure branch; direct job scope-failure scenario remains. |
| transient failures past the retry budget fail the run | partial | Client exhaustion/protocol guards implemented; exact non-Slack Ruby exception class text remains. |
| failed runs stay resumable: a new run continues from the mapping | deferred | WS16 continuation: implement and exercise the original behavior. |
| workspace runs exclude private channels when asked | deferred | WS16 continuation: implement and exercise the original behavior. |
| two queued runs run one at a time | covered | slack_job_live_lease_and_lost_run_claim_never_execute_callback plus registered serial worker |

## test/jobs/slack_import/workspace_import_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| dry run writes nothing except the run row and its issues | covered | slack_sql_store_dry_run_collects_samples_without_domain_rows_or_reply_fetches |
| workspace import creates rooms, memberships, messages, threads, boosts and pins | covered | slack_sequence_matches_rails_import_undo_reimport_database_rows: every field in 89 tables |
| tiny step budget spans several steps for one conversation | covered | slack_runner_zero_budget_stops_after_page_and_preserves_reply_cursor plus actual fixture completion |
| a second import creates zero duplicates | covered | slack_undo_actual_overlapping_imports_require_lifo_and_name_later_importer |
| catch-up picks up a new message and a late reply | deferred | WS16 continuation: implement and exercise the original behavior. |
| catch-up skips a thread whose mapped thread was deleted | deferred | WS16 continuation: implement and exercise the original behavior. |
| import skips a thread whose parent message was deleted mid-run | deferred | WS16 continuation: implement and exercise the original behavior. |
| a multi-year conversation spanning several steps imports everything | deferred | WS16 continuation: implement and exercise the original behavior. |
| a full import after a date-bounded test import imports everything older too | deferred | WS16 continuation: implement and exercise the original behavior. |
| catch-up after a kept test import and a full import re-reads only 30 days | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo is last-in, first-out per conversation | covered | slack_undo_actual_overlapping_imports_require_lifo_and_name_later_importer |
| a full import's coverage ends when an earlier run under it is undone | deferred | WS16 continuation: implement and exercise the original behavior. |
| a full import finishes rooms an earlier test import created | deferred | WS16 continuation: implement and exercise the original behavior. |
| finishing never moves an earlier membership's read pointer backwards | deferred | WS16 continuation: implement and exercise the original behavior. |
| per-conversation record lookups seek the identity index | deferred | WS16 continuation: implement and exercise the original behavior. |
| finishing refreshes the heartbeat while looping over rooms | deferred | WS16 continuation: implement and exercise the original behavior. |
| completing a run kicks the next queued run | deferred | WS16 continuation: implement and exercise the original behavior. |
| undoing a run kicks the next queued run | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo is blocked while another run is queued | covered | 109 real Rails HTTP actions compare blocking reason and unchanged DB/queue. |
| message timestamps keep exact microseconds | covered | slack_writer_timestamp_microseconds_do_not_round_through_float plus full Rails row differential |
| truncated reaction lists import the listed users with one issue per message | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo removes exactly what the run created and leaves the rest | covered | slack_undo_deletes_data_search_and_mappings_then_reimports_fixture plus kept-content tests and Rails differential |
| undo keeps mappings for kept placeholders, so re-import creates no duplicates | partial | Claimed-user mapping and kept-root reimport covered; all original session/Google/authorship variants remain. |
| undo keeps a created room that gained foreign messages and records an issue | covered | slack_undo_keeps_live_thread_parent_for_foreign_or_scheduled_replies |
| undo keeps a created room that holds an event and a scheduled message | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo keeps threads and rooms with real activity, with members and mappings | covered | slack_undo_keeps_live_thread_parent_for_foreign_or_scheduled_replies |
| undo keeps the parent of a thread holding a saved reply | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo keeps an imported message someone started a thread on | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo keeps imported messages holding a poll, a saved item or someone else's pin | covered | slack_undo_keeps_poll_saved_item_foreign_pin_and_pending_root_reply |
| undo keeps a thread whose reply holds a poll | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo keeps a thread with a pending scheduled reply, and the message it quotes | covered | slack_undo_keeps_live_thread_parent_for_foreign_or_scheduled_replies |
| undo removes a thread whose scheduled replies were all sent | deferred | WS16 continuation: implement and exercise the original behavior. |
| import stays silent: no foreign jobs, broadcasts, unread or inbox items | covered | slack_writer_history_replies_pins_reactions_are_quiet_and_keep_microseconds; rejecting broadcast sink and real job queue |
| imported messages are searchable | covered | slack_writer_history_replies_pins_reactions_are_quiet_and_keep_microseconds; message_search_index included in both Rails differentials |

## test/models/slack/client_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| auth.test sends a bearer user token | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| team.info | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| users.list pages with cursor and limit | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| conversations.list sends types and keeps archived channels | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| conversations.members sends the channel | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| conversations.history sends channel, bounds, cursor and limit 200 | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| history fixtures arrive newest-first like conversations.history | partial | Fixture byte identity is covered; explicit ordering assertion still needed. |
| conversations.replies sends channel and parent ts | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| HTTP 429 raises a rate-limit error carrying Retry-After | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| HTTP 429 without Retry-After defaults to 60 seconds | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| 5xx responses retry with backoff then raise | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| network errors retry then raise | partial | Exact Ruby transport exception class/message remains unported; retry count and final RequestError are covered. |
| a 5xx that recovers returns the payload | partial | Fixture server sequence recovery still needed. |
| ok:false auth errors raise AuthError | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| ok:false missing_scope raises ScopeError with the needed scope | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| other ok:false errors raise RequestError without retrying | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |
| on_request fires once per attempt for api call counts | partial | Callback counts tested for failed retries; recovery sequence still needed. |
| pacing sleeps between rapid Tier 2 calls when enabled | covered | Local TLS fixture requests, Rails error vectors, retry and pacing tests. |

## test/models/slack/markdown_converter_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| unescapes Slack entities | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| user mentions become Smartfire mention tokens | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| unknown users fall back to the label or id | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| channel references become hashes | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| broadcast mentions stay literal text | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| broadcast mentions with labels stay literal text | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| subteam mentions become handles and dates become fallbacks | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| date tokens keep their fallback with or without a link part | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| a crafted date token converts in linear time | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| links convert to markdown and bare urls pass through | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| mailto links convert | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| emphasis converts to markdown | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| emphasis leaves snake_case and code untouched | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| emphasis leaves urls alone | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| emphasis leaves bare urls and angle-bracket links alone | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| code on the fence's first line stays code | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| bullets become dashes | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| emoji shortcodes are kept and skin tones stripped | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| files append link lines | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| files without any url are skipped | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| attachments quote in when the text is empty | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| bot messages quote attachments even with text | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| human messages with text skip attachments | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| me messages become italic | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| long messages truncate at the source limit with a flag | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| short messages are not flagged | covered | Rails-generated converter vectors; crafted-token timing guard is a Rust test. |
| rendering: mentions resolve to attachments through a real save | deferred | WS16 continuation: implement and exercise the original behavior. |
| rendering: broadcast mentions create no attachments or notifications | deferred | WS16 continuation: implement and exercise the original behavior. |
| rendering: fenced first-line code and bare urls render as intended | deferred | WS16 continuation: implement and exercise the original behavior. |
| rendering: emphasis, links, bullets and quotes render as markdown | deferred | WS16 continuation: implement and exercise the original behavior. |

## test/models/slack_import_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| start! creates a queued run with normalized options and enqueues it | covered | 33 actual Ruby normalization/time vectors plus real HTTP run creation and durable enqueue comparison. |
| start! defaults to including private channels | covered | 33 actual Ruby normalization/time vectors plus real HTTP run creation and durable enqueue comparison. |
| start! rejects unparseable date bounds | covered | 33 actual Ruby normalization/time vectors plus real HTTP run creation and durable enqueue comparison. |
| cancel! stops queued and running runs at their boundary | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| undo! enqueues the undo job and resets progress tracking | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| undo! refuses dry runs and active runs | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| claim_running! lets a single queued run through while none runs | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| claim_running! refuses while another run is undoing | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| claim_running! refuses while a cancelled run holds a fresh step lease | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| a stale step lease no longer blocks claims | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| a second acquire fails while a fresh lease is held | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| a stale lease can be taken over without losing saved progress | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| releasing with the wrong token keeps the lease | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| a step that raises still releases its lease | deferred | WS16 continuation: implement and exercise the original behavior. |
| a continuing step releases its lease before enqueueing the next job | deferred | WS16 continuation: implement and exercise the original behavior. |
| a continuing undo releases its lease before enqueueing the next job | deferred | WS16 continuation: implement and exercise the original behavior. |
| refresh_step_lease! renews a held lease without touching other state | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| refresh_step_lease! refuses a token that no longer holds the lease | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| a runner step refreshes its lease wherever it refreshes the heartbeat | deferred | WS16 continuation: implement and exercise the original behavior. |
| a runner without a lease token leaves the lease alone | deferred | WS16 continuation: implement and exercise the original behavior. |
| an undoer step refreshes its lease when it saves undo state | deferred | WS16 continuation: implement and exercise the original behavior. |
| another job's lease write is not mistaken for progress on conflict | deferred | WS16 continuation: implement and exercise the original behavior. |
| a lease-looking state on a completed run does not block claims | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| step lease stamps are written in UTC even under a user time zone | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| fresh leases block and stale leases pass under user time zones | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| a step job that cannot claim its queued run exits without re-enqueueing | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo! refuses with no status change while another run is queued, running or undoing | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| the undo claim itself refuses a queued run that slips in after the pre-check | deferred | WS16 continuation: implement and exercise the original behavior. |
| the undo claim itself refuses a fresh lease held outside an active status | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| undo_blocked_reason is nil when nothing else is active | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| undo waits while the run itself holds a fresh step lease | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| undo waits while another run holds a fresh step lease | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| the later-overlap answer refreshes after reload | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| failed and cancelled runs kick the next queued run | deferred | WS16 continuation: implement and exercise the original behavior. |
| step_finishing does not overwrite a cancelled run | deferred | WS16 continuation: implement and exercise the original behavior. |
| step_finishing completes a running run and kicks the next queued one | deferred | WS16 continuation: implement and exercise the original behavior. |
| record_issue! caps issues with a suppression notice | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| sweep re-enqueues stale running runs | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| sweep starts the oldest queued run only when nothing runs or undoes | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| sweep never enqueues a second job for a run with one pending | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |
| sweep re-enqueues stalled undoing runs | covered | Database lifecycle tests; independent writers for claims, leases, undo and sweeps. |

## test/system/slack_import_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| admin saves credentials, dry-runs, plans, and watches a test import | covered | slack_admin_credentials_preview_plan_import_progress_and_undo_over_http: real sudo password/session, credentials, preview, selection, import, polling and undo controls. |

## Totals

190 covered, 9 partial, 45 deferred; 244 Rails tests inventoried.
