require "json"
require "active_support/testing/time_helpers"
class HuddleJobContracts
  include ActiveSupport::Testing::TimeHelpers
  def reset
    travel_to Time.utc(2026, 1, 1, 12)
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships push/subscriptions])
    @david, @jason = %w[david jason].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
    ENV["LIVEKIT_API_SECRET"] = "ws13b-fixture-api-secret"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_INTERNAL_URL")
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |*_| }
  end
  def run
    invitations = %w[recipient off connected missing].map do |name|
      reset
      room = Room.find(ActiveRecord::FixtureSet.identify(:david_and_jason))
      member = room.memberships.find_by!(user: @jason)
      grant = HuddleGrant.issue!(session: @david.sessions.create!(token: "ws13b-job-session"), membership: room.memberships.find_by!(user: @david))
      item = ActivityItem.find_by!(user: @jason, source: grant)
      member.update!(involvement: "nothing") if name == "off"
      member.connected if name == "connected"
      input = {item: item.attributes, grant: grant.attributes.merge("identity" => "ws13b-job-participant"), session: grant.session.attributes, recipient_id: @jason.id, sender_id: @david.id, room_id: room.id, membership_id: member.id, involvement: member.involvement, connected_at: member.connected_at, missing: name == "missing"}
      item.destroy! if name == "missing"
      pushes = []
      Rails.configuration.x.web_push_pool.define_singleton_method(:queue) { |payload, subscriptions| pushes << {payload: payload, subscription_ids: subscriptions.order(:id).pluck(:id)} }
      Huddle::PushInvitationJob.perform_now(item.id)
      {name: name, input: input, pushes: pushes}
    end
    reset
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "ws13b-fixture-api-key"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "ws13b-fixture-gateway-secret"
    room = Rooms::Voice.create!(id: 9001, creator: @david, name: "Lounge")
    [@david, @jason].each_with_index { |u,i|room.memberships.create!(id:9011+i,user:u) }
    grant = HuddleGrant.issue!(session:@david.sessions.create!(token:"ws13b-presence-job-session"),membership:room.memberships.find_by!(user:@david))
    grant.update_columns(last_seen_at:Time.current)
    streams = []
    ActionCable.server.define_singleton_method(:broadcast) { |stream, _| streams << stream }
    Huddle::BroadcastPresenceJob.perform_now(grant.id)
    presence = {counts: streams.tally, missing: []}
    streams.clear
    Huddle::BroadcastPresenceJob.perform_now(-1)
    grant.update_columns(room_id:-1)
    Huddle::BroadcastPresenceJob.perform_now(grant.id)
    presence[:missing] = streams
    # The original orchestrator declarations observe the phase calls and the
    # exception-class-only log. Its own reconcile_once method remains real.
    phases = %w[normal resolver_failure stream_failure resolver_sql_failure stream_validation_failure].map do |name|
      calls, logs = [], []
      Huddle::InvitationResolver.define_singleton_method(:resolve_overdue!) { calls << "invitations"; raise StandardError,"boom" if name == "resolver_failure"; raise ActiveRecord::StatementInvalid,"boom" if name == "resolver_sql_failure" }
      Stream.define_singleton_method(:end_stale_live!) { calls << "streams"; raise StandardError,"boom" if name == "stream_failure"; raise ActiveRecord::RecordInvalid.new(Stream.new) if name == "stream_validation_failure" }
      HuddleCleanup.define_singleton_method(:reconcile_now) { calls << "cleanup"; 0 }
      Rails.logger.define_singleton_method(:error) { |message| logs << message }
      Huddle::Reconciler.new.reconcile_once
      {name:name,calls:calls,logs:logs}
    end
    puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:Time.current.to_i,invitations:invitations,presence:presence,reconciler:phases)
  ensure
    travel_back
  end
end
HuddleJobContracts.new.run
