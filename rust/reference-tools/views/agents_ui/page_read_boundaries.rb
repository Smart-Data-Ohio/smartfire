# Pinned Rails controller responses, including the lazy expiry loop's current clock.
require 'action_dispatch/testing/integration'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ApplicationController.allow_forgery_protection = false
ActiveRecord::Base.logger = nil
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
admin = User.find(127326141)
browser = lambda do
  session = ActionDispatch::Integration::Session.new(Rails.application)
  session.host! 'campfire.test'
  session
end
headers = {'Cookie' => "session_token=#{labels.fetch('session_cookies.david')}", 'Accept' => 'text/html', 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0'}
tour_fragment = lambda { |body| body[/<div id="tour" hidden.*?^<\/div>/m] }
tours = [nil, '2026-01-01 00:00:00', '', 'not-a-date', '0', 0, '2026-02-31 12:00:00', '2026-01-01'].map do |value|
  ActiveRecord::Base.connection.execute("UPDATE users SET tour_completed_at=#{ActiveRecord::Base.connection.quote(value)} WHERE id=#{admin.id}")
  session = browser.call
  session.get('/agents', headers:)
  body = tour_fragment.call(session.response.body)
  raise 'missing tour fragment' unless body
  result = {value:, status: session.response.status, body:}
  ActiveSupport::ExecutionContext.clear
  result
end
agent = User.find(394959859).agent
AgentApproval.where(agent:).delete_all
[['review199 overdue', -10], ['review199 crosses deadline', 1]].each do |summary, delta|
  approval = AgentApproval.create!(agent:, action: 'deploy', summary:, expires_at: Time.current + 3600)
  approval.update_columns(expires_at: Time.current + delta)
end
initial = Time.current
first = true
travel = method(:travel_to)
AgentApproval.prepend(Module.new do
  define_method(:expire_if_due!) do
    if first
      first = false
      travel.call(initial + 2)
    end
    super()
  end
end)
session = browser.call
session.get("/agents/#{agent.id}/approvals", headers:)
rows = AgentApproval.where(agent:).order(:id).map do |approval|
  [approval.summary, approval.status, ActivityItem.where(source: approval, handled_at: nil).count]
end
menu = session.response.body[/<menu class="flex flex-column gap margin-none pad txt-align-start">.*?<\/menu>/m]
raise 'missing approval menu' unless menu
expiry = {status: session.response.status, rows:, body: menu}
puts JSON.pretty_generate(reference: 'd7c7de92', tours:, expiry:)
warn "Rails page-read boundaries: #{tours.size} tour values; #{rows.size} approvals; #{rows.count { |row| row[1] == 'expired' }} expired; #{rows.sum { |row| row[2] }} unhandled items"
travel_back
