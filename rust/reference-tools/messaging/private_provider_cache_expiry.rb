# Deterministically reproduce the Icons TTL expiring between a warm-up request
# and its measured request. This changes only fixture cache state, not the clock
# used by Rails, and never sleeps. The generator must still reproduce the pinned
# private_provider_pages.json (including its 32/33 read counts).
module ExpireIconsAfterFixtureWarmup
  def get(path, **options)
    result = super
    if path.start_with?('/searches?')
      @fixture_search_requests = (@fixture_search_requests || 0) + 1
      if @fixture_search_requests.odd?
        Icons.instance_variable_set(:@custom_cache_at, -Float::INFINITY)
      end
    end
    result
  end
end
ActionDispatch::Integration::Session.prepend(ExpireIconsAfterFixtureWarmup)
load File.join(__dir__, 'private_provider_pages.rb')
