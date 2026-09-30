# Agent credential/grant management pages from pinned Smartfire Rails.
require_relative "../bots_ui/setup"
bot = User.find_by!(name: "Bender Bot")
agent = bot.agent
david = User.find_by!(email_address: "david@37signals.com")
kevin = User.find_by!(email_address: "kevin@37signals.com")
result = { pages: {}, facts: {}, now: Time.current.iso8601, bot_id: bot.id, bot_name: bot.name }

def credential_facts(credential)
  { id: credential.id, name: credential.name, last_four: credential.token_last_four,
    created_by: credential.created_by.name, created_at: credential.created_at.iso8601,
    expires_at: credential.expires_at&.iso8601, last_used_at: credential.last_used_at&.iso8601,
    revoked: credential.revoked? }
end

def credentials_case(result, name, bot, viewer, form)
  credentials = bot.agent.agent_credentials.order(created_at: :desc).to_a
  result[:facts][name] = { credentials: credentials.map { |c| credential_facts(c) },
    name: form.name, expires_at: form.expires_at&.strftime("%Y-%m-%dT%H:%M:%S"),
    errors: form.errors.full_messages.to_sentence.presence, error_fields: form.errors.attribute_names }
  result[:pages][name] = render_with(user: viewer, template: "accounts/bots/credentials/index", layout: "application", assigns: { bot:, credentials:, credential: form })
end
credentials_case(result, "credentials_admin", bot, david, AgentCredential.new)
agent.update!(owner: kevin)
credentials_case(result, "credentials_owner", bot, kevin, AgentCredential.new)
expired, = AgentCredential.create_with_secret!(agent:, name: "Expired <runner>", created_by: david, expires_at: 1.day.ago)
expired.update!(token_last_four: "e123")
revoked, = AgentCredential.create_with_secret!(agent:, name: "Revoked & runner", created_by: david)
revoked.update!(token_last_four: "r123")
revoked.revoke!
used, = AgentCredential.create_with_secret!(agent:, name: "Used runner", created_by: david, expires_at: 2.days.from_now)
used.update!(last_used_at: 2.hours.ago, token_last_four: "u123")
credentials_case(result, "credentials_states", bot, david, AgentCredential.new(expires_at: 1.day.from_now))
invalid = AgentCredential.new(agent:, name: "", created_by: david, token_digest: "fixture-digest", token_last_four: "abcd")
invalid.valid?
credentials_case(result, "credentials_invalid", bot, david, invalid)
result[:pages]["credential_show"] = render_with(user: david, template: "accounts/bots/credentials/show", layout: "application", assigns: { bot:, plain_secret: "fixture-reveal-once-secret" })
agent.agent_credentials.delete_all
credentials_case(result, "credentials_empty", bot, david, AgentCredential.new)

def grants_case(result, name, bot, viewer, form)
  Current.user = viewer
  grants = bot.agent.agent_grants.includes(:room, :granted_by).order(:revoked_at, :capability, :room_id).to_a
  room_name = ->(room) { ApplicationController.helpers.room_display_name(room) }
  result[:facts][name] = { legacy: bot.agent.legacy_capabilities?, capability: form.capability, room_id: form.room_id&.to_s,
    errors: form.errors.full_messages.to_sentence.presence, error_fields: form.errors.attribute_names,
    rooms: bot.rooms.ordered.map { |r| [room_name.call(r), r.id.to_s] },
    grants: grants.map { |g| { id: g.id, capability: g.capability, room_name: g.workspace_wide? ? "Workspace-wide" : (g.room ? room_name.call(g.room) : "Deleted room"),
      granted_by: g.granted_by.name, created_at: g.created_at.iso8601, revoked: g.revoked? } } }
  result[:pages][name] = render_with(user: viewer, template: "accounts/bots/grants/index", layout: "application", assigns: { bot:, agent: bot.agent, grants:, grant: form })
end
grants_case(result, "grants_legacy", bot, david, AgentGrant.new)
room = Room.find_by!(name: "All Talk")
AgentGrant.create!(agent:, granted_by: david, capability: "post_messages", room:)
AgentGrant.create!(agent:, granted_by: david, capability: "external_action")
revoked = AgentGrant.create!(agent:, granted_by: david, capability: "react")
revoked.revoke!
grants_case(result, "grants_admin", bot, david, AgentGrant.new)
grants_case(result, "grants_owner", bot, kevin, AgentGrant.new)
invalid = AgentGrant.new(agent:, granted_by: david, capability: "dm_anyone", room:)
invalid.valid?
grants_case(result, "grants_invalid", bot, david, invalid)
File.write("/rails/storage/db/bot-access-ui.json", JSON.pretty_generate(result) + "\n")
puts "Rails bot access UI goldens: #{result[:pages].size} pages"
