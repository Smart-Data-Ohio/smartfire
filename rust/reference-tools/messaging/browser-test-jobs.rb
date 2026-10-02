# Match ActiveJob::TestHelper#before_setup in pinned test_helper.rb:13.
# Ordinary behaviour checks keep the production adapter and workers.
Rails.application.config.after_initialize do
  if ENV["WS8BM_TEST_JOB_ADAPTER"] == "1"
    ActiveJob::Base.queue_adapter = ActiveJob::QueueAdapters::TestAdapter.new
    warn "WS8bm reference jobs: ActiveJob::TestAdapter; perform_enqueued_jobs=false"
  end
end
