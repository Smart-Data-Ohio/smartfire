require "json"
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
  raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false
load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026,1,1,12)
Account.create!(name:"Domain oracle")
owner=User.create!(name:"Oracle",email_address:"oracle@example.test",password:"fixture-password")
room=Rooms::Closed.create!(creator:owner,name:"Domain")
direct=Rooms::Direct.create!(creator:owner,name:"Direct")
display=[{}, {payload:{base:{repo:{full_name:"Smart-Data-Ohio/Smartfire"}}}},
 {html_url:"https://github.com/Smart-Data-Ohio/Smartfire/pull/5"},
 {html_url:"https://evil.test/github.com/A/B trailing"}]
[nil,{},[],true,3,"bad","A/B","A/B\n","A /B","A/B/C","A\u00a0/B","A\u2003/B"].each { |v| display<<{payload:{base:{repo:{full_name:v}}}} }
[nil,[],3,"bad",{base:[]},{base:{repo:true}}].each { |v| display<<{payload:v} }
display=display.map do |attrs|
 pr=Github::PullRequest.new(owner:"stored",repo:"names",number:5,**attrs)
 begin
  {attributes:attrs,display:pr.display_full_name}
 rescue=>error
  {attributes:attrs,error:error.class.name}
 end
end
files=[nil,"","  ","oops","[]","null","false","3","{}",
 JSON.generate({files:[{filename:"a"}],total_count:0})]
[nil,false,true,3,"single",{a:1,b:2},[1,2]].each { |v| files<<JSON.generate({files:v,total_count:" 2tail"}) }
[nil,false,true,[],{},-3,1,2.9,"wat","12tail","-3"].each { |v| files<<JSON.generate({files:[1,2,3],total_count:v}) }
files=files.map do |storage|
 begin
  {storage:,summary:Github::PullRequest.new(changed_files:storage).changed_files_summary}
 rescue=>error
  {storage:,error:error.class.name}
 end
end
module DomainBroadcastCapture
 def broadcast_card_updates; Thread.current[:domain_broadcasts]<<id; end
end
Github::PullRequest.prepend(DomainBroadcastCapture)
updates=[{}, {title:nil}, {title:"Title"}, {private:false}, {owner:"MiXeD"}, {repo:" "}, {number:0}, {owner:""}].map do |attrs|
 travel_to Time.utc(2026,1,1,12)
 pr=Github::PullRequest.for_reference(owner:"Rails",repo:"Rails",number:1)
 Thread.current[:domain_broadcasts]=[]
 travel_to Time.utc(2026,1,1,12,1)
 ok=pr.update(attrs)
 result={attributes:attrs,ok:,errors:pr.errors.messages,updated_at:pr.reload.updated_at.strftime("%F %T.%6N"),broadcasts:Thread.current[:domain_broadcasts].size}
 pr.delete
 result
end
subs=[]
["rails"," Rails ",".","..","...","has space","rails/hack","","\u00a0Rails\u00a0","my-org"].each do |name|
 ["owner","repo"].each do |field|
  attrs={owner:"rails",repo:"rails",events:[]}.merge(field.to_sym=>name)
  sub=Github::RepositorySubscription.new(room:,**attrs)
  subs<<{attributes:attrs,create:true,valid:sub.valid?,owner:sub.owner,repo:sub.repo,events:sub.events,errors:sub.errors.messages}
 end
end
[nil,[],false,{},"",["opened","closed"],["opened","push"],[1],"opened"].each do |events|
 [true,false].each do |create|
  attrs={owner:"rails",repo:"rails",events:}
  sub=Github::RepositorySubscription.new(room:,**attrs)
  sub.id=999 unless create
  sub.instance_variable_set(:@new_record,false) unless create
  subs<<{attributes:attrs,create:,valid:sub.valid?,owner:sub.owner,repo:sub.repo,events:sub.events,errors:sub.errors.messages}
 end
end
vectors={display:,files:,updates:,subscriptions:subs}
File.write("/work/vectors/github_domain.json",JSON.pretty_generate(vectors)+"\n")
puts "GitHub domain Rails oracle: #{display.size} display, #{files.size} file summaries, #{updates.size} saves, #{subs.size} subscription validations; reference d7c7de92"
