module Message::BotWebhookFanout
  # Legacy webhook fan-out for a newly posted message, shared by human
  # posts and agent posts. Agent-backed bots are delivered only through
  # Agent::DeliveryJob (see Message::AgentDelivery); the legacy webhook
  # bypasses grant and rate checks, so it serves bots without an Agent
  # row only. The hop limit still applies: a chain that reached it stops
  # here instead of looping through a legacy bot.
  def self.deliver_for(message)
    room = message.room
    bots = (room.direct? ? room.users.active_bots : message.mentionees.active_bots)
      .excluding(message.creator).where.missing(:agent)
    return if bots.empty?
    return if Agent::Delivery.hop_for_message(message) >= Agent::Delivery::HOP_LIMIT

    bots.each { |bot| bot.deliver_webhook_later(message) }
  end
end
