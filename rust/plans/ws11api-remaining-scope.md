# WS11 API remaining scope before cutover

Updated on `rust/ws11api-next`, stacked on #202 and merged with main
`34b3cd40c7043150cd659397f513626b2729253f`, which includes #192 and #202.
The pinned Rails reference remains `d7c7de92`. This is an implementation audit
and an exact named-case ledger, not a claim that every possible input is tested.

## Implementation status

All four REST and five MCP work writes formerly returning REST500 / MCP -32603
are installed on main via #202. They use WS12's writer, handoff and tag services;
244 pinned response/state/job vectors and flat query-growth controls remain in place.
No pending work-write adapter remains. No other unported WS11 API implementation
was found in the prior audit or this round. Broader named-case evidence remains partial.

#203 (`rust/ws11api-proxy-headers`, `7ec7d26c`) is separately under review.
Its representation proxy header correction and nine-header oracle are not duplicated
on this branch. That reviewed follow-up remains owned by #203 until main includes it.
Both approved JPEG/video crash differences and unconditional committed-file retention
remain explicitly documented; they are deliberate differences, not unported API paths.

This round trims repeated capability and serialization-identity queries in WS11's
write adapters and ledger webhook callback. WS12 services and their authorization,
history, audit and queue behavior remain authoritative and unchanged.

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

The pinned domain inventory has 378 cases in 26 files. This round closes 15 individual
comparisons with real model/job paths and fresh pinned Rails vectors: three reply-token
cases, two delivery lock/hop cases, one kill-switch ownership case, and nine assignment
ledger/history/commit/rollback cases. The mapping is now **312 comparisons, 66 deferred**
(previously 297/81). Executed pass counts are reported separately from mapping counts.
The source manifests and vector projections do not silently credit broader API tests
as closure of unmapped names. No new production behavior mismatch was found in the
15 comparisons; negative mutations prove the new assertions discriminate.

The delivery lock case uses Rails' original lock spy and SQL transaction observation,
then challenges Rust with two independent SQLite writer connections sharing one file:
one remaining allowance, two concurrent source writes, one admitted job, one suppression.
This tests the actual serialized write boundary instead of copying a Ruby method spy.
The old absent-HTTP-credential deferral is closed by existing REST/MCP auth vectors.

Every remaining name is listed below. Deferral is evidence debt unless stated otherwise;
installed production code and broader API vectors do not replace a named comparison.
This is a coherent partial checkpoint: WS11-owned comparisons still remain, so the
remaining list is **not only owner-blocked**.

### test/models/agent_test.rb

Owner: WS11-ui rendered broadcast cases; WS11 domain cases compared.

Reason: WS11-ui owns rendered badge/directory broadcasts and secret-leak HTML assertions.

- status change broadcasts badge and directory row replaces to agents:all
- status broadcasts carry no credentials or grants

### test/services/slash_commands/dispatcher_test.rb

Owner: WS11 agent dispatch; WS8 built-in commands.

Reason: WS8 owns built-in command behavior; WS11 custom agent registration/invocation comparisons are complete.

- registry holds every shipped command with metadata
- command_text? matches slash commands but not escapes or play passthrough
- huddle starts a call when configured
- huddle errors when unconfigured
- event opens the prefilled form url
- event without a time prefills the title only
- event rejects past times
- bare event opens the blank form
- poll opens the builder in channels but not threads
- remind posts and saves with a reminder
- remind rejects unusable input without posting
- status sets emoji and text until end of day
- status rejects blank arguments
- dnd toggles, takes durations, and turns off
- dnd rejects garbage durations
- ooo sets an end with a note, and off clears it
- ooo takes week durations, dates, and datetimes
- ooo bare tomorrow and weekdays run to the end of the day
- ooo bare dates run to the end of the day
- ooo bare month dates roll to next year when this year's passed
- ooo day durations stay exact
- ooo broadcasts the badge and the notice
- ooo off while calendar OOO covers says the calendar still shows it
- ooo rejects blank arguments, garbage, past times, and long notes
- shrug posts with the shrug
- posting commands in a board answer an error without posting
- posting commands in a board thread still post
- slash posts in threads skip the legacy webhook fanout
- slash posts in channels fan out to legacy webhooks
- slash posts in threads process attachments once
- me posts an action line
- me requires an action
- play posts through the normal message path
- slash posts never start a stream
- unknown commands error with the available list

### test/models/channel_thread_agent_assignment_test.rb

Owner: WS11 agent callbacks; WS12 mutation producers.

Reason: WS12 owns the first eight owner-eligibility/viewer cases and the eight agent-work mutation/validation cases. WS11 owns the final four hop/deletion/root callback comparisons; these remain unworked at this checkpoint.

- an active member agent with post_messages is an eligible work owner
- a legacy agent keeps post eligibility through the fallback
- a suspended agent is rejected with a validation error
- a non-member agent is rejected with a validation error
- an agent without post_messages is rejected with a validation error
- a human outside the parent room keeps the human eligibility error
- a bot without an agent row is rejected with a validation error
- suspending the agent or revoking its membership reads as an unavailable owner
- an agent status update records a work event with the note
- an agent status update reaches the inbox through the human path
- an agent tags update replaces the set without touching the status
- an agent work update validates tags and run_url
- an agent result update records the event with the agent as actor
- an agent cannot write the result of work it does not own
- an agent cannot move work it does not own
- an agent status update rejects unknown statuses and long notes
- two agents assigning posts to each other stop at the hop limit
- deleting an agent-owned thread emits work_unassigned
- deleting a thread without an agent owner emits nothing
- a human assignment starts a new root at hop 0

### test/models/message_streaming_test.rb

Owner: WS11 finalization; WS12 activity; WS14/15 external reference sync.

Reason: WS11 owns complete start/coalescing/finalization assertions; WS12 activity and WS14/15 external-reference side effects keep their owners. The full four named projections remain unworked.

- stream start broadcasts the append without the unread broadcast
- finalize fires every side effect exactly once
- appends re-render without firing side effects
- a coalesced update sends a trailing broadcast with the final text

### test/models/user/bot_test.rb

Owner: WS11 bot domain/removal; WS11-api by-bot HTTP surface.

Reason: WS11 owns fixed-entropy factory/reset comparisons and the actual queued webhook execution comparison. Real digest/token/HTTP tests are additional evidence; these three names remain unworked.

- create bot
- reset bot key
- deliver message by webhook

### test/models/agent_budgets_test.rb

Owner: WS11 domain.

Reason: WS12 owns ActivityItem.accessible_to and capped WorkHandoffs.create producers/viewers. The complete named comparisons remain deferred to that owner.

- a stranger cannot read another agent's budget item
- an agent-created handoff counts toward no budget, even with caps exhausted

## Peer-owned fixed read costs

Query growth remains flat at 5/50 owned rows. The remaining fixed gap is primarily
inside WS12's `agent_work::{find_owned,writable,update_work,set_result,handoff_work}`,
`ChannelThread::{create_board_post,update_work,update_result}` and their after-commit
board rendering. They repeat thread/room/owner membership/capability reads and complete
board render preloads per callback. WS12 owns those service/read-boundary optimizations;
this branch names them without changing their transaction or policy contract.
`work_threads.rs` and `presenters/boards.rs` remain untouched. WS15g/WS15e retain external
GitHub/Fizzy clients and account authorization. WS11-ui owns account/agent HTML pages.
