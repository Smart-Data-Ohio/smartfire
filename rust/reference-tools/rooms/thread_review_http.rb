# Actual HTTP responses. Only token/nonce entropy is fixed; no rendered child is supplied.
require "json"
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
Rails.application.env_config["action_dispatch.content_security_policy_nonce_generator"] = ->(_) { "NONCE" }
ActiveJob::Base.queue_adapter = :test
ChannelThreadsController.prepend(Module.new do
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end)
viewer = User.find(127326141)
mapping = Github::PullRequestThread.find_by!(channel_thread_id: 8)
thread, pr = mapping.channel_thread, mapping.pull_request
rows = []
[["public", false], ["private", true], ["unknown", nil], ["unmapped", false]].each do |name, private|
  ActiveRecord::Base.transaction do
    pr.update_columns(private:)
    mapping.destroy! if name == "unmapped"
    Rails.cache.clear
    request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
    request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
    browser = ActionDispatch::Integration::Session.new(Rails.application)
    browser.host! "campfire.test"
    path = "/rooms/#{thread.room_id}/threads/#{thread.id}"
    browser.get(path, headers: {"Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"})
    raise "Rails thread request failed: #{browser.response.status}" unless browser.response.status == 200
    main = browser.response.body[/<main class="thread".*?<\/main>/m]
    raise "thread main missing" unless main
    rows << {name:, private:, mapped: name != "unmapped", path:, status: browser.response.status, body: main}
    raise ActiveRecord::Rollback
  end
end
paths = %w[app/controllers/channel_threads_controller.rb app/views/channel_threads/show.html.erb app/helpers/github/pull_requests_helper.rb app/views/github/pull_requests/_thread_header.html.erb]
puts JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], sources: paths.to_h { |p| [p, Digest::SHA256.file(Rails.root.join(p)).hexdigest] }, thread_id: thread.id, room_id: thread.room_id, pull_request_id: pr.id, rows:)
warn "Rails PR-thread HTTP oracle: 4 responses; public/private/unknown/unmapped; no injected child HTML"
