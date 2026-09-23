require "test_helper"

class RoomMailboxTest < ActionMailbox::TestCase
  ONE_PIXEL_PNG = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="

  setup do
    @domain_before = ENV["INBOUND_EMAIL_DOMAIN"]
    ENV["INBOUND_EMAIL_DOMAIN"] = "mail.test"
    @room = rooms(:designers)
    @token = @room.regenerate_inbound_email_token!
  end

  teardown do
    ENV["INBOUND_EMAIL_DOMAIN"] = @domain_before
  end

  test "mail from a member posts as that member" do
    assert_difference -> { @room.messages.count }, 1 do
      deliver_room_mail(
        from: "david@37signals.com",
        subject: "Launch update",
        body: "We ship Friday.",
        authentication_results: "mx.mail.test; dkim=pass header.d=37signals.com"
      )
    end

    message = @room.messages.order(:created_at).last
    assert_equal users(:david), message.creator
    assert_includes message.markdown_source, "We ship Friday."
    assert_includes message.markdown_source, "Launch update"
  end

  test "member matching is case-insensitive" do
    deliver_room_mail(
      from: "David@37Signals.com", body: "Hello",
      authentication_results: "mx.mail.test; dkim=pass header.d=37signals.com"
    )

    assert_equal users(:david), @room.messages.order(:created_at).last.creator
  end

  test "a member From without an authentication pass posts as the Email bot" do
    deliver_room_mail(from: "david@37signals.com", body: "Totally from David.")

    message = @room.messages.order(:created_at).last
    assert_equal "Email", message.creator.name
    assert_predicate message.creator, :bot?
    assert_includes message.markdown_source, "david@37signals.com"
  end

  test "a member From with a dmarc pass posts as the member" do
    deliver_room_mail(
      from: "david@37signals.com", body: "Hello",
      authentication_results: "mx.mail.test; dmarc=pass (p=REJECT) header.from=37signals.com"
    )

    assert_equal users(:david), @room.messages.order(:created_at).last.creator
  end

  test "a pass for another domain posts as the Email bot" do
    deliver_room_mail(
      from: "david@37signals.com", body: "Hello",
      authentication_results: "mx.mail.test; dkim=pass header.d=evil.test"
    )

    message = @room.messages.order(:created_at).last
    assert_equal "Email", message.creator.name
    assert_includes message.markdown_source, "david@37signals.com"
  end

  test "multiple authentication results post as the Email bot" do
    mail = Mail.new(from: "david@37signals.com", to: room_address, body: "Hello")
    mail.header["Authentication-Results"] = "mx.mail.test; dkim=pass header.d=37signals.com"
    mail.header["Authentication-Results"] = "attacker.test; dkim=pass header.d=37signals.com"
    receive_inbound_email_from_source(mail.to_s)

    assert_equal 2, mail.header.fields.count { |field| field.name.casecmp?("Authentication-Results") }
    assert_equal "Email", @room.messages.order(:created_at).last.creator.name
  end

  test "mail from a non-member posts as the Email bot with the sender shown" do
    receive_inbound_email_from_mail(
      from: "outsider@example.com", to: room_address,
      subject: "Tip", body: "Saw this."
    )

    message = @room.messages.order(:created_at).last
    assert_equal "Email", message.creator.name
    assert_predicate message.creator, :bot?
    assert_includes message.markdown_source, "outsider@example.com"
    assert_includes message.markdown_source, "Saw this."
    assert @room.memberships.exists?(user_id: message.creator_id)
  end

  test "mail from a member of another room posts as the Email bot" do
    other = users(:kevin)
    memberships(:kevin_designers).destroy!

    receive_inbound_email_from_mail(
      from: other.email_address, to: room_address, body: "Hello"
    )

    assert_equal "Email", @room.messages.order(:created_at).last.creator.name
  end

  test "an unknown token posts nothing" do
    assert_no_difference -> { Message.count } do
      receive_inbound_email_from_mail(
        from: "david@37signals.com", to: "room-bogus@mail.test", body: "Hello"
      )
    end
  end

  test "nothing posts while inbound email is disabled" do
    ENV.delete("INBOUND_EMAIL_DOMAIN")

    assert_no_difference -> { Message.count } do
      receive_inbound_email_from_mail(
        from: "david@37signals.com", to: room_address, body: "Hello"
      )
    end
  end

  test "a rotated token retires the old address" do
    old_address = room_address
    @room.regenerate_inbound_email_token!

    assert_no_difference -> { Message.count } do
      receive_inbound_email_from_mail(
        from: "david@37signals.com", to: old_address, body: "Hello"
      )
    end
  end

  test "deleted rooms receive nothing" do
    @room.update!(deleted_at: Time.current)

    assert_no_difference -> { Message.count } do
      receive_inbound_email_from_mail(
        from: "david@37signals.com", to: room_address, body: "Hello"
      )
    end
  end

  test "board rooms receive nothing" do
    board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) }, users: [ users(:david) ])
    token = board.regenerate_inbound_email_token!

    assert_no_difference -> { Message.count } do
      receive_inbound_email_from_mail(
        from: "david@37signals.com", to: "room-#{token}@mail.test", body: "Hello"
      )
    end
  end

  test "html-only mail is sanitized to text" do
    mail = Mail.new(
      from: "david@37signals.com", to: room_address, subject: "Note",
      content_type: "text/html",
      body: "<p>Hello <b>there</b></p><script>alert('x')</script>"
    )

    receive_inbound_email_from_source(mail.to_s)

    message = @room.messages.order(:created_at).last
    assert_includes message.markdown_source, "Hello"
    assert_not_includes message.markdown_source, "<script>"
    assert_not_includes message.markdown_source, "alert("
    assert_not_includes message.markdown_source, "<b>"
  end

  test "an attachment within the limit lands on the message" do
    mail = Mail.new(from: "david@37signals.com", to: room_address, subject: "File", body: "See attached.")
    mail.add_file(filename: "notes.txt", content: "file-bytes")

    receive_inbound_email_from_source(mail.to_s)

    message = @room.messages.order(:created_at).last
    assert message.attachment.attached?
    assert_equal "notes.txt", message.attachment.filename.to_s
    assert_includes message.markdown_source, "notes.txt"
  end

  test "an oversized attachment is named, not attached" do
    mail = Mail.new(from: "david@37signals.com", to: room_address, subject: "Big", body: "Big file.")
    mail.add_file(filename: "big.bin", content: "x" * (RoomMailbox::MAX_ATTACHMENT_BYTES + 1))

    receive_inbound_email_from_source(mail.to_s)

    message = @room.messages.order(:created_at).last
    assert_not message.attachment.attached?
    assert_includes message.markdown_source, "big.bin (not attached: over the 10 MB limit)"
  end

  test "a disallowed attachment type is named, not attached" do
    mail = Mail.new(from: "david@37signals.com", to: room_address, subject: "Tool", body: "Run this.")
    mail.add_file(filename: "tool.exe", content: "MZ-bytes")

    receive_inbound_email_from_source(mail.to_s)

    message = @room.messages.order(:created_at).last
    assert_not message.attachment.attached?
    assert_includes message.markdown_source, "tool.exe (not attached: file type not allowed)"
  end

  test "an office document within the limit lands on the message" do
    mail = Mail.new(from: "david@37signals.com", to: room_address, subject: "Report", body: "See attached.")
    mail.add_file(filename: "report.docx", content: "PK-bytes")

    receive_inbound_email_from_source(mail.to_s)

    message = @room.messages.order(:created_at).last
    assert message.attachment.attached?
    assert_equal "report.docx", message.attachment.filename.to_s
  end

  test "an image within the limit lands on the message" do
    mail = Mail.new(from: "david@37signals.com", to: room_address, subject: "Photo", body: "See attached.")
    mail.add_file(filename: "photo.png", content: Base64.decode64(ONE_PIXEL_PNG))

    receive_inbound_email_from_source(mail.to_s)

    message = @room.messages.order(:created_at).last
    assert message.attachment.attached?
    assert_equal "photo.png", message.attachment.filename.to_s
  end

  test "a room accepts at most 30 emailed messages per hour" do
    travel_to Time.current.change(min: 30) do
      with_memory_cache do
        assert_difference -> { @room.messages.count }, 30 do
          30.times do |index|
            deliver_room_mail(from: "outsider#{index}@example.com", body: "Hello #{index}")
          end
        end

        assert_no_difference -> { Message.count } do
          deliver_room_mail(from: "late@example.com", body: "One too many")
        end
      end
    end
  end

  test "an empty mail with no attachment posts nothing" do
    assert_no_difference -> { Message.count } do
      receive_inbound_email_from_mail(
        from: "david@37signals.com", to: room_address, body: "  "
      )
    end
  end

  test "a mentioned legacy bot receives its webhook" do
    bot = User.create_bot!(name: "Legacy Bot", skip_open_room_grant: true, webhook_url: "http://example.com/legacy")
    @room.memberships.create!(user: bot)
    delivery = stub_request(:post, "http://example.com/legacy").to_return(status: 200)

    perform_enqueued_jobs only: Bot::WebhookJob do
      receive_inbound_email_from_mail(
        from: "david@37signals.com", to: room_address, body: "Hey @[Legacy Bot], look at this."
      )
    end

    assert_requested delivery, times: 1
  end

  private
    def room_address
      "room-#{@token}@mail.test"
    end

    def deliver_room_mail(from:, body: "Hello", authentication_results: nil, **options)
      mail = Mail.new({ from:, to: room_address, body:, **options })
      mail.header["Authentication-Results"] = authentication_results if authentication_results
      receive_inbound_email_from_source(mail.to_s)
    end

    def with_memory_cache
      store = ActiveSupport::Cache::MemoryStore.new
      previous = Rails.cache
      Rails.cache = store
      yield
    ensure
      Rails.cache = previous
    end
end
