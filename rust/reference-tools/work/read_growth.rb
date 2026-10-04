# PR #193 follow-ups: full JSON bytes, opening recorder facts and measured Rails reads.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
root = ENV.fetch('PARITY_WORK')
JSON.parse(File.read(File.join(root, 'reference-tools/work/read-growth-source-hashes.json'))).each do |path, hash|
  raise "Rails source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new(File::NULL)
ApplicationController.logger = Rails.logger
conn = ActiveRecord::Base.connection
base = JSON.parse(File.read(File.join(root, 'vectors/human_work_http.json')))['rows'][0]['setup']
stamp = "'2026-03-02 16:00:00'"
labels = JSON.parse(File.read(File.join(root, 'parity/.seed/default/labels.json')))
headers = { 'Cookie' => "session_token=#{labels.fetch('session_cookies.david')}", 'User-Agent' => 'Mozilla' }
def reads
  sql = []
  subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') do |*args|
    event = args.last
    sql << event[:sql] if event[:name] != 'SCHEMA' && !event[:cached] && event[:sql].match?(/\A\s*(SELECT|WITH)/)
  end
  yield sql
ensure
  ActiveSupport::Notifications.unsubscribe(subscriber)
end
json_rows = %w[shared humans agents icons].product([10, 100]).map do |kind, size|
  result = nil
  ActiveRecord::Base.transaction do
    setup = base + ['UPDATE channel_threads SET work_status=NULL', 'DELETE FROM work_thread_links', 'DELETE FROM work_thread_events']
    size.times do |i|
      room = 486777696
      owner = 127326141
      if kind != 'shared'
        room = 2_100_000_000 + i
        owner = 2_000_000_000 + i
        role = kind == 'agents' ? 2 : 0
        setup << "INSERT INTO rooms(id,creator_id,name,type,created_at,updated_at) VALUES(#{room},127326141,'Review room #{i}','Rooms::Closed',#{stamp},#{stamp})"
        setup << "INSERT INTO users(id,name,email_address,role,created_at,updated_at) VALUES(#{owner},'Review owner #{i}','review-#{i}@example.test',#{role},#{stamp},#{stamp})"
        [127326141,149087659,owner].each do |member|
          setup << "INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(#{room},#{member},'mentions',#{stamp},#{stamp})"
        end
        if kind == 'agents'
          agent = 2_200_000_000 + i
          setup << "INSERT INTO agents(id,user_id,owner_id,created_at,updated_at) VALUES(#{agent},#{owner},127326141,#{stamp},#{stamp})"
          setup << "INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(#{agent},#{room},127326141,'post_messages',#{stamp},#{stamp})"
        end
        if kind == 'icons'
          setup << "INSERT INTO workspace_icons(name,title,creator_id,created_at,updated_at) VALUES('work_icon_#{i}','Work #{i}',127326141,#{stamp},#{stamp})"
          setup << "UPDATE users SET icon_name='work_icon_#{i}' WHERE id=#{owner}"
        end
      end
      setup << "INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) VALUES(#{1000+i},#{room},127326141,'Query work #{i}','planned',#{owner},#{stamp},#{stamp},#{stamp})"
    end
    if kind == 'icons'
      setup << "UPDATE users SET icon_name='work_icon_0' WHERE id=127326141"
    end
    setup.each { |sql| conn.execute(sql) }
    Icons.expire_custom_cache! # Raw fixture SQL bypasses WorkspaceIcon after-commit invalidation.
    browser = ActionDispatch::Integration::Session.new(Rails.application)
    browser.host! 'campfire.test'
    browser.get('/work?state=all', headers:)
    reads do |sql|
      browser.get('/work.json?state=all', headers:)
      raise "JSON HTTP #{browser.response.status}" unless browser.response.status == 200
      result = { kind:, size:, setup:, path: '/work.json?state=all', status: browser.response.status, body: browser.response.body, reads: sql.size }
    end
    warn "Rails work JSON #{kind} size=#{size}: #{result[:reads]} SELECTs; complete response; 0 masks"
    raise ActiveRecord::Rollback
  end
  result
end
opening = [10, 200].map do |size|
  result = nil
  ActiveRecord::Base.transaction do
    setup = base + ["UPDATE rooms SET type='Rooms::Board' WHERE id=486777696", 'DELETE FROM work_thread_links', 'DELETE FROM activity_items', 'DELETE FROM memberships WHERE room_id=486777696']
    size.times do |i|
      user = i == 0 ? 127326141 : i == 1 ? 149087659 : 2_400_000_000 + i
      unless [127326141, 149087659].include?(user)
        setup << "INSERT INTO users(id,name,email_address,role,created_at,updated_at) VALUES(#{user},'Board owner #{format('%03d',i)}','board-owner-#{i}@example.test',0,#{stamp},#{stamp})"
      end
      setup << "INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(486777696,#{user},'everything',#{stamp},#{stamp})"
    end
    setup.each { |sql| conn.execute(sql) }
    room = Room.find(486777696)
    creator = User.find(127326141)
    post = nil
    count = nil
    reads do |sql|
      post = ChannelThread.create_board_post!(room:, creator:, name: 'Opening query probe', work_status: 'planned', first_message: 'Opening message')
      count = sql.size
    end
    message = post.messages.first!
    items = conn.select_all("SELECT id,user_id,source_type,source_id,event_type,read_at,handled_at,created_at,updated_at FROM activity_items WHERE source_type='Message' AND source_id=#{message.id} ORDER BY user_id").to_a
    raise 'recipient fan-out drift' unless items.size == size-1
    result = { size:, recipients: size-1, setup:, reads: count, thread_id: post.id, message_id: message.id, items: }
    warn "Rails board opening recipients=#{size-1}: #{count} SELECTs; #{items.size} complete recorder facts"
    raise ActiveRecord::Rollback
  end
  result
end
puts JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA"), json: json_rows, opening:)
