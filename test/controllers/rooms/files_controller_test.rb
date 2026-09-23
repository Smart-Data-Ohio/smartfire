require "test_helper"

class Rooms::FilesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
  end

  test "lists uploads newest first with jump links" do
    older = create_upload("alpha-report.pdf", "application/pdf", created_at: 2.days.ago)
    newer = create_upload("beta-mockup.png", "image/png", created_at: 1.day.ago)

    get room_files_url(@room)

    assert_response :success
    names = css_select(".room-files__name").map(&:text)
    assert_equal [ "beta-mockup.png", "alpha-report.pdf" ], names
    assert_select "a.room-files__jump[href=?]", room_at_message_url(@room, newer), count: 1
    assert_select "a.room-files__jump[href=?]", room_at_message_url(@room, older), count: 1
  end

  test "lists Drive attachments as generic picker-only rows" do
    message = @room.messages.create!(body: "drive plan", client_message_id: "files-drive", creator: users(:david))
    drive = message.drive_attachments.create!(file_id: "1a2b3c4d5e6f7g8h9i0j")

    get room_files_url(@room)

    assert_response :success
    assert_select ".room-files__drive-link[href=?]", drive.url, text: /Google Drive file/
    assert_select "a.room-files__jump[href=?]", room_at_message_url(@room, message), count: 1
  end

  test "type filters narrow uploads" do
    create_upload("filter-shot.png", "image/png")
    create_upload("filter-notes.pdf", "application/pdf")

    get room_files_url(@room, type: "images")

    assert_response :success
    assert_equal [ "filter-shot.png" ], css_select(".room-files__name").map(&:text)

    get room_files_url(@room, type: "documents")

    assert_equal [ "filter-notes.pdf" ], css_select(".room-files__name").map(&:text)

    get room_files_url(@room, type: "videos")

    assert_empty css_select(".room-files__name")
  end

  test "filename search matches substrings and escapes wildcards" do
    create_upload("quarterly-report.pdf", "application/pdf")
    create_upload("team-photo.png", "image/png")

    get room_files_url(@room, filename: "QUARTERLY")

    assert_response :success
    assert_equal [ "quarterly-report.pdf" ], css_select(".room-files__name").map(&:text)

    get room_files_url(@room, filename: "%")

    assert_empty css_select(".room-files__name")
  end

  test "uploads page cumulatively" do
    32.times do |index|
      create_upload("paged-file-#{index.to_s.rjust(2, "0")}.txt", "text/plain")
    end

    get room_files_url(@room)

    assert_response :success
    assert_equal 30, css_select(".room-files__name").size

    get room_files_url(@room, page: 2)

    assert_equal 32, css_select(".room-files__name").size
  end

  test "only the room's own files are listed" do
    create_upload("designers-only.pdf", "application/pdf")
    other = rooms(:watercooler).messages.create!(body: "elsewhere", client_message_id: "files-elsewhere", creator: users(:david))
    other.attachment.attach(fixture_file_upload("moon.jpg", "image/jpeg"))

    get room_files_url(@room)

    assert_response :success
    assert_equal [ "designers-only.pdf" ], css_select(".room-files__name").map(&:text)
  end

  test "non-members get nothing" do
    sign_in :kevin

    get room_files_url(rooms(:watercooler))

    assert_response :not_found
  end

  test "rendering costs the same queries for 4 files as for 16" do
    seed_files(2)

    # Warm request-scoped caches (custom icons) so both counted runs start even.
    get room_files_url(@room)
    assert_response :success

    few = count_queries { get room_files_url(@room) }

    seed_files(6)

    many = count_queries { get room_files_url(@room) }

    assert_equal few, many
  end

  private
    def create_upload(filename, content_type, created_at: Time.current)
      message = @room.messages.create!(
        body: filename, client_message_id: "files-#{filename}-#{SecureRandom.hex(4)}",
        creator: users(:david), created_at:
      )
      message.attachment.attach(
        io: StringIO.new("contents of #{filename}"), filename:, content_type:
      )
      message.attachment.update_columns(created_at:)
      message
    end

    def seed_files(count)
      count.times do |index|
        create_upload("seed-#{index}-#{SecureRandom.hex(2)}.txt", "text/plain")
        @room.messages.create!(body: "seed drive #{index}", client_message_id: "files-seed-#{SecureRandom.hex(4)}", creator: users(:david))
          .drive_attachments.create!(file_id: "seed#{SecureRandom.hex(8)}")
      end
    end

    def count_queries(&block)
      count = 0
      subscriber = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 unless payload[:name] == "SCHEMA"
      end
      block.call
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscriber)
    end
end
