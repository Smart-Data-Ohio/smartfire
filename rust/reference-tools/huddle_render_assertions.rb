require "json"
require "active_support/testing/time_helpers"

# Capture the actual callback-produced Turbo bytes, including each viewer's
# panel. Destinations are those asserted by the original model declarations.
class HuddleRenderAssertions
  include ActiveSupport::Testing::TimeHelpers
  NOW = Time.utc(2026, 3, 2, 16)
  def reset
    travel_to NOW
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships sessions])
    %w[URL INTERNAL_URL API_KEY API_SECRET GATEWAY_SECRET].zip(%w[wss://public.example.test http://internal.example.test:7880 ws13-fixture-api-key ws13-fixture-api-secret ws13-fixture-gateway-secret]).each { |key,value| ENV["LIVEKIT_#{key}"] = value }
    Rails.application.routes.default_url_options[:host] = "example.org"
    @david, @jason, @kevin = %w[david jason kevin].map { |k| User.find(ActiveRecord::FixtureSet.identify(k)) }
    @frames, @jobs = [], []
    frames, jobs = @frames, @jobs
    ActionCable.server.define_singleton_method(:broadcast) { |stream, data| frames << { stream: stream, html: data } }
    Huddle::BroadcastPresenceJob.define_singleton_method(:perform_later) { |id| jobs << id }
    Huddle::JoinNoticeJob.define_singleton_method(:perform_later) { |*_| }
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |*_| }
  end
  def definitions
    [
      ["mark_out", "watercooler", ["mark_out"], "seen"],
      ["issue_voice", "voice", ["issue"], nil],
      ["revoke_voice", "voice", ["revoke"], "issue"],
      ["first_voice", "voice_solo", ["seen", "presence_job", "repeat_seen"], "issue"],
      ["issue_open", "hq", ["issue"], nil],
      ["issue_closed", "watercooler", ["issue"], nil],
      ["issue_direct", "david_and_jason", ["issue"], nil],
      ["revoke_channel", "watercooler", ["revoke"], "issue"],
      ["revoke_direct", "david_and_jason", ["revoke"], "issue"],
      ["first_channel", "watercooler", ["seen", "presence_job", "repeat_seen"], "issue"],
      ["unconfigured", "watercooler", ["issue", "seen", "presence_job", "revoke"], nil],
      ["destroyed_room", "voice_solo", ["destroy"], "issue"],
      ["issue_stage", "stage_two", ["issue"], nil],
      ["revoke_stage", "stage_two", ["revoke"], "issue"],
      ["stream_start", "stage", ["stream_start"], nil],
      ["stream_end", "stage", ["stream_end"], "stream"],
      ["stream_twice", "stage", ["stream_end", "stream_end"], "stream"],
      ["host_stop", "stage", ["host_stop"], "speaker_stream"],
      ["self_stop", "stage", ["self_stop"], "stream"],
      ["automatic_stop", "stage", ["stream_end"], "stream"]
    ]
  end
  def room_for(name)
    return Room.find(ActiveRecord::FixtureSet.identify(name)) unless name.start_with?("voice", "stage")
    type = name.start_with?("voice") ? Rooms::Voice : Rooms::Stage
    room = type.create!(id: 9001, name: type == Rooms::Voice ? "Lounge" : "Town Hall", creator: @david)
    users = name == "voice_solo" ? [@david] : name == "stage" ? [@david,@jason,@kevin] : [@david,@jason]
    users.each_with_index { |u,i| room.memberships.create!(id: 9011+i,user: u,stage_role: type == Rooms::Stage ? (i == 0 ? "host" : "listener") : nil) }
    room
  end
  def issue
    @grant = HuddleGrant.issue!(session: Session.find(ActiveRecord::FixtureSet.identify(:david_safari)), membership: @member)
  end
  def stream(member = @member)
    @stream = Stream.create!(room: @room, membership: member, user: member.user, quality: "1080p15")
  end
  def run
    cases = definitions.map do |name,room,operations,setup|
      reset
      ENV.delete("LIVEKIT_GATEWAY_SECRET") if name == "unconfigured"
      @room = room_for(room)
      @member = @room.memberships.find_by!(user: @david)
      issue if %w[issue seen].include?(setup)
      @grant.update_columns(last_seen_at: Time.current) if setup == "seen"
      stream if setup == "stream"
      if setup == "speaker_stream"
        member = @room.memberships.find_by!(user: @jason)
        member.update!(stage_role: "speaker")
        stream(member)
      end
      users = User.order(:id).map { |u| u.attributes.slice("id","name","role","status","updated_at") }
      # Use a deterministic setup coordinate, without masking any rendered bytes.
      @grant&.update_columns(identity: "ws13b-render-#{@grant.id}")
      input = { room: @room.attributes, memberships: @room.memberships.order(:id).map(&:attributes), users: users, grant: @grant&.attributes, stream: @stream&.attributes, session_id: Session.find(ActiveRecord::FixtureSet.identify(:david_safari)).id }
      destinations = [{ kind: "header", user_id: @david.id, stream: Turbo::StreamsChannel.send(:stream_name_from, [@room,:messages]) }]
      unless name == "destroyed_room"
        users.each { |u| destinations << {kind: "sidebar", user_id: u["id"], stream: Turbo::StreamsChannel.send(:stream_name_from,[User.find(u["id"]),:rooms])} }
      end
      steps = operations.map do |op|
        @frames.clear; @jobs.clear
        value = case op
        when "issue"; issue; true
        when "seen"; @grant.record_seen!; nil
        when "repeat_seen"; travel_to(NOW+11); @grant.record_seen!; nil
        when "presence_job"; Huddle::BroadcastPresenceJob.perform_now(@grant.id); nil
        when "revoke"; @grant.revoke!; nil
        when "mark_out"; @grant.mark_out_of_call!
        when "destroy"; @room.destroy!; nil
        when "stream_start"; stream; nil
        when "stream_end"; @stream.end!; nil
        when "host_stop"; @stream.end!(ended_by: @david); nil
        when "self_stop"; @stream.end!(ended_by: @david); nil
        end
        grant_state = @grant && {last_seen_at: @grant.reload.last_seen_at&.to_i, revoked: @grant.revoked?, in_call: @grant.in_call?}
        {op: op, value: value, grant_state: grant_state, presence_jobs: @jobs.dup, frames: destinations.to_h { |d| [d[:stream], @frames.select { |f| f[:stream] == d[:stream] }.map { |f| f[:html] }] }}
      end
      @grant = @stream = nil
      {name: name, configured: name != "unconfigured", input: input, destinations: destinations, steps: steps}
    end
    puts JSON.pretty_generate(reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: NOW.to_i, cases: cases)
  ensure
    travel_back
  end
end
HuddleRenderAssertions.new.run
