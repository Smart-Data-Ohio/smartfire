require 'json'
require 'active_support/testing/time_helpers'
# Public controller oracle: production Rails, real verified cookies, fixed input rows.
class HuddlePublicOracle
  include ActiveSupport::Testing::TimeHelpers
  ENVIRONMENT={'LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret'}.freeze
  def run
    ActiveRecord::Schema.verbose=false
    load Rails.root.join('db/schema.rb')
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join('test/fixtures'),%w[accounts users rooms memberships])
    travel_to Time.utc(2026,1,1,12)
    @users=%w[david jason kevin bender].map {|k|User.find(ActiveRecord::FixtureSet.identify(k))}
    @users.each {|u|u.update_columns(updated_at:Time.current)}
    @original=@users.map(&:attributes)
    ApplicationController.allow_forgery_protection=false
    ActionCable.server.define_singleton_method(:broadcast) {|*_|}
    Huddle::CleanupJob.define_singleton_method(:perform_later) {|*_|}
    # Only identifiers created after login are fixed. Tokens are checked structurally,
    # with their HS256 signature, scopes and exact TTL, by the Rust HTTP test.
    SecureRandom.singleton_class.alias_method(:ws13_original_hex,:hex)
    SecureRandom.define_singleton_method(:hex) {|n=nil|n==32 ? '4'*64 : ws13_original_hex(n)}
    SecureRandom.define_singleton_method(:uuid) {'00000000-0000-4000-8000-000000000013'}
    cases=[]
    %w[show join participants leave presence].each do |action|
      {anonymous:{actor:nil},bot_key:{actor:nil,bot_key:true},session_bot:{actor:3},inactive:{inactive:true},missing_configuration:{unconfigured:true}}.each {|name,opts|cases << scenario("#{action}_#{name}",opts.merge(action:action))}
    end
    %w[show join participants leave].each do |action|
      {outsider:{outsider:true},deleted:{deleted:true},removed:{removed:true}}.each {|name,opts|cases << scenario("#{action}_#{name}",opts.merge(action:action))}
    end
    {
      show_voice:{},show_direct:{type:'Rooms::Direct'},show_group:{type:'Rooms::Direct',group:true},
      join_voice:{action:'join'},join_closed:{action:'join',type:'Rooms::Closed'},join_open:{action:'join',type:'Rooms::Open'},
      join_direct:{action:'join',type:'Rooms::Direct'},join_group:{action:'join',type:'Rooms::Direct',group:true},
      join_listener:{action:'join',type:'Rooms::Stage',role:'listener'},join_host:{action:'join',type:'Rooms::Stage',role:'host'},
      join_speaker:{action:'join',type:'Rooms::Stage',role:'speaker'},join_missing_role:{action:'join',type:'Rooms::Stage',role:nil},
      join_muted:{action:'join',muted:true},join_reused:{action:'join',grants:'live'},join_endpoint_alias:{action:'join',aliased:true},
      participants_live:{action:'participants',grants:'mixed'},participants_direct:{action:'participants',type:'Rooms::Direct',grants:'live'},
      participants_group:{action:'participants',type:'Rooms::Direct',group:true,grants:'mixed'},participants_empty:{action:'participants'},
      leave_live:{action:'leave',grants:'mixed'},leave_voice:{action:'leave',grants:'live'},leave_group:{action:'leave',type:'Rooms::Direct',group:true,grants:'live'},
      leave_empty:{action:'leave'},leave_twice:{action:'leave',grants:'live',repeat:true},
      presence_live:{action:'presence',grants:'mixed',extra_room:true},presence_empty:{action:'presence'},
      presence_deleted:{action:'presence',grants:'live',deleted:true},presence_removed:{action:'presence',grants:'live',removed:true}
    }.each {|name,opts|cases << scenario(name.to_s,opts)}
    puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:Time.current.to_i,cases:cases})
  ensure
    travel_back
  end
  def scenario(name,opts)
    ActiveSupport::ExecutionContext.clear
    ENV.update(ENVIRONMENT)
    @original.each {|attrs|User.find(attrs['id']).update_columns(attrs.slice('role','status','updated_at'))}
    @users.each(&:reload)
    type=opts.fetch(:type,'Rooms::Voice')
    room=Room.create!(id:9001,type:type,name:type=='Rooms::Direct' ? nil : 'WS13 room',creator:@users[0])
    room.memberships.delete_all
    room_users=opts[:group] ? @users.take(3) : @users.take(2)
    room_users.each_with_index {|u,i|room.memberships.create!(id:9011+i,user:u,stage_role:type=='Rooms::Stage' ? (i==0 ? opts.fetch(:role,'host') : 'listener') : nil,server_muted_at:opts[:muted] && i==0 ? Time.current : nil)}
    actor_index=opts.fetch(:actor,0);actor_index=2 if opts[:outsider];actor=actor_index && @users[actor_index]
    Rails.cache.clear
    request=ActionDispatch::Integration::Session.new(Rails.application);request.host! 'campfire.test'
    if actor
      actor.update!(email_address:'bender@example.test',password:'secret123456') if actor.bot?
      request.post('/session',params:{email_address:actor.email_address,password:'secret123456'},headers:{'HTTP_X_FORWARDED_PROTO'=>'https'})
      raise "login failed #{name}: #{request.response.status}" unless request.cookies['session_token']
      @actor_session=Session.where(user_id:actor.id).order(id: :desc).first
      @actor_session.update_columns(two_factor_verified_at:Time.current)
    else
      @actor_session=nil
    end
    if opts[:grants]
      rows=[[@users[0],@actor_session,0,nil],[@users[0],nil,1,nil],[@users[1],nil,2,nil]]
      rows += [[@users[1],nil,3,20],[@users[1],nil,4,21],[@users[1],nil,5,:never],[@users[1],nil,6,:revoked]] if opts[:grants]=='mixed'
      rows.each do |u,session,i,age|
        session ||= Session.create!(id:-7001-i,user:u,token:"ws13-public-session-#{i}")
        member=room.memberships.find_by!(user:u)
        HuddleGrant.create!(id:17+i,identity:"ws13-public-identity-#{i}",room_name:Huddle.room_name(room.id),room:room,membership:member,user:u,session:session,stage_role:member.stage_role,server_muted:member.server_muted?,last_seen_at:age==:never ? nil : Time.current-(age.is_a?(Integer) ? age : 0),revoked_at:age==:revoked ? Time.current : nil,last_issued_at:Time.current-60)
      end
    end
    if opts[:extra_room]
      extra=Room.create!(id:9002,type:'Rooms::Closed',name:'Other',creator:@users[0]);extra.memberships.delete_all
      [@users[0],@users[1]].each_with_index {|u,i|extra.memberships.create!(id:9021+i,user:u)}
      session=Session.create!(id:-7100,user:@users[1],token:'ws13-public-extra')
      HuddleGrant.create!(id:30,identity:'ws13-public-extra',room_name:Huddle.room_name(extra.id),room:extra,membership:extra.memberships.find_by!(user:@users[1]),user:@users[1],session:session,last_seen_at:Time.current)
      # Outsider room: current user is not a member.
      secret=Room.create!(id:9003,type:'Rooms::Voice',name:'Secret',creator:@users[1]);secret.memberships.delete_all
      member=secret.memberships.create!(id:9031,user:@users[1]);session=Session.create!(id:-7200,user:@users[1],token:'ws13-public-secret')
      HuddleGrant.create!(id:31,identity:'ws13-public-secret',room_name:Huddle.room_name(secret.id),room:secret,membership:member,user:@users[1],session:session,last_seen_at:Time.current)
    end
    room.update_columns(deleted_at:Time.current) if opts[:deleted]
    room.memberships.find_by!(user:@users[0]).delete if opts[:removed]
    actor.update_columns(status: :banned) if opts[:inactive]
    ENV.delete('LIVEKIT_API_SECRET') if opts[:unconfigured]
    ENV['LIVEKIT_URL']='wss://internal.example.test:7880/client' if opts[:aliased]
    input={rooms:Room.where(id:9001..9003).map(&:attributes),memberships:Membership.where(room_id:9001..9003).map(&:attributes),users:@users.map {|u|u.reload.attributes.slice('id','name','role','status','updated_at')},grants:HuddleGrant.order(:id).map(&:attributes)}
    action=opts.fetch(:action,'show');path=action=='presence' ? '/users/huddle_presence' : "/rooms/#{room.id}/huddle#{%w[leave participants].include?(action) ? "/#{action}" : ''}"
    params=opts[:bot_key] ? {bot_key:"#{@users[3].id}-BenderToken1"} : {room:'client-room',identity:'client-identity'}
    verb=%w[join leave].include?(action) ? 'post' : 'get'
    request.public_send(verb,path,params:params,headers:{'HTTP_X_FORWARDED_PROTO'=>'https'})
    request.public_send(verb,path,params:params,headers:{'HTTP_X_FORWARDED_PROTO'=>'https'}) if opts[:repeat]
    body=request.response.body.empty? ? nil : JSON.parse(request.response.body)
    claims=body.is_a?(Hash) && body['token'] ? JWT.decode(body['token'],ENVIRONMENT['LIVEKIT_API_SECRET'],true,algorithm:'HS256').first : nil
    {name:name,input:input,actor_id:actor&.id,actor_session_id:@actor_session&.id,action:action,path:path,method:verb,params:params,unconfigured:!!opts[:unconfigured],aliased:!!opts[:aliased],repeat:!!opts[:repeat],status:request.response.status,content_type:request.response.headers['Content-Type'],cache_control:request.response.headers['Cache-Control'],body:body,claims:claims,grants:HuddleGrant.order(:id).map {|g|g.attributes.slice('id','session_id','user_id','membership_id','room_id','stage_role','server_muted','last_seen_at','revoked_at')}}
  ensure
    ActivityItem.delete_all;HuddleInvitation.delete_all if defined?(HuddleInvitation)
    HuddleGrant.delete_all;HuddleCleanup.delete_all;Session.delete_all
    Membership.where(room_id:9001..9003).delete_all;Room.where(id:9001..9003).delete_all
  end
end
HuddlePublicOracle.new.run
