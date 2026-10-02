# Human HTTP approval through the real execution job. Only the remote client is a fake.
require 'action_dispatch/testing/integration'
ApplicationController.allow_forgery_protection = false
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
owner = User.find(labels.fetch('users.david'))
bot = User.find(labels.fetch('users.bender'))
agent = bot.agent
room = Room.find(labels.fetch('rooms.watercooler'))
Current.user = owner
agent.update!(owner:, suspended_at: nil, daily_external_action_cap: nil)
bot.update!(status: :active)
Membership.find_or_create_by!(room:, user: bot)
agent.agent_grants.where(capability: 'external_action').delete_all
credential, = AgentCredential.create_with_secret!(agent:, name: 'Execution bridge', created_by: owner)
GithubConnectedAccount.where(user: bot).destroy_all
account = GithubConnectedAccount.create!(user: bot, github_login: 'machine', access_token: 'fixture-agent-token')
pr = Github::PullRequest.create!(owner: 'rails', repo: 'rails', number: 999)
thread = ChannelThread.create!(room:, creator: owner, name: 'Execution bridge')
Github::PullRequestThread.create!(pull_request: pr, room:, channel_thread: thread)
# Preserve the real job's account/claim/outcome logic, faking only its external client.
requests = []
client = Object.new
client.define_singleton_method(:create_issue_comment) do |pull_request, body:|
  requests << { path: "/repos/#{pull_request.full_name}/issues/#{pull_request.number}/comments", body: }
  { 'html_url' => 'https://github.com/rails/rails/pull/999#fixture' }
end
Github::WriteClient.define_singleton_method(:new) do |token:|
  raise "wrong execution token #{token.inspect}" unless token == 'fixture-agent-token'
  client
end
expected = {
  'success' => ['completed', nil, 1],
  'identity_changed' => ['failed', "The agent's GitHub account changed since this was approved", 0],
  'grant_revoked' => ['failed', 'Agent no longer has the external_action capability', 0],
  'credential_revoked' => ['completed', nil, 1],
  'suspended' => ['failed', 'Agent is suspended or deactivated', 0],
  'deactivated' => ['failed', 'Agent is suspended or deactivated', 0]
}
expected.each do |name, (status, message, count)|
  agent.update!(suspended_at: nil)
  bot.update!(status: :active)
  account.update!(github_login: 'machine')
  grant = AgentGrant.create!(agent:, room:, capability: 'external_action', granted_by: owner)
  action = Github::AgentPullRequestAction.new(pull_request: pr, kind: 'comment', body: 'Nice work')
  approval = AgentApproval.create!(agent:, room:, agent_credential: credential, action: action.action_name, summary: action.summary, payload: action.payload_json, github_account_id: account.id, github_login: account.github_login)
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! 'campfire.test'
  browser.patch("/agent_approvals/#{approval.id}.json", params: { decision: 'approved' }, headers: { 'Cookie' => "session_token=#{labels.fetch('session_cookies.david')}", 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0' })
  raise "#{name}: approval HTTP #{browser.response.status}" unless browser.response.status == 200 && JSON.parse(browser.response.body)['status'] == 'approved'
  case name
  when 'identity_changed' then account.update!(github_login: 'changed')
  when 'grant_revoked' then grant.revoke!
  when 'credential_revoked' then credential.revoke!
  when 'suspended' then agent.update!(suspended_at: Time.current)
  when 'deactivated' then bot.update!(status: :deactivated)
  end
  requests.clear
  # Integration sessions clear the runner executor context; run the job in its own executor.
  2.times { Rails.application.executor.wrap { Github::PerformAgentActionJob.perform_now(approval.id) } }
  events = agent.agent_events.where(agent_approval_id: approval.id, event_type: 'github_action_completed')
  raise "#{name}: completion count #{events.count}" unless events.count == 1
  metadata = events.first.metadata
  raise "#{name}: wrong outcome #{metadata.inspect}" unless metadata['status'] == status && metadata['message'] == message
  raise "#{name}: wrong transport count #{requests.size}" unless requests.size == count
  raise "#{name}: wrong transport body" if count == 1 && requests.first != { path: '/repos/rails/rails/issues/999/comments', body: 'Nice work' }
  grant.revoke! unless grant.revoked?
end
puts 'Rails human approval execution bridge: 6 HTTP decisions; 6 real job outcomes; 2 remote calls; 6 duplicate executions are idempotent; 0 failures'

agent.update!(suspended_at: nil)
bot.update!(status: :active)
FizzyConnectedAccount.where(user: owner).destroy_all
fizzy = FizzyConnectedAccount.create!(user: owner, fizzy_account_id: '12345', fizzy_user_id: 'fixture-user', fizzy_user_name: 'Fixture User', access_token: 'fixture-owner-token')
fizzy_requests = []
fizzy_client = Object.new
fizzy_client.define_singleton_method(:create_comment) do |account_id, number, body:|
  fizzy_requests << { path: "/#{account_id}/cards/#{number}/comments.json", body: }
  { 'url' => 'https://app.fizzy.do/created' }
end
Fizzy::Client.define_singleton_method(:new) do |token:|
  raise 'wrong Fizzy execution token' unless token == 'fixture-owner-token'
  fizzy_client
end
expected.each do |name, (status, message, count)|
  agent.update!(suspended_at: nil)
  bot.update!(status: :active)
  fizzy.update!(fizzy_user_id: 'fixture-user')
  grant = AgentGrant.create!(agent:, capability: 'external_action', granted_by: owner)
  action = Fizzy::AgentCardAction.new(account_id: '12345', kind: 'comment', number: 579, body: 'Nice work')
  approval = AgentApproval.create!(agent:, agent_credential: credential, action: action.action_name, summary: action.summary, payload: action.payload_json, fizzy_connected_account_id: fizzy.id, fizzy_user_id: fizzy.fizzy_user_id)
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! 'campfire.test'
  browser.patch("/agent_approvals/#{approval.id}.json", params: { decision: 'approved' }, headers: { 'Cookie' => "session_token=#{labels.fetch('session_cookies.david')}", 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0' })
  raise "Fizzy #{name}: approval HTTP #{browser.response.status}" unless browser.response.status == 200 && JSON.parse(browser.response.body)['status'] == 'approved'
  case name
  when 'identity_changed' then fizzy.update!(fizzy_user_id: 'changed')
  when 'grant_revoked' then grant.revoke!
  when 'credential_revoked' then credential.revoke!
  when 'suspended' then agent.update!(suspended_at: Time.current)
  when 'deactivated' then bot.update!(status: :deactivated)
  end
  fizzy_requests.clear
  2.times { Rails.application.executor.wrap { Fizzy::PerformAgentActionJob.perform_now(approval.id) } }
  events = agent.agent_events.where(agent_approval_id: approval.id, event_type: 'fizzy_action_completed')
  raise "Fizzy #{name}: completion count #{events.count}" unless events.count == 1
  metadata = events.first.metadata
  message = "The agent owner's Fizzy account changed since this was approved" if name == 'identity_changed'
  raise "Fizzy #{name}: wrong outcome #{metadata.inspect}" unless metadata['status'] == status && metadata['message'] == message
  raise "Fizzy #{name}: wrong transport count #{fizzy_requests.size}" unless fizzy_requests.size == count
  raise "Fizzy #{name}: wrong transport body" if count == 1 && fizzy_requests.first != { path: '/12345/cards/579/comments.json', body: 'Nice work' }
  grant.revoke! unless grant.revoked?
end
puts 'Rails human Fizzy approval execution bridge: 6 HTTP decisions; 6 real job outcomes; 2 remote calls; 6 duplicate executions are idempotent; 0 failures'
