# Read the columns written through Rust's account model back through Rails' model.
require "json"
require "digest"
expected = JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES")))
expected.each { |path, hash| raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash }
output = JSON.parse(File.read(ENV.fetch("GITHUB_RUST_OUTPUT")))
ActiveJob::Base.queue_adapter = :test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose = false
load Rails.root.join("db/schema.rb")
Account.create!(name: "Rust rollback")
user = User.create!(name: "Rust writer", email_address: "rollback@example.test", password: "fixture-password", role: :administrator)
row = output.fetch("row").merge("user_id" => user.id)
conn = ActiveRecord::Base.connection
columns = row.keys.map { |key| conn.quote_column_name(key) }.join(", ")
values = row.values.map { |value| conn.quote(value) }.join(", ")
conn.execute("INSERT INTO github_connected_accounts (#{columns}) VALUES (#{values})")
account = GithubConnectedAccount.find_by!(user_id: user.id)
checks = 0
check = lambda do |actual, expected|
  raise "Rollback check failed" unless actual == expected
  checks += 1
end
check.call(account.valid?, true)
check.call(account.access_token, output.fetch("plaintext").fetch("access_token"))
check.call(account.refresh_token, output.fetch("plaintext").fetch("refresh_token"))
check.call(account.connected?, true)
check.call(account.app_token?, true)
check.call(Rails.application.message_verifier("github_app_oauth_state").verified(output.fetch("signed_state")), "fixture-session-state")
puts "GitHub Rails rollback: #{checks} checks passed; 0 failed; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
