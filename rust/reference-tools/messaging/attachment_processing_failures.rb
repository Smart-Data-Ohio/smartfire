# Run through attachment_processing.sh: pinned Rails/media with the approved #226 overlay.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
ActiveJob::Base.logger = Rails.logger

travel_to Time.utc(2026, 3, 2, 16) do
  queue = ActiveJob::Base.queue_adapter
  server = ActionCable.server
  original_broadcast = server.method(:broadcast)
  seed = ActiveStorage::Blob.find(9)
  blob = ActiveStorage::Blob.create_and_upload!(io: StringIO.new(seed.download), filename: seed.filename, content_type: seed.content_type)
  message = Message.create!(creator_id: 127326141, room_id: 486777696, attachment: blob, client_message_id: "completion-failure")
  queue.enqueued_jobs.clear
  message.process_attachment_later
  job = queue.enqueued_jobs.find { |j| j[:job] == Message::AttachmentProcessingJob }
  queue.enqueued_jobs.clear
  server.define_singleton_method(:broadcast) { |*, **| raise "simulated temporary broadcast failure" }
  ActiveJob::Base.execute(job)
  retries = queue.enqueued_jobs.select { |j| j[:job] == Message::AttachmentProcessingJob }
  result = {
    completion_failure: {
      preview_attached: blob.reload.preview_image.attached?,
      retry_jobs: retries.length,
      retry_executions: retries.map { |j| j["executions"] },
      lease: blob.attributes.slice("message_processing_token", "message_processing_expires_at")
    }
  }
  server.define_singleton_method(:broadcast, original_broadcast)

  trigger = Message.create!(creator_id: 127326141, room_id: 486777696, body: "Root webhook trigger")
  blob = ActiveStorage::Blob.create_and_upload!(io: StringIO.new("corrupt MOV"), filename: "attachment.mov", content_type: "video/quicktime")
  queue.enqueued_jobs.clear
  frames = []
  server.define_singleton_method(:broadcast) { |channel, payload| frames << { channel: channel, payload: payload } }
  Webhook.find_by!(user_id: 394959859).send(:receive_attachment_reply, trigger, attachment: blob)
  jobs = queue.enqueued_jobs.select { |j| j[:job] == Message::AttachmentProcessingJob }
  result[:root_webhook] = {
    processing_jobs: jobs.length,
    preview_attached: blob.reload.preview_image.attached?,
    token_suffix: blob.message_processing_token&.split(":")&.last,
    append_without_poster: frames.any? { |f| f[:payload].is_a?(String) && f[:payload].include?("<video") && !f[:payload].include?("poster=") }
  }
  puts JSON.pretty_generate(result)
end
