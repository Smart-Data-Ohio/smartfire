require "test_helper"

class MessageFirstPaintTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
  end

  test "room page messages are programmatically focusable before JavaScript runs" do
    get room_url(@room)
    assert_response :success

    # The roving tab stop arrives with the lazy message-list module, but every
    # message must already accept .focus(): without a tabindex the call is a
    # silent no-op and keyboard focus never leaves the composer.
    assert_select "div.message", minimum: 1
    assert_select "div.message:not([tabindex='-1'])", count: 0

    template = response.body[/<script type="text\/template".*?<\/script>/m]
    assert_not_nil template, "expected the pending-message template on the room page"
    assert_includes template, 'tabindex="-1"',
      "expected the pending-message template to match messages/_message"
  end
end
