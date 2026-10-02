# Actual Rails SELECT counts for the #191/#195 board probes; no simulated SQL.
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
conn = ActiveRecord::Base.connection
setup = JSON.parse(File.read(File.join(root, 'vectors/human_work_http.json')))['rows'][0]['setup']
now = "'2026-03-02 16:00:00'"
rows = []
%w[legacy explicit humans opening unlinked event pull_request].each do |surface|
  [10, 200].each do |size|
    ActiveRecord::Base.transaction do
      setup.each { |sql| conn.execute(sql) }
      conn.execute("UPDATE rooms SET type='Rooms::Board' WHERE id=486777696")
      conn.execute('DELETE FROM work_thread_links')
      conn.execute('DELETE FROM activity_items')
      bots = %w[legacy explicit].include?(surface)
      if bots || %w[humans opening].include?(surface)
        conn.execute('DELETE FROM memberships WHERE room_id=486777696')
        size.times do |i|
          user = !bots && i == 0 ? 127326141 : !bots && i == 1 ? 149087659 : 2_400_000_000 + i
          unless [127326141, 149087659].include?(user)
            conn.execute("INSERT INTO users(id,name,email_address,role,created_at,updated_at) VALUES(#{user},'Board owner #{format('%03d',i)}','board-owner-#{i}@example.test',#{bots ? 2 : 0},#{now},#{now})")
          end
          conn.execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(486777696,#{user},'#{user == 149087659 ? 'everything' : 'invisible'}',#{now},#{now})")
          if bots
            agent = 2_500_000_000 + i
            conn.execute("INSERT INTO agents(id,user_id,owner_id,created_at,updated_at) VALUES(#{agent},#{user},127326141,#{now},#{now})")
            conn.execute("INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(#{agent},486777696,127326141,'post_messages',#{now},#{now})") if surface == 'explicit'
            conn.execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) VALUES(#{2000+i},486777696,127326141,'Agent post #{i}','planned',#{user},#{now},#{now},#{now})")
          end
        end
      else
        conn.execute("UPDATE events SET cancelled_at=#{now} WHERE room_id=486777696")
        size.times do |i|
          id = 2_600_000_000 + i
          if surface == 'pull_request'
            conn.execute("INSERT INTO github_pull_requests(id,owner,repo,number,private,state,title,created_at,updated_at) VALUES(#{id},'query','board',#{i+1},0,'open','Linked PR #{i}',#{now},#{now})")
          else
            conn.execute("INSERT INTO events(id,room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(#{id},486777696,127326141,'Query event #{i}','2026-03-03 16:00:00','UTC',#{now},#{now})")
          end
          next if surface == 'unlinked'
          conn.execute("INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,event_id,github_pull_request_id,created_at,updated_at) VALUES(91,127326141,'#{surface}',#{surface == 'event' ? id : 'NULL'},#{surface == 'pull_request' ? id : 'NULL'},#{now},#{now})")
        end
      end
      room = Room.find(486777696)
      thread = ChannelThread.find(91)
      thread.room = room
      creator = User.find(127326141)
      posts = ChannelThread.where(id: 2000...2200).includes(:work_owner).to_a if bots
      queries = []
      callback = ActiveSupport::Notifications.subscribe('sql.active_record') do |*args|
        event = args.last
        queries << event if event[:name] != 'SCHEMA' && !event[:cached] && event[:sql].match?(/\A\s*(SELECT|WITH)/)
      end
      begin
        result = if bots
          ChannelThread.board_owner_active_map(room, posts)
        elsif surface == 'humans'
          thread.work_owner_candidates
        elsif surface == 'opening'
          ChannelThread.create_board_post!(room:, creator:, name: 'Opening query probe', work_status: 'planned', first_message: 'Opening message')
        else
          links = thread.work_thread_links.ordered.includes(:github_pull_request, :event).to_a
          links.each { |link| surface == 'event' ? link.event.title : link.github_pull_request.full_name }
          options = room.events.upcoming.soonest_first.where.not(id: thread.work_thread_links.where.not(event_id: nil).select(:event_id)).to_a
          [links, options]
        end
      ensure
        ActiveSupport::Notifications.unsubscribe(callback)
      end
      raise 'unavailable agent' if bots && (result.size != size || result.values.any? { |value| !value })
      raise 'wrong human choices' if surface == 'humans' && result[0].size != size
      if %w[unlinked event pull_request].include?(surface)
        raise 'wrong links/options' unless result[surface == 'unlinked' ? 1 : 0].size == size
      end
      rows << {surface:, size:, selects: queries.size}
      warn "Rails board #{surface} size=#{size}: #{queries.size} SELECTs"
      raise ActiveRecord::Rollback
    end
  end
end
puts JSON.pretty_generate(reference: 'd7c7de92 plus approved board drift', rows:)
