module Agents
  # Shared approval requests for the REST agent approvals API and the MCP
  # request_approval / get_approval tools. Owns the external_action
  # capability checks, the external_id replay rule, expiry resolution,
  # and the payload shapes, so both surfaces decide identically.
  class Approvals
    POLL_MAX_LIMIT = 100

    def self.list(agent:, status: nil)
      unless agent.has_capability_anywhere?(:external_action)
        return ServiceResult.fail("Forbidden: agent lacks external_action capability", status: :forbidden)
      end

      status_filter = status.presence_in(AgentApproval::STATUSES)
      scope = AgentApproval.where(agent_id: agent.id).order(id: :desc)
      scope = apply_effective_status_filter(scope, status_filter) if status_filter
      approvals = scope.limit(POLL_MAX_LIMIT).includes(:room, :decided_by).to_a
      approvals.each(&:expire_if_due!)

      # Re-filter pending after lazy expiry so expired rows never read as pending.
      if status_filter == "pending"
        approvals.select!(&:pending_effective?)
      elsif status_filter == "expired"
        approvals.select!(&:expired_effective?)
      end

      ServiceResult.ok(approvals.map { |approval| approval_payload(approval) })
    end

    def self.show(agent:, id:)
      approval = AgentApproval.where(agent_id: agent.id).includes(:room, :decided_by).find_by(id: id)
      return ServiceResult.fail("Approval not found", status: :not_found) unless approval

      unless agent.can?(:external_action, approval.room)
        return ServiceResult.fail("Forbidden: agent lacks external_action capability", status: :forbidden)
      end

      approval.expire_if_due!

      ServiceResult.ok(approval_payload(approval))
    end

    # fields carries string keys: action, summary, room_id, payload,
    # external_id, expires_at, expires_in. A repeated external_id returns
    # the existing row instead of a duplicate.
    def self.create(agent:, fields:, credential: nil)
      room = find_request_room(agent, fields["room_id"])
      return ServiceResult.fail("Room not found", status: :not_found) if fields["room_id"].present? && room.nil?

      unless agent.can?(:external_action, room)
        return ServiceResult.fail("Forbidden: agent lacks external_action capability", status: :forbidden)
      end

      if fields["external_id"].present?
        existing = AgentApproval.where(agent_id: agent.id, external_id: fields["external_id"]).first
        if existing
          existing.expire_if_due!
          return ServiceResult.ok(approval_created_payload(existing))
        end
      end

      # github.* approvals carry an executable payload the server built and
      # bound to the summary the decider sees; they are only created through
      # the pull-request actions endpoint, never with agent-supplied payloads.
      if fields["action"].to_s.start_with?("github.")
        return ServiceResult.fail(
          "github.* actions are requested through /rooms/:room_id/agents/github/pull_request_actions",
          status: :unprocessable_entity
        )
      end

      # fizzy.* approvals carry the same kind of server-built executable
      # payload; they are only created through the Fizzy card actions
      # endpoint, never with agent-supplied payloads.
      if fields["action"].to_s.start_with?("fizzy.")
        return ServiceResult.fail(
          "fizzy.* actions are requested through /agents/fizzy/card_actions",
          status: :unprocessable_entity
        )
      end

      if (denial = Budgets.check(agent, :external_actions))
        return denial
      end

      approval = AgentApproval.new(
        agent: agent,
        room: room,
        agent_credential: credential,
        action: fields["action"],
        summary: fields["summary"],
        payload: serialize_payload(fields["payload"]),
        external_id: fields["external_id"],
        expires_at: resolve_expires_at(fields)
      )

      if approval.save
        ServiceResult.ok(approval_created_payload(approval), status: :created)
      else
        ServiceResult.fail(approval.errors.full_messages.to_sentence)
      end
    rescue ActiveRecord::RecordNotUnique
      # Two identical requests raced past the replay lookup; the loser
      # answers with the winner's row exactly like a replay.
      existing = AgentApproval.find_by!(agent_id: agent.id, external_id: fields["external_id"])
      existing.expire_if_due!
      ServiceResult.ok(approval_created_payload(existing))
    rescue ArgumentError => error
      ServiceResult.fail(error.message)
    end

    def self.cancel(agent:, id:)
      approval = AgentApproval.where(agent_id: agent.id).includes(:room, :decided_by).find_by(id: id)
      return ServiceResult.fail("Approval not found", status: :not_found) unless approval

      unless agent.can?(:external_action, approval.room)
        return ServiceResult.fail("Forbidden: agent lacks external_action capability", status: :forbidden)
      end

      begin
        approval.cancel_by_agent!
      rescue ActiveRecord::RecordInvalid
        return ServiceResult.fail(approval.errors.full_messages.to_sentence.presence || "Request cannot be cancelled")
      end

      ServiceResult.ok(approval_payload(approval))
    end

    def self.approval_created_payload(approval)
      {
        id: approval.id,
        status: approval.effective_status,
        expires_at: approval.expires_at&.utc
      }.compact
    end

    def self.approval_payload(approval)
      decided_by_name = approval.decided_by&.name
      {
        id: approval.id,
        action: approval.action,
        summary: approval.summary,
        payload: parse_stored_payload(approval.payload),
        room_id: approval.room_id,
        room_name: approval.room&.name,
        external_id: approval.external_id,
        status: approval.effective_status,
        expires_at: approval.expires_at&.utc,
        created_at: approval.created_at&.utc,
        decided_by: decided_by_name,
        decided_by_id: approval.decided_by_id,
        decided_at: approval.decided_at&.utc,
        decision_note: approval.decision_note,
        note: approval.decision_note
      }.compact
    end

    # Effective-status filter in SQL so the limit applies after filtering.
    # Pending means stored pending with a future deadline; expired means
    # stored expired or stored pending past its deadline.
    def self.apply_effective_status_filter(scope, status)
      case status
      when "pending"
        scope.where(status: "pending").where("expires_at > ?", Time.current)
      when "expired"
        scope.where("status = ? OR (status = ? AND expires_at <= ?)", "expired", "pending", Time.current)
      else
        scope.where(status: status)
      end
    end

    def self.find_request_room(agent, room_id)
      return nil if room_id.blank?

      room = Room.alive.find_by(id: room_id)
      return nil unless room && Membership.exists?(user_id: agent.user_id, room_id: room.id)

      room
    end

    def self.serialize_payload(payload)
      case payload
      when nil then nil
      when String then payload
      when ActionController::Parameters then payload.to_unsafe_h.to_json
      when Hash, Array then payload.to_json
      else payload.to_s
      end
    end

    def self.resolve_expires_at(fields)
      if fields["expires_in"].present?
        seconds = Integer(fields["expires_in"], exception: false)
        raise ArgumentError, "Invalid expires_in" if seconds.nil?

        seconds.seconds.from_now
      elsif fields["expires_at"].present?
        parsed = Time.zone.parse(fields["expires_at"].to_s)
        raise ArgumentError, "Invalid expires_at" if parsed.nil?

        parsed
      end
    end

    def self.parse_stored_payload(stored)
      return nil if stored.nil?

      JSON.parse(stored)
    rescue JSON::ParserError
      stored
    end
    private_class_method :find_request_room,
      :serialize_payload, :resolve_expires_at, :parse_stored_payload
  end
end
