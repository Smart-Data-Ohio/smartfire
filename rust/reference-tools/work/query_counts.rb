# Actual pinned Rails HTTP query counts for PR #193, using the review's fixtures.
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
row = JSON.parse(File.read(File.join(root, 'vectors/human_work_http.json')))['rows'][0]
row['setup'].each { |sql| conn.execute(sql) }
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = User.find(127326141).sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { 'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}", 'User-Agent' => 'Mozilla' }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
rows = []
[10, 100].each do |size|
  conn.execute('UPDATE channel_threads SET work_status=NULL')
  conn.execute('DELETE FROM work_thread_links')
  conn.execute('DELETE FROM work_thread_events')
  conn.execute('DELETE FROM channel_threads WHERE id>=1000 AND id<1200')
  size.times do |i|
    conn.execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) VALUES(#{1000+i},486777696,127326141,'Query work #{i}','planned',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
  end
  measure = lambda do |path, surface|
    queries = []
    subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') do |*args|
      event = args.last
      queries << event unless event[:name] == 'SCHEMA' || event[:sql].match?(/\A(?:BEGIN|COMMIT|PRAGMA)/)
    end
    begin
      browser.get(path, headers: headers)
    ensure
      ActiveSupport::Notifications.unsubscribe(subscriber)
    end
    raise "#{surface}: HTTP #{browser.response.status}" unless browser.response.status == 200
    rows << { 'surface' => surface, 'size' => size, 'queries' => queries.count { |event| !event[:cached] }, 'cached' => queries.count { |event| event[:cached] } }
  end
  measure.call('/work?state=all', 'index')
  measure.call('/work.json?state=all', 'json-index')
  conn.execute("UPDATE channel_threads SET work_status='planned' WHERE id IN (90,91)")
  [90, 91].each do |thread|
    size.times do
      conn.execute("INSERT INTO work_thread_events(channel_thread_id,actor_id,event_type,from_status,to_status,metadata,created_at,updated_at) VALUES(#{thread},149087659,'work_update','planned','blocked','{}','2026-03-02 16:00:00','2026-03-02 16:00:00')")
    end
  end
  measure.call('/rooms/486777696/threads/91', 'ordinary-history')
  measure.call('/rooms/699448332/threads/90', 'board-history')
end
puts JSON.pretty_generate({ 'reference' => ENV.fetch("PARITY_REFERENCE_SHA"), 'rows' => rows })
rows.each { |row| warn "Rails query oracle #{row['surface']} size=#{row['size']}: #{row['queries']} SQL; #{row['cached']} cached" }
