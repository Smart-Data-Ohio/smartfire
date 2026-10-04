require 'json'
require 'active_support/testing/time_helpers'
class HuddleStreamLifecycleOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose=false
    load Rails.root.join('db/schema.rb')
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join('test/fixtures'),%w[accounts users rooms memberships])
    travel_to Time.utc(2026,1,1,12)
    @users=%w[david jason kevin bender].map {|k|User.find(ActiveRecord::FixtureSet.identify(k))}
    @frames=[]
    ActionCable.server.define_singleton_method(:broadcast) {|stream,html|@ws13_frames << {stream:stream,html:html}}
    ActionCable.server.instance_variable_set(:@ws13_frames,@frames)
    cases=[]
    %w[720p15 1080p15 1080p30 bad_quality explicit_start unique_live free_after_end repeat_end host_stop self_stop automatic_end member_without_grant user_without_grant speaker_user_without_grant last_host admin_successor earliest_successor other_host empty_stage inactive_admin].each {|name|cases << scenario(name)}
    puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:Time.current.to_i,cases:cases})
  ensure
    travel_back
  end
  def scenario(name)
    room=Rooms::Stage.create!(id:9001,name:'WS13 Stage',creator:@users[0])
    members=@users.each_with_index.map {|u,i|room.memberships.create!(id:9011+i,user:u,stage_role:i==0 ? 'host' : 'listener',created_at:Time.current-(i==2 ? 200 : 100-i))}
    members[1].update_columns(stage_role:'host') if name=='other_host'
    @users[1].update_columns(role: :administrator) if name=='admin_successor'
    @users[1].update_columns(role: :administrator,status: :banned) if name=='inactive_admin'
    Membership.where(room_id:9001).where.not(id:9011).delete_all if name=='empty_stage'
    members[1].update_columns(stage_role: 'speaker') if name=='speaker_user_without_grant'
    starter=name=='speaker_user_without_grant' ? members[1] : members[0]
    quality=%w[720p15 1080p15 1080p30].include?(name) ? name : name=='bad_quality' ? '4k60' : '1080p15'
    operations=[]
    operations << {op:'create',membership_id:starter.id,user_id:starter.user_id,quality:quality,started_at:name=='explicit_start' ? (Time.current-50).iso8601 : nil}
    operations << {op:'create',membership_id:starter.id,user_id:starter.user_id,quality:'720p15'} if name=='unique_live'
    if %w[free_after_end repeat_end host_stop self_stop automatic_end].include?(name)
      actor=name=='host_stop' ? @users[1] : name=='self_stop' ? @users[0] : nil
      operations << {op:'end',actor_id:actor&.id}
      operations << {op:'end',actor_id:nil} if name=='repeat_end'
      operations << {op:'create',membership_id:starter.id,user_id:starter.user_id,quality:'720p15'} if name=='free_after_end'
    end
    if %w[member_without_grant last_host admin_successor earliest_successor other_host empty_stage inactive_admin].include?(name)
      members[1].update_columns(stage_role:'host') if name=='member_without_grant'
      operations << {op:'destroy_member',id:starter.id}
    end
    operations << {op:'deactivate',id:starter.user_id} if %w[user_without_grant speaker_user_without_grant].include?(name)
    input={room:room.attributes,users:@users.map {|u|u.reload.attributes.slice('id','name','role','status','inbox_preferences')},memberships:room.memberships.reload.map(&:attributes),grants:[],items:[],streams:[]}
    @frames.clear
    results=[]
    stream=nil
    operations.each do |operation|
      error=nil
      begin
        case operation[:op]
        when 'create'; stream=Stream.create!(id:40+Stream.count,room:room,membership:starter,user:starter.user,quality:operation[:quality],started_at:operation[:started_at]); operation[:id]=stream.id
        when 'end'; stream.end!(ended_by:operation[:actor_id] && User.find(operation[:actor_id]))
        when 'destroy_member'; starter.destroy!
        when 'deactivate'; starter.user.deactivate
        end
      rescue ActiveRecord::RecordInvalid=>e;error=e.record.errors.messages
      rescue ActiveRecord::RecordNotUnique;error='not_unique'
      end
      results << {operation:operation,error:error,streams:Stream.order(:id).map(&:attributes),members:room.memberships.reload.order(:id).map {|m|m.attributes.slice('id','stage_role','hand_raised_at')},notes:room.messages.where(system_note:true).map {|m|{creator_id:m.creator_id,body:m.body.to_plain_text}},frames:@frames.select {|f|f[:html].is_a?(String)}.map {|f|{stream:f[:stream],action:f[:html][/action="([^"]+)"/,1],target:f[:html][/target="([^"]+)"/,1],stopped:f[:html].include?('stream-stopped')}}}
      @frames.clear
    end
    {name:name,input:input,results:results}
  ensure
    Stream.delete_all;HuddleGrant.delete_all;HuddleCleanup.delete_all
    Message.where(room_id:9001).delete_all;ActionText::RichText.where(record_type:'Message').delete_all
    Membership.where(room_id:9001).delete_all;Room.where(id:9001).delete_all
    @users[0].update_columns(status: :active)
    @users[1].update_columns(role: :member,status: :active)
  end
end
HuddleStreamLifecycleOracle.new.run
