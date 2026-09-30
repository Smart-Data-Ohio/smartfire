require "json"
load "/tools/load_fixtures.rb"
ActiveJob::Base.queue_adapter=:test
changes=[nil,"plain",["a","b"],{name:["Old","New"],password:"secret",current_password:"secret",pwd:"secret",access_token:"secret",api_key:"secret",bot_key:"secret",session:{id:1},nested:{credentials:{token:"secret"},keep:"yes"},list:[{secret:"secret"},"plain"],join_code:"secret",transfer_id:"secret",email_address:["a@example.test","b@example.test"],key:"keep",KEYBOARD:"keep",apiKey:"secret",before:{authorization:"secret"}}]
filters=changes.map{|input|{input:input,output:AuditLog.filter_secrets(input)}}
labels=[nil,"","  ","hunter2","victim@example.com"," DAVID@37signals.com ","dotless@intranet","a@.example","a@example.","a@@example.com","a@a.b","a b@example.com","a\u00a0b@example.com",("a"*300)+"@example.com",("a"*990)+"@example.com"]
User.create!(name:"Dotless",email_address:"dotless@intranet")
labels=labels.map{|input|{input:input,label:AuditLog.failure_actor_label(input)}}
urls=[nil,""," ","https://example.com/hook?token=secret","http://example.com:3000/hook","https://example.com:443/hook","HTTPS://EXAMPLE.COM:443/x","https://user:password@[::1]:8443/path?q=secret","ftp://example.com:21/x","custom://host:42/x","http://é.test","::::","/relative","https://example.com/a","https://example.com/b"]
origins=urls.map{|input|{input:input,summary:AuditLog.webhook_origin_summary(input)}}
admin=User.find(ActiveRecord::FixtureSet.identify("david")); member=User.find(ActiveRecord::FixtureSet.identify("kevin"));room=Room.find(ActiveRecord::FixtureSet.identify("watercooler"))
Current.user=admin
entries=[AuditLog.record!(action:"user.role.change",actor:admin,target:member,changes:{role:AuditLog.pair("member","administrator"),token:"secret"},ip_address:"203.0.113.7",user_agent:"a"*600), AuditLog.record!(action:"room.destroy",target:room),AuditLog.record!(action:"future.action",actor_label:"Override",target_label:"Target",changes:"context")]
records=entries.map{|e|e.attributes.except("id","created_at","updated_at")}
readonly=entries.first
refusals=[:save!,:destroy,:delete].map{|method|begin;readonly.public_send(method);"allowed";rescue ActiveRecord::ReadOnlyRecord;"readonly";end}
refusals << (begin;readonly.update!(action:"user.unban");"allowed";rescue ActiveRecord::ReadOnlyRecord;"readonly";end)
validation=["", " ", "future.action"].map{|action|{action:action,valid:AuditLog.new(action:action).valid?}}
Current.reset
request=ActionDispatch::TestRequest.create;request.remote_addr="198.51.100.9";request.user_agent="Audit browser"
inputs=[{email:"victim@example.com",offset:0},{email:"Victim@Example.com",offset:0}]+(0..20).map{|i|{email:"target#{i}@example.com",offset:0}}+[{email:"victim@example.com",offset:300},{email:"victim@example.com",offset:301},{email:"hunter2",offset:301},{email:"not a password",offset:301}]
base=Time.current
failures=inputs.map do|row|
 travel_to(base+row[:offset],with_usec:true)
 entry=AuditLog.record_sign_in_failure!(email:row[:email],method:"password",request:request)
 row.merge(result:entry && entry.attributes.slice("action","actor_id","actor_label","details","ip_address","user_agent"),count:AuditLog.where(action:"session.sign_in.failure").count)
end
File.write(ARGV.fetch(0),JSON.pretty_generate({actions:AuditLog::ACTIONS,filters:filters,labels:labels,origins:origins,records:records,refusals:refusals,validation:validation,failures:failures})+"\n")
puts "WS8 audit vectors: #{AuditLog::ACTIONS.size} actions, #{filters.size} redactions, #{labels.size} labels, #{origins.size} origins, #{records.size} snapshots, #{failures.size} throttled writes"
