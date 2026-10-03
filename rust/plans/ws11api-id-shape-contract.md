# Agent API candidate IDs (#210 follow-up)

`reads::lookup_ids` implements Active Record's integer predicate candidate casting.
It compacts nulls and unwraps a sole candidate recursively. With multiple
candidates, nested arrays and objects cast independently to nil rather than being
flattened or rejected. Integer strings, prefixes, booleans, negatives, duplicates,
float truncation and range rejection are pinned in `agent_id_casting.json`.
Selection stays in the original global/association scope and primary-key order.
One JSON bind keeps candidate lists below SQLite's parameter limit.

The shared Rails matrix has 111 cases / 117 responses, including Alpha/Beta
selection, mixed objects followed by valid IDs, reaction replay, scoped cursors,
context priority/constraints, revoked grants and inactive authentication. It
sweeps room, thread, message, poll, approval, step, owned-work/post, receiver and
nested acknowledgement-member candidates across MCP and applicable REST reads.
GitHub approval PR candidates use global lookup before the existing room-thread
policy. REST strong parameters and path IDs retain their existing permitted
shape; non-ID arrays and Ruby to_i scalar cursors are not rewritten as row finders.
Fizzy's external string keys retain WS15e's existing string coercion.

`id_args` adapts candidate arrays only where installed service APIs take scalar
IDs. It does not replace grant, membership, validation or ledger service checks.
Nested acknowledgements preserve the submitted value in the result envelope.
The matrix compares literal statuses/body/selected headers, every column of 20
projected tables and queued jobs. Shared snapshots and complete changed-table
overlays are lossless storage, not masks. A recent pin-note fixture removes
random input entropy while keeping pin rows, timestamps and IDs compared.

The initial 52-case regression failed with 18 exact observable differences on the
starting implementation; the integer candidate regression also failed. The
extended acknowledgement regression failed with 15 differences across its three
cases before adaptation. Successful reaction replay remains created true/false.
Eight two-size candidate reader checks remain flat. New budget labels also have
an independent wrong-observable oracle negative control.

The separate array count recorder now disables pool caching after the request
executor's Active Record hook and asserts zero cache hits/enabled-cache SELECTs.
Normal cached Rails executions are 26/26 threads and 27/27 cursors (three hits).
Fully uncached Rails executions are 29/29 and 30/30 with zero hits; these are not
fixture/setup reads. Historical receipts are explicitly corrected.
