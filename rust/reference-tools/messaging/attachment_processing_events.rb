# Pinned Rails with #226's approved overlay, via attachment_processing.sh.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
ActiveJob::Base.logger = Rails.logger

travel_to Time.utc(2026, 3, 2, 16) do
  queue = ActiveJob::Base.queue_adapter
  agent = Agent.find(773018776)
  AgentGrant.where(agent_id: agent.id).delete_all
  blob = ActiveStorage::Blob.create_and_upload!(io: StringIO.new("corrupt MOV"), filename: "attachment.mov", content_type: "video/quicktime")
  message = Message.create!(creator: agent.user, room_id: 486777696, attachment: blob, client_message_id: "agent-step-recovery")
  frames = []
  ActionCable.server.define_singleton_method(:broadcast) { |channel, payload| frames << { channel: channel, payload: payload } }
  reset = -> {
    blob.update_columns(message_processing_token: nil, message_processing_expires_at: nil)
    queue.enqueued_jobs.clear
    frames.clear
  }
  snapshot = -> {
    blob.reload
    video_frames = frames.select { |f| f[:payload].is_a?(String) && f[:payload].include?("<video") }
    {
      processing_jobs: queue.enqueued_jobs.count { |j| j[:job] == Message::AttachmentProcessingJob },
      preview_attached: blob.preview_image.attached?,
      token_suffix: blob.message_processing_token&.split(":")&.last,
      expires_in: blob.message_processing_expires_at && (blob.message_processing_expires_at - Time.current).to_i,
      video_frames: video_frames.length,
      poster_present: video_frames.any? { |f| f[:payload].include?("poster=") }
    }
  }
  result = {}
  reset.call
  created = Agents::Steps.create(agent: agent, fields: { "message_id" => message.id, "name" => "Inspect video" })
  raise created.error unless created.status == :created
  result[:step_create] = snapshot.call.merge(http_status: Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(created.status))
  reset.call
  updated = Agents::Steps.update(agent: agent, id: AgentStep.where(message: message).sole.id, fields: { "status" => "done" })
  raise updated.error unless updated.status == :ok
  result[:step_update] = snapshot.call.merge(http_status: Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(updated.status))

  # The notifier and digest dispatchers render the same full message partial as
  # broadcast_create. Their usual producers have no attachment, but the render seam
  # must retain recovery if it receives a video-bearing message.
  reset.call
  message.broadcast_create
  result[:full_append] = snapshot.call
  reset.call
  message.update_column(:system_note, true)
  message.broadcast_create
  result[:system_note] = snapshot.call
  reset.call
  message.update_column(:system_note, false)
  message.broadcast_replace_to message.conversation, :messages,
    target: ActionView::RecordIdentifier.dom_id(message, :thread_indicator),
    partial: "messages/thread_indicator", locals: { message: message, reply_count: 1 }
  result[:thread_indicator] = snapshot.call
  puts JSON.pretty_generate(result)
end
