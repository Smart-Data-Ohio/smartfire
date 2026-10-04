require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
travel_to Time.utc(2026,3,2,16) do
  cases={}
  [false,true].each do |reject|
    user=User.create!(name:"WS11 removal",email_address:"ws11-removal-#{reject}@example.test",password:"ws11 public fixture password")
    GithubConnectedAccount.create!(user:user,github_login:"ws11-removal",access_token:"ws11 public fixture")
    FizzyConnectedAccount.create!(user:user,fizzy_account_id:"ws11-public",access_token:"ws11 public fixture")
    if reject
      User.connection.execute("CREATE TRIGGER ws11_reject_fizzy_removal BEFORE DELETE ON fizzy_connected_accounts BEGIN SELECT RAISE(ABORT,'WS11 rejected peer deletion'); END")
    end
    error=nil
    begin user.destroy!;rescue ActiveRecord::StatementInvalid;error="peer deletion failed";end
    cases[reject ? :failure : :success]={error:error,user:User.exists?(user.id),github:GithubConnectedAccount.exists?(user_id:user.id),fizzy:FizzyConnectedAccount.exists?(user_id:user.id)}
    User.connection.execute("DROP TRIGGER ws11_reject_fizzy_removal") if reject
  end
  puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases)
end
