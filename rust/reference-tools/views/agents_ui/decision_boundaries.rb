# Execute the pinned controller through Rails' HTTP stack, including failure
# boundaries. These are real mutations on a private default-seed copy.
require "action_dispatch/testing/integration"
labels = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "parity/.seed/default/labels.json")))
ActionController::Base.allow_forgery_protection = false
agent = Agent.find(labels.fetch("agents.bender"))
david = User.find(labels.fetch("users.david"))
def request(labels, approval, decision, format)
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! "campfire.test"
  browser.patch("/agent_approvals/#{approval.id}#{format}", params: { decision: }, headers: {
    "Cookie" => "session_token=#{labels.fetch('session_cookies.david')}",
    "HTTP_USER_AGENT" => "Mozilla/5.0 Chrome/140.0.0.0"
  })
  browser.response
ensure
  ActiveSupport::ExecutionContext.clear
end
%w[denied invalid].each do |decision|
  Current.user = david
  approval = AgentApproval.create!(agent:, action: "deploy", summary: "HTML fallback boundary")
  response = request(labels, approval, decision, "")
  raise "HTML fallback differs: #{response.status} #{response.headers['Location']}" unless response.status == 303 && response.headers['Location'] == 'http://campfire.test/activity'
end
ActiveRecord::Base.connection.execute("CREATE TRIGGER reject_decision_audit BEFORE INSERT ON audit_logs WHEN NEW.action='agent.approval.decide' BEGIN SELECT RAISE(ABORT,'fixture decision audit rejection'); END;")
%w[approved denied].each do |decision|
  Current.user = david
  approval = AgentApproval.create!(agent:, action: "deploy", summary: "Audit failure boundary")
  begin
    response = request(labels, approval, decision, ".json")
    raise "expected HTTP 500" unless response.status == 500
  rescue ActiveRecord::StatementInvalid => error
    raise unless error.message.include?('fixture decision audit rejection')
  end
  raise 'decision was rolled back' unless approval.reload.status == decision
  raise 'ledger was rolled back' unless AgentEvent.where(event_type: 'approval_decided').count { |event| event.metadata['approval_id'] == approval.id } == 1
  raise 'inbox handling was rolled back' if ActivityItem.where(source: approval, handled_at: nil).exists?
  raise 'an audit was unexpectedly stored' if AuditLog.where(action: 'agent.approval.decide', target_id: approval.id).exists?
end
puts 'Rails human decision boundaries: 2 HTML fallbacks; 2 committed decisions after rejected audits; 4 actions passed'
