require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false;load Rails.root.join('db/schema.rb')
travel_to Time.utc(2026,1,1,12)
Account.create!(name:'Slack oracle');user=User.create!(id:811,name:'Oracle <&>',role: :administrator)
workspace=SlackWorkspace.create!(id:851,client_id:'fixture-client',client_secret:'fixture-secret',configured_by:user,team_id:'TFIXTURE')
class Ws16RunGoldenController < ApplicationController
 def form_authenticity_token(form_options: {})
  action,method=form_options.values_at(:action,:method)
  action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
 end
end
controller=Ws16RunGoldenController.new
controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'example.org','rack.input'=>StringIO.new)))
controller.set_response!(ActionDispatch::Response.new)
renderer=Ws16RunGoldenController.renderer.new(http_host:'example.org',https:false,'rack.session'=>{})
conversations=[{id:'C1',name:'Public <&>',type:'public_channel',archived:false,members:3,messages:12,threads:2,target:{action:'merge',room_id:861}}, {id:'G2',name:'Private " <script>',type:'private_channel',archived:true,target:{action:'skip'}}, {id:'D3',name:'Personal',type:'im',target:{action:'merge',room_id:999}}, {id:'M4',name:'Group',type:'mpim'}, {id:'X5',name:'Other',type:'future_kind'}].map(&:stringify_keys)
conversations.each{|c|c['target']=c['target']&.stringify_keys}
stats={'phase'=>'messages','current'=>'#<unsafe &>','api_calls'=>0,'users'=>{'total'=>5,'matched'=>1,'placeholders'=>2,'deactivated'=>1,'bots'=>1},'counts'=>{'rooms_created'=>2,'rooms_merged'=>1,'messages'=>10,'replies'=>4,'threads'=>3,'reactions'=>2,'pins'=>1,'files_linked'=>0,'skipped'=>3},'conversations'=>conversations,'samples'=>[{'conversation'=>'Public <&>','slack_text'=>'<script>alert("x")</script> &','markdown'=>'**Hello** @[Unknown] <script>alert("x")</script>'}]}
room=Rooms::Open.create!(id:861,name:'Room <&>',creator:user)
RoomTarget=Struct.new(:id,:name)
rows=[]
serialize=->(r){r.attributes.slice('id','kind','mode','status','options','stats','error').merge('created_at'=>r.created_at.to_s,'started_at'=>r.started_at&.to_s,'finished_at'=>r.finished_at&.to_s,'user_name'=>r.user.name,'issues_count'=>r.issues.count,'queued_behind'=>r.queued? && SlackImport.active.where.not(id:r.id).exists?,'undo_reason'=>r.undo_blocked_reason,'issues'=>r.issues.order(:id).map{|i|i.attributes.slice('level','slack_ref','message')})}
%w[workspace personal].product(%w[dry_run import],%w[queued running completed failed cancelled undoing undone]).each do |kind,mode,status|
 SlackImport::Issue.delete_all;SlackImport.delete_all
 run=SlackImport.create!(id:853,slack_workspace:workspace,user:,kind:,mode:,status:,stats:,started_at:Time.current-60,finished_at: %w[completed failed cancelled undone].include?(status) ? Time.current : nil,error:status=='failed' ? 'Fixture <&> failure' : nil)
 Current.reset;Current.user=user
 %w[admin personal status].each do |view|
  next if view=='personal' && kind!='personal'
  template={'admin'=>'accounts/slack_import_runs/show','personal'=>'slack/imports/show','status'=>'accounts/slack_import_runs/status'}.fetch(view)
  page=GearedPagination::Recordset.new(run.issues.order(:id),per_page:50).page(nil)
  html=renderer.render(template:,layout:false,assigns:{run:,queued_behind:false,conversations:,issues:[],page:})
  rows << {name:"#{kind}-#{mode}-#{status}-#{view}",view:,data:serialize.call(run),html:}
 end
end
SlackImport.delete_all
run=SlackImport.create!(id:853,slack_workspace:workspace,user:,kind:'workspace',mode:'dry_run',status:'completed',stats:)
rows << {name:'plan',view:'plan',data:serialize.call(run),rooms:[{id:861,name:room.name}],oldest:'2025-12-18',html:renderer.render(template:'accounts/slack_import_runs/plan',layout:false,assigns:{run:,conversations:,samples:stats['samples'],rooms:[room]})}
[false,true].each do |populated|
 runs=populated ? [run] : []
 rows << {name:"index-#{populated}",view:'index',runs:runs.map{|r|serialize.call(r)},html:renderer.render(template:'accounts/slack_import_runs/index',layout:false,assigns:{runs:})}
end
['unset','disconnected','rejected','connected','runs'].each do |state|
 connection=case state
 when 'rejected' then SlackConnection.new(slack_workspace:workspace,user:,access_token:'fixture-user-grant',disconnected_reason:'Rejected <&>')
 when 'connected','runs' then SlackConnection.new(slack_workspace:workspace,user:,access_token:'fixture-user-grant')
 end
 runs=state=='runs' ? [run] : []
 w=state=='unset' ? nil : workspace
 data={team_known:!!w,connection_exists:!!connection,connected:connection&.connected? || false,disconnected_reason:connection&.disconnected_reason&.presence}
 rows << {name:"personal-index-#{state}",view:'personal_index',setup:data,runs:runs.map{|r|serialize.call(r)},html:renderer.render(template:'slack/imports/index',layout:false,assigns:{workspace:w,connection:,runs:})}
end
# Pagination, blocking reason and queued-behind are observable branches beyond basic statuses.
SlackImport.delete_all
run=SlackImport.create!(id:853,slack_workspace:workspace,user:,kind:'workspace',mode:'import',status:'completed',stats:,started_at:Time.current-60)
55.times{|i|run.issues.create!(level:i.even? ? 'warning' : 'error',slack_ref:i.even? ? 'C<&>' : nil,message:"Issue #{i} <&>")}
[1,2,3].each do |number|
 page=GearedPagination::Recordset.new(run.issues.order(:id),per_page:50).page(number.to_s)
 data=serialize.call(run).merge('issues'=>page.records.map{|i|i.attributes.slice('level','slack_ref','message')},'next_page'=>page.last? ? nil : page.next_param.to_i)
 rows << {name:"issues-page-#{number}",view:'admin',data:,html:renderer.render(template:'accounts/slack_import_runs/show',layout:false,assigns:{run:,queued_behind:false,issues:page.records,page:})}
end
SlackImport.create!(id:854,slack_workspace:workspace,user:,kind:'personal',mode:'dry_run',status:'running')
rows << {name:'undo-blocked',view:'admin',data:serialize.call(run).merge('issues'=>run.issues.limit(50).map{|i|i.attributes.slice('level','slack_ref','message')},'next_page'=>2),html:renderer.render(template:'accounts/slack_import_runs/show',layout:false,assigns:{run:,queued_behind:false,issues:run.issues.limit(50),page:GearedPagination::Recordset.new(run.issues,per_page:50).page(nil)})}
run.update!(status:'queued',stats:{})
rows << {name:'queued-behind',view:'status',data:serialize.call(run),html:renderer.render(template:'accounts/slack_import_runs/status',layout:false,assigns:{run:,queued_behind:true})}
File.write(File.join(ENV.fetch('PARITY_WORK'),'vectors/slack/run_views.json'),JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),cases:rows})+"\n")
puts "Slack run views: #{rows.size} complete Rails template bodies generated with shared deterministic CSRF inputs"
