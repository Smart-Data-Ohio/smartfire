class Agent::Delivery
  RATE_LIMIT_PER_MINUTE = 20
  RATE_WINDOW = 1.minute
  HOP_LIMIT = 3
  TRIGGER_WINDOW = 5.minutes

  # Raised when a webhook cannot be built (message, approval, or thread
  # gone): the delivery job records it and does not retry.
  class UndeliverableWebhook < StandardError; end

  class << self
    # Called after a message commits. Writes a `posted` row when the author
    # is an agent, then creates one pending event per recipient agent (or a
    # suppression row when rate- or hop-limited) and enqueues delivery jobs.
    def enqueue_for_message(message)
      room = message.room
      return unless room

      sender_agent = Agent.find_by(user_id: message.creator_id)
      message_hop, chain_id = message_hop_and_chain_for(message, sender_agent)

      if sender_agent
        sender_agent.agent_events.create!(
          event_type: "posted",
          room: room,
          message: message,
          actor_id: message.creator_id,
          outcome: "delivered",
          chain_id: chain_id,
          metadata: { "hop" => message_hop, "thread_id" => message.thread_id }
        )
      end

      recipients_for(message).each do |agent, event_type|
        if message_hop >= HOP_LIMIT
          agent.agent_events.create!(
            event_type: "delivery_suppressed_hop_limit",
            room: room,
            message: message,
            actor_id: message.creator_id,
            outcome: "suppressed",
            detail: "Hop limit reached (hop #{message_hop})",
            chain_id: chain_id,
            metadata: { "hop" => message_hop }
          )
          next
        end

        if rate_limited?(agent, room)
          agent.agent_events.create!(
            event_type: "delivery_suppressed_rate_limit",
            room: room,
            message: message,
            actor_id: message.creator_id,
            outcome: "suppressed",
            detail: "Rate limit exceeded (#{RATE_LIMIT_PER_MINUTE} per minute)",
            chain_id: chain_id,
            metadata: { "hop" => message_hop }
          )
          next
        end

        event = agent.agent_events.create!(
          event_type: event_type,
          room: room,
          message: message,
          actor_id: message.creator_id,
          outcome: "pending",
          chain_id: chain_id,
          metadata: { "hop" => message_hop }
        )
        Agent::DeliveryJob.perform_later(event.id)
      end
    end

    # Shared work payload for event polling and the webhook. The url is
    # the workspace permalink path for the thread, the same form thread
    # push notifications use; agents combine it with their configured
    # host. The thread_id, status, and assigned_by keys predate the
    # shared builder and keep their names; everything else comes from it.
    def work_payload(thread, assigned_by:)
      Agents::WorkPayload.for(thread).merge(
        thread_id: thread.id,
        status: thread.work_status,
        assigned_by: assigned_by
      )
    end

    # Posts a work assignment change to the agent's webhook. The payload
    # carries the same additive agent key as other deliveries plus the
    # event type (work_assigned or work_unassigned) and a work key with
    # the thread fields. Response bodies are ignored: an assignment
    # notification never creates a reply message.
    def post_work_webhook!(webhook, event, work:, agent:)
      payload = {
        agent: { id: agent.id, name: agent.user.name, owner: agent.owner&.name, delivery_id: event.id },
        event_type: event.event_type,
        work: work
      }.to_json

      webhook.post_payload(payload, secret: agent.ensure_webhook_signing_secret!)
    end

    # Posts an approval decision to the agent's webhook. The payload carries
    # the same additive agent key as message deliveries plus an approval key
    # with the decision fields. Response bodies are ignored: a decision
    # notification never creates a reply message.
    def post_approval_webhook!(webhook, approval, agent:, delivery_id:)
      payload = {
        agent: { id: agent.id, name: agent.user.name, owner: agent.owner&.name, delivery_id: delivery_id },
        approval: {
          approval_id: approval.id,
          status: approval.status,
          decided_by: approval.decided_by&.name,
          note: approval.decision_note
        }
      }.to_json

      webhook.post_payload(payload, secret: agent.ensure_webhook_signing_secret!)
    end

    # Posts a GitHub write-action result to the agent's webhook. The payload
    # carries the same additive agent key as approval decisions plus a
    # github_action key with the completion fields. Response bodies are
    # ignored: a completion notification never creates a reply message.
    def post_github_action_webhook!(webhook, event, agent:)
      metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
      payload = {
        agent: { id: agent.id, name: agent.user.name, owner: agent.owner&.name, delivery_id: event.id },
        github_action: {
          approval_id: metadata["approval_id"],
          action: metadata["action"],
          status: metadata["status"],
          url: metadata["url"],
          message: metadata["message"]
        }.compact
      }.to_json

      webhook.post_payload(payload, secret: agent.ensure_webhook_signing_secret!)
    end

    # Posts one event row to the agent's webhook. Message events carry
    # the message payload with sync replies; approval, GitHub action,
    # and work events carry their own payloads and ignore response
    # bodies. Raises UndeliverableWebhook when the payload cannot be
    # built, and lets transport errors propagate for the caller to
    # retry. Any completed HTTP response counts as delivered.
    def post_event_webhook!(webhook, event, agent:)
      case event.event_type
      when *AgentEvent::MESSAGE_DELIVERABLE_TYPES
        message = Message.find_by(id: event.message_id)
        raise UndeliverableWebhook, "Message no longer available" unless message

        webhook.deliver(message, agent: agent, delivery_id: event.id)
      when "approval_decided"
        approval = AgentApproval.find_by(id: event.agent_approval_id) if event.agent_approval_id
        approval ||= AgentApproval.find_by(id: event.metadata["approval_id"]) if event.metadata.is_a?(Hash)
        raise UndeliverableWebhook, "Approval no longer available" unless approval

        post_approval_webhook!(webhook, approval, agent: agent, delivery_id: event.id)
      when "github_action_completed"
        post_github_action_webhook!(webhook, event, agent: agent)
      when *AgentEvent::WORK_DELIVERABLE_TYPES
        metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
        thread = ChannelThread.find_by(id: metadata["thread_id"])
        work = if thread
          work_payload(thread, assigned_by: metadata["assigned_by"])
        else
          metadata["work_snapshot"]
        end
        raise UndeliverableWebhook, "Thread no longer available" unless work

        post_work_webhook!(webhook, event, work: work, agent: agent)
      else
        raise UndeliverableWebhook, "Event type #{event.event_type} has no webhook payload"
      end
    end

    # Runs inside Agent::DeliveryJob. Re-checks everything at perform time:
    # grant, membership, rate, hop, and message existence. Marks the row
    # delivered and enqueues its webhook when one is configured, or
    # records a suppression. Idempotent: settled rows are left alone, and
    # the webhook job posts at most once per row. An agent that acked the
    # row by polling before this ran still gets its webhook: acking marks
    # polling state only and never cancels a webhook still owed.
    def perform(event)
      event = AgentEvent.find_by(id: event.is_a?(AgentEvent) ? event.id : event)
      return unless event
      return unless AgentEvent::MESSAGE_DELIVERABLE_TYPES.include?(event.event_type)

      if event.outcome == "acknowledged"
        enqueue_owed_webhook(event)
        return
      end

      return unless event.outcome == "pending"

      agent = event.agent
      room = Room.find_by(id: event.room_id)
      message = Message.find_by(id: event.message_id)

      if message.nil? || room.nil?
        claim(event, outcome: "suppressed", detail: "Message no longer available")
        return
      end

      unless agent.active? && member_of?(agent, room) && agent.can?(:read_messages, room)
        suppress(event, "delivery_suppressed_revoked", "Grant revoked or room access removed")
        return
      end

      if event.hop >= HOP_LIMIT
        suppress(event, "delivery_suppressed_hop_limit", "Hop limit reached (hop #{event.hop})")
        return
      end

      if rate_limited?(agent, room, exclude: event)
        suppress(event, "delivery_suppressed_rate_limit", "Rate limit exceeded (#{RATE_LIMIT_PER_MINUTE} per minute)")
        return
      end

      webhook_status = agent.user.webhook ? "pending" : "none"
      return unless claim(event, outcome: "delivered", webhook_status: webhook_status)

      Agent::EventWebhookJob.perform_later(event.id) if webhook_status == "pending"
    end

    # Webhook half of an acknowledged row: the agent already read it by
    # polling, so no grant or membership re-check applies, but a
    # configured webhook is still owed exactly one POST.
    def enqueue_owed_webhook(event)
      return unless event.agent.user.webhook
      return unless AgentEvent.where(id: event.id, outcome: "acknowledged", webhook_status: "none")
        .update_all(webhook_status: "pending") == 1

      Agent::EventWebhookJob.perform_later(event.id)
    end

    # Message hop for the legacy webhook gate. Recomputes the same value
    # enqueue_for_message used, so the legacy path honors the hop limit
    # even though it writes no ledger rows of its own.
    def hop_for_message(message)
      sender_agent = Agent.find_by(user_id: message.creator_id)
      message_hop_for(message, sender_agent)
    end

    # A work assignment carries the assigning agent's chain forward, so
    # two agents assigning posts to each other escalate and stop instead
    # of looping. Human assigners always start a new root at hop 0.
    def work_assignment_hop_and_chain_for(actor)
      agent = actor.is_a?(User) ? (actor.agent || Agent.find_by(user_id: actor.id)) : nil
      return [ 0, SecureRandom.uuid ] unless agent

      trigger = hop_trigger_for(agent)
      trigger ? [ trigger.hop + 1, trigger.chain_id || SecureRandom.uuid ] : [ 0, SecureRandom.uuid ]
    end

    private
      # One entry per recipient agent: [agent, event_type]. Direct rooms
      # notify every other agent member; elsewhere a reply to an agent's
      # message wins over a mention when both apply to the same agent.
      def recipients_for(message)
        room = message.room
        agents_by_user_id = Agent.where(user_id: room.user_ids).index_by(&:user_id)
        return [] if agents_by_user_id.empty?

        if room.direct?
          agents_by_user_id.filter_map do |user_id, agent|
            next if user_id == message.creator_id

            [ agent, "direct_message" ]
          end
        else
          recipients = {}

          message.mentionees.each do |user|
            next if user.id == message.creator_id
            next unless (agent = agents_by_user_id[user.id])

            recipients[agent.id] ||= [ agent, "mention" ]
          end

          if (reply_source = message.reply_to_message) && reply_source.creator_id != message.creator_id
            if (agent = agents_by_user_id[reply_source.creator_id])
              recipients[agent.id] = [ agent, "reply" ]
            end
          end

          recipients.values
        end
      end

      # Human messages start a chain at hop 0. An agent's message continues
      # the chain of its server-authorized trigger: the most recent message
      # or work event pending for, delivered to, or acknowledged by the
      # agent in any room within the trigger window, so bridging rooms
      # carries the chain instead of resetting it. The agent's own posted
      # rows, suppression rows, and approval decisions are never triggers,
      # and neither the request body nor the reply target influences the
      # hop. A message with no recent trigger is a new root at hop 0.
      # Legacy bot messages continue the chain of the message they answer,
      # read from its ledger rows (including the sender's posted row).
      def message_hop_and_chain_for(message, sender_agent)
        if sender_agent
          trigger = hop_trigger_for(sender_agent)
          return trigger ? [ trigger.hop + 1, trigger.chain_id || SecureRandom.uuid ] : [ 0, SecureRandom.uuid ]
        end

        return legacy_message_hop_and_chain_for(message) if message.creator&.bot?

        [ 0, SecureRandom.uuid ]
      end

      def message_hop_for(message, sender_agent)
        message_hop_and_chain_for(message, sender_agent).first
      end

      def hop_trigger_for(agent)
        agent.agent_events.where(event_type: AgentEvent::HOP_TRIGGER_TYPES, outcome: AgentEvent::HOP_TRIGGER_OUTCOMES)
          .where("agent_events.created_at >= ?", TRIGGER_WINDOW.ago)
          .order(id: :desc).first
      end

      # A legacy bot has no ledger of its own, so its chain comes from the
      # message it answers: a reply continues its source message's highest
      # recorded hop (any agent's rows, including posted rows), while a
      # root post continues the most recent room message within the window
      # that mentioned the bot or replied to one of its messages. Anything
      # else is a new root at hop 0.
      def legacy_message_hop_and_chain_for(message)
        source = message.reply_to_message || legacy_trigger_message_for(message)
        return [ 0, SecureRandom.uuid ] unless source

        row = AgentEvent.where(message_id: source.id, outcome: AgentEvent::HOP_TRIGGER_OUTCOMES)
          .order(hop: :desc, id: :desc).first
        row ? [ row.hop + 1, row.chain_id || SecureRandom.uuid ] : [ 0, SecureRandom.uuid ]
      end

      # Bounded scan for the newest room message within the trigger window
      # that plausibly triggered a legacy bot's root post: one that
      # mentions the bot or replies to its messages. Mention parsing reads
      # the preloaded bodies; no per-row user query runs.
      def legacy_trigger_message_for(message)
        bot_id = message.creator_id
        candidates = Message.where(room_id: message.room_id)
          .where("messages.created_at >= ?", TRIGGER_WINDOW.ago)
          .where.not(id: message.id, creator_id: bot_id)
          .order(id: :desc).limit(25)
          .includes(:rich_text_body, :reply_to_message)

        candidates.find do |candidate|
          (candidate.reply_to_message_id.present? && candidate.reply_to_message&.creator_id == bot_id) ||
            mentioned_user_ids(candidate).include?(bot_id)
        end
      end

      def mentioned_user_ids(message)
        if message.body&.body
          message.body.body.attachables.grep(User).map(&:id)
        else
          []
        end
      end

      def rate_limited?(agent, room, exclude: nil)
        scope = agent.agent_events.message_deliverable
          .where(room_id: room.id)
          .where("created_at >= ?", RATE_WINDOW.ago)
          .where(outcome: %w[ pending delivered acknowledged ])
        scope = scope.where.not(id: exclude.id) if exclude
        scope.count >= RATE_LIMIT_PER_MINUTE
      end

      def member_of?(agent, room)
        Membership.exists?(user_id: agent.user_id, room_id: room.id)
      end

      # Atomically transitions a pending row; false when another job
      # already claimed it, so two concurrent jobs cannot both post and the
      # loser exits without posting or writing a duplicate suppression row.
      def claim(event, outcome:, detail: nil, webhook_status: nil)
        updates = { outcome: outcome }
        updates[:detail] = detail unless detail.nil?
        updates[:webhook_status] = webhook_status unless webhook_status.nil?
        AgentEvent.where(id: event.id, outcome: "pending").update_all(updates) == 1
      end

      def suppress(event, event_type, detail)
        return unless claim(event, outcome: "suppressed", detail: detail)

        event.agent.agent_events.create!(
          event_type: event_type,
          room_id: event.room_id,
          message_id: event.message_id,
          actor_id: event.actor_id,
          outcome: "suppressed",
          detail: detail,
          chain_id: event.chain_id,
          metadata: { "hop" => event.hop }
        )
      end
  end
end
