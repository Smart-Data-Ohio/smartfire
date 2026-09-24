require "test_helper"

class SavedItem::ReminderDispatcherTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
    @message = messages(:first)
  end

  test "a due reminder notifies once and enqueues push" do
    saved_item = SavedItem.create!(user: @user, message: @message, remind_at: 1.hour.from_now)

    travel_to 61.minutes.from_now do
      assert_enqueued_with(job: SavedItem::ReminderPushJob, args: [ saved_item ]) do
        SavedItem::ReminderDispatcher.dispatch_due!
      end
    end

    assert_not_nil saved_item.reload.reminded_at

    item = ActivityItem.find_by!(user: @user, source: saved_item)
    assert_equal "message_reminder", item.event_type
    assert_predicate item, :unread?
  end

  test "a reminder fires only once across runs" do
    saved_item = SavedItem.create!(user: @user, message: @message, remind_at: 1.minute.from_now)

    travel_to 2.minutes.from_now do
      SavedItem::ReminderDispatcher.dispatch_due!
      first_fired_at = saved_item.reload.reminded_at

      assert_no_enqueued_jobs only: SavedItem::ReminderPushJob do
        SavedItem::ReminderDispatcher.dispatch_due!
      end

      assert_equal first_fired_at, saved_item.reload.reminded_at
      assert_equal 1, ActivityItem.where(user: @user, source: saved_item, event_type: "message_reminder").count
    end
  end

  test "a reminder rescheduled past now before the lock is taken does not fire" do
    saved_item = SavedItem.create!(user: @user, message: @message, remind_at: 1.minute.from_now)

    travel_to 2.minutes.from_now do
      # The row was selected while due, then the saver moved the reminder
      # before the dispatcher took the lock: the post-lock re-check must
      # see the new time and leave the item unclaimed for its new moment.
      saved_item.update!(remind_at: 1.hour.from_now)

      assert_no_enqueued_jobs only: SavedItem::ReminderPushJob do
        assert_no_difference -> { ActivityItem.count } do
          SavedItem::ReminderDispatcher.send(:dispatch_item!, saved_item, now: Time.current)
        end
      end
    end

    assert_nil saved_item.reload.reminded_at
    assert_predicate saved_item, :reminder_pending?
  end

  test "future reminders and items without reminders do not fire" do
    future = SavedItem.create!(user: @user, message: @message, remind_at: 1.hour.from_now)
    plain = SavedItem.create!(user: @user, message: messages(:second))

    assert_no_enqueued_jobs only: SavedItem::ReminderPushJob do
      SavedItem::ReminderDispatcher.dispatch_due!
    end

    assert_nil future.reload.reminded_at
    assert_nil plain.reload.reminded_at
    assert_empty ActivityItem.where(user: @user, source: [ @message, messages(:second) ])
  end

  test "a reminder scheduled in another time zone fires at the same instant" do
    # Pinned: the reminder is a fixed wall-clock instant whose UTC
    # mapping depends on September daylight time, and the future-date
    # validation would reject it once the real clock passes it.
    travel_to Time.utc(2026, 9, 24, 12, 0)

    # 9am in New York is 1pm UTC (September daylight time).
    remind_at = Time.use_zone("America/New_York") { Time.zone.parse("2026-09-24 09:00") }
    assert_equal "2026-09-24T13:00:00.000Z", remind_at.utc.iso8601(3)

    saved_item = SavedItem.create!(user: @user, message: @message, remind_at:)

    travel_to Time.utc(2026, 9, 24, 12, 59)
    SavedItem::ReminderDispatcher.dispatch_due!
    assert_nil saved_item.reload.reminded_at

    travel_to Time.utc(2026, 9, 24, 13, 1)
    SavedItem::ReminderDispatcher.dispatch_due!
    assert_not_nil saved_item.reload.reminded_at

    assert_equal "message_reminder", ActivityItem.find_by!(user: @user, source: saved_item).event_type
  end

  test "a saver who lost room access is claimed without notifying" do
    saved_item = SavedItem.create!(user: @user, message: @message, remind_at: 1.minute.from_now)
    memberships(:david_designers).destroy!

    travel_to 2.minutes.from_now do
      assert_no_enqueued_jobs only: SavedItem::ReminderPushJob do
        SavedItem::ReminderDispatcher.dispatch_due!
      end
    end

    assert_not_nil saved_item.reload.reminded_at
    assert_nil ActivityItem.find_by(user: @user, source: saved_item)
  end

  test "every due reminder dispatches in one run" do
    david_item = SavedItem.create!(user: @user, message: @message, remind_at: 1.minute.from_now)
    jason_item = SavedItem.create!(user: users(:jason), message: messages(:second), remind_at: 1.minute.from_now)

    travel_to 2.minutes.from_now do
      assert_enqueued_jobs 2, only: SavedItem::ReminderPushJob do
        SavedItem::ReminderDispatcher.dispatch_due!
      end
    end

    assert_equal "message_reminder", ActivityItem.find_by!(user: @user, source: david_item).event_type
    assert_equal "message_reminder", ActivityItem.find_by!(user: users(:jason), source: jason_item).event_type
  end
end
