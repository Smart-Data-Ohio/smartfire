require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
module Ws15eNoFizzy
 def start(*) = raise('Approval construction must not call HTTP')
end
Net::HTTP.singleton_class.prepend(Ws15eNoFizzy)
ActionCable.server.define_singleton_method(:broadcast) { |*| }
Time.zone='UTC'
fields={kind:'comment',number:579,body:' Nice work '}.stringify_keys
cases=%w[comment create move close reopen override pref_optout blank_external replay replay_expired replay_cap forbidden room_grant suspended missing_owner no_account disconnected invalid invalid_multiple budget payload_large summary_large]
vectors=cases.map do |name|
 travel_to(Time.utc(2026,3,2,16,0,0)) do
  User.update_all(status: :deactivated)
   owner=User.create!(name:'Owner',role: :member)
   admin=User.create!(name:'Admin',role: :administrator)
   bot=User.create!(name:'Agent',role: :bot)
   agent=Agent.create!(user:bot,owner:owner,kind:'workspace')
   agent.update_column(:owner_id,nil) if name=='missing_owner'
   agent.update_column(:suspended_at,Time.current) if name=='suspended'
   account=nil
   unless name=='no_account'
    account=FizzyConnectedAccount.create!(user:owner,fizzy_account_id:'897362094',fizzy_user_id:'03user1',fizzy_user_name:'Owner',access_token:'fixture-owner')
    account.update!(disconnected_reason:'Disconnected') if name=='disconnected'
   end
   AgentGrant.create!(agent:agent,granted_by:admin,capability:'external_action') unless %w[forbidden room_grant].include?(name)
   if name=='room_grant'
    room=Room.create!(name:'Room',type:'Rooms::Closed',creator:owner)
    AgentGrant.create!(agent:agent,granted_by:admin,capability:'external_action',room:room)
   end
   owner.update_column(:inbox_preferences,{'agent_approvals'=>false}) if name=='pref_optout'
   input=fields.dup
   input.merge!('kind'=>'create','board_id'=>'board','title'=>' Ship it ','description'=>'Description') if %w[create payload_large].include?(name)
   input.merge!('kind'=>'move','column_id'=>'col') if name=='move'
   input['kind']=name if %w[close reopen].include?(name)
   input['account_id']='other' if name=='override'
   input['external_id']='  ' if name=='blank_external'
   input['external_id']=' replay ' if name.start_with?('replay')
   input.delete('body') if name=='invalid'
   input.merge!('kind'=>'move','account_id'=>'bad/id','number'=>'-1','column_id'=>nil) if name=='invalid_multiple'
   input['description']='界'*1400 if name=='payload_large'
   input['account_id']='a'*550 if name=='summary_large'
   if name=='budget'
    agent.update!(daily_external_action_cap:1)
    AgentApproval.create!(agent:agent,action:'other',summary:'Other request',status:'denied')
   end
   if name.start_with?('replay')
    Agents::FizzyCardActions.create(agent:agent,fields:input)
    input={'external_id'=>' replay ','kind'=>'invalid'}
    agent.update!(daily_external_action_cap:1) if name=='replay_cap'
    AgentApproval.last.update_column(:expires_at,Time.current) if name=='replay_expired'
   end
   result=Agents::FizzyCardActions.create(agent:agent.reload,fields:input)
   Agents::FizzyCardActions.create(agent:agent,fields:input) if name=='budget'
   response=result.ok? ? result.payload : result.failure_body
   # IDs and times are exported through independently asserted row relationships/TTL.
   response=response.except(:id,:expires_at) if result.ok?
   approvals=AgentApproval.where(agent:agent).order(:id).map do |a|
    {action:a.action,summary:a.summary,payload:a.payload,external_id:a.external_id,status:a.status,ttl:(a.expires_at-a.created_at).to_i,room:a.room_id,identity_match: a.fizzy_connected_account_id==account&.id && a.fizzy_user_id==account&.fizzy_user_id,identity_name:a.fizzy_user_name}
   end
   ids=AgentApproval.where(agent:agent).pluck(:id)
   notice_ids=AgentBudgetNotice.where(agent:agent).pluck(:id)
   inbox=ActivityItem.where(source_type:'AgentApproval',source_id:ids).or(ActivityItem.where(source_type:'AgentBudgetNotice',source_id:notice_ids)).order(:id).map { |item| {recipient:item.user.name,type:item.event_type,handled:item.handled_at.present?,read:item.read_at.present?} }.sort_by { |i| [i[:type],i[:recipient]] }
   vector={name:name,fields:input,status:Rack::Utils.status_code(result.status),body:response,error:result.error,approvals:approvals,inbox:inbox,notices:AgentBudgetNotice.where(agent:agent).pluck(:cap,:day).map { |cap,day| {cap:cap,day:day.to_s} }}
   Thread.current[:ws15e_vector]=vector
 end
 Thread.current[:ws15e_vector]
end
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),requests:vectors})+"\n")
puts "WS15e Fizzy agent requests Rails oracle: #{vectors.size} service cases"
