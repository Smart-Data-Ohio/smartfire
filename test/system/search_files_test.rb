require "application_system_test_case"

class SearchFilesTest < ApplicationSystemTestCase
  setup do
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
    join_room @room
  end

  test "filter chips show parsed operators and remove them" do
    @room.messages.create!(
      creator: users(:jz), markdown_source: "chiptune launch notes",
      client_message_id: "system-chip-match"
    )

    visit searches_url(q: "from:@jz has:file chiptune")
    assert_selector ".search-filter-chip", count: 2, wait: 10
    assert_selector ".search-filter-chip", text: "from: jz"

    # No message matches (the note has no file), but the page is not empty:
    # the watermark must stay hidden so it never covers the chip controls.
    assert_no_selector ".message-area--empty", visible: true

    click_link "Remove has: file filter"
    assert_selector ".search-filter-chip", count: 1, wait: 10
    assert_selector ".search-filter-chip", text: "from: jz"
    assert_text "chiptune launch notes", wait: 10
  end

  test "a pasted permalink renders a quote card with a working jump link" do
    source = @room.messages.create!(
      creator: users(:david), markdown_source: "quoted system words here",
      client_message_id: "system-quote-source"
    )
    @room.messages.create!(
      creator: users(:jz), markdown_source: "see this /rooms/#{@room.id}/@#{source.id}",
      client_message_id: "system-quote-quoting"
    )

    visit room_url(@room)
    assert_selector "blockquote.message-quote", text: /quoted system words here/, wait: 10
    assert_selector ".message-quote__author", text: "David"

    within "blockquote.message-quote" do
      click_link "Jump to message"
    end
    assert_current_path %r{/rooms/#{@room.id}/@#{source.id}}, wait: 10
  end

  test "a cross-room permalink loads its quote frame for members and outsiders" do
    source_room = rooms(:watercooler)
    source = source_room.messages.create!(
      creator: users(:jason), markdown_source: "cross-room quoted system words",
      client_message_id: "system-quote-cross-source"
    )
    @room.messages.create!(
      creator: users(:jz), markdown_source: "see this /rooms/#{source_room.id}/@#{source.id}",
      client_message_id: "system-quote-cross-quoting"
    )

    # JZ belongs to Designers but not Watercooler: the lazy frame loads and
    # resolves to the private-room chip.
    visit room_url(@room)
    assert_selector "turbo-frame.message-link-frame", wait: 10
    assert_selector ".message-quote-private", text: "Message in a private room", wait: 10

    # David belongs to both rooms: the same frame loads the full card.
    sign_in "david@37signals.com"
    visit room_url(@room)
    assert_selector "blockquote.message-quote", text: /cross-room quoted system words/, wait: 10
    assert_selector ".message-quote__author", text: "Jason"
  end

  test "the Files tab lists uploads and Drive rows with working filters" do
    message = @room.messages.create!(
      creator: users(:jz), body: "system file rows",
      client_message_id: "system-files-upload"
    )
    message.attachment.attach(
      io: file_fixture("earth.png").open, filename: "system-cover.png", content_type: "image/png"
    )
    @room.messages.create!(
      creator: users(:jz), body: "system drive rows",
      client_message_id: "system-files-drive"
    ).drive_attachments.create!(file_id: "1a2b3c4d5e6f7g8h9i0j")

    visit room_url(@room)
    click_link "Show files"
    assert_selector ".room-files__name", text: "system-cover.png", wait: 10
    assert_selector ".room-files__drive-link", text: /Google Drive file/

    fill_in "Search by filename", with: "cover"
    click_button "Search"
    assert_selector ".room-files__name", text: "system-cover.png", wait: 10

    click_link "Images"
    assert_selector ".room-files__name", count: 1, wait: 10

    click_link "Videos"
    assert_text "No uploads match.", wait: 10
  end
end
