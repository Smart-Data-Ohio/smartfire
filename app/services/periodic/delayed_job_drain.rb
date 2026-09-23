module Periodic
  # Moves due resque-scheduler delayed jobs (ActiveJob retries with backoff)
  # onto their queues without requiring scheduler mastership.
  #
  # Resque::Scheduler.handle_delayed_items only enqueues `if am_master`, and
  # am_master is memoized for the life of the process (resque-scheduler
  # 4.11.0, scheduler.rb: `@am_master = master? unless
  # defined?(@am_master)`), against a hostname:pid lock with a 3-minute TTL
  # (scheduler/lock/base.rb). A process that boots while another scheduler's
  # lock is live therefore memoizes false forever — and from then on every
  # tick raises `TypeError: nil can't be coerced into Integer` (`count +=
  # actual_batch_size` in enqueue_delayed_items_for_timestamp, where the
  # batch size stays unset for a non-master) without enqueueing anything.
  #
  # The periodic runner is the only scheduler, so there is no election to
  # win: loop over Resque.next_delayed_timestamp, which returns only items
  # at or before now (delaying_extensions.rb), and hand each timestamp to
  # Resque::Scheduler.enqueue_next_item, which pops and enqueues every item
  # with no master check (scheduler.rb; also used by the resque-web "queue
  # now" path in server.rb). Unlike handle_delayed_items it never calls
  # procline, so the process title is left alone.
  module DelayedJobDrain
    def self.drain_due!(now: Time.current)
      while (timestamp = Resque.next_delayed_timestamp(now))
        loop do
          break unless Resque::Scheduler.enqueue_next_item(timestamp)
        end
      end
    end
  end
end
