# WS11 API remaining scope before cutover

Audit baseline: main `4fd0a74ccb0d596a0687557f3cc1f52a5a58ad08`, which merged #192.
This branch starts from that main; it does not merge or change #202's
`rust/ws11api-work-writes` (`05bdf0f7ce31af9d989ec99e9b31e37a6545f243`).
The Rails reference remains `d7c7de92`. This is a source/routing/service audit plus
the actual differential suites, not a claim that every imaginable input was tested.

## API implementation still pending on main

These are all reachable uses of `models::agent_api_pending::execute`: five operations
exposed as four REST actions and five MCP tools. Their denial/validation paths exist;
their valid writes still return generic REST500 or MCP -32603 and do not write.
WS12's services are present on main. All nine adapters are implemented and verified in
#202, including 244 response/state/job differentials, flat SELECT growth and atomic
rollback checks. They are awaiting review/merge, not waiting for a missing WS12 service.
The user explicitly instructed this worker to leave that branch alone; none is duplicated here.

| Remaining behavior on main | Interface | Reason it remains |
| --- | --- | --- |
| Create board post | POST /rooms/:room_id/agents/posts | Adapter awaiting #202 merge |
| Update owned work (status/note/tags/run URL) | PATCH /agents/work/:id | Adapter awaiting #202 merge |
| Write/clear result | PUT /agents/work/:id/result | Adapter awaiting #202 merge |
| Handoff owned work with context | POST /agents/work/:id/handoff | Adapter awaiting #202 merge |
| Create board post | MCP create_board_post | Adapter awaiting #202 merge |
| Update owned work | MCP update_work | Adapter awaiting #202 merge |
| Update board post | MCP update_board_post | Adapter awaiting #202 merge |
| Write/clear result | MCP set_result | Adapter awaiting #202 merge |
| Handoff owned work | MCP handoff_work | Adapter awaiting #202 merge |

The representation proxy's extra Content-Transfer-Encoding header was the only additional
confirmed unblocked implementation gap found in this audit. It is fixed on this branch
for healthy200 and handled empty404 responses, with six failing-first regressions and
a nine-header Rails oracle. Streaming blob proxies share the corrected helper. Disk
downloads and byte-range send_data responses retain their Rails transfer-encoding headers.

After this fix, no further unported WS11-API implementation was identified outside #202.
Only the #202 review/merge hold remains for API writes. No additional stand-in, fake success,
unregistered agent/bot job or dead transport path was found.

## Completed API behavior checked by the audit

The 35 non-MCP JSON actions in config/routes.rb all have controller bindings and boundary
vectors. Thirty-one use installed services; four are the #202 writes above. The six by-bot
actions and MCP POST/GET/DELETE transports have real bindings. All38 metadata tools have
explicit dispatch; five are #202 writes, the other33 call installed service/model adapters.

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
| Throttling | Per-credential/controller/action minute buckets, global MCP600 and per-tool limits, concurrency, exact429/Retry-After | agent_api.rs; surface/permission vectors; concurrency HTTP regressions |
| Agent tokens | Digest/constant-time auth, unknown/blank/revoked/expired/inactive states, fresh grants, no agent token on human endpoint | concerns.rs; agent_access/credential; authentication/HTTP/security vectors |
| Bot keys/reply tokens | Digest keys, plaintext scrub registered once per process, create-only room-bound15min replies, membership/deactivation, reset/show-once/sudo | concerns.rs; User::Bot domains; verifiers; by-bot HTTP tests and WS11-ui account controllers |
| Webhook transport | Timestamp/HMAC headers, encrypted signing secrets, SSRF/DNS/IP pinning,7s timeout, transient legacy retries, five-attempt agent backoff/Retry-After | integrations/{webhook,jobs,agent_jobs}.rs; agents_webhook_contract.json; real socket/claim tests |
| Webhook production paths | Message/slash/work/approval/action payloads, thread-aware sync replies, hop/fanout rules, durable jobs/claims/recovery and token scrubbing | agent_delivery/payloads/work_events; agent_jobs; jobs/periodic.rs; installed path/callback vectors |

Existing approved JPEG/video crash and committed-file-retention differences remain explicit
in ws11api-approved-differences.md. They are deliberate reviewed differences, not hidden
unported paths. Native media-byte/version differences still require the pinned runtime for
exact byte comparisons; sizes/checksums are never masked.

## Broader WS11 evidence still partial

The domain worker's tracked deferred-domain-cases.json retains81 exact named comparisons
across eight files. The domain inventory has378 source cases;297 are mapped comparisons.
An unmapped name is evidence debt, not proof that its production behavior is unimplemented.
No names are silently credited as closed by the API vectors, this header fix or a source grep.
The domain manifest stays unchanged; its owning workers reconcile those individual ports.

The old manifest note about absent HTTP credentials is stale as an API deferral: the real
Bearer parser and REST/MCP unauthenticated/invalid-token vectors already exercise that
boundary. Named domain comparisons for credentials have no unmapped names. Admin/key/
credential/grant/secret pages and human approval decisions are installed WS11-ui adapters;
they are separate from the agent JSON API. The domain report's two rendered status-broadcast
names remain named-evidence debt with WS11-ui, not missing agent REST endpoints.

The list below preserves every outstanding named case and its declared owner. The reason
is specified per file; items remain unclosed because the tracked named-case mapping lacks
that individual comparison, even when installed production code and broader vectors cover
the behavior. Dependencies now being on main does not itself close a named differential.

### test/models/agent_test.rb

Owner: WS11-ui rendered broadcast cases; WS11 domain cases compared.

Reason: Rendered badge/directory broadcast and secret-leak assertions belong to WS11-ui; not JSON transport work.

- status change broadcasts badge and directory row replaces to agents:all
- status broadcasts carry no credentials or grants

### test/services/slash_commands/dispatcher_test.rb

Owner: WS11 agent dispatch; WS8 built-in commands.

Reason: These are WS8 built-in command comparisons; agent custom registration/invocation is already installed and compared.

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

### test/jobs/agent/delivery_job_test.rb

Owner: WS11 domain.

Reason: The domain worker has not mapped the specific lock/rate and self-assignment hop comparisons; real writer/claim/hop tests are broader evidence.

- rate check and insert run inside the agent lock
- self-assigned work does not raise the agent's own hop count

### test/models/channel_thread_agent_assignment_test.rb

Owner: WS11 agent callbacks; WS12 mutation producers.

Reason: Individual WS11-ledger/WS12-producer comparisons are not mapped. Main has WS12 producers; #202 adds their HTTP writers but has not merged.

- an active member agent with post_messages is an eligible work owner
- a legacy agent keeps post eligibility through the fallback
- a suspended agent is rejected with a validation error
- a non-member agent is rejected with a validation error
- an agent without post_messages is rejected with a validation error
- a human outside the parent room keeps the human eligibility error
- a bot without an agent row is rejected with a validation error
- suspending the agent or revoking its membership reads as an unavailable owner
- assignment writes one work_assigned row in the same transaction as the work event
- unassignment writes one work_unassigned row
- unassignment after the agent left the room writes the ledger row but enqueues no webhook
- assignment webhooks are enqueued after the outermost transaction commits
- reassignment to a human writes work_unassigned and no work_assigned
- reassignment between agents notifies both
- status-only changes and human-to-human assignment write no agent rows
- assignment rows roll back when the work event fails
- the work event rolls back when the assignment row fails
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

Reason: Individual complete start/coalescing/finalization comparisons are not mapped. HTTP and installed peer-callback/stream tests are broader evidence; peers own their side effects.

- stream start broadcasts the append without the unread broadcast
- finalize fires every side effect exactly once
- appends re-render without firing side effects
- a coalesced update sends a trailing broadcast with the final text

### test/models/user/bot_test.rb

Owner: WS11 bot domain/removal; WS11-api by-bot HTTP surface.

Reason: Two fixed-random key-generation cases, three named reply-token model cases and one queued webhook execution case are not mapped. Digest/reply/HTTP/real-delivery behavior already has separate contracts.

- create bot
- reset bot key
- reply token authenticates its bot for its room only
- reply token expires
- reply token is refused once the bot leaves or deactivates
- deliver message by webhook

### test/models/agent_budgets_test.rb

Owner: WS11 domain.

Reason: Individual activity-viewer and capped-handoff comparisons are not mapped; WS12 owns the producers/viewer domain, and #202 supplies the handoff HTTP writer.

- a stranger cannot read another agent's budget item
- an agent-created handoff counts toward no budget, even with caps exhausted

### test/models/agent_kill_switch_test.rb

Owner: WS11 callbacks; WS12 owned-board mutation producer.

Reason: The exact owned-work-not-reassigned comparison is not mapped; WS11 lifecycle and WS12 owned-work producer evidence must be reconciled by their owners.

- kill switch leaves owned work assigned without reassigning it

## Peer-owned items, not WS11 API gaps

- WS12's /work.json query-growth fix is in #201. It is the human work-pane endpoint;
  this branch does not touch work_threads.rs or presenters/boards.rs.
- External GitHub/Fizzy access/action execution belongs to WS15g/WS15e. Main's real
  clients and authorization APIs are called; they are not replaced with local approximations.
- Agent HTML, accounts/bot administration and rendered agent broadcasts belong to WS11-ui.
  Their outstanding named comparisons above stay visible instead of being mislabeled
  missing MCP/REST functionality.

This checkpoint ports the only newly confirmed unblocked API gap, the proxy headers.
The remaining API writes are already implemented in #202 and held for its review/merge.
Broader named-case evidence remains partial with the explicit owners above; no claim is
made that the entire WS11 domain is cutover-complete.
