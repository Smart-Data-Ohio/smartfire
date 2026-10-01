require "json"
require "active_support/testing/time_helpers"

class HuddleHandsOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose=false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"),%w[accounts users rooms memberships])
    @david,@jason=%w[david jason].map {|key|User.find(ActiveRecord::FixtureSet.identify(key))}
    ActionCable.server.define_singleton_method(:broadcast) {|*_|}
    variants={
      "listener_raise"=>{},"listener_double_raise"=>{hand:true},
      "listener_lower"=>{operation:"lower",hand:true},"listener_lower_empty"=>{operation:"lower"},
      "speaker_raise"=>{role:"speaker"},"host_raise"=>{role:"host"},
      "voice_raise"=>{type:"Rooms::Voice"},"channel_raise"=>{type:"Rooms::Closed"},
      "voice_lower_empty"=>{type:"Rooms::Voice",operation:"lower"},
      "channel_lower_empty"=>{type:"Rooms::Closed",operation:"lower"},
      "speaker_lower_corrupt_hand"=>{role:"speaker",hand:true,operation:"lower"},
      "host_double_raise_corrupt_hand"=>{role:"host",hand:true},
      "promote_speaker_clears_hand"=>{hand:true,operation:"role",next_role:"speaker"},
      "promote_host_clears_hand"=>{hand:true,operation:"role",next_role:"host"},
      "same_role_clears_hand"=>{hand:true,operation:"role",next_role:"listener"},
      "voice_invalid_stage_role_raise"=>{type:"Rooms::Voice",role:"listener"},
      "voice_invalid_stage_role_lower"=>{type:"Rooms::Voice",role:"listener",operation:"lower"}
    }
    cases=variants.map {|name,options|scenario(name,options)}
    puts JSON.pretty_generate({reference_pin:"d7c7de92",now:Time.current.to_i,cases:cases})
  ensure
    travel_back
  end
  def scenario(name,options)
    travel_to Time.utc(2026,1,1,12)
    room=Room.create!(id:9001,type:options.fetch(:type,"Rooms::Stage"),name:"WS13 hand validation",creator:@david)
    member=room.memberships.create!(id:9011,user:@jason,created_at:Time.current-120,updated_at:Time.current-120)
    room.memberships.create!(id:9012,user:@david,stage_role:room.stage? ? "host" : nil)
    member.update_columns(stage_role:options[:role]) if options[:role]
    member.update_columns(hand_raised_at:Time.current-10) if options[:hand]
    input={room:room.attributes,memberships:room.memberships.reload.map(&:attributes),users:[@david,@jason].map {|u|u.attributes.slice("id","name","role","status","inbox_preferences")},grants:[],items:[]}
    operation=options.fetch(:operation,"raise")
    changed,error=nil,nil
    begin
      changed=case operation
      when "raise" then member.raise_hand!
      when "lower" then member.lower_hand!
      when "role" then member.change_stage_role!(options[:next_role])
      end
    rescue ActiveRecord::RecordInvalid=>exception
      error=exception.record.errors.messages
    end
    {name:name,input:input,membership_id:member.id,operation:operation,next_role:options[:next_role],changed:changed,errors:error,membership:member.reload.attributes}
  ensure
    Membership.where(room_id:9001).delete_all
    Room.where(id:9001).delete_all
  end
end
HuddleHandsOracle.new.run
