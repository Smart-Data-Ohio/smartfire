# Shared legacy-bot webhook fan-out for newly posted root messages.
# Agent-backed bots are delivered only through Agent::DeliveryJob (see
# Message::AgentDelivery); the legacy webhook bypasses grant and rate
# checks, so it serves bots without an Agent row only. The hop limit
# still applies: a chain that reached it stops here instead of
# looping through a legacy bot.
module Messages::BotWebhooks
  extend ActiveSupport::Concern

  private
    def deliver_webhooks_to_bots(message, room: @room)
      bots = bots_eligible_for_webhook(message, room).excluding(message.creator).where.missing(:agent)
      return if bots.empty?
      return if Agent::Delivery.hop_for_message(message) >= Agent::Delivery::HOP_LIMIT

      bots.each { |bot| bot.deliver_webhook_later(message) }
    end

    def bots_eligible_for_webhook(message, room)
      room.direct? ? room.users.active_bots : message.mentionees.active_bots
    end
end
