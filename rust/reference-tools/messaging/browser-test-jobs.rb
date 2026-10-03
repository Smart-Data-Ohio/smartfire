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
end
