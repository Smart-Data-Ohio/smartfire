# Real pinned Rails recipient endpoints. Only Google-independent setup and request entropy are fixed.
require 'json'
require 'action_dispatch/testing/integration'
Rails.logger=ActiveSupport::Logger.new($stderr)
ActionController::Base.logger=Rails.logger
ApplicationController.logger=Rails.logger
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=true
Rails.application.env_config['action_dispatch.show_exceptions']=:all
Net::HTTP.define_singleton_method(:start) { |*| raise 'recipient fixture attempted live HTTP' }
NOW=Time.utc(2026,3,2,16)
Time.define_singleton_method(:current) { NOW }
ApplicationController.prepend(Module.new do
  def form_authenticity_token(form_options:{})
    'RECIPIENT-CSRF'
  end
  def valid_authenticity_token?(session,token)
    token=='RECIPIENT-CSRF'
  end
end)
def sign_in(client,user)
  session=Session.create!(user:,user_agent:'Recipient fixture',ip_address:'127.0.0.1',two_factor_verified_at:NOW)
  req=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
  jar=ActionDispatch::Cookies::CookieJar.build(req,{})
  jar.signed[:session_token]={value:session.token};client.cookies['session_token']=jar[:session_token]
end
def observe(client)
  body=client.response.body
  {status:client.response.status,body:body.empty? ? '' : (JSON.parse(body) rescue body),cache_control:client.response.headers['Cache-Control']}
end
specs=%w[index no_consent bot_members ineligible invalid_email nil_email domains missing_key missing_project anonymous nonmember bot_key bot_session agent_session].map { |name|{name:,method:'get'} }
specs+=%w[canonical duplicate empty stale invalid_ids agent_selection scalar missing emails oversized not_configured anonymous_validate nonmember_validate bot_key_validate no_csrf].map { |name|{name:,method:'post'} }
specs+=%w[throttle_index throttle_shared].map { |name|{name:,method:'get'} }
rows=[]
specs.each do |spec|
  ActiveRecord::Base.transaction(requires_new:true) do
    ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_PICKER_API_KEY']='test-picker-key';ENV['GOOGLE_CLOUD_PROJECT_NUMBER']='123456789012'
    ENV.delete('GOOGLE_PICKER_API_KEY') if %w[missing_key not_configured].include?(spec[:name])
    ENV.delete('GOOGLE_CLOUD_PROJECT_NUMBER') if spec[:name]=='missing_project'
    Rails.cache=ActiveSupport::Cache::MemoryStore.new
    GoogleAccount.delete_all
    jz=User.find_by!(name:'JZ');david=User.find_by!(name:'David');jason=User.find_by!(name:'Jason');kevin=User.find_by!(name:'Kevin');bender=User.find_by!(name:'Bender Bot')
    room=Room.find_by!(name:'Designers')
    path="/rooms/#{room.id}/drive_recipients"
    actor=jz;agent_id=nil;setup={users:{},removed:[],agent_user:nil}
    if spec[:name]=='bot_members'
      room=Room.find(486777696);path="/rooms/#{room.id}/drive_recipients";actor=david
    end
    if %w[ineligible invalid_email nil_email].include?(spec[:name])
      setup[:users]={jason.id=>{status:1},david.id=>{email_address:''},User.find_by!(name:'Rita Lopez').id=>{status:0,name:'Zed Outsider',email_address:'zed@external.test'}}
      setup[:users][david.id][:email_address]='not-an-email' if spec[:name]=='invalid_email'
      setup[:users][david.id][:email_address]=nil if spec[:name]=='nil_email'
      setup[:agent_user]=kevin.id
    elsif spec[:name]=='domains'
      setup[:users]={david.id=>{name:'Amy Smart',email_address:'amy@smartdata.net'},jason.id=>{name:'Bob Cnbs',email_address:'bob@cnbssoftware.com'},kevin.id=>{name:'Cal External',email_address:'cal@contractor.test'}}
    elsif spec[:name]=='agent_selection'
      setup[:agent_user]=kevin.id;setup[:users]={david.id=>{status:1}}
    elsif spec[:name]=='agent_session'
      setup[:agent_user]=jz.id
    end
    Agent.create!(user:User.find(setup[:agent_user]),owner:david) if setup[:agent_user]
    setup[:users].each { |id,attrs|User.find(id).update_columns(attrs) }
    actor=bender if spec[:name]=='bot_session'
    client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
    sign_in(client,actor) unless %w[anonymous anonymous_validate bot_key bot_key_validate].include?(spec[:name])
    path="/rooms/#{Room.find_by!(name:'All Pets').id}/drive_recipients" if %w[nonmember nonmember_validate].include?(spec[:name])
    path+='/validate' if spec[:method]=='post'
    params=case spec[:name]
    when 'canonical','stale','agent_selection' then {user_ids:[kevin.id,david.id]}
    when 'duplicate' then {user_ids:[david.id,david.id]}
    when 'empty' then {user_ids:[]}
    when 'invalid_ids' then {user_ids:[jz.id,bender.id,9_900_000_001]}
    when 'scalar' then {user_ids:david.id}
    when 'missing' then {}
    when 'emails' then {user_ids:['stranger@external.test']}
    when 'oversized' then {user_ids:(1..101).to_a}
    else {user_ids:[david.id]}
    end
    headers={'Accept'=>'application/json'}
    headers['X-CSRF-Token']='RECIPIENT-CSRF' unless spec[:name]=='no_csrf'
    path+="?bot_key=#{bender.id}-BenderToken1" if %w[bot_key bot_key_validate].include?(spec[:name])
    before=nil
    if spec[:name]=='stale'
      client.get(path.delete_suffix('/validate'),headers:);before=observe(client)
      Membership.where(room:,user:kevin).delete_all;setup[:removed]=[kevin.id]
    end
    if spec[:name].start_with?('throttle')
      before=[]
      60.times {client.get(path,headers:);before<<observe(client)}
      if spec[:name]=='throttle_shared'
        spec=spec.merge(method:'post');path+='/validate'
      end
    end
    client.public_send(spec[:method],path,params:spec[:method]=='post' ? params : nil,headers:,as: :json)
    result=observe(client)
    after=nil
    if spec[:name].start_with?('throttle')
      # Independent users retain their budget; both actions share only the requesting user's bucket.
      sign_in(client,david);client.get(path.delete_suffix('/validate'),headers:);after=observe(client)
    end
    rows<<{spec:,actor_id:actor.id,room_id:room.id,path:,setup:,params:,before:,result:,after:}
    raise ActiveRecord::Rollback
  end
end
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:NOW.iso8601,rows:})
warn "Pinned Rails Drive recipients: #{rows.size} real request scenarios; no Google HTTP"
