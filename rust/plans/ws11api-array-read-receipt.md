> Measurement correction (#210): the original recorder counted SELECT notifications,
> including three query-cache hits. Its outer uncached block was undone by the
> request executor. The fixed recorder disables every pool after the executor's
> AR hook and asserts zero cache hits/enabled-cache SELECT events. It reproduces
> 29/29 thread and 30/30 cursor executions with caching truly disabled. With
> normal request caching, Astra measured 26/26 and 27/27 physical executions plus
> three hits; the former explanation of three fixture/setup reads was incorrect.
> All response/state vectors and the flat candidate-growth result are unchanged.

# Agent array lookup regression

The new two-size regression was run before replacing the per-candidate loops on
this stack (45bffea35 plus the main merge 2d019ce5d). It failed with:

```text
thread: 31 -> 76 SELECTs
before: 32 -> 77 SELECTs
after: 32 -> 77 SELECTs
react: 16 -> 61 SELECTs
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2573 filtered out; finished in 1.47s
```

After batching, the same reader/writer instrumentation reports:

| Surface | Rust before, 5/50 missing IDs | Rust after, 5/50 | Fresh Rails recorder, 5/50 |
| --- | --- | --- | --- |
| MCP thread | 31/76 | 26/26 | 29/29 |
| MCP before | 32/77 | 27/27 | 30/30 |
| MCP after | 32/77 | 27/27 | 30/30 |
| MCP reaction | 16/61 | 11/11 | 9/9 |
| MCP context message | 27/27 | 27/27 | 34/34 |
| MCP context thread | 26/26 | 26/26 | 27/27 |
| REST context message | 28/28 | 28/28 | 33/33 |
| REST context thread | 27/27 | 27/27 | 26/26 |

The original fresh recorder counted SQL notifications, including three query-cache
hits; it was not actually uncached because the executor re-enabled the cache.
The corrected recorder disables caching after that hook and asserts zero hits.
Astra's normal-cache physical executions are 26/27; the truly uncached capture is
29/30. Both stay flat. Rust reader counts use reader connections; reactions also
capture the writer after two idempotent warmups.

The Rails recorder is `array_read_contract.rb`, pinned at d7c7de92. The 24 exact
status/body/selected-header vectors cover both sizes and global thread selection
before membership, room-scoped selection, conversation-scoped cursors, revoked
grants before invalid limits and inactive authentication before missing rows.
The 40,000-candidate control exceeds SQLite's variable ceiling. Each lookup binds
one JSON array through `json_each`, with at most five binds including scoping.
Candidates retain Rails' database primary-key selection order; query order is not
request-array order and a forbidden globally selected thread does not fall back.

Sweep: context message/thread lookup already used a single IN query, now also
bounded by one JSON bind. The reaction loop was the additional growing lookup and
is fixed. REST `/agents/context` shares those helpers. Room arrays in board
filters use the preloaded accessible-room relation. Work write array IDs already
use a single JSON-backed IN query. MCP `ack_events` deliberately retains its
per-event authorization/acknowledgment loop, matching Rails' loop and maximum100;
its individual members can themselves be candidate arrays, now covered by the
next-3 matrix and bounded scoped lookup. Drive IDs have no per-ID database
lookup. Scalar-only parameters retain their existing Ruby coercion rules.

Commands used (from the worktree; target and environment in `.scratch/next-2/run.sh`):

```sh
bash .scratch/next-2/run.sh test -p campfire ws11_array_ -- --nocapture --test-threads=8
PARITY_NAMESPACE=ws11api-next2-arrays PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/array_read_contract.rb
```

The final fresh-clone run is reported separately in ws11api-next-2-report.md.
