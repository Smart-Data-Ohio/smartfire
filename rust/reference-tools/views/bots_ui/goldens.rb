# Pinned Smartfire Rails bot page states.
require_relative "setup"

bots = User.active_bots.ordered.includes(agent: :owner).to_a
bender = User.find_by!(name: "Bender Bot")
david = User.find_by!(email_address: "david@37signals.com")
kevin = User.find_by!(email_address: "kevin@37signals.com")
result = { "pages" => {}, "facts" => {} }
result["pages"]["index"] = render_with(user: david, template: "accounts/bots/index", layout: "application", assigns: { bots: bots })
result["pages"]["new"] = render_with(user: david, template: "accounts/bots/new", layout: "application", assigns: { bot: User.active_bots.new })
result["facts"]["bots"] = bots.map do |bot|
  {
    id: bot.id, name: bot.name, avatar_path: render_with(inline: "<%= fresh_user_avatar_path(bot) %>", locals: { bot: }),
    kind: bot.agent&.kind, owner_name: bot.agent&.owner&.name,
    rooms: bot.rooms.without_directs.ordered.map { |room| { id: room.id, name: ApplicationController.helpers.room_display_name(room) } }
  }
end

def form_facts(bot)
  agent = bot.agent
  account = bot.github_connected_account
  {
    id: bot.id, name: bot.name, webhook_url: bot.webhook_url, icon_name: bot.icon_name,
    error_fields: bot.errors.attribute_names, agent_error_fields: agent&.errors&.attribute_names,
    errors: bot.errors.full_messages.to_sentence.presence, agent_errors: agent&.errors&.full_messages&.to_sentence&.presence,
    avatar_attachment_url: bot.avatar.attached? ? render_with(inline: "<%= url_for(bot.avatar) %>", locals: { bot: }) : nil,
    agent: agent && agent.attributes.slice("id", "owner_id", "provider", "runtime", "description", "daily_message_cap", "daily_board_post_cap", "daily_external_action_cap", "suspended_at"),
    budget_usage_line: agent && ApplicationController.helpers.agent_budget_usage_line(agent),
    signing_secret: agent&.webhook_signing_secret || bot.webhook&.signing_secret,
    github: account && { usable: account.usable?, login: account.github_login, disconnected_reason: account.disconnected_reason }
  }
end

def edit_case(result, name, bot, viewer)
  result["facts"][name] = form_facts(bot)
  result["pages"][name] = render_with(user: viewer, template: "accounts/bots/edit", layout: "application", assigns: { bot: bot, agent: bot.agent })
end

edit_case(result, "edit_admin", bender, david)
bender.agent.update!(owner: kevin, provider: "OpenAI <Provider>", runtime: "Codex & CLI", description: "Work\n<script>escaped</script>", daily_message_cap: 12)
bender.update!(icon_name: "openai")
edit_case(result, "edit_owner", bender.reload, kevin)
bender.agent.update!(suspended_at: Time.current, webhook_signing_secret: "fixture-webhook-signing-value")
edit_case(result, "edit_suspended", bender.reload, david)
account = bender.create_github_connected_account!(github_login: "fixture-login", access_token: "fixture-github-token")
edit_case(result, "edit_github_connected_admin", bender.reload, david)
edit_case(result, "edit_github_connected_owner", bender, kevin)
account.update!(disconnected_reason: "Token <rejected>")
edit_case(result, "edit_github_rejected", bender.reload, david)
account.update_columns(disconnected_reason: nil, access_token: "invalid-encrypted-fixture")
edit_case(result, "edit_github_unreadable", bender.reload, david)
invalid = User.active_bots.new(name: "Invalid <Bot>", icon_name: ":notanicon:")
invalid.valid?
result["facts"]["new_invalid"] = form_facts(invalid)
result["pages"]["new_invalid"] = render_with(user: david, template: "accounts/bots/new", layout: "application", assigns: { bot: invalid })
bender.agent.description = "x" * 501
bender.agent.valid?
edit_case(result, "edit_invalid_agent", bender, david)
# Rejected cap fields retain before-type-cast values in Rails number inputs.
[" 12 ", "12.5", "abc", true, false].each_with_index do |input, index|
  bender.reload
  bender.agent.assign_attributes(daily_message_cap: input)
  bender.agent.valid?
  name = "edit_invalid_raw_cap_#{index}"
  edit_case(result, name, bender, david)
  result["facts"][name]["raw_caps"] = { "messages" => bender.agent.daily_message_cap_before_type_cast }
end
legacy = User.create_bot!(name: "Legacy <Bot>", webhook_url: "https://example.com/legacy")
edit_case(result, "edit_legacy", legacy, david)
# A missing agent must stay missing on these read-only pages.
raise "read created legacy agent" if legacy.reload.agent
result["pages"]["key"] = render_with(user: david, template: "accounts/bots/keys/show", layout: "application", assigns: { bot: bender, bot_key: "#{bender.id}-fixtureKey12" })
File.write("/rails/storage/db/bots-ui.json", JSON.pretty_generate(result) + "\n")
puts "Rails bot UI goldens: #{result.fetch('pages').size} pages"
