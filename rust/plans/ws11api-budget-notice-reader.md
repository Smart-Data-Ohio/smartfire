# Typed AgentBudgetNotice owner reader

WS11 exports `campfire_db::AgentBudgetNotice` with typed IDs, cap, civil day and
persisted timestamps; `find`, optional `find_by_id`, and one-bind `for_ids` reads.
`agent`, `cap_label`, `budget_limit`, `budget_limit_for_agent`, and
`activity_recipient_ids` expose the Rails reader policy. The preloaded limit
method rejects a different agent. Current existing owners remain recipients even
when inactive or bots; only the ownerless fallback filters active human admins.

`budget_notice_reader.rb` captures all three caps for owner, inactive owner, bot
owner and ownerless audiences against d7c7de92. Twelve complete typed rows and
current recipients are compared without masks. A wrong board-post label in that
oracle causes the committed reader regression to fail, then the exact oracle
bytes are restored. Existing inbox HTTP goldens check the HTML and JSON bytes.

`presenters/activity.rs` now loads notices, agents and actor users in three
bounded-bind batches. The prior read-only SQL fact reader in path, HTML card and
JSON source is removed. Both HTML and JSON preloads cost 3/3 SELECTs at 5/50 rows.
No activity eligibility, recording, write/fanout or grant logic changes here.

WS12 owns adding this typed source to `ActivityItems::Recorder`; WS8b-m2 can use
this public owner API in its activity producer/presentation work. This branch
exports the reader and replaces its own presenter seams rather than duplicating
those producers.

The reader is a read-side contract. It does not introduce a new budget-recording
API or override WS12's source eligibility/recorder service.
