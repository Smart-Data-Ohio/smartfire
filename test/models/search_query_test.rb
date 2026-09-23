require "test_helper"

class SearchQueryTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    @other_room = rooms(:watercooler)
  end

  test "plain text becomes a phrase-quoted FTS expression" do
    query = SearchQuery.parse("cats AND dogs")

    assert_equal %("cats" "AND" "dogs"), query.match_expression
    assert_not query.filters?
    assert_not query.blank_query?
  end

  test "from: matches creators by name substring, case-insensitively" do
    jz_message = @room.messages.create!(body: "jz filter target", client_message_id: "from-jz", creator: users(:jz))
    @room.messages.create!(body: "other filter target", client_message_id: "from-other", creator: users(:david))

    results = SearchQuery.parse("from:@jz filter target").apply_to_messages(Message.all)

    assert_equal [ jz_message ], results.to_a
  end

  test "from: without an @ prefix works the same" do
    query = SearchQuery.parse("from:jason hello")

    assert_equal [ "jason" ], query.from_names
    assert_equal "hello", query.text
  end

  test "from: with no matching user matches nothing" do
    @room.messages.create!(body: "nobody filter target", client_message_id: "from-nobody", creator: users(:david))

    results = SearchQuery.parse("from:@nosuchperson filter target").apply_to_messages(Message.all)

    assert_empty results.to_a
  end

  test "in: narrows to matching rooms" do
    room_message = @room.messages.create!(body: "room filter target", client_message_id: "in-room", creator: users(:david))
    @other_room.messages.create!(body: "room filter target", client_message_id: "in-other", creator: users(:david))

    results = SearchQuery.parse("in:#Designers room filter target").apply_to_messages(Message.all)

    assert_equal [ room_message ], results.to_a
  end

  test "in: with no matching room matches nothing" do
    @room.messages.create!(body: "lost room filter target", client_message_id: "in-lost", creator: users(:david))

    results = SearchQuery.parse("in:#nosuchroom lost room filter target").apply_to_messages(Message.all)

    assert_empty results.to_a
  end

  test "has:link matches messages with a stored link" do
    linked = @room.messages.create!(
      markdown_source: "see the linkfilter spec at https://example.com/spec",
      client_message_id: "has-link-yes", creator: users(:david)
    )
    @room.messages.create!(body: "no linkfilter here", client_message_id: "has-link-no", creator: users(:david))

    results = SearchQuery.parse("has:link linkfilter").apply_to_messages(Message.all)

    assert_equal [ linked ], results.to_a
  end

  test "has:file matches uploaded and Drive attachments" do
    drive_message = @room.messages.create!(
      body: "drive filefilter plan", client_message_id: "has-file-drive", creator: users(:david)
    )
    drive_message.drive_attachments.create!(file_id: "1a2b3c4d5e6f7g8h9i0j")
    upload_message = @room.messages.create!(
      body: "upload filefilter plan", client_message_id: "has-file-upload", creator: users(:david)
    )
    upload_message.attachment.attach(
      io: StringIO.new("plan"), filename: "plan.txt", content_type: "text/plain"
    )
    @room.messages.create!(body: "no filefilter here", client_message_id: "has-file-no", creator: users(:david))

    results = SearchQuery.parse("has:file filefilter").apply_to_messages(Message.all)

    assert_equal [ drive_message.id, upload_message.id ].sort, results.ids.sort
  end

  test "has:image matches image uploads only" do
    image_message = @room.messages.create!(body: "imagefilter shot", client_message_id: "has-image-yes", creator: users(:david))
    image_message.attachment.attach(
      io: file_fixture("earth.png").open, filename: "shot.png", content_type: "image/png"
    )
    text_message = @room.messages.create!(body: "imagefilter notes", client_message_id: "has-image-no", creator: users(:david))
    text_message.attachment.attach(
      io: StringIO.new("notes"), filename: "notes.txt", content_type: "text/plain"
    )

    results = SearchQuery.parse("has:image imagefilter").apply_to_messages(Message.all)

    assert_equal [ image_message ], results.to_a
  end

  test "has:pin matches pinned messages only" do
    pinned = @room.messages.create!(body: "pinfilter announcement", client_message_id: "has-pin-yes", creator: users(:david))
    MessagePin.pin!(message: pinned, pinner: users(:david))
    @room.messages.create!(body: "pinfilter draft", client_message_id: "has-pin-no", creator: users(:david))

    results = SearchQuery.parse("has:pin pinfilter").apply_to_messages(Message.all)

    assert_equal [ pinned ], results.to_a
  end

  test "has: with an unknown value stays plain text" do
    query = SearchQuery.parse("has:bogus hello")

    assert_empty query.has_values
    assert_equal "has:bogus hello", query.text
    assert_not query.filters?
  end

  test "date operators narrow by creation day" do
    day = Date.new(2026, 3, 10)
    on_day = @room.messages.create!(
      body: "datefilter milestone", client_message_id: "date-on",
      creator: users(:david), created_at: day.in_time_zone("UTC").noon
    )
    before_day = @room.messages.create!(
      body: "datefilter milestone", client_message_id: "date-before",
      creator: users(:david), created_at: (day - 2).in_time_zone("UTC").noon
    )
    after_day = @room.messages.create!(
      body: "datefilter milestone", client_message_id: "date-after",
      creator: users(:david), created_at: (day + 2).in_time_zone("UTC").noon
    )

    assert_equal [ on_day ], SearchQuery.parse("on:2026-03-10 datefilter").apply_to_messages(Message.all).to_a
    assert_equal [ on_day ], SearchQuery.parse("after:2026-03-09 before:2026-03-11 datefilter").apply_to_messages(Message.all).to_a
    assert_equal [ before_day ], SearchQuery.parse("before:2026-03-10 datefilter").apply_to_messages(Message.all).to_a
    assert_equal [ after_day ], SearchQuery.parse("after:2026-03-10 datefilter").apply_to_messages(Message.all).to_a
  end

  test "invalid dates stay plain text" do
    query = SearchQuery.parse("before:2026-13-45 hello")

    assert_nil query.before_date
    assert_equal "before:2026-13-45 hello", query.text
  end

  test "is:thread matches thread messages only" do
    thread = @room.channel_threads.create!(name: "filter thread", creator: users(:david))
    threaded = thread.post_message!(
      creator: users(:david),
      attributes: { body: "threadfilter reply", client_message_id: "is-thread-yes" }
    )
    @room.messages.create!(body: "threadfilter root", client_message_id: "is-thread-no", creator: users(:david))

    results = SearchQuery.parse("is:thread threadfilter").apply_to_messages(Message.all)

    assert_equal [ threaded ], results.to_a
  end

  test "operators combine with each other and the text" do
    match = @room.messages.create!(body: "combofilter launch", client_message_id: "combo-yes", creator: users(:jz))
    @room.messages.create!(body: "combofilter launch", client_message_id: "combo-wrong-author", creator: users(:david))
    @other_room.messages.create!(body: "combofilter launch", client_message_id: "combo-wrong-room", creator: users(:jz))

    results = SearchQuery.parse("from:@jz in:#Designers combofilter").apply_to_messages(Message.all)

    assert_equal [ match ], results.to_a
  end

  test "filter-only queries list without any text" do
    match = @room.messages.create!(body: "unrelated words here", client_message_id: "textless-yes", creator: users(:jz))

    results = SearchQuery.parse("from:@jz").apply_to_messages(users(:david).reachable_messages)

    assert_includes results.to_a, match
  end

  test "system notes never match, even for filter-only queries" do
    note = @room.messages.create!(
      markdown_source: "pinned a message", system_note: true,
      client_message_id: "system-note-search", creator: users(:david)
    )

    results = SearchQuery.parse("from:@David").apply_to_messages(Message.all)

    assert_not_includes results.to_a, note
  end

  test "chips carry labels and removal queries" do
    query = SearchQuery.parse("from:@jz has:file launch")

    assert_equal [ "from: jz", "has: file" ], query.chips.map(&:label)
    assert_equal [ "has:file launch", "from:@jz launch" ], query.chips.map(&:remove_query)
  end

  test "board posts match names in accessible board rooms" do
    board = Rooms::Board.create!(name: "Launch Board", creator: users(:david))
    board.memberships.grant_to(users(:david))
    post = board.channel_threads.create!(name: "Ship the searchboard", creator: users(:david), work_status: "planned")
    hidden_board = Rooms::Board.create!(name: "Hidden Board", creator: users(:kevin))
    hidden_board.memberships.grant_to(users(:kevin))
    hidden_board.channel_threads.create!(name: "Ship the searchboard secretly", creator: users(:kevin), work_status: "planned")

    results = SearchQuery.parse("searchboard").board_posts_for(users(:david))

    assert_equal [ post ], results.to_a
  end

  test "work threads exclude board posts and hidden rooms" do
    work = @room.channel_threads.create!(name: "Fix the searchwork bug", creator: users(:david), work_status: "planned")
    board = Rooms::Board.create!(name: "Work Board", creator: users(:david))
    board.memberships.grant_to(users(:david))
    board.channel_threads.create!(name: "Fix the searchwork post", creator: users(:david), work_status: "planned")
    hidden = Rooms::Closed.create!(name: "Hidden Room", creator: users(:kevin))
    hidden.memberships.grant_to(users(:kevin))
    hidden.channel_threads.create!(name: "Fix the searchwork hidden", creator: users(:kevin), work_status: "planned")

    results = SearchQuery.parse("searchwork").work_threads_for(users(:david))

    assert_equal [ work ], results.to_a
  end

  test "events match titles in accessible rooms" do
    results = SearchQuery.parse("launch checklist").events_for(users(:david))

    assert_equal [ events(:launch_party) ], results.to_a
    assert_empty SearchQuery.parse("launch checklist").events_for(users(:bender))
  end

  test "LIKE wildcards in operator values match literally" do
    @room.messages.create!(body: "wildcard target", client_message_id: "wildcard", creator: users(:david))

    assert_empty SearchQuery.parse("from:@% wildcard").apply_to_messages(Message.all).to_a
    assert_empty SearchQuery.parse("in:#_ wildcard").apply_to_messages(Message.all).to_a
  end

  test "quote characters in operators cannot break out of the query" do
    @room.messages.create!(body: "injection target", client_message_id: "injection", creator: users(:david))

    assert_empty SearchQuery.parse(%(from:@" OR "1"="1 injection)).apply_to_messages(Message.all).to_a
    assert_empty SearchQuery.parse(%(in:#" OR "1"="1 injection)).apply_to_messages(Message.all).to_a
    assert_nothing_raised do
      SearchQuery.parse(%(" OR 1=1 --)).apply_to_messages(Message.all).to_a
    end
  end
end
