require 'json'; require 'digest'; require 'rack/mock'; require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch('GITHUB_REFERENCE_HASHES'))).each{|p,h|raise "Reference drift: #{p}" unless Digest::SHA256.file(Rails.root.join(p)).hexdigest==h}
ActiveJob::Base.queue_adapter=:test; ApplicationController.allow_forgery_protection=false
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false;load Rails.root.join('db/schema.rb');travel_to Time.utc(2026,1,1,12)
Rails.cache=ActiveSupport::Cache::MemoryStore.new
module NoGithubCalls
 def start(host,*args,**kwargs);raise "Agent request unexpectedly contacted #{host}";end
end
Net::HTTP.singleton_class.prepend(NoGithubCalls)
cases=[{name:'bad',secret:'bad'},{name:'suspended',suspended:true},{name:'expired_credential',expired:true},{name:'revoked_credential',revoked:true},
 {name:'nonmember',member:false},{name:'unknown_room',room:999},{name:'unmapped',mapping:false},{name:'cross_room',room:825,other_member:true},{name:'missing_pr',missing_pr:true},
 {name:'no_grant',grant:'post_messages'},{name:'cross_grant',grant_room:825},{name:'no_account',linked:false},{name:'disconnected',disconnected:true},
 {name:'blank_comment',submitted:{kind:'comment',body:' '}},{name:'blank_changes',submitted:{kind:'request_changes'}},{name:'invalid_reviewers',submitted:{kind:'request_review',reviewers:'alice, bob!!'}},{name:'bad_kind',submitted:{kind:'merge'}},
 {name:'comment',submitted:{kind:'comment',body:' Nice work ',external_id:'gh-1'}},{name:'approve',submitted:{kind:'approve'}},{name:'changes',submitted:{kind:'request_changes',body:'Fix the typo'}},
 {name:'reviewers',submitted:{kind:'request_review',reviewers:' @Alice, alice @BOB '}},{name:'array_reviewers',submitted:{kind:'request_review',reviewers:['alice','@bob']}},
 {name:'replay',submitted:{kind:'comment',body:'First',external_id:'gh-dup'},replay:true},{name:'budget',cap:0},{name:'throttle',repeat:61}]
vectors=cases.map do |c|
 ActiveRecord::Base.connection.execute('DROP TABLE IF EXISTS message_search_index')
 ActiveRecord::Base.connection.disable_referential_integrity {load Rails.root.join('db/schema.rb')}
 ActiveRecord::Base.connection.schema_cache.clear!
 ActiveRecord::Base.descendants.each(&:reset_column_information)
 Account.create!(name:'Agent HTTP oracle');owner=User.create!(id:811,name:'Oracle',role: :administrator)
 bot=User.create!(id:813,name:'Machine',role: :bot);agent=Agent.create!(id:881,user:bot,owner:owner)
 credential=AgentCredential.create!(id:882,agent:agent,created_by:owner,name:'HTTP',token_digest:AgentCredential.digest('fixture-agent-secret'),token_last_four:'cret')
 room=Rooms::Closed.create!(id:815,creator:owner,name:'Cards');other=Rooms::Closed.create!(id:825,creator:owner,name:'Other')
 message=room.messages.create!(id:818,creator:owner,markdown_source:'Discuss',client_message_id:'parent')
 thread=ChannelThread.create!(id:817,room:room,creator:owner,parent_message:message)
 pr=Github::PullRequest.create!(id:816,owner:'rails',repo:'rails',number:12)
 Membership.create!(room:room,user:bot)

 ActivityItem.delete_all;AgentApproval.delete_all;AgentBudgetNotice.delete_all;AgentGrant.delete_all;GithubConnectedAccount.delete_all;Github::PullRequestThread.delete_all;ActiveRecord::Base.connection.execute("DELETE FROM sqlite_sequence WHERE name = 'agent_approvals'");Rails.cache.clear
 agent.update_columns(last_seen_at:nil,suspended_at:c[:suspended] ? Time.current : nil,daily_external_action_cap:c[:cap]);credential.update_columns(expires_at:c[:expired] ? Time.current : nil,revoked_at:c[:revoked] ? Time.current : nil,last_used_at:nil,last_used_ip:nil)
 Membership.where(user:bot).delete_all;Membership.create!(room:room,user:bot) unless c[:member]==false;Membership.create!(room:other,user:bot) if c[:other_member]
 Github::PullRequestThread.create!(pull_request:pr,room:room,channel_thread:thread) unless c[:mapping]==false
 AgentGrant.create!(agent:agent,room:Room.find(c.fetch(:grant_room,815)),granted_by:owner,capability:c.fetch(:grant,'external_action'))
 account=GithubConnectedAccount.create!(user:bot,github_login:'machine',access_token:'fixture-agent-token') unless c[:linked]==false
 account.mark_disconnected!('Disconnected') if c[:disconnected]
 body={pull_request_id:816}.merge(c.fetch(:submitted,{kind:'comment',body:'hi'}));body.delete(:pull_request_id) if c[:missing_pr]
 path="/rooms/#{c.fetch(:room,815)}/agents/github/pull_request_actions"
 statuses=[];res=nil
 c.fetch(:repeat,1).times do
  res=Rack::MockRequest.new(Rails.application).post('http://example.org'+path,'CONTENT_TYPE'=>'application/json','HTTP_AUTHORIZATION'=>['Bearer', c.fetch(:secret,'fixture-agent-secret')].join(' '),input:JSON.generate(body));statuses<<res.status
 end
 replay=nil
 if c[:replay]
  r=Rack::MockRequest.new(Rails.application).post('http://example.org'+path,'CONTENT_TYPE'=>'application/json','HTTP_AUTHORIZATION'=>['Bearer', 'fixture-agent-secret'].join(' '),input:JSON.generate(body.merge(kind:'merge',body:'Second')))
  replay={status:r.status,body:JSON.parse(r.body)}
 end
 rows=AgentApproval.order(:id).map{|a|{action:a.action,summary:a.summary,payload:JSON.parse(a.payload),external_id:a.external_id,status:a.status,expires_at:a.expires_at.iso8601,agent_id:a.agent_id,agent_credential_id:a.agent_credential_id,room_id:a.room_id,github_login:a.github_login,inbox:ActivityItem.exists?(source:a,user:owner)}}
 {**c,request_body:body,path:path,status:res.status,body:res.body.empty? ? nil : JSON.parse(res.body),statuses:statuses,cache_control:res['Cache-Control'],retry_after:res['Retry-After'],replay:replay,approvals:rows,credential_used:credential.reload.last_used_at.present?,agent_seen:agent.reload.last_seen_at.present?}
end
File.write('/work/vectors/github_agent_http.json',JSON.pretty_generate(vectors)+"\n")
puts "GitHub agent HTTP Rails oracle: #{vectors.size} authorization/action/replay/budget/throttle cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
