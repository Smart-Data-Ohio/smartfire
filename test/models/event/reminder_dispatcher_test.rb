require "test_helper"

class Event::ReminderDispatcherTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    @organizer = users(:david)
    @event = @room.events.create!(
      organizer: @organizer, title: "Standup", starts_at: 10.minutes.from_now, time_zone: "UTC"
    )
    @event.attendances.create!(user: users(:jason), response: :going)
    @event.attendances.create!(user: users(:jz), response: :maybe)
    @event.attendances.create!(user: users(:kevin), response: :declined)
  end

  test "a due event reminds each going and maybe attendee once and enqueues push" do
    assert_enqueued_with(job: Event::ReminderPushJob, args: [ @event ]) do
      Event::ReminderDispatcher.dispatch_due!
    end

    assert_not_nil @event.reload.reminded_at

    [ @organizer, users(:jason), users(:jz) ].each do |attendee|
      items = ActivityItem.where(user: attendee, source: @event)
      assert_equal 1, items.count
      assert_equal "event_reminder", items.first.event_type
      assert_predicate items.first, :unread?
    end
  end

  test "members with notifications off keep their invitation and get no reminder" do
    memberships(:jason_designers).update!(involvement: "nothing")

    Event::ReminderDispatcher.dispatch_due!

    assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: @event).event_type
    assert_equal "event_reminder", ActivityItem.find_by!(user: users(:jz), source: @event).event_type
  end

  test "an attendee with reminders switched off keeps the invitation while others are reminded" do
    users(:jason).update!(inbox_preferences: { "event_reminders" => false })

    assert_enqueued_with(job: Event::ReminderPushJob, args: [ @event ]) do
      Event::ReminderDispatcher.dispatch_due!
    end

    assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: @event).event_type
    assert_equal "event_reminder", ActivityItem.find_by!(user: users(:jz), source: @event).event_type

    @event.update_with_announcement!({ starts_at: 2.days.from_now }, actor: @organizer)
    assert_equal "event_update", ActivityItem.find_by!(user: users(:jason), source: @event).event_type

    follow_up = @room.events.create!(
      organizer: @organizer, title: "Follow-up", starts_at: 2.days.from_now, time_zone: "UTC"
    )
    assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: follow_up).event_type
  end

  test "declined attendees keep their invitation and get no reminder" do
    Event::ReminderDispatcher.dispatch_due!

    kevin_item = ActivityItem.find_by!(user: users(:kevin), source: @event)
    assert_equal "event_invitation", kevin_item.event_type
  end

  test "cancelled events are skipped" do
    @event.cancel!(actor: @organizer)
    clear_enqueued_jobs

    assert_no_enqueued_jobs only: Event::ReminderPushJob do
      Event::ReminderDispatcher.dispatch_due!
    end

    assert_nil @event.reload.reminded_at
    assert_equal "event_cancelled", ActivityItem.find_by!(user: users(:jason), source: @event).event_type
  end

  test "only events starting soon are due" do
    @event.update!(starts_at: 2.days.from_now)

    assert_no_enqueued_jobs only: Event::ReminderPushJob do
      Event::ReminderDispatcher.dispatch_due!
    end

    assert_nil @event.reload.reminded_at
    assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: @event).event_type
  end

  test "recently started events are still reminded but old ones are not" do
    @event.update!(starts_at: 3.minutes.ago)
    old_event = @room.events.create!(
      organizer: @organizer, title: "Old standup", starts_at: 2.hours.ago, time_zone: "UTC"
    )

    Event::ReminderDispatcher.dispatch_due!

    assert_not_nil @event.reload.reminded_at
    assert_equal "event_reminder", ActivityItem.find_by!(user: users(:jason), source: @event).event_type
    assert_nil old_event.reload.reminded_at
    assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: old_event).event_type
  end

  test "a second run creates nothing" do
    Event::ReminderDispatcher.dispatch_due!
    clear_enqueued_jobs

    assert_no_difference -> { ActivityItem.where(source: @event).count } do
      assert_no_enqueued_jobs only: Event::ReminderPushJob do
        Event::ReminderDispatcher.dispatch_due!
      end
    end
  end

  test "consecutive occurrences of a series are each reminded once at their own time" do
    # Derive the until-date from the start's own date: Date.current + 1
    # equals the start date when 10.minutes.from_now crosses midnight UTC,
    # which the recurrence validation correctly rejects.
    starts_at = 10.minutes.from_now
    series = @room.events.create!(
      organizer: @organizer, title: "Daily sync", starts_at:, time_zone: "UTC",
      recurrence_rule: "daily", recurrence_until: starts_at.in_time_zone("UTC").to_date + 1
    )
    occurrences = series.series_events.to_a
    assert_equal 2, occurrences.size
    occurrences.second.update!(starts_at: 70.minutes.from_now)
    occurrences.each { |occurrence| occurrence.attendances.create!(user: users(:jason), response: :going) }

    Event::ReminderDispatcher.dispatch_due!

    assert_not_nil occurrences.first.reload.reminded_at
    assert_nil occurrences.second.reload.reminded_at
    assert_equal "event_reminder", ActivityItem.find_by!(user: users(:jason), source: occurrences.first).event_type

    travel_to 1.hour.from_now do
      Event::ReminderDispatcher.dispatch_due!
    end

    assert_not_nil occurrences.second.reload.reminded_at
    assert_equal "event_reminder", ActivityItem.find_by!(user: users(:jason), source: occurrences.second).event_type
  end

  test "a series starting just before midnight still builds its occurrences" do
    # At 23:50 UTC, 10.minutes.from_now lands on the next date; the
    # until-date must follow the start's date, not Date.current.
    travel_to Time.utc(2026, 9, 22, 23, 50) do
      starts_at = 10.minutes.from_now
      series = @room.events.create!(
        organizer: @organizer, title: "Midnight sync", starts_at:, time_zone: "UTC",
        recurrence_rule: "daily", recurrence_until: starts_at.in_time_zone("UTC").to_date + 1
      )

      assert_equal 2, series.series_events.count
      assert_equal Date.new(2026, 9, 23), starts_at.in_time_zone("UTC").to_date
    end
  end

  test "events in soft-deleted rooms are skipped" do
    # Memberships stay intact so only the soft-delete can skip the event:
    # begin_destroy! would also fail the organizer check on the remind stamp.
    @room.update_columns(deleted_at: Time.current)

    assert_no_enqueued_jobs only: Event::ReminderPushJob do
      assert_no_difference -> { ActivityItem.where(source: @event).count } do
        Event::ReminderDispatcher.dispatch_due!
      end
    end

    assert_nil @event.reload.reminded_at
  end

  test "a stale event after runner downtime is claimed without reminding or pushing" do
    # The 15-minute window passed while the runner was down; the event is
    # still inside the grace window, so it is claimed like any due event,
    # but neither the inbox nor the push goes out because the moment passed.
    @event.update!(starts_at: 2.days.from_now)
    stale = @room.events.create!(
      organizer: @organizer, title: "Missed standup",
      starts_at: 30.minutes.ago, ends_at: 30.minutes.from_now, time_zone: "UTC"
    )
    stale.attendances.create!(user: users(:jason), response: :going)

    assert_no_enqueued_jobs only: Event::ReminderPushJob do
      Event::ReminderDispatcher.dispatch_due!
    end

    assert_not_nil stale.reload.reminded_at
    assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: stale).event_type
  end

  test "an ended event is claimed without reminding" do
    @event.update!(starts_at: 2.days.from_now)
    ended = @room.events.create!(
      organizer: @organizer, title: "Finished standup",
      starts_at: 50.minutes.ago, ends_at: 5.minutes.ago, time_zone: "UTC"
    )
    ended.attendances.create!(user: users(:jason), response: :going)

    assert_no_enqueued_jobs only: Event::ReminderPushJob do
      Event::ReminderDispatcher.dispatch_due!
    end

    assert_not_nil ended.reload.reminded_at
    assert_equal "event_invitation", ActivityItem.find_by!(user: users(:jason), source: ended).event_type
  end

  test "one failing event does not stop the others" do
    other = @room.events.create!(
      organizer: @organizer, title: "Other standup", starts_at: 10.minutes.from_now, time_zone: "UTC"
    )
    Event.any_instance.stubs(:remind_attendees!).raises(RuntimeError).then.returns(nil)

    Event::ReminderDispatcher.dispatch_due!

    assert_nil @event.reload.reminded_at
    assert_not_nil other.reload.reminded_at
  end
end
