# System-test Chrome profiles live under the repo's tmp/ instead of the
# shared /tmp tmpfs, which parallel workers filled to quota with leaked
# headless-Chrome profiles. One directory per test process (PID): the
# driven_by block resolves it lazily so forked parallel workers each get
# their own. Stale directories (whose PID no longer runs) are removed at
# suite start; each worker removes its own directory after the suite.
module SystemTestChromeProfile
  class << self
    def dir
      Rails.root.join("tmp/chrome-profiles/#{Process.pid}").to_s
    end

    def cleanup_stale!
      Dir[base.join("*")].each do |path|
        pid = File.basename(path).to_i
        next if pid <= 0 || process_running?(pid)

        FileUtils.rm_rf(path)
      end
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
end
