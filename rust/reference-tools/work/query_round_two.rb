# Actual pinned Rails requests for PR #193's distinct-association and inbox probes.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
root = ENV.fetch('PARITY_WORK')
JSON.parse(File.read(File.join(root, 'reference-tools/work/source-hashes.json'))).each do |path, hash|
  raise "Rails source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new(File::NULL)
ApplicationController.logger = Rails.logger
conn = ActiveRecord::Base.connection
human = JSON.parse(File.read(File.join(root, 'vectors/human_work_http.json')))['rows'][0]['setup']
inbox = JSON.parse(File.read(File.join(root, 'vectors/inbox-http.json')))['setup']
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = User.find(127326141).sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { 'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}", 'User-Agent' => 'Mozilla' }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
now = "'2026-03-02 16:00:00'"
rows = []
%w[human_rooms agent_rooms pane_options inbox_json inbox_html].each do |surface|
  [10, 100].each do |size|
    ActiveRecord::Base.transaction do
      is_inbox = surface.start_with?('inbox')
      (is_inbox ? inbox : human).each { |sql| conn.execute(sql) }
      if surface.end_with?('_rooms')
        conn.execute('UPDATE channel_threads SET work_status=NULL')
        conn.execute('DELETE FROM work_thread_links')
        conn.execute('DELETE FROM work_thread_events')
        size.times do |i|
          user, room, thread = 2_000_000_000 + i, 2_100_000_000 + i, 1000 + i
          role = surface == 'agent_rooms' ? 2 : 0
          conn.execute("INSERT INTO rooms(id,creator_id,name,type,created_at,updated_at) VALUES(#{room},127326141,'Review room #{i}','Rooms::Closed',#{now},#{now})")
          conn.execute("INSERT INTO users(id,name,email_address,role,created_at,updated_at) VALUES(#{user},'Review owner #{i}','review-#{i}@example.test',#{role},#{now},#{now})")
          [127326141, user].each { |member| conn.execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(#{room},#{member},'mentions',#{now},#{now})") }
          if role == 2
            agent = 2_200_000_000 + i
            conn.execute("INSERT INTO agents(id,user_id,owner_id,created_at,updated_at) VALUES(#{agent},#{user},127326141,#{now},#{now})")
            conn.execute("INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(#{agent},#{room},127326141,'post_messages',#{now},#{now})")
          end
          conn.execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) VALUES(#{thread},#{room},127326141,'Query work #{i}','planned',#{user},#{now},#{now},#{now})")
        end
        conn.execute("INSERT INTO rooms(id,creator_id,name,type,created_at,updated_at) VALUES(2100000999,149087659,'Hidden room','Rooms::Closed',#{now},#{now})")
        conn.execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,created_at,updated_at,last_activity_at) VALUES(9999,2100000999,149087659,'HIDDEN_SENTINEL','planned',#{now},#{now},#{now})")
      elsif surface == 'pane_options'
        conn.execute('DELETE FROM work_thread_links')
        conn.execute('DELETE FROM work_thread_events')
        conn.execute("UPDATE events SET cancelled_at=#{now} WHERE room_id=486777696")
        size.times { |i| conn.execute("INSERT INTO events(id,room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(#{2_300_000_000+i},486777696,127326141,'Option #{i}','2026-03-03 16:00:00','UTC',#{now},#{now})") }
      else
        conn.execute('DELETE FROM activity_items')
        size.times do |i|
          event = 8_700_000_000 + i
          conn.execute("INSERT INTO work_thread_events(id,actor_id,channel_thread_id,created_at,event_type,from_owner_id,from_owner_name,from_status,metadata,to_owner_id,to_owner_name,to_status,updated_at) SELECT #{event},actor_id,channel_thread_id,created_at,event_type,from_owner_id,from_owner_name,from_status,metadata,to_owner_id,to_owner_name,to_status,updated_at FROM work_thread_events WHERE id=8300000001")
          conn.execute("INSERT INTO activity_items(id,user_id,source_id,source_type,event_type,created_at,updated_at) VALUES(#{8_800_000_000+i},127326141,#{event},'WorkThreadEvent','work_update',#{now},#{now})")
        end
      end
      path = is_inbox ? '/activity' : surface == 'pane_options' ? '/rooms/486777696/threads/91' : '/work?state=all'
      hdr = headers.merge('Accept' => surface == 'inbox_json' ? 'application/json' : 'text/html')
      hdr['Turbo-Frame'] = 'activity_test' if is_inbox
      queries = []
      subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') do |*args|
        event = args.last
        queries << event unless event[:name] == 'SCHEMA' || event[:sql].match?(/\A(?:BEGIN|COMMIT|PRAGMA)/)
      end
      begin
        browser.get(path, headers: hdr)
      ensure
        ActiveSupport::Notifications.unsubscribe(subscriber)
      end
      raise "#{surface}: HTTP #{browser.response.status}" unless browser.response.status == 200
      if surface.end_with?('_rooms')
        raise 'wrong index size' unless browser.response.body.scan('class="work-threads__item"').length == size
        raise 'hidden work leaked' if browser.response.body.include?('HIDDEN_SENTINEL')
      elsif surface == 'inbox_json'
        raise 'wrong inbox size' unless JSON.parse(browser.response.body)['activity_items'].length == size
      end
      rows << { surface:, size:, queries: queries.count { |event| !event[:cached] }, cached: queries.count { |event| event[:cached] } }
      raise ActiveRecord::Rollback
    end
  end
end
puts JSON.pretty_generate(reference: 'd7c7de92 plus approved board drift', rows:)
rows.each { |row| warn "Rails R2 #{row[:surface]} size=#{row[:size]}: #{row[:queries]} SQL; #{row[:cached]} cached" }
