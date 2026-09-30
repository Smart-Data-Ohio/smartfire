require "json"
require "active_support/testing/time_helpers"
class HuddleRingPolicySeam
  include ActiveSupport::Testing::TimeHelpers
  def setup_sql(recipient)
    connection=ActiveRecord::Base.connection
    fields=%w[presence_setting dnd_enabled dnd_until quiet_hours_enabled quiet_hours_start_minute quiet_hours_end_minute time_zone meeting_status_enabled meeting_dnd_enabled ooo_until ooo_notify_enabled]
    recipient.reload
    assignments=fields.map { |key| "#{key}=#{connection.quote(recipient.attributes_before_type_cast[key])}" }.join(",")
    statements=["UPDATE users SET #{assignments} WHERE id=#{recipient.id}","DELETE FROM dnd_allowed_users WHERE user_id=#{recipient.id}","DELETE FROM calendar_meeting_caches WHERE user_id=#{recipient.id}"]
    records=DndAllowedUser.where(user:recipient).order(:id).to_a
    records << Calendar::MeetingCache.find_by(user:recipient)
    records.compact.each do |record|
      attrs=record.reload.attributes_before_type_cast
      statements << "INSERT INTO #{record.class.table_name}(#{attrs.keys.join(',')}) VALUES(#{attrs.values.map { |v| connection.quote(v) }.join(',')})"
    end
    statements.join("; ")+";"
  end
  def run
    ActiveRecord::Schema.verbose=false
    titles=File.read(Rails.root.join("test/models/huddle/ring_policy_test.rb")).scan(/^  test "(.*)" do$/).flatten
    cases=%w[default dnd allowed_dnd quiet_override meeting allowed_meeting ooo allowed_ooo].each_with_index.map do |name,index|
      travel_to Time.utc(2026,9,23,12)
      load Rails.root.join("db/schema.rb")
      ActiveRecord::FixtureSet.reset_cache
      ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"),%w[accounts users rooms memberships sessions])
      ENV["LIVEKIT_API_SECRET"]="ws13b-fixture-api-secret"
      ENV.delete("LIVEKIT_URL")
      Huddle::RingPolicy.quiet_check=nil
      ActionCable.server.define_singleton_method(:broadcast) { |*_| }
      Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |*_| }
      david,jason,kevin=%w[david jason kevin].map { |key|User.find(ActiveRecord::FixtureSet.identify(key)) }
      room=Room.find(ActiveRecord::FixtureSet.identify(:david_and_jason))
      grant=HuddleGrant.issue!(session:Session.find(ActiveRecord::FixtureSet.identify(:david_safari)),membership:room.memberships.find_by!(user:david))
      jason.update!(presence_setting:"dnd") if %w[dnd allowed_dnd].include?(name)
      if %w[meeting allowed_meeting].include?(name)
        jason.update!(meeting_status_enabled:true,meeting_dnd_enabled:true)
        Calendar::MeetingCache.create!(user:jason,fetched_at:Time.current,busy_intervals:[[5.minutes.ago.iso8601,55.minutes.from_now.iso8601]])
      end
      jason.update!(ooo_until:1.day.from_now) if %w[ooo allowed_ooo].include?(name)
      DndAllowedUser.create!(user:jason,allowed_user:david) if name.start_with?("allowed_")
      Huddle::RingPolicy.quiet_check=->(user) { user==jason } if name=="quiet_override"
      attempts=if name.start_with?("allowed_") then [[jason,david],[jason,kevin]] elsif name=="quiet_override" then [[jason,nil],[david,nil]] elsif name=="ooo" then [[jason,david],[jason,david]] else [[jason,david]] end
      outcomes=attempts.each_with_index.map do |(recipient,caller),n|
        jason.update!(ooo_notify_enabled:true) if name=="ooo" && n==1
        sound=Huddle::RingPolicy.ring?(recipient.reload,caller:caller)
        # Capture the actual invitation producer with the same caller. The
        # override ignores callers, including the original no-caller case.
        grant.user=caller if caller
        frames=[]
        ActionCable.server.define_singleton_method(:broadcast) { |stream,payload| frames << {stream:stream,payload:payload} }
        grant.send(:broadcast_suppressed_invitation!,recipient)
        raise "policy/producer mismatch" unless frames.size==1 && frames[0][:payload][:huddleInvitation][:silent]==!sound
        fields=%w[id presence_setting dnd_enabled dnd_until quiet_hours_enabled quiet_hours_start_minute quiet_hours_end_minute time_zone meeting_status_enabled meeting_dnd_enabled ooo_until ooo_notify_enabled]
        {setup_sql:setup_sql(recipient),context:{recipient:recipient.attributes.slice(*fields),caller_id:caller&.id,kind:"huddle",now:Time.current.to_i,quiet_override:name=="quiet_override",allowed_user_ids:DndAllowedUser.where(user:recipient).order(:id).pluck(:allowed_user_id),meeting_cache:Calendar::MeetingCache.find_by(user:recipient)&.attributes},sound_allowed:sound,sender_id:grant.user.id,broadcast:frames[0]}
      end
      {name:name,title:titles[index],outcomes:outcomes}
    end
    puts JSON.pretty_generate(reference_pin:"d7c7de92",cases:cases)
  ensure
    Huddle::RingPolicy.quiet_check=nil
    travel_back
  end
end
HuddleRingPolicySeam.new.run
