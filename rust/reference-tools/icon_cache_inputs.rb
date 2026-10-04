# Explicit input state for request-query oracles, using the application's public cache API.
# Historical extra reads were Icons' stamp query after its one-second monotonic TTL.
module OracleIconCacheInputs
  def self.prepare(state)
    raise "unknown icon cache input: #{state}" unless %w[initial warm expired].include?(state)
    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    loaded = Icons.instance_variable_defined?(:@custom_stamp)
    if state == 'initial'
      raise 'initial icon cache input was already loaded' if loaded
    elsif loaded || state == 'expired'
      # A cache hit does not reset its timestamp: force a real stamp refresh before
      # capture so a warm input cannot begin with an almost-expired memo.
      Icons.expire_custom_cache!
      ActiveRecord::Base.uncached { Icons.client_icon_names }
      Icons.expire_custom_cache! if state == 'expired'
    else
      # Redirects do not load this catalog. Preserve startup until its first render.
      state = 'initial'
    end
    [started, state]
  end

  def self.verify!(started, name:)
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started
    # Reject a slow capture instead of recording accidental TTL-expiry SQL under load.
    raise "icon cache input elapsed beyond TTL for #{name}: #{elapsed}s" if elapsed >= Icons::CUSTOM_CACHE_TTL_SECONDS
    elapsed
  end
end
