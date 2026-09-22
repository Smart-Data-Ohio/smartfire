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

  test "before and after work on joined scopes" do
    cursor = rooms(:designers).messages.ordered.last

    before_messages = users(:david).reachable_messages.before(cursor).to_a
    after_messages = users(:david).reachable_messages.after(cursor).to_a

    assert before_messages.none? { |message| ([ message.created_at, message.id ] <=> [ cursor.created_at, cursor.id ]) >= 0 }
    assert after_messages.none? { |message| ([ message.created_at, message.id ] <=> [ cursor.created_at, cursor.id ]) <= 0 }
  end
end
