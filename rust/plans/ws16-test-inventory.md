# WS16 Rails test inventory — partial

Owner for every deferred or partial row: **WS16 continuation**. No tests are reassigned to other workstreams.

Covered is a behavior mapping, not a claim that the original Rails test was run against Rust. The executable Rust coverage is in `db/src/tests/slack*_test.rs`, `campfire/src/integrations/slack/client/tests.rs`, and the 481 generated converter vectors. Runner/protocol, actual durable worker execution, mappers, quiet Message save/undo and workspace/personal Rails row differentials now have executable coverage. HTTP/OAuth and remaining fault/large-history cases are deferred below.

## test/controllers/accounts/slack_import_runs_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| run pages are admin-only | deferred | WS16 continuation: implement and exercise the original behavior. |
| run list shows every run newest first | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting a dry run needs a connected account | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting a dry run is blocked while another run is active | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting a dry run creates a workspace dry run with the options | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting a dry run ignores unparseable dates | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting a dry run defaults to including private channels | deferred | WS16 continuation: implement and exercise the original behavior. |
| run page shows status, counts, timestamps, and issues with pagination | deferred | WS16 continuation: implement and exercise the original behavior. |
| run page polls while active and stops when finished | deferred | WS16 continuation: implement and exercise the original behavior. |
| status frame renders the run without polling itself | deferred | WS16 continuation: implement and exercise the original behavior. |
| status frame marks a finished run so polling stops | deferred | WS16 continuation: implement and exercise the original behavior. |
| run page shows queued-behind while another run is active | deferred | WS16 continuation: implement and exercise the original behavior. |
| administrators can view personal runs from the admin page | deferred | WS16 continuation: implement and exercise the original behavior. |
| plan renders conversations, targets, and samples | deferred | WS16 continuation: implement and exercise the original behavior. |
| plan escapes Slack text and sample markdown | deferred | WS16 continuation: implement and exercise the original behavior. |
| plan needs a completed dry run | deferred | WS16 continuation: implement and exercise the original behavior. |
| test import starts with checked conversations and a recent default | deferred | WS16 continuation: implement and exercise the original behavior. |
| test import date bounds are full timestamps covering their days | deferred | WS16 continuation: implement and exercise the original behavior. |
| full import starts with no date bounds | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting an import keeps only conversations in the dry run | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting an import with only unknown conversations is rejected | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting an import drops room targets outside alive open and closed rooms | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting an import with nothing checked is rejected | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting an import is blocked without a connection or with an active run | deferred | WS16 continuation: implement and exercise the original behavior. |
| catch-up repeats a full import's conversations and targets | deferred | WS16 continuation: implement and exercise the original behavior. |
| catch-up refuses a date-bounded test import | deferred | WS16 continuation: implement and exercise the original behavior. |
| catch-up needs a completed import | deferred | WS16 continuation: implement and exercise the original behavior. |
| only a completed full import offers catch-up | deferred | WS16 continuation: implement and exercise the original behavior. |
| cancel and undo act on workspace runs | deferred | WS16 continuation: implement and exercise the original behavior. |
| administrators may cancel and undo personal runs | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo is blocked with a reason while another run is active | deferred | WS16 continuation: controller behavior deferred; domain overlap/naming checks exist. |
| undo is blocked with a reason while a later import covers the same conversations | deferred | WS16 continuation: controller behavior deferred; domain overlap/naming checks exist. |
| cancel and undo refuse finished and non-undoable runs | deferred | WS16 continuation: implement and exercise the original behavior. |

## test/controllers/accounts/slack_imports_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| setup is admin-only | deferred | WS16 continuation: implement and exercise the original behavior. |
| setup shows the manifest with the callback URL and user scopes | deferred | WS16 continuation: implement and exercise the original behavior. |
| the client secret is never rendered back | deferred | WS16 continuation: implement and exercise the original behavior. |
| saving credentials requires sudo | deferred | WS16 continuation: implement and exercise the original behavior. |
| saving credentials creates the workspace and records who configured it | deferred | WS16 continuation: implement and exercise the original behavior. |
| a blank secret keeps the stored one | deferred | WS16 continuation: implement and exercise the original behavior. |
| invalid credentials re-render with errors | deferred | WS16 continuation: implement and exercise the original behavior. |
| removing credentials requires sudo | deferred | WS16 continuation: implement and exercise the original behavior. |
| removing credentials clears them and every connection but keeps runs | deferred | WS16 continuation: implement and exercise the original behavior. |
| removing credentials is blocked while a run is active | deferred | WS16 continuation: implement and exercise the original behavior. |

## test/controllers/slack/connections_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| disconnect revokes remotely and destroys the connection | deferred | WS16 continuation: implement and exercise the original behavior. |
| disconnect redirects a member to the personal page | deferred | WS16 continuation: implement and exercise the original behavior. |
| disconnect is blocked while one of the member's runs is active | deferred | WS16 continuation: implement and exercise the original behavior. |
| another member's active run does not block disconnect | deferred | WS16 continuation: implement and exercise the original behavior. |
| disconnect without a connection still redirects | deferred | WS16 continuation: implement and exercise the original behavior. |
| disconnect requires sudo | deferred | WS16 continuation: implement and exercise the original behavior. |

## test/controllers/slack/imports_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| personal page needs workspace setup first | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal page explains the scope and offers connect | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal page lists only the member's own runs | deferred | WS16 continuation: implement and exercise the original behavior. |
| members can view their own personal runs | deferred | WS16 continuation: implement and exercise the original behavior. |
| other members' runs 404 on the personal page | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal status frame renders the member's own run | deferred | WS16 continuation: implement and exercise the original behavior. |
| workspace runs 404 on the personal page | deferred | WS16 continuation: implement and exercise the original behavior. |
| administrators use the admin pages for other members' runs | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting a preview needs workspace setup and a connection | deferred | WS16 continuation: implement and exercise the original behavior. |
| starting a preview creates a personal dry run | deferred | WS16 continuation: implement and exercise the original behavior. |
| one active run per member | deferred | WS16 continuation: implement and exercise the original behavior. |
| another member's active run queues the preview behind it | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal import starts from the preview's checked conversations | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal import never passes room targets or date bounds | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal import needs a completed preview with checked conversations | deferred | WS16 continuation: implement and exercise the original behavior. |
| members can cancel and undo their own runs | deferred | WS16 continuation: implement and exercise the original behavior. |
| undo is blocked with a reason while another run is active | deferred | WS16 continuation: controller behavior deferred; domain overlap/naming checks exist. |
| undo is blocked with a reason while a later import covers the same conversations | deferred | WS16 continuation: controller behavior deferred; domain overlap/naming checks exist. |
| run page loads later runs' stats once across the undo checks | deferred | WS16 continuation: implement and exercise the original behavior. |
| personal run page shows the plan with skip checkboxes | deferred | WS16 continuation: implement and exercise the original behavior. |

## test/controllers/slack/oauth_controller_test.rb

| Rails test | State | Coverage or remaining work |
|---|---|---|
| start redirects to Slack with user scopes and no bot scope | deferred | WS16 continuation: implement and exercise the original behavior. |
| start pins the team once it is known | deferred | WS16 continuation: implement and exercise the original behavior. |
| start without configured credentials redirects back | deferred | WS16 continuation: implement and exercise the original behavior. |
| start requires sudo | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback success upserts the connection and sets the team | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback success without a team.info answer still connects | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback clears a previous disconnected reason | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback with a state mismatch is rejected | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback state cannot be replayed | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback state is bound to the user who started it | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback with access_denied stores nothing | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback from a different team is rejected | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback missing a required scope is rejected with the missing ones | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback refuses a Slack account linked to another member | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback rescues a duplicate connection raced in after the check | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback fills a missing team name from team.info | deferred | WS16 continuation: implement and exercise the original behavior. |
| a failed exchange returns to the page the flow started from | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback with a failed exchange stores nothing | deferred | WS16 continuation: implement and exercise the original behavior. |
| callback returns to the personal page when the flow started there | deferred | WS16 continuation: implement and exercise the original behavior. |
| an off-allowlist return_to falls back to the default page | deferred | WS16 continuation: implement and exercise the original behavior. |
| a member's first connection does not name the workspace team | deferred | WS16 continuation: implement and exercise the original behavior. |
| the code, token, and secret parameters are filtered from request logs | deferred | WS16 continuation: implement and exercise the original behavior. |

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
| undo is blocked while another run is queued | partial | Domain blocking and actual LIFO guarded; original controller response remains. |
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
| start! creates a queued run with normalized options and enqueues it | partial | Creation/enqueue covered; options normalization remains deferred. |
| start! defaults to including private channels | deferred | WS16 continuation: implement and exercise the original behavior. |
| start! rejects unparseable date bounds | deferred | WS16 continuation: implement and exercise the original behavior. |
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
| admin saves credentials, dry-runs, plans, and watches a test import | deferred | WS16 continuation: implement and exercise the original behavior. |

## Totals

95 covered, 10 partial, 139 deferred; 244 Rails tests inventoried.
