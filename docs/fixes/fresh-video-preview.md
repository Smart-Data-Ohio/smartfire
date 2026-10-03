# Fresh video previews (Rails fix)

## Root cause

Investigated against `origin/main` at `f85fb4200`, using Rails pinned at
`1a02651ac37f` and the read-only diagnosis from `d7d3390fd` / `b6abbb8ad`
(`rust/reference-tools/agents/review192r3_attachment_diagnosis.rb`).
`Message#process_attachment` analyzed and processed media synchronously.
Inside an enclosing transaction, Active Storage saves the generated JPEG's
attachment row but uploads its bytes in a record `after_commit` callback.
`preview(format: :webp).processed` immediately opens that JPEG to create the
WebP, raising `ActiveStorage::FileNotFoundError`. Multipart uploads fail even
earlier: analysis opens the original before its deferred upload. Preprocessing
a direct-uploaded video's preview outside the transaction avoids the first
failure; it does not fix the multipart original-upload ordering.

## Paths and callers

| Entry / caller | Processing transaction and observed impact |
| --- | --- |
| Web thread composer / `ChannelThreadMessagesController#create` and agent REST `Agents::Posting.post_thread_reply` | `ChannelThread#post_message!` runs inside `requires_new` / `with_lock`. Fresh multipart originals and fresh direct-upload previews both fail and roll back. This also covers board thread replies. |
| Forwarding / `Messages::Forwarder#save_forward!` | Room and thread destinations run inside the forward transaction. Each video gets a new original blob, without a copied preview, so even a previously processed source needs a fresh preview and fails. |
| Synchronous video webhook replies / `Webhook#create_sync_reply` | Thread replies use `post_message!` and fail on the generated JPEG. Reached by `Bot::WebhookJob` and agent delivery. Root replies use `create_with_attachment!` after the message save and succeed. |
| Web root composer, bot API (`Messages::ByBotsController`, both legacy and agent-backed), agent REST root / `Agents::Posting.post_root` | Process after `save!`, without an enclosing posting transaction. Fresh videos succeed; processing exceptions previously could still fail a request for a committed message. |
| MCP `post_message` / `open_dm`, agent REST `/agents/dms`, streaming APIs | Do not accept binary attachments. MCP and DM posting share `Agents::Posting`, but allow only text and Drive IDs. No fresh-video failure. |
| Slack importer / `Slack::MessageWriter` | History and reply pages use transactions, but import file links through `Slack::MarkdownConverter`, not Active Storage uploads, and call neither processing method. No fresh-video failure. |
| `RoomMailbox#process` / Action Mailbox job | Calls processing after saving; its attachment allowlist excludes videos. |
| `ScheduledMessage::Dispatcher#post!` / periodic runner | Calls processing inside its transaction (or delegates to `post_message!`), but scheduled messages carry text only. |
| `SlashCommands::Handlers#post_message`, `Agents::Polls.create`, `Rooms::PollsController#create` | Processing callers that post text only. Root slash messages process after saving; polls process after their transaction; thread slash messages delegate to `post_message!`. |
| `MessagePin#post_pin_note!`, `Event::ChannelTimeline#announce_in_channel`, `Github::Notifier.post_room_message!` | Remaining `create_with_attachment!` callers; all post text only. Pin notes are transactional, event announcements run after commit, and GitHub notifications process after saving. |

The current browser composer uses multipart `FileUploader` requests. Signed
direct-upload blob IDs are also accepted by the posting endpoints; the regression
suite covers both, including the authenticated metadata POST and disk PUT.
`create_with_attachment!` itself adds no outer transaction: callers determine
whether processing happens before commit.

## Fix and rendering

When a posting transaction is open, `process_attachment` uses
`ActiveRecord.after_all_transactions_commit` to enqueue
`Message::AttachmentProcessingJob` after Active Storage's record upload
callbacks. Nested savepoints defer to the outermost commit; rollback queues
nothing. The job runs the existing analysis and thumbnail logic outside the
posting transaction, makes up to three processing attempts, touches the
message to expire its presentation, and broadcasts a presentation replacement
to its room or thread using the existing Turbo pattern. Active Storage's normal
analysis jobs remain enabled.

Outside a transaction, successful synchronous image, video, and file processing
is preserved. Synchronous exceptions and enqueue errors are reported through
`Rails.error.handle` and do not fail a committed post. Background processing
errors remain job failures, rather than rolling back messages.

Pending videos render a playable original without a poster. The browser test
exposed a race between an immediate lazy poster request and the new job, both
generating the first JPEG; withholding the poster until its preview image is
attached avoids that race. Completion adds the existing lazy poster URL and
refreshes dimensions through the presentation broadcast. Rendering never calls
`.processed`; the subsequent poster request processes its resized WebP outside
a posting transaction. Images and existing video previews retain their rendering.

## Evidence

The first commit contains regressions that fail on main with the real storage
error (seven failing transactional paths, three passing root controls):

```text
10 runs, 37 assertions, 0 failures, 7 errors, 0 skips
```

Tests reuse `alpha-centuri.mov`; no new fixture is added. The committed source,
JPEG, and recorded WebP files are checked without using `.processed` to create
missing outputs in the assertions. Additional coverage exercises real direct
upload bytes, nested commit/rollback, pending rendering and poster delivery,
stream updates, decoder failure/retry, already processed videos, images, and
nonrepresentable files. The browser regression uploads through the thread
composer with normal CSRF protection enabled.

Validation used `BUNDLE_PATH=/home/riels/.cache/campfire-bundle`, at most eight
unit/integration workers and four browser workers:

```text
Regression + attachment model tests:
23 runs, 214 assertions, 0 failures, 0 errors, 0 skips
Related models/controllers/helpers/services/importer/mailbox tests:
472 runs, 2578 assertions, 0 failures, 0 errors, 0 skips
Related system tests (sending, threads, interactions, toolbars, mobile actions,
Drive attachments, file search, Markdown):
59 runs, 786 assertions, 0 failures, 0 errors, 0 skips
Regressions plus agent ledger/profile tests after real-commit cleanup:
127 runs, 614 assertions, 0 failures, 0 errors, 0 skips
PARALLEL_WORKERS=8 bin/rails test --seed 9661:
5517 runs, 32197 assertions, 0 failures, 0 errors, 3 skips
bin/rubocop (all five changed Ruby files):
5 files inspected, no offenses detected
```

Nontransactional regressions clean up their committed messages, threads, media,
sessions and agent ledger rows, and restore room/unread stamps; otherwise they
contaminate later tests even though ordinary fixture tables are reset. No
production access, deploy, or Rust modification was used.
