# Raw responses from the pinned ActivityItemsController, on a private default seed.
require 'action_dispatch/testing/integration'
require 'json'
ApplicationController.allow_forgery_protection = false
ActiveRecord::Base.logger = nil
Rails.logger = ActiveSupport::Logger.new($stderr)
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
david = User.find(127326141)
jason = User.find(149087659)
room = Room.find(486777696)
ActivityItem.delete_all
huddle = HuddleGrant.create!(id: 8300000003, user: jason, room: Room.find(186869642), membership: Membership.find_by!(user: jason, room_id: 186869642), session: jason.sessions.first, identity: 'ws11ui-inbox', room_name: 'ws11ui-inbox', last_issued_at: Time.current)
work = WorkThreadEvent.create!(id: 8300000001, thread: ChannelThread.first, actor: jason, event_type: 'work_update', from_status: nil, to_status: 'in_progress', from_owner_name: nil, to_owner_name: 'David')
nudge = BoardSlaNudge.create!(id: 8300000002, room: work.thread.room, channel_thread: work.thread, recipient: david, work_status: 'in_progress', stage: 'escalation', status_entered_at: 90.minutes.ago)
notice = AgentBudgetNotice.create!(id: 8300000004, agent: Agent.first, cap: 'messages', day: Date.current)
sources = [room.messages.order(:id).first, SavedItem.first, work, nudge, huddle, Event.first, AgentApproval.first, notice, ScheduledMessage.first, TwoFactorCredential.first, david.sessions.first]
types = %w[mention message_reminder work_update work_sla huddle_started event_update agent_approval_request agent_budget_exceeded scheduled_message_dropped two_factor_lockout new_sign_in]
# Make every source visible to this recipient without changing its owner or payload.
sources.each do |source|
  r = case source; when SavedItem then source.message.room; when WorkThreadEvent then source.thread.room; else source.try(:room); end
  Membership.find_or_create_by!(user: david, room: r) { |m| m.involvement = 'everything' } if r
end
ActivityItem.delete_all
sources.zip(types).each_with_index { |(source, type), index| ActivityItem.create!(id: 8400000000 + index, user: david, source:, event_type: type) }
ActivityItem.create!(id: 8400000099, user: jason, source: sources.first, event_type: 'mention')
# Snapshot exact rows used by the fixture, including timestamps and serialized payloads.
tables = %w[memberships huddle_grants work_thread_events board_sla_nudges agent_budget_notices activity_items]
setup = tables.flat_map do |table|
  ["DELETE FROM #{table}"] + ActiveRecord::Base.connection.select_all("SELECT * FROM #{table}").map do |row|
    "INSERT INTO #{table} (#{row.keys.map { |k| ActiveRecord::Base.connection.quote_column_name(k) }.join(',')}) VALUES (#{row.values.map { |v| ActiveRecord::Base.connection.quote(v) }.join(',')})"
  end
end
cases = []
add = ->(name, method, path, params = {}, sql = [], viewer = 'david', accept = 'application/json') { cases << { name:, method:, path:, params:, sql:, viewer:, accept: } }
%w[unread read handled bogus].each { |state| add.call("index #{state}", 'get', "/activity.json?status=#{state}") }
ActivityItem::TYPE_FILTERS.keys.each { |type| add.call("type #{type}", 'get', "/activity.json?type=#{type}") }
add.call('count', 'get', '/activity/unread_count.json')
%w[read handled].each do |action|
  ['', action, action == 'read' ? 'unread' : 'unhandled', 'bogus', ['read'], {'x' => 'read'}].each do |state|
    add.call("#{action} #{state.inspect}", 'patch', "/activity/8400000000/#{action}.json", {'state' => state})
  end
  [8400000004, 8400000099, -1].each { |id| add.call("#{action} source #{id}", 'patch', "/activity/#{id}/#{action}.json") }
  add.call("#{action} revoked", 'patch', "/activity/8400000000/#{action}.json", {}, ["DELETE FROM memberships WHERE user_id=127326141"])
  %w[text/html text/vnd.turbo-stream.html].each do |accept|
    add.call("#{action} redirect #{accept}", 'patch', "/activity/8400000000/#{action}", {'state'=>action,'status'=>'read','type'=>'huddles'}, [], 'david', accept)
    add.call("#{action} invalid #{accept}", 'patch', "/activity/8400000000/#{action}", {'state'=>'bogus','status'=>'wrong','type'=>'wrong'}, [], 'david', accept)
  end
end
add.call('revoked index', 'get', '/activity.json', {}, ["DELETE FROM memberships WHERE user_id=127326141"])
add.call('index frame', 'get', '/activity', {}, [], 'david', 'text/html')
add.call('index stream', 'get', '/activity.turbo_stream', {}, [], 'david', 'text/vnd.turbo-stream.html')
add.call('open JSON', 'post', '/activity/8400000000/open.json')
add.call('open redirect', 'post', '/activity/8400000004/open', {}, [], 'david', 'text/html')
paging = ["DELETE FROM activity_items"] + room.messages.order(:id).limit(105).each_with_index.map do |message,index|
  "INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,created_at,updated_at) VALUES (#{8600000000+index},#{david.id},'Message',#{message.id},'mention','2026-03-02 16:00:00.000000','2026-03-02 16:00:00.000000')"
end
['', '?before=8600000005', '?before=missing', '?before=999999999', '?before=8400000099'].each { |query| add.call("pagination #{query}", 'get', "/activity.json#{query}", {}, paging) }
add.call('pagination stream', 'get', '/activity.turbo_stream', {}, paging, 'david', 'text/vnd.turbo-stream.html')
# Every polymorphic source has an HTML destination even when its JSON source is nil.
sources.each_with_index do |source, index|
  %w[text/html text/vnd.turbo-stream.html application/json].each do |accept|
    add.call("open #{source.class.name} #{accept}", 'post', "/activity/#{8400000000 + index}/open", {}, [], 'david', accept)
  end
end
cases.each do |entry|
  ActiveRecord::Base.transaction do
    setup.each { |sql| ActiveRecord::Base.connection.execute(sql) }
    entry.fetch(:sql).each { |sql| ActiveRecord::Base.connection.execute(sql) }
    browser = ActionDispatch::Integration::Session.new(Rails.application)
    browser.host! 'campfire.test'
    headers = { 'Cookie' => "session_token=#{labels.fetch("session_cookies.#{entry.fetch(:viewer)}")}", 'Accept'=>entry.fetch(:accept), 'Turbo-Frame'=>'activity_test', 'HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0' }
    begin
      browser.public_send(entry.fetch(:method), entry.fetch(:path), params: entry.fetch(:params), headers:)
      entry[:status] = browser.response.status
      entry[:body] = browser.response.body
      entry[:headers] = %w[content-type cache-control pragma location].to_h { |h| [h, browser.response.headers[h]] }
    rescue ActiveRecord::RecordNotFound
      entry[:status] = 404
      entry[:body] = nil # Production error templates belong to WS9; assert status and no write.
    end
    entry[:state] = ActivityItem.find_by(id: 8400000000)&.attributes&.slice('read_at','handled_at','updated_at')
    if entry[:name].start_with?('open ')
      entry[:opened_state] = ActivityItem.find_by(id: entry[:path].split('/')[2])&.attributes&.slice('read_at','handled_at','updated_at')
    end
    raise ActiveRecord::Rollback
  end
  ActiveSupport::ExecutionContext.clear
end
puts JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA"), setup:, cases:)
warn "inbox Rails HTTP oracle: #{cases.size} responses; 11 source types; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
