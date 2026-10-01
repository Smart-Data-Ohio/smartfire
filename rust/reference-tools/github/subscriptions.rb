require "json"
require "digest"
require "cgi"
require "rack/mock"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path,hash|
 raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash
end
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
Rails.cache=ActiveSupport::Cache::MemoryStore.new
# Match ActionDispatch::IntegrationTest's forgery setting. Rust requests carry real CSRF tokens.
ApplicationController.allow_forgery_protection=false
ActiveRecord::Schema.verbose=false
load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026,1,1,12)
Account.create!(name:"Subscription oracle")
owner=User.create!(id:811,name:"Oracle",email_address:"oracle@example.test",password:"fixture-password",role: :administrator)
other_user=User.create!(id:812,name:"Other",email_address:"other@example.test",password:"fixture-password")
room=Rooms::Closed.create!(id:815,creator:owner,name:"Cards")
other=Rooms::Closed.create!(id:825,creator:owner,name:"Other")
session=Session.create!(user:owner,two_factor_verified_at:Time.current)
cookie_request=ActionDispatch::Request.new(Rails.application.env_config.merge(Rack::MockRequest.env_for("http://example.org/")))
cookies=ActionDispatch::Cookies::CookieJar.build(cookie_request,{})
cookies.signed[:session_token]=session.token
cookie="session_token=#{CGI.escape(cookies[:session_token])}"
module SubscriptionAccessOracle
 def repository_readable?(owner,repo)
  Thread.current[:subscription_requests]<<[owner,repo,@token]
  case Thread.current[:subscription_status]
  when 401 then raise Github::WriteClient::Unauthorized,"GitHub rejected the linked token"
  when 404 then false
  when 500 then raise Github::WriteClient::Error,"GitHub refused the request"
  else true
  end
 end
end
Github::WriteClient.prepend(SubscriptionAccessOracle)
module SubscriptionFlashOracle
 def redirect_to(*args,**kwargs)
  super.tap {Thread.current[:subscription_flash]=flash.to_hash}
 end
end
Rooms::GithubSubscriptionsController.prepend(SubscriptionFlashOracle)
params={github_repository_subscription:{full_name:"Rails/Rails"}}
cases=[
 {name:"default",linked:true},
 {name:"explicit",linked:true,body:{github_repository_subscription:{full_name:" Rails / Rails ",events:["opened","merged","","merged"]}}},
 {name:"open",linked:true,kind:"Rooms::Open"},
 {name:"creator",linked:true,role:0},
 {name:"duplicate",linked:true,preset:["rails/rails"]},
 {name:"malformed",linked:true,body:{github_repository_subscription:{full_name:"not-a-repo"}}},
 {name:"unknown_events",linked:true,body:{github_repository_subscription:{full_name:"rails/rails",events:["evil"]}}},
 {name:"empty_events_default",linked:true,body:{github_repository_subscription:{full_name:"rails/rails",events:[""]}}},
 {name:"update",linked:true,preset:["rails/rails"],method:"PATCH",body:{github_repository_subscription:{events:["merged",""]}}},
 {name:"update_empty",preset:["rails/rails"],method:"PATCH",body:{github_repository_subscription:{events:[""]}}},
 {name:"update_nil",preset:["rails/rails"],method:"PATCH",body:{github_repository_subscription:{}}},
 {name:"destroy_last",preset:["rails/rails"],method:"DELETE"},
 {name:"destroy_one",preset:["rails/rails","rails/propshaft"],method:"DELETE"},
 {name:"denied",linked:true,access:404,role:0,body:{github_repository_subscription:{full_name:"rails/rails",skip_access_check:"1"}}},
 {name:"unlinked"},
 {name:"disconnected",linked:true,initially_disconnected:true},
 {name:"override_denied",linked:true,access:404,body:{github_repository_subscription:{full_name:"rails/rails",skip_access_check:"1"}}},
 {name:"override_unlinked",body:{github_repository_subscription:{full_name:"rails/rails",skip_access_check:"1"}}},
 {name:"override_not_string",body:{github_repository_subscription:{full_name:"rails/rails",skip_access_check:1}}},
 {name:"unauthorized",linked:true,access:401},
 {name:"transport",linked:true,access:500},
 {name:"plain",role:0,room_creator:812,linked:true},
 {name:"nonmember",member:false,linked:true},
 {name:"direct",kind:"Rooms::Direct",linked:true},
 {name:"deleted",deleted:true,linked:true},
 {name:"cross_room_update",preset:["rails/rails"],preset_room:825,method:"PATCH",body:{github_repository_subscription:{events:["merged"]}}},
 {name:"cross_room_destroy",preset:["rails/rails"],preset_room:825,method:"DELETE"},
 {name:"missing_params",linked:true,body:{}},
]
vectors=cases.map do |c|
 Rails.cache.clear;GithubConnectedAccount.delete_all;Github::Notification.delete_all;Github::RepositorySubscription.delete_all
 Membership.where(user:User.active_bots.where(name:"GitHub")).delete_all
 owner.update_columns(role:c.fetch(:role,1))
 room.update_columns(type:c.fetch(:kind,"Rooms::Closed"),creator_id:c.fetch(:room_creator,811),deleted_at:c[:deleted] ? Time.current : nil)
 Membership.find_or_create_by!(room:,user:owner)
 Membership.where(room:,user:owner).delete_all if c[:member]==false
 GithubConnectedAccount.create!(user:owner,github_login:"oracle",access_token:"fixture-viewer-token",token_source:"pat",disconnected_reason:c[:initially_disconnected] ? "Disconnected" : nil) if c[:linked]
 presets=Array(c[:preset]).map { |full|a,b=full.split("/");Github::RepositorySubscription.create!(room:Room.find(c.fetch(:preset_room,815)),owner:a,repo:b,created_by:owner,reader_verified:true)}
 Thread.current[:subscription_requests]=[];Thread.current[:subscription_status]=c.fetch(:access,200);Thread.current[:subscription_flash]={}
 method=c.fetch(:method,"POST");path="http://example.org/rooms/815/github_subscriptions#{method=='POST' ? '' : '/' + presets.first.id.to_s}"
 res=Rack::MockRequest.new(Rails.application).request(method,path,"HTTP_COOKIE"=>cookie,"CONTENT_TYPE"=>"application/json",input:JSON.generate(c.fetch(:body,params)))
 rows=Github::RepositorySubscription.order(:owner,:repo).map{|s|s.attributes.slice("room_id","owner","repo","events","created_by_id","reader_verified")}
 {**c,request_body:c.fetch(:body,params),status:res.status,location:res['Location'],content_type:res['Content-Type'],body:res.status>=400 ? nil : res.body,flash:Thread.current[:subscription_flash],rows:,requests:Thread.current[:subscription_requests],bot_member:Membership.where(room_id:815,user:User.active_bots.where(name:"GitHub")).exists?,disconnected:GithubConnectedAccount.first&.disconnected_reason}
end
File.write("/work/vectors/github_subscription_http.json",JSON.pretty_generate(vectors)+"\n")
puts "GitHub subscriptions Rails oracle: #{vectors.size} HTTP/status/flash/persistence cases; reference d7c7de92"
sections=[{name:"admin_empty"},{name:"admin_subscribed",subscribed:true},{name:"creator_subscribed",subscribed:true,role:0},{name:"plain",subscribed:true,role:0,creator:812},{name:"direct",kind:"Rooms::Direct"}].map do |c|
 Github::Notification.delete_all;Github::RepositorySubscription.delete_all
 owner.update_columns(role:c.fetch(:role,1));owner.reload
 room.update_columns(type:c.fetch(:kind,"Rooms::Closed"),creator_id:c.fetch(:creator,811))
 r=Room.find(815)
 Github::RepositorySubscription.create!(id:881,room:r,owner:"rails",repo:"rails",events:["merged"],created_by:owner) if c[:subscribed]
 Current.user=owner
 subscriptions=r.github_repository_subscriptions.order(:owner,:repo).map{|s|s.attributes.slice("id","owner","repo","events")}
 {**c,subscriptions:,html:ApplicationController.render(partial:"rooms/github_subscriptions/section",locals:{room:r})}
end
File.write("/work/vectors/github_subscription_sections.json",JSON.pretty_generate(sections)+"\n")
puts "GitHub subscription sections Rails oracle: #{sections.size} role/room/form HTML cases; reference d7c7de92"
