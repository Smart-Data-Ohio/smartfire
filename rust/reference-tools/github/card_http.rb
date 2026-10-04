require "json"
require "digest"
require "cgi"
require "rack/mock"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
 raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
Rails.cache=ActiveSupport::Cache::MemoryStore.new
ActiveRecord::Schema.verbose=false
load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026,1,1,12)
Account.create!(name:"HTTP card oracle")
owner=User.create!(id:811,name:"Oracle",email_address:"oracle@example.test",password:"fixture-password",role: :administrator)
room=Rooms::Closed.create!(id:815,creator:owner,name:"Cards")
other=Rooms::Closed.create!(id:825,creator:owner,name:"Other")
Membership.find_or_create_by!(room:,user:owner)
message=room.messages.create!(id:818,creator:owner,markdown_source:"Discussion",client_message_id:"card-parent")
other.messages.create!(id:828,creator:owner,markdown_source:"Other",client_message_id:"other-parent")
thread=ChannelThread.create!(id:817,room:,creator:owner,parent_message:message,name:"Discussion")
session=Session.create!(user:owner,two_factor_verified_at:Time.current)
cookie_request=ActionDispatch::Request.new(Rails.application.env_config.merge(Rack::MockRequest.env_for("http://example.org/")))
cookies=ActionDispatch::Cookies::CookieJar.build(cookie_request,{})
cookies.signed[:session_token]=session.token
cookie="session_token=#{CGI.escape(cookies[:session_token])}"
module CardHttpAccess
 def repository_readable?(owner,repo)
  Thread.current[:card_requests]<<[owner,repo]
  case Thread.current[:card_status]
  when 401 then raise Github::WriteClient::Unauthorized,"GitHub rejected the linked token"
  when 404 then false
  when 500 then raise Github::WriteClient::Error,"GitHub refused the request"
  else true
  end
 end
end
Github::WriteClient.prepend(CardHttpAccess)
cases=[{name:"readable",linked:true},{name:"unlinked"},{name:"disconnected",linked:true,disconnected:true},
 {name:"denied",linked:true,access:404},{name:"unauthorized",linked:true,access:401},{name:"cached",linked:true,repeat:2},
 {name:"transport",linked:true,access:500,repeat:2},{name:"public",private:false},{name:"unknown",private:nil},
 {name:"thread_readable",linked:true,thread:true},{name:"thread_unlinked",thread:true},
 {name:"nonmember",linked:true,member:false},{name:"cross_room",message:828,linked:true},
 {name:"unreferenced",reference:false,linked:true},{name:"missing",context:false,linked:true},
 {name:"both_contexts",linked:true,thread:true,both:true}, {name:"missing_thread_mapping",thread:true,mapping:false,linked:true},
 {name:"deleted_room",deleted:true,linked:true},{name:"message_prefix_cast",message:"818tail",linked:true}]
vectors=cases.map do |c|
 Rails.cache.clear;GithubConnectedAccount.delete_all;Github::PullRequestReference.delete_all;Github::PullRequestThread.delete_all;Github::PullRequest.delete_all
 room.update_columns(deleted_at:c[:deleted] ? Time.current : nil)
 Membership.find_or_create_by!(room:,user:owner)
 Membership.where(room:,user:owner).delete_all if c[:member]==false
 pr=Github::PullRequest.create!(id:816,owner:"rails",repo:"rails",number:12,title:"Secret title",private:c.fetch(:private,true),state:"open",fetched_at:Time.current,changed_files:JSON.generate({files:[{filename:"app/a.rb",status:"modified",additions:4,deletions:2}],total_count:3}))
 Github::PullRequestReference.create!(pull_request:pr,message:) unless c[:reference]==false
 Github::PullRequestThread.create!(pull_request:pr,room:,channel_thread:thread) unless c[:mapping]==false
 GithubConnectedAccount.create!(user:owner,github_login:"oracle",access_token:"fixture-viewer-token",token_source:"pat",disconnected_reason:c[:disconnected] ? "Disconnected" : nil) if c[:linked]
 Thread.current[:card_requests]=[];Thread.current[:card_status]=c.fetch(:access,200)
 context=c[:context]==false ? "" : c[:thread] ? "?thread_id=817#{c[:both] ? '&message_id=818' : ''}" : "?message_id=#{c.fetch(:message,818)}"
 responses=Array.new(c.fetch(:repeat,1)) {res=Rack::MockRequest.new(Rails.application).get("http://example.org/rooms/815/github/pull_requests/816/card#{context}","HTTP_COOKIE"=>cookie);{status:res.status,body:res.body,content_type:res['Content-Type']}}
 raise "Signed session did not authenticate" if responses.any? { |r| [302,303].include?(r[:status]) }
 {**c,initially_disconnected:c[:disconnected]||false,responses:,requests:Thread.current[:card_requests],disconnected:GithubConnectedAccount.first&.disconnected_reason}
end
File.write("/work/vectors/github_card_http.json",JSON.pretty_generate(vectors)+"\n")
puts "GitHub viewer card Rails oracle: #{vectors.size} HTTP cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
