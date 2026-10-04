# #226 oracle: run against the pinned image with only the approved Rails files
# overlaid by attachment_processing.sh. No processing or request behavior is patched.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
ActiveJob::Base.logger = Rails.logger

def processing_jobs
  ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job] == Message::AttachmentProcessingJob }
end

def lease(blob)
  blob.reload.attributes.slice("message_processing_token", "message_processing_expires_at")
end

def fresh_message(bytes: nil)
  seed = ActiveStorage::Blob.find(9)
  blob = ActiveStorage::Blob.create_and_upload!(io: StringIO.new(bytes || seed.download),
    filename: seed.filename, content_type: seed.content_type)
  @message_number = (@message_number || 0) + 1
  message = Message.create!(creator_id: 127326141, room_id: 486777696, attachment: blob,
    client_message_id: "attachment-processing-#{@message_number}")
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  [message, blob]
end

travel_to Time.utc(2026, 3, 2, 16) do
  frames = []
  ActionCable.server.define_singleton_method(:broadcast) { |channel, payload| frames << { channel: channel, payload: payload } }
  message, blob = fresh_message
  before = ApplicationController.render(partial: "messages/presentation", locals: { message: message })
  queued = processing_jobs.sole
  token = queued[:args][2]
  pending = lease(blob).merge("token_suffix" => token.split(":").last)
  pending.delete("message_processing_token")
  frames.clear
  ActiveJob::Base.execute(queued)
  after = ApplicationController.render(partial: "messages/presentation", locals: { message: message.reload })
  preview = blob.reload.preview_image.blob
  success = { pending: pending, lease: lease(blob), before: before, after: after,
    frames: frames, message: message.reload.attributes,
    blob: blob.attributes.except("key"),
    queued_job: { class: queued[:job].name, queue: queued[:queue], message_id: message.id, blob_id: blob.id, token_suffix: token.split(":").last },
    image_analysis_jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job] == ActiveStorage::AnalyzeJob }.map { |j| j[:args][0]["_aj_globalid"].split("/").last.to_i },
    source_metadata: blob.metadata, preview_metadata: preview.metadata,
    files: [blob, preview, preview.variant_records.sole.image.blob].map { |b| bytes = b.download; { content_type: b.content_type, bytes: bytes.bytesize, sha256: Digest::SHA256.hexdigest(bytes) } } }

  # Use the same atomic claim as enqueue/job execution, including renewal by its owner.
  message, blob = fresh_message
  claims = %w[first:0 second:0].map { |t| message.claim_attachment_processing(blob, t) }
  renewal = message.claim_attachment_processing(blob, "first:0")
  active = lease(blob)
  travel 15.minutes
  expired = message.claim_attachment_processing(blob, "second:0")
  message.release_attachment_processing(blob.id, "first:0")
  stale_release = lease(blob)
  message.release_attachment_processing(blob.id, "second:0")
  claim = { claims: claims, renewal: renewal, active: active, expired: expired,
    stale_release: stale_release, released: lease(blob) }

  message, blob = fresh_message(bytes: "corrupt MOV")
  message.process_attachment_later
  retries = []
  srand(226)
  3.times do
    job = processing_jobs.last
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    begin
      ActiveJob::Base.execute(job)
    rescue => error
      retries << { terminal: error.class.name }
    end
    if (retry_job = processing_jobs.last)
      retries << { executions: retry_job["executions"], wait: retry_job[:at] - Time.now.to_f }
      travel_to Time.at(retry_job[:at])
    end
  end
  terminal = lease(blob)
  3.times { message.recover_attachment_preview }
  terminal_jobs = processing_jobs.size
  terminal_html = ApplicationController.render(partial: "messages/presentation", locals: { message: message.reload })

  message, blob = fresh_message
  delays = 6.times.map do |n|
    token = "refused:#{n.clamp(0, 5)}"
    message.claim_attachment_processing(blob, token)
    message.defer_attachment_processing(blob.id, token)
    state = lease(blob)
    seconds = blob.message_processing_expires_at - Time.current
    travel seconds
    state.merge("delay" => seconds)
  end
  puts JSON.pretty_generate(success: success, claim: claim, retries: retries,
    terminal: terminal, terminal_jobs: terminal_jobs, terminal_html: terminal_html, enqueue_cooldowns: delays)
end
