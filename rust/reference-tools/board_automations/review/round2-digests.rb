# Real Rails dispatch/render/publish. Destroy a loaded event's venue before its
# association is read; separately fail one board's broadcast after its note commits.
require 'json'
require 'digest'
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/board_automations/dispatch-source-hashes.json'))).each do |file,hash|
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest==hash
end
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
Rails.application.routes.default_url_options={host:'example.com',protocol:'http'}
Rails.logger=ActiveSupport::Logger.new($stderr)
conn=ActiveRecord::Base.connection
setup=['DELETE FROM board_sla_rules','DELETE FROM board_stale_digests','DELETE FROM audit_logs',"UPDATE sqlite_sequence SET seq=9000000000 WHERE name='messages'"]
34.times do |i|
  room=974000000+i
  title=i==33 ? "Late event http://example.com/rooms/#{room}/events/974900001" : "Healthy post #{i}"
  setup += ["INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES(#{room},'Rooms::Board','Broadcast board #{i}',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')",
    "INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES(#{room},127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')",
    "INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(#{room},'planned',60,240,'2026-03-02 16:00:00','2026-03-02 16:00:00')",
    "INSERT INTO channel_threads(room_id,creator_id,name,work_status,work_status_changed_at,created_at,updated_at,last_activity_at) VALUES(#{room},127326141,#{conn.quote(title)},'planned','2026-03-02 12:00:00','2026-03-02 12:00:00','2026-03-02 12:00:00','2026-03-02 12:00:00')"]
end
setup += ["INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES(974900000,'Rooms::Voice','Vanishing venue',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')",
  "INSERT INTO events(id,room_id,organizer_id,title,starts_at,time_zone,venue_room_id,created_at,updated_at) VALUES(974900001,974000033,127326141,'Late event','2026-03-02 17:00:00','UTC',974900000,'2026-03-02 16:00:00','2026-03-02 16:00:00')",
  "CREATE TRIGGER round2_message_id AFTER INSERT ON messages WHEN NEW.system_note=1 AND NEW.room_id>=974000000 BEGIN UPDATE messages SET client_message_id='round2-' || NEW.room_id WHERE id=NEW.id; END"]
$round2_frames=[]
Message.before_create { self.client_message_id="round2-#{room_id}" if system_note? && room_id.between?(974000000,974000033) }
ActionCable.server.define_singleton_method(:broadcast) { |stream,payload,*| $round2_frames << {stream:stream,payload:payload} }
module Round2DigestBroadcast
  def broadcast_create
    if room_id==974000033
      if $round2_kind=='missing-venue'
        # Retain the loaded Event with its old venue ID, just like the Rust preload race.
        events.to_a
        Room.find(974900000).destroy!
        raise 'venue association did not degrade' unless events.first.venue.nil?
      elsif $round2_kind=='failed-broadcast'
        raise 'injected broadcast failure'
      end
    end
    super
  end
end
Message.prepend(Round2DigestBroadcast)
rows=[]
%w[missing-venue failed-broadcast].each do |kind|
    Room.where(id:(974000000..974000033).to_a+[974900000]).destroy_all
    conn.transaction { setup.each { |sql|conn.execute(sql) } }
    $round2_frames=[]; $round2_kind=kind
    BoardAutomations::DigestDispatcher.dispatch_due!(now:Time.current)
    frames=$round2_frames.select { |f| f[:payload].is_a?(String) && f[:payload].include?('action="append"') }
    facts=BoardStaleDigest.order(:room_id).map { |d|[d.room_id,!!d.message_id] }
    before=frames.size
    BoardAutomations::DigestDispatcher.dispatch_due!(now:Time.current)
    raise 'repeat published again' unless $round2_frames.select { |f| f[:payload].is_a?(String) && f[:payload].include?('action="append"') }.size==before
    rows << {kind:kind,setup:setup,claims:facts,frames:frames,notes:Message.where(system_note:true,room_id:974000000..974000033).count}
    conn.execute('DROP TRIGGER round2_message_id')
end
puts JSON.pretty_generate(rows:rows)
warn "Rails round2 digest oracle: 34 boards; missing venue publishes=#{rows[0][:frames].size}; failed broadcast publishes=#{rows[1][:frames].size}; failed claim attached=#{rows[1][:claims].last[1]}; repeats=0"
