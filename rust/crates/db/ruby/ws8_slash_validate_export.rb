# Read and edit Rust-written slash rows through the pinned Rails models.
require "json"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter = :test
checked = 0
Dir[File.join(ARGV.fetch(0), "case-*.json")].sort.each do |path|
  meta = JSON.parse(File.read(path))
  database = path.sub(/\.json\z/, ".sqlite3")
  config = ActiveRecord::Base.connection_db_config.configuration_hash.merge(database: database)
  ActiveRecord::Base.establish_connection(config)
  travel_to Time.zone.parse(meta.fetch("now")), with_usec: true
  user = User.find(meta.fetch("user_id"))
  raise "User invalid: #{user.errors.full_messages}" unless user.valid?
  if meta["user"]
    meta["user"].each do |key,expected|
      actual = user.public_send(key)
      actual = actual.utc.strftime("%Y-%m-%d %H:%M:%S.%6N").sub(/\.000000\z/, "") if actual.is_a?(Time) || actual.is_a?(ActiveSupport::TimeWithZone)
      raise "User #{key} mismatch" unless actual == expected
    end
  end
  if meta["message_id"]
    message = Message.find(meta.fetch("message_id"))
    [message, message.rich_text_body, message.room, message.thread,
      SavedItem.find_by(message: message, user: user)].compact.each do |record|
      raise "#{record.class}: #{record.errors.full_messages}" unless record.valid?
    end
    raise "rich text mismatch" unless message.body.body.to_html == meta.fetch("body")
    index = ActiveRecord::Base.connection.select_value("SELECT body FROM message_search_index WHERE rowid=#{message.id}")
    raise "index mismatch" unless index == meta.fetch("index")
    Message.transaction do
      message.update!(markdown_source: "#{message.markdown_source}\n\nRails rollback check")
      message.reload
      raise "edit failed" unless message.markdown_source.end_with?("Rails rollback check")
      raise ActiveRecord::Rollback
    end
  else
    User.transaction do
      user.update!(custom_status_text: "Rails rollback check")
      raise "edit failed" unless user.reload.custom_status_text == "Rails rollback check"
      raise ActiveRecord::Rollback
    end
  end
  checked += 1
end
raise "No exported slash scenarios" unless checked.positive?
puts "WS8 slash Rails rollback: #{checked} Rust-written scenarios read, validated and edited"
