class Fizzy::PerformAgentActionJob < ApplicationJob
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
  # level, since a stuck claim means a Fizzy write may or may not have
  # happened. Never raises.
  def self.recover_stuck_claims!(now: Time.current)
    AgentEvent.where(event_type: "fizzy_action_completed")
      .where("agent_events.created_at < ?", now - STUCK_CLAIM_AFTER)
      .where(RUNNING_CLAIM_CONDITION)
      .find_each do |event|
        begin
          event = AgentEvent.find(event.id)
          next unless event.metadata.is_a?(Hash) && event.metadata["status"] == "running"

          message = "Fizzy action execution timed out"
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
          if pending_webhook
            Agent::EventWebhookJob.perform_later(event.id, event.webhook_attempts.to_i)
          end
          Rails.logger.error "Stuck Fizzy claim #{event.id} for approval #{event.metadata["approval_id"]} " \
            "marked failed after #{STUCK_CLAIM_AFTER.inspect}"
        rescue => error
          Rails.logger.error "Stuck Fizzy claim recovery failed for event #{event.id}: #{error.class}: #{error.message}"
        end
      end
  end

  # Runs an approved agent Fizzy write action. Re-checks everything at
  # perform time — the approval is still approved, the agent is active
  # with a workspace-wide external_action grant, and the owner's linked
  # account is still usable and still the recorded one — and performs
  # the action with the owner's token otherwise. Every outcome is
  # recorded as a fizzy_action_completed ledger row; a failed re-check
  # or Fizzy error makes no Fizzy request beyond the failed call
  # itself. Never raises for Fizzy or re-check failures, and never
  # retries. Runs at most once per approval: a queue retry or a
  # duplicate enqueue finds the earlier outcome row and stops.
  # attempts: 1 opts out of the inherited transient retries.
  retry_on(*ApplicationJob::TRANSIENT_ERRORS, attempts: 1)

  def perform(approval_id)
    approval = AgentApproval.find_by(id: approval_id)
    return unless approval&.fizzy_action?

    agent = approval.agent
    return if already_executed?(agent, approval)

    if (reason = premature_failure_reason(approval, agent))
      return record_outcome(approval, agent, status: "failed", message: reason)
    end

    payload = parse_payload(approval.payload)
    action = payload && Fizzy::AgentCardAction.from_payload(payload: payload)
    unless action&.valid?
      return record_outcome(approval, agent, status: "failed", message: "Approval payload is invalid")
    end

    # The decider saw the approval's action name and summary, never the
    # payload. Only an action whose name and summary rebuild to exactly what
    # was approved may run, so a payload that describes a different card,
    # kind, board, or body than the summary is refused.
    unless action.action_name == approval.action && action.summary == approval.summary
      return record_outcome(approval, agent, status: "failed",
        message: "Approval summary does not match its payload")
    end

    account = agent.owner&.fizzy_connected_account
    unless account&.usable?
      return record_outcome(approval, agent, status: "failed",
        message: "Agent owner has no usable Fizzy account")
    end

    # The decider approved acting as the identity recorded at request time;
    # a relinked or replaced connection must never inherit that approval.
    unless approval.fizzy_identity_matches?(account)
      return record_outcome(approval, agent, status: "failed",
        message: "The agent owner's Fizzy account changed since this was approved")
    end

    begin
      client = Fizzy::Client.new(token: account.access_token)
    rescue ActiveRecord::Encryption::Errors::Decryption
      account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
      return record_outcome(approval, agent, status: "failed",
        message: FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
    end

    # The outcome row is claimed before the Fizzy write: the unique index
    # admits exactly one claim per approval, so a concurrent job that also
    # passed already_executed? loses the insert here and returns without
    # calling Fizzy. The winner rewrites its claim with the real outcome
    # below, so a failed call still records a failure, never a retry.
    event = claim_execution(approval, agent)
    return unless event

    begin
      response = action.perform(client)
    rescue Fizzy::Client::Unauthorized
      return finish_unauthorized_claim(event, approval, agent, account, client)
    rescue Fizzy::Client::Refused, Fizzy::Client::Error => error
      return finish_claim(event, approval, agent, status: "failed", message: error.message)
    end

    url = response.is_a?(Hash) ? response["url"] : nil
    finish_claim(event, approval, agent, status: "completed", url: url)
  end

  private
    def already_executed?(agent, approval)
      completion_events(agent, approval).exists?
    end

    def completion_events(agent, approval)
      agent.agent_events.where(event_type: "fizzy_action_completed").where(
        "agent_approval_id = ? OR CAST(json_extract(metadata, '$.approval_id') AS INTEGER) = ?",
        approval.id, approval.id
      )
    end

    def premature_failure_reason(approval, agent)
      return "Approval is no longer approved" unless approval.status == "approved"
      return "Agent is suspended or deactivated" unless agent&.active?
      return "Agent no longer has the external_action capability" unless agent.can?(:external_action, nil)

      nil
    end

    def parse_payload(stored)
      parsed = stored.is_a?(String) ? JSON.parse(stored) : nil
      parsed.is_a?(Hash) ? parsed : nil
    rescue JSON::ParserError
      nil
    end

    # A 401 on a write is ambiguous: the token may be revoked, or it may
    # be a read-only token (Fizzy's read permission rejects writes with
    # 401). A cheap identity read tells them apart so a valid read-only
    # token is never marked disconnected.
    def finish_unauthorized_claim(event, approval, agent, account, client)
      begin
        client.identity
      rescue Fizzy::Client::Error
        account.mark_disconnected!("Fizzy rejected the linked token (401)")
        return finish_claim(event, approval, agent, status: "failed",
          message: "Fizzy rejected the agent owner's linked token (401)")
      end

      finish_claim(event, approval, agent, status: "failed",
        message: "The agent owner's Fizzy token is read-only; card writes need a Read + Write token")
    end

    # The unique index on (agent, approval) for completion rows makes the
    # insert the atomic claim: a duplicate enqueue that passed
    # already_executed? first lands here, loses the insert, and returns
    # the winner instead of writing a second row.
    def record_outcome(approval, agent, status:, message: nil, url: nil)
      pending_webhook = agent.user.webhook.present?
      event = agent.agent_events.create!(
        event_type: "fizzy_action_completed",
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
      enqueue_outcome_webhook(event)
      event
    rescue ActiveRecord::RecordNotUnique
      completion_events(agent, approval).order(:id).first!
    end

    # Inserts the winner's outcome row ahead of the Fizzy write. Returns
    # the claim, or nil when another job already owns this approval (its
    # claim, completed row, or premature-failure row holds the key). The
    # claim carries webhook_status none so no webhook goes out until the
    # winner rewrites it with the real outcome.
    def claim_execution(approval, agent)
      agent.agent_events.create!(
        event_type: "fizzy_action_completed",
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

    # Rewrites the winner's claim with the Fizzy result. A crash between
    # the claim and this write leaves a "running" row behind, which keeps
    # later runs from posting a duplicate write; at-most-once is the
    # correct bias for a call Fizzy may already have applied. The rewrite
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
      enqueue_outcome_webhook(event) if written
      event
    end

    def enqueue_outcome_webhook(event)
      return unless event.webhook_pending?

      ActiveRecord.after_all_transactions_commit do
        Agent::EventWebhookJob.perform_later(event.id, event.webhook_attempts.to_i)
      end
    end
end
