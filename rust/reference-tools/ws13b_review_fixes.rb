require "json"
require "active_support/testing/time_helpers"

# Probe the real issuance, revocation callbacks and gateway parser at d7c7de92.
# Rails broadcasts rings synchronously; there is no deferred Rails ring job.
class WS13bReviewFixes
  include ActiveSupport::Testing::TimeHelpers

  def setup
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    travel_to Time.utc(2026, 1, 1, 12)
    ENV["LIVEKIT_API_SECRET"] = "ws13b-review-fixture-value"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_INTERNAL_URL")
    @frames = []
    frames = @frames
    ActionCable.server.define_singleton_method(:broadcast) do |stream, payload|
      frames << payload if stream.end_with?("_activity") && payload[:huddleInvitation]
    end
    @jobs = []
    @job_classes = []
    jobs = @jobs
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |id| jobs << id }
    @caller = User.find(ActiveRecord::FixtureSet.identify("david"))
    @recipient = User.find(ActiveRecord::FixtureSet.identify("jason"))
    @room = Room.find(ActiveRecord::FixtureSet.identify("david_and_jason"))
    @membership = @room.memberships.find_by!(user: @caller)
    @session = @caller.sessions.create!(token: "ws13b-review-session")
  end

  def issue = HuddleGrant.issue!(session: @session, membership: @membership)

  def run
    observer = ActiveSupport::Notifications.subscribe("enqueue.active_job") do |event|
      @job_classes << event.payload[:job].class.name
    end
    rings = %w[revoke remove_recipient remove_caller sign_out leave suppressed_revoke suppressed_leave].map do |operation|
      setup
      @recipient.update!(inbox_preferences: {huddle_invitations: false}) if operation.start_with?("suppressed_")
      grant = issue
      initial = @frames.map { |frame| frame[:huddleInvitation].slice(:eventType, :state) }
      @frames.clear
      grant.record_seen! unless operation == "remove_recipient"
      case operation
      when "revoke", "suppressed_revoke" then grant.revoke!(create_cleanup: false)
      when "remove_recipient" then @room.memberships.find_by!(user: @recipient).destroy!
      when "remove_caller" then @membership.destroy!
      when "sign_out" then @session.destroy!
      when "leave", "suppressed_leave" then grant.mark_out_of_call!
      end
      {operation: operation, initial: initial,
       after_mutation: @frames.map { |frame| frame[:huddleInvitation].slice(:eventType, :state) },
       ring_job_enqueued: @job_classes.include?("Notifications::HuddleRingJob")}
    end
    failures = %w[create reuse group].map do |operation|
      setup
      target = ""
      if operation == "group"
        kevin = User.find(ActiveRecord::FixtureSet.identify("kevin"))
        @room.memberships.create!(user: kevin, involvement: "everything")
        target = " AND NEW.user_id=#{kevin.id}"
      end
      if operation == "reuse"
        issue
        ActivityItem.delete_all
        travel 180
      end
      @jobs.clear
      ActiveRecord::Base.connection.execute("CREATE TRIGGER reject_huddle_item BEFORE INSERT ON activity_items WHEN NEW.event_type='huddle_started'#{target} BEGIN SELECT RAISE(ABORT,'review invitation failure'); END")
      begin
        issue
        raise "expected invitation failure"
      rescue ActiveRecord::StatementInvalid => error
        {operation: operation, error: error.class.name, grants: HuddleGrant.where(session: @session).count,
         items: ActivityItem.where(source_type: "HuddleGrant").count,
         last_issued_at: HuddleGrant.find_by!(session: @session).last_issued_at.iso8601,
         push_jobs: @jobs.size}
      end
    end
    setup
    inputs = ["2026-01-01 17:00:00 +0500", "2026-01-01 07:00:00 -0500",
      "2026-01-01 17:00:00 +05:00", "2026-01-01T17:00:00+0500",
      "2026-01-01T12:00:00Z", "2026-01-01 17:00:00.500000 +0500",
      "2026-01-01T24:00:00Z", "2026-01-01T24:01:00Z", "2026-01-01T24:00:01Z",
      "2026-01-01T12:00:60Z", "2026-01-01 17:00:00 +2500"]
    times = inputs.map do |input|
      controller = Internal::HuddleController.new
      controller.params = ActionController::Parameters.new(disconnected_at: input)
      floor = controller.send(:disconnected_at_param)
      {input: input, parsed: floor&.iso8601(6)}
    end
    boundaries = [-1, 0, 1].map do |seconds|
      setup
      grant = issue
      seen = Time.utc(2026, 1, 1, 12) + seconds
      grant.update_columns(last_seen_at: seen)
      controller = Internal::HuddleController.new
      controller.params = ActionController::Parameters.new(disconnected_at: inputs.first)
      floor = controller.send(:disconnected_at_param)
      changed = grant.mark_out_of_call!(seen_after: floor)
      {seen_at: seen.iso8601(6), changed: changed, seen_after: grant.reload.last_seen_at&.iso8601(6)}
    end
    puts JSON.pretty_generate(reference_pin: "d7c7de92", rings: rings, failures: failures, times: times, boundaries: boundaries)
  ensure
    ActiveSupport::Notifications.unsubscribe(observer) if observer
    travel_back
  end
end
WS13bReviewFixes.new.run
