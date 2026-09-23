class BounceMailbox < ApplicationMailbox
  # Catches mail no other route matches (anything without a room-
  # token recipient). Marks it bounced instead of raising RoutingError,
  # so misaddressed mail is recorded as refused with no job failure and
  # no reply sent: this app has no outbound mail, and answering
  # misaddressed (usually spam) mail would backscatter to spoofed
  # senders. Unknown room- tokens still route to RoomMailbox, which
  # drops them silently so token probes learn nothing.
  def process
    inbound_email.bounced!
  end
end
