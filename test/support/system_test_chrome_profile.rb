require "capybara/selenium/driver"

# System-test Chrome profiles live under ~/.cache/campfire-chrome-tmp (one
# directory per worker PID) instead of the shared /tmp tmpfs, which
# parallel workers filled to quota with leaked headless-Chrome profiles.
# The redirect goes through TMPDIR at browser launch: chromedriver then
# keeps its managed-profile launch path, including the data:, startup tab
# that activates the window. Never pass --user-data-dir here instead: an
# explicit profile dir launches Chrome without that startup tab, leaving
# the driven tab inactive (document.hasFocus() false), which suppresses
# focus/focusin delivery and flakes every focus-dependent system test.
# Stale directories (whose PID no longer runs) are removed at suite start;
# each worker removes its own directory after the suite.
module SystemTestChromeProfile
  class << self
    # Per-process TMPDIR for browser launches, created on demand.
    def tmpdir
      dir = File.join(base, Process.pid.to_s)
      FileUtils.mkdir_p(dir)
      dir
    end

    def cleanup_own!
      FileUtils.rm_rf(File.join(base, Process.pid.to_s))
    end

    def cleanup_stale!
      return unless File.directory?(base)

      Dir[File.join(base, "*")].each do |path|
        pid = File.basename(path).to_i
        next if pid <= 0 || process_running?(pid)

        FileUtils.rm_rf(path)
      end
    end

    # Runs the block with TMPDIR pointed at this process's browser dir.
    # Chromedriver and Chrome inherit it when they spawn, so temp
    # profiles land there; the previous value is always restored, so no
    # other part of the test process observes the redirect.
    def with_browser_tmpdir
      old_tmpdir = ENV["TMPDIR"]
      ENV["TMPDIR"] = tmpdir
      yield
    ensure
      if old_tmpdir.nil?
        ENV.delete("TMPDIR")
      else
        ENV["TMPDIR"] = old_tmpdir
      end
    end

    # Under $HOME (not the repo): Chrome's SingletonSocket path must fit
    # the 108-byte sun_path limit, and a worktree-nested tmp/ blows past
    # it (Chrome exits FATAL). Short, per-user, off the /tmp tmpfs.
    def base
      File.join(Dir.home, ".cache/campfire-chrome-tmp")
    end

    private
      def process_running?(pid)
        Process.kill(0, pid)
        true
      rescue Errno::ESRCH
        false
      rescue Errno::EPERM
        # A live process we may not signal: leave its profile alone.
        true
      end
  end

  module TmpdirBrowserLaunch
    def browser
      SystemTestChromeProfile.with_browser_tmpdir { super }
    end
  end
end

Capybara::Selenium::Driver.prepend(SystemTestChromeProfile::TmpdirBrowserLaunch)
