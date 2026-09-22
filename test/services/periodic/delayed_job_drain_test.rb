require "test_helper"

# A job class the drain can enqueue through Resque's real encode/decode and
# queue path. Plain Ruby: no hooks, no scheduled override.
class DelayedDrainTestJob
  def self.queue = :drain_test

  def self.perform(*) = nil
end

class Periodic::DelayedJobDrainTest < ActiveSupport::TestCase
  # No redis-server is available in this environment and fakeredis is not in
  # the bundle, so these tests run the real drain code against a minimal
  # in-memory stand-in for the Redis commands the drain path issues
  # (Resque.redis is stubbed; the method under test is never mocked).
  class FakeSchedulerRedis
    attr_reader :queues

    def initialize
      @lists = Hash.new { |hash, key| hash[key] = [] }
      @sets = Hash.new { |hash, key| hash[key] = Set.new }
      @zsets = Hash.new { |hash, key| hash[key] = {} }
      @queues = Hash.new { |hash, key| hash[key] = [] }
    end

    def rpush(key, value)
      @lists[key.to_s] << value
      @lists[key.to_s].size
    end

    def lpop(key)
      @lists[key.to_s].shift
    end

    def llen(key)
      @lists[key.to_s].size
    end

    def sadd(key, members)
      Array(members).each { |member| @sets[key.to_s] << member.to_s }
    end

    def srem(key, members)
      Array(members).count { |member| @sets[key.to_s].delete?(member.to_s) }
    end

    def zadd(key, score, member)
      @zsets[key.to_s][member.to_s] = score.to_i
    end

    def zrangebyscore(key, min, max, limit: nil)
      min = min == "-inf" ? -Float::INFINITY : min.to_i
      max = max == "+inf" ? Float::INFINITY : max.to_i
      members = @zsets[key.to_s].select { |_, score| score >= min && score <= max }
        .sort_by { |member, score| [ score, member ] }.map(&:first)
      limit ? members.slice(limit[0], limit[1]) || [] : members
    end

    def zrem(key, member)
      @zsets[key.to_s].delete(member.to_s) ? 1 : 0
    end

    def del(*keys)
      keys.flatten.count do |key|
        key = key.to_s
        [ @lists, @sets, @zsets ].any? { |store| !store[key].empty? && store.delete(key) }
      end
    end

    def watch(*)
      yield
    end

    def multi
      yield self
      []
    end

    def redis = self

    def unwatch = true

    def push_to_queue(queue, encoded_item)
      @queues[queue.to_s] << encoded_item
    end
  end

  setup do
    @fake_redis = FakeSchedulerRedis.new
    # data_store is aliased to redis at load time, so stub both.
    Resque.stubs(:redis).returns(@fake_redis)
    Resque.stubs(:data_store).returns(@fake_redis)

    # The reported failure state: this process booted while another
    # scheduler's master lock was live and memoized non-master forever.
    @had_master_memo = Resque::Scheduler.instance_variable_defined?(:@am_master)
    @previous_master_memo = Resque::Scheduler.instance_variable_get(:@am_master) if @had_master_memo
    Resque::Scheduler.instance_variable_set(:@am_master, false)

    @original_procline = $0
  end

  teardown do
    if @had_master_memo
      Resque::Scheduler.instance_variable_set(:@am_master, @previous_master_memo)
    else
      Resque::Scheduler.remove_instance_variable(:@am_master)
    end
    $0 = @original_procline
  end

  test "the old entry point raises TypeError once non-master is memoized" do
    Resque.delayed_push(1.minute.ago, delayed_job_hash)

    error = assert_raises(TypeError) { Resque::Scheduler.handle_delayed_items }
    assert_equal "nil can't be coerced into Integer", error.message
  end

  test "drain moves due delayed jobs onto their queues despite the stale lock" do
    Resque.delayed_push(1.minute.ago, delayed_job_hash(args: [ "due" ]))
    Resque.delayed_push(1.hour.from_now, delayed_job_hash(args: [ "future" ]))

    Periodic::DelayedJobDrain.drain_due!

    queued = @fake_redis.queues["drain_test"].map { |encoded| Resque.decode(encoded) }
    assert_equal [ "due" ], queued.flat_map { |job| job["args"] }
  end

  test "drain leaves future items queued and rewrites no process title" do
    Resque.delayed_push(1.minute.ago, delayed_job_hash(args: [ "due" ]))
    Resque.delayed_push(1.hour.from_now, delayed_job_hash(args: [ "future" ]))

    Periodic::DelayedJobDrain.drain_due!

    assert_equal $0, @original_procline
    remaining = @fake_redis.zrangebyscore(:delayed_queue_schedule, "-inf", "+inf")
    assert_equal 1, remaining.size
  end

  test "a runner tick drains delayed jobs despite the stale lock" do
    Event::ReminderDispatcher.stubs(:dispatch_due!)
    Resque.delayed_push(1.minute.ago, delayed_job_hash(args: [ "due" ]))

    Periodic::Runner.new(reminders_interval: 30, retention_interval: 86_400).tick

    queued = @fake_redis.queues["drain_test"].map { |encoded| Resque.decode(encoded) }
    assert_equal [ "due" ], queued.flat_map { |job| job["args"] }
    assert_equal $0, @original_procline
  end

  private
    def delayed_job_hash(args: [ "work" ])
      { class: "DelayedDrainTestJob", args:, queue: "drain_test" }
    end
end
