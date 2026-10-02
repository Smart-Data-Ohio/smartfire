require 'json';require 'cgi';require 'rack/mock';require 'base64';require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
Rails.cache=ActiveSupport::Cache::MemoryStore.new
ApplicationController.allow_forgery_protection=true
ActionDispatch::Request.prepend(Module.new{def content_security_policy_nonce; 'NONCE'; end})
ApplicationController.prepend(Module.new do
 def form_authenticity_token(form_options: {})
  action,method=form_options.values_at(:action,:method)
  action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
 end
 def content_security_policy_nonce; 'NONCE'; end
end)
ActiveRecord::Schema.verbose=false
travel_to Time.utc(2026,1,1,12)
Net::HTTP.singleton_class.prepend(Module.new{define_method(:start){|*|raise 'No Slack network allowed in run controller oracle'}})
flash_capture=Module.new{def redirect_to(*args,**kwargs);super.tap{Thread.current[:flash]=flash.to_hash};end}
[Accounts::SlackImportRunsController,Slack::ImportsController].each{|c|c.prepend(flash_capture)}
base='/account/slack_import/runs';personal='/slack/imports'
cases=[]
# Every action's authorization and object scoping.
%w[index create show status plan start_import catch_up cancel undo].each do |action|
 method=%w[create start_import catch_up cancel undo].include?(action) ? 'POST' : 'GET'
 path=action=='index'||action=='create' ? base : "#{base}/853#{action=='show' ? '' : '/'+(action=='start_import' ? 'import' : action)}"
 cases << {name:"admin-member-#{action}",role:0,method:,path:}
 cases << {name:"admin-#{action}",method:,path:}
 unless %w[index create].include?(action)
  cases << {name:"admin-missing-#{action}",method:,path:path.sub('853','999')}
 end
end
%w[index create show status cancel undo].each do |action|
 method=%w[create cancel undo].include?(action) ? 'POST' : 'GET'
 path=action=='index'||action=='create' ? personal : "#{personal}/853#{action=='show' ? '' : '/'+(action=='start_import' ? 'import' : action)}"
 cases << {name:"personal-#{action}",role:0,kind:'personal',method:,path:}
 unless %w[index create].include?(action)
  cases << {name:"personal-foreign-#{action}",role:0,kind:'personal',run_user:812,method:,path:}
  cases << {name:"personal-workspace-#{action}",role:0,method:,path:}
  cases << {name:"personal-missing-#{action}",role:0,method:,path:path.sub('853','999')}
 end
end
cases << {name:'admin-index-equal-created-at',path:base,later:true}
cases << {name:'personal-index-equal-created-at',path:personal,later:true,kind:'personal',role:0}
cases << {name:'setup-active-equal-created-at',path:'/account/slack_import',status:'running',other_active:true}
[true,false].each do |admin|
 path=admin ? base : personal
 cases << {name:"#{admin}-unconfigured",method:'POST',path:,unconfigured:true,role:admin ? 1 : 0}
 cases << {name:"#{admin}-unconnected",method:'POST',path:,unconnected:true,role:admin ? 1 : 0}
 cases << {name:"#{admin}-rejected",method:'POST',path:,rejected:true,role:admin ? 1 : 0}
 cases << {name:"#{admin}-own-active",method:'POST',path:,status:'running',role:admin ? 1 : 0}
 cases << {name:"#{admin}-other-active",method:'POST',path:,other_active:true,role:admin ? 1 : 0}
 cases << {name:"#{admin}-csrf",method:'POST',path:,bad_csrf:true,role:admin ? 1 : 0}
end
cases << {name:'dry-run-options',method:'POST',path:base,body:{include_private:'0',oldest:'2025-01-02',latest:'2025-01-03'}}
cases << {name:'dry-run-invalid-dates',method:'POST',path:base,body:{oldest:'bad',latest:'2025-02-30'}}
cases << {name:'dry-run-time-zone',method:'POST',path:base,zone:'America/New_York',body:{oldest:'2025-03-09',latest:'2025-03-09'}}
['test','full'].each do |preset|
 cases << {name:"start-#{preset}",method:'POST',path:"#{base}/853/import",mode:'dry_run',body:{preset:,conversation_ids:['C1','BAD','C1','G2'],room_targets:{C1:'861',G2:'skip',BAD:'new'},oldest:'2025-03-09',latest:'2025-03-09'}}
end
cases << {name:'start-default-test',method:'POST',path:"#{base}/853/import",mode:'dry_run',body:{conversation_ids:['C1']}}
cases << {name:'start-targets',method:'POST',path:"#{base}/853/import",mode:'dry_run',body:{preset:'full',conversation_ids:['C1','G2','D3','M4','X5'],room_targets:{C1:'861',G2:'862',D3:'863',M4:'864',X5:'-861'}}}
['new','skip','000861','junk'].each{|target|cases << {name:"target-#{target}",method:'POST',path:"#{base}/853/import",mode:'dry_run',body:{preset:'full',conversation_ids:'C1',room_targets:{C1:target}}}}
[true,false].each do |admin|
 path=admin ? "#{base}/853/import" : personal
 c={method:'POST',path:,mode:'dry_run',kind:admin ? 'workspace' : 'personal',role:admin ? 1 : 0}
 body=admin ? {} : {mode:'import',dry_run_id:853}
 cases << c.merge(name:"#{admin}-empty-selection",body:body.merge(conversation_ids:['',' ']))
 cases << c.merge(name:"#{admin}-unknown-selection",body:body.merge(conversation_ids:['BAD']))
 cases << c.merge(name:"#{admin}-valid-selection",body:body.merge(conversation_ids:['C1','BAD','C1','G2'],room_targets:{C1:861},oldest:'2025-01-01'))
 cases << c.merge(name:"#{admin}-unfinished-preview",status:'running',body:body.merge(conversation_ids:['C1']))
 cases << c.merge(name:"#{admin}-wrong-kind-preview",kind:admin ? 'personal' : 'workspace',body:body.merge(conversation_ids:['C1']))
end
cases << {name:'personal-wrong-preview',method:'POST',path:personal,role:0,kind:'personal',run_user:812,mode:'dry_run',body:{mode:'import',dry_run_id:853,conversation_ids:['C1']}}
cases << {name:'catch-up',method:'POST',path:"#{base}/853/catch_up",options:{conversation_ids:['C1'],room_targets:{C1:861},include_private:false}}
cases << {name:'catch-up-bounded',method:'POST',path:"#{base}/853/catch_up",options:{oldest:'2025-01-01T00:00:00Z'}}
cases << {name:'catch-up-dry',method:'POST',path:"#{base}/853/catch_up",mode:'dry_run'}
cases << {name:'catch-up-active',method:'POST',path:"#{base}/853/catch_up",other_active:true}
cases << {name:'catch-up-no-grant',method:'POST',path:"#{base}/853/catch_up",unconnected:true}
cases << {name:'plan-pending',path:"#{base}/853/plan",mode:'dry_run',status:'queued'}
%w[workspace personal].product(%w[queued running completed failed cancelled undoing undone],%w[cancel undo]).each do |kind,status,action|
 cases << {name:"#{kind}-#{status}-#{action}",method:'POST',path:"#{base}/853/#{action}",kind:,status:}
end
cases << {name:'undo-blocked-active',method:'POST',path:"#{base}/853/undo",other_active:true}
cases << {name:'undo-blocked-later-same',method:'POST',path:"#{base}/853/undo",later:true}
cases << {name:'undo-blocked-later-other',method:'POST',path:"#{base}/853/undo",later:true,later_user:812}
cases << {name:'undo-blocked-finishing',method:'POST',path:"#{base}/853/undo",lease:true}
cases << {name:'undo-dry-run',method:'POST',path:"#{base}/853/undo",mode:'dry_run'}
rows=cases.map do |c|
 ActiveRecord::Base.clear_query_caches_for_current_thread;load Rails.root.join('db/schema.rb')
 Account.create!(name:'Slack oracle');user=User.create!(id:811,name:'Oracle',role:c.fetch(:role,1));other=User.create!(id:812,name:'Other')
 user.update_columns(time_zone:c[:zone]) if c[:zone]
 session=Session.create!(user:,two_factor_verified_at:Time.current)
 w=SlackWorkspace.create!(id:851,client_id:'fixture-client',client_secret:'fixture-secret',configured_by:user,team_id:c[:unconfigured] ? nil : 'TFIXTURE')
 connection=SlackConnection.create!(id:852,slack_workspace:w,user:,slack_user_id:'UFIXTURE',access_token:'fixture-user-grant',disconnected_reason:c[:rejected] ? 'Rejected' : nil) unless c[:unconnected]
 Rooms::Open.create!(id:861,name:'Existing open',creator:user);Rooms::Closed.create!(id:862,name:'Existing closed',creator:user)
 Rooms::Open.create!(id:863,name:'Deleted',creator:user).update_columns(deleted_at:Time.current)
 Rooms::Direct.create!(id:864,name:'Direct',creator:user)
 options=c.fetch(:options,{}).stringify_keys
 stats={'conversations'=>%w[C1 G2 D3 M4 X5].map{|id|{'id'=>id,'target'=>{'action'=>'create'}}}}
 run=SlackImport.create!(id:853,slack_workspace:w,slack_connection:connection,user:c[:run_user]==812 ? other : user,kind:c.fetch(:kind,'workspace'),mode:c.fetch(:mode,'import'),status:c.fetch(:status,'completed'),options:,stats:,started_at:Time.current-60,state:c[:lease] ? {'step_started_at'=>Time.current.iso8601(6),'step_lease_token'=>'fixture-lease'} : {})
 if c[:other_active] || c[:later]
  SlackImport.create!(id:854,slack_workspace:w,user:c[:later_user]==812 ? other : c[:later] ? user : other,kind:'personal',mode:'import',status:c[:later] ? 'completed' : 'running',stats:,started_at:Time.current)
 end
 Thread.current[:flash]={};ActiveJob::Base.queue_adapter.enqueued_jobs.clear
 request=ActionDispatch::Request.new(Rails.application.env_config.merge(Rack::MockRequest.env_for('http://example.org/')))
 cookies=ActionDispatch::Cookies::CookieJar.build(request,{})
 cookies.signed[:session_token]=session.token
 csrf=Base64.urlsafe_encode64([7].pack('C')*32,padding:false)
 cookies.encrypted[:_campfire_session]={value:{'session_id'=>'0123456789abcdef0123456789abcdef','_csrf_token'=>csrf}}
 cookie="session_token=#{CGI.escape(cookies[:session_token])}; _campfire_session=#{CGI.escape(cookies[:_campfire_session])}"
 masked=Base64.urlsafe_encode64(([9]*32+[14]*32).pack('C*'),padding:false)
 response=Rack::MockRequest.new(Rails.application).request(c.fetch(:method,'GET'),'http://example.org'+c[:path],'HTTP_COOKIE'=>cookie,'CONTENT_TYPE'=>'application/json','HTTP_X_CSRF_TOKEN'=>c[:bad_csrf] ? 'invalid' : masked,'action_dispatch.content_security_policy_nonce_generator'=>->(_){'NONCE'},input:JSON.generate(c.fetch(:body,{})))
 snapshot=SlackImport.order(:id).map{|r|r.attributes.slice('id','slack_workspace_id','slack_connection_id','user_id','kind','mode','status','options','state','stats','error','started_at','heartbeat_at','finished_at','created_at','updated_at')}
 audit=AuditLog.order(:id).map{|l|l.attributes.slice('action','actor_id','target_type','target_id','details')}
 jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.map{|j|{'class'=>j[:job].name,'arguments'=>j[:args],'queue'=>j[:queue]}}
 {**c,status_code:response.status,location:response['Location'],flash:Thread.current[:flash],runs:snapshot,audit:,jobs:,response_body:response.body}
end
File.write(File.join(ENV.fetch('PARITY_WORK'),'vectors/slack/runs_http.json'),JSON.pretty_generate(rows)+"\n")
puts "Slack run HTTP oracle: #{rows.size} real Rails action cases with signed sessions and verified CSRF generated"
