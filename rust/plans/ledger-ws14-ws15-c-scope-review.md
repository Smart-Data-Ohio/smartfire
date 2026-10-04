# PR #247 count and enqueue scope review

Reviewed head: `af543814eea99a1e2210a80420045dbc3ece565c`. Rails: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`.

All 50 C closures were checked against their original Rails setup, assertion bodies, native readers and shared helpers for the two requested patterns. WS14g-074 and WS14g-080 are corrected. No additional instance was found in the other 48 closures. The [assertion map](ledger-ws14-ws15-c-assertions.md) supplies each exact Rails and native assertion location. This scan makes no claim about patterns outside those two.

| Record | Rails declaration | Scope finding |
|---|---|---|
| WS14e-051 | `test/models/event/reminder_pusher_test.rb:32` | The full local push receiver request list is counted and the decrypted body is compared; no endpoint/class filter hides an extra delivery. |
| WS14e-057 | `test/models/event/reminder_pusher_test.rb:115` | Each suppressed case asserts the full receiver request list empty, with no type/user/payload filter. The running positive control then asserts exactly one decrypted delivery. |
| WS14g-004 | `test/controllers/messages_drive_attachments_test.rb:221` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-005 | `test/controllers/messages_drive_attachments_test.rb:239` | The DOM is the whole edit response, matching Rails' root selectors. File inputs include the full type/name/value predicate; two chip/removal counts use their exact classes. |
| WS14g-007 | `test/controllers/sudos_controller_test.rb:29` | The audit count uses action=sudo.confirm.success in both Rails and SQL; before/after difference is +1. No narrower user/session filter hides audit rows. |
| WS14g-011 | `test/controllers/sudos_controller_test.rb:67` | The extra passkey membership/duplicate count is a supplemental control on the named verifier, not a replacement for a Rails collection count. |
| WS14g-015 | `test/controllers/sudos_controller_test.rb:109` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-022 | `test/controllers/sudos_controller_test.rb:210` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-025 | `test/controllers/sudos_controller_test.rb:238` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-027 | `test/controllers/sudos_controller_test.rb:264` | The form count includes its exact action, and the input predicate is scoped under that form, matching Rails' nested assert_select. |
| WS14g-034 | `test/controllers/sudos_controller_test.rb:449` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-035 | `test/integration/drive_picker_test.rb:11` | All zero DOM counts use the whole response and the exact controller/menu/attachment-button selectors, in both no-account and calendar-only cases. |
| WS14g-036 | `test/integration/drive_picker_test.rb:30` | Controller/button/dialog predicates include every Rails selector attribute. Menu items are scoped under the single asserted attach-menu, so unrelated roles cannot satisfy the two-item count. |
| WS14g-037 | `test/integration/drive_share_picker_test.rb:16` | The enhanced controller/button counts cover the response. The Drive menu item predicate includes its text and is scoped under the single asserted attach-menu. |
| WS14g-038 | `test/integration/drive_share_picker_test.rb:32` | Controller counts cover the entire rendered response with the exact data-controller values. |
| WS14g-039 | `test/integration/drive_share_picker_test.rb:44` | Controller counts cover the entire rendered response with the exact data-controller values. |
| WS14g-040 | `test/integration/drive_share_picker_test.rb:55` | Controller/menu zero counts cover the entire response; no subtree omits another instance. |
| WS14g-041 | `test/integration/drive_share_picker_test.rb:65` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-053 | `test/jobs/calendar/disconnect_cleanup_job_test.rb:27` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-054 | `test/jobs/calendar/disconnect_cleanup_job_test.rb:44` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-056 | `test/jobs/calendar/disconnect_cleanup_job_test.rb:73` | Rails asserts a positive DisconnectCleanupJob retry with exact args; class-selected pending read verifies that identity and unchanged args, not a claim about every job class. |
| WS14g-058 | `test/jobs/calendar/disconnect_cleanup_job_test.rb:113` | Rails asserts a positive DisconnectCleanupJob retry with exact args; class-selected pending read verifies that identity and unchanged args. |
| WS14g-059 | `test/jobs/calendar/disconnect_cleanup_job_test.rb:128` | Rails assert_no_enqueued_jobs explicitly has only: Calendar::DisconnectCleanupJob; selected pending emptiness and terminal completion preserve that scope. |
| WS14g-062 | `test/jobs/calendar/disconnect_cleanup_job_test.rb:197` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-063 | `test/jobs/calendar/disconnect_cleanup_job_test.rb:218` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-066 | `test/jobs/calendar/remote_delete_job_test.rb:10` | The recorded client sees all requests for this isolated app; count=1 is supplemental and method/path/current token are compared. |
| WS14g-067 | `test/jobs/calendar/remote_delete_job_test.rb:19` | Full recorded request list is empty, without method/path/class filtering; stronger than Rails' no-DELETE predicate. |
| WS14g-068 | `test/jobs/calendar/remote_delete_job_test.rb:25` | Full recorded request list is empty, without method/path/class filtering; stronger than Rails' no-DELETE predicate. |
| WS14g-069 | `test/jobs/calendar/remote_delete_job_test.rb:33` | Full recorded request list is empty, without method/path/class filtering; stronger than Rails' no-DELETE predicate. |
| WS14g-070 | `test/jobs/calendar/remote_delete_job_test.rb:41` | Both requests are read from the full recorded list and both exact DELETE identities are compared; count=2 is supplemental. |
| WS14g-071 | `test/jobs/calendar/remote_delete_job_test.rb:53` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-072 | `test/jobs/calendar/remote_delete_job_test.rb:64` | Rails asserts a positive RemoteDeleteJob retry with exact args; selected pending row verifies the original class and payload. |
| WS14g-073 | `test/jobs/calendar/remote_delete_job_test.rb:73` | Rails assert_no_enqueued_jobs explicitly has only: Calendar::RemoteDeleteJob; selected pending emptiness and exactly one observed initial insertion reject another remote retry. |
| WS14g-074 | `test/jobs/calendar/sync_entry_job_test.rb:444` | FIXED: persisted entries now use Jason AND the three occurrence event IDs; count, occurrence identity set and distinct remote IDs are asserted. The SyncEntry enqueue count retains Rails' explicit only: filter. |
| WS14g-077 | `test/jobs/calendar/sync_entry_job_test.rb:484` | Rails explicitly limits the count to only: Calendar::SyncEntryJob; Rust retains that class filter and compares exact event/user args. |
| WS14g-078 | `test/jobs/calendar/sync_entry_job_test.rb:497` | Rails before-commit zero check explicitly selects only: Calendar::SyncEntryJob; independent queue reader applies that filter. After commit Rails expects the named job/args, which Rust compares. |
| WS14g-079 | `test/jobs/calendar/sync_entry_job_test.rb:508` | Both Rails assertions are positive named SyncEntry job/args expectations; class selection does not hide any Rails full-set prohibition. |
| WS14g-080 | `test/jobs/calendar/sync_entry_job_test.rb:521` | FIXED: positive SyncEntry identity assertion remains selected; unchanged save compares the entire post-clear enqueue observation set, with no class filter. |
| WS14g-083 | `test/jobs/calendar/sync_entry_job_test.rb:552` | Rails asserts positive named RemoteDelete job/args, then the exact DELETE request; class selection matches that expectation. |
| WS14g-084 | `test/jobs/calendar/sync_entry_job_test.rb:566` | Rails asserts positive named RemoteDelete job/args, then performs only that job class; Rust preserves the selected runner/readback scope and exact remote identity. |
| WS14g-121 | `test/models/drive_attachment_test.rb:9` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-122 | `test/models/drive_attachment_test.rb:18` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-123 | `test/models/drive_attachment_test.rb:25` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-124 | `test/models/drive_attachment_test.rb:37` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-125 | `test/models/drive_attachment_test.rb:47` | Each attachment count uses DriveAttachment::for_message with that specific saved message ID, matching first/second.drive_attachments.count. |
| WS14g-126 | `test/models/drive_attachment_test.rb:58` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-127 | `test/models/drive_attachment_test.rb:70` | Both before/after queries count the entire drive_attachments table, matching Rails' global DriveAttachment.count difference of -1. |
| WS14g-128 | `test/models/drive_attachment_test.rb:79` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-184 | `test/models/google_account_test.rb:44` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
| WS14g-185 | `test/models/google_account_test.rb:61` | Not applicable: the original declaration has no scoped count or whole-job-set enqueue check. |
