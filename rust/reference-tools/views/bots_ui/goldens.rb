# Exact bot UI pages from pinned Smartfire Rails, with fixtures and fixed CSRF/nonce values.
require "json"
require "ostruct"

ActiveRecord::Base.logger = nil

# `fixtures :all` with test_helper.rb's fixture classes.
require "active_record/fixtures"
fixtures = Rails.root.join("test/fixtures")
ActiveRecord::FixtureSet.create_fixtures(fixtures,
  Dir[fixtures.join("**/*.yml")].map { |path| path.delete_prefix("#{fixtures}/").delete_suffix(".yml") },
  { "twitter_posts" => Twitter::Post, "twitter_post_references" => Twitter::PostReference })

OUT = "/rails/storage/db/goldens.json"
HOST = "campfire.test"
USER_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"

class GoldenController < ApplicationController
  # "GLOBAL" for csrf_meta_tags, "<method>:<action>" for a form's per-form token.
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end

class GoldenSearchesController < SearchesController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end

  def self.controller_name = "searches"
end

def user(email) = User.find_by!(email_address: email)

def renderer_env(flash: {}, query: nil)
  {
    http_host: HOST,
    https: false,
    "HTTP_USER_AGENT" => USER_AGENT,
    "QUERY_STRING" => query.to_s,
    "rack.session" => {},
    "action_dispatch.content_security_policy" => Rails.application.config.content_security_policy,
    "action_dispatch.content_security_policy_nonce_generator" => ->(_request) { "NONCE" },
    "action_dispatch.request.flash_hash" => ActionDispatch::Flash::FlashHash.new(flash.stringify_keys)
  }
end

def render_with(controller: GoldenController, user: nil, flash: {}, query: nil, **options)
  Current.reset
  Current.user = user
  controller.renderer.new(renderer_env(flash:, query:)).render(**options)
ensure
  Current.reset
end



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
legacy = User.create_bot!(name: "Legacy <Bot>", webhook_url: "https://example.com/legacy")
edit_case(result, "edit_legacy", legacy, david)
# A missing agent must stay missing on these read-only pages.
raise "read created legacy agent" if legacy.reload.agent
result["pages"]["key"] = render_with(user: david, template: "accounts/bots/keys/show", layout: "application", assigns: { bot: bender, bot_key: "#{bender.id}-fixtureKey12" })
File.write("/rails/storage/db/bots-ui.json", JSON.pretty_generate(result) + "\n")
puts "Rails bot UI goldens: #{result.fetch('pages').size} pages"
