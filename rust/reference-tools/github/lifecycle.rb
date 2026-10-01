require "json";require "digest";require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each{|p,h|raise "Reference drift: #{p}" unless Digest::SHA256.file(Rails.root.join(p)).hexdigest==h}
ActiveJob::Base.queue_adapter=:test;ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false;load Rails.root.join("db/schema.rb");travel_to Time.utc(2026,1,1,12)
Account.create!(name:"Lifecycle oracle")
cases=[{name:"human"},{name:"bot",bot:true},{name:"already_disconnected",initial_reason:"Other reason"},{name:"same_reason",initial_reason:"Account deactivated"},{name:"invalid_account",invalid:true},{name:"late_failure",reject_user:true}]
vectors=cases.each_with_index.map do |c,i|
 user=User.create!(name:"Lifecycle #{i}",role:c[:bot] ? :bot : :member)
 account=GithubConnectedAccount.create!(user:,github_login:"oracle",access_token:"fixture-retained",refresh_token:"fixture-refresh-retained",token_source:"app",last_error:"old error",disconnected_reason:c[:initial_reason])
 account.update_columns(updated_at:Time.utc(2025,1,1,12),token_source:c[:invalid] ? "invalid" : "app")
 if c[:reject_user] then ActiveRecord::Base.connection.execute("CREATE TRIGGER reject_lifecycle_user BEFORE UPDATE ON users WHEN NEW.id=#{user.id} AND NEW.status=1 BEGIN SELECT RAISE(ABORT,'user rejected'); END") end
 error=nil
 begin user.deactivate;rescue => e;error=e.class.name;end
 ActiveRecord::Base.connection.execute("DROP TRIGGER IF EXISTS reject_lifecycle_user")
 a=account.reload
 {**c,error:,status:user.reload.status,reason:a.disconnected_reason,last_error:a.last_error,updated_at:a.updated_at.iso8601,access:a.access_token,refresh:a.refresh_token}
end
File.write("/work/vectors/github_lifecycle.json",JSON.pretty_generate(vectors)+"\n");puts "GitHub lifecycle Rails oracle: #{vectors.size} deactivation/validation/rollback cases; reference d7c7de92"
