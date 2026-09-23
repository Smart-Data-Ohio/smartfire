require "test_helper"
require_relative "../support/system_test_chrome_profile"

class SystemTestChromeProfileTest < ActiveSupport::TestCase
  test "dir points at the repo tmp with the current PID" do
    dir = SystemTestChromeProfile.dir

    assert_equal Rails.root.join("tmp/chrome-profiles/#{Process.pid}").to_s, dir
    assert_not_includes dir, "/tmp/org.chromium"
  end

  test "next_dir hands out a fresh directory per browser" do
    first = SystemTestChromeProfile.next_dir
    second = SystemTestChromeProfile.next_dir

    assert_not_equal first, second
    assert_match %r{\A#{Regexp.escape(Rails.root.join("tmp/chrome-profiles").to_s)}/#{Process.pid}-\d+\z}, first
  end

  test "detached_options swaps the profile dir without touching the shared options" do
    shared = Selenium::WebDriver::Chrome::Options.new
    shared.add_argument("--mute-audio")
    shared.add_argument("--user-data-dir=/tmp/placeholder")

    first = SystemTestChromeProfile.detached_options(shared)
    second = SystemTestChromeProfile.detached_options(shared)

    [ first, second ].each do |copy|
      args = copy.as_json["goog:chromeOptions"]["args"]
      assert_includes args, "--mute-audio"
      assert_equal 1, args.grep(%r{\A--user-data-dir=#{Regexp.escape(Rails.root.join("tmp/chrome-profiles").to_s)}/}).size
    end
    assert_not_equal first.as_json, second.as_json

    shared_args = shared.as_json["goog:chromeOptions"]["args"]
    assert_equal [ "--mute-audio", "--user-data-dir=/tmp/placeholder" ], shared_args
  end

  test "cleanup_own! removes every directory of this process" do
    base = Rails.root.join("tmp/chrome-profiles")
    own = [ base.join(Process.pid.to_s), base.join("#{Process.pid}-3") ]
    other = base.join("987654320-1")
    own.each { |dir| FileUtils.mkdir_p(dir) }
    FileUtils.mkdir_p(other)

    begin
      SystemTestChromeProfile.cleanup_own!

      own.each { |dir| assert_not File.exist?(dir) }
      assert File.exist?(other)
    ensure
      FileUtils.rm_rf(own)
      FileUtils.rm_rf(other)
    end
  end

  test "cleanup_stale! removes dead-PID profiles and keeps live ones" do
    base = Rails.root.join("tmp/chrome-profiles")
    dead = base.join("987654321")
    dead_browser = base.join("987654321-2")
    live = base.join(Process.pid.to_s)
    live_browser = base.join("#{Process.pid}-9")
    [ dead, dead_browser, live, live_browser ].each { |dir| FileUtils.mkdir_p(dir) }
    File.write(dead.join("Preferences"), "{}")

    begin
      SystemTestChromeProfile.cleanup_stale!

      assert_not File.exist?(dead)
      assert_not File.exist?(dead_browser)
      assert File.exist?(live)
      assert File.exist?(live_browser)
    ensure
      FileUtils.rm_rf(dead)
      FileUtils.rm_rf(dead_browser)
      FileUtils.rm_rf(live)
      FileUtils.rm_rf(live_browser)
    end
  end

  test "cleanup_stale! ignores non-PID entries" do
    base = Rails.root.join("tmp/chrome-profiles")
    other = base.join("README")
    FileUtils.mkdir_p(base)
    File.write(other, "do not delete\n")

    begin
      SystemTestChromeProfile.cleanup_stale!

      assert File.exist?(other)
    ensure
      FileUtils.rm_f(other)
    end
  end
end
