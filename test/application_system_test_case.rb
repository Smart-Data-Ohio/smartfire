require "test_helper"
require_relative "support/system_test_chrome_profile"

WebMock.disable!
Capybara.enable_aria_label = true

# Headless Chrome's temporary profile goes to the repo's tmp/ (one
# directory per worker PID) instead of the shared /tmp tmpfs, which
# parallel workers filled to quota. Stale entries are removed here; each
# worker removes its own directory after the suite (below). PID liveness
# is checked per entry so concurrently booting workers never remove a
# live sibling's profile.
SystemTestChromeProfile.cleanup_stale!
Minitest.after_run { FileUtils.rm_rf(SystemTestChromeProfile.dir) }

class ApplicationSystemTestCase < ActionDispatch::SystemTestCase
  # Cross-session Turbo Stream broadcasts (stage roles, voice presence,
  # stream badges and dots) render in another browser through the test
  # cable adapter's thread pool; on a loaded CI runner that exceeds the
  # 2 s default and the old 10 s waits. Use only on assertions that wait
  # for a broadcast result, not everywhere.
  BROADCAST_WAIT = 15

  # A stage role change makes the affected browser rejoin LiveKit with
  # fresh credentials through the node gateway; on a slow runner the full
  # round trip exceeds the 20 s polling budget used elsewhere in the suite.
  LIVEKIT_REJOIN_WAIT = 30

  # The code highlighter boots a worker and compiles Shiki grammars on
  # first use; the c++ fence stalls ~1 s uncontended and ~1.8 s under a
  # 4-browser load, past the 2 s default. The worker abandons a block
  # after 15 s (see tokenize in code_highlighter.js), so a 20 s bound
  # means a timeout is a genuine highlighting failure, not impatience.
  HIGHLIGHT_WAIT = 20

  # Each worker drives its own headless Chrome; twenty of them starve each
  # other and fail at sign-in on a developer machine, while four stay green.
  # PARALLEL_WORKERS still overrides this, which is how CI runs a single worker.
  parallelize(workers: [ Etc.nprocessors, 4 ].min)

  # --mute-audio keeps chat sounds (/play, notifications) off the host
  # speakers while tests run. --user-data-dir keeps the temporary Chrome
  # profile under the repo's tmp/ (see SystemTestChromeProfile); the block
  # runs lazily so forked parallel workers each resolve their own PID.
  driven_by :selenium, using: :headless_chrome, screen_size: [ 1400, 1400 ] do |options|
    options.add_argument "--mute-audio"
    options.add_argument "--user-data-dir=#{SystemTestChromeProfile.dir}"
  end

  include SystemTestHelper

  # test_helper.rb denies every net connect for each test. System tests drive
  # a real browser through chromedriver over localhost, and the Capybara
  # selenium driver reuses one HTTP client (and its Net::HTTP connection)
  # across every test in the process. A connection created while WebMock was
  # enabled is an instance of WebMock's Net::HTTP subclass and keeps
  # consulting WebMock's global config for later chromedriver requests even
  # after WebMock.disable!, so localhost must stay reachable here no matter
  # which test ran first. The server suite keeps the deny-all default.
  setup do
    WebMock.disable_net_connect!(allow_localhost: true)
  end
end
