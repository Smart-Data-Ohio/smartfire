module Periodic
  # The single loop behind the `periodic` Procfile process (bin/periodic).
  # Each tick runs every task whose interval has elapsed; a failing task is
  # logged without stopping the other tasks or the loop. Delayed ActiveJob
  # retries, event reminders, stuck room-destroy recovery, stranded agent
  # webhook and stuck GitHub-claim recovery, the expired presence-lease
  # sweep, and the daily retention prune all live here so production needs
  # no extra long-running process for any of them. Add a sweeper by
  # appending to the task list below: a name, an interval in seconds, and
  # an idempotent callable.
  class Runner
    Task = Data.define(:name, :interval, :run)

    DELAYED_JOBS_INTERVAL = 30.seconds
    STUCK_ROOM_SWEEP_INTERVAL = 5.minutes
    AGENT_SWEEP_INTERVAL = 30.seconds
    PRESENCE_SWEEP_INTERVAL = 1.minute

    def initialize(reminders_interval: 30, retention_interval: 24.hours.to_i, logger: Rails.logger)
      @tasks = [
        Task.new("delayed jobs", DELAYED_JOBS_INTERVAL, -> { Periodic::DelayedJobDrain.drain_due! }),
        Task.new("event reminders", reminders_interval, -> { Event::ReminderDispatcher.dispatch_due! }),
        Task.new("stuck rooms", STUCK_ROOM_SWEEP_INTERVAL, -> { Room::DestroyJob.reenqueue_stuck! }),
        Task.new("stranded agent webhooks", AGENT_SWEEP_INTERVAL, -> { Agent::Delivery.recover_stranded_webhooks! }),
        Task.new("stuck GitHub claims", AGENT_SWEEP_INTERVAL, -> { Github::PerformAgentActionJob.recover_stuck_claims! }),
        Task.new("retention prune", retention_interval, -> { Retention::PruneJob.perform_later }),
        Task.new("presence leases", PRESENCE_SWEEP_INTERVAL, -> { WorkspacePresenceLease.prune })
      ]
      @last_run = {}
      @logger = logger
      @stopping = false
    end

    def run
      %w[ INT TERM ].each { |signal| Signal.trap(signal) { @stopping = true } }

      until @stopping
        tick
        sleep tick_interval unless @stopping
      end
    end

    # One pass over the due tasks, extracted so tests can run it without
    # the sleep loop. Returns the names of the tasks that ran.
    def tick(now: Time.current)
      @tasks.filter_map do |task|
        next unless due?(task, now)

        # Only a success counts as a run: a Redis error at the tick must
        # not skip the daily prune for 24 hours.
        @last_run[task.name] = now if run_task(task)
        task.name
      end
    end

    def tick_interval
      @tasks.map(&:interval).min
    end

    def self.parse_interval!(value, name)
      interval = Float(value, exception: false)
      unless interval&.positive? && interval.finite?
        raise ArgumentError, "#{name} must be a positive finite number"
      end

      interval
    end

    private
      def due?(task, now)
        last = @last_run[task.name]
        last.nil? || last + task.interval <= now
      end

      def run_task(task)
        task.run.call
        true
      rescue => error
        @logger.error "Periodic #{task.name} failed: #{error.class}: #{error.message}"
        false
      end
  end
end
