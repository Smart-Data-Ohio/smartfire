# Unrelated writes preserve the existing message's Rails helper key and HTML.
require "json"
require "tempfile"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ApplicationController.allow_forgery_protection = false
Rails.application.config.hosts.clear
Rails.cache = ActiveSupport::Cache::MemoryStore.new
ActionView::PartialRenderer.collection_cache = Rails.cache
ApplicationController.perform_caching = true
room = Room.find(486777696)
david = User.find(127326141)
jason = User.find(149087659)
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = david.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
renderer = ApplicationController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
capture = ->(message) do
  ["/rooms/#{room.id}", "/rooms/#{room.id}/messages"].each do |path|
    browser.get(path, headers:)
    raise "status #{browser.response.status}" unless browser.response.successful?
  end
  Rails.application.executor.wrap do
    message.reload
    Message.preload_rendering_details([message])
    html = renderer.render(partial: "messages/message", locals: { message: })
    raise "missing shared partial" unless browser.response.body.include?(html)
    { html:, key: ActiveSupport::Cache.expand_cache_key(ApplicationController.helpers.message_with_pr_cards_cache_key(message)) }
  end
end
rows = %w[unrelated_post unused_icon_upload].map do |name|
  result = nil
  ActiveRecord::Base.transaction do
    travel_to Time.utc(2026, 3, 2, 16)
    Rails.cache.clear
    message = room.root_messages.create!(creator: david, markdown_source: "Stable fragment :github:", client_message_id: "cache-stability")
    message.boosts.create!(booster: jason, content: "👍")
    before = capture.call(message)
    travel_to Time.utc(2026, 3, 2, 16, 0, 2)
    if name == "unrelated_post"
      browser.post("/rooms/#{room.id}/messages", params: { message: { markdown_source: "Unrelated post" } }, headers: headers.merge("Accept" => "text/vnd.turbo-stream.html"))
      raise "post failed #{browser.response.status}" unless browser.response.successful?
      raise "no post" unless room.messages.exists?(markdown_source: "Unrelated post")
    else
      Tempfile.create(["cache-stability", ".svg"]) do |file|
        file.write('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><path d="M0 0h64v64H0z"/></svg>')
        file.flush
        upload = Rack::Test::UploadedFile.new(file.path, "image/svg+xml")
        browser.post("/account/icons", params: { workspace_icon: { name: "review_unused_icon", title: "Unused review icon", image: upload } }, headers:)
        raise "upload failed #{browser.response.status}" unless browser.response.redirect?
        raise "no icon" unless WorkspaceIcon.exists?(name: "review_unused_icon")
      end
    end
    after = capture.call(message)
    raise "changed unrelated fragment" unless before == after
    result = { name:, message_id: message.id, client_message_id: message.client_message_id, before:, after: }
    raise ActiveRecord::Rollback
  end
  Icons.expire_custom_cache!
  result
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], rows:) + "\n")
puts "WS8bm cache stability: #{rows.size} unrelated HTTP writes preserve Rails helper keys and HTML"
