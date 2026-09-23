require "test_helper"
require_relative "../support/system_test_chrome_profile"

class SystemTestChromeProfileTest < ActiveSupport::TestCase
  test "tmpdir is a per-PID dir under the home cache, off the /tmp tmpfs" do
    dir = SystemTestChromeProfile.tmpdir

    assert_equal File.join(SystemTestChromeProfile.base, Process.pid.to_s), dir
    assert_not_includes dir, "/tmp/org.chromium"
    assert File.directory?(dir), "expected tmpdir to create the directory"
  ensure
    SystemTestChromeProfile.cleanup_own!
  end

  test "a managed profile under tmpdir fits Chrome's socket path limit" do
    # Chrome exits FATAL when the SingletonSocket path exceeds sun_path
    # (108 bytes); chromedriver appends org.chromium.Chromium.XXXXXX.
    socket = File.join(SystemTestChromeProfile.tmpdir, "org.chromium.Chromium.XXXXXX/SingletonSocket")

    assert_operator socket.bytesize, :<, 108,
      "expected the profile socket path (#{socket.bytesize} bytes) to fit sun_path: #{socket}"
  ensure
    SystemTestChromeProfile.cleanup_own!
  end

  test "with_browser_tmpdir redirects TMPDIR inside and restores it after" do
    with_env("TMPDIR", "/tmp/original") do
      SystemTestChromeProfile.with_browser_tmpdir do
        assert_equal SystemTestChromeProfile.tmpdir, ENV["TMPDIR"]
      end

      assert_equal "/tmp/original", ENV["TMPDIR"]
    end
  ensure
    SystemTestChromeProfile.cleanup_own!
  end

  test "with_browser_tmpdir restores an unset TMPDIR and survives exceptions" do
    with_env("TMPDIR", nil) do
      assert_raises(RuntimeError) do
        SystemTestChromeProfile.with_browser_tmpdir { raise "boom" }
      end

      assert_nil ENV["TMPDIR"]
    end
  ensure
    SystemTestChromeProfile.cleanup_own!
  end

  test "cleanup_own! removes this process's directory and keeps others" do
    base = SystemTestChromeProfile.base
    own = File.join(base, Process.pid.to_s)
    other = File.join(base, "987654320")
    FileUtils.mkdir_p(own)
    FileUtils.mkdir_p(other)

    begin
      SystemTestChromeProfile.cleanup_own!

      assert_not File.exist?(own)
      assert File.exist?(other)
    ensure
      FileUtils.rm_rf(own)
      FileUtils.rm_rf(other)
    end
  end

  test "cleanup_stale! removes dead-PID profiles and keeps live ones" do
    base = SystemTestChromeProfile.base
    dead = File.join(base, "987654321")
    live = File.join(base, Process.pid.to_s)
    FileUtils.mkdir_p(dead)
    FileUtils.mkdir_p(live)
    File.write(File.join(dead, "SingletonLock"), "stale\n")

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
    base = SystemTestChromeProfile.base
    other = File.join(base, "README")
    FileUtils.mkdir_p(base)
    File.write(other, "do not delete\n")

    begin
      SystemTestChromeProfile.cleanup_stale!

      assert File.exist?(other)
    ensure
      FileUtils.rm_f(other)
    end
  end

  private
    def with_env(key, value)
      existed = ENV.key?(key)
      old = ENV[key]
      value.nil? ? ENV.delete(key) : ENV[key] = value
      yield
    ensure
      existed ? ENV[key] = old : ENV.delete(key)
    end
end
