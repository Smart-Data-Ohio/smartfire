require "test_helper"

class Periodic::RunnerTest < ActiveSupport::TestCase
  setup do
    @runner = Periodic::Runner.new(reminders_interval: 30, retention_interval: 86_400)
  end

  test "a tick moves due delayed jobs onto their queues" do
    Resque::Scheduler.expects(:handle_delayed_items).once
    Event::ReminderDispatcher.stubs(:dispatch_due!)

    @runner.tick
  end

  test "a tick runs every task the first time" do
    Resque::Scheduler.stubs(:handle_delayed_items)
    Event::ReminderDispatcher.expects(:dispatch_due!).once

    assert_enqueued_with(job: Retention::PruneJob) do
      assert_equal [ "delayed jobs", "event reminders", "retention prune" ], @runner.tick
    end
  end

  test "a tick skips tasks whose interval has not elapsed" do
    Resque::Scheduler.stubs(:handle_delayed_items)
    Event::ReminderDispatcher.stubs(:dispatch_due!)

    travel_to Time.current do
      @runner.tick
      assert_no_enqueued_jobs { assert_empty @runner.tick }
    end
  end

  test "a tick reruns tasks whose interval has elapsed but not the daily prune" do
    Resque::Scheduler.stubs(:handle_delayed_items)
    Event::ReminderDispatcher.stubs(:dispatch_due!)

    now = Time.current
    travel_to(now) { @runner.tick }

    travel_to(now + 30.seconds) do
      Resque::Scheduler.unstub(:handle_delayed_items)
      Event::ReminderDispatcher.unstub(:dispatch_due!)
      Resque::Scheduler.expects(:handle_delayed_items).once
      Event::ReminderDispatcher.expects(:dispatch_due!).once

      assert_no_enqueued_jobs do
        assert_equal [ "delayed jobs", "event reminders" ], @runner.tick
      end
    end
  end

  test "a tick enqueues the retention prune once its interval has elapsed" do
    Resque::Scheduler.stubs(:handle_delayed_items)
    Event::ReminderDispatcher.stubs(:dispatch_due!)

    now = Time.current
    travel_to(now) { @runner.tick }

    travel_to(now + 86_400.seconds) do
      assert_enqueued_with(job: Retention::PruneJob) { @runner.tick }
    end
  end

  test "a failing task is logged and does not stop the other tasks" do
    Resque::Scheduler.stubs(:handle_delayed_items).raises(Redis::BaseConnectionError, "Redis down")
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
