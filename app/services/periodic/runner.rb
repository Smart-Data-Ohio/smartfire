module Periodic
  # The single loop behind the `periodic` Procfile process (bin/periodic).
  # Each tick runs every task whose interval has elapsed; a failing task is
  # logged without stopping the other tasks or the loop. Delayed ActiveJob
  # retries, event and saved-item reminders, scheduled-message dispatch,
  # poll closing, stuck room-destroy recovery, stranded agent webhook and
  # stuck GitHub/Fizzy-claim recovery, the expired presence-lease sweep,
  # the one-time plaintext bot-token clearing, and the daily retention prune all live here so production needs no
  # extra long-running process for any of them. Add a sweeper by appending
  # to the task list below: a name, an interval in seconds, and an
  # idempotent callable.
  class Runner
    Task = Data.define(:name, :interval, :run)

    DELAYED_JOBS_INTERVAL = 30.seconds
    STUCK_ROOM_SWEEP_INTERVAL = 5.minutes
    AGENT_SWEEP_INTERVAL = 30.seconds
    BOT_TOKEN_CLEAR_INTERVAL = 24.hours.to_i
    PRESENCE_SWEEP_INTERVAL = 1.minute

    def initialize(reminders_interval: 30, retention_interval: 24.hours.to_i, logger: Rails.logger)
      bot_tokens_cleared = false
      clear_bot_tokens_once = -> do
        unless bot_tokens_cleared
          Bots::ClearPlaintextTokens.run!
          bot_tokens_cleared = true
        end
      end

      @tasks = [
        Task.new("delayed jobs", DELAYED_JOBS_INTERVAL, -> { Periodic::DelayedJobDrain.drain_due! }),
        Task.new("event reminders", reminders_interval, -> { Event::ReminderDispatcher.dispatch_due! }),
        Task.new("saved item reminders", reminders_interval, -> { SavedItem::ReminderDispatcher.dispatch_due! }),
        Task.new("scheduled messages", reminders_interval, -> { ScheduledMessage::Dispatcher.dispatch_due! }),
        Task.new("poll closing", reminders_interval, -> { Poll.close_due! }),
        Task.new("stuck rooms", STUCK_ROOM_SWEEP_INTERVAL, -> { Room::DestroyJob.reenqueue_stuck! }),
        Task.new("stranded agent webhooks", AGENT_SWEEP_INTERVAL, -> { Agent::Delivery.recover_stranded_webhooks! }),
        Task.new("stuck GitHub claims", AGENT_SWEEP_INTERVAL, -> { Github::PerformAgentActionJob.recover_stuck_claims! }),
        Task.new("stuck Fizzy claims", AGENT_SWEEP_INTERVAL, -> { Fizzy::PerformAgentActionJob.recover_stuck_claims! }),
        Task.new("calendar push channels", Calendar::PushChannel::RENEW_INTERVAL, -> { Calendar::PushChannel.renew_expiring! }),
        Task.new("clear plaintext bot tokens", BOT_TOKEN_CLEAR_INTERVAL, clear_bot_tokens_once),
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
