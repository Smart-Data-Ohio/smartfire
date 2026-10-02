# Browser fixtures are created by pinned Rails, then copied unchanged to both apps.
# Source: test/system/message_list_a11y_test.rb at d7c7de92.
kind = ARGV.fetch(0)
# The production parity seed has many extra provider/state examples. The
# pinned list regressions use just the three original Designers fixtures.
if %w[message_list message_destinations history boosts toolbar interactions actions-mobile thread-pr].include?(kind) || kind.start_with?("unread-") || kind.start_with?("composer-") || kind.start_with?("highlight")
  Room.find(654632876).root_messages.where.not(id: [309456473, 908005739, 607264868]).destroy_all
end
case kind
when "mobile-layout"
  # MobileLayoutTest signs in JZ from the ordinary fixtures, which contain no
  # enrolled credential or remembered device. The rich parity seed adds both
  # for authentication coverage; remove just those unrelated extras here.
  user=User.find(773523953)
  user.two_factor_credential&.destroy!
  TwoFactorRememberedDevice.where(user:).destroy_all
when "board-touch"
  user=User.find(773523953)
  board=Rooms::Board.create_for({name:"Launch",creator:user},users:[user])
  post=ChannelThread.create!(room:board,creator:user,name:"Ship it",work_status:"planned")
  File.write(Rails.root.join("storage/db/browser-fixture.json"),JSON.generate(board_id:board.id,post_id:post.id))
when "work-controller"
  room=Room.find(654632876);creator=User.find(773523953)
  thread=ChannelThread.create!(room:,creator:,name:"Design discussion")
  ThreadMembership.join!(thread,creator)
  scenario=ARGV[1].to_s
  if scenario.start_with?("assigned owner", "only a thread manager")
    thread.update!(work_status:"planned",work_owner_id:712064548)
  elsif scenario.start_with?("the owner picker", "a member who")
    thread.update!(work_status:"planned")
  end
  message=thread.messages.create!(room:,creator:,markdown_source:"Keep this history",client_message_id:"work-history")
  metadata={thread_id:thread.id,history_message_id:message.id}
  %w[eligible suspended outside reader botless].each do |kind|
    bot=User.create_bot!(name:"#{kind.capitalize} Owner Agent")
    metadata[:"#{kind}_id"]=bot.id
    room.memberships.grant_to(bot) unless kind=="outside"
    next if kind=="botless"
    agent=bot.create_agent!(kind: :workspace,owner:User.find(127326141))
    agent.update!(provider:"TestLab",description:"Does the work") if kind=="eligible"
    AgentGrant.create!(agent:,room: kind=="outside" ? nil : room,granted_by:User.find(127326141),capability:kind=="reader" ? "read_messages" : "post_messages")
    agent.suspend! if kind=="suspended"
    metadata[:eligible_agent_id]=agent.id if kind=="eligible"
  end
  # A removed member retains their identity. This is the same inactive-owner
  # state as :343's deactivate, without deactivating Kevin's test session.
  revoked=User.create!(name:"Kevin",email_address:"revoked@fixtures.test",password:"test-password",role: :member)
  revoked_thread=ChannelThread.create!(room:,creator:,name:"Revoked work owner",work_status:"planned")
  room.memberships.grant_to(revoked)
  revoked_thread.update_work!(actor:creator,work_owner_id:revoked.id)
  revoked.deactivate
  metadata[:revoked_thread_id]=revoked_thread.id
  File.write(Rails.root.join("storage/db/browser-fixture.json"),JSON.generate(metadata))
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
when "toolbar", "interactions", "actions-mobile"
  require Rails.root.join("app/helpers/emoji_helper")
  metadata = { reaction_count: EmojiHelper::REACTIONS.length,
    message_permalink: Rails.application.routes.url_helpers.room_at_message_path(654632876, Message.find(607264868)) }
  if kind == "toolbar"
    WorkspaceIcon.find_by(name: "acme")&.destroy!
    icon = WorkspaceIcon.new(name: "acme", title: "Acme Corp", creator: User.find(127326141))
    icon.image.attach(io: File.open(Rails.root.join("test/fixtures/files/workspace_icons/clean.svg")), filename: "clean.svg")
    icon.save!
  elsif kind == "interactions"
    destination = ChannelThread.create!(room: Room.find(654632876), creator: User.find(773523953), name: "Forward destination")
    metadata[:forward_thread_id] = destination.id
  end
  File.write(Rails.root.join("storage/db/browser-fixture.json"), JSON.generate(metadata))
when /^highlight/
  # Extract original literals from the reference test itself; no candidate
  # render or hand-authored vector supplies the expected source.
  source = File.read(Rails.root.join("storage/db/code-highlighting-reference.rb"))
  samples = eval(source.match(/SAMPLES = (\{.*?\})\.freeze/m)[1])
  literal = source.split("source = <<~'MARKDOWN'\n")[1].split("    MARKDOWN")[0].lines.map { |line| line.delete_prefix("      ") }.join
  code_source = eval(source.match(/test "code and copying.*?source = ("(?:[^"\\]|\\.)*")/m)[1])
  replacement = eval(source.match(/replacement = ("(?:[^"\\]|\\.)*")/m)[1])
  wait = File.read(Rails.root.join("storage/db/application-system-reference.rb")).match(/HIGHLIGHT_WAIT = (\d+)/)[1].to_i
  metadata = { samples: samples.to_a, literal_code_source: literal, code_source:, code_replacement: replacement, highlight_wait: wait }
  if kind == "highlight-thread"
    room=Room.find(654632876);author=User.find(773523953)
    thread=ChannelThread.create!(room:,creator:author,name:"Code review")
    ThreadMembership.join!(thread,author)
    code='const greeting: string = "' + 'Hello '*40 + '";'
    message=thread.messages.create!(room:,creator:author,markdown_source:"```ts\n#{code}\n```",client_message_id:"thread-code")
    metadata[:code_thread_id]=thread.id;metadata[:code_thread_message_id]=message.id
  elsif kind == "highlight-search"
    body = "HighlightSearchExample\n\n```javascript\nconst value = true;\n```"
    message = Room.find(654632876).messages.create!(creator: User.find(773523953), markdown_source: body, client_message_id: "highlight-search-example")
    metadata[:code_search_id] = message.id
  end
  File.write(Rails.root.join("storage/db/browser-fixture.json"), JSON.generate(metadata))
when "thread-anchor", "thread-anchor-race"
  race = kind == "thread-anchor-race"
  room = Room.find(654632876)
  author = User.find(773523953)
  thread = ChannelThread.create!(room:, creator: author, name: race ? "Anchored unread race" : "Shared anchor thread")
  member = ThreadMembership.join!(thread, author)
  count = race ? Message::PAGE_SIZE * 3 + 1 : Message::PAGE_SIZE * 2 + 5
  first = count.seconds.ago
  messages = count.times.map do |index|
    thread.messages.create!(room:, creator: author, markdown_source: "#{race ? 'Anchored race' : 'Shared anchor'} post #{index}",
      client_message_id: "#{kind}-#{index}", created_at: race ? first + index.seconds : Time.current)
  end
  member.update!(unread_at: nil)
  File.write(Rails.root.join("storage/db/browser-fixture.json"), JSON.generate(thread_id: thread.id, anchor_id: messages[5].id, page_size: Message::PAGE_SIZE))
when "thread-pr"
  message = Room.find(654632876).messages.create!(creator: User.find(773523953),
    markdown_source: "review https://github.com/rails/rails/pull/12", client_message_id: "system-discuss-flow")
  pull_request = message.github_pull_requests.first!
  # The original current public card and files are persisted by Rails. No
  # browser response substitution or fake provider markup is involved.
  pull_request.update!(private: false, title: "Fix login", author_login: "alice", state: "open",
    base_branch: "main", head_branch: "shiny", review_decision: "approved", check_status: "passing",
    html_url: "https://github.com/rails/rails/pull/12", github_updated_at: 1.hour.ago,
    fetched_at: Time.current, fetch_error: nil,
    changed_files: { "files" => [{ "filename" => "app/models/user.rb", "additions" => 10, "deletions" => 2, "status" => "modified" }], "total_count" => 1 }.to_json,
    changed_files_fetched_at: Time.current)
  pull_request.update_column(:fetch_requested_at, nil)
  raise "Unexpected existing discussion" if Github::PullRequestThread.exists?(github_pull_request_id: pull_request.id, room_id: message.room_id)
  File.write(Rails.root.join("storage/db/browser-fixture.json"), JSON.generate(pr_id: pull_request.id, pr_message_id: message.id))
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
