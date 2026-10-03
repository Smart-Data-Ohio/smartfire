# Actual Rails destruction at the loaded-post/fresh-thread boundaries. No swallowed oracle errors.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/board_automations/dispatch-source-hashes.json'))).each do |file, hash|
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash
end
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new(File::NULL)
def destroy_review_board(id)
  return unless $review_destroy_room == id
  $review_destroy_room = nil
  Room.find(id).begin_destroy!
  Room::DestroyJob.perform_now(id)
end
module DigestDeletionRace
  def stale_posts_for(board, now:)
    posts = super
    destroy_review_board(board.id)
    posts
  end
end
module SlaDeletionRace
  def dispatch_thread!(rule, thread, now:, pushed_recipient_ids:)
    destroy_review_board(rule.room_id)
    super
  end
end
BoardAutomations::DigestDispatcher.singleton_class.prepend(DigestDeletionRace)
BoardAutomations::SlaDispatcher.singleton_class.prepend(SlaDeletionRace)
conn = ActiveRecord::Base.connection
setup = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'vectors/board_automations.json')))['sla'][0]['setup']
healthy_setup = [
  "UPDATE channel_threads SET work_status_changed_at='2026-03-02 12:00:00' WHERE id=970000001",
  "INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES(980900001,'Rooms::Board','Healthy board',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')",
  "INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES(980900001,127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')",
  "INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(980900001,'planned',60,240,'2026-03-02 16:00:00','2026-03-02 16:00:00')",
  "INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_status_changed_at,created_at,updated_at,last_activity_at) VALUES(980900002,980900001,127326141,'Healthy post','planned','2026-03-02 12:00:00','2026-03-02 12:00:00','2026-03-02 12:00:00','2026-03-02 12:00:00')"
]
rows = %w[digest sla].map do |kind|
  row = nil
  ActiveRecord::Base.transaction do
    (setup + healthy_setup).each { |sql| conn.execute(sql) }
    $review_destroy_room = ChannelThread.find(970000001).room_id
    destroyed_id = $review_destroy_room
    if kind == 'digest'
      BoardAutomations::DigestDispatcher.dispatch_due!(now: Time.current)
      claims = BoardStaleDigest.where(room_id: 980900001).where.not(message_id: nil).count
    else
      BoardAutomations::SlaDispatcher.dispatch_due!(now: Time.current)
      claims = BoardSlaNudge.where(room_id: 980900001).count
    end
    row = { kind:, destroyed: !Room.exists?(destroyed_id), healthy_claims: claims }
    raise 'healthy board did not finish' unless claims == (kind == 'digest' ? 1 : 2)
    raise 'destroy injection did not run' unless row[:destroyed]
    raise ActiveRecord::Rollback
  end
  ActiveSupport::IsolatedExecutionState.clear
  row
end
puts JSON.pretty_generate(rows:)
warn 'Rails deletion races: real Room::DestroyJob; vanished boards skipped; healthy digest=1 and SLA=2'
