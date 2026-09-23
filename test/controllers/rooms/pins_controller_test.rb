require "test_helper"

class Rooms::PinsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
  end

  test "index lists pins newest first with jump and unpin actions" do
    MessagePin.pin!(message: messages(:first), pinner: users(:david))
    MessagePin.pin!(message: messages(:second), pinner: users(:jason))

    get room_pins_url(@room)

    assert_response :success
    list = Nokogiri::HTML5.fragment(response.body).at_css("ol.pins-panel__items")
    assert_not_nil list, "expected a pins list in #{response.body}"
    excerpts = list.css(".pins-panel__excerpt").map(&:text)
    assert_equal [ messages(:second).plain_text_body.truncate(200), messages(:first).plain_text_body.truncate(200) ], excerpts
    assert list.at_css("a[href='#{room_at_message_url(@room, messages(:second))}']"), "expected a jump link in #{response.body}"
    assert_equal 2, list.css("form[action='#{message_pin_path(messages(:second))}'], form[action='#{message_pin_path(messages(:first))}']").length
    assert_includes response.body, "Pinned by"
  end

  test "index renders an empty state without pins" do
    get room_pins_url(@room)

    assert_response :success
    assert_includes response.body, "No pinned messages yet"
  end

  test "index is 404 for a non-member" do
    private_room = Rooms::Closed.create!(name: "Private", creator: users(:jason))
    private_room.memberships.grant_to users(:jason)

    get room_pins_url(private_room)

    assert_response :not_found
  end
end
