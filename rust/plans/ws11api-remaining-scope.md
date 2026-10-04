# WS11 API remaining scope before cutover

Updated on `rust/ws11api-next-6`, stacked on next-5 `491b9425`, merged with main `7e35a5fd6` (merge `f61de2c57`). #215 and #216
are merged. The pinned Rails reference remains `d7c7de92`. Domain named-case
mapping and broader API assertion evidence are separate inventories.

## Implementation status

All four REST and five MCP work writes formerly returning REST500 / MCP -32603
are installed on main via #202. They use WS12's writer, handoff and tag services;
244 pinned response/state/job vectors and flat query-growth controls remain in place.
No pending work-write adapter remains. No other unported WS11 API implementation
was found in the prior audit or this round. The broader named-case evidence is complete for all 51 audited declarations.

#203 (`rust/ws11api-proxy-headers`, `7ec7d26c`) is now merged through main.
Its Content-Transfer-Encoding correction is retained for successful streams and
handled empty 404s. The prior claim of exact proxy header parity was too broad:
Rust retains six security defaults that pinned Rails HTTP/1.1 Live responses omit.
The maintainer approved those six additions. The new oracle uses config.ru
Rack::Deflater and explicit HTTP/1.1, compares every response header and rejects
unexpected names and duplicate values. Only those six additions and the named
Date/X-Request-Id/X-Runtime request values are approved differences. Disk and
range transfer-encoding headers remain compared. No unapproved header drift is
accepted; ws11api-approved-differences.md records the exact scope.
Both approved JPEG/video crash differences and unconditional committed-file retention
remain explicitly documented; they are deliberate differences, not unported API paths.

The preceding rounds trimmed repeated capability/serialization-identity reads and
batched array candidates. The preceding API round fixed nested/singleton/null/hash integer
candidate coercion, sweeps scalar-ID service adapters with the shared Rails matrix,
and corrected executor query caching in the uncached oracle. WS12 services retain
authorization, history, audit and queue ownership. The named-case checker now
recognizes the nine assignment macro invocations, preserves pinned delivery case
order and runs in CI seed preparation. Race differences from Astra's #205
classification remain explicitly approved, with current-state checks retained.

The typed AgentBudgetNotice owner API is exported, and all three activity presenter
read seams use its batch reader. `ws11api-budget-notice-reader.md` records its exact
API and audiences. WS12 owns its ActivityItems::Recorder source integration.

## Completed API behavior checked by the audit

The 35 non-MCP JSON actions in config/routes.rb all have controller bindings and boundary
vectors. All 35 use installed services, including the four WS12 writes merged with #202. The six by-bot
actions and MCP POST/GET/DELETE transports have real bindings. All 38 metadata tools have
explicit dispatch; all 38 call installed service/model adapters, including the five WS12 writes merged with #202.

| Surface | Behavior implemented | Evidence in this tree |
| --- | --- | --- |
| Agent identity/presence | GET/PATCH /agents/me, omission/null/string casts, status validation, no secrets/no-store | agents.rs; agent_http.json; agent_surface.json |
| Events | Poll cursor/limit/envelope, X-Smartfire-Next-Since, viewer/grant/private-link redaction, acknowledgments | agent_event_access/polling; agent_polling_http.json; agent_permissions_http.json |
| Approvals | List/show/create/cancel, replay, budgets, zone-aware expiry/DST, permission-before-validation | agents/approvals.rs; agent_surface.json; agent_review192_http.json |
| Conversations | Root/thread post, signed attachment, context, DMs, streaming start/append/finalize, idempotency | conversations.rs; agent_conversation_http.json; agent_attachments_http.json |
| Message operations | Reactions, pin/unpin, create/read polls, steps, slash register/unregister | service adapters; reaction/pin/poll/HTTP vectors |
| Media proxy headers | Every header compared; six explicitly approved security additions on HTTP/1.1 proxies; disk/range transfer encoding preserved | config.ru Rack::Deflater oracle; 21 representation responses, 4 blob controls; duplicate/name/value mutation controls |
| Work readers | List/show work and board posts, filters, preloads, array IDs, private link policy | reads.rs; agent_reads_http.json; agent_review192_http.json |
| Fizzy | Four reads and approval-only card actions; all nine read/write MCP aliases, owner-only credential and no network on denied writes | integrations.rs; installed WS15e reads/requests/jobs; Fizzy HTTP vectors |
| GitHub | REST approval-only PR actions, owner identity/account policy, grants/budgets/replay and no external write before approval | github/agent_actions.rs; WS15g approval_requests and Accounts; owner HTTP tests |
| Legacy/agent bot API | Index/create/update/destroy and boost create/destroy, board/thread/system-note policy, replacements/fanout/grants/budgets | messages/by_bots.rs and boosts/by_bots.rs; bot/legacy/reaction/permission vectors |
| MCP transport | Stateless single-object JSON-RPC, four versions, Origin, modern header mirroring/base64, method/error/status rules, no sessions/SSE, tools metadata/envelopes | mcp.rs; 84 base vectors; surface and per-service vectors |
| Throttling | Per-credential/controller/action minute buckets, global MCP600 and per-tool limits, concurrency, exact 429 bodies and Retry-After | agent_api.rs; surface/permission vectors; concurrency HTTP regressions |
| Agent tokens | Digest/constant-time auth, unknown/blank/revoked/expired/inactive states, fresh grants, no agent token on human endpoint | concerns.rs; agent_access/credential; authentication/HTTP/security vectors |
| Bot keys/reply tokens | Digest keys, plaintext scrub registered once per process, create-only room-bound 15-minute replies, membership/deactivation, reset/show-once/sudo | concerns.rs; User::Bot domains; verifiers; by-bot HTTP tests and WS11-ui account controllers |
| Webhook transport | Timestamp/HMAC headers, encrypted signing secrets, SSRF/DNS/IP pinning, 7s timeout, transient legacy retries, five-attempt agent backoff/Retry-After | integrations/{webhook,jobs,agent_jobs}.rs; agents_webhook_contract.json; real socket/claim tests |
| Webhook production paths | Message/slash/work/approval/action payloads, thread-aware sync replies, hop/fanout rules, durable jobs/claims/recovery and token scrubbing | agent_delivery/payloads/work_events; agent_jobs; jobs/periodic.rs; installed path/callback vectors |

Existing approved JPEG/video crash and committed-file-retention differences remain explicit
in ws11api-approved-differences.md. They are deliberate reviewed differences, not hidden
unported paths. Native media-byte/version differences still require the pinned runtime for
exact byte comparisons; sizes/checksums are never masked.

## Completed named comparisons

All **378 domain declarations in 26 pinned files** are mapped and have actual
cargo passes. The separate broader #215 assertion audit is also complete:
**51 passed, 0 pending**. Next-5 closed 20 declarations; this round closes the
remaining **31**, against **182 freshly executed Rails HTTP responses and four
real producer outputs**. Each has an individual activated producer control
rejected at its intended assertion. An additional query-growth control is rejected
at the flat-read assertion. No assertion or vector is mutated by a control.

REST/MCP bodies, statuses and selected headers are compared as raw bytes. Human
history/HTML checks reproduce each original declaration's actual JSON fields,
selectors, text and links. The agent-board integration uses real session cookies,
CSRF, a human HTTP post, an agent reply, result update and a rendered inbox card.
Webhook cases claim committed durable jobs and post them through the real runner;
only DNS/TCP is routed to the local recording endpoint. Grant revocation and
membership removal happen after a successful poll. Both 60-handoff throttle limits
and shared REST/MCP bucket behavior are exercised. Positive fixture premises run
in the existing CI checker, so an empty list or denied request cannot earn a credit.

Complete work/history/ledger/handoff/audit facts and logical queue job class/args
are checked, including thread/message targets, notes, acknowledgments and inbox source
IDs where applicable. The message-rate case compares a full logical job multiset
with duplicates preserved: Rails' test adapter and Rust's atomic queue register
push/delivery jobs in different order, which the original declaration does not
specify. This comparison does not claim identical queue execution order. Scheduling
other agents' hooks for later test steps changes only test run-at inputs.

| Original API file | Next-5 | Next-6 | Pending |
| --- | ---: | ---: | ---: |
| `test/controllers/agents/mcp_handoff_test.rb` | 2 | 1 | 0 |
| `test/controllers/agents/posts_controller_test.rb` | 2 | 8 | 0 |
| `test/controllers/agents/work_controller_test.rb` | 14 | 8 | 0 |
| `test/controllers/agents/work_delivery_test.rb` | 0 | 9 | 0 |
| `test/controllers/agents/work_handoff_test.rb` | 2 | 4 | 0 |
| `test/integration/agent_boards_test.rb` | 0 | 1 | 0 |

`ws11api-named-api-cases.json` records each original name, exact Rust function,
vector, producer recipe and receipt. The checker reads each declaration's own
receipt file and optionally requires its actual cargo pass. The domain ledger has
**0 deferred names** and the broader API ledger has **0 pending declarations**.
**Only the peer-owned Recorder integration below remains flagged for this scope.**

### Work polling read fix

The fresh two-size comparison exposed per-event room/actor/thread/payload and
private-link preflight reads. At 5/50 work events, REST grew from **61 to 466** and
MCP from **62 to 467**; Rails stayed **17/17** and **19/19**. Batched event/thread
associations and shared work payload preloads now hold Rust at **25/25** and
**26/26**. Repository decisions remain occurrence-scoped; Accounts still owns
permission caching, and current identity sealing/redaction remains enforced.
The regression first failed against next-5's production implementation. All other
measured board/work/read/write/ack paths remain flat at 5/50 rows; the exact table
is in `ws11api-next-6-report.md`. No WS12 grant, ledger, audit or service transaction
logic is replaced.

### Recorder review and actual peer-owned seam

Fresh uncached Rails and Rust run real Recorder transactions, commit callbacks,
complete persisted items and activity frames at 10/100 recipients. Schema discovery
is warmed before each measured Rails size; the executor surrounds `uncached`, and
zero cache hits are required. Source and user preflight reads remain **1 + 1**.
Create SELECTs are **22/202 in both apps**. Repeat SELECTs are **12/102 in Rust**
and **32/302 in Rails**. Rust's per-recipient growth is no faster than Rails.
Activated controls for per-recipient user reloads and wrong persisted event types
fail the precise cost/fact assertions. No Recorder service implementation changed.

The AgentBudgetNotice Recorder source integration is genuinely still peer-owned:
`rust/crates/db/src/models/activity_item/recorder.rs:377` requires an explicit owner
reader, whose public contract is in `recording_source.rs:47`. The typed WS11 reader
and presenter consumers are installed. Wiring the source producer belongs to WS12's
unmerged `rust/ws12-board-automations-4` round. No WS12 board-automation file is edited.

## Peer-owned fixed read costs

The preceding API round removed repeated adapter capability and room lookups, repeated ledger
identity/hop/actor facts, eligibility reads and post-insert/update ledger reloads.
Complete current authorization and fresh private-account sealing remain. Query
growth stays flat at 5/50 owned rows; exact before/after totals are recorded in
ws11api-next-3-report.md. Required WS11 auth, response and event reads remain;
those costs are not assigned to peers. At the preceding API checkpoint, the largest fixed gap was inside
WS12's `agent_work::{find_owned,writable,update_work,set_result,handoff_work}`,
`ChannelThread::{create_board_post,update_work,update_result}` and their after-commit
board rendering. They repeat thread/room/owner membership/capability reads and complete
board render preloads per callback. WS12 owns those service/read-boundary optimizations; #215 includes owner read-cost work. This round rechecks the nine adapter paths at 5/50 rows and the generic Recorder at 10/100 recipients; it changes no service transaction or policy contract.
`work_threads.rs` and `presenters/boards.rs` remain untouched. WS15g/WS15e retain external
GitHub/Fizzy clients and account authorization. WS11-ui owns account/agent HTML pages.
