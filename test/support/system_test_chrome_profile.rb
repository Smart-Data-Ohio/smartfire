require "capybara/selenium/driver"

# System-test Chrome profiles live under the repo's tmp/ instead of the
# shared /tmp tmpfs, which parallel workers filled to quota with leaked
# headless-Chrome profiles. Each browser launch takes the next per-process
# directory: the driven_by block runs once per test, but multi-session
# tests launch one Chrome per session, and two live Chromes cannot share a
# --user-data-dir (SingletonLock), so Capybara::Selenium::Driver instances
# each get a detached copy of the shared Selenium options below.
# Stale directories (whose PID no longer runs) are removed at suite start;
# each worker removes its own directories after the suite.
module SystemTestChromeProfile
  PROFILE_ARG_PREFIX = "--user-data-dir=".freeze

  class << self
    # Placeholder for the driven_by block (always replaced per browser).
    def dir
      Rails.root.join("tmp/chrome-profiles/#{Process.pid}").to_s
    end

    def next_dir
      @counter ||= 0
      @counter += 1
      Rails.root.join("tmp/chrome-profiles/#{Process.pid}-#{@counter}").to_s
    end

    def cleanup_own!
      FileUtils.rm_rf(Dir[base.join("#{Process.pid}*")])
    end

    def cleanup_stale!
      Dir[base.join("*")].each do |path|
        pid = File.basename(path).to_i
        next if pid <= 0 || process_running?(pid)

        FileUtils.rm_rf(path)
      end
    end

    # A copy of the shared Selenium options pointing at a fresh profile
    # directory. Selenium::Options#dup shares the internal args array, so
    # the copy detaches it before swapping the directory, then verifies
    # through the serialized capabilities.
    def detached_options(shared)
      copy = shared.dup
      detached = shared.instance_variable_get(:@options).dup
      detached[:args] = Array(detached[:args]).reject { |arg| arg.to_s.start_with?(PROFILE_ARG_PREFIX) }
      copy.instance_variable_set(:@options, detached)

      dir = next_dir
      copy.add_argument("#{PROFILE_ARG_PREFIX}#{dir}")

      serialized = Array(copy.as_json.dig("goog:chromeOptions", "args"))
      unless serialized.one? { |arg| arg == "#{PROFILE_ARG_PREFIX}#{dir}" }
        raise "SystemTestChromeProfile could not set a unique profile dir (selenium #{Selenium::WebDriver::VERSION})"
      end

      copy
    end

    private
      def base
        Rails.root.join("tmp/chrome-profiles")
      end

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

  module PerBrowserProfileDir
    def initialize(app, **options)
      selenium_options = options[:options]
      if selenium_options.is_a?(Selenium::WebDriver::Chromium::Options)
        options = options.merge(options: SystemTestChromeProfile.detached_options(selenium_options))
      end
      super(app, **options)
    end
  end
end

Capybara::Selenium::Driver.prepend(SystemTestChromeProfile::PerBrowserProfileDir)
