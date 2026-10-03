# Independent PR215 race oracle. Real source saves and after-commit Cable transport;
# a second ActiveRecord connection changes the next recipient between commits.
require "json"
require "digest"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
hashes = {
  "app/models/work_thread_event.rb" => "d6b62b3f5bab3141a3284be84b5ad387a127430754b880fb7db0aa0a6b430c19",
  "app/models/activity_item.rb" => "2e6d1915f243be790d6b82dedbdd7525fda0d849d705935bb9ffae38de69eb29",
  "app/services/activity_items/recorder.rb" => "cc8edf555e5e0b337e0f54e6c333a61865a38516e605dfa7387053802917fdac",
  "app/models/channel_thread.rb" => "89f301160437d1d35b61df1c9a54351fd31d376389517ae6ced9bfcfa3581519",
  "app/channels/activity_channel.rb" => "6620fab78c1d74529f0c06a8a3d7482be7dd50c0d7df3fdea36d728bbd4a20fe"
}
hashes.each do |path, hash|
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ApplicationJob.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
identify = ->(name) { ActiveRecord::FixtureSet.identify(name) }
travel_to Time.utc(2026, 3, 2, 16) do
  actor = User.find(identify.call("david"))
  first = User.find(identify.call("jason"))
  later = User.find(identify.call("kevin"))
  [first, later].each { |user| user.update_columns(status: User.statuses.fetch("active")) }
  room = Rooms::Board.create!(name: "Recorder race review", creator: actor)
  [actor, first, later].each do |user|
    membership = room.memberships.find_or_create_by!(user: user)
    membership.update_columns(involvement: "everything")
  end
  thread = ChannelThread.create_board_post!(room: room, creator: first, name: "Plan",
    work_status: "planned", owner_id: later.id)
  ActivityItem.delete_all
  first_stream = ActivityChannel.stream_name_for(first.id)
  later_stream = ActivityChannel.stream_name_for(later.id)
  armed = true
  frames = []
  concurrent_connection_id = nil
  main_connection_id = ActiveRecord::Base.connection.object_id
  deactivated = User.statuses.fetch("deactivated")
  observer = ->(*args) do
    payload = args.last
    frames << { stream: payload[:broadcasting], payload: payload[:message] }
    if armed && payload[:broadcasting] == first_stream
      armed = false
      Thread.new do
        ActiveRecord::Base.connection_pool.with_connection do |connection|
          concurrent_connection_id = connection.object_id
          User.where(id: later.id).update_all(status: deactivated)
        end
      end.value
    end
  end
  ActiveSupport::Notifications.subscribed(observer, "broadcast.action_cable") do
    thread.update_work!(actor: actor, work_status: "done")
  end
  raise "first recipient callback was not observed" if armed
  raise "deactivation did not use a second connection" if concurrent_connection_id == main_connection_id
  event = thread.work_thread_events.order(:id).last!
  later_rows = ActivityItem.where(user_id: later.id, source_type: "WorkThreadEvent", source_id: event.id).count
  later_frames = frames.count { |frame| frame[:stream] == later_stream }
  result = {
    reference: "746d69cb Rails source hashes pinned above", source_event_id: event.id,
    later_recipient_active: User.find(later.id).active?, persisted_items: later_rows,
    activity_frames: later_frames, second_connection: concurrent_connection_id != main_connection_id,
    frames: frames
  }
  puts JSON.pretty_generate(result)
  raise "later recipient became active again" if result[:later_recipient_active]
  raise "inherited recipient snapshot insertion changed" unless later_rows == 1
  raise "Rails broadcast a deactivated recipient" unless later_frames == 0
  warn "PR215_RAILS_RECORDER_RACE later_recipient_active=false persisted_items=#{later_rows} activity_frames=#{later_frames} second_connection=true"
end
