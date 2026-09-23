class Calendar::WatchChannelJob < ApplicationJob
  # Idempotent: re-opens the member's push channel. Transient Google
  # failures retry with backoff; permanent ones are recorded on the
  # channel and the next renewal retries.
  retry_on Google::Client::Unavailable, wait: :polynomially_longer, attempts: 8

  def perform(user_id)
    user = User.find_by(id: user_id)
    Calendar::PushChannel.watch_for!(user) if user
  end
end
