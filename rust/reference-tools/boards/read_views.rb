# Actual Rails row fragments and complete board-list responses, without output masks.
require "json"
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
Rails.application.env_config["action_dispatch.content_security_policy_nonce_generator"] = ->(_) { "NONCE" }
ActiveJob::Base.queue_adapter = :test
module BoardGoldenTokens
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
ApplicationController.prepend(BoardGoldenTokens)
board = Room.find(699448332)
viewer = User.find(127326141)
fragments = []
board.channel_threads.ordered.each do |thread|
  [:board_row, :board_column_row].each do |suffix|
    html = ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(partial: "rooms/boards/row", locals: {thread:, dom_suffix: suffix})
    fragments << {thread_id: thread.id, column: suffix == :board_column_row, html:}
  end
end
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
rows = []
["", "?view=board", "?status=done", "?status=all&owner=me&tag=release", "?tag=missing", "?status=invalid&owner=unknown&tag= Rust ", "?view=board&status=done&owner=agents"].each_with_index do |query, index|
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! "campfire.test"
  path = "/rooms/#{board.id}#{query.gsub(' ', '%20')}"
  browser.get(path, headers: {"Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}", "User-Agent" => "Mozilla"})
  raise "Rails board request failed: #{browser.response.status}" unless browser.response.status == 200
  rows << {name: "board-#{index}", room_id: board.id, user_id: viewer.id, path:, status: browser.response.status, html: browser.response.body}
  ActiveSupport::IsolatedExecutionState.clear
end
# Each form/error case gets the seed's untouched session and rows. Controller failures are
# actual HTTP responses; no injected child fragments or normalization.
forms = [
  ["new-admin",127326141,"get","/rooms/boards/new",{}],
  ["new-member",149087659,"get","/rooms/boards/new",{}],
  ["edit-admin",127326141,"get","/rooms/boards/699448332/edit",{}],
  ["edit-member",149087659,"get","/rooms/boards/699448332/edit",{}],
  ["namespace-show",127326141,"get","/rooms/boards/699448332",{}],
  ["edit-nonmember",712064548,"get","/rooms/boards/699448332/edit",{}],
  ["invalid-create",127326141,"post","/rooms/boards",{"room[name]"=>"","user_ids[]"=>"127326141"}],
  ["invalid-update",127326141,"patch","/rooms/boards/699448332",{"room[name]"=>"","user_ids[]"=>"127326141"}]
]
previous = ActionController::Base.allow_forgery_protection
ActionController::Base.allow_forgery_protection = false
forms.each do |name,user_id,method,path,pairs|
  ActiveRecord::Base.transaction do
    viewer = User.find(user_id)
    request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new))
    request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at:nil).first!.token
    browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! "campfire.test"
    browser.public_send(method,path,params:URI.encode_www_form(pairs),headers:{"Cookie"=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}","User-Agent"=>"Mozilla","Content-Type"=>"application/x-www-form-urlencoded"})
    rows << {name:,room_id:board.id,user_id:,path:,method:,form:pairs.to_a,status:browser.response.status,location:browser.response.headers["Location"],html:browser.response.body}
    raise ActiveRecord::Rollback
  end
  ActiveSupport::IsolatedExecutionState.clear
end
ActionController::Base.allow_forgery_protection=previous
files = %w[app/models/channel_thread.rb app/models/thread_tag.rb app/views/rooms/boards/_index.html.erb app/views/rooms/boards/_row.html.erb app/views/rooms/boards/_nav.html.erb app/views/layouts/application.html.erb]
puts JSON.pretty_generate(reference: "d7c7de92", board_reference: "origin/main b908ebc2 approved drift", layout_reference: "2e20b24c", sources: files.to_h { |f| [f, Digest::SHA256.file(Rails.root.join(f)).hexdigest] }, fragments:, rows:)
warn "Rails board read oracle: #{fragments.size} row fragments and #{rows.size} complete HTTP responses; approved board drift; no masks"
