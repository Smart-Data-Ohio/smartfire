# WS8b-m2 approved differences

## Atomic durable enqueue failure

Approved by the lead in the slice-H instructions on 2026-10-03, under
`decisions.md` decision 2. Rust rolls back the triggering metadata, sibling
claims and durable jobs when a job INSERT fails. No broadcasts escape that
transaction. This is an approved difference, not an owner-blocked item.

Rails enqueues after commit through its external adapter. Its false return and
`ActiveJob::EnqueueError` retain the committed writes and may enqueue the other
sibling; a hard adapter exception also retains the already committed writes.
The 24 actual Rails cases remain in `vectors/messaging/adapter_rejections.json`.
They are not re-pinned to Rust's results.

`probe_adapter_rejections.py` proves eight real Generic/LinkedIn first/second
SQLite enqueue failures at 4/16 references, comparing full persisted rollback
rows and actual empty queues/publications. Its producer control swallows the
real error and must fail at `adapter actual durable refusal`. Only this enqueue
boundary is excepted; successful callbacks and jobs still require Rails parity.
No owner API change or workaround is required.
