# Real visible-row pages and immediate-send broadcasts; independent of Rust output.
require 'json'
require 'action_dispatch/testing/integration'
require_relative 'oracle-database'
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
Icons.custom_icons
Icons.instance_variable_set(:@custom_cache_at, Float::INFINITY)
Random.define_singleton_method(:uuid) { 'fixture-send-now-zone' }
conn = ActiveRecord::Base.connection
source = JSON.parse(File.read('/work/vectors/messaging/older_calendar_execution.json'))
reset = MessagingOracleDatabase.scenarios(ARGV.fetch(0))
def browser_for(user)
  request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
  request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at:nil).first!.token
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! 'campfire.test'
  [browser, {'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}", 'Accept'=>'text/html'}]
end
groups = []
source.fetch('groups').each do |group|
 %w[ordinary mixed direct].each do |kind|
  reset.call do
   conn = ActiveRecord::Base.connection
   group.fetch('rows').each do |table,rows|
    rows.each { |r| conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{r.keys.map { |k| conn.quote_column_name(k) }.join(',')}) VALUES (#{r.values.map { |v| conn.quote(v) }.join(',')})") }
   end
   user = User.find(127326141)
   direct = kind=='direct' ? user.rooms.where(type:'Rooms::Direct').first! : nil
   group.fetch('old_ids').each_with_index do |id,index|
    message = Message.find(id)
    message.update_columns(room_id:direct.id) if direct
    saved = SavedItem.create!(user:,message:)
    row = ScheduledMessage.create!(user:,room:message.room,markdown_source:'Scaling review <draft> & proof',send_at:Time.utc(2026,3,3,16))
    if kind=='mixed'
     saved.update_columns(status:'done') if index.odd?
     row.update_columns(thread_id:group.fetch('rows').fetch('channel_threads').first.fetch('id')) if index==0
     row.update_columns(sent_at:Time.current,sent_message_id:message.id) if index==1
     row.update_columns(dropped_at:Time.current,drop_reason:'review access loss') if index==2
     if index==3
      stranded = Room.where.not(id:user.memberships.select(:room_id)).where(deleted_at:nil).first!
      row.update_columns(room_id:stranded.id)
     end
    end
   end
   browser,headers = browser_for(user)
   pages = []
   %w[/saved /scheduled_messages].each do |path|
    queries=[]
    observer=->(*args) { p=args.last; queries << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
    ActiveSupport::Notifications.subscribed(observer,'sql.active_record') { browser.get(path,headers:) }
    prefix=path=='/saved' ? 'saved-items' : 'scheduled-messages'
    section=browser.response.body[/<section class="#{prefix}__page".*?<\/section>/m]
    raise "missing #{path}" unless browser.response.status==200 && section
    pages << {path:,status:browser.response.status,content_type:browser.response.headers['Content-Type'],html:section,reads:queries.length}
    puts "WS8bm2 review229 Rails visible=#{group.fetch('size')} kind=#{kind} path=#{path} reads=#{queries.length}"
   end
   groups << group.slice('size','rows','old_ids').merge(kind:,direct_room_id:direct&.id,pages:)
  end
 end
end
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,html,**| frames << {stream:,html:} }
sends=[]
%w[UTC America/New_York Asia/Kolkata].each do |zone|
 reset.call do
  user=User.find(127326141);user.update_columns(time_zone:zone)
  room=Room.find(699448326)
  row=ScheduledMessage.create!(user:,room:,markdown_source:'Review send now',send_at:Time.utc(10000,3,5,9))
  browser,headers=browser_for(user)
  frames.clear
  browser.post("/scheduled_messages/#{row.id}/send_now",params:'{}',headers:headers.merge('Accept'=>'application/json','Content-Type'=>'application/json'))
  row.reload
  sends << {zone:,status:browser.response.status,content_type:browser.response.headers['Content-Type'],body:browser.response.body,
    scheduled:ActiveRecord::Base.connection.select_one("SELECT * FROM scheduled_messages WHERE id=#{row.id}"),
    message:ActiveRecord::Base.connection.select_one("SELECT * FROM messages WHERE id=#{row.sent_message_id}"),
    frames:frames.select { |f| f[:stream]=="#{room.to_gid_param}:messages" }.dup}
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],groups:,sends:)+"\n")
puts "WS8bm2 review229 Rails: #{groups.length*2} complete HTTP feature sections at 4/16 visible rows; #{sends.length} real immediate sends, complete persisted rows and frames"
