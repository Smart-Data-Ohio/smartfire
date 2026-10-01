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
Account.create!(name:"Health oracle")
13.times do |i|
 user=User.create!(id:900+i,name:"User #{i}",email_address:"user#{i}@example.test",password:"fixture-password")
 account=GithubConnectedAccount.create!(id:1000+i,user:,github_login:"login#{i}",access_token:"fixture-secret-#{i}",token_source:i.even? ? "app" : "pat",disconnected_reason:i==0 ? nil : i==1 ? "" : "reason <#{i}>",last_error:i==0 ? nil : i==1 ? "" : "error &#{i}")
 account.update_columns(updated_at:Time.current+i.seconds)
 Github::PullRequest.create!(id:1100+i,owner:"owner#{i}",repo:"repo",number:i+1,fetch_error:i==0 ? nil : i==1 ? "" : "fetch <#{i}>").update_columns(updated_at:Time.current+i.seconds)
end
[-24.hours-Rational(1,1_000_000),-24.hours,0].each_with_index { |age,i| Github::WebhookDelivery.create!(delivery_guid:"health-#{i}",event:"ping",created_at:Time.current+age) }
configs=[{}, {"GITHUB_TOKEN"=>"fixture-workspace-secret"}, {"GITHUB_APP_CLIENT_ID"=>"fixture-id"},
 {"GITHUB_APP_CLIENT_ID"=>"fixture-id","GITHUB_APP_CLIENT_SECRET"=>"fixture-app-secret"},
 {"GITHUB_WEBHOOK_SECRET"=>"fixture-webhook-secret"}, {"GITHUB_TOKEN"=>" \t","GITHUB_APP_CLIENT_ID"=>"fixture-id","GITHUB_APP_CLIENT_SECRET"=>"\u00a0","GITHUB_WEBHOOK_SECRET"=>"\u2003"},
 {"GITHUB_TOKEN"=>"fixture-workspace-secret","GITHUB_APP_CLIENT_ID"=>"fixture-id","GITHUB_APP_CLIENT_SECRET"=>"fixture-app-secret","GITHUB_WEBHOOK_SECRET"=>"fixture-webhook-secret"}]
Current.user=User.find(900)
vectors=configs.map do |config|
 %w[GITHUB_TOKEN GITHUB_APP_CLIENT_ID GITHUB_APP_CLIENT_SECRET GITHUB_WEBHOOK_SECRET].each { |key| ENV.delete(key) }
 config.each { |k,v| ENV[k]=v }
 snapshot=Accounts::IntegrationsHealthController.new.send(:github_snapshot)
 html=ApplicationController.render(template:"accounts/integrations_health/show",layout:false,assigns:{github:snapshot,google:{configured:false,connected:0,disconnected:[],entry_errors:[],push:{enabled:false,count:0,expiring:[]}},fizzy:{configured:false,note:"disabled"},agent_delivery:{pending:0,failed:0,recent_errors:[]},email:{enabled:false,rooms_with_addresses:0}})
 section=html[html.index('  <section aria-labelledby="health-github">')..].split("  </section>",2).first+"  </section>\n"
 raise "Health leaks a token" if section.include?("fixture-")
 {config:config.transform_values { |v| v.start_with?("fixture") ? "configured-fixture" : v },snapshot:,html:section}
end
File.write("/work/vectors/github_health.json",JSON.pretty_generate(vectors)+"\n")
puts "GitHub health Rails oracle: #{vectors.size} configuration/count/list/HTML cases; reference d7c7de92"
