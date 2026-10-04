require "json"
require "active_support/testing/time_helpers"

class HuddleNeighborMention
  include ActiveSupport::Testing::TimeHelpers
  include Rails.application.routes.url_helpers
  def run
    travel_to Time.utc(2026,3,2,16)
    ActiveRecord::Schema.verbose=false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"),%w[accounts users rooms memberships sessions])
    ENV["LIVEKIT_API_SECRET"]="ws13b-fixture-api-secret"
    ENV.delete("LIVEKIT_URL")
    Huddle::RingPolicy.quiet_check=->(_) { false }
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |*_| }
    david,jason=%w[david jason].map { |k|User.find(ActiveRecord::FixtureSet.identify(k)) }
    jason.update!(inbox_preferences:{"huddle_invitations"=>false})
    room=Room.find(ActiveRecord::FixtureSet.identify(:david_and_jason))
    grant=HuddleGrant.issue!(session:Session.find(ActiveRecord::FixtureSet.identify(:david_safari)),membership:room.memberships.find_by!(user:david))
    before=ActivityItem.where(user:jason).count
    travel_to Time.current+46
    Huddle::InvitationResolver.resolve_overdue!
    after=ActivityItem.where(user:jason).count
    attachment=ApplicationController.render(partial:"users/mention",locals:{user:jason})
    body="Hey <action-text-attachment sgid=\"#{jason.attachable_sgid}\" content-type=\"application/vnd.campfire.mention\" content=\"#{attachment.gsub('"','&quot;')}\"></action-text-attachment>"
    message=Room.find(ActiveRecord::FixtureSet.identify(:designers)).messages.create!(creator:david,body:body,client_message_id:"huddle-switch-neighbour")
    item=ActivityItem.find_by!(user:jason,source:message)
    result={reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:Time.utc(2026,3,2,16).to_i,before:before,after_timeout:after,body:body,message:{room_id:message.room_id,creator_id:message.creator_id,client_message_id:message.client_message_id},mention:{user_id:item.user_id,source_type:item.source_type,event_type:item.event_type},plain_text:message.body.to_plain_text}
    room=message.room
    member=room.memberships.find_by!(user:jason)
    result[:guards]=%w[everything nothing muted invisible self bot inactive system_note streaming thread_mentions thread_nothing].map do |name|
      member.update_columns(involvement:%w[nothing muted invisible].include?(name) ? name : "everything")
      jason.update_columns(role:name=="bot" ? "bot" : "member",status:name=="inactive" ? "deactivated" : "active")
      thread=nil
      if name.start_with?("thread_")
        thread=ChannelThread.create!(room:room,creator:david,name:"Mention guard")
        thread.memberships.create!(user:jason,involvement:name=="thread_nothing" ? "nothing" : "mentions")
      end
      guard=room.messages.create!(creator:name=="self" ? jason : david,body:body,client_message_id:"ws13b-mention-#{name}",system_note:name=="system_note",streaming:name=="streaming",thread:thread)
      {name:name,recorded:ActivityItem.exists?(user:jason,source:guard)}
    end
    jason.update_columns(role:"member",status:"active")
    member.update_columns(involvement:"everything")
    item.mark_handled!
    initial=item.reload.attributes
    ActivityItems::Recorder.record_message!(message)
    raise "recorder refreshed existing mention" unless item.reload.attributes==initial
    result[:idempotent]={read_at:item.read_at.to_i,handled_at:item.handled_at.to_i,event_type:item.event_type}
    puts JSON.pretty_generate(result)
  ensure
    Huddle::RingPolicy.quiet_check=nil
    travel_back
  end
end
HuddleNeighborMention.new.run
