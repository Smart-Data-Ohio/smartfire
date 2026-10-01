# Capture actual room/refresh HTTP responses; only Rails supplies the card bytes.
require "json"
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.logger.level = Logger::FATAL
ActiveJob::Base.queue_adapter = :test
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
viewer = User.find(127326141)
room = Room.find(486777696)
pull_request = Github::PullRequest.find(1)
pull_request.update_columns(github_updated_at: Time.utc(2026, 7, 1, 12))
events = []
messages = []
[["spring", "2026-03-08T06:30:00Z", "2026-03-08T07:30:00Z"],
 ["fall", "2026-11-01T05:30:00Z", "2026-11-01T06:30:00Z"]].each_with_index do |(name, starts, ends), index|
  event = Event.new(id: 8000000601 + index, room:, organizer: viewer, title: "Zone #{name}",
    starts_at: Time.iso8601(starts), ends_at: Time.iso8601(ends), time_zone: "UTC")
  Event.insert_all!([event.attributes.except("created_at", "updated_at").merge("created_at" => Time.current, "updated_at" => Time.current)])
  message = room.root_messages.create_with_attachment!(id: 9000000601 + index, creator: viewer,
    client_message_id: "ws8br-zone-#{name}", markdown_source: "Zone #{name}")
  EventReference.create!(event_id: event.id, message_id: message.id)
  events << event.id
  messages << message.id
end

def container(body, id)
  offset = body.index(%{<div id="#{id}"}) or raise "missing #{id}"
  depth = 0
  body[offset..].to_enum(:scan, %r{</?div\b[^>]*>}).each do
    match = Regexp.last_match
    depth += match[0].start_with?("</") ? -1 : 1
    return body[offset, match.end(0)] if depth.zero?
  end
  raise "unclosed #{id}"
end

request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
cookie = "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"
cases = []
["Hawaii", "Eastern Time (US & Canada)", "UTC", nil, "", "Not a real zone"].each do |zone|
  viewer.update_columns(time_zone: zone)
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! "campfire.test"
  ["/rooms/#{room.id}", "/rooms/#{room.id}/refresh?since=1772467199999", "/rooms/654632876/threads/8"].each do |path|
    Rails.cache.clear
    browser.get(path, headers: { "Cookie" => cookie, "Accept" => path.include?("refresh") ? "text/vnd.turbo-stream.html" : "text/html" })
    raise "Rails request failed #{path}: #{browser.response.status}" unless browser.response.status == 200
    cards = if path.include?("threads")
      [{ target: "github_pr_header_channel_thread_8", html: container(browser.response.body, "github_pr_header_channel_thread_8") }]
    else
      messages.map do |id|
        message = Message.find(id)
        { message_id: id, target: "event_cards_message_#{message.client_message_id}", html: container(browser.response.body, "event_cards_message_#{message.client_message_id}") }
      end
    end
    cases << { zone:, path:, status: browser.response.status, cards: }
  end
end
rows = {
  "events" => Event.where(id: events).order(:id).map(&:attributes),
  "messages" => Message.where(id: messages).order(:id).map(&:attributes),
  "action_text_rich_texts" => ActionText::RichText.where(record_type: "Message", record_id: messages).order(:id).map(&:attributes),
  "event_references" => EventReference.where(message_id: messages).order(:id).map(&:attributes)
}
rows.each_value do |records|
  records.each do |record|
    record.transform_values! { |value| value.is_a?(Time) || value.is_a?(ActiveSupport::TimeWithZone) ? value.utc.strftime("%Y-%m-%d %H:%M:%S.%6N") : value }
  end
end
sources = %w[app/controllers/concerns/set_time_zone.rb app/views/rooms/events/_card.html.erb app/views/rooms/events/_cards.html.erb app/views/github/pull_requests/_card.html.erb app/views/github/pull_requests/_thread_header.html.erb].to_h { |path| [path, Digest::SHA256.file(Rails.root.join(path)).hexdigest] }
puts JSON.generate({ reference: "d7c7de92", sources:, rows:,
  github: { id: pull_request.id, updated_at: pull_request.github_updated_at.utc.strftime("%Y-%m-%d %H:%M:%S.%6N") }, cases: })
