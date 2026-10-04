# Actual Rails collection keys and real cache writes, never Rust-derived expectations.
require "json"
require "digest"
require "set"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.logger.level = Logger::FATAL
ActiveJob::Base.queue_adapter = :test
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
viewer = User.find(127326141)
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
browser.cookies[:session_token] = request.cookie_jar[:session_token]
writes = Set.new
cache = ActionController::Base.cache_store
[:write, :write_multi].each do |method|
  cache.define_singleton_method(method) do |*args, **kwargs, &block|
    keys = method == :write ? [args.first] : args.first.keys
    keys.each do |key|
      expanded = ActiveSupport::Cache.expand_cache_key(key)
      writes << expanded if expanded.include?("messages/_message:") || expanded.include?("messages/message:")
    end
    super(*args, **kwargs, &block)
  end
end

def get_keys(browser, viewer, zone, room_id)
  viewer.update_columns(time_zone: zone)
  browser.get("/rooms/#{room_id}/messages")
  raise "GET failed #{browser.response.status}" unless browser.response.status == 200
  records = browser.controller.instance_variable_get(:@messages)
  helper = browser.controller.view_context
  keys = Time.use_zone(zone) { records.to_h { |message| [message.id.to_s, ActiveSupport::Cache.expand_cache_key(helper.message_with_pr_cards_cache_key(message))] } }
  { zone:, room_id:, keys: }
end
pairs = [["github_alias", 654632876, ["Hawaii", "Pacific/Honolulu"]], ["ordinary", 486777696, ["Hawaii", "Asia/Kolkata"]]].map do |kind, room_id, zones|
  Rails.cache.clear
  writes.clear
  cases = zones.map do |zone|
    row = get_keys(browser, viewer, zone, room_id)
    row.merge(written_fragments: writes.size)
  end
  raise "cache instrumentation must observe real collection writes" unless cases[0][:written_fragments].positive?
  raise "keys must be shared" unless cases[0][:keys] == cases[1][:keys]
  raise "fragments must be reused" unless cases[0][:written_fragments] == cases[1][:written_fragments]
  { kind:, cases: }
end
pull_request = Github::PullRequest.find(1)
seasonal = ["2026-03-08T06:30:00Z", "2026-03-08T07:30:00Z", "2026-11-01T05:30:00Z", "2026-11-01T06:30:00Z", "2026-07-01T12:00:00Z"].flat_map do |instant|
  pull_request.update_columns(updated_at: Time.iso8601(instant))
  ["Hawaii", "Pacific/Honolulu", "Eastern Time (US & Canada)", "Asia/Kolkata", "Kathmandu", "Sydney", "Adelaide", "UTC"].map do |zone|
    Rails.cache.clear
    get_keys(browser, viewer, zone, 654632876).merge(instant:)
  end
end
sources = %w[app/controllers/concerns/set_time_zone.rb app/helpers/github/pull_requests_helper.rb app/views/messages/index.html.erb app/controllers/rooms_controller.rb].to_h { |path| [path, Digest::SHA256.file(Rails.root.join(path)).hexdigest] }
puts JSON.generate({ reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], sources:, pairs:, seasonal: })
