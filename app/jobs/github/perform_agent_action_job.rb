class Github::PerformAgentActionJob < ApplicationJob
  # A running claim older than this is presumed stuck (its worker crashed
  # between the claim insert and the outcome rewrite) and failed by the
  # periodic sweep.
  STUCK_CLAIM_AFTER = 15.minutes

  # Both claim writers (finish_claim below and the stuck-claim sweep)
  # condition their outcome rewrite on the claim still running, so a
  # worker finishing while the sweep runs — or vice versa — cannot
  # overwrite the winner's result: the first write wins and the other
  # becomes a no-op that enqueues no webhook.
  RUNNING_CLAIM_CONDITION = "json_extract(agent_events.metadata, '$.status') = 'running'"

  # Marks stuck running claims failed. Runs from the periodic
  # periodic runner (bin/periodic) alongside the webhook sweep. Each stuck row is
  # rewritten like a failed finish_claim (failed status with a timeout
  # message, webhook enqueued when configured) and logged at error
  # level, since a stuck claim means a GitHub write may or may not have
  # happened. Never raises.
  def self.recover_stuck_claims!(now: Time.current)
    AgentEvent.where(event_type: "github_action_completed")
      .where("agent_events.created_at < ?", now - STUCK_CLAIM_AFTER)
      .where(RUNNING_CLAIM_CONDITION)
      .find_each do |event|
        begin
          event = AgentEvent.find(event.id)
          next unless event.metadata.is_a?(Hash) && event.metadata["status"] == "running"

          message = "GitHub action execution timed out"
          metadata = event.metadata.merge("status" => "failed", "message" => message)
          pending_webhook = event.agent.user.webhook.present?
          written = AgentEvent.where(id: event.id).where(RUNNING_CLAIM_CONDITION)
            .update_all(
              detail: message,
              webhook_status: pending_webhook ? "pending" : "none",
              webhook_next_attempt_at: (Time.current if pending_webhook),
              metadata: metadata
            ) == 1
          next unless written

          event.reload
          if (approval = AgentApproval.find_by(id: event.agent_approval_id || event.metadata["approval_id"]))
            AuditLog.record!(action: "agent.github_action.execute", actor: event.actor,
              target: approval, changes: { action: approval.action, status: "failed", message: message })
          end
          if pending_webhook
            Agent::EventWebhookJob.perform_later(event.id, event.webhook_attempts.to_i)
          end
          Rails.logger.error "Stuck GitHub claim #{event.id} for approval #{event.metadata["approval_id"]} " \
            "marked failed after #{STUCK_CLAIM_AFTER.inspect}"
        rescue => error
          Rails.logger.error "Stuck GitHub claim recovery failed for event #{event.id}: #{error.class}: #{error.message}"
        end
      end
  end

  # Runs an approved agent GitHub write action. Re-checks everything at
  # perform time — the approval is still approved, the agent is active and
  # still belongs to the room with external_action, the PR thread mapping
  # is still present, and the agent's linked account is still usable — and
  # performs the action with the agent's own token otherwise. Every outcome
  # is recorded as a github_action_completed ledger row; a failed re-check
  # or GitHub error makes no GitHub request beyond the failed call itself.
  # Never raises for GitHub or re-check failures, and never retries. Runs
  # at most once per approval: a queue retry or a duplicate enqueue finds
  # the earlier outcome row and stops. attempts: 1 opts out of the inherited
  # transient retries.
  retry_on(*ApplicationJob::TRANSIENT_ERRORS, attempts: 1)

  def perform(approval_id)
    approval = AgentApproval.find_by(id: approval_id)
    return unless approval&.github_action?

    agent = approval.agent
    room = approval.room
    return if already_executed?(agent, approval)

    if (reason = premature_failure_reason(approval, agent, room))
      return record_outcome(approval, agent, room, status: "failed", message: reason)
    end

    payload = parse_payload(approval.payload)
    pull_request = payload && Github::PullRequest.find_by(id: payload["pull_request_id"])
    unless pull_request && Github::PullRequestThread.exists?(github_pull_request_id: pull_request.id, room_id: room.id)
      return record_outcome(approval, agent, room, status: "failed",
        message: "The pull request is no longer discussed in this room")
    end

    action = Github::AgentPullRequestAction.from_payload(pull_request: pull_request, payload: payload)
    unless action.valid?
      return record_outcome(approval, agent, room, status: "failed", message: "Approval payload is invalid")
    end

    # The decider saw the approval's action name and summary, never the
    # payload. Only an action whose name and summary rebuild to exactly what
    # was approved may run, so a payload that describes a different PR,
    # kind, body, or reviewer list than the summary is refused.
    unless action.action_name == approval.action && action.summary == approval.summary
      return record_outcome(approval, agent, room, status: "failed",
        message: "Approval summary does not match its payload")
    end

    account = Github::AgentIdentity.resolve(agent)
    unless account&.usable?
      return record_outcome(approval, agent, room, status: "failed",
        message: "Agent has no usable GitHub account")
    end

    # The decider approved acting as the identity recorded at request time;
    # a relinked or replaced connection must never inherit that approval.
    unless approval.github_identity_matches?(account)
      return record_outcome(approval, agent, room, status: "failed",
        message: "The agent's GitHub account changed since this was approved")
    end

    begin
      token = account.access_token_for_use
      if token.nil?
        return record_outcome(approval, agent, room, status: "failed",
          message: "Agent has no usable GitHub account")
      end
      client = Github::WriteClient.new(token: token)
    rescue ActiveRecord::Encryption::Errors::Decryption
      account.mark_disconnected!(GithubConnectedAccount::UNREADABLE_TOKEN_REASON)
      return record_outcome(approval, agent, room, status: "failed",
        message: GithubConnectedAccount::UNREADABLE_TOKEN_REASON)
    end

    # The outcome row is claimed before the GitHub write: the unique index
    # admits exactly one claim per approval, so a concurrent job that also
    # passed already_executed? loses the insert here and returns without
    # calling GitHub. The winner rewrites its claim with the real outcome
    # below, so a failed call still records a failure, never a retry.
    event = claim_execution(approval, agent, room)
    return unless event

    begin
      response = action.perform(client)
    rescue Github::WriteClient::Unauthorized
      account.mark_disconnected!("GitHub rejected the linked token (401)")
      return finish_claim(event, approval, agent, status: "failed",
        message: "GitHub rejected the agent's linked token (401)")
    rescue Github::WriteClient::Refused, Github::WriteClient::Error => error
      return finish_claim(event, approval, agent, status: "failed", message: error.message)
    end

    url = response.is_a?(Hash) ? response["html_url"] : nil
    finish_claim(event, approval, agent, status: "completed", url: url)
  end

  private
    # Historical duplicate rows keep agent_approval_id NULL (the additive
    # migration backfills only the lowest id per approval), so the column
    # lookup falls back to the metadata payload SQLite already stores.
    def already_executed?(agent, approval)
      completion_events(agent, approval).exists?
    end

    def completion_events(agent, approval)
      agent.agent_events.where(event_type: "github_action_completed").where(
        "agent_approval_id = ? OR CAST(json_extract(metadata, '$.approval_id') AS INTEGER) = ?",
        approval.id, approval.id
      )
    end

    def premature_failure_reason(approval, agent, room)
      return "Approval is no longer approved" unless approval.status == "approved"
      return "Agent is suspended or deactivated" unless agent&.active?
      return "Room no longer exists" if room.nil?
      return "Agent is no longer a member of the room" unless Membership.exists?(user_id: agent.user_id, room_id: room.id)
      return "Agent no longer has the external_action capability" unless agent.can?(:external_action, room)

      nil
    end

    def parse_payload(stored)
      parsed = stored.is_a?(String) ? JSON.parse(stored) : nil
      parsed.is_a?(Hash) ? parsed : nil
    rescue JSON::ParserError
      nil
    end

    # The unique index on (agent, approval) for completion rows makes the
    # insert the atomic claim: a duplicate enqueue that passed
    # already_executed? first lands here, loses the insert, and returns
    # the winner instead of writing a second row.
    def record_outcome(approval, agent, room, status:, message: nil, url: nil)
      pending_webhook = agent.user.webhook.present?
      event = agent.agent_events.create!(
        event_type: "github_action_completed",
        room: room,
        actor: approval.decided_by,
        outcome: "delivered",
        detail: (message if status == "failed"),
        agent_approval_id: approval.id,
        webhook_status: pending_webhook ? "pending" : "none",
        webhook_next_attempt_at: (Time.current if pending_webhook),
        metadata: {
          "approval_id" => approval.id,
          "action" => approval.action,
          "status" => status,
          "url" => url,
          "message" => message
        }.compact
      )
      record_execution_audit(approval, status: status, message: message, url: url)
      enqueue_outcome_webhook(event)
      event
    rescue ActiveRecord::RecordNotUnique
      completion_events(agent, approval).order(:id).first!
    end

    # Inserts the winner's outcome row ahead of the GitHub write. Returns
    # the claim, or nil when another job already owns this approval (its
    # claim, completed row, or premature-failure row holds the key). The
    # claim carries webhook_status none so no webhook goes out until the
    # winner rewrites it with the real outcome.
    def claim_execution(approval, agent, room)
      agent.agent_events.create!(
        event_type: "github_action_completed",
        room: room,
        actor: approval.decided_by,
        outcome: "delivered",
        agent_approval_id: approval.id,
        webhook_status: "none",
        metadata: {
          "approval_id" => approval.id,
          "action" => approval.action,
          "status" => "running"
        }
      )
    rescue ActiveRecord::RecordNotUnique
      nil
    end

    # Rewrites the winner's claim with the GitHub result. A crash between
    # the claim and this write leaves a "running" row behind, which keeps
    # later runs from posting a duplicate write; at-most-once is the
    # correct bias for a call GitHub may already have applied. The rewrite
    # lands only while the claim is still running: when the stuck-claim
    # sweep already failed the row, this becomes a no-op that enqueues no
    # webhook, so the two writers cannot overwrite each other's outcome.
    def finish_claim(event, approval, agent, status:, message: nil, url: nil)
      pending_webhook = agent.user.webhook.present?
      written = AgentEvent.where(id: event.id).where(RUNNING_CLAIM_CONDITION)
        .update_all(
          detail: (message if status == "failed"),
          webhook_status: pending_webhook ? "pending" : "none",
          webhook_next_attempt_at: (Time.current if pending_webhook),
          metadata: {
            "approval_id" => approval.id,
            "action" => approval.action,
            "status" => status,
            "url" => url,
            "message" => message
          }.compact
        ) == 1
      event.reload
      if written
        record_execution_audit(approval, status: status, message: message, url: url)
        enqueue_outcome_webhook(event)
      end
      event
    end

    # The human who approved is the actor: the job executes with their authority.
    def record_execution_audit(approval, status:, message: nil, url: nil)
      AuditLog.record!(action: "agent.github_action.execute", actor: approval.decided_by,
        target: approval, changes: { action: approval.action, status: status, url: url, message: message }.compact)
    end

    def enqueue_outcome_webhook(event)
      return unless event.webhook_pending?

      ActiveRecord.after_all_transactions_commit do
        Agent::EventWebhookJob.perform_later(event.id, event.webhook_attempts.to_i)
      end
    end
end
