class Message::AttachmentProcessingJob < ApplicationJob
  retry_on StandardError, wait: :polynomially_longer, attempts: 3
  discard_on ActiveJob::DeserializationError

  def perform(message)
    return unless message.attachment?

    message.process_attachment_now
    # Analysis touches the message through its blob, but generating a
    # preview on an already analyzed video may not. Expire its cached
    # presentation in either case, then refresh the room or thread stream.
    message.touch
    message.broadcast_replace_to message.message_stream_target, :messages,
      target: [ message, :presentation ], partial: "messages/presentation",
      attributes: { maintain_scroll: true }
  end
end
