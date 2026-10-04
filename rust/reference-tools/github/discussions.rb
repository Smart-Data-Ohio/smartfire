require "json";require "digest";require "rack/mock";require "cgi";require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each{|p,h|raise "Reference drift: #{p}" unless Digest::SHA256.file(Rails.root.join(p)).hexdigest==h}
ActiveJob::Base.queue_adapter=:test;ApplicationController.allow_forgery_protection=false
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false;load Rails.root.join("db/schema.rb");travel_to Time.utc(2026,1,1,12)
Account.create!(name:"Discussion oracle");user=User.create!(id:811,name:"Oracle",role: :administrator)
room=Rooms::Closed.create!(id:815,creator:user,name:"Cards")
message=room.messages.create!(id:818,creator:user,markdown_source:"Discussion",client_message_id:"card-parent")
other=Rooms::Closed.create!(id:825,creator:user,name:"Other")
other_message=other.messages.create!(id:828,creator:user,markdown_source:"Other",client_message_id:"other-parent")
pr=Github::PullRequest.create!(id:816,owner:"rails",repo:"rails",number:12)
Github::PullRequestReference.create!(pull_request:pr,message:)
session=Session.create!(user:,two_factor_verified_at:Time.current)
r=ActionDispatch::Request.new(Rails.application.env_config.merge(Rack::MockRequest.env_for("http://example.org/")));cookies=ActionDispatch::Cookies::CookieJar.build(r,{});cookies.signed[:session_token]=session.token;cookie="session_token=#{CGI.escape(cookies[:session_token])}"
cases=[{name:"new"},{name:"existing",mapping:true},{name:"nonmember",member:false},{name:"unreferenced",reference:false},{name:"reply",reply:true},{name:"direct",kind:"Rooms::Direct"},{name:"missing_pr",pull_request_id:9999},{name:"wrong_room",message_id:828}]
vectors=cases.map do |c|
 message.update_columns(thread_id:nil)
 Github::PullRequestThread.delete_all;ThreadMembership.delete_all;ChannelThread.delete_all;Membership.where(room:,user:).delete_all
 Membership.create!(room:,user:) unless c[:member]==false
 room.update_columns(type:c.fetch(:kind,"Rooms::Closed"))
 message.update_columns(thread_id:nil)
 Github::PullRequestReference.delete_all;Github::PullRequestReference.create!(pull_request:pr,message:) unless c[:reference]==false
 # Fixture IDs stay fixed so complete redirect paths can be compared without normalization.
 unless c[:mapping]||c[:reply]
  ChannelThread.create!(id:817,room:other,creator:user,parent_message:other_message,name:"Other")
 end
 if c[:mapping]||c[:reply]
  thread=ChannelThread.create!(id:817,room:Room.find(815),creator:user,parent_message:message,name:"Discussion")
  Github::PullRequestThread.create!(pull_request:pr,room:,channel_thread:thread) if c[:mapping]
  message.update_columns(thread_id:817) if c[:reply]
 end
 ActiveJob::Base.queue_adapter.enqueued_jobs.clear
 body={pull_request_id:c.fetch(:pull_request_id,816),message_id:c.fetch(:message_id,818)}
 res=Rack::MockRequest.new(Rails.application).post("http://example.org/rooms/815/github/pull_request_threads","HTTP_COOKIE"=>cookie,"CONTENT_TYPE"=>"application/json",input:JSON.generate(body))
 mapping=Github::PullRequestThread.first
 {**c,request_body:body,status:res.status,location:res["Location"],body:res.status<400 ? res.body : nil,threads:ChannelThread.where(room_id:815).count,mappings:Github::PullRequestThread.count,mapping_thread:mapping&.channel_thread_id,joined:mapping && ThreadMembership.exists?(thread_id:mapping.channel_thread_id,user_id:811),fetches:ActiveJob::Base.queue_adapter.enqueued_jobs.count{|j|j[:job]==Github::FetchPullRequestJob}}
end
File.write("/work/vectors/github_discussions_http.json",JSON.pretty_generate(vectors)+"\n");puts "GitHub discuss Rails oracle: #{vectors.size} HTTP/redirect/thread/membership/job cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
