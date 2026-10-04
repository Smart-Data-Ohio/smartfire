require "json"
require "active_support/testing/time_helpers"

# ActivityItem#broadcast_updated and HuddleGrant#refresh_invitation! at the current reference pin.
# Rails emits these frames synchronously; only the delivery endpoints are observed.
class WS13bRingGenerations
  include ActiveSupport::Testing::TimeHelpers

  def setup
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    travel_to Time.utc(2026, 1, 1, 12)
    ENV["LIVEKIT_API_SECRET"] = "ws13b-review-fixture-value"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_INTERNAL_URL")
    @caller = User.find(ActiveRecord::FixtureSet.identify("david"))
    @recipient = User.find(ActiveRecord::FixtureSet.identify("jason"))
    @room = Room.find(ActiveRecord::FixtureSet.identify("david_and_jason"))
    @member = @room.memberships.find_by!(user: @caller)
    @session = @caller.sessions.create!(token: "ws13b-generation-session")
    @frames = []
    frames = @frames
    target = ActivityChannel.stream_name_for(@recipient.id)
    ActionCable.server.define_singleton_method(:broadcast) do |stream, payload|
      frames << payload if stream == target && payload[:huddleInvitation]
    end
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |*_| }
  end

  def issue(session = @session) = HuddleGrant.issue!(session: session, membership: @member)

  def phase(name, grant)
    item = ActivityItem.find_by(user: @recipient, source_type: "HuddleGrant")
    {name: name, grant_id: grant.id, invited_at: (item&.created_at || grant.reload.last_issued_at).iso8601(6),
     item_id: item&.id, source_id: item&.source_id, frames: @frames.shift(@frames.size)}
  end

  def run
    cases = %w[handled unhandled missed same_attempt_new_grant ended_retry suppressed_retry].map do |operation|
      setup
      @recipient.update!(inbox_preferences: {huddle_invitations: false}) if operation == "suppressed_retry"
      grant = issue
      initial = phase("initial", grant)
      item = ActivityItem.find_by(user: @recipient, source_type: "HuddleGrant")
      case operation
      when "handled" then item.mark_handled!
      when "missed" then item.update!(event_type: "huddle_missed")
      when "ended_retry", "suppressed_retry"
        grant.record_seen!
        grant.mark_out_of_call!
      end
      mutation = phase("mutation", grant)
      travel 181
      session = operation == "same_attempt_new_grant" ? @caller.sessions.create!(token: "ws13b-generation-retry") : @session
      retried = issue(session)
      retry_phase = phase("retry", retried)
      {operation: operation, phases: [initial, mutation, retry_phase]}
    end
    puts JSON.pretty_generate(reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], cases: cases)
  ensure
    travel_back
  end
end
WS13bRingGenerations.new.run
