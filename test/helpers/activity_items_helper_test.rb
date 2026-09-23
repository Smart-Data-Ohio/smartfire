require "test_helper"

class ActivityItemsHelperTest < ActionView::TestCase
  include ActivityItemsHelper

  test "review request label" do
    item = ActivityItem.new(event_type: "pr_review_request")

    assert_equal "Review requested", activity_item_event_label(item)
  end

  test "event reminder body names the venue" do
    voice = Rooms::Voice.create_for({ name: "Lounge", creator: users(:david) }, users: [ users(:david) ])
    event = events(:launch_party)
    event.update!(venue_room_id: voice.id)
    item = ActivityItem.new(user: users(:david), source: event, event_type: "event_reminder")

    assert_equal "Starts in 15 minutes: Launch party planning in Lounge.", activity_item_event_body(item)
  end

  test "event reminder body without a venue is unchanged" do
    item = ActivityItem.new(user: users(:david), source: events(:launch_party), event_type: "event_reminder")

    assert_equal "Starts in 15 minutes: Launch party planning.", activity_item_event_body(item)
  end

  test "lockout item names the alert and links the profile" do
    credential = TwoFactorCredential.create!(user: users(:david),
      secret: TwoFactorCredential.generate_secret, confirmed_at: Time.current)
    item = ActivityItem.new(user: users(:david), source: credential, event_type: "two_factor_lockout")

    assert_equal "Sign-in lockout", activity_item_event_label(item)
    assert_equal "Several wrong sign-in codes were entered for your account.", activity_item_source_body(item)
    assert_equal "Two-step sign-in", activity_item_source_label(item)
    assert_equal user_profile_path, activity_item_source_path(item)
  end
end
