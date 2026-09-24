ENV["RAILS_ENV"] ||= "test"
require_relative "../config/environment"

require "rails/test_help"
require "minitest/unit"
require "mocha/minitest"
require "webmock/minitest"
require "turbo/broadcastable/test_helper"

WebMock.enable!

class ActiveSupport::TestCase
  include ActiveJob::TestHelper

  parallelize(workers: :number_of_processors)

  # Setup all fixtures in test/fixtures/*.yml for all tests in alphabetical order.
  fixtures :all

  set_fixture_class twitter_posts: Twitter::Post, twitter_post_references: Twitter::PostReference

  include SessionTestHelper, BotKeyTestHelper, MentionTestHelper, TurboTestHelper, DnsTestHelper, WorkspaceIconTestHelper, TwoFactorTestHelper

  setup do
    ActionCable.server.pubsub.clear
    Icons.expire_custom_cache!

    Rails.configuration.tap do |config|
      config.x.web_push_pool.shutdown
      config.x.web_push_pool = WebPush::Pool.new \
        invalid_subscription_handler: config.x.web_push_pool.invalid_subscription_handler
    end

    WebMock.disable_net_connect!
    # Controller rate limits count against the test-only memory store
    # (see test.rb); clear it so no test inherits another's counts.
    ActionController::Base.cache_store.clear

    # Webhook and unfurl deliveries resolve through the SSRF guard; answer
    # every hostname with a public address so no test depends on real DNS.
    # IP literals skip the resolver, and per-test stubs (DnsTestHelper)
    # override this for private-address and failure cases.
    Resolv.stubs(:getaddresses).returns([ "93.184.216.34" ])
  end

  teardown do
    WebMock.reset!
  end
end

# Suite-wide clock shifter for hunting date-dependent tests (see
# test/test_helpers/clock_offset.rb). TEST_CLOCK_OFFSET_DAYS=0 or unset
# runs the suite at the real time.
TEST_CLOCK_OFFSET_DAYS = ENV.fetch("TEST_CLOCK_OFFSET_DAYS", "0").to_i
TEST_CLOCK_OFFSET = TEST_CLOCK_OFFSET_DAYS.zero? ? nil : TEST_CLOCK_OFFSET_DAYS.days

if TEST_CLOCK_OFFSET
  require_relative "test_helpers/clock_offset"
  ActiveSupport::TestCase.prepend(ClockOffsetTestHelper)
end
