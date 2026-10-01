# Browser fixtures are created by pinned Rails, then copied unchanged to both apps.
# Source: test/system/message_list_a11y_test.rb at d7c7de92.
kind = ARGV.fetch(0)
# The production parity seed has many extra provider/state examples. The
# pinned list regressions use just the three original Designers fixtures.
if %w[message_list history].include?(kind) || kind.start_with?("unread-")
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
when "search"
  room = Room.find(654632876)
  creator = User.find(773523953)
  room.messages.create!(creator:, markdown_source: "system paging alpha",
    client_message_id: "system-search-alpha", created_at: 1.hour.ago)
  41.times do |index|
    room.messages.create!(creator:, markdown_source: "system paging filler #{index}",
      client_message_id: "system-search-filler-#{index}")
  end
when "forward"
  Room.find(654632876).messages.create!(creator: User.find(773523953),
    markdown_source: "| Keep |\n| --- |\n| row |\n\n```ruby\nputs :forwarded\n```",
    client_message_id: "system-forward-source")
when /^unread-/
  room = Room.find(654632876)
  if kind == "unread-many"
    # The original short history fits in the viewport and cannot prove that
    # initial scrolling happened. Keep seven incoming posts, with read history
    # loaded beforehand, so the scroll assertion distinguishes a broken jump.
    12.times do |index|
      room.root_messages.create!(creator: User.find(773523953),
        markdown_source: "Read history #{index}\nsecond line\nthird line",
        client_message_id: "divider-read-history-#{index}", created_at: 1.minute.ago + index.seconds)
    end
  end
  membership = room.memberships.find_by!(user_id: 773523953)
  membership.read
  membership.update_columns(connected_at: nil, connections: 0)
  first = nil
  bodies = case kind
  when "unread-few" then ["Fresh one", "Fresh two"]
  when "unread-many" then 7.times.map { |i| "Catch-up #{i}" }
  when "unread-pill" then 25.times.map { |i| "Pill #{i}\nsecond line" }
  when "unread-offpage" then ["First unread off page"] + (Message::PAGE_SIZE + 1).times.map { |i| "Later #{i}" }
  when "unread-menu" then []
  else raise "Unknown unread fixture"
  end
  bodies.each_with_index do |body, index|
    message = room.root_messages.create!(creator: User.find_by!(name: "Kevin"), body:,
      client_message_id: "divider-#{kind}-#{index}")
    first ||= message
  end
  membership.mark_unread_before(first) if first
  File.write(Rails.root.join("storage/db/browser-fixture.json"),
    JSON.generate(first_unread_id: first&.id, target_id: 908005739, page_size: Message::PAGE_SIZE))
else
  raise "Unknown browser fixture"
end
