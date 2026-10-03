# PR214 scalar-ID compatibility corpus

The repaired array predicate keeps null compaction, singleton delegation and independent
casting of multi-candidate nested arrays/objects. String conversion matches pinned
Active Model decimal conversion: six Ruby whitespace classes (including vertical tab),
optional sign, optional `0d`/`0D`, underscores between digits and signed-i64 range checks.
`0x`/`0b`/`0o` retain decimal-to-i's numeric zero prefix; they do not switch radix.
A numeric decimal prefix without following digits (`0d`, `0d_1`) casts to zero.
Unicode whitespace and NUL prefixes remain nonnumeric; overflow remains rejected.

`pr214_id_corpus.rb` freshly executes 29,491 real Active Record predicates: 2,681 scalar
forms crossed with eleven scalar/array positions. Fixtures include zero, both signed
range endpoints, negative Alpha, Alpha and Beta. This distinguishes zero from NULL,
checks overflow rejection and primary-key ordering, and catches nested flattening.
The generator covers whitespace/sign/prefix/underscore/tail/overflow combinations.
Inputs, complete selected IDs and category counts are committed without masks.

The original parser fails with 6,426 mismatches. After the repair the same 29,491
predicates match with zero mismatches. The existing HTTP/state/job matrix is extended
to 139 cases / 145 responses: scalar and array-position reproductions for VT and radix
prefixes, and both empty-parent MCP cases plus REST controls. Every projected state
column, literal response/status/selected headers and queued job remains compared.
All generators are part of the normal agent vector recapture script.

The two webhook scheduling regressions use a real clock advancing by one microsecond
per call, compare returned and stored timestamps, and retain raw Rails timestamp receipts.
Both returned schedules were one microsecond later before the fix. Delivery captures
its UPDATE timestamp once; work webhook queuing returns the exact timestamp used by
its successful conditional UPDATE. No row reread is added and queue atomicity is unchanged.

Step parents apply Rails presence before integer coercion. An empty array no longer
becomes parent ID zero; both absent parents produce the shared 422 required-parent
validation. The two MCP envelopes and REST 422 controls make no step or job writes;
normal agent/credential authentication usage timestamps match Rails. Work/step service authorization and validation ownership remain unchanged.
