# Actual Rails row fragments and complete board-list responses, without output masks.
require "json"
require "digest"
hashes=JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/boards/source-hashes.json")))
hashes.each { |file,hash| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash }
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
variants = {
  "unassigned"=>["UPDATE channel_threads SET work_owner_id=NULL WHERE id=4"],
  "closed"=>["UPDATE channel_threads SET closed_at='2026-03-02 16:00:00' WHERE id=4"],
  "locked"=>["UPDATE channel_threads SET locked_at='2026-03-02 16:00:00' WHERE id=4"],
  "escaped"=>["UPDATE channel_threads SET name='Ship <x> & \"quote\"' WHERE id=4"],
  "unavailable"=>["UPDATE users SET status=1 WHERE id=127326141"],
  "membership-revoked"=>["DELETE FROM memberships WHERE room_id=699448332 AND user_id=127326141"],
  "linked"=>["INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,url,created_at,updated_at) VALUES(4,127326141,'drive_file','https://drive.google.com/file/d/boards1234567','2026-03-02 16:00:00','2026-03-02 16:00:00')"]
}
variants.each do |name,setup|
  ActiveRecord::Base.transaction do
    setup.each { |sql| ActiveRecord::Base.connection.execute(sql) }
    thread=ChannelThread.find(4)
    [false,true].each do |column|
      html=ApplicationController.renderer.new(http_host:"campfire.test",https:false).render(partial:"rooms/boards/row",locals:{thread:,dom_suffix:column ? :board_column_row : :board_row})
      fragments << {name:,setup:,thread_id:thread.id,column:,html:}
    end
    raise ActiveRecord::Rollback
  end
end
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
rows = []
["", "?view=board", "?status=done", "?status=all&owner=me&tag=release", "?tag=missing", "?status=invalid&owner=unknown&tag= Rust ", "?view=board&status=done&owner=agents", "?owner[]=127326141&tag[]=rust", "?page=1_2tail"].each_with_index do |query, index|
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! "campfire.test"
  path = "/rooms/#{board.id}#{query.gsub(' ', '%20')}"
  browser.get(path, headers: {"Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}", "User-Agent" => "Mozilla"})
  raise "Rails board request failed: #{browser.response.status}" unless browser.response.status == 200
  rows << {name: "board-#{index}", room_id: board.id, user_id: viewer.id, path:, status: browser.response.status, html: browser.response.body}
  ActiveSupport::IsolatedExecutionState.clear
end
fixtures = [
  ["empty-board",["DELETE FROM thread_tags WHERE channel_thread_id IN (SELECT id FROM channel_threads WHERE room_id=699448332)","DELETE FROM channel_threads WHERE room_id=699448332"],"",{}],
  ["page-one",["WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i<55) INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,last_activity_at,created_at,updated_at) SELECT 8700000000+i,699448332,127326141,'Paging post '||i,'planned','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00' FROM n"],"",{}],
  ["page-two",["WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i<55) INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,last_activity_at,created_at,updated_at) SELECT 8700000000+i,699448332,127326141,'Paging post '||i,'planned','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00' FROM n"],"?page=2",{}],
  ["digest",["INSERT INTO board_stale_digests(room_id,digest_on,message_id,created_at,updated_at) VALUES(699448332,'2026-03-02',(SELECT id FROM messages WHERE thread_id=4 LIMIT 1),'2026-03-02 16:00:00','2026-03-02 16:00:00')"],"",{}],
  ["list-frame",[],"",{"Turbo-Frame"=>"main"}],
  ["column-frame",[],"?view=board",{"Turbo-Frame"=>"main"}]
]
fixtures.each do |name,setup,query,headers|
  ActiveRecord::Base.transaction do
    setup.each { |sql| ActiveRecord::Base.connection.execute(sql) }
    browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! "campfire.test"
    path="/rooms/#{board.id}#{query}"
    browser.get(path,headers:headers.merge("Cookie"=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}","User-Agent"=>"Mozilla"))
    raise "Rails board fixture failed: #{name} #{browser.response.status}" unless browser.response.status==200
    rows << {name:,setup:,headers:,room_id:board.id,user_id:viewer.id,path:,status:browser.response.status,html:browser.response.body}
    raise ActiveRecord::Rollback
  end
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
  ["blank-create",127326141,"post","/rooms/boards",{"room[name]"=>"","user_ids[]"=>"127326141"}],
  ["blank-update",127326141,"patch","/rooms/boards/699448332",{"room[name]"=>"","user_ids[]"=>"127326141"}],
  ["invalid-create",127326141,"post","/rooms/boards",{"room[name]"=>"Bad icon","room[icon_name]"=>"unknown-ws12-icon","user_ids[]"=>"127326141"}],
  ["invalid-update",127326141,"patch","/rooms/boards/699448332",{"room[name]"=>"Bad icon","room[icon_name]"=>"unknown-ws12-icon","user_ids[]"=>"127326141"}]
]
Rooms::BoardsController.skip_before_action :verify_authenticity_token
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
files = %w[app/models/channel_thread.rb app/models/thread_tag.rb app/views/rooms/boards/_index.html.erb app/views/rooms/boards/_row.html.erb app/views/rooms/boards/_nav.html.erb app/views/layouts/application.html.erb]
puts JSON.pretty_generate(reference: "d7c7de92", board_reference: "origin/main b908ebc2 approved drift", layout_reference: "2e20b24c", sources: files.to_h { |f| [f, Digest::SHA256.file(Rails.root.join(f)).hexdigest] }, fragments:, rows:)
warn "Rails board read oracle: #{fragments.size} row fragments and #{rows.size} complete HTTP responses; approved board drift; no masks"
