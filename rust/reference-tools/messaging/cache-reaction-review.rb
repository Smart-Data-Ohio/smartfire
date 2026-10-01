# Review regressions recorded from pinned Rails, using actual room/mutation requests.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ApplicationController.allow_forgery_protection = false
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
Rails.application.config.hosts.clear
Rails.cache = ActiveSupport::Cache::MemoryStore.new
room = Room.find(486777696)
user = User.find(127326141)
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
source = room.root_messages.create!(creator: user, markdown_source: "Review before edit", client_message_id: "review-cache-source")
reply = room.root_messages.create!(creator: User.find(149087659), markdown_source: "Review reply", reply_to_message: source, client_message_id: "review-cache-reply")
renderer = ApplicationController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
helper = ApplicationController.helpers
controller = MessagesController.new
states = []
capture = ->(name) do
  browser.get("/rooms/#{room.id}", headers:)
  raise "room status #{browser.response.status}" unless browser.response.successful?
  Rails.application.executor.wrap do
    reply.reload
    Message.preload_rendering_details([reply])
    html = renderer.render(partial: "messages/message", locals: { message: reply })
    raise "room did not mount reply" unless browser.response.body.include?(html)
    # Keep both Rails compositions exact: the collection helper, and index's page validator.
    collection_key = ActiveSupport::Cache.expand_cache_key(helper.message_with_pr_cards_cache_key(reply))
    validator = ActiveSupport::Cache.expand_cache_key([
      [reply, source.reload], controller.send(:rendered_related_stamp, [reply, source]),
      controller.send(:rendered_pin_stamp, [reply, source]), MessagesHelper::PRESENTATION_CACHE_VERSION
    ])
    states << { name:, time: Time.current.iso8601(6), collection_key:, validator:, html: }
  end
end
capture.call("initial")
travel_to Time.utc(2026, 3, 2, 16) + 0.125.seconds, with_usec: true
browser.patch("/rooms/#{room.id}/messages/#{source.id}.json", params: { message: { markdown_source: "Review after edit" } }, headers:, as: :json)
raise "edit status #{browser.response.status}" unless browser.response.successful?
capture.call("source_edited")
travel_to Time.utc(2026, 3, 2, 16) + 0.250.seconds, with_usec: true
browser.patch(Rails.application.routes.url_helpers.user_profile_path, params: { user: { name: "Review renamed author" } }, headers:)
raise "profile status #{browser.response.status}" unless browser.response.redirect?
capture.call("author_renamed")

reactions = ["\u00a0👍\u00a0", "\u2003👍\u2003", "\u202f👍\u202f", "\u3000:thumbsup:\u3000", "\0 \t\n\r\v\f👍\f\v\r\n\t \0", " :thumbsup: ", " :gpt: ", " :does_not_exist: "].map do |content|
  source.reload.boosts.destroy_all
  resolved = Boost.resolve_content(content)
  counts = 2.times.map do
    browser.post("/messages/#{source.id}/boosts", params: { boost: { content: } }, headers:, as: :json)
    raise "boost status #{browser.response.status}" unless browser.response.redirect?
    source.boosts.count
  end
  { content:, resolved:, reaction: Boost.reaction?(resolved), counts:, stored: source.boosts.order(:id).pluck(:content) }
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", source_id: source.id, reply_id: reply.id, states:, reactions:) + "\n")
puts "WS8bma review oracle: #{states.size} real room states; #{reactions.size} reaction pairs; Rails collection keys and page validators"
