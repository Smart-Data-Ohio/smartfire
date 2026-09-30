# Validate the built Smartfire corpus using the reference's actual models and encrypted secrets.
require "json"
seed = ARGV.first || "default"
abort "unknown parity seed #{seed}" unless %w[default first_run unread live_rooms imports agents_ui].include?(seed)
if seed == "first_run"
  checks = { no_account: !Account.exists?, no_users: !User.exists?, no_rooms: !Room.exists?, no_messages: !Message.exists? }
else
  enrolled_credentials = TwoFactorCredential.where.not(confirmed_at: nil)
checks = {
  markdown: Message.where.not(markdown_source: [nil, ""]).exists?,
  thread_replies: Message.where.not(thread_id: nil).exists?,
  forwards: Message.where.not(forwarded_from_message_id: nil).exists?,
  polls_open: Poll.open.exists?,
  polls_closed: Poll.where.not(closed_at: nil).exists?,
  pins: MessagePin.exists?,
  saved_pending: SavedItem.where(status: "in_progress").exists?,
  saved_done: SavedItem.where(status: "done").exists?,
  scheduled_pending: ScheduledMessage.pending.exists?,
  scheduled_sent: ScheduledMessage.past.exists?,
  group_dms: Room.directs.any? { |room| room.users.count > 2 },
  voice: Room.voices.exists?,
  stage: Rooms::Stage.exists?,
  boards: Room.boards.exists?,
  all_work_states: (ChannelThread.work.distinct.pluck(:work_status).sort == ChannelThread::WORK_STATUSES.sort),
  events: Event.exists?,
  agents: Agent.exists?,
  approvals: AgentApproval.exists?,
  activity: ActivityItem.exists?,
  github_recording: Github::PullRequest.where.not(fetched_at: nil).exists?,
  fizzy_recording: Fizzy::CardCache.where.not(fetched_at: nil).exists?,
  x_recording: Twitter::Post.where.not(fetched_at: nil).exists?,
  linkedin_recording: LinkEmbed.where("normalized_url LIKE '%linkedin.com%'").where.not(title: nil).exists?,
  link_recording: LinkEmbed.where(normalized_url: "https://example.com/parity").exists?,
  remembered_two_factor: TwoFactorRememberedDevice.exists?,
  enrolled_two_factor: enrolled_credentials.exists? && enrolled_credentials.all? { |credential| credential.secret.present? },
  setup_secret: TwoFactorSetupSecret.all.any? { |secret| secret.secret == "JBSWY3DPEHPK3PXP" },
  verified_sessions: Session.where.not(two_factor_verified_at: nil).exists?
}
  checks[:no_presence_leases] = !WorkspacePresenceLease.exists?
  case seed
  when "agents_ui"
    ui_agent = Agent.joins(:user).find_by!(users: { name: "Deploy Bot" })
    checks[:ui_owner] = ui_agent.owner == User.find_by!(email_address: "kevin@37signals.com")
    checks[:ui_credentials] = ui_agent.agent_credentials.count == 3
    checks[:ui_digest_identifiers] = ui_agent.agent_credentials.all? { |credential| credential.token_last_four == credential.token_digest[0, 4] }
    checks[:ui_credential_states] = ui_agent.agent_credentials.any?(&:revoked?) && ui_agent.agent_credentials.any?(&:expired?)
    checks[:ui_grants] = ui_agent.agent_grants.active.count == 4 && !ui_agent.legacy_capabilities?
    checks[:ui_revoked_grant] = ui_agent.agent_grants.where.not(revoked_at: nil).exists?
    checks[:ui_steps] = AgentStep.where(agent: ui_agent).count == 2
    checks[:ui_pending_and_denied] = ui_agent.agent_approvals.where(status: "pending").exists? && ui_agent.agent_approvals.where(status: "denied").exists?
    checks[:ui_overdue] = ui_agent.agent_approvals.where(status: "pending").any?(&:expired_effective?)
    checks[:ui_ledger] = ui_agent.agent_events.where(webhook_status: "failed", webhook_attempts: 2).exists?
    checks[:ui_status] = ui_agent.status == "working" && ui_agent.status_note.present?
  when "unread"
    checks[:unread_memberships] = Membership.where.not(unread_at: nil).count >= 3
  when "live_rooms"
    checks[:recorded_grants] = HuddleGrant.count == 2
    checks[:recorded_stream] = Stream.where.not(started_at: nil).exists?
    checks[:stage_speaker] = HuddleGrant.where(stage_role: "speaker").exists?
  when "imports"
    checks[:slack_workspace] = SlackWorkspace.where(team_id: "PARITY").exists?
    checks[:workspace_preview] = SlackImport.where(kind: "workspace", mode: "dry_run", status: "completed").exists?
    checks[:personal_preview] = SlackImport.where(kind: "personal", mode: "dry_run", status: "completed").exists?
    checks[:failed_import] = SlackImport.where(status: "failed", error: "Recorded permission error").exists?
  end
end
puts JSON.pretty_generate(checks: checks, passed: checks.count { |_, value| value }, failed: checks.count { |_, value| !value })
abort "parity seed verification failed" unless checks.values.all?
