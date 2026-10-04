require "json"
require "socket"
require "active_support/testing/time_helpers"

class HuddleCleanupOracle
  include ActiveSupport::Testing::TimeHelpers

  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ENV["LIVEKIT_INTERNAL_URL"] = "http://127.0.0.1:52313"
    ENV["LIVEKIT_API_KEY"] = "ws13-fixture-api-key"
    ENV["LIVEKIT_API_SECRET"] = "ws13-fixture-api-secret"
    enqueues = []
    Huddle::CleanupJob.define_singleton_method(:perform_later) { |id| enqueues << id }
    server = TCPServer.new("127.0.0.1", 52313)
    replies = Queue.new
    worker = Thread.new do
      loop do
        socket = server.accept
        headers = []
        while (line = socket.gets) && line != "\r\n"
          headers << line
        end
        length = headers.find { |h| h.downcase.start_with?("content-length:") }.to_s.split(":", 2).last.to_i
        socket.read(length) if length > 0
        status = replies.pop
        socket.write("HTTP/1.1 #{status} Fixture\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
        socket.close
      end
    rescue IOError, Errno::EBADF
    end
    now = Time.utc(2026, 1, 1, 12)
    travel_to now
    states = []
    cleanup = HuddleCleanup.create!(operation: :remove_participant, room_name: "opaque-room", identity: "opaque-participant")
    capture = lambda do |name, outcome|
      states << { name: name, result: outcome, row: cleanup.reload.attributes.slice(
        "attempts", "completed_at", "created_at", "enqueued_at", "last_attempted_at", "next_attempt_at", "updated_at"
      ).transform_values { |v| v.respond_to?(:utc) ? v.utc.strftime("%Y-%m-%d %H:%M:%S") : v } }
    end
    capture.call("created", nil)
    capture.call("lease_blocks_reconcile", cleanup.perform!)
    replies << 503
    capture.call("queued_failure", cleanup.perform_from_queue!)
    capture.call("duplicate_queue_delivery", cleanup.perform_from_queue!)
    8.times do |index|
      travel_back; travel_to cleanup.reload.next_attempt_at
      replies << 503
      capture.call("retry_#{index + 2}", cleanup.perform!)
    end
    travel_back; travel_to cleanup.reload.next_attempt_at
    replies << 404
    capture.call("already_removed_success", cleanup.perform!)
    capture.call("completed_noop", cleanup.perform!)
    puts JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: now.to_i, enqueues: enqueues.size, states: states })
  ensure
    travel_back
    server&.close
    worker&.kill
    worker&.join
  end
end
HuddleCleanupOracle.new.run
