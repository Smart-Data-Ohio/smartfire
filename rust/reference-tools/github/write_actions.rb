require "json";require "digest";require "rack/mock";require "cgi";require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each{|p,h|raise "Reference drift: #{p}" unless Digest::SHA256.file(Rails.root.join(p)).hexdigest==h}
ActiveJob::Base.queue_adapter=:test;ApplicationController.allow_forgery_protection=false
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false;load Rails.root.join("db/schema.rb");travel_to Time.utc(2026,1,1,12)
Account.create!(name:"Write actions oracle");user=User.create!(id:811,name:"Oracle",role: :administrator)
room=Rooms::Closed.create!(id:815,creator:user,name:"Cards")
message=room.messages.create!(id:818,creator:user,markdown_source:"Discussion",client_message_id:"card-parent")
thread=ChannelThread.create!(id:817,room:,creator:user,parent_message:message,name:"Discussion")
pr=Github::PullRequest.create!(id:816,owner:"rails",repo:"rails",number:12)
session=Session.create!(user:,two_factor_verified_at:Time.current)
r=ActionDispatch::Request.new(Rails.application.env_config.merge(Rack::MockRequest.env_for("http://example.org/")));cookies=ActionDispatch::Cookies::CookieJar.build(r,{});cookies.signed[:session_token]=session.token;cookie="session_token=#{CGI.escape(cookies[:session_token])}"
module WriteTransport
 def start(host,*args,**kwargs)
  raise "Unexpected host #{host}" unless host=="api.github.com"
  http=Object.new
  def http.post(path,body,headers)
   Thread.current[:requests]<<["POST",path,JSON.parse(body),headers["Authorization"].delete_prefix("Bearer ")]
   r=Net::HTTPResponse::CODE_TO_OBJ.fetch(Thread.current[:status].to_s).new("1.1",Thread.current[:status].to_s,"Fixture")
   r.define_singleton_method(:body){Thread.current[:response_body]};r
  end
  yield http
 end
end
Net::HTTP.singleton_class.prepend(WriteTransport)
module WriteRenderCapture
 def render_write_result(**kwargs)
  Thread.current[:render_locals]=kwargs
  super
 end
end
[Github::PullRequestCommentsController,Github::PullRequestReviewsController,Github::PullRequestReviewRequestsController].each{|c|c.prepend(WriteRenderCapture)}
cases=[]
%w[comments reviews review_requests].each do |action|
 payload=case action;when "comments" then {body:" Nice work "};when "reviews" then {event:"APPROVE"};else {reviewers:" @Alice,alice @BOB,bob "};end
 [{name:"success",linked:true},{name:"stream_success",linked:true,stream:true},{name:"unlinked"},{name:"disconnected",linked:true,initially_disconnected:true},{name:"unauthorized",linked:true,api_status:401},{name:"refused",linked:true,api_status:403,stream:true},{name:"invalid",linked:true,api_status:422},{name:"network",linked:true,api_status:500},{name:"nonmember",member:false,linked:true},{name:"unmapped",mapping:false,linked:true}].each{|c|cases<<{**c,name:"#{action}_#{c[:name]}",action:,submitted:payload}}
end
cases += [{name:"comment_blank",action:"comments",linked:true,submitted:{body:"  "}},{name:"review_note",action:"reviews",linked:true,submitted:{event:"REQUEST_CHANGES",body:" Fix the typo "}},{name:"review_blank",action:"reviews",linked:true,stream:true,submitted:{event:"REQUEST_CHANGES",body:" "}},{name:"review_event",action:"reviews",linked:true,submitted:{event:"COMMENT"}},{name:"review_failed_note",action:"reviews",linked:true,api_status:422,submitted:{event:"REQUEST_CHANGES",body:"Fix the typo"}},{name:"reviewers_invalid",action:"review_requests",linked:true,submitted:{reviewers:"alice, bob!!"}},{name:"reviewers_empty",action:"review_requests",linked:true,submitted:{reviewers:" "}},{name:"reviewers_many",action:"review_requests",linked:true,submitted:{reviewers:(1..16).map{|i|"user#{i}"}.join(", ")}}]
[{name:"show_linked",linked:true},{name:"show_unlinked"},{name:"show_disconnected",linked:true,initially_disconnected:true},{name:"show_nonmember",member:false},{name:"show_unmapped",mapping:false}].each{|c|cases<<{**c,action:"show",submitted:{}}}
vectors=cases.map do |c|
 GithubConnectedAccount.delete_all;Github::PullRequestThread.delete_all;Membership.where(room:,user:).delete_all
 Membership.create!(room:,user:) unless c[:member]==false
 Github::PullRequestThread.create!(pull_request:pr,room:,channel_thread:thread) unless c[:mapping]==false
 GithubConnectedAccount.create!(user:,github_login:"oracle",access_token:"fixture-viewer-token",disconnected_reason:c[:initially_disconnected] ? "Disconnected" : nil) if c[:linked]
 Thread.current[:render_locals]={};Thread.current[:requests]=[];Thread.current[:status]=c.fetch(:api_status,201);Thread.current[:response_body]=JSON.generate(message:"Denied <review> @[Member]\nTry again",id:12)
 body={pull_request_id:816,**c[:submitted]};path=c[:action]=="show" ? "/rooms/815/github/pull_request_write_actions/816" : "/rooms/815/github/pull_request_#{c[:action]}"
 # Controllers are exercised here; the body has IntegrationTest's token-free form rendering.
 res=Rack::MockRequest.new(Rails.application).request(c[:action]=="show" ? "GET" : "POST","http://example.org"+path,"HTTP_COOKIE"=>cookie,"CONTENT_TYPE"=>"application/json","HTTP_ACCEPT"=>c[:stream] ? "text/vnd.turbo-stream.html" : "text/html",input:JSON.generate(body))
 a=user.reload.github_connected_account
 render_data={thread_id:817,room_id:815,pull_request_id:816,linked:!a.nil?,usable:a&.usable? || false,login:a&.github_login || "",notice:nil,alert:nil,comment_body:nil,review_body:nil,reviewers_body:nil}.merge(Thread.current[:render_locals].except(:status))
 {**c,render_data:,request_body:body,path:,status:res.status,content_type:res["Content-Type"],body:res.status==404 ? nil : res.body,requests:Thread.current[:requests],disconnected:a&.disconnected_reason}
end
File.write("/work/vectors/github_write_http.json",JSON.pretty_generate(vectors)+"\n");puts "GitHub write actions Rails oracle: #{vectors.size} controller/frame/status/request cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
inputs=[nil,"", " @Alice, alice @BOB,bob ","-a","a-","a--b","a_b","user!!","a"*39,"a"*40,(1..15).map{|i|"user#{i}"},(1..16).map{|i|"user#{i}"},["alice, bob","@Alice"],false,12,"ſame","Kate","alice\u00a0bob"]
File.write("/work/vectors/github_review_logins.json",JSON.pretty_generate(inputs.map{|input|{input:,expected:Github::ReviewLogins.normalize(input)}})+"\n");puts "GitHub review logins Rails oracle: #{inputs.size} normalization cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
