# Server interaction assertions from test/system/events_test.rb at d7c7de92.
# Run inside the pinned Rails test image with networking disabled. Uses real
# controllers, models, inbox open and lazy Turbo frame responses, without pixels.
require_relative "../../../test/test_helper"

class CutoverEventInteractionsTest < ActionDispatch::IntegrationTest
  setup do
    travel_to Time.utc(2026, 9, 22, 12)
    @room = rooms(:designers)
    sign_in :david
  end

  teardown { travel_back }

  def schedule(title, description: nil, repeating: false)
    get room_url(@room)
    assert_response :success
    assert_select "a[href=?]", room_events_path(@room)
    get room_events_url(@room)
    assert_select "h1", text: "Events"
    assert_select "a[href=?]", new_room_event_path(@room), text: "New event"
    get new_room_event_url(@room)
    %w[title description starts_at recurrence_rule recurrence_until].each do |field|
      assert_select "[name='event[#{field}]']"
    end
    assert_select "input[value='Schedule event']"
    input = { title:, description:, starts_at: "2026-09-30T15:30", time_zone: "UTC" }
    input.merge!(recurrence_rule: "weekly", recurrence_until: "2026-10-14") if repeating
    post room_events_url(@room), params: { event: input }
    event = Event.where(title:).order(:created_at, :id).first!
    assert_redirected_to room_event_path(@room, event)
    get room_event_url(@room, event)
    assert_select "h1", text: title
    assert_includes response.body, description if description
    assert_select ".room-events__response p", text: /Currently:\s+Going/
    if repeating
      assert_includes response.body, "Part of a series"
      assert_includes response.body, "Next occurrence"
    end
    get room_events_url(@room)
    assert_includes response.body, title
    if repeating
      assert_includes response.body, "Repeats weekly"
      assert_includes response.body, "3 occurrences remaining"
    end
    event
  end

  def inbox_response(event, repeating: false)
    item = ActivityItem.find_by!(user: users(:jason), source: event, event_type: "event_invitation")
    delete session_url
    sign_in :jason
    get activity_items_url
    assert_response :success
    assert_select "#activity_item_#{item.id}" do
      assert_includes response.body, "Event invitation"
      assert_includes response.body, event.title
      assert_includes response.body, "repeats weekly until" if repeating
    end
    post open_activity_item_url(item)
    assert_response :see_other
    assert_redirected_to room_event_path(@room, event)
    get room_event_url(@room, event)
    assert_select "h1", text: event.title
    patch room_event_attendance_url(@room, event), params: { response: "going" }
    assert_redirected_to room_event_path(@room, event)
    get room_event_url(@room, event)
    assert_select ".room-events__response p", text: /Currently:\s+Going/
  end

  test "schedule invitation inbox open and response" do
    event = schedule("Launch retro", description: "Bring your notes.")
    inbox_response(event)
    assert_equal "going", event.reload.response_for(users(:jason))
  end

  test "repeating schedule invites once and copies response" do
    head = schedule("Weekly planning", repeating: true)
    assert_equal 3, head.series_events.count
    assert_equal 1, ActivityItem.where(user: users(:jason), source: head.series_events).count
    inbox_response(head, repeating: true)
    assert_equal ["going"] * 3, head.series_events.map { |event| event.response_for(users(:jason)) }
  end

  test "announcement card response stays in requested frame" do
    event = schedule("Card session")
    message = event.referencing_messages.order(:id).first!
    delete session_url
    sign_in :jason
    get room_url(@room)
    assert_response :success
    assert_includes response.body, "Scheduled an event: Card session"
    assert_select ".event-card__title", text: "Card session"
    assert_includes response.body, "Organized by David"
    frame = "response_for_message_#{message.id}_event_#{event.id}"
    url = room_event_attendance_path(@room, event, message_id: message.id)
    assert_select "turbo-frame[src=?]", url
    get url, headers: { "Turbo-Frame" => frame }
    assert_response :success
    assert_includes response.body, "No response yet"
    assert_select "button[value='going']"
    patch room_event_attendance_url(@room, event), params: { response: "going", message_id: message.id.to_s }, headers: { "Turbo-Frame" => frame }
    assert_response :success
    assert_nil response.headers["Location"]
    assert_select "turbo-frame[id=?]", frame
    assert_select ".event-card__current", text: /Currently:\s+Going/
    get room_url(@room)
    assert_includes response.body, "Scheduled an event: Card session"
    assert_equal "going", event.reload.response_for(users(:jason))
  end
end
