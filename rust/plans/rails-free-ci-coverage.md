# Rails-free CI: coverage audit and plan (P1b/P2)

Audited on origin/main 7b402893f against the reference pin 78b9b1546bdab4c6c1c9b8ddb94512f661289112.
Goal: `.github/workflows/rust.yml` runs no Ruby, Rails or reference image, without losing coverage.

## Finding that shapes the plan

No paired harness diffs Rails output against Rust output. behavior-check, WS11-UI, ledger,
`system_behavior.py` and `check_cutover_browser.py` all assert fixed expectations transcribed from
the Rails system tests, once per app. Rails reaches the Rust half only through its inputs:
Rails-built seeds, Rails-runner fixtures (`behavior-fixtures.rb`, `*_fixture.rb`), two
Rails-rendered values in the ledger lifecycle case, and the reference image the WS14/WS15
originals run their Ruby Capybara bodies in. So "port" means: run the Rust half alone, on inputs
recorded from Rails once, now.

## What was recorded from Rails (Stage A)

| artifact | from | size |
|---|---|---|
| `parity/seeds/frozen/{default,first_run,agents_ui,ledger_originals}` + `manifest.json` | `ci-seed prepare/image/build/validate` exactly as CI (Rails validation passed); `ledger_originals` via `parity/bin/seed build` | 4.4 MiB packed in git (13 storage files, shared by default and agents_ui) |
| `test-support/behavior-fixtures/KEY/{patch.sql,db/*,storage/*}` + `index.json` | every distinct `behavior-fixtures.rb` invocation (49 fixtures for 145 cases), sqldiff from the frozen default seed, round trip checked | 1.1 MiB |
| `test-support/agents-ui-fixtures/{pages,budget,work,inbox,inbox-filter}/{patch.sql,labels.json}` | `system_fixture.rb`, `budget_fixture.rb`, `work_fixture.rb`, `inbox_browser_fixture.rb` on a live reference booted from the agents_ui seed, round trip checked | 52 KiB |
| `test-support/ledger-lifecycle.json` | `ledger_browser_lifecycle.rb` setup on the ledger_originals seed: the Rails-rendered mention attachment and the encrypted `google_accounts` row | 4 KiB |
| `fixtures_ruby.sqlite3`, `scenario_ruby.sqlite3` (to serialize to JSON in Stage B) | `reference-tools/db/differential.sh --prepare-only` at `CAMPFIRE_FIXTURES_NOW='2026-03-02 16:00:00.123456'`; both ignored tests pass against them, `message_save_touches.json` and `schema.sql` match | 2 × 1.5 MB raw, ~100 KiB as JSON |

`parity/bin/frozen-seeds` checks (manifest hashes, the `SECRET_KEY_BASE`/VAPID digest, schema
versions against `crates/db/src/schema_migrations.txt`), restores to `parity/.seed/NAME`, and
migrates: `frozen-seeds migrate target/debug/campfire` runs P0's `campfire db-migrate DATABASE`
on a copy of each seed, checkpoints, and re-records the manifest. That's the hook for future
schema changes; until P0 lands, main's command still wants a MIGRATIONS_DIR argument.

The recorders (`reference-tools/messaging/record-fixtures.py`,
`reference-tools/views/agents_ui/record-fixtures.py`) are committed with the recordings for
provenance and deleted with the Ruby in P2.

## Counts

| group | tests | retire | port |
|---|---|---|---|
| WS14/WS15 messaging originals | 53 | 6 | 47 (44 browser, 3 HTTP) |
| behavior-check cases | 145 | 6 | 139 (Rust-only harness on recorded fixtures) |
| WS11-UI originals | 7 | 1 | 6 |
| Ledger browser | 4 | 0 | 4 (drop the `browser-profile` case) |
| agents-ui `system_behavior.py` scenarios | 3 | 0 | 3 |
| C221–C223 cutover | 3 | 0 | 3 (drop the Rails pass and "Rails must pass" gate) |
| Database differential | 3 | 1 | 2 (frozen snapshots) |
| **total** | **218** | **14** | **204** |

Already Rust-only, unchanged: C224–C227 (`compare` on `vectors/ws12_browser_remaining.json`),
the huddle gateway Node suite, `huddle_system_cases_in_real_browser`, LiveKit, ACME.

## WS14/WS15 originals (53)

Today each runs its Ruby Capybara body inside the reference image against Rust. Every
expectation is a literal in the Ruby, so the ports need no recording. Paths are under
`crates/campfire/src/controllers/` unless they start with `app/`.

| id | title (Rails source) | Rust-only coverage today | action |
|---|---|---|---|
| WS14e-101 | scheduling an event announces it in the room with a card members respond from (`events_test.rb:102`) | `rooms/events/tests/cutover/interactions.rs::cutover_interaction_announcement_card_response_stays_in_requested_frame` (forms, card, response, frame headers, DB). Only the browser's current path is unproven: plain Turbo frame behaviour | **Retire** |
| WS14g-224 | attach Drive files from the picker, send textless, and remove through edit (`drive_attachments_test.rb:10`) | Server only: `messages/drive_tests.rs::root_and_thread_drive_requests_match_rails_bytes_order_json_validation_and_rollback`, `app/cutover_c_tests.rs::cutover_c_drive_edit_form_has_two_removable_chips_and_exact_hidden_sentinels`. Missing: picker and chip JS, 24px target, textless send | Port: Rust-only Playwright suite A (Drive/sudo), fake Google client with `google_calendar_test_helper.rb` payloads |
| WS14g-225 | edit a room message in the composer and remove one of two attachments (`drive_attachments_test.rb:79`) | Server PATCH replace/clear rows in `drive_tests.rs`. Missing: menu → composer edit mode → chips | Port: Rust-only Playwright suite A (Drive/sudo), fake Google client with `google_calendar_test_helper.rb` payloads |
| WS14g-226 | attach a Drive file from the thread composer (`drive_attachments_test.rb:116`) | Thread rows in `drive_tests.rs`. Missing: thread composer picker JS | Port: Rust-only Playwright suite A (Drive/sudo), fake Google client with `google_calendar_test_helper.rb` payloads |
| WS14g-227 | a viewer with the Drive scope sees a picked file upgraded to a preview chip (`drive_link_previews_test.rb:10`) | `app/google_drive_tests.rs::google_drive_file_json_and_all_mime_kinds_match_rails`. Missing: chip render and upgrade (`drive_link_controller.js`) | Port: Rust-only Playwright suite A (Drive/sudo), fake Google client with `google_calendar_test_helper.rb` payloads |
| WS14g-228 | a viewer with the Drive scope keeps a plain chip for a file never picked (`drive_link_previews_test.rb:24`) | `google_drive_tests.rs::google_drive_transport_quota_and_forbidden_fail_with_rails_statuses`. Missing: plain-chip fallback (JS) | Port: Rust-only Playwright suite A (Drive/sudo), fake Google client with `google_calendar_test_helper.rb` payloads |
| WS14g-229 | a viewer without the Drive scope sees a plain chip and fetches nothing (`drive_link_previews_test.rb:36`) | Meta tag absent: `cutover_c_picker_without_drive_consent_omits_legacy_menu_even_with_calendar_grant`. Missing: plain chip, zero fetches | Port: Rust-only Playwright suite A (Drive/sudo), fake Google client with `google_calendar_test_helper.rb` payloads |
| WS14g-230 | composer Drive picker inserts the chosen file link at the caret (`drive_link_previews_test.rb:51`) | `google_drive_tests.rs::google_drive_lists_never_cache_trim_and_cap_terms_and_return_502_on_failure`. Missing: picker search, keyboard, caret insert | Port: Rust-only Playwright suite A (Drive/sudo), fake Google client with `google_calendar_test_helper.rb` payloads |
| WS14g-231 | composer omits the Drive menu item without the Drive scope (`drive_link_previews_test.rb:88`) | `app/cutover_c_tests.rs::cutover_c_picker_without_drive_consent_omits_legacy_menu_even_with_calendar_grant` checks no menu and no `aria-haspopup`; not that the plain button exists | Port: add one assertion to that test (exactly one `composer__attachment-btn` without `aria-haspopup`) |
| WS14g-232 | review dialog offers attach-only and an explicit grant with names and emails (`drive_share_test.rb:115`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim; keeps its mutation check |
| WS14g-233 | attach-only pins the chip and writes no Drive permissions (`drive_share_test.rb:150`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim; keeps its mutation check |
| WS14g-234 | grant validates, preserves writers, and creates only missing readers (`drive_share_test.rb:178`) | Server half (recipient list, membership revalidation): `app/google_drive_tests.rs::google_drive_recipients_require_human_membership_but_no_google_grant`; the dialog flow and Drive writes are client JS | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-235 | partial failure reports per recipient and retries only outstanding grants (`drive_share_test.rb:220`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-236 | cancelled picker selection shares nothing and can be retried (`drive_share_test.rb:256`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-237 | cancelled Google authorization shares nothing and can be retried (`drive_share_test.rb:273`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-238 | picker cancel returns quietly and the drive button works again (`drive_share_test.rb:289`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim; keeps its mutation check |
| WS14g-239 | closed consent popup returns quietly to the composer (`drive_share_test.rb:307`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-240 | consent popup closed via GIS error callback returns quietly (`drive_share_test.rb:324`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-241 | real error dialog closes with the Close button and the drive button works again (`drive_share_test.rb:340`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-242 | real error dialog closes with Esc (`drive_share_test.rb:362`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-243 | try again re-opens the picker after a real error (`drive_share_test.rb:377`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-244 | script load failure offers retry without hanging the composer (`drive_share_test.rb:396`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-245 | first use loads scripts then continues on a fresh gesture (`drive_share_test.rb:412`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-246 | unshareable file disables the grant but keeps attach-only (`drive_share_test.rb:426`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-247 | folder selection disables the grant but keeps attach-only (`drive_share_test.rb:443`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-248 | expired Google session reconnects on an explicit gesture and continues (`drive_share_test.rb:461`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-249 | grant rejects a recipient who left mid-review and refreshes the list (`drive_share_test.rb:484`) | Server half (recipient list, membership revalidation): `app/google_drive_tests.rs::google_drive_recipients_require_human_membership_but_no_google_grant`; the dialog flow and Drive writes are client JS | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-250 | thread composer grants against the parent room membership (`drive_share_test.rb:508`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-251 | navigation disposes the dialog, token, and picker state (`drive_share_test.rb:542`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-252 | mobile viewport keeps the review dialog usable (`drive_share_test.rb:563`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim; keeps its mutation check |
| WS14g-253 | grant requires fresh review when a recipient email changes (`drive_share_test.rb:594`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-254 | changed identity can be re-approved against the new email (`drive_share_test.rb:617`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-255 | retry revalidates membership before writing (`drive_share_test.rb:634`) | Server half (recipient list, membership revalidation): `app/google_drive_tests.rs::google_drive_recipients_require_human_membership_but_no_google_grant`; the dialog flow and Drive writes are client JS | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-256 | reconnect revalidates before resuming grants (`drive_share_test.rb:663`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-257 | cancelled authorization invalidates a delayed token callback (`drive_share_test.rb:687`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-258 | cancelled picker invalidates a delayed selection callback (`drive_share_test.rb:716`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-259 | grant is blocked when attachments are already full (`drive_share_test.rb:735`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-260 | mid-flight capacity loss preserves grant outcomes (`drive_share_test.rb:770`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-261 | rate-limited grants are classified and retry cleanly (`drive_share_test.rb:796`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-262 | dialog checkboxes are visible and long names wrap (`drive_share_test.rb:822`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim; keeps its mutation check |
| WS14g-263 | retry does not write while attachment capacity is unavailable (`drive_share_test.rb:854`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-264 | completed outcomes stay visible through reconnect and refreshed review (`drive_share_test.rb:895`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-265 | reconciled access is confirmed, not claimed as newly granted (`drive_share_test.rb:928`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-266 | existing and reconciled access are distinguished with no new grants (`drive_share_test.rb:958`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-267 | server errors are classified as service failures, not policy denials (`drive_share_test.rb:986`) | None outside the browser: `drive_share_controller.js` client behaviour (dialog, GIS/picker mocks, retries, classification) | Port: Rust-only Playwright suite B (`parity/system/drive-share.test.mjs`), `MOCK_JS` copied verbatim |
| WS14g-269 | opting in shows In a meeting for a stubbed busy interval, then clears after it ends (`meeting_status_test.rb:7`) | Pieces: `users/statuses/tests.rs::ws17_opting_into_meeting_status_enqueues_a_first_refresh`, `app/google_meeting_refresh_tests.rs::google_meeting_refresh_complete_states_and_requests_match_pinned_rails`, `users/statuses/calendar_tests.rs::ws17_meeting_{start,end}_broadcasts_badge`. Missing: `/users/:id` badge before and after +6 min | Port: Rust HTTP test (fake Google client with the helper's busy + transparent events, frozen clock) |
| WS14g-270 | the profile links to connect without a Google account (`meeting_status_test.rb:34`) | `users/profile_sections_tests.rs::configured_calendar_profile_uses_real_account_metadata_and_forms` (same link selector, zero meeting switches) | **Retire** |
| WS14g-271 | set OOO until tomorrow, badge and DM notice show for another user, then clear it (`out_of_office_test.rb:4`) | `users/statuses/tests.rs::ws17_sets_out_of_office_with_a_preset_and_a_note_and_broadcasts_it`, `ws17_clears_out_of_office_early…`, `rooms/ws17_ooo_tests.rs::ws17_dm_ooo_shows_notice_above_composer`. Missing: `/users/:id` OOO badge, set then cleared | Port: Rust HTTP test |
| WS14g-272 | one prompt, then the action continues automatically (`sudo_mode_test.rb:8`) | `app/sudo_tests.rs::sudo_gate_and_replay_use_a_new_valid_csrf_token_without_executing_early` (replay form, `data-controller="auto-submit"`). Missing: the JS auto-submit firing | Port: Rust-only Playwright suite A (Drive/sudo), fake Google client with `google_calendar_test_helper.rb` payloads |
| WS14g-273 | a wrong password keeps the action gated (`sudo_mode_test.rb:27`) | `app/sudo_tests.rs::sudo_password_prompt_and_rejection_statuses` (401, "Confirmation failed. Try again.", audit row, join code still gated) | **Retire** |
| WS15g-062 | a linked member comments from a PR thread and sees the inline confirmation (`github_pr_write_actions_test.rb:6`) | `github/write_tests.rs::github_write_http_results_payloads_own_token_prompts_retry_text_and_streams_match_rails` (`comments_success`, exact GitHub request, notice, cleared field); Discuss→panel is behaviour-check `threads` "discusses a pull request from its card" (ported) | **Retire** |
| WS15g-063 | a linked member requests a review from a PR thread and sees the inline confirmation (`github_pr_write_actions_test.rb:52`) | Same test (`review_requests_success`), plus `github_review_logins_normalization_matches_rails_odd_shapes_and_boundaries` | **Retire** |
| WS15g-064 | a member without a linked token sees the connect prompt in the thread (`github_pr_write_actions_test.rb:98`) | Same test, case `show_unlinked` (Connect GitHub present, comment form absent) | **Retire** |

## behavior-check.py (145 cases, 17 files)

Each case asserts fixed expectations against the app under test. Today the Rust half is fed a
fixture DB that `behavior-fixtures.rb` built in Rails, pinned test sources via `git show PIN:`,
and media tooling from the Rails image; six cases run Ruby Capybara natively. Port: one Rust-only
harness that applies `test-support/behavior-fixtures/KEY/patch.sql` (via `index.json`) to the
frozen default seed, vendors the four pinned test sources it reads, takes media libraries from
the Rust image, rewrites the six native cases in Playwright, and drops the Selenium atom check
that runs inside the Rails image. Rails is about 75–85% of this suite's runtime today.

| file | cases | fixture(s) | Rust-only coverage outside the browser | action |
|---|---|---|---|---|
| drive_attachments | 3 | `drive--*` | Same titles as WS14g-224/225/226 | **Retire 3** (ported once, as originals) |
| channel_threads_controller | 8 | `work-controller--*` | :331, :433, :496 fully covered (`ws12_browser_remaining` c229/c230 HTTP, `ws12_work_remaining_tests.rs::ws12_ordinary_work_owner_options_batch_agent_profiles_at_two_sizes`); :304, :356, :375, :413, :480 at model level only (`work_mutations_test.rs`, `agent_assignment_cases_test.rs`) | **Retire 3**, port 5 on recorded fixtures |
| motion | 9 | `motion--*` | `views/tests/core.rs::application_test_environment_matches_rails_motion_attribute` (attribute only) | Port 9; "mobile drawer animates in…" moves from native Capybara to Playwright |
| mobile_layout | 5 | `mobile-layout` | Markup goldens only | Port 5 |
| sending_messages | 4 | seed, `workspace-upload` | Message create/edit/destroy HTTP tests | Port 4; "uploading a fresh video…" to Playwright |
| workspace_markdown | 9 | seed, `workspace-upload` | Markdown/sanitizer unit tests | Port 9; "Markdown replies and file attachments…" and "late upload progress…" to Playwright |
| threads | 15 | seed, `thread-anchor`, `thread-anchor-race`, `thread-pr` | Thread HTTP tests | Port 15; "keeps the thread drawer usable on a phone…" to Playwright |
| message_list_a11y | 29 | `message_list`, `message_destinations`, `history`, `board-touch` | None (focus/ARIA/live-region JS) | Port 29 |
| search_forward_edit | 3 | `search`, `forward`, `edit-card` | Search/forward HTTP tests | Port 3 |
| unread_divider | 5 | `unread-{few,many,pill,offpage,menu}` | `rooms/room_shell_tests.rs::room_shell_unread_pointer_matches_count_threshold_deleted_cursor_and_off_page_jump` (server half) | Port 5 |
| composer | 11 | `composer-drafts`, `composer-typing` | Autocomplete endpoints only | Port 11 |
| composer_attach_menu | 9 | `attach-menu`, `attach-share`, `drive--*` | `app/cutover_c_tests.rs` picker markup | Port 9 |
| boosting_messages | 4 | `boosts` | Boost HTTP tests | Port 4 |
| message_interactions | 10 | `interactions` | Message actions endpoints | Port 10; "a release click landing on the just-opened menu…" to Playwright |
| message_actions_mobile | 2 | `actions-mobile` | None | Port 2 |
| message_toolbar | 13 | `toolbar` | Boost endpoints only | Port 13 |
| code_highlighting | 6 | `highlight`, `highlight-thread`, `highlight-search` | Richtext fence rendering | Port 6 (vendors `code_highlighting_test.rb`, `application_system_test_case.rb`) |

## WS11-UI originals (7) and ledger browser (4)

Both runners (`run_original_browser_assertions.py`, `run_ledger_browser_assertions.py`) assert
fixed expectations; strip `reference up/down`, the Rails target loop, the oracle env and
`PARITY_IMAGE`. Server halves are covered by ordinary Rust tests; the client halves (Stimulus,
focus, geometry, CSP in a real browser) exist only here.

| test | behaviours | Rust-only coverage today | action |
|---|---|---|---|
| WS11 `people` | profile card open/focus, multi-select DM and huddle, long-press, bar copy | `users/people_tests.rs`, `rooms/directs_rails_cases.rs` (server) | Port: Rust-only run |
| WS11 `pickers` | DM picker filter, Enter, toggles, phone geometry | `rooms/directs/picker_tests.rs` (server) | Port |
| WS11 `members` | phone member card Escape/Tab focus | member panel markup goldens | Port |
| WS11 `group` | create/rename/add/leave group DM through the UI | `directs_rails_cases.rs` (near-full server) | Port |
| WS11 `tours` | 5-step tour, completion, restart | `users/preferences_tests.rs` (stamp only) | Port |
| WS11 `stars` | star/unstar regrouping, row context menu | `users/stars_tests.rs` | Port |
| WS11 `worker` | served service worker passes the Node event harness | `pwa.rs::original_service_worker_logic_checks_the_real_http_script` (same harness, full) | **Retire** |
| Ledger `navigation` (49 cases) | keyboard, sidebar menus, Turbo nav, header layout, unread | `rooms/original_audit_tests.rs`, `rooms/reads_rails_cases.rs`, `rooms/sidebars_rails_cases.rs` (server) | Port on the frozen `ledger_originals` seed |
| Ledger `members` (31) | multi-select, presence leases, motion/geometry | `channels/tests/reference_test.rs::workspace_presence_*`, `rooms/members_rails_cases.rs` (server) | Port |
| Ledger `surfaces` (25) | audit, icons, drawer, ringing, worker, timezone, unread, CSP in a real browser | `accounts/audit_logs/tests.rs`, `app/security_tests.rs`, `pwa.rs` (server) | Port; drop `browser-profile` (Chromedriver profile, no app) |
| Ledger `lifecycle` (2) | mute delivery; calendar meeting badge before/after +6 min | `db/tests/room_test.rs::muted_members_go_unread_only_when_mentioned`, `app/google_meeting_refresh_tests.rs` | Port with `test-support/ledger-lifecycle.json` (mention + `google_accounts` row) |

## agents-ui and C221–C223

`system_behavior.py` copied a backup of the Rails instance after a fixture script into the Rust
DB, merged the fixture's stdout into the labels, then ran `system_cases.mjs` against both apps.
Port: apply `test-support/agents-ui-fixtures/S/patch.sql` to the frozen agents_ui seed, merge
`labels.json`, run the Rust pass only. `check_cutover_browser.py` keeps the Rust baseline and
the writer-defect controls (trigger SQL and expected FAIL strings already live in Python) and
loses the "Rails must pass" gate.

| test | behaviours | Rust-only coverage today | action |
|---|---|---|---|
| agents-ui `pages` | directory, profile, approval from inbox, "Steps (2)", working presence, live streaming, sudo replay | `presenters/accounts/tests/approval_decisions.rs`, `agent_conversation_tests.rs::agent_conversation_stream_bytes`, `app/sudo_tests.rs` (server) | Port on `agents-ui-fixtures/pages` |
| agents-ui `budget` | "0/1 messages · 0/10 board posts", 201→429→429, one notice, kill switch | `messages/tests.rs::ws11_bot_key_message_budget_overflow_notifies_once`, `kill_switch.rs` | Port on `budget` |
| agents-ui `work` | track as work, agent owner, Bearer PATCH to in_progress, history | `agent_work_writes_tests.rs`, `ws12_work_remaining_tests.rs` | Port on `work` |
| C221 `inbox` | mention item, badge hide on handled, live re-show; control on `handled_at` | `activity_items/tests.rs::ws11ui_inbox_http_matches_pinned_rails_bytes_and_permissions` | Port on `inbox`, keep the control |
| C222 `inbox-filter` | events filter, `event_reminders` saved false; control | `ws12_inbox_remaining_tests.rs` c016–c018 | Port on `inbox-filter`, keep the control |
| C223 `work` | as `work`, with control on status | as above | Port on `work`, keep the control |

## Database differential (3)

`correctness.sh database` only runs `differential.sh --prepare-only`, so the save-touches,
schema-identity and Rails-reads-Rust checks never ran in CI.

| test | behaviours | Rust-only coverage today | action |
|---|---|---|---|
| `fixtures_test.rs::fixtures_match_ruby_row_for_row` | every fixture table row for row vs Rails `db:fixtures:load` (label CRC ids, ERB times, defaults, FTS shadow tables, `sqlite_sequence`, bcrypt) | spot checks only | **Port**: serialize the recorded `fixtures_ruby.sqlite3` to `crates/db/src/tests/fixtures_rails_rows.json`, compare non-ignored at the fixed NOW, `CAMPFIRE_FIXTURES_DUMP=write` regenerates from Rust |
| `differential_test.rs::scenario_matches_ruby` | `scenario.rb` steps, 13 tables normalized | per-operation tests | **Port**: same, `scenario_rails_rows.json` |
| `two_factor_rollback_test.rs::read_rails_rollback_changes` | Rust reads a DB Rails changed (TOTP stamp, re-encrypted secret, backup code, remember cookie) | `two_factor_test.rs` on `vectors/two_factor.json` ciphertexts; `rails_compat/src/totp.rs` remember cookie | **Retire** (rollback to Rails is ruled out; the Rails-written values are already frozen) |

## Drift tests (read Rails sources at test time)

| test | guards | plan |
|---|---|---|
| `schema.rs` `schema_sha1_matches_reference_schema_rb` | `schema_sha1.txt` = SHA1 of `db/schema.rb` | **Delete**; covered by the hash lock below |
| `schema.rs` `migration_versions_match_reference_migrations` | version list = `db/migrate` | **Freeze**: `baseline_is_frozen` checks 129 unique versions (20231215043540…20261003180000) and sha256 of the baseline files (`schema.sql`, `schema_migrations.txt`, `schema_sequences.txt`, `schema_sha1.txt`) against a committed `SHA256SUMS` |
| `schema.rs` `schema_sql_is_the_whole_reference_schema` | every Rails table exists | **Trim**: drop the `schema.rb` loop, keep the FTS/metadata/partial-index assertions; completeness is `a_prepared_database_has_exactly_schema_sql` plus the hash lock |
| `models/sound.rs` `builtin_sounds_match_reference` | `BUILTIN` = `app/models/sound.rb` | **Freeze**: `BUILTIN` is the source of truth; keep `deeper` → `sounds/top.webp`, assert every name has an mp3 under `rust/web` |
| `pwa.rs` `ws17_service_worker_is_served_byte_identical_to_rails` | served bytes = Rails files | **Trim**: drop the `app/views/pwa/service_worker.js` read; `pwa_http_bodies_match_rails_before_and_after_first_run` already compares against `vectors/users_pwa_*.json` (verified equal to Rails) |
| `rich_text.rs` `vendored_icon_catalog_matches_reference` + 3 helper tests | vendored `icons.yml` = `config/icons.yml` (identical today) | **Delete** all four and `check_icon_reference`; the vendored file is the source |
| `views/tests/template_coverage.rs` (two tests) | every Rails template has a disposition | **Freeze**: the 284 `rails_templates` keys in `parity/template-coverage.json` are the list; drop the `app/views` directory-equality pair and the `is_file` check for two sources outside `rust/` |
| `ws17_review_test.rs:120` `include_str!` of `ws17_notification_push.rb` | the generator didn't paper over Rails | **Delete** the lines, with the `.rb` |
| `bounded_provider_tests.rs:20` `include_str!` of `older_provider_callbacks.rb` | no literal Bearer header in fixture sources | **Drop** the `.rb` entry, keep the Rust one |
| `slack/client/tests.rs` vendored fixtures = `test/fixtures/files/slack` | | Unchanged after P1a (two Rust-owned copies); dedupe optional |

After these, `rails_root()` and the `CAMPFIRE_REFERENCE` fallbacks have no callers left in tests.

## Deletion manifest (P2)

`rust/reference-tools`: 1,247 of 1,408 tracked files have no Rust, CI or vector reader and go,
plus `crates/db/ruby/*.rb` (19). `db/` goes after P0 merges (P0 edits it). Kept: the non-Ruby
inputs Rust tests read (`attachments/fixtures`, the post-pin people.css and
profile_card_controller.js, the service-worker harness, the agents-ui `*_inputs.json(.gz)`, the
port-lease helpers, `huddle_gateway_node.mjs`) and the Node/Python drivers of the ported
suites. The 31 Ruby files those suites still run go when each suite is ported.
