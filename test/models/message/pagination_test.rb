require "test_helper"

class Message::PaginationTest < ActiveSupport::TestCase
  test "page edges split same-timestamp messages on id" do
    room = rooms(:designers)
    timestamp = Time.zone.local(2026, 2, 2, 12, 0, 0)
    tied = 5.times.map do |index|
      room.messages.create!(
        body: "tie #{index}",
        client_message_id: "tie-#{index}",
        creator: users(:david),
        created_at: timestamp
      )
    end
    first, second, third, fourth, fifth = tied

    before_page = room.messages.page_before(third)
    assert_includes before_page, first
    assert_includes before_page, second
    assert_not_includes before_page, third
    assert_not_includes before_page, fourth

    after_page = room.messages.page_after(third)
    assert_includes after_page, fourth
    assert_includes after_page, fifth
    assert_not_includes after_page, third
    assert_not_includes after_page, second

    around = room.messages.page_around(third)
    assert_equal tied, tied & around
  end

  test "paging across same-timestamp page edges visits every message exactly once" do
    room = Rooms::Closed.create!(name: "Paging edges", creator: users(:david))
    early = Time.zone.local(2026, 4, 1, 12, 0, 0)
    late = Time.zone.local(2026, 4, 2, 12, 0, 0)
    created = []
    10.times do |index|
      created << room.messages.create!(
        body: "old tie #{index}", client_message_id: "page-old-#{index}",
        creator: users(:david), created_at: early
      )
    end
    25.times do |index|
      created << room.messages.create!(
        body: "middle #{index}", client_message_id: "page-mid-#{index}",
        creator: users(:david), created_at: early + (index + 1).minutes
      )
    end
    10.times do |index|
      created << room.messages.create!(
        body: "new tie #{index}", client_message_id: "page-new-#{index}",
        creator: users(:david), created_at: late
      )
    end
    expected_ids = created.map(&:id).sort

    first = room.messages.first_page
    following = room.messages.page_after(first.last)
    assert_equal expected_ids, (first + following).map(&:id).sort
    assert_equal expected_ids.size, (first + following).map(&:id).uniq.size

    last = room.messages.last_page
    preceding = room.messages.page_before(last.first)
    assert_equal expected_ids, (preceding + last).map(&:id).sort
    assert_equal expected_ids.size, (preceding + last).map(&:id).uniq.size
  end

  test "before and after work on joined scopes" do
    cursor = rooms(:designers).messages.ordered.last

    before_messages = users(:david).reachable_messages.before(cursor).to_a
    after_messages = users(:david).reachable_messages.after(cursor).to_a

    assert before_messages.none? { |message| ([ message.created_at, message.id ] <=> [ cursor.created_at, cursor.id ]) >= 0 }
    assert after_messages.none? { |message| ([ message.created_at, message.id ] <=> [ cursor.created_at, cursor.id ]) <= 0 }
  end
end
