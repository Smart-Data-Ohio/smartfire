class Room::PushMessageJob < ApplicationJob
  discard_on ActiveJob::DeserializationError

  def perform(room, message)
    Room::MessagePusher.new(room:, message:).push
  end
end
