# Browser fixtures are created by pinned Rails, then copied unchanged to both apps.
# Source: test/system/message_list_a11y_test.rb at d7c7de92.
kind = ARGV.fetch(0)
# The production parity seed has many extra provider/state examples. The
# pinned list regressions use just the three original Designers fixtures.
if %w[message_list message_destinations history boosts].include?(kind) || kind.start_with?("unread-") || kind.start_with?("composer-")
  Room.find(654632876).root_messages.where.not(id: [309456473, 908005739, 607264868]).destroy_all
end
case kind
when "message_list"
  # No additional rows are needed.
when "message_destinations"
  room = Room.find(654632876)
  creator = User.find(773523953)
  room.messages.create!(creator:, body: "A searchable menu result", client_message_id: "menu-search-result")
  thread = ChannelThread.create!(room:, creator:, name: "Menu audit thread", parent_message: Message.find(607264868))
  reply = thread.messages.create!(room:, creator:, markdown_source: "A thread reply with a menu", client_message_id: "menu-thread-reply")
  File.write(Rails.root.join("storage/db/browser-fixture.json"), JSON.generate(menu_thread_id: thread.id, menu_reply_id: reply.id))
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
when "edit-card"
  message = Room.find(654632876).messages.create!(creator: User.find(773523953),
    markdown_source: "nothing linked yet", client_message_id: "system-edit-card")
  File.write(Rails.root.join("storage/db/browser-fixture.json"), JSON.generate(edit_card_id: message.id))
when /^composer-/
  pets = Room.find_by!(name: "All Pets")
  pets.memberships.grant_to(User.find(773523953))
  User.find_by!(name: "Kevin").update!(name: "David") if kind == "composer-typing"
  File.write(Rails.root.join("storage/db/browser-fixture.json"), JSON.generate(pets_id: pets.id))
when "attach-menu"
  user = User.find(773523953)
  user.google_account&.destroy!
  GoogleAccount.create!(user:, email: "jz@gmail.test", refresh_token: "refresh-token-#{user.id}",
    access_token: "access-token-#{user.id}", access_token_expires_at: 1.hour.from_now,
    scopes: "openid email https://www.googleapis.com/auth/calendar.events https://www.googleapis.com/auth/drive.file")
  User.find(712064548).google_account&.destroy!
when "boosts"
  Message.find(607264868).boosts.create!(booster: User.find(127326141), content: "Older note")
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
