# WS14 / WS15 cutover assertions still open

**Partial slice: 144 exact declarations remain without a discriminating acceptance receipt.** These are not declared absent production behavior, and are not waived because they have another owner. They are still in the cutover gate.

The three inventories retain historical receipts and current per-record dispositions. This list is generated from `ledger-ws14-ws15.json`. Continuation batches and Rails/native execution receipts are recorded in `ledger-ws14-ws15-b-report.md`.

| Record | Current ledger row | Rails declaration | Assertion still required |
|---|---|---|---|
| WS14e-051 | `rust/plans/ws14e-test-inventory.md:174` | `test/models/event/reminder_pusher_test.rb:32` | Reopened after PR #239 review: execute the real Event::ReminderPushJob with a persisted voice venue named Lounge and assert the queued push body includes the suffix in Lounge. The cited pusher corpus never sets venue_room_id; source-only payload comparisons do not prove production job/transport assembly. |
| WS14e-057 | `rust/plans/ws14e-test-inventory.md:180` | `test/models/event/reminder_pusher_test.rb:115` | Reopened after PR #239 review: execute the real Event::ReminderPushJob for an event started within five minutes but ended one minute ago and assert no push is queued, with a still-running control. The cited corpus has no ends_at input; start-age staleness cannot establish end-time suppression. |
| WS14g-004 | `rust/plans/ws14g-rails-test-map.md:148` | `test/controllers/messages_drive_attachments_test.rb:221` | viewers with and without Drive consent receive identical attachment markup |
| WS14g-005 | `rust/plans/ws14g-rails-test-map.md:149` | `test/controllers/messages_drive_attachments_test.rb:239` | edit form lists attachments as removable chips with the blank sentinel |
| WS14g-007 | `rust/plans/ws14g-rails-test-map.md:265` | `test/controllers/sudos_controller_test.rb:29` | confirming with the password verifies and audit-logs |
| WS14g-011 | `rust/plans/ws14g-rails-test-map.md:269` | `test/controllers/sudos_controller_test.rb:67` | register_verifier adds a verifier |
| WS14g-015 | `rust/plans/ws14g-rails-test-map.md:273` | `test/controllers/sudos_controller_test.rb:109` | confirming with the password still works for enrolled users |
| WS14g-022 | `rust/plans/ws14g-rails-test-map.md:280` | `test/controllers/sudos_controller_test.rb:210` | browsing the audit log needs no confirmation |
| WS14g-025 | `rust/plans/ws14g-rails-test-map.md:283` | `test/controllers/sudos_controller_test.rb:238` | signing in again starts unverified |
| WS14g-027 | `rust/plans/ws14g-rails-test-map.md:285` | `test/controllers/sudos_controller_test.rb:264` | the replay form rebuilds nested params |
| WS14g-034 | `rust/plans/ws14g-rails-test-map.md:298` | `test/controllers/sudos_controller_test.rb:449` | the confirmation limit lives in the shared rate-limit store, not per-process memory |
| WS14g-035 | `rust/plans/ws14g-rails-test-map.md:313` | `test/integration/drive_picker_test.rb:11` | composer omits the Drive menu item without Drive consent |
| WS14g-036 | `rust/plans/ws14g-rails-test-map.md:314` | `test/integration/drive_picker_test.rb:30` | composer carries the Drive menu item with the Drive scope |
| WS14g-037 | `rust/plans/ws14g-rails-test-map.md:318` | `test/integration/drive_share_picker_test.rb:16` | composer carries a single enhanced Drive menu item when sharing is configured |
| WS14g-038 | `rust/plans/ws14g-rails-test-map.md:319` | `test/integration/drive_share_picker_test.rb:32` | enhanced menu item needs no Drive consent and wins over the legacy picker |
| WS14g-039 | `rust/plans/ws14g-rails-test-map.md:320` | `test/integration/drive_share_picker_test.rb:44` | composer falls back to the legacy picker when sharing is not configured |
| WS14g-040 | `rust/plans/ws14g-rails-test-map.md:321` | `test/integration/drive_share_picker_test.rb:55` | composer omits every Drive menu item without sharing or Drive consent |
| WS14g-041 | `rust/plans/ws14g-rails-test-map.md:322` | `test/integration/drive_share_picker_test.rb:65` | signed-out visitors see no share metas or buttons |
| WS14g-053 | `rust/plans/ws14g-rails-test-map.md:340` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:27` | refreshes an expired access token from the snapshot |
| WS14g-054 | `rust/plans/ws14g-rails-test-map.md:341` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:44` | an invalid_grant skips deletes but still revokes |
| WS14g-056 | `rust/plans/ws14g-rails-test-map.md:343` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:73` | a refresh 503 retries without revoking until the deletes succeed |
| WS14g-058 | `rust/plans/ws14g-rails-test-map.md:345` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:113` | a revoke 5xx schedules a retry |
| WS14g-059 | `rust/plans/ws14g-rails-test-map.md:346` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:128` | an exhausted retry logs the failure at error level |
| WS14g-062 | `rust/plans/ws14g-rails-test-map.md:350` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:197` | an unreadable credentials blob logs a warning with the account id |
| WS14g-063 | `rust/plans/ws14g-rails-test-map.md:351` | `test/jobs/calendar/disconnect_cleanup_job_test.rb:218` | tokens never reach the job logs |
| WS14g-066 | `rust/plans/ws14g-rails-test-map.md:379` | `test/jobs/calendar/remote_delete_job_test.rb:10` | deletes the remote copy with the user's current credentials |
| WS14g-067 | `rust/plans/ws14g-rails-test-map.md:380` | `test/jobs/calendar/remote_delete_job_test.rb:19` | skips a missing account without a request |
| WS14g-068 | `rust/plans/ws14g-rails-test-map.md:381` | `test/jobs/calendar/remote_delete_job_test.rb:25` | skips a disconnected account without a request |
| WS14g-069 | `rust/plans/ws14g-rails-test-map.md:382` | `test/jobs/calendar/remote_delete_job_test.rb:33` | skips a grant without the calendar scope |
| WS14g-070 | `rust/plans/ws14g-rails-test-map.md:383` | `test/jobs/calendar/remote_delete_job_test.rb:41` | treats gone and revoked copies as success |
| WS14g-071 | `rust/plans/ws14g-rails-test-map.md:384` | `test/jobs/calendar/remote_delete_job_test.rb:53` | an invalid_grant disconnects and counts as success |
| WS14g-072 | `rust/plans/ws14g-rails-test-map.md:385` | `test/jobs/calendar/remote_delete_job_test.rb:64` | a transient failure schedules a retry |
| WS14g-073 | `rust/plans/ws14g-rails-test-map.md:386` | `test/jobs/calendar/remote_delete_job_test.rb:73` | other Google failures are logged without raising or retrying |
| WS14g-074 | `rust/plans/ws14g-rails-test-map.md:422` | `test/jobs/calendar/sync_entry_job_test.rb:444` | responding going on the first event of a series syncs every occurrence |
| WS14g-077 | `rust/plans/ws14g-rails-test-map.md:425` | `test/jobs/calendar/sync_entry_job_test.rb:484` | updating event times enqueues syncs for connected going/maybe attendees |
| WS14g-078 | `rust/plans/ws14g-rails-test-map.md:426` | `test/jobs/calendar/sync_entry_job_test.rb:497` | an update inside a transaction enqueues only after commit |
| WS14g-079 | `rust/plans/ws14g-rails-test-map.md:427` | `test/jobs/calendar/sync_entry_job_test.rb:508` | setting or clearing the venue enqueues a sync, like a title change |
| WS14g-080 | `rust/plans/ws14g-rails-test-map.md:428` | `test/jobs/calendar/sync_entry_job_test.rb:521` | updating only the title enqueues a sync, an unchanged save does not |
| WS14g-083 | `rust/plans/ws14g-rails-test-map.md:431` | `test/jobs/calendar/sync_entry_job_test.rb:552` | destroying a room enqueues remote deletes for its entries |
| WS14g-084 | `rust/plans/ws14g-rails-test-map.md:432` | `test/jobs/calendar/sync_entry_job_test.rb:566` | shrinking a series enqueues remote deletes for destroyed occurrences |
| WS14g-121 | `rust/plans/ws14g-rails-test-map.md:543` | `test/models/drive_attachment_test.rb:9` | a message with attachments and no text is valid |
| WS14g-122 | `rust/plans/ws14g-rails-test-map.md:544` | `test/models/drive_attachment_test.rb:18` | a textless message without attachments is still invalid |
| WS14g-123 | `rust/plans/ws14g-rails-test-map.md:545` | `test/models/drive_attachment_test.rb:25` | an invalid file id is rejected |
| WS14g-124 | `rust/plans/ws14g-rails-test-map.md:546` | `test/models/drive_attachment_test.rb:37` | duplicate file ids on one message collapse to a single row |
| WS14g-125 | `rust/plans/ws14g-rails-test-map.md:547` | `test/models/drive_attachment_test.rb:47` | the same file id may attach to different messages |
| WS14g-126 | `rust/plans/ws14g-rails-test-map.md:548` | `test/models/drive_attachment_test.rb:58` | the 11th attachment is rejected |
| WS14g-127 | `rust/plans/ws14g-rails-test-map.md:549` | `test/models/drive_attachment_test.rb:70` | destroying the message destroys its attachments |
| WS14g-128 | `rust/plans/ws14g-rails-test-map.md:550` | `test/models/drive_attachment_test.rb:79` | url is the open link for the file id |
| WS14g-184 | `rust/plans/ws14g-rails-test-map.md:658` | `test/models/google_account_test.rb:44` | connected, usable, and expiry predicates |
| WS14g-185 | `rust/plans/ws14g-rails-test-map.md:659` | `test/models/google_account_test.rb:61` | calendar? treats blank scopes as granted and requires calendar.events otherwise |
| WS14g-224 | `rust/plans/ws14g-rails-test-map.md:707` | `test/system/drive_attachments_test.rb:10` | attach Drive files from the picker, send textless, and remove through edit |
| WS14g-225 | `rust/plans/ws14g-rails-test-map.md:708` | `test/system/drive_attachments_test.rb:79` | edit a room message in the composer and remove one of two attachments |
| WS14g-226 | `rust/plans/ws14g-rails-test-map.md:709` | `test/system/drive_attachments_test.rb:116` | attach a Drive file from the thread composer |
| WS14g-227 | `rust/plans/ws14g-rails-test-map.md:713` | `test/system/drive_link_previews_test.rb:10` | a viewer with the Drive scope sees a picked file upgraded to a preview chip |
| WS14g-228 | `rust/plans/ws14g-rails-test-map.md:714` | `test/system/drive_link_previews_test.rb:24` | a viewer with the Drive scope keeps a plain chip for a file never picked |
| WS14g-229 | `rust/plans/ws14g-rails-test-map.md:715` | `test/system/drive_link_previews_test.rb:36` | a viewer without the Drive scope sees a plain chip and fetches nothing |
| WS14g-230 | `rust/plans/ws14g-rails-test-map.md:716` | `test/system/drive_link_previews_test.rb:51` | composer Drive picker inserts the chosen file link at the caret |
| WS14g-231 | `rust/plans/ws14g-rails-test-map.md:717` | `test/system/drive_link_previews_test.rb:88` | composer omits the Drive menu item without the Drive scope |
| WS14g-232 | `rust/plans/ws14g-rails-test-map.md:721` | `test/system/drive_share_test.rb:115` | review dialog offers attach-only and an explicit grant with names and emails |
| WS14g-233 | `rust/plans/ws14g-rails-test-map.md:722` | `test/system/drive_share_test.rb:150` | attach-only pins the chip and writes no Drive permissions |
| WS14g-234 | `rust/plans/ws14g-rails-test-map.md:723` | `test/system/drive_share_test.rb:178` | grant validates, preserves writers, and creates only missing readers |
| WS14g-235 | `rust/plans/ws14g-rails-test-map.md:724` | `test/system/drive_share_test.rb:220` | partial failure reports per recipient and retries only outstanding grants |
| WS14g-236 | `rust/plans/ws14g-rails-test-map.md:725` | `test/system/drive_share_test.rb:256` | cancelled picker selection shares nothing and can be retried |
| WS14g-237 | `rust/plans/ws14g-rails-test-map.md:726` | `test/system/drive_share_test.rb:273` | cancelled Google authorization shares nothing and can be retried |
| WS14g-238 | `rust/plans/ws14g-rails-test-map.md:727` | `test/system/drive_share_test.rb:289` | picker cancel returns quietly and the drive button works again |
| WS14g-239 | `rust/plans/ws14g-rails-test-map.md:728` | `test/system/drive_share_test.rb:307` | closed consent popup returns quietly to the composer |
| WS14g-240 | `rust/plans/ws14g-rails-test-map.md:729` | `test/system/drive_share_test.rb:324` | consent popup closed via GIS error callback returns quietly |
| WS14g-241 | `rust/plans/ws14g-rails-test-map.md:730` | `test/system/drive_share_test.rb:340` | real error dialog closes with the Close button and the drive button works again |
| WS14g-242 | `rust/plans/ws14g-rails-test-map.md:731` | `test/system/drive_share_test.rb:362` | real error dialog closes with Esc |
| WS14g-243 | `rust/plans/ws14g-rails-test-map.md:732` | `test/system/drive_share_test.rb:377` | try again re-opens the picker after a real error |
| WS14g-244 | `rust/plans/ws14g-rails-test-map.md:733` | `test/system/drive_share_test.rb:396` | script load failure offers retry without hanging the composer |
| WS14g-245 | `rust/plans/ws14g-rails-test-map.md:734` | `test/system/drive_share_test.rb:412` | first use loads scripts then continues on a fresh gesture |
| WS14g-246 | `rust/plans/ws14g-rails-test-map.md:735` | `test/system/drive_share_test.rb:426` | unshareable file disables the grant but keeps attach-only |
| WS14g-247 | `rust/plans/ws14g-rails-test-map.md:736` | `test/system/drive_share_test.rb:443` | folder selection disables the grant but keeps attach-only |
| WS14g-248 | `rust/plans/ws14g-rails-test-map.md:737` | `test/system/drive_share_test.rb:461` | expired Google session reconnects on an explicit gesture and continues |
| WS14g-249 | `rust/plans/ws14g-rails-test-map.md:738` | `test/system/drive_share_test.rb:484` | grant rejects a recipient who left mid-review and refreshes the list |
| WS14g-250 | `rust/plans/ws14g-rails-test-map.md:739` | `test/system/drive_share_test.rb:508` | thread composer grants against the parent room membership |
| WS14g-251 | `rust/plans/ws14g-rails-test-map.md:740` | `test/system/drive_share_test.rb:542` | navigation disposes the dialog, token, and picker state |
| WS14g-252 | `rust/plans/ws14g-rails-test-map.md:741` | `test/system/drive_share_test.rb:563` | mobile viewport keeps the review dialog usable |
| WS14g-253 | `rust/plans/ws14g-rails-test-map.md:742` | `test/system/drive_share_test.rb:594` | grant requires fresh review when a recipient email changes |
| WS14g-254 | `rust/plans/ws14g-rails-test-map.md:743` | `test/system/drive_share_test.rb:617` | changed identity can be re-approved against the new email |
| WS14g-255 | `rust/plans/ws14g-rails-test-map.md:744` | `test/system/drive_share_test.rb:634` | retry revalidates membership before writing |
| WS14g-256 | `rust/plans/ws14g-rails-test-map.md:745` | `test/system/drive_share_test.rb:663` | reconnect revalidates before resuming grants |
| WS14g-257 | `rust/plans/ws14g-rails-test-map.md:746` | `test/system/drive_share_test.rb:687` | cancelled authorization invalidates a delayed token callback |
| WS14g-258 | `rust/plans/ws14g-rails-test-map.md:747` | `test/system/drive_share_test.rb:716` | cancelled picker invalidates a delayed selection callback |
| WS14g-259 | `rust/plans/ws14g-rails-test-map.md:748` | `test/system/drive_share_test.rb:735` | grant is blocked when attachments are already full |
| WS14g-260 | `rust/plans/ws14g-rails-test-map.md:749` | `test/system/drive_share_test.rb:770` | mid-flight capacity loss preserves grant outcomes |
| WS14g-261 | `rust/plans/ws14g-rails-test-map.md:750` | `test/system/drive_share_test.rb:796` | rate-limited grants are classified and retry cleanly |
| WS14g-262 | `rust/plans/ws14g-rails-test-map.md:751` | `test/system/drive_share_test.rb:822` | dialog checkboxes are visible and long names wrap |
| WS14g-263 | `rust/plans/ws14g-rails-test-map.md:752` | `test/system/drive_share_test.rb:854` | retry does not write while attachment capacity is unavailable |
| WS14g-264 | `rust/plans/ws14g-rails-test-map.md:753` | `test/system/drive_share_test.rb:895` | completed outcomes stay visible through reconnect and refreshed review |
| WS14g-265 | `rust/plans/ws14g-rails-test-map.md:754` | `test/system/drive_share_test.rb:928` | reconciled access is confirmed, not claimed as newly granted |
| WS14g-266 | `rust/plans/ws14g-rails-test-map.md:755` | `test/system/drive_share_test.rb:958` | existing and reconciled access are distinguished with no new grants |
| WS14g-267 | `rust/plans/ws14g-rails-test-map.md:756` | `test/system/drive_share_test.rb:986` | server errors are classified as service failures, not policy denials |
| WS14g-269 | `rust/plans/ws14g-rails-test-map.md:764` | `test/system/meeting_status_test.rb:7` | opting in shows In a meeting for a stubbed busy interval, then clears after it ends |
| WS14g-270 | `rust/plans/ws14g-rails-test-map.md:765` | `test/system/meeting_status_test.rb:34` | the profile links to connect without a Google account |
| WS14g-271 | `rust/plans/ws14g-rails-test-map.md:769` | `test/system/out_of_office_test.rb:4` | set OOO until tomorrow, badge and DM notice show for another user, then clear it |
| WS14g-272 | `rust/plans/ws14g-rails-test-map.md:773` | `test/system/sudo_mode_test.rb:8` | one prompt, then the action continues automatically |
| WS14g-273 | `rust/plans/ws14g-rails-test-map.md:774` | `test/system/sudo_mode_test.rb:27` | a wrong password keeps the action gated |
| WS15g-004 | `rust/plans/ws15g-rails-tests.md:99` | `test/controllers/agents/github_action_delivery_test.rb:69` | completion appears in event polling with the github_action payload |
| WS15g-005 | `rust/plans/ws15g-rails-tests.md:100` | `test/controllers/agents/github_action_delivery_test.rb:95` | failed completions poll with the message and no url |
| WS15g-006 | `rust/plans/ws15g-rails-tests.md:101` | `test/controllers/agents/github_action_delivery_test.rb:114` | ack works on completion rows |
| WS15g-007 | `rust/plans/ws15g-rails-tests.md:102` | `test/controllers/agents/github_action_delivery_test.rb:129` | completion posts the webhook with agent and github_action keys |
| WS15g-008 | `rust/plans/ws15g-rails-tests.md:103` | `test/controllers/agents/github_action_delivery_test.rb:151` | completion rows are readable by their own agent only |
| WS15g-009 | `rust/plans/ws15g-rails-tests.md:104` | `test/controllers/agents/github_action_delivery_test.rb:173` | the ledger page lists the completion with its status |
| WS15g-010 | `rust/plans/ws15g-rails-tests.md:105` | `test/controllers/agents/github_action_delivery_test.rb:188` | the ledger page lists failures with their reason |
| WS15g-011 | `rust/plans/ws15g-rails-tests.md:134` | `test/controllers/github/connections_controller_test.rb:71` | the profile cannot edit the login while a verified account is linked |
| WS15g-012 | `rust/plans/ws15g-rails-tests.md:135` | `test/controllers/github/connections_controller_test.rb:86` | the profile edits the login again once the link is disconnected |
| WS15g-013 | `rust/plans/ws15g-rails-tests.md:140` | `test/controllers/github/connections_controller_test.rb:144` | linking never logs the pasted token |
| WS15g-014 | `rust/plans/ws15g-rails-tests.md:141` | `test/controllers/github/connections_controller_test.rb:156` | the token parameter is filtered from request logs |
| WS15g-015 | `rust/plans/ws15g-rails-tests.md:157` | `test/controllers/github/pull_request_comments_controller_test.rb:156` | posting never logs the member's token |
| WS15g-016 | `rust/plans/ws15g-rails-tests.md:177` | `test/controllers/github/pull_request_review_requests_controller_test.rb:210` | requesting never logs the member's token |
| WS15g-017 | `rust/plans/ws15g-rails-tests.md:199` | `test/controllers/github/pull_request_threads_controller_test.rb:54` | discuss reuses the winner and drops the loser when the race is lost at the unique index |
| WS15g-018 | `rust/plans/ws15g-rails-tests.md:200` | `test/controllers/github/pull_request_threads_controller_test.rb:66` | discuss reuses the winner and drops the loser when the race is lost at the validation |
| WS15g-020 | `rust/plans/ws15g-rails-tests.md:219` | `test/controllers/github/webhooks_controller_test.rb:26` | valid pull_request signature updates the record and broadcasts once |
| WS15g-021 | `rust/plans/ws15g-rails-tests.md:236` | `test/controllers/github/webhooks_controller_test.rb:220` | subscribed repositories enqueue subscription delivery and post once |
| WS15g-022 | `rust/plans/ws15g-rails-tests.md:237` | `test/controllers/github/webhooks_controller_test.rb:239` | redelivered subscription events post nothing |
| WS15g-023 | `rust/plans/ws15g-rails-tests.md:251` | `test/controllers/rooms/github/pull_request_cards_controller_test.rb:105` | a cached denial no longer applies after the member relinks |
| WS15g-024 | `rust/plans/ws15g-rails-tests.md:252` | `test/controllers/rooms/github/pull_request_cards_controller_test.rb:139` | a transport error renders the empty frame and caches nothing |
| WS15g-025 | `rust/plans/ws15g-rails-tests.md:288` | `test/helpers/github_pull_requests_helper_test.rb:35` | cache key for a message without pull requests is just the message |
| WS15g-027 | `rust/plans/ws15g-rails-tests.md:290` | `test/helpers/github_pull_requests_helper_test.rb:53` | cache key changes when an older thread reply is deleted |
| WS15g-028 | `rust/plans/ws15g-rails-tests.md:291` | `test/helpers/github_pull_requests_helper_test.rb:66` | cache key reads the reply count without a query |
| WS15g-029 | `rust/plans/ws15g-rails-tests.md:292` | `test/helpers/github_pull_requests_helper_test.rb:78` | cache key carries the streaming flag |
| WS15g-030 | `rust/plans/ws15g-rails-tests.md:293` | `test/helpers/github_pull_requests_helper_test.rb:89` | cache key changes when a step is added to the message |
| WS15g-034 | `rust/plans/ws15g-rails-tests.md:297` | `test/helpers/github_pull_requests_helper_test.rb:151` | cache key changes when a quoted source is deleted |
| WS15g-035 | `rust/plans/ws15g-rails-tests.md:298` | `test/helpers/github_pull_requests_helper_test.rb:167` | cache key changes when a poll is voted and retracted |
| WS15g-036 | `rust/plans/ws15g-rails-tests.md:299` | `test/helpers/github_pull_requests_helper_test.rb:186` | cache key carries the system note flag |
| WS15g-037 | `rust/plans/ws15g-rails-tests.md:300` | `test/helpers/github_pull_requests_helper_test.rb:194` | cache key changes when the message is pinned and unpinned |
| WS15g-039 | `rust/plans/ws15g-rails-tests.md:302` | `test/helpers/github_pull_requests_helper_test.rb:227` | cache key changes when a referenced X post is fetched |
| WS15g-040 | `rust/plans/ws15g-rails-tests.md:303` | `test/helpers/github_pull_requests_helper_test.rb:241` | cache key changes when a referenced link embed is fetched |
| WS15g-042 | `rust/plans/ws15g-rails-tests.md:321` | `test/integration/github_pr_cards_test.rb:156` | rendering a room page costs no extra queries per message with a PR link |
| WS15g-043 | `rust/plans/ws15g-rails-tests.md:326` | `test/integration/github_pr_cards_test.rb:254` | the open-room join page leaks no card content to non-members |
| WS15g-047 | `rust/plans/ws15g-rails-tests.md:336` | `test/integration/github_pr_threads_test.rb:62` | the files summary omits the more line when everything is shown |
| WS15g-048 | `rust/plans/ws15g-rails-tests.md:337` | `test/integration/github_pr_threads_test.rb:78` | a PR thread without fetched files shows a loading summary |
| WS15g-049 | `rust/plans/ws15g-rails-tests.md:338` | `test/integration/github_pr_threads_test.rb:92` | an ordinary thread shows no PR header |
| WS15g-050 | `rust/plans/ws15g-rails-tests.md:339` | `test/integration/github_pr_threads_test.rb:105` | file paths from the API render as text |
| WS15g-056 | `rust/plans/ws15g-rails-tests.md:420` | `test/jobs/github/fetch_pull_request_job_test.rb:305` | Reopened after shared-receipt audit: execute the registered Github::FetchPullRequestJob through recorded API responses for a mapped PR and assert its thread stream receives the card/header replacement with the fetched title and file path. Direct PullRequest updates prove the downstream callback but omit the fetch-job producer. |
| WS15g-057 | `rust/plans/ws15g-rails-tests.md:421` | `test/jobs/github/fetch_pull_request_job_test.rb:328` | Reopened after PR #239 review: execute the registered Github::FetchPullRequestJob for a PR with neither referencing messages nor thread mappings and observe zero card/header publications, while proving the fetch committed. The cited callback test always creates both routes and expects positive frames. |
| WS15g-058 | `rust/plans/ws15g-rails-tests.md:427` | `test/jobs/github/perform_agent_action_job_test.rb:34` | approving a github action enqueues the job, denying does not |
| WS15g-059 | `rust/plans/ws15g-rails-tests.md:431` | `test/jobs/github/perform_agent_action_job_test.rb:78` | approving a non-github action enqueues nothing |
| WS15g-061 | `rust/plans/ws15g-rails-tests.md:594` | `test/models/github/write_client_test.rb:102` | network errors raise Error without logging the token |
| WS15g-062 | `rust/plans/ws15g-rails-tests.md:629` | `test/system/github_pr_write_actions_test.rb:6` | a linked member comments from a PR thread and sees the inline confirmation |
| WS15g-063 | `rust/plans/ws15g-rails-tests.md:630` | `test/system/github_pr_write_actions_test.rb:52` | a linked member requests a review from a PR thread and sees the inline confirmation |
| WS15g-064 | `rust/plans/ws15g-rails-tests.md:631` | `test/system/github_pr_write_actions_test.rb:98` | a member without a linked token sees the connect prompt in the thread |
