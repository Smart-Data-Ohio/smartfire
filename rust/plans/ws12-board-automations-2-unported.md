# WS12 cutover inventory at the board automation checkpoint

The automation features in this round are implemented, including settings, tag-rule HTTP writes,
SLA/digest sweeps, scheduler registration, source/job atomicity and cleanup. The older
`ws12-board-automations-checkpoint.md` description of digest recipient, timezone, weekend and
quiet-hour features was incorrect: pinned Rails posts one quiet root system note per board/day.
Those invented features are not deferred work. The recurring source is
`app/services/periodic/runner.rb`; this Rails tree has no `config/recurring.yml`.

## Remaining functional scope

| Rails files | Still unported behaviour | Reason and owner |
| --- | --- | --- |
| `app/services/activity_items/recorder.rb` | The public generic recorder does not yet accept persisted source types beyond Message, WorkThreadEvent and BoardSlaNudge. SavedItem, Event, HuddleGrant, AgentApproval, AgentBudgetNotice, ScheduledMessage, Session, TwoFactorCredential and arbitrary persisted polymorphic sources under `skip_source_check` need an explicit typed source/authorization interface and the full recipient/idempotency contract. | WS12-owned continuation. Current owning-domain writers already record their inbox rows; this is the remaining shared API/integration gap, rather than an absent inbox feature. Preserve WS13 huddle ringing/refresh callbacks and WS11 budget fanout instead of duplicating them. |
| `app/helpers/activity_items_helper.rb`, `app/controllers/activity_items_controller.rb` and matching inbox views | AgentBudgetNotice facts still use three flagged read-only SQL seams in the merged activity presenter (HTML preload/card and JSON source). | WS11 must export a typed budget-notice reader; WS12/WS11-UI then replace these seams and verify complete inbox output. BoardSlaNudge seams are already closed by #200's model API. |

This is **not an owner-blocked-only stop**: the generic recorder continuation remains WS12-owned.
There is no new automation dependency or permissions question for the lead.

## Complete owned Rails file audit

| Owned Rails surface | Current implementation / remaining boundary |
| --- | --- |
| `app/models/rooms/board.rb`, `app/controllers/rooms/boards_controller.rb`, board room/list views | Board STI, explicit memberships, create/update policies and audits, pagination/filtering/list/pane HTML: existing WS12 slices on main. No missing production behaviour identified in this audit. |
| `app/models/board_tag_assignment.rb`, `app/models/thread_tag.rb`, board post mutations and result/header views | Tag normalization, eligibility, lexical assignment, no owner override, callbacks, title/tags/result/work writes: existing WS12 domain. This round adds the automation configuration HTTP page and tag add/remove audits. |
| `app/models/board_sla_rule.rb`, `app/controllers/rooms/boards/automations_controller.rb`, `app/views/rooms/boards/automations/show.html.erb` | This round: numericality/status/room/uniqueness validation, dirty-column timer updates, all-status form validation, omission/clear/no-op/error behaviour, creator/admin after membership checks, tag and timer audits, complete rendered settings bytes. |
| `app/models/board_sla_nudge.rb` | Merged #200 model/reader/atomic claim API retained; this branch fixes ID-0 validation and shares the source recorder with the SLA dispatcher. |
| `app/models/board_stale_digest.rb` | This round: per-board/day DB claim, optional message association, retained claim on posting failure and separate association update. |
| `app/models/board_automations/sla_dispatcher.rb`, `digest_dispatcher.rb`; board entries in `app/services/periodic/runner.rb` | This round: thresholds, active human membership/agent owner fallback, crossing/stage claims, push dedupe, isolated failures, daily quiet note, ordering/20-item cap/escaping/ages, no repeat writes, 5-minute/hourly wiring using the injected app clock. |
| `app/models/board_automations/nudge_pusher.rb`, `app/jobs/board_automations/nudge_push_job.rb` | Existing WS17 source/policy/delivery implementation; this round calls its actual typed BoardNudgeJob and tests source/inbox/queue rollback. No duplicate push implementation. |
| `app/models/work_handoff.rb`, `work_thread_event.rb`, `work_thread_link.rb`; `app/controllers/work_threads_controller.rb`, `threads/work/{handoffs,links}_controller.rb`; `app/helpers/work_thread_links_helper.rb` and matching pages | Existing WS12 model/controller/view slices: human rights, stale updates, history/audits, packages, all link kinds, credentialed Drive lookup, PR claim/job integration, work listing and complete responses. #201's reviewed query batching, bounded loaders and asset-manifest brand paths are now on main and retained by this branch's merge. |
| `app/models/activity_item.rb`, `app/channels/activity_channel.rb` | Merged domain authorization, cursors, read/handled state, dirty-column writes, snapshot broadcasts and all-source inbox access. Generic recorder and budget reader boundaries are listed above. |
| `app/models/{user_star,user/starring}.rb`, `app/controllers/users/stars_controller.rb`, matching member/sidebar/phone views | Existing stars slice and reviewed #181 versions remain on main. No missing production behaviour identified. |
| `app/models/agents/work_payload.rb`, `app/services/agents/{board_posts,work_handoffs,work_threads,working_presence}.rb`, `app/controllers/agents/work_controller.rb` | Shared WS12 services consume the real WS11 models. #192/#202 own and implement REST/MCP/controller adapters on main. No local duplication or remaining flagged board/work writer identified. |
| `app/javascript/controllers/{activity_inbox,activity_indicator,board_list}_controller.js` | Original asset sources are used by the Rust asset bundle; server contracts are covered by domain/HTTP vectors. No pixel acceptance phase remains. |
| `app/jobs/room/destroy_job.rb` board dependencies | Shared WS8 destruction API retained; new pinned cleanup outcomes cover nudges, tags, rules, digests, notes, inbox sources, scheduled messages, polls, options and votes. |

`ws12-rails-cases.json` is the full 488-declaration assertion ledger. Its remaining deferred
entries are explicitly **unreconciled original assertion sets**, not a count of absent production
features. Earlier reports deferred agent/inbox surfaces before their peer PRs merged; that old
status must not be used to claim those features are still missing. Detailed named-case
reconciliation and any uncovered behavioural assertion remain WS12 validation work. Current
automation model/controller/cleanup declarations are mapped to this round's tests and vectors.
