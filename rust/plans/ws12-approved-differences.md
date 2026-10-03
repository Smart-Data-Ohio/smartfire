# Approved board SLA request differences

The maintainer's PR #206 review ruling rejects reproducing accidental Rails
crashes for malformed SLA settings. The PR #215 follow-up explicitly includes
missing and scalar `sla_rules` roots: `{}` and `{"sla_rules": false}`, as well as
null, either boolean, integer, float and string roots.

Pinned Rails raises and serves its production 500 page without changing rules,
tag assignments or audits. Rust returns 400 with an empty body and makes zero
writes. `board_sla_missing_scalars.json` retains nine actual Rails requests and
unchanged facts; the HTTP regression traces every connection and asserts zero
INSERT/UPDATE/DELETE/REPLACE executions on each rejected request.

The earlier approval also includes malformed top-level arrays retaining hashes
and retained arrays under a work-status key. Accepted filtered arrays and
numeric-key hashes still match Rails, including rule deletion and audit writes.

The response comparator applies the exception only to PATCH requests on the
SLA settings endpoint, with Rails status 500 and an input exactly present in
`board_sla_approved_crash_inputs.json`. That list contains 35 distinct proven
crash inputs from the original, round-two and missing/scalar Rails corpora.
Valid roots and unrelated 500s receive no waiver. No other status, response
body, header or persisted-fact difference is approved by this document.
