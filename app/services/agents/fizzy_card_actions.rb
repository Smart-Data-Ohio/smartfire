module Agents
  # Shared Fizzy write-action requests for POST
  # /agents/fizzy/card_actions and the MCP create_fizzy_card /
  # comment_on_fizzy_card / move_fizzy_card / close_fizzy_card /
  # reopen_fizzy_card tools. Owns the workspace-wide external_action
  # check, the agent-owner account resolution, the external_id replay
  # rule, and the approval creation, so both surfaces request
  # identically. Never calls Fizzy: it creates an AgentApproval for a
  # human decider, and the action only runs when approved.
  class FizzyCardActions
    def self.create(agent:, fields:, credential: nil)
      unless agent.can?(:external_action, nil)
        return ServiceResult.fail("Forbidden: agent lacks external_action capability", status: :forbidden)
      end

      owner = agent.owner
      if owner.nil?
        return ServiceResult.fail("Agent has no owner recorded")
      end

      account = owner.fizzy_connected_account
      unless account&.usable?
        return ServiceResult.fail("Agent owner has no usable Fizzy account")
      end

      if fields["external_id"].present?
        existing = AgentApproval.where(agent_id: agent.id, external_id: fields["external_id"]).first
        if existing
          existing.expire_if_due!
          return ServiceResult.ok(Approvals.approval_created_payload(existing))
        end
      end

      action = ::Fizzy::AgentCardAction.new(
        account_id: fields["account_id"].presence || account.fizzy_account_id,
        kind: fields["kind"].to_s,
        board_id: fields["board_id"],
        number: fields["number"],
        column_id: fields["column_id"],
        title: fields["title"],
        description: fields["description"],
        body: fields["body"]
      )
      unless action.valid?
        # REST renders the errors hash; MCP renders the sentence. Both
        # carry the same field failures.
        return ServiceResult.fail(
          action.errors.full_messages.to_sentence,
          payload: { errors: action.errors.to_hash }
        )
      end

      approval = AgentApproval.new(
        agent: agent,
        agent_credential: credential,
        action: action.action_name,
        summary: action.summary,
        payload: action.payload_json,
        external_id: fields["external_id"].presence,
        fizzy_connected_account_id: account.id,
        fizzy_user_id: account.fizzy_user_id,
        fizzy_user_name: account.fizzy_user_name
      )

      if approval.save
        ServiceResult.ok(Approvals.approval_created_payload(approval), status: :accepted)
      else
        ServiceResult.fail(approval.errors.full_messages.to_sentence)
      end
    rescue ActiveRecord::RecordNotUnique
      # Two identical requests raced past the replay lookup; the loser
      # answers with the winner's row exactly like a replay.
      existing = AgentApproval.find_by!(agent_id: agent.id, external_id: fields["external_id"])
      existing.expire_if_due!
      ServiceResult.ok(Approvals.approval_created_payload(existing))
    end
  end
end
