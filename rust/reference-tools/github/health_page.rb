require "json"
require "digest"
require "rack/mock"
require "cgi"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each {|p,h|raise "Reference drift: #{p}" unless Digest::SHA256.file(Rails.root.join(p)).hexdigest==h}
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false;load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026,1,1,12)
Account.create!(name:"Health oracle")
user=User.create!(id:811,name:"Oracle",role: :administrator)
bot=User.create!(id:813,name:"Machine",role: :bot)
agent=Agent.create!(id:881,user:bot,owner:user)
room=Rooms::Closed.create!(id:815,creator:user,name:"Cards")
room.update_columns(inbound_email_token:"fixture-address")
room2=Rooms::Closed.create!(id:825,creator:user,name:"Other")
room2.update_columns(inbound_email_token:"fixture-deleted",deleted_at:Time.current)
GithubConnectedAccount.create!(user:,github_login:"oracle",access_token:"fixture-secret",refresh_token:"fixture-refresh",token_source:"app",disconnected_reason:"revoked <account>",last_error:"read &failed")
GoogleAccount.create!(user:,email:"oracle@example.test",disconnected_reason:"expired <grant>")
Calendar::PushChannel.create!(user:,channel_id:"fixture-channel",token_digest:"fixture-digest",expires_at:24.hours.from_now,last_error:"watch <error>")
AgentEvent.create!(agent:,event_type:"github_action_completed",webhook_status:"pending",webhook_last_error:"delivery <error>")
AgentEvent.create!(agent:,event_type:"github_action_completed",webhook_status:"failed",created_at:24.hours.ago)
AgentEvent.create!(agent:,event_type:"github_action_completed",webhook_status:"failed",created_at:24.hours.ago-1.second)
configs=[{}, {"GITHUB_TOKEN"=>"configured-fixture","GITHUB_APP_CLIENT_ID"=>"configured-fixture","GITHUB_APP_CLIENT_SECRET"=>"configured-fixture","GITHUB_WEBHOOK_SECRET"=>"configured-fixture","GOOGLE_CLIENT_ID"=>"configured-fixture","GOOGLE_CLIENT_SECRET"=>"configured-fixture","GOOGLE_CALENDAR_WEBHOOK_URL"=>"https://example.test/callback","INBOUND_EMAIL_DOMAIN"=>"example.test"}]
keys=configs.flat_map(&:keys).uniq
Current.user=user
vectors=configs.map do |config|
 keys.each{|k|ENV.delete(k)};config.each{|k,v|ENV[k]=v}
 c=Accounts::IntegrationsHealthController.new
 snapshots={github:c.send(:github_snapshot),google:c.send(:google_snapshot),fizzy:Integrations::FizzyStatus.snapshot,agent_delivery:c.send(:agent_delivery_snapshot),email:c.send(:email_snapshot)}
 html=ApplicationController.render(template:"accounts/integrations_health/show",layout:false,assigns:snapshots)
 {config:,snapshot:snapshots.as_json,html:}
end
File.write("/work/vectors/github_health_page.json",JSON.pretty_generate(vectors)+"\n")
puts "Integration health Rails oracle: #{vectors.size} complete page/count/configuration cases; reference d7c7de92"
