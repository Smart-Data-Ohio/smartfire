# Thread activity/name are not rendered by message partials; parent counts are.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ApplicationController.allow_forgery_protection = false
Rails.application.config.hosts.clear
Rails.cache = ActiveSupport::Cache::MemoryStore.new
ActionView::PartialRenderer.collection_cache = Rails.cache
ApplicationController.perform_caching = true
travel_to Time.utc(2026, 3, 2, 16)
room = Room.find(486777696)
david = User.find(127326141)
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = david.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
renderer = ApplicationController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
parent = room.root_messages.create!(creator: david, markdown_source: "Thread parent", client_message_id: "thread-cache-parent")
thread = ChannelThread.create!(room:, creator: david, parent_message: parent, name: "Cache thread")
replies = 2.times.map do |i|
  thread.post_message!(creator: david, attributes: { markdown_source: "Existing reply #{i}", client_message_id: "thread-cache-reply-#{i}" })
end
capture = ->(name) do
  pages = ["/rooms/#{room.id}", "/rooms/#{room.id}/threads/#{thread.id}/messages"].map do |path|
    browser.get(path, headers:)
    raise "status #{browser.response.status}" unless browser.response.successful?
    browser.response.body
  end
  Rails.application.executor.wrap do
    messages = [parent, *replies].map do |message|
      message.reload
      Message.preload_rendering_details([message])
      html = renderer.render(partial: "messages/message", locals: { message: })
      raise "missing shared partial" unless pages.any? { |body| body.include?(html) }
      { id: message.id, key: ActiveSupport::Cache.expand_cache_key(ApplicationController.helpers.message_with_pr_cards_cache_key(message)), html: }
    end
    { name:, thread_updated_at: thread.reload.updated_at.iso8601(6), messages: }
  end
end
states = [capture.call("before")]
travel_to Time.utc(2026, 3, 2, 16, 0, 2)
browser.post("/rooms/#{room.id}/threads/#{thread.id}/messages.json", params: { message: { markdown_source: "Another reply", client_message_id: "thread-cache-another" } }, headers:, as: :json)
raise "post failed #{browser.response.status}" unless browser.response.successful?
raise "no post" unless thread.messages.exists?(client_message_id: "thread-cache-another")
states << capture.call("posted")
travel_to Time.utc(2026, 3, 2, 16, 0, 4)
thread.update!(name: "Renamed cache thread")
states << capture.call("renamed")
raise "thread stamp unchanged" unless states.map { |state| state[:thread_updated_at] }.uniq.size == 3
raise "replies changed" unless states.map { |state| state[:messages].drop(1) }.uniq.size == 1
raise "parent rename changed" unless states[1][:messages].first == states[2][:messages].first
raise "parent count unchanged" if states[0][:messages].first == states[1][:messages].first
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], thread_id: thread.id, states:) + "\n")
puts "WS8bm thread cache stability: replies retain Rails helper keys/HTML after post and rename; parent count refreshes"
