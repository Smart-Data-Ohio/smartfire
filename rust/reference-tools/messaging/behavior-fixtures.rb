# Browser fixtures are created by pinned Rails, then copied unchanged to both apps.
# Source: test/system/message_list_a11y_test.rb at d7c7de92.
kind = ARGV.fetch(0)
# The production parity seed has many extra provider/state examples. The
# pinned list regressions use just the three original Designers fixtures.
if %w[message_list history].include?(kind)
  Room.find(654632876).root_messages.where.not(id: [309456473, 908005739, 607264868]).destroy_all
end
case kind
when "message_list"
  # No additional rows are needed.
when "history"
  count = Message::PAGE_SIZE + 5
  first_created_at = count.seconds.ago
  count.times do |index|
    Message.create!(room: Room.find(654632876), creator: User.find(773523953),
      markdown_source: "History post #{index}", client_message_id: "a11y-history-#{index}",
      created_at: first_created_at + index.seconds)
  end
else
  raise "Unknown browser fixture"
end
