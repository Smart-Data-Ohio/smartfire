# WS12 agent work model APIs

Use `campfire_db::models::agent_work` from REST/MCP adapters. This module implements the shared Rails `Agents::BoardPosts`, `Agents::WorkThreads` and `Agents::WorkHandoffs` services, using the merged WS11 grants, budgets, ledger, delivery and presence code.

| Adapter operation | Model API | Success |
| --- | --- | --- |
| Board post list / `list_board_posts` | `agent_work::list_board_posts(conn, agent, room, status, owner, tag)` | `Outcome<Vec<ChannelThread>>`, 200 |
| Board post create / `create_board_post` | `agent_work::create_board_post(tx, agent, room, BoardPostInput)` | `Outcome<ChannelThread>`, 201 |
| Work list / `list_work` | `agent_work::list_work(conn, agent)` | `Outcome<Vec<ChannelThread>>`, 200 |
| Work show | `agent_work::show_work(conn, agent, id)` | `Outcome<ChannelThread>`, 200 |
| Work update / `update_work` / `update_board_post` | `agent_work::update_work(tx, agent, id, AgentWorkChanges)` | `Outcome<ChannelThread>`, 200 |
| Result write / `set_result` | `agent_work::set_result(tx, agent, id, Option<&Value>)` | `Outcome<ChannelThread>`, 200 |
| Handoff / `handoff_work` | `agent_work::handoff_work(tx, agent, id, receiver_agent_id, HandoffPackage, &audit_log::Context)` | `Outcome<HandedOffWork>`, 201 |
| Presence write / `set_presence` | Existing WS11 `agent_working_presence::set(tx, agent_id, text)` | Existing `ServiceResult` |
| Work/board JSON rendering | Existing WS11 `agent_payloads::work_payload(conn, thread, owner_id, &RepositoryAccess)` | `serde_json::Value` |

`Outcome::Denied(ServiceResult)` is a successfully completed database operation. Return `Ok(denied)` from the write closure, commit, and render its `status` and `failure_body()`. Budget denials write a notice; converting denial to a database error would discard that notice. Genuine database/job enqueue failures remain `Err` and roll back every source row and durable job.

Board callers retain the Rails room type, membership, read/post/manage grants and throttle checks. The service checks the board-post budget before validating fields. `BoardPostInput` accepts optional string title/body/status/run URL and raw JSON tags/owner. Use the adapter's existing Rails string-column casting when extracting those string fields. Work services own fresh ownership, membership and read access, then check `manage_threads` before field validation. List filters run before their 100-row cap.

`AgentWorkChanges` is exported from both `models::channel_thread` and the crate root. Each `Option<Value>` distinguishes omission (`None`) from explicit null (`Some(Value::Null)`). A missing result key is `None`; explicit null clears the result. Pass the actual JSON value to retain Rails `.to_s` semantics, Unicode limits and tag normalization. `HandoffPackage` is exported at the crate root and contains string summary plus raw JSON link/question collections. Pass request audit context for IP/user-agent snapshots.

The shared `ChannelThread::{update_work_by_agent, update_result_by_agent, hand_off}` methods recheck ownership under the writer lock and preserve Rails stale-instance/no-op rules. A human handoff caller must authorize its sender and call `WorkHandoff::receiver_error` before `hand_off`, as Rails does. Handoff history, audit, sender/receiver ledger snapshots and webhook jobs commit together. Human handoff controllers/pages are the next WS12 slice.

`BoardTagAssignment::{create, update, destroy, find, for_room}` supplies rule persistence and Rails validations. Rule configuration authorization remains with the board manager controller. Newly added tags register the after-commit callback automatically on thread creation/metadata/agent edits. It chooses only the first matching rule in tag order, rechecks current eligibility, and never replaces an assigned owner. Callback failure preserves the original committed post/tag write. Read the thread after the outer write returns when rendering a board response: auto-assignment has its own committed transaction.

WS12 supplies these domain APIs. WS11-API owns replacing REST/MCP flagged adapters; WS8b-m's ordinary work-pane/history/owner-picker rendering still requires the next WS12 page slice. No duplicate agent implementation is needed.
