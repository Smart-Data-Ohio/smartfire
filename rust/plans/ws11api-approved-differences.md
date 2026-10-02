# Approved agent API difference: fresh JPEG in a closed thread

The maintainer explicitly ruled on PR #192's second review that Rust must not
reproduce an accidental Rails crash. This exception applies to the
`fresh_jpeg_closed_thread` vector in `agent_review192r2_attachment.json`:
Rails returns 500, while Rust returns 201. Rust's body and selected headers match
the successful Rails idempotent replay of that already-committed message.

The pinned Rails revision is d7c7de92, with Rails gem 1a02651ac37f. The exact
exception is `IOError: closed stream`, raised by `IO.copy_stream` in
`activestorage/lib/active_storage/service/disk_service.rb:23`.
`ActiveStorage::VariantWithRecord#transform_blob` yields the transformed output
at `app/models/active_storage/variant_with_record.rb:49-52`. That output closes
before the enclosing `ChannelThread#post_message!` transaction commits
(`app/models/channel_thread.rb:489`). The deferred attachment upload then uses
the closed stream. This is an after-commit defect, not a deliberate validation.

Both applications commit the same message, reopened thread, source analysis,
variant and attachment rows. The fresh variant's file is absent. Neither source
nor variant analysis is queued; the only queued job is
`ChannelThread::PushMessageJob`, with the same thread/message Global IDs.
Rust discards the staged variant after commit without raising the Rails defect.
An error inserting the variant inside the transaction still rolls everything
back, including the reopened thread and durable jobs.

The committed vector retains both statuses and bodies, the original exception
and stack, and the agreed state. Its regression checks all message/thread
columns, source metadata, all nonrandom variant/blob attributes, attachment
relationships, file presence and size, and all queued job classes/logical named arguments. Runtime-specific Active Job
Global ID envelopes are mapped to the Rust queue's named arguments, retaining
both IDs and their relationship.
Only random storage keys and attachment/variant surrogate IDs are omitted;
relationships and exact counts are checked. Native media versions may change
encoded image size/checksum; the pinned runtime is the canonical state oracle.
This approval does not permit other status, header, response or state drift.

## Fresh video in a closed thread (PR #192 third review)

The maintainer also approved returning Rust 201 for a fresh uploaded copy of
fixture video 9. Pinned Rails returns 500 with
`ActiveStorage::FileNotFoundError: ActiveStorage::FileNotFoundError`, raised at
`activestorage/lib/active_storage/service/disk_service.rb:152`, in
`DiskService#stream` rescuing `Errno::ENOENT`. The preview image upload is deferred
until commit, but `VariantWithRecord#transform_blob` opens that image at line 48
while `ChannelThread#post_message!` still has one transaction open. This is an
accidental upload-ordering defect, not a validation. Rails rolls back the message,
thread reopening, preview/variant rows and jobs; the separately uploaded source
blob/file survives.

The `fresh_video_closed_thread` vector in `agent_review192r3_attachment.json`
retains that failing request and its rollback state. Its separate approved state
comes from ordinary Rails preprocessing outside the posting transaction, followed
by the same HTTP request: the preview is generated and analyzed first, then the
WebP variant is generated. This produces Rails' successful response and a valid
media-state oracle without patching the failing request or masking either status.
Rust returns 201, commits the message and reopening, retains the source, preview
and WebP files, and queues variant analysis and thread push with the recorded
logical arguments. Every nonrandom row field and every file size is asserted in
the pinned runtime. Queue execution/insertion order is not an API contract.

The JPEG discard is scoped to the original JPEG source, before a video source is
replaced by its JPEG preview. The documented JPEG file-absence exception above
is retained; it cannot apply to a video's JPEG preview. New/reused video variants
and reused existing JPEG variants retain their files. Injected in-transaction
failures remove staged preview/variant files and roll back every domain/media/job
row. These narrow approvals authorize neither other missing files nor other
response/state differences.

## Committed file ownership and missing-file serving (PR #192 fourth and fifth reviews)

Staged original/preview/variant files are retained immediately when COMMIT
succeeds, before fallible model callbacks. A callback exception still propagates
and cancels later ordinary callbacks, matching pinned Rails' transaction runner.
It cannot revoke ownership of committed files or the notifications for committed
durable jobs. Failed writes, savepoints and failed COMMIT still remove staged
files. The callback experiment and its committed receipt live in
`reference-tools/agents/review192r4_commit_callbacks.rb` and
`review-192-r4-callbacks.json`.

This file-retention guarantee is a deliberate state difference. A Rails
after-commit callback that raises before Active Storage's deferred upload leaves
the image blob, attachment and variant rows committed without the image file;
later callbacks, including the upload, are skipped. Rust retains the staged file
whenever COMMIT succeeds, independently of those fallible callbacks. Both return
500 for the callback exception and skip later ordinary callbacks. The maintainer
approved preserving committed file ownership instead of reproducing Rails'
missing-file state. The fresh-JPEG discard exception above remains explicit.

Representation requests preserve Rails' handled missing-file responses. Processed
records are reused without regenerating files: an existing WebP can be served
without its intermediate JPEG preview. When the final variant file is absent,
the redirect still returns 302 to the signed disk URL, whose follow-up is an empty
404; the representation proxy returns an empty 404 with the image content type,
disposition and no-cache header. Rows, files and jobs stay unchanged. These are
ordinary handled responses, not an approved crash difference. The three scenarios
and their exact bodies/headers are recorded in
`agent_review192r5_representations.json`.
The approved JPEG posting exception remains a metadata exception: Rails permits
subsequent posting to reuse its committed variant record even without its file.
Message attachment preparation preserves that behavior; missing-file serving
uses the handled responses above. The video oracle retains every attachment's record_id and
record_type, with exact foreign keys and generated row-count checks.
