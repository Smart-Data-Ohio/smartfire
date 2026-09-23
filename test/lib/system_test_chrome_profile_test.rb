require "test_helper"
require_relative "../support/system_test_chrome_profile"

class SystemTestChromeProfileTest < ActiveSupport::TestCase
  test "dir points at the repo tmp with the current PID" do
    dir = SystemTestChromeProfile.dir

    assert_equal Rails.root.join("tmp/chrome-profiles/#{Process.pid}").to_s, dir
    assert_not_includes dir, "/tmp/org.chromium"
  end

  test "cleanup_stale! removes dead-PID profiles and keeps live ones" do
    base = Rails.root.join("tmp/chrome-profiles")
    dead = base.join("987654321")
    live = base.join(Process.pid.to_s)
    FileUtils.mkdir_p(dead)
    FileUtils.mkdir_p(live)
    File.write(dead.join("Preferences"), "{}")

    begin
      SystemTestChromeProfile.cleanup_stale!

      assert_not File.exist?(dead)
      assert File.exist?(live)
    ensure
      FileUtils.rm_rf(dead)
      FileUtils.rm_rf(live)
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
