# Exact human approval and activity pages/cards from our pinned Rails.
require_relative "../bots_ui/setup"
bot = User.find_by!(name: "Bender Bot")
agent = bot.agent
admin = User.find_by!(email_address: "david@37signals.com")
owner = User.find_by!(email_address: "kevin@37signals.com")
agent.update!(owner:)
room = Room.find_by!(name: "All Talk")
result = { pages: {}, cards: {}, facts: {}, now: Time.current.iso8601, bot_id: bot.id, bot_name: bot.name, agent_id: agent.id }

def approval_facts(a, viewer)
  Current.user = viewer
  { id: a.id, bot_name: a.agent.user.name, avatar_url: render_with(inline: "<%= fresh_user_avatar_path(bot) %>", locals: { bot: a.agent.user }),
    room_name: a.room && ApplicationController.helpers.room_display_name(a.room), action: a.action, summary: a.summary,
    status: a.effective_status, expires_at: a.expires_at.iso8601, decided_by: a.decided_by&.name, decision_note: a.decision_note.presence,
    github_login: a.github_login.presence, fizzy_user_name: a.fizzy_user_name.presence, approvable: a.approvable_by?(viewer) }
end
def history_case(result, name, agent, viewer, approvals, filter: nil, page: 1, has_next: false)
  result[:facts][name] = { viewer: viewer.name, approvals: approvals.map { |a| approval_facts(a, viewer) }, filter:, page:, has_next: }
  result[:pages][name] = render_with(user: viewer, template: "agents/approvals/for_agent", layout: "application", assigns: { agent:, bot: agent.user, approvals:, status_filter: filter, page:, has_next: })
end
def card_case(result, name, approval, viewer)
  result[:facts][name] = { viewer: viewer.name, approval: approval_facts(approval, viewer) }
  result[:cards][name] = render_with(user: viewer, partial: "agent_approvals/card", locals: { approval: })
end
agent.agent_approvals.destroy_all
history_case(result, "approvals_empty", agent, admin, [])
generic = AgentApproval.create!(agent:, room:, action: "deploy", summary: "Deploy <this> & wait", expires_at: 1.hour.from_now)
github = AgentApproval.create!(agent:, room:, action: "github.comment", summary: "Comment on the pull request", github_login: "agent-octocat", github_account_id: 123)
fizzy = AgentApproval.create!(agent:, action: "fizzy.comment", summary: "Add a card comment", fizzy_user_name: "Pat & Lee", fizzy_user_id: "pat", fizzy_connected_account_id: 456)
card_case(result, "card_generic_admin", generic, admin)
card_case(result, "card_generic_owner", generic, owner)
card_case(result, "card_github_admin", github, admin)
card_case(result, "card_github_owner", github, owner)
card_case(result, "card_fizzy_owner", fizzy, owner)
history_case(result, "approvals_admin", agent, admin, [fizzy, github, generic], filter: "pending", has_next: true)
history_case(result, "approvals_owner", agent, owner, [github, generic], page: 2, has_next: true)
generic.decide!(decision: "denied", by: admin, note: "Needs <review> & retry")
card_case(result, "card_denied", generic, owner)
approved = AgentApproval.create!(agent:, action: "release", summary: "Ship it")
approved.decide!(decision: "approved", by: owner)
card_case(result, "card_approved", approved, admin)
cancelled = AgentApproval.create!(agent:, action: "cancel", summary: "No longer needed")
cancelled.cancel_by_agent!
card_case(result, "card_cancelled", cancelled, admin)
expired = AgentApproval.create!(agent:, action: "expire", summary: "Too late")
expired.update_columns(expires_at: 1.minute.ago)
expired.expire_if_due!
card_case(result, "card_expired", expired, owner)
history_case(result, "approvals_settled", agent, admin, [expired, cancelled, approved, generic], filter: "denied", page: 3)

def event_facts(agent, e, viewer)
  Current.user = viewer
  h = ApplicationController.helpers
  metadata = e.metadata.is_a?(Hash) ? e.metadata : {}
  { id: e.id, event_type: e.event_type, outcome: e.outcome, created_at: e.created_at.iso8601,
    room_name: e.room && h.room_display_name(e.room), actor_name: e.actor&.name, message_id: e.message_id, hop: e.hop,
    detail: e.detail.presence, webhook_status: e.webhook_status, webhook_attempts: e.webhook_attempts.to_i, webhook_last_error: e.webhook_last_error.presence,
    external_metadata: e.metadata.is_a?(Hash), external_action: metadata["action"]&.to_s, external_status: metadata["status"]&.to_s, external_message: metadata["message"].presence&.to_s,
    handoff_summary: e.event_type == "work_handed_off" && metadata["handoff"].is_a?(Hash) ? metadata["handoff"]["summary"].to_s : nil,
    content: h.agent_event_content_visible?(agent, e, viewer) ? e.message.plain_text_body : nil }
end
def ledger_case(result, name, agent, viewer, events, filter: nil, page: 1, has_next: false)
  result[:facts][name] = { viewer: viewer.name, events: events.map { |e| event_facts(agent, e, viewer) }, filter:, page:, has_next: }
  result[:pages][name] = render_with(user: viewer, template: "agents/events/ledger", layout: "application", assigns: { agent:, bot: agent.user, events:, outcome_filter: filter, page:, has_next: })
end
ledger_case(result, "ledger_empty", agent, admin, [])
message = Message.find_by!(room:, creator: admin)
Membership.find_or_create_by!(room:, user: bot)
normal = agent.agent_events.create!(event_type: "mention", room:, message:, actor: admin, outcome: "delivered", hop: 2, detail: "Delivered <safely> & recorded", webhook_status: "failed", webhook_attempts: 2, webhook_last_error: "Timeout & retry")
github_event = agent.agent_events.create!(event_type: "github_action_completed", outcome: "delivered", metadata: { action: "comment", status: "failed", message: "API <unavailable>" })
fizzy_event = agent.agent_events.create!(event_type: "fizzy_action_completed", outcome: "acknowledged", webhook_status: "delivered", webhook_attempts: 1, metadata: { action: "close", status: "succeeded" })
handoff = agent.agent_events.create!(event_type: "work_handed_off", room:, outcome: "pending", metadata: { handoff: { summary: "More work <next> " * 20 } })
suppressed = agent.agent_events.create!(event_type: "delivery_suppressed_revoked", room:, outcome: "suppressed", detail: "Read capability revoked")
ledger_case(result, "ledger_admin", agent, admin, [suppressed, handoff, fizzy_event, github_event, normal], filter: "delivered", has_next: true)
ledger_case(result, "ledger_owner", agent, owner, [normal, handoff], page: 2, has_next: true)
AgentGrant.create!(agent:, granted_by: admin, capability: "post_messages")
ledger_case(result, "ledger_redacted", agent, admin, [normal], filter: "suppressed", page: 3)
File.write("/rails/storage/db/agent-history-ui.json", JSON.pretty_generate(result) + "\n")
puts "Rails agent history UI goldens: #{result[:pages].size} pages, #{result[:cards].size} cards"
