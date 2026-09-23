require "test_helper"

class SavedItemTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
    @message = messages(:first)
  end

  test "saving defaults to in progress without a reminder" do
    saved_item = SavedItem.create!(user: @user, message: @message)

    assert_predicate saved_item, :in_progress?
    assert_nil saved_item.remind_at
    assert_not saved_item.reminder_pending?
  end

  test "one saved item per user per message" do
    SavedItem.create!(user: @user, message: @message)

    assert_raises ActiveRecord::RecordInvalid do
      SavedItem.create!(user: @user, message: @message)
    end

    assert_nothing_raised do
      SavedItem.create!(user: users(:jason), message: @message)
    end
  end

  test "reminders must be in the future" do
    assert_raises ActiveRecord::RecordInvalid do
      SavedItem.create!(user: @user, message: @message, remind_at: 1.minute.ago)
    end

    assert_nothing_raised do
      SavedItem.create!(user: @user, message: messages(:second), remind_at: 1.minute.from_now)
    end
  end

  test "accessible items hide rooms the user can no longer see" do
    saved_item = SavedItem.create!(user: @user, message: @message)
    other_room_item = SavedItem.create!(user: @user, message: messages(:fourth))

    assert_equal [ saved_item, other_room_item ].map(&:id).sort, SavedItem.accessible_to(@user).ids.sort

    memberships(:david_designers).destroy!

    assert_equal [ other_room_item.id ], SavedItem.accessible_to(@user).ids

    @message.room.memberships.grant_to(@user)

    assert_includes SavedItem.accessible_to(@user).ids, saved_item.id
  end

  test "accessible items hide other users, bots, and deleted rooms" do
    SavedItem.create!(user: @user, message: @message)
    SavedItem.create!(user: users(:jason), message: @message)

    assert_equal [ @message.id ], SavedItem.accessible_to(@user).map(&:message_id)
    assert_empty SavedItem.accessible_to(users(:bender))

    @message.room.update!(deleted_at: Time.current)

    assert_empty SavedItem.accessible_to(@user)
  end

  test "due reminders are pending items whose time has come" do
    due = SavedItem.create!(user: @user, message: @message, remind_at: 1.hour.from_now)
    travel_to 61.minutes.from_now do
      assert_includes SavedItem.due_reminders, due
    end

    assert_not_includes SavedItem.due_reminders, due
    assert_not_includes SavedItem.due_reminders, SavedItem.create!(user: @user, message: messages(:second))

    fired = SavedItem.create!(user: @user, message: messages(:third), remind_at: 1.hour.from_now)
    travel_to 61.minutes.from_now do
      fired.update!(reminded_at: Time.current)
      assert_not_includes SavedItem.due_reminders, fired
    end
  end

  test "a firing reminder transitions the existing inbox item" do
    mention = ActivityItem.create!(user: @user, source: @message, event_type: "mention")
    mention.mark_read!
    saved_item = SavedItem.create!(user: @user, message: @message, remind_at: 1.minute.from_now)

    item = nil
    assert_no_difference -> { ActivityItem.where(user: @user, source: @message).count } do
      item = saved_item.transition_reminder_item!
    end

    assert_equal "message_reminder", item.event_type
    assert_predicate item, :unread?
  end

  test "destroying the message destroys its saved items" do
    SavedItem.create!(user: @user, message: @message)

    assert_difference -> { SavedItem.count }, -1 do
      @message.destroy!
    end
  end

  test "setting a new reminder time after a firing re-arms the reminder" do
    saved_item = SavedItem.create!(user: @user, message: @message, remind_at: 1.minute.from_now)

    travel_to 2.minutes.from_now do
      SavedItem::ReminderDispatcher.dispatch_due!
    end
    assert_not_nil saved_item.reload.reminded_at

    saved_item.update!(remind_at: 1.hour.from_now)

    assert_nil saved_item.reload.reminded_at
    assert_predicate saved_item, :reminder_pending?

    travel_to 61.minutes.from_now do
      assert_includes SavedItem.due_reminders, saved_item
    end
  end
end
