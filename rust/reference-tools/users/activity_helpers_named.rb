# The four exact ActivityItemsHelper declarations; invoke actual helpers on real sources.
require "digest"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/activity-helpers-source-hashes.json"))).each do |path, hash|
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
identify = ->(name) { ActiveRecord::FixtureSet.identify(name) }
helper = Object.new.extend(ActivityItemsHelper).extend(Rails.application.routes.url_helpers)
def helper.url_options; {}; end
rows = []
setup = {}
travel_to Time.utc(2026, 3, 2, 16) do
  david = User.find(identify.call("david"))
  review = ActivityItem.new(event_type: "pr_review_request")
  rows << {key: "review", facts: {label: helper.activity_item_event_label(review)}}
  event = Event.find(identify.call("launch_party"))
  voice = Rooms::Voice.create_for({name: "Lounge", creator: david}, users: [david])
  event.update!(venue_room_id: voice.id)
  reminder = ActivityItem.new(user: david, source: event, event_type: "event_reminder")
  rows << {key: "venue", facts: {body: helper.activity_item_event_body(reminder)}}
  setup["rooms"] = [ActiveRecord::Base.connection.select_one("SELECT * FROM rooms WHERE id=#{voice.id}")]
  setup["events"] = [ActiveRecord::Base.connection.select_one("SELECT * FROM events WHERE id=#{event.id}")]
  event.update!(venue_room_id: nil)
  rows << {key: "no_venue", facts: {body: helper.activity_item_event_body(reminder)}}
  TwoFactorCredential.where(user: david).delete_all
  credential = TwoFactorCredential.create!(user: david, secret: TwoFactorCredential.generate_secret, confirmed_at: Time.current)
  alert = ActivityItem.new(user: david, source: credential, event_type: "two_factor_lockout")
  rows << {key: "lockout", facts: {label: helper.activity_item_event_label(alert), body: helper.activity_item_source_body(alert), title: helper.activity_item_source_label(alert), path: helper.activity_item_source_path(alert)}}
  setup["two_factor_credentials"] = [ActiveRecord::Base.connection.select_one("SELECT * FROM two_factor_credentials WHERE id=#{credential.id}")]
end
puts JSON.pretty_generate({setup: setup, rows: rows}.as_json)
warn "Rails activity helper named oracle: #{rows.size} complete assertion sets; 0 masks"
