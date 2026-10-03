# Original activity_items_controller_test.rb:198: both source-body producers' copy.
require 'digest'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
%w[app/helpers/activity_items_helper.rb test/controllers/activity_items_controller_test.rb].each do |path|
  expected = {'app/helpers/activity_items_helper.rb'=>'40a2231e001e46fe4b2838102759d9c4d5c1b8cba2817ce1dfcf5fe5c3d96d19', 'test/controllers/activity_items_controller_test.rb'=>'3cfcd2915ff8d6fd504c46534bdca089b31e21ab3b4d0b90617e49edfe4b337b'}.fetch(path)
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == expected
end
ApplicationJob.queue_adapter = :test
helper = Object.new.extend(ActivityItemsHelper)
rows = []
travel_to Time.utc(2026, 3, 2, 16) do
  %w[jason kevin].zip(%w[huddle_missed huddle_started]).each do |name, kind|
    caller = User.find(ActiveRecord::FixtureSet.identify(name))
    # Persist the actual polymorphic source; identity, session and membership are inputs.
    room = Room.find(ActiveRecord::FixtureSet.identify(name == 'jason' ? 'david_and_jason' : 'david_and_kevin'))
    session = caller.sessions.create!
    grant = HuddleGrant.create!(user: caller, session: session, membership: room.memberships.find_or_create_by!(user: caller), room: room, identity: "ws12-copy-#{name}", room_name: "ws12-copy-#{name}", last_issued_at: Time.current)
    item = ActivityItem.new(user_id: ActiveRecord::FixtureSet.identify('david'), source: grant, event_type: kind)
    rows << {event_type: kind, caller_id: caller.id, room_id: room.id, label: helper.activity_item_event_label(item), author: caller.name, body: helper.activity_item_source_body(item)}
  end
end
puts JSON.pretty_generate({rows: rows})
warn "WS12_HUDDLE_COPY_RAILS 2 persisted-source comparisons; 0 masks"
