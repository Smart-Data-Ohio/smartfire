# WS11 API remaining scope before cutover

Updated on `rust/ws11api-next-2`, stacked on #205 (45bffea35), merged
with main 500c3f6987e9aa48382e2be15396597dc37a66af. The #205 branch is not rewritten.
The pinned Rails reference remains `d7c7de92`. This is an implementation audit
and an exact named-case ledger, not a claim that every possible input is tested.

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

The pinned domain inventory has 378 cases in 26 files. This round closes all
13 remaining WS11-owned names: four streaming projections, three bot cases,
four assignment callbacks and two budget comparisons. The ledger is now
**325 comparisons, 53 deferred** (previously 312/66). Executed pass counts are
reported separately from mapping counts. Each new named comparison has a fresh
pinned Rails vector and an independently wrong observable negative control.
No new production behavior mismatch was found in those 13 cases.

Streaming uses real model callbacks, actual WebSocket frames and both human
subscribers' unread broadcasts, activity/ledger/index projections and logical
queued arguments. Factory/reset entropy is controlled at the generator input.
Queued webhook comparisons run actual durable claims and handlers; only the
external dialer is replaced, like the Rails Net::HTTP fixture. WS12's merged
activity viewer and handoff service execute unchanged, including stranger
redaction and exhausted message-cap handoff behavior. The original budget case
sets all three caps but consumes the message cap; this comparison preserves that
named case rather than claiming three separately exhausted usage buckets.

The delivery lock comparison from #205 retains its two independent writers.
The array lookup sweep additionally fixes thread/cursor/reaction N+1 lookups,
preserves Rails' global/scoped selection and permission order, and handles
40,000 candidates with one JSON bind. All WS11-owned named comparisons are now
closed. **Only peer-owned evidence remains**; none is called an unmerged service
blocker. The exact remaining names follow.

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

Reason: WS12 owns these eight owner-eligibility/viewer and eight mutation/validation named comparisons. The four WS11 hop/deletion/root names are now compared using installed WS12 producers. These are peer-owned evidence obligations, not missing service seams.

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




## Peer-owned fixed read costs

Query growth remains flat at 5/50 owned rows. The remaining fixed gap is primarily
inside WS12's `agent_work::{find_owned,writable,update_work,set_result,handoff_work}`,
`ChannelThread::{create_board_post,update_work,update_result}` and their after-commit
board rendering. They repeat thread/room/owner membership/capability reads and complete
board render preloads per callback. WS12 owns those service/read-boundary optimizations;
this branch names them without changing their transaction or policy contract.
`work_threads.rs` and `presenters/boards.rs` remain untouched. WS15g/WS15e retain external
GitHub/Fizzy clients and account authorization. WS11-ui owns account/agent HTML pages.
