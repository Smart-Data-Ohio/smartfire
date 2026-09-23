require "test_helper"

class Periodic::RunnerTest < ActiveSupport::TestCase
  setup do
    @runner = Periodic::Runner.new(reminders_interval: 30, retention_interval: 86_400)
  end

  test "a tick moves due delayed jobs onto their queues" do
    Periodic::DelayedJobDrain.expects(:drain_due!).once
    Event::ReminderDispatcher.stubs(:dispatch_due!)
    SavedItem::ReminderDispatcher.stubs(:dispatch_due!)

    @runner.tick
  end

  test "a tick runs every task the first time" do
    Periodic::DelayedJobDrain.stubs(:drain_due!)
    Event::ReminderDispatcher.expects(:dispatch_due!).once

    assert_enqueued_with(job: Retention::PruneJob) do
      assert_equal [ "delayed jobs", "event reminders", "saved item reminders", "stuck rooms", "stranded agent webhooks", "stuck GitHub claims", "clear plaintext bot tokens", "retention prune" ], @runner.tick
    end
  end

  test "a tick skips tasks whose interval has not elapsed" do
    Periodic::DelayedJobDrain.stubs(:drain_due!)
    Event::ReminderDispatcher.stubs(:dispatch_due!)
    SavedItem::ReminderDispatcher.stubs(:dispatch_due!)

    travel_to Time.current do
      @runner.tick
      assert_no_enqueued_jobs { assert_empty @runner.tick }
    end
  end

  test "a tick reruns tasks whose interval has elapsed but not the daily prune" do
    Periodic::DelayedJobDrain.stubs(:drain_due!)
    Event::ReminderDispatcher.stubs(:dispatch_due!)
    SavedItem::ReminderDispatcher.stubs(:dispatch_due!)

    now = Time.current
    travel_to(now) { @runner.tick }

    travel_to(now + 30.seconds) do
      Periodic::DelayedJobDrain.unstub(:drain_due!)
      Event::ReminderDispatcher.unstub(:dispatch_due!)
      Periodic::DelayedJobDrain.expects(:drain_due!).once
      Event::ReminderDispatcher.expects(:dispatch_due!).once

      assert_no_enqueued_jobs do
        assert_equal [ "delayed jobs", "event reminders", "saved item reminders", "stranded agent webhooks", "stuck GitHub claims" ], @runner.tick
      end
    end
  end

  test "a tick re-enqueues destroys for rooms stuck as deleted once the sweep is due" do
    Periodic::DelayedJobDrain.stubs(:drain_due!)
    Event::ReminderDispatcher.stubs(:dispatch_due!)
    SavedItem::ReminderDispatcher.stubs(:dispatch_due!)

    stuck = Rooms::Closed.create_for({ name: "Stuck", creator: users(:david) }, users: [ users(:david) ])
    stuck.begin_destroy!
    stuck.update_columns(deleted_at: 11.minutes.ago)
    fresh = Rooms::Closed.create_for({ name: "Fresh", creator: users(:david) }, users: [ users(:david) ])
    fresh.begin_destroy!

    now = Time.current
    travel_to(now) do
      assert_enqueued_with(job: Room::DestroyJob, args: [ stuck.id ]) { @runner.tick }

      destroy_args = enqueued_jobs.select { |job| job[:job] == Room::DestroyJob }.map { |job| job[:args] }
      assert_equal [ [ stuck.id ] ], destroy_args
    end

    travel_to(now + 30.seconds) do
      assert_no_enqueued_jobs { @runner.tick }
    end

    # The first sweep claimed the room, so the next due sweep enqueues
    # nothing; once the claim expires the sweep re-enqueues.
    travel_to(now + 5.minutes) do
      assert_no_enqueued_jobs { @runner.tick }
    end

    travel_to(now + 11.minutes) do
      assert_enqueued_with(job: Room::DestroyJob, args: [ stuck.id ]) { @runner.tick }
    end
  end

  test "the plaintext bot-token clearing runs once per runner" do
    Periodic::DelayedJobDrain.stubs(:drain_due!)
    Event::ReminderDispatcher.stubs(:dispatch_due!)
    SavedItem::ReminderDispatcher.stubs(:dispatch_due!)

    bot = User.create_bot!(name: "Runner Clear")
    bot.update_columns(bot_token: "RunnerClear1")

    now = Time.current
    travel_to(now) { @runner.tick }
    assert_nil bot.reload.read_attribute(:bot_token)

    bot.update_columns(bot_token: "RunnerAgain2")
    travel_to(now + 25.hours) { @runner.tick }
    assert_equal "RunnerAgain2", bot.reload.read_attribute(:bot_token),
      "the one-time flag stops later ticks from clearing again"
  end

  test "a tick enqueues the retention prune once its interval has elapsed" do
    Periodic::DelayedJobDrain.stubs(:drain_due!)
    Event::ReminderDispatcher.stubs(:dispatch_due!)
    SavedItem::ReminderDispatcher.stubs(:dispatch_due!)

    now = Time.current
    travel_to(now) { @runner.tick }

    travel_to(now + 86_400.seconds) do
      assert_enqueued_with(job: Retention::PruneJob) { @runner.tick }
    end
  end

  test "a failed prune is retried instead of skipped for a day" do
    Periodic::DelayedJobDrain.stubs(:drain_due!)
    Event::ReminderDispatcher.stubs(:dispatch_due!)
    SavedItem::ReminderDispatcher.stubs(:dispatch_due!)

    now = Time.current
    Retention::PruneJob.stubs(:perform_later).raises(Redis::BaseConnectionError, "Redis down")
    travel_to(now) { @runner.tick }
    Retention::PruneJob.unstub(:perform_later)

    travel_to(now + 30.seconds) do
      assert_enqueued_with(job: Retention::PruneJob) { @runner.tick }
    end
  end

  test "a failing task is logged and does not stop the other tasks" do
    Periodic::DelayedJobDrain.stubs(:drain_due!).raises(Redis::BaseConnectionError, "Redis down")
    Event::ReminderDispatcher.expects(:dispatch_due!).once

    assert_enqueued_with(job: Retention::PruneJob) { @runner.tick }
  end

  test "the tick interval is the shortest task interval" do
    assert_equal 30, @runner.tick_interval
  end

  test "parse_interval! accepts positive finite numbers and rejects the rest" do
    assert_equal 30.0, Periodic::Runner.parse_interval!("30", "EXAMPLE_INTERVAL")

    [ "0", "-30", "bogus", "Infinity", nil ].each do |value|
      error = assert_raises(ArgumentError) { Periodic::Runner.parse_interval!(value, "EXAMPLE_INTERVAL") }
      assert_equal "EXAMPLE_INTERVAL must be a positive finite number", error.message
    end
  end
end
