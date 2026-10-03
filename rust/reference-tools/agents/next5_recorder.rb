# Real Recorder transactions, persisted recipients, callbacks and uncached SQL.
require "json"
require "digest"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
hashes = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/generic-recorder-source-hashes.json")))
hashes.each { |path, hash| raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash }
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream, payload, **| frames << {stream: stream, payload: payload} }
rows = []
travel_to Time.utc(2026, 3, 2, 16) do
  [10, 100].each do |size|
    ids = (0...size).map { |index| 2115000000 + index }
    User.insert_all!(ids.map { |id| {id: id, name: "Recorder recipient", role: 0, status: 0, created_at: Time.current, updated_at: Time.current} })
    # Warm model/adapter schema discovery equally before both measured sizes.
    # No request/result query is cached; the measured boundary remains uncached.
    ActivityItems::Recorder.record!(recipient: User.find(ids.first), source: Room.find(486777696), event_type: "work_update", skip_source_check: true)
    ActivityItem.delete_all
    ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=2115100000 WHERE name='activity_items'")
    %w[create repeat].each do |phase|
      frames.clear
      queries = []
      cache_hits = 0
      execution = Rails.application.executor.run!(reset: true)
      begin
        ActiveRecord::Base.uncached do
          callback = ->(*args) do
            event = ActiveSupport::Notifications::Event.new(*args)
            sql = event.payload[:sql]
            if sql.match?(/\A\s*SELECT/i)
              cache_hits += 1 if event.payload[:cached]
              queries << sql unless event.payload[:cached]
            end
          end
          ActiveSupport::Notifications.subscribed(callback, "sql.active_record") do
            ActiveRecord::Base.transaction do
              source = Room.find(486777696)
              recipients = User.where(id: ids).order(:id).to_a
              recipients.each { |user| ActivityItems::Recorder.record!(recipient: user, source: source, event_type: "work_update", skip_source_check: true) }
            end
          end
        end
      ensure
        execution.complete!
      end
      raise "query cache enabled" unless cache_hits.zero?
      facts = ActivityItem.order(:user_id).map do |item|
        {id: item.id, user: item.user_id, source_type: item.source_type, source_id: item.source_id, event: item.event_type,
          read_at: item.read_at&.to_i, handled_at: item.handled_at&.to_i, created_at: item.created_at.to_i, updated_at: item.updated_at.to_i}
      end
      rows << {size: size, phase: phase, selects: queries.size, cache_hits: cache_hits, items: facts, frames: frames.dup}
      warn "WS11_RECORDER_RAILS size=#{size} phase=#{phase} SELECTs=#{queries.size} cache_hits=#{cache_hits}"
    end
    User.where(id: ids).delete_all
  end
end
puts JSON.pretty_generate({reference: "d7c7de92", rows: rows})
