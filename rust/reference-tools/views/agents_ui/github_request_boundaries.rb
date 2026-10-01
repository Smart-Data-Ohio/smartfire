# The pinned HTTP create commits its approval before after_create_commit fan-out.
require 'action_dispatch/testing/integration'
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
ApplicationController.allow_forgery_protection = false
agent = Agent.find(labels.fetch('agents.bender'))
owner = User.find(labels.fetch('users.david'))
room = Room.find(labels.fetch('rooms.watercooler'))
Current.user = owner
agent.update!(daily_external_action_cap: nil)
AgentGrant.create!(agent:, room:, capability: 'external_action', granted_by: owner)
credential, token = AgentCredential.create_with_secret!(agent:, name: 'Boundary', created_by: owner)
GithubConnectedAccount.where(user: agent.user).destroy_all
GithubConnectedAccount.create!(user: agent.user, github_login: 'machine', access_token: 'fixture-boundary-token')
pr = Github::PullRequest.create!(owner: 'rails', repo: 'rails', number: 999)
thread = ChannelThread.create!(room:, creator: owner, name: 'Boundary')
Github::PullRequestThread.create!(pull_request: pr, room:, channel_thread: thread)
ActiveRecord::Base.connection.execute("CREATE TRIGGER reject_request_inbox BEFORE INSERT ON activity_items WHEN NEW.source_type='AgentApproval' BEGIN SELECT RAISE(ABORT,'fixture request inbox rejection'); END;")
request = { pull_request_id: pr.id, kind: 'comment', body: 'First', external_id: 'boundary-after-commit' }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers = { 'Authorization' => "Bearer #{token}", 'Content-Type' => 'application/json', 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0' }
path = "/rooms/#{room.id}/agents/github/pull_request_actions"
begin
  browser.post(path, params: JSON.generate(request), headers:)
  raise "expected 500, got #{browser.response.status}" unless browser.response.status == 500
rescue ActiveRecord::StatementInvalid => error
  raise unless error.message.include?('fixture request inbox rejection')
end
approval = AgentApproval.find_by!(agent:, external_id: request[:external_id])
raise 'approval did not survive fan-out error' unless approval.status == 'pending'
raise 'unexpected inbox row' if ActivityItem.exists?(source: approval)
ActiveRecord::Base.connection.execute('DROP TRIGGER reject_request_inbox')
browser.post(path, params: JSON.generate(request), headers:)
raise 'replay differs' unless browser.response.status == 200 && JSON.parse(browser.response.body)['id'] == approval.id
raise 'replay retroactively created an inbox row' if ActivityItem.exists?(source: approval)
puts 'Rails GitHub request fault boundaries: 1 committed pending approval after rejected inbox insert; replay 200, same id, no retroactive inbox; 2 requests passed'
