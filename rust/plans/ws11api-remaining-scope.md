# WS11 API remaining scope before cutover

Updated on `rust/ws11api-next-5`, based on main `f85fb420`. #215 and #216
are merged. The pinned Rails reference remains `d7c7de92`. Domain named-case
mapping and broader API assertion evidence are separate inventories.

## Implementation status

All four REST and five MCP work writes formerly returning REST500 / MCP -32603
are installed on main via #202. They use WS12's writer, handoff and tag services;
244 pinned response/state/job vectors and flat query-growth controls remain in place.
No pending work-write adapter remains. No other unported WS11 API implementation
was found in the prior audit or this round. Broader named-case evidence remains partial.

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

## Broader named-case evidence still partial

All **378 domain declarations in 26 pinned files** are now mapped. This round
freshly executes the 16 merged WS12 assignment cases: identical complete owner,
validation, history and inbox facts, with no masks. All 16 producer controls
activate and fail the intended persisted-fact assertion. The generated `cases!`
functions are now discovered by the CI named-case checker; its regression fails
when those macro invocations are hidden.

The #215 assertion audit also flags 51 broader API declarations. That is a
separate obligation, previously obscured by the domain-only ledger. This round
closes **20 of 51** with **31 fresh Rails responses**, exact body bytes/status/
selected headers, committed work/history/ledger/handoff/audit/jobs, and one
activated producer control rejected at the intended assertion per declaration.
Set/clear and set/repeat execute real writes, rather than pre-seeding the result.
Non-owner writes supply valid fields; scoped grants and suspended receivers are
isolated. Session requests use real cookies and CSRF tokens; bot-key requests
use an authenticated key. No WS11-API behavior difference was found in this slice.

`ws11api-named-api-cases.json` is the exact broader ledger. Its CI checker verifies
pinned declaration names, generated Rust functions, real vector observations and
mutation receipts. Runtime pass counts additionally require cargo's actual output.
The domain ledger has **0 deferred names**; the broader API ledger still has
**31 pending declarations, all unblocked WS11-API evidence work**. This checkpoint
is partial. It does not claim only owner-blocked items remain.

| Original API file | Passed this round | Pending evidence |
| --- | ---: | ---: |
| `test/controllers/agents/mcp_handoff_test.rb` | 2 | 1 |
| `test/controllers/agents/posts_controller_test.rb` | 2 | 8 |
| `test/controllers/agents/work_controller_test.rb` | 14 | 8 |
| `test/controllers/agents/work_delivery_test.rb` | 0 | 9 |
| `test/controllers/agents/work_handoff_test.rb` | 2 | 4 |
| `test/integration/agent_boards_test.rb` | 0 | 1 |

The exact pending declarations follow; their original assertion locations and
reasons remain in the JSON ledger. None is labelled an unmerged service blocker.

### `test/controllers/agents/mcp_handoff_test.rb` — owner WS11-API

- Line 100: handoff_work shares its throttle bucket with the rest endpoint

### `test/controllers/agents/posts_controller_test.rb` — owner WS11-API

- Line 288: lists the board's open posts newest activity first
- Line 312: a reply moves the post to the top of the list
- Line 328: lists by single status, done, and all
- Line 360: lists by owner me, agents, and id
- Line 395: caps the list at 100 posts
- Line 416: a legacy agent without grants can list but cannot create
- Line 430: listing is 404 once the agent leaves the board
- Line 438: creation, listing, and event delivery share one work payload

### `test/controllers/agents/work_controller_test.rb` — owner WS11-API

- Line 12: list shows only owned threads newest first with the work fields
- Line 63: list is Bearer-only and empty without owned threads
- Line 114: show includes links with pull request, event, and drive entries
- Line 158: list includes links on owned threads
- Line 184: patch changes the status and records the note in work history
- Line 228: list excludes rooms the agent no longer belongs to
- Line 476: put result requires ownership and manage_threads
- Line 522: put result checks ownership and grants before markdown presence

### `test/controllers/agents/work_delivery_test.rb` — owner WS11-API

- Line 14: assignment appears in event polling with the work payload
- Line 34: assignment work payload includes links
- Line 51: unassignment appears in event polling
- Line 64: ack works on work rows
- Line 75: assignment enqueues the webhook instead of blocking on it
- Line 97: assignment posts no webhook without read_messages
- Line 113: polling omits work rows for rooms the agent lost read_messages in
- Line 128: polling omits work rows after membership removal
- Line 142: work events do not count toward the message rate limit

### `test/controllers/agents/work_handoff_test.rb` — owner WS11-API

- Line 45: the receiver polls the handoff with its context package
- Line 64: the receiver acks the handoff row
- Line 76: the handoff enqueues the receiver webhook with the package
- Line 176: handoffs throttle at 60 a minute per credential

### `test/integration/agent_boards_test.rb` — owner WS11-API

- Line 17: an agent post flows from creation through reply to result

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
