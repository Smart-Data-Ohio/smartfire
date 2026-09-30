# Optional UI corpus. Keep default's legacy-capability assumptions intact while
# giving agent administration/history/step captures real Rails-created states.
based_on "default"
Current.reset
david, kevin, bot = %i[david kevin deploy_bot].map { |name| user(name) }
channel, board = room(:designers), room(:board)

at NOW - 3.hours
agent = label :agents, :ui, bot.create_agent!(kind: :workspace, owner: kevin,
  provider: "Fixture provider", runtime: "Rust worker", description: "Reviews <changes> & reports results.",
  daily_message_cap: 50, daily_board_post_cap: 10, daily_external_action_cap: 5)
agent.update!(webhook_signing_secret: "fixture-ui-signing-secret")

# The displayed identifier is the first four digest characters, never a substring
# of a secret. Both values describe the same deterministic synthetic credential.
credential, = AgentCredential.create_with_secret!(agent:, name: "UI runner", created_by: david, expires_at: NOW + 1.day)
digest = AgentCredential.digest("fixture-ui-runner-secret")
credential.update!(token_digest: digest, token_last_four: digest[0, 4], last_used_at: NOW - 5.minutes)
label :agent_credentials, :ui, credential
label :agent_tokens, :ui, "fixture-ui-runner-secret"
at NOW - 2.hours
expired, = AgentCredential.create_with_secret!(agent:, name: "Expired <runner>", created_by: david, expires_at: NOW - 1.hour)
digest = AgentCredential.digest("fixture-ui-expired-secret")
expired.update!(token_digest: digest, token_last_four: digest[0, 4])
label :agent_credentials, :ui_expired, expired
at NOW - 1.hour
revoked, = AgentCredential.create_with_secret!(agent:, name: "Revoked & runner", created_by: david)
digest = AgentCredential.digest("fixture-ui-revoked-secret")
revoked.update!(token_digest: digest, token_last_four: digest[0, 4])
revoked.revoke!
label :agent_credentials, :ui_revoked, revoked

at NOW - 30.minutes
board.memberships.grant_to [bot]
%w[read_messages post_messages].each do |capability|
  label :agent_grants, "ui_#{capability}", AgentGrant.create!(agent:, room: channel, capability:, granted_by: david)
end
label :agent_grants, :ui_threads, AgentGrant.create!(agent:, room: board, capability: "manage_threads", granted_by: david)
label :agent_grants, :ui_board_posts, AgentGrant.create!(agent:, room: board, capability: "post_messages", granted_by: david)
revoked_grant = label :agent_grants, :ui_revoked, AgentGrant.create!(agent:, capability: "react", granted_by: david)
revoked_grant.revoke!

at NOW - 20.minutes
thread = label :threads, :agent_ui, ChannelThread.create!(room: board, creator: david,
  name: "Agent UI parity work", work_status: "in_progress", work_owner: bot, last_activity_at: NOW - 20.minutes)
ThreadMembership.join!(thread, david)
ThreadMembership.join!(thread, bot)
label :agent_steps, :ui_done, AgentStep.create!(agent:, channel_thread: thread,
  name: "Inspect <changes>", status: "done", position: 0, duration_ms: 1050,
  input_summary: "Request & context", output_summary: "Checks passed")
label :agent_steps, :ui_running, AgentStep.create!(agent:, channel_thread: thread,
  name: "Prepare report", status: "running", position: 1)

at NOW - 15.minutes
label :agent_approvals, :ui_pending, AgentApproval.create!(agent:, room: channel, action: "deploy", summary: "Ship the UI parity slice", expires_at: NOW + 1.hour)
decided = label :agent_approvals, :ui_denied, AgentApproval.create!(agent:, action: "deploy", summary: "Rejected earlier request", expires_at: NOW + 1.hour)
decided.decide!(decision: "denied", by: david, note: "Wait for the remaining parity cases.")
overdue = label :agent_approvals, :ui_overdue, AgentApproval.create!(agent:, action: "deploy", summary: "Overdue request", expires_at: NOW + 1.hour)
overdue.update_columns(expires_at: NOW - 1.minute)

at NOW - 10.minutes
message = post channel, bot, NOW - 10.minutes, "<p>UI ledger content &amp; evidence.</p>", as: :agent_ui
label :agent_events, :ui_delivered, AgentEvent.create!(agent:, room: channel, message:, actor: david,
  event_type: "mention", outcome: "delivered",
  detail: "Recorded delivery", webhook_status: "failed", webhook_attempts: 2, webhook_last_error: "Recorded timeout")
label :agent_events, :ui_suppressed, AgentEvent.create!(agent:, room: channel, actor: david,
  event_type: "mention", outcome: "suppressed", detail: "Recorded missing capability")
at NOW
agent.update!(status: "working", status_note: "Review <changes> & continue", last_seen_at: NOW - 2.minutes)
Membership.update_all(unread_at: nil, connected_at: nil, connections: 0)
