# Full room HTTP responses: child partials are rendered by Rails itself.
require "json"
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
Rails.application.env_config["action_dispatch.content_security_policy_nonce_generator"] = ->(_) { "NONCE" }
ActiveJob::Base.queue_adapter = :test
RoomsController.prepend(Module.new do
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end)
rows = []
[[654632876,127326141,"empty_channel"], [186869642,127326141,"empty_pair"], [699448329,712064548,"empty_group"], [201306877,773523953,"empty_open"]].each do |room_id,user_id,name|
  ActiveRecord::Base.transaction do
    stamp = Room.find(room_id).updated_at
    Message.where(room_id:).destroy_all
    # This is an empty rendering fixture, not a test of message-destruction touches.
    Room.where(id: room_id).update_all(updated_at: stamp)
    viewer = User.find(user_id)
    request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
    request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
    browser = ActionDispatch::Integration::Session.new(Rails.application)
    browser.host! "campfire.test"
    path = "/rooms/#{room_id}"
    browser.get(path, headers: {"Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}", "User-Agent" => "Mozilla"})
    raise "Rails room request failed: #{browser.response.status}" unless browser.response.status == 200
    rows << {name:, room_id:, user_id:, path:, status: browser.response.status, body: browser.response.body}
    raise ActiveRecord::Rollback
  end
end
paths = %w[app/controllers/rooms_controller.rb app/views/rooms/show.html.erb app/views/rooms/show/_thread_panel.html.erb app/views/polls/_builder.html.erb app/views/rooms/pins/_panel.html.erb app/views/messages/_template.html.erb app/views/rooms/show/_composer.html.erb app/views/layouts/application.html.erb]
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'), sources: paths.to_h { |p| [p, Digest::SHA256.file(Rails.root.join(p)).hexdigest] }, rows:)
warn "Rails empty-shell HTTP oracle: 4 full responses; channel/pair/group/open; no injected child HTML"
