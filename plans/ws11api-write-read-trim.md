# WS11 write read-cost follow-up (#205)

Complete SELECT counts at 5/50 owned rows, using the existing paired work-write
fixtures and both reader and writer traces:

| Path | This round's pre-trim Rust | Rust after | Pinned Rails |
| --- | --- | --- | --- |
| REST create | 189/189 | 169/169 | 60/60 |
| MCP create | 177/177 | 167/167 | 61/61 |
| REST update | 83/83 | 83/83 | 33/33 |
| MCP update / update_board_post | 83/83 | 83/83 | 34/34 |
| REST result | 33/33 | 33/33 | 26/26 |
| MCP result | 33/33 | 33/33 | 27/27 |
| REST handoff | 105/105 | 94/94 | 55/55 |
| MCP handoff | 105/105 | 94/94 | 56/56 |

Astra's earlier independent fixture had 198/186 create reads. This table uses
this round's actual pre-trim measurements after the main update and typed preload
work, not a substituted historical baseline. Counts are physical SQLite statement
traces for the entire measured request. Rails work-vector boundaries are unchanged.

REST create's twelve adapter preflight reads become two: the member room is
loaded once and reused for its board check; both current-user capabilities are
loaded together. The writer still checks membership and current live capabilities,
and obtains the complete selected room directly instead of querying ID then row.

WS11 ledger creation returns the stored INSERT row, retaining database defaults
and validation. Webhook eligibility reads current agent/user/membership/grant/room
and webhook facts in one statement. The conditional status update changes the
returned event only when it succeeds, and queues the same job in the same write.
There is no subsequent ledger reload. Message hop calculation reuses the sender
agent found on the same writer connection, and assignment/handoff callbacks load
the actor once for their events. No grants, ledger/audit writes or queue policy
are duplicated, and no permission fact is cached across writer boundaries.

The stricter current-state checks and private-account final sealing approved in
ws11api-approved-differences.md remain intact. The existing 244 literal
response/state/job vectors pass. The tightened fixed-cost regression fails before
these removals, then passes at both sizes. A separate nine-state/seven-capability
comparison checks the batch against the existing current-user policy, including
revocation, suspension, inactive users, missing/deleted rooms and human callers.

Four authentication/ban reads, current adapter identities, repository-candidate
selection and committed work payload reads remain WS11-owned and required. The
remaining larger fixed costs in `agent_work`, ChannelThread work producers, board
preloads/tag callbacks and history/activity are WS12-owned. This change does not
touch `work_threads.rs`, `presenters/boards.rs`, WS12 writers or their contracts.
