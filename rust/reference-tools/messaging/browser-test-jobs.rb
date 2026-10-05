# Match ActiveJob::TestHelper#before_setup in pinned test_helper.rb:13.
# Ordinary behaviour checks keep the production adapter and workers.
Rails.application.config.after_initialize do
  # workspace_markdown_test.rb setup explicitly enables forgery protection.
  ActionController::Base.allow_forgery_protection = true if ENV["WS8BM_TEST_FORGERY_PROTECTION"] == "1"
  if ENV["WS8BM_TEST_JOB_ADAPTER"] == "1"
    ActiveJob::Base.queue_adapter = ActiveJob::QueueAdapters::TestAdapter.new
    raise "unexpected job execution" unless ActiveJob::Base.queue_adapter.perform_enqueued_jobs.nil?
    warn "WS8bm reference jobs: ActiveJob::TestAdapter; perform_enqueued_jobs=nil"
  end
  if ENV["WS8BM_NATIVE_MEDIA"] == "1"
    # Relocate the test Disk service into the fixture's mounted storage. The
    # original native service.exist? assertions read these same real files.
    ActiveStorage::Blob.services.fetch("test").instance_variable_set(:@root, Rails.root.join("storage/files").to_s)
  end
end

# Tools-only server boundary for the original test's explicit job execution.
# ActiveJob::TestHelper normally shares the test server's process. Here the
# Selenium driver runs separately, so dispatch its request to that same queue.
class Ws8bmAttachmentJobs
  def initialize(app)
    @app = app
  end
  def call(env)
    return @app.call(env) unless ENV["WS8BM_NATIVE_MEDIA"] == "1" && env["REQUEST_METHOD"] == "POST" && env["PATH_INFO"] == "/__ws8bm__/attachment-processing"
    adapter = ActiveJob::Base.queue_adapter
    raise "wrong adapter" unless adapter.is_a?(ActiveJob::QueueAdapters::TestAdapter)
    selected = adapter.enqueued_jobs.select { |job| job[:job] == Message::AttachmentProcessingJob }
    raise "no attachment processing job" if selected.empty?
    selected.each do |job|
      adapter.enqueued_jobs.delete(job)
      adapter.performed_jobs << job
      ActiveJob::Base.execute(job)
    end
    [200, {"content-type" => "application/json"}, [JSON.generate(performed: selected.size, only: "Message::AttachmentProcessingJob")]]
  end
end
Rails.application.config.middleware.use Ws8bmAttachmentJobs
