class Message::StreamTrailingBroadcastJob < ApplicationJob
  # Trailing edge of the stream broadcast throttle: when an append's
  # broadcast coalesces away, the throttle enqueues this job, which waits
  # out the rest of the window and then sends the latest text — unless
  # the stream finalized or a newer broadcast landed first. Queued jobs
  # carry the last broadcast stamp they saw; only the job whose stamp is
  # still current broadcasts, so a burst of coalesced appends still sends
  # exactly one trailing broadcast. Idempotent: a repeat run finds its
  # stamp stale and stays quiet.
  def perform(message_id, last_broadcast_at)
    wait_until = Time.zone.parse(last_broadcast_at.to_s) + Message::STREAM_BROADCAST_INTERVAL
    wait = wait_until - Time.current
    sleep(wait) if wait.positive?

    message = Message.find_by(id: message_id)
    return if message.nil? || !message.streaming?
    return if message.stream_broadcast_at&.utc&.iso8601(6) != last_broadcast_at

    message.broadcast_stream_update
  end
end
