# Parent association reuse while rendering old, open board posts in pinned Rails.
require 'json'
require 'digest'
ActiveRecord::Base.logger = nil
ActiveJob::Base.queue_adapter = :test
rows = [2, 12].map do |count|
  observed = nil
  ActiveRecord::Base.transaction do
    board = Room.find(699448332)
    ids = count.times.map do |i|
      post = board.channel_threads.create!(creator_id:127326141, name:"Query probe #{i}", work_status:'planned')
      post.update_columns(last_activity_at:Time.current - 7.days)
      post.id
    end
    posts = board.channel_threads.where(id:ids).order(:id).to_a
    queries = []
    listener = ->(*, payload) { queries << payload[:sql] if payload[:sql].match?(/FROM "rooms"/) }
    states = nil
    ActiveSupport::Notifications.subscribed(listener, 'sql.active_record') { states = posts.map(&:status) }
    observed = {posts:count, room_reads:queries.size, states:}
    raise "board association unexpectedly loaded rooms: #{queries}" unless queries.empty?
    raise ActiveRecord::Rollback
  end
  observed
end
sources = %w[app/models/channel_thread.rb app/models/room.rb].to_h { |f| [f, Digest::SHA256.file(Rails.root.join(f)).hexdigest] }
puts JSON.pretty_generate(reference:'d7c7de92', now:Time.current.utc.iso8601(6), sources:, rows:)
rows.each { |r| warn "Pinned Rails board lifecycle: #{r[:posts]} aged posts; #{r[:room_reads]} Room SELECTs" }
