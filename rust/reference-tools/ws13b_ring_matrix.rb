require "json"
require "digest"
require "active_support/testing/time_helpers"

# Rails has synchronous Cable rings, not Notifications::HuddleRingJob. Observe
# those real callbacks first; project their delivery through Rust's extra queue.
# Never infer an invitation emission from last_issued_at or from a case name.
module WS13bRingMatrixItemObserver
  def broadcast_activity_change
    WS13bRingMatrix.current.with_context(kind: "item", item: self) { super }
  end
end
module WS13bRingMatrixGrantObserver
  def broadcast_suppressed_invitation!(recipient)
    WS13bRingMatrix.current.with_context(kind: "banner", grant: self) { super }
  end
  def broadcast_call_ended_to_invitee
    probe = WS13bRingMatrix.current
    probe.end_call(room_id) if room && user && !others_in_call?
    super
  end
end
ActivityItem.prepend(WS13bRingMatrixItemObserver)
HuddleGrant.prepend(WS13bRingMatrixGrantObserver)

class WS13bRingMatrix
  include ActiveSupport::Testing::TimeHelpers
  class << self
    attr_accessor :current
  end

  def specs
    families = []
    [0, 1, 120, 121, 181].each do |delay|
      families << ["pending_initial_reissue_#{delay}", [step("issue"), step("issue", delay), step("drain")]]
      families << ["pending_retry_reissue_#{delay}", [step("issue"), step("drain"), step("issue", 181), step("issue", delay), step("drain")]]
    end
    families << ["two_pending_retries", [step("issue"), step("issue", 181), step("issue", 181), step("drain")]]
    families << ["sliding_dedupe", [step("issue"), step("issue", 0), step("issue", 1), step("issue", 120), step("issue", 1), step("drain")]]
    [0, 1, 181].each { |delay| families << ["delayed_initial_#{delay}", [step("issue"), step("drain", delay)]] }
    %w[quiet_revoke live_revoke end remove_recipient remove_caller sign_out].each do |action|
      [false, true].each do |delivered|
        steps = [step("issue")]
        steps << step("drain") if delivered
        steps += [step(action, 1), step("drain")]
        families << ["#{action}_#{delivered ? 'delivered' : 'pending'}", steps]
      end
    end
    %w[live_revoke end].each do |action|
      [0, 1, 181].each { |delay| families << ["#{action}_regrant_#{delay}", [step("issue"), step(action), step("issue", delay), step("drain")]] }
      families << ["#{action}_regrant_dedupe", [step("issue"), step(action), step("issue", 181), step("issue", 1), step("drain")]]
    end
    [0, 1, 181].each { |delay| families << ["new_session_#{delay}", [step("issue"), step("new_session", delay), step("drain")]] }
    families << ["group_continues", [step("issue"), step("group_continues", 1), step("drain")]]
    cases = families.flat_map do |name, steps|
      [false, true].product([false, true]).map do |banner, newest|
        {name: "#{name}/#{banner ? 'banner' : 'item'}/#{newest ? 'newest' : 'oldest'}", banner: banner, newest_first: newest, steps: steps}
      end
    end
    %w[read handled missed unread_cycle].each do |mutation|
      [0, 1, 181].product([false, true]).each do |delay, newest|
        steps = [step("issue"), step("drain"), step(mutation, mutation == "missed" ? 46 : 1), step("issue", delay), step("drain")]
        cases << {name: "#{mutation}_retry_#{delay}/item/#{newest ? 'newest' : 'oldest'}", banner: false, newest_first: newest, steps: steps}
      end
    end
    cases
  end

  def step(action, seconds = 0) = {action: action, seconds: seconds}

  def setup(spec)
    self.class.current = self
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
    @session = @caller.sessions.create!(token: "ws13b-matrix-session")
    @recipient.update!(inbox_preferences: {huddle_invitations: false}) if spec[:banner]
    @pending, @emissions, @immediate, @pushes = [], [], [], []
    @next_emission = 0
    @context = nil
    @projection = false
    probe = self
    ActionCable.server.define_singleton_method(:broadcast) { |stream, payload| probe.observe(stream, payload) }
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |id| probe.push(id) }
  end

  def with_context(context)
    previous = @context
    @context = context
    yield
  ensure
    @context = previous
  end

  def push(id) = @pushes << id

  def observe(stream, payload)
    return unless stream == ActivityChannel.stream_name_for(@recipient.id) && payload[:huddleInvitation]
    frame = JSON.parse(JSON.generate(payload))
    if @projection
      @projected << frame
    elsif @context
      @emissions << frame
      @next_emission += 1
      item = @context[:item]
      grant = item ? item.source : @context[:grant]
      target = item ? ["item", item.id] : ["banner", @recipient.id, grant.room_id, grant.user_id]
      @pending.each { |old| old[:superseded] = true if old[:target] == target }
      @pending << {id: @next_emission, target: target, item_id: item&.id, source_id: grant.id,
        created_at: item&.created_at, event_type: item&.event_type, state: item&.state, room_id: grant.room_id}
    else
      @immediate << frame
    end
  end

  # Observe the guarded last-participant end callback, even when its banner's
  # one-minute ended-frame window has expired. This is explicitly queue projection.
  def end_call(room_id)
    @pending.each { |job| job[:ended] = true if job[:room_id] == room_id && (!job[:item_id] || job[:event_type] == "huddle_started") }
  end

  def issue = @grant = HuddleGrant.issue!(session: @session, membership: @member)

  def drain(spec)
    @projected = []
    jobs = @pending.sort_by { |job| job[:id] }
    jobs.reverse! if spec[:newest_first]
    jobs.each do |job|
      next if job[:superseded] || job[:ended]
      grant = HuddleGrant.find_by(id: job[:source_id])
      next unless grant && @recipient.reload.active? && !@recipient.bot?
      membership = Membership.find_by(room_id: grant.room_id, user_id: @recipient.id)
      next unless membership
      item = ActivityItem.find_by(id: job[:item_id]) if job[:item_id]
      if job[:item_id]
        next unless item && item.user_id == @recipient.id && item.source_id == job[:source_id] &&
          item.created_at == job[:created_at] && item.event_type == job[:event_type] && item.state == job[:state]
      end
      if !item || (item.event_type == "huddle_started" && item.unread?)
        next unless Room.alive.exists?(id: grant.room_id)
        next if %w[nothing invisible].include?(membership.involvement)
        next if grant.revoked? && !HuddleGrant.active.in_call.where(room_id: grant.room_id).exists?
      end
      @projection = true
      if item
        # Invoke the Rails payload builder; no hand-authored expected payloads.
        observe(ActivityChannel.stream_name_for(@recipient.id), item.send(:activity_broadcast_payload))
      else
        grant.send(:broadcast_suppressed_invitation!, @recipient)
      end
      @projection = false
    end
    @pending.clear
    @projected
  ensure
    @projection = false
  end

  def apply(action)
    case action
    when "issue" then issue
    when "new_session"
      @session = @caller.sessions.create!(token: "ws13b-matrix-second-session")
      issue
    when "quiet_revoke" then @grant.revoke!(create_cleanup: false)
    when "live_revoke" then @grant.record_seen!; @grant.revoke!(create_cleanup: false)
    when "end" then @grant.record_seen!; @grant.mark_out_of_call!
    when "remove_recipient" then @room.memberships.find_by!(user: @recipient).destroy!
    when "remove_caller" then @grant.record_seen!; @member.destroy!
    when "sign_out" then @grant.record_seen!; @session.destroy!
    when "group_continues"
      other = User.find(ActiveRecord::FixtureSet.identify("kevin"))
      member = @room.memberships.create!(user: other, involvement: "everything")
      session = other.sessions.create!(token: "ws13b-matrix-group-session")
      live = HuddleGrant.issue!(session: session, membership: member)
      live.record_seen!
      @grant.record_seen!
      @grant.revoke!(create_cleanup: false)
    when "read" then item.mark_read!
    when "handled" then item.mark_handled!
    when "missed" then Huddle::InvitationResolver.resolve_overdue!(user: @recipient)
    when "unread_cycle" then item.mark_handled!; item.mark_unread!
    else raise "unknown action #{action}"
    end
  end

  def item = ActivityItem.find_by!(user: @recipient, source_type: "HuddleGrant")

  def snapshot
    ActivityItem.where(user: @recipient, source_type: "HuddleGrant").order(:id).map do |row|
      {id: row.id, source_id: row.source_id, event_type: row.event_type, state: row.state, created_at: row.created_at.iso8601(6)}
    end
  end

  def run
    cases = specs.map do |spec|
      setup(spec)
      phases = spec[:steps].map do |step|
        travel step[:seconds]
        delivered = step[:action] == "drain" ? drain(spec) : (apply(step[:action]); [])
        {emissions: @emissions.shift(@emissions.size), immediate: @immediate.shift(@immediate.size),
         pushes: @pushes.shift(@pushes.size), delivered: delivered, items: snapshot}
      end
      {spec: spec, phases: phases}
    end
    files = %w[app/models/huddle_grant.rb app/models/activity_item.rb app/models/huddle/ring_policy.rb app/models/huddle/invitation_resolver.rb app/jobs/huddle/push_invitation_job.rb app/models/huddle/invitation_pusher.rb]
    puts JSON.pretty_generate(reference_pin: "d7c7de92", source_sha256: files.to_h { |file| [file, Digest::SHA256.file(Rails.root.join(file)).hexdigest] },
      projection: "Rails synchronous callback emissions through Rust deferred queue; not a Rails ring job", cases: cases)
  ensure
    travel_back
  end
end
WS13bRingMatrix.new.run
