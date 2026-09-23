class Agent::Delivery
  RATE_LIMIT_PER_MINUTE = 20
  RATE_WINDOW = 1.minute
  HOP_LIMIT = 3
  TRIGGER_WINDOW = 5.minutes
  # A webhook row still pending past this age with attempts remaining is
  # presumed stranded (its enqueue never ran) and picked up by the sweep.
  STRANDED_WEBHOOK_AFTER = 2.minutes
  # A pending row that already spent every attempt but never recorded an
  # outcome (its worker died between the claim and the write) would match
  # no sweep and sit pending forever. Once it has been past the stranded
  # grace for this further margin — long enough that no live worker can
  # still be writing its outcome — the sweep marks it failed instead.
  EXHAUSTED_WEBHOOK_AFTER = 5.minutes

  # Raised when a webhook cannot be built (message, approval, or thread
  # gone): the delivery job records it and does not retry.
  class UndeliverableWebhook < StandardError; end

  # Raised when the webhook endpoint answers 429, 408, or 5xx: the
  # delivery job retries with backoff. Carries the endpoint's
  # Retry-After delay in seconds when it sent a parseable one.
  class RetryableWebhookResponse < StandardError
    attr_reader :retry_after

    def initialize(message, retry_after: nil)
      @retry_after = retry_after
      super(message)
    end
  end

  # Raised when the endpoint answers any other non-2xx status: the
  # delivery job records it and does not retry.
  class PermanentWebhookResponse < StandardError; end

  # Upper bound for an endpoint-provided Retry-After delay.
  RETRY_AFTER_MAX = 1.hour

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

        # The rate check and the row insert share the agent's row lock so
        # concurrent enqueues cannot both pass the count and over-deliver.
        # Each recipient locks only its own agent row; different agents
        # proceed in parallel.
        event = agent.with_lock do
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
            nil
          else
            agent.agent_events.create!(
              event_type: event_type,
              room: room,
              message: message,
              actor_id: message.creator_id,
              outcome: "pending",
              chain_id: chain_id,
              metadata: { "hop" => message_hop }
            )
          end
        end
        Agent::DeliveryJob.perform_later(event.id) if event
      end
    end

    # Shared work payload for event polling and the webhook. The url is
    # the workspace permalink path for the thread, the same form thread
    # push notifications use; agents combine it with their configured
    # host. The thread_id, status, and assigned_by keys predate the
    # shared builder and keep their names; everything else comes from it.
    def work_payload(thread, assigned_by:, agent: nil)
      Agents::WorkPayload.for(thread, agent: agent).merge(
        thread_id: thread.id,
        status: thread.work_status,
        assigned_by: assigned_by
      )
    end

    # Delivers a slash_command invocation row to the agent's webhook.
    # Mirrors the work-assignment delivery: the row is already committed
    # as delivered (polling reads it), and this owes the webhook POST.
    # Re-checks membership and the post_messages grant, since invoking a
    # command requires them.
    def deliver_command_webhook(event)
      return unless AgentEvent::SLASH_DELIVERABLE_TYPES.include?(event.event_type)

      agent = event.agent
      room = Room.alive.find_by(id: event.room_id)
      return unless room
      return unless Membership.exists?(user_id: agent.user_id, room_id: room.id) && agent.can?(:post_messages, room)
      return unless agent.user.webhook
      return unless event.webhook_status == "none"

      event.update!(webhook_status: "pending", webhook_next_attempt_at: Time.current)
      Agent::EventWebhookJob.perform_later(event.id, event.webhook_attempts.to_i)
    end

    # Posts a slash-command invocation to the agent's webhook. The payload
    # carries the same additive agent key as other deliveries plus the
    # event type, the invoking user, the room, the thread id when invoked
    # in a thread (so the agent can reply in place), and the command name
    # with its raw arguments. Response bodies are ignored.
    def post_slash_command_webhook!(webhook, event, agent:)
      metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
      payload = {
        agent: { id: agent.id, name: agent.user.name, owner: agent.owner&.name, delivery_id: event.id },
        event_type: event.event_type,
        user: event.actor ? { id: event.actor.id, name: event.actor.name } : nil,
        room: event.room ? { id: event.room.id, name: event.room.name } : nil,
        thread_id: metadata["thread_id"],
        command: { name: metadata["command"], arguments: metadata["arguments"] }
      }.compact.to_json

      webhook.post_payload(payload, secret: agent.ensure_webhook_signing_secret!)
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

    # Posts a Fizzy write-action result to the agent's webhook. The payload
    # carries the same additive agent key as approval decisions plus a
    # fizzy_action key with the completion fields. Response bodies are
    # ignored: a completion notification never creates a reply message.
    def post_fizzy_action_webhook!(webhook, event, agent:)
      metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
      payload = {
        agent: { id: agent.id, name: agent.user.name, owner: agent.owner&.name, delivery_id: event.id },
        fizzy_action: {
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
    # Fizzy action, and work events carry their own payloads and ignore
    # response bodies. Raises UndeliverableWebhook when the payload cannot be
    # built, RetryableWebhookResponse on a 429, 408, or 5xx answer,
    # PermanentWebhookResponse on any other non-2xx answer, and lets
    # transport errors propagate for the caller to retry. Only a 2xx
    # response counts as delivered.
    def post_event_webhook!(webhook, event, agent:)
      response = case event.event_type
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
      when *AgentEvent::SLASH_DELIVERABLE_TYPES
        post_slash_command_webhook!(webhook, event, agent: agent)
      when "fizzy_action_completed"
        post_fizzy_action_webhook!(webhook, event, agent: agent)
      when *AgentEvent::WORK_DELIVERABLE_TYPES
        metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
        thread = ChannelThread.find_by(id: metadata["thread_id"])
        work = if thread
          work_payload(thread, assigned_by: metadata["assigned_by"], agent: agent)
        else
          metadata["work_snapshot"]
        end
        raise UndeliverableWebhook, "Thread no longer available" unless work

        post_work_webhook!(webhook, event, work: work, agent: agent)
      else
        raise UndeliverableWebhook, "Event type #{event.event_type} has no webhook payload"
      end

      check_webhook_response!(response)
      response
    end

    # Runs inside Agent::DeliveryJob. Re-checks everything at perform time:
    # grant, membership, rate, hop, and message existence. Marks the row
    # delivered and enqueues its webhook after the claim commits when one
    # is configured, or records a suppression. Idempotent: settled rows
    # are left alone, and the webhook job posts at most once per row. An
    # agent that acked the row by polling before this ran still gets its
    # webhook: acking marks polling state only and never cancels a
    # webhook still owed.
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
      room = Room.alive.find_by(id: event.room_id)
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
      webhook_next_attempt_at = Time.current if webhook_status == "pending"
      return unless claim(event, outcome: "delivered", webhook_status: webhook_status,
        webhook_next_attempt_at: webhook_next_attempt_at)

      enqueue_webhook_after_commit(event.id, event.webhook_attempts.to_i) if webhook_status == "pending"
    end

    # Webhook half of an acknowledged row: the agent already read it by
    # polling, so no grant or membership re-check applies, but a
    # configured webhook is still owed exactly one POST.
    def enqueue_owed_webhook(event)
      return unless event.agent.user.webhook
      return unless AgentEvent.where(id: event.id, outcome: "acknowledged", webhook_status: "none")
        .update_all(webhook_status: "pending", webhook_next_attempt_at: Time.current) == 1

      enqueue_webhook_after_commit(event.id, event.webhook_attempts.to_i)
    end

    # Re-enqueues webhook rows stranded in pending with attempts
    # remaining: the row write committed but its enqueue never ran (a
    # crash or a Redis outage in between), or a retry's re-enqueue was
    # lost. Runs from the periodic runner (bin/periodic). A row is
    # stranded only once its scheduled attempt is past by the grace
    # period, so a retry waiting out Retry-After is never re-enqueued
    # early; rows that were never scheduled (no next attempt recorded)
    # fall back to their age. A row still waiting on its scheduled
    # retry past the grace period may post twice; delivery is
    # at-least-once, so receivers already tolerate redelivery, and the
    # webhook job's attempt claim keeps the duplicate from burning an
    # extra attempt.
    def recover_stranded_webhooks!(now: Time.current)
      recover_stranded_deliveries!(now: now)
      fail_exhausted_webhooks!(now: now)
      grace = now - STRANDED_WEBHOOK_AFTER
      AgentEvent.where(webhook_status: "pending")
        .where("agent_events.webhook_attempts < ?", Agent::EventWebhookJob::MAX_ATTEMPTS)
        .where(
          "((agent_events.webhook_next_attempt_at IS NULL AND agent_events.created_at < :grace) " \
            "OR agent_events.webhook_next_attempt_at < :grace)",
          grace: grace
        )
        .find_each do |event|
          begin
            Agent::EventWebhookJob.perform_later(event.id, event.webhook_attempts.to_i)
            # Conditional on the values just read: a worker that ran after
            # the sweep's read (a 429 storing a Retry-After delay, a
            # delivery, a failure) changed the attempts or the next
            # attempt, so this write backs off instead of clobbering it.
            # The stale enqueue above is harmless: its attempt number no
            # longer matches and the job exits on its claim.
            AgentEvent.where(id: event.id, webhook_status: "pending",
                webhook_attempts: event.webhook_attempts,
                webhook_next_attempt_at: event.webhook_next_attempt_at)
              .update_all(webhook_next_attempt_at: Time.current)
          rescue => error
            Rails.logger.error "Stranded webhook recovery failed for event #{event.id}: #{error.class}: #{error.message}"
          end
        end
    end

    # Fails pending webhook rows that spent every attempt without
    # recording an outcome, once they have been past the stranded grace
    # for EXHAUSTED_WEBHOOK_AFTER. Without this the attempts filter in
    # the recovery sweep above hides them and they sit pending forever.
    def fail_exhausted_webhooks!(now: Time.current)
      cutoff = now - STRANDED_WEBHOOK_AFTER - EXHAUSTED_WEBHOOK_AFTER
      AgentEvent.where(webhook_status: "pending")
        .where("agent_events.webhook_attempts >= ?", Agent::EventWebhookJob::MAX_ATTEMPTS)
        .where(
          "((agent_events.webhook_next_attempt_at IS NULL AND agent_events.created_at < :cutoff) " \
            "OR agent_events.webhook_next_attempt_at < :cutoff)",
          cutoff: cutoff
        )
        .update_all(webhook_status: "failed",
          webhook_last_error: "Delivery attempts exhausted without a recorded outcome")
    end

    # Re-enqueues message deliveries stranded in pending past the grace
    # period: the row write committed but its DeliveryJob enqueue never
    # ran. Runs from the same periodic sweep as the webhook recovery
    # above. The delivery job re-checks grants and membership at
    # perform time, so a recovered row still suppresses correctly when
    # access went away while it was stranded.
    def recover_stranded_deliveries!(now: Time.current)
      AgentEvent.message_deliverable.where(outcome: "pending")
        .where("agent_events.created_at < ?", now - STRANDED_WEBHOOK_AFTER)
        .find_each do |event|
          begin
            Agent::DeliveryJob.perform_later(event.id)
          rescue => error
            Rails.logger.error "Stranded delivery recovery failed for event #{event.id}: #{error.class}: #{error.message}"
          end
        end
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

      # Rows where the actor is the recipient (an agent assigning work
      # to itself) never trigger hops, so self-assignment cannot raise
      # the agent's own chain.
      def hop_trigger_for(agent)
        agent.agent_events.where(event_type: AgentEvent::HOP_TRIGGER_TYPES, outcome: AgentEvent::HOP_TRIGGER_OUTCOMES)
          .where("agent_events.created_at >= ?", TRIGGER_WINDOW.ago)
          .where("agent_events.actor_id IS NULL OR agent_events.actor_id != ?", agent.user_id)
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

      # Only a 2xx answer delivers. A 429, 408, or 5xx is worth
      # retrying, carrying the endpoint's Retry-After delay when it
      # sent a parseable one; anything else non-2xx (a wrong URL, a
      # refused payload, an unhandled redirect) fails permanently.
      def check_webhook_response!(response)
        code = response.code.to_i
        return if code.between?(200, 299)

        if code == 429 || code == 408 || code >= 500
          raise RetryableWebhookResponse.new("Webhook endpoint returned #{code}", retry_after: parse_retry_after(response))
        else
          raise PermanentWebhookResponse, "Webhook endpoint returned #{code}"
        end
      end

      # Retry-After arrives as delay seconds or an HTTP date. Unparseable
      # values mean no hint; valid ones clamp into 0..RETRY_AFTER_MAX so
      # a hostile header cannot stall a delivery for days.
      def parse_retry_after(response)
        value = response["Retry-After"].to_s.strip
        return if value.empty?

        delay = if value.match?(/\A\d+\z/)
          value.to_i
        else
          begin
            Time.httpdate(value) - Time.current
          rescue ArgumentError, TypeError
            return
          end
        end

        [ [ delay, 0 ].max, RETRY_AFTER_MAX ].min
      end

      # The Redis enqueue waits for the row write to commit, so a
      # rolled-back claim never leaves a job behind. (A crash between the
      # commit and the enqueue still strands the row; the periodic sweep
      # picks those up.) The attempt travels with the job so a duplicate
      # enqueue finds its number stale and exits without posting.
      def enqueue_webhook_after_commit(event_id, attempt)
        ActiveRecord.after_all_transactions_commit do
          AgentEvent.where(id: event_id, webhook_next_attempt_at: nil)
            .update_all(webhook_next_attempt_at: Time.current)
          Agent::EventWebhookJob.perform_later(event_id, attempt)
        end
      end

      # Atomically transitions a pending row; false when another job
      # already claimed it, so two concurrent jobs cannot both post and the
      # loser exits without posting or writing a duplicate suppression row.
      def claim(event, outcome:, detail: nil, webhook_status: nil, webhook_next_attempt_at: nil)
        updates = { outcome: outcome }
        updates[:detail] = detail unless detail.nil?
        updates[:webhook_status] = webhook_status unless webhook_status.nil?
        updates[:webhook_next_attempt_at] = webhook_next_attempt_at unless webhook_next_attempt_at.nil?
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
