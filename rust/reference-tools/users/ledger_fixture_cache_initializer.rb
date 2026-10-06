# Original test_helper cache cleanup, inside the actual rendering process.
# The file is private fixture IPC in an isolated seed; no HTTP control route.
if Rails.env.test?
  Rails.application.config.active_storage.service = :local
  # The class-specific flag is applied before browser class setup. Reading it
  # through both controller APIs also drives the real form/meta token producer.
  Rails.application.config.action_controller.allow_forgery_protection = true
  class << ActionController::Base
    def allow_forgery_protection
      !File.exist?('/rails/storage/db/ledger-forgery-off')
    end
  end
  module LedgerOriginalForgeryPolicy
    def allow_forgery_protection
      !File.exist?('/rails/storage/db/ledger-forgery-off')
    end
  end
  ActionController::Base.prepend(LedgerOriginalForgeryPolicy)
end
class LedgerFixtureCacheReset
  def initialize(app)
    @app, @generation, @mutex = app, nil, Mutex.new
  end

  def call(env)
    path = '/rails/storage/db/ledger-cache-generation'
    generation = File.read(path) if File.exist?(path)
    @mutex.synchronize do
      if generation && generation != @generation
        Icons.expire_custom_cache!
        Rails.cache.clear
        ActionController::Base.cache_store.clear
        ActionCable.server.pubsub.clear if ActionCable.server.pubsub.respond_to?(:clear)
        adapter = ActiveJob::Base.queue_adapter
        adapter.enqueued_jobs.clear
        adapter.performed_jobs.clear
        @generation = generation
        File.write('/rails/storage/db/ledger-cache-renderer-ack', generation)
      end
    end
    @app.call(env)
  end
end
Rails.application.config.middleware.insert_before(0, LedgerFixtureCacheReset)
