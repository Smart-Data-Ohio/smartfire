require "test_helper"

class Event::ReminderPusherTest < ActiveSupport::TestCase
  test "pushes the reminder to going and maybe attendees who are still members" do
    event = events(:launch_party)
    event.update!(starts_at: 15.minutes.from_now, ends_at: 75.minutes.from_now)
    memberships(:jason_designers).destroy!

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, subscriptions|
      payload.fetch(:body) == "Starts in 15 minutes: Launch party planning" &&
        payload.fetch(:path) == Rails.application.routes.url_helpers.room_event_path(event.room, event) &&
        subscriptions.map(&:user_id) == [ users(:david).id ]
    end

    Event::ReminderPusher.new(event:).push
  end

  test "push reminders ignore the event_reminders inbox switch" do
    event = events(:launch_party)
    users(:david).update!(inbox_preferences: { "event_reminders" => false })

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |_payload, subscriptions|
      subscriptions.map(&:user_id).include?(users(:david).id)
    end

    Event::ReminderPusher.new(event:).push
  end

  test "the push body names the venue" do
    voice = Rooms::Voice.create_for({ name: "Lounge", creator: users(:david) }, users: [ users(:david) ])
    event = events(:launch_party)
    event.update!(venue_room_id: voice.id, starts_at: 15.minutes.from_now, ends_at: 75.minutes.from_now)

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, _subscriptions|
      payload.fetch(:body) == "Starts in 15 minutes: Launch party planning in Lounge"
    end

    Event::ReminderPusher.new(event:).push
  end

  test "a direct room reminder is titled by the organizer" do
    room = rooms(:david_and_jason)
    event = room.events.create!(
      organizer: users(:david), title: "Quick call", starts_at: 10.minutes.from_now, time_zone: "UTC"
    )
    event.attendances.create!(user: users(:jason), response: :going)

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, _subscriptions|
      payload.fetch(:title) == "David"
    end

    Event::ReminderPusher.new(event:).push
  end

  test "the push body counts down the actual minutes" do
    event = rooms(:designers).events.create!(
      organizer: users(:david), title: "Quick sync", starts_at: 3.minutes.from_now, time_zone: "UTC"
    )

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, _subscriptions|
      payload.fetch(:body) == "Starts in 3 minutes: Quick sync"
    end

    Event::ReminderPusher.new(event:).push
  end

  test "the push body uses the singular minute" do
    event = rooms(:designers).events.create!(
      organizer: users(:david), title: "Quick sync", starts_at: 1.minute.from_now + 20.seconds, time_zone: "UTC"
    )

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, _subscriptions|
      payload.fetch(:body) == "Starts in 1 minute: Quick sync"
    end

    Event::ReminderPusher.new(event:).push
  end

  test "an event starting now says so" do
    event = rooms(:designers).events.create!(
      organizer: users(:david), title: "Quick sync", starts_at: 30.seconds.from_now, time_zone: "UTC"
    )

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, _subscriptions|
      payload.fetch(:body) == "Starting now: Quick sync"
    end

    Event::ReminderPusher.new(event:).push
  end

  test "a recently started event still says starting now" do
    event = rooms(:designers).events.create!(
      organizer: users(:david), title: "Quick sync",
      starts_at: 4.minutes.ago, ends_at: 1.hour.from_now, time_zone: "UTC"
    )

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, _subscriptions|
      payload.fetch(:body) == "Starting now: Quick sync"
    end

    Event::ReminderPusher.new(event:).push
  end

  test "an event that already ended is skipped" do
    event = rooms(:designers).events.create!(
      organizer: users(:david), title: "Missed sync",
      starts_at: 2.hours.ago, ends_at: 1.minute.ago, time_zone: "UTC"
    )

    Rails.configuration.x.web_push_pool.expects(:queue).never

    Event::ReminderPusher.new(event:).push
  end

  test "an event that started long ago is skipped" do
    event = rooms(:designers).events.create!(
      organizer: users(:david), title: "Missed sync",
      starts_at: 30.minutes.ago, ends_at: 1.hour.from_now, time_zone: "UTC"
    )

    Rails.configuration.x.web_push_pool.expects(:queue).never

    Event::ReminderPusher.new(event:).push
  end
end
