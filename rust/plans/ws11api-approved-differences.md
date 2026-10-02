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
