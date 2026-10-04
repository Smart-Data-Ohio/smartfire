require 'json'
require 'active_support/testing/time_helpers'
# Real production controller requests with verified sessions. As in the Rails
# integration suite, forgery protection is off for the oracle; Rust HTTP tests send CSRF.
class HuddleModerationOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose=false
    load Rails.root.join('db/schema.rb')
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join('test/fixtures'),%w[accounts users rooms memberships])
    travel_to Time.utc(2026,1,1,12)
    @users=%w[david jason kevin].map {|k|User.find(ActiveRecord::FixtureSet.identify(k))}
    @users[1].update_columns(role: :member)
    ApplicationController.allow_forgery_protection=false
    @frames=[];ActionCable.server.define_singleton_method(:broadcast) {|stream,html|@ws13_frames << {stream:stream,html:html}};ActionCable.server.instance_variable_set(:@ws13_frames,@frames)
    ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
    Huddle::CleanupJob.define_singleton_method(:perform_later) {|*_|}
    cases=[]
    {host_mute:{}, host_unmute:{muted:true,action:'unmute'},host_disconnect:{action:'disconnect'},repeat_mute:{muted:true},noop_unmute:{action:'unmute'},admin_mute:{actor:0},admin_self_unmute:{actor:0,target:0,muted:true,action:'unmute'},host_self_mute:{target:1},host_self_unmute:{target:1,muted:true,action:'unmute'},host_self_disconnect:{target:1,action:'disconnect'},host_admin_mute:{target:0},host_admin_unmute:{target:0,action:'unmute',muted:true},host_admin_disconnect:{target:0,action:'disconnect'},speaker_mute:{actor:2,target:1},speaker_unmute:{actor:2,target:1,action:'unmute'},speaker_disconnect:{actor:2,target:1,action:'disconnect'},unknown_target:{unknown:true},outsider:{outsider:true},plain_room:{type:'Rooms::Open',actor:0},voice_admin_mute:{type:'Rooms::Voice',actor:0},voice_admin_unmute:{type:'Rooms::Voice',actor:0,muted:true,action:'unmute'},voice_member_mute:{type:'Rooms::Voice'},mute_stream_without_grant:{stream:true,no_grant:true},repeat_mute_ends_stream:{stream:true,muted:true},unmute_keeps_stream:{stream:true,muted:true,action:'unmute'},disconnect_stream_without_grant:{stream:true,no_grant:true,action:'disconnect'},html_redirect:{format:'html'},turbo_stage:{format:'turbo_stream'},turbo_voice:{type:'Rooms::Voice',actor:0,format:'turbo_stream'}}.each {|name,opts|cases << scenario(name.to_s,opts)}
    puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:Time.current.to_i,cases:cases})
  ensure
    travel_back
  end
  def scenario(name,opts)
    ActiveSupport::ExecutionContext.clear
    type=opts[:type] || 'Rooms::Stage'
    room=Room.create!(id:9001,type:type,name:'WS13 Stage',creator:@users[0])
    room.memberships.delete_all
    members=@users.each_with_index.map {|u,i|room.memberships.create!(id:9011+i,user:u,stage_role:type=='Rooms::Stage' ? %w[listener host speaker][i] : nil)}
    target=members[opts[:target] || 2];actor=@users[opts[:actor] || 1]
    members[opts[:actor] || 1].delete if opts[:outsider]
    target.update_columns(server_muted_at:Time.current-60) if opts[:muted]
    Stream.create!(id:40,room:room,membership:target,user:target.user,quality:'1080p15',created_at:Time.current-60,started_at:Time.current-60) if opts[:stream]
    if !opts[:no_grant]
      session=Session.create!(id:7001,user:target.user,token:'ws13-moderation-session-7001')
      HuddleGrant.create!(id:17,identity:'ws13-moderation-identity',room_name:'ws13-moderation-room',room:room,membership:target,user:target.user,session:session,stage_role:target.stage_role,server_muted:target.server_muted?,last_seen_at:Time.current,created_at:Time.current-60)
    end
    input={room:room.attributes,users:@users.map {|u|u.attributes.slice('id','name','role','status','inbox_preferences')},memberships:room.memberships.reload.map(&:attributes),grants:HuddleGrant.all.map(&:attributes),items:[],streams:Stream.all.map(&:attributes)}
    request=ActionDispatch::Integration::Session.new(Rails.application)
    request.host! 'campfire.test'
    Rails.cache.clear
    request.post('/session',params:{email_address:actor.email_address,password:'secret123456'},headers:{'HTTP_X_FORWARDED_PROTO'=>'https'})
    raise "sign-in #{request.response.status}" unless request.cookies['session_token']
    Session.where(user_id:actor.id).order(id: :desc).first.update_columns(two_factor_verified_at:Time.current)
    @frames.clear
    action=opts[:action] || 'mute';verb=action=='unmute' ? 'delete' : 'post';path_action=action=='unmute' ? 'mute' : action
    format=opts[:format] || 'json';headers={'HTTP_X_FORWARDED_PROTO'=>'https','Accept'=>{'html'=>'text/html','json'=>'application/json','turbo_stream'=>'text/vnd.turbo-stream.html'}[format]}
    request.public_send(verb,"/rooms/#{room.id}/call_moderation/#{opts[:unknown] ? 0 : target.id}/#{path_action}",headers:headers)
    {name:name,input:input,actor_id:actor.id,target_id:opts[:unknown] ? 0 : target.id,action:action,format:format,status:request.response.status,content_type:request.response.headers['Content-Type'],body:format=='html' && request.response.status==302 || format=='turbo_stream' && request.response.status==200 ? nil : request.response.body,location:request.response.headers['Location'],members:room.memberships.reload.order(:id).map {|m|m.attributes.slice('id','server_muted_at','updated_at')},grants:HuddleGrant.order(:id).map {|g|g.attributes.slice('id','revoked_at')},streams:Stream.order(:id).map {|s|s.attributes.slice('id','ended_at')},cleanups:HuddleCleanup.order(:id).map {|c|c.attributes.slice('operation','huddle_grant_id','room_name','identity')},rosters:@frames.count {|f|f[:html].is_a?(String) && f[:html].include?('target="stage_roster_')},role_events:@frames.select {|f|f[:html].is_a?(String) && f[:html].include?('data-huddle-rejoin-room-id')}.map {|f|f[:html]}}
  ensure
    Stream.delete_all;HuddleGrant.delete_all;HuddleCleanup.delete_all;Session.delete_all
    Membership.where(room_id:9001).delete_all;Room.where(id:9001).delete_all
  end
end
HuddleModerationOracle.new.run
