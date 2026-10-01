require 'json'
require 'active_support/testing/time_helpers'
# Real production controller requests with verified sessions. As in the Rails
# integration suite, forgery protection is off for the oracle; Rust HTTP tests send CSRF.
class HuddleParticipationOracle
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
    {
      promote_listener:{actor:0,target:0,role:'speaker'}, demote_speaker:{role:'listener'},
      demote_without_grant:{role:'listener',stream:true,no_grant:true},
      host_to_speaker:{target:1,role:'speaker',second_host:true,stream:true},
      speaker_to_host:{role:'host',stream:true}, same_speaker:{role:'speaker',stream:true},
      same_listener_ends_stream:{actor:0,target:0,role:'listener',stream:true},
      promotion_clears_hand:{actor:0,target:0,role:'speaker',hand:true},
      sole_host:{target:1,role:'listener'}, last_host_speaker:{target:1,role:'speaker'},
      listener_forbidden:{actor:0,target:2,role:'speaker',nonadmin:true},
      speaker_forbidden:{actor:2,role:'speaker'}, administrator_target:{target:0,role:'host'},
      administrator_actor:{actor:0,role:'listener'}, administrator_own:{actor:0,target:0,role:'speaker'},
      unknown_role:{role:'unknown'}, unknown_target:{unknown:true}, outsider:{outsider:true},
      voice_room:{type:'Rooms::Voice'}, turbo_role:{role:'listener',format:'turbo_stream'},
      raise_listener:{action:'raise',actor:0}, double_raise:{action:'raise',actor:0,hand:true},
      raise_speaker:{action:'raise',actor:2}, raise_host:{action:'raise'},
      lower_own:{action:'lower',actor:0,hand:true}, lower_empty:{action:'lower',actor:0},
      lower_host:{action:'lower',target:0,hand:true,explicit:true},
      lower_administrator:{action:'lower',actor:0,target:2,hand:true,explicit:true},
      lower_speaker_denied:{action:'lower',actor:2,target:0,hand:true,explicit:true},
      lower_listener_explicit_self:{action:'lower',actor:2,listener:true,hand:true,explicit:true},
      lower_unknown:{action:'lower',explicit:true,unknown:true},
      lower_voice:{action:'lower',type:'Rooms::Voice'},
      turbo_raise:{action:'raise',actor:0,format:'turbo_stream'},
      turbo_lower:{action:'lower',actor:0,hand:true,format:'turbo_stream'}
    }.each {|name,opts|cases << scenario(name.to_s,opts)}
    rate=scenario('raise_rate_limit',{action:'raise',actor:0,requests:11})
    puts JSON.pretty_generate({reference_pin:'d7c7de92',now:Time.current.to_i,cases:cases,rate:rate})
  ensure
    travel_back
  end
  def scenario(name,opts)
    ActiveSupport::ExecutionContext.clear
    @users[0].update_columns(role: :administrator)
    @users.each(&:reload)
    type=opts[:type] || 'Rooms::Stage'
    room=Room.create!(id:9001,type:type,name:'WS13 Stage',creator:@users[0])
    room.memberships.delete_all
    members=@users.each_with_index.map {|u,i|room.memberships.create!(id:9011+i,user:u,stage_role:type=='Rooms::Stage' ? %w[listener host speaker][i] : nil)}
    target=members[opts[:target] || 2];actor=@users[opts[:actor] || 1]
    members[opts[:actor] || 1].delete if opts[:outsider]
    members[0].user.update_columns(role: :member) if opts[:nonadmin]
    members[0].update_columns(stage_role:'host') if opts[:second_host]
    members[2].update_columns(stage_role:'listener') if opts[:listener]
    # A hand belongs to the actor for own-hand operations, and to the target otherwise.
    hand_member=opts[:action] && !opts[:explicit] ? members[opts[:actor] || 1] : target
    hand_member.update_columns(hand_raised_at:Time.current-60) if opts[:hand]
    members.reject(&:destroyed?).each { |m| m.reload if Membership.exists?(m.id) }
    @users.each(&:reload)
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
    action=opts[:action] || 'role'
    verb={'role'=>'patch','raise'=>'post','lower'=>'delete'}[action]
    target_id=opts[:unknown] ? 0 : target.id
    path="/rooms/#{room.id}/stage/#{action=='role' ? "roles/#{target_id}" : 'hand'}"
    params=action=='role' ? {membership_id:target_id,stage_role:opts[:role] || 'speaker'} : opts[:explicit] ? {membership_id:target_id} : {}
    format=opts[:format] || 'html';headers={'HTTP_X_FORWARDED_PROTO'=>'https','Accept'=>{'html'=>'text/html','json'=>'application/json','turbo_stream'=>'text/vnd.turbo-stream.html'}[format]}
    request.public_send(verb,path,params:params,headers:headers)
    statuses=[request.response.status]
    (2..opts.fetch(:requests,1)).each do
      request.public_send(verb,path,params:params,headers:headers)
      statuses << request.response.status
    end
    {statuses:statuses,name:name,input:input,actor_id:actor.id,target_id:target_id,action:action,params:params,format:format,status:request.response.status,content_type:request.response.headers['Content-Type'],body:format=='html' && request.response.status==302 || format=='turbo_stream' && request.response.status==200 ? nil : request.response.body,location:request.response.headers['Location'],members:room.memberships.reload.order(:id).map {|m|m.attributes.slice('id','stage_role','hand_raised_at','updated_at')},grants:HuddleGrant.order(:id).map {|g|g.attributes.slice('id','stage_role','revoked_at')},streams:Stream.order(:id).map {|s|s.attributes.slice('id','ended_at')},cleanups:HuddleCleanup.order(:id).map {|c|c.attributes.slice('operation','huddle_grant_id','room_name','identity')},rosters:@frames.count {|f|f[:html].is_a?(String) && f[:html].include?('target="stage_roster_')},panels:@frames.count {|f|f[:html].is_a?(String) && f[:html].include?('target="stage_panel_')},role_events:@frames.select {|f|f[:html].is_a?(String) && f[:html].include?('data-huddle-rejoin-room-id')}.map {|f|f[:html]}}

  ensure
    Stream.delete_all;HuddleGrant.delete_all;HuddleCleanup.delete_all;Session.delete_all
    Membership.where(room_id:9001).delete_all;Room.where(id:9001).delete_all
  end
end
HuddleParticipationOracle.new.run
