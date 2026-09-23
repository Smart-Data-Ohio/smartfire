require "test_helper"

class Message::ReferenceSyncTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    @source = @room.messages.create!(
      body: "quoted source words", client_message_id: "ref-sync-source", creator: users(:david)
    )
  end

  test "extract_message_ids finds /@ permalinks" do
    assert_equal [ @source.id ],
      Message::ReferenceSync.extract_message_ids("see /rooms/#{@room.id}/@#{@source.id} for details")
    assert_equal [ @source.id ],
      Message::ReferenceSync.extract_message_ids("https://smartfire.example/rooms/#{@room.id}/@#{@source.id}?x=1#fragment")
    assert_equal [ @source.id, @source.id + 1 ].sort,
      Message::ReferenceSync.extract_message_ids("/rooms/1/@#{@source.id} and /rooms/2/@#{@source.id + 1}")
  end

  test "extract_message_ids ignores anything else" do
    assert_empty Message::ReferenceSync.extract_message_ids("no links here")
    assert_empty Message::ReferenceSync.extract_message_ids("/rooms/1/events/2")
    assert_empty Message::ReferenceSync.extract_message_ids(nil)
  end

  test "posting a permalink creates a reference" do
    quote = @room.messages.create!(
      markdown_source: "look at this /rooms/#{@room.id}/@#{@source.id}",
      client_message_id: "ref-sync-quote", creator: users(:david)
    )

    assert_equal [ @source ], quote.referenced_messages
  end

  test "cross-room permalinks sync" do
    quote = rooms(:watercooler).messages.create!(
      markdown_source: "from next door /rooms/#{@room.id}/@#{@source.id}",
      client_message_id: "ref-sync-cross", creator: users(:david)
    )

    assert_equal [ @source ], quote.referenced_messages
  end

  test "sync is idempotent and drops removed permalinks on edit" do
    quote = @room.messages.create!(
      markdown_source: "look /rooms/#{@room.id}/@#{@source.id}",
      client_message_id: "ref-sync-edit", creator: users(:david)
    )

    assert_no_difference -> { MessageReference.count } do
      Message::ReferenceSync.call(quote)
    end

    quote.update!(markdown_source: "no links anymore")

    assert_empty quote.reload.referenced_messages
  end

  test "extract_message_ids caps at ten unique ids in order of appearance" do
    ids = (1..12).to_a
    text = ids.map { |id| "/rooms/1/@#{id}" }.join(" ") + " /rooms/1/@2"
    assert_equal (1..10).to_a, Message::ReferenceSync.extract_message_ids(text)
  end

  test "posting more than ten permalinks quotes only the first ten" do
    sources = 12.times.map do |index|
      @room.messages.create!(
        body: "capped source #{index}", client_message_id: "ref-sync-cap-#{index}", creator: users(:david)
      )
    end
    links = sources.map { |source| "/rooms/#{@room.id}/@#{source.id}" }.join(" ")

    quote = @room.messages.create!(
      markdown_source: "many #{links}",
      client_message_id: "ref-sync-cap-quote", creator: users(:david)
    )

    assert_equal sources.first(10).map(&:id).sort, quote.referenced_messages.map(&:id).sort
  end

  test "permalinks inside code spans and fenced blocks are ignored" do
    quoted = @room.messages.create!(
      body: "code test quoted", client_message_id: "ref-sync-code-quoted", creator: users(:david)
    )
    fenced = @room.messages.create!(
      body: "code test fenced", client_message_id: "ref-sync-code-fenced", creator: users(:david)
    )
    spanned = @room.messages.create!(
      body: "code test spanned", client_message_id: "ref-sync-code-spanned", creator: users(:david)
    )

    quote = @room.messages.create!(
      markdown_source: <<~MARKDOWN,
        look /rooms/#{@room.id}/@#{quoted.id}
        `/rooms/#{@room.id}/@#{spanned.id}`
        ```
        /rooms/#{@room.id}/@#{fenced.id}
        ```
      MARKDOWN
      client_message_id: "ref-sync-code-quote", creator: users(:david)
    )

    assert_equal [ quoted ], quote.referenced_messages
  end

  test "a labeled permalink still quotes through its href" do
    quote = @room.messages.create!(
      markdown_source: "look [over here](/rooms/#{@room.id}/@#{@source.id})",
      client_message_id: "ref-sync-labeled", creator: users(:david)
    )

    assert_equal [ @source ], quote.referenced_messages
  end

  test "missing, self, and system-note targets create nothing" do
    quote = @room.messages.create!(
      markdown_source: "missing /rooms/#{@room.id}/@999999999",
      client_message_id: "ref-sync-missing", creator: users(:david)
    )
    assert_empty quote.referenced_messages

    quote.update!(markdown_source: "self /rooms/#{@room.id}/@#{quote.id}")
    assert_empty quote.reload.referenced_messages

    note = @room.messages.create!(
      markdown_source: "pinned a message", system_note: true,
      client_message_id: "ref-sync-note", creator: users(:david)
    )
    quote.update!(markdown_source: "note /rooms/#{@room.id}/@#{note.id}")
    assert_empty quote.reload.referenced_messages
  end
end
