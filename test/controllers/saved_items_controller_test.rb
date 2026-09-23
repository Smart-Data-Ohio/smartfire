require "test_helper"

class SavedItemsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @user = users(:david)
    @message = messages(:first)
  end

  test "index lists saved messages with status filters" do
    open_item = SavedItem.create!(user: @user, message: @message)
    done_item = SavedItem.create!(user: @user, message: messages(:second), status: :done)
    SavedItem.create!(user: users(:jason), message: @message)

    get saved_items_url
    assert_response :success
    assert_includes response.body, dom_id(open_item)
    assert_includes response.body, dom_id(done_item)

    get saved_items_url(status: "in_progress")
    assert_response :success
    assert_includes response.body, dom_id(open_item)
    assert_not_includes response.body, dom_id(done_item)

    get saved_items_url(status: "done")
    assert_response :success
    assert_not_includes response.body, dom_id(open_item)
    assert_includes response.body, dom_id(done_item)
  end

  test "index hides items whose room access was lost" do
    saved_item = SavedItem.create!(user: @user, message: @message)

    get saved_items_url
    assert_includes response.body, dom_id(saved_item)

    memberships(:david_designers).destroy!

    get saved_items_url
    assert_response :success
    assert_not_includes response.body, dom_id(saved_item)
    assert_not_includes response.body, @message.plain_text_body.truncate(500)
  end

  test "create saves with a reminder" do
    remind_at = 1.hour.from_now.utc.iso8601(3)

    assert_difference -> { @user.saved_items.count }, 1 do
      post saved_items_url(format: :json), params: { message_id: @message.id, saved_item: { remind_at: } }
    end

    assert_response :created
    saved_item = @user.saved_items.last
    assert_equal @message, saved_item.message
    assert_predicate saved_item, :in_progress?
    assert_equal Time.zone.parse(remind_at), saved_item.remind_at
    assert_equal saved_item_url(saved_item, format: :json), response.parsed_body["url"]
  end

  test "create without a reminder leaves remind_at blank" do
    post saved_items_url(format: :json), params: { message_id: @message.id }

    assert_response :created
    assert_nil @user.saved_items.last.remind_at
  end

  test "create again updates the reminder instead of duplicating" do
    SavedItem.create!(user: @user, message: @message)
    remind_at = 2.hours.from_now.utc.iso8601(3)

    assert_no_difference -> { SavedItem.count } do
      post saved_items_url(format: :json), params: { message_id: @message.id, saved_item: { remind_at: } }
    end

    assert_response :created
    assert_equal Time.zone.parse(remind_at), @user.saved_items.sole.remind_at
  end

  test "create again after a fired reminder re-arms it" do
    saved_item = SavedItem.create!(user: @user, message: @message, remind_at: 1.minute.from_now)
    travel_to(2.minutes.from_now) { SavedItem::ReminderDispatcher.dispatch_due! }
    assert_not_nil saved_item.reload.reminded_at

    remind_at = 2.hours.from_now.utc.iso8601(3)
    post saved_items_url(format: :json), params: { message_id: @message.id, saved_item: { remind_at: } }

    assert_response :created
    assert_nil saved_item.reload.reminded_at
    assert_equal Time.zone.parse(remind_at), saved_item.remind_at
  end

  test "create rejects past and unparseable reminders" do
    post saved_items_url(format: :json), params: { message_id: @message.id, saved_item: { remind_at: 1.minute.ago.utc.iso8601 } }
    assert_response :unprocessable_entity

    post saved_items_url(format: :json), params: { message_id: @message.id, saved_item: { remind_at: "not a time" } }
    assert_response :unprocessable_entity

    assert_empty @user.saved_items
  end

  test "create is 404 for a message the user cannot see" do
    private_room = Rooms::Closed.create!(name: "Private", creator: users(:jason))
    private_room.memberships.grant_to users(:jason)
    private_message = private_room.root_messages.create!(creator: users(:jason), markdown_source: "Secret", client_message_id: "private-save")

    post saved_items_url(format: :json), params: { message_id: private_message.id }

    assert_response :not_found
    assert_empty @user.saved_items
  end

  test "update marks done and reopens" do
    saved_item = SavedItem.create!(user: @user, message: @message)

    patch saved_item_url(saved_item, format: :json), params: { saved_item: { status: "done" } }
    assert_response :success
    assert_predicate saved_item.reload, :done?

    patch saved_item_url(saved_item, format: :json), params: { saved_item: { status: "in_progress" } }
    assert_response :success
    assert_predicate saved_item.reload, :in_progress?
  end

  test "update rejects an invalid status" do
    saved_item = SavedItem.create!(user: @user, message: @message)

    patch saved_item_url(saved_item, format: :json), params: { saved_item: { status: "archived" } }

    assert_response :unprocessable_entity
    assert_predicate saved_item.reload, :in_progress?
  end

  test "update and destroy are 404 for hidden or foreign items" do
    hidden = SavedItem.create!(user: @user, message: @message)
    memberships(:david_designers).destroy!
    foreign = SavedItem.create!(user: users(:jason), message: messages(:second))

    patch saved_item_url(hidden, format: :json), params: { saved_item: { status: "done" } }
    assert_response :not_found

    delete saved_item_url(hidden, format: :json)
    assert_response :not_found

    patch saved_item_url(foreign, format: :json), params: { saved_item: { status: "done" } }
    assert_response :not_found

    assert_predicate hidden.reload, :in_progress?
  end

  test "destroy removes the item" do
    saved_item = SavedItem.create!(user: @user, message: @message)

    assert_difference -> { SavedItem.count }, -1 do
      delete saved_item_url(saved_item, format: :json)
    end

    assert_response :no_content
  end
end
