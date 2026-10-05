require "application_system_test_case"
require "timeout"

# Regression test for the recurring DriveAttachmentsTest CI flake
# (WebMock::NetConnectNotAllowedError for GET drive/v3/files/:id,
# attributed to the test whose teardown it lands in).
#
# A Drive chip preview fetch that is still running on the app server when
# the test ends must complete against that test's stubs: the stubs stay
# registered until the server has no pending requests left.
class DriveTeardownRaceTest < ApplicationSystemTestCase
  include GoogleCalendarTestHelper
  include WebMockSystemTestHelper

  FILE_ID = "1AbcDefGhIjKlMnOpQrSt"
  DOCS_URL = "https://docs.google.com/document/d/#{FILE_ID}/edit"

  setup do
    # Give this test a fresh shared server, started by a named session.
    # The default session reuses its port but has a different middleware
    # counter, making a current-session-only teardown deterministically wrong.
    @original_app = Capybara.app
    @original_session_name = Capybara.session_name
    Capybara.app = ->(env) { @original_app.call(env) }
    Capybara.session_name = :drive_teardown_server_owner
    visit "about:blank"
    @server_owner = page.server
    Capybara.session_name = @original_session_name
  end

  teardown do
    Capybara.app = @original_app
    Capybara.session_name = @original_session_name
  end

  test "an in-flight Drive metadata fetch completes against its own stubs at teardown" do
    started = Queue.new
    proceed = Queue.new
    $drive_teardown_race_gate = { file_id: FILE_ID, started:, proceed: }
    begin
      connect_google!(users(:jz), scopes: DRIVE_SCOPES)
      stub_google_drive_file(FILE_ID)
      sign_in "jz@37signals.com"
      join_room rooms(:designers)
      assert_equal @server_owner.port, page.server.port
      refute_same @server_owner, page.server

      send_message "Please review #{DOCS_URL} before Friday"

      # The app server is now blocked inside #drive_file, before its
      # WebMock lookup. Release it only once this test's teardown is
      # underway, so the lookup provably lands after the body ended.
      Timeout.timeout(20) { started.pop }
      Thread.new { sleep 5; proceed << true }

      # The link is present as soon as the message renders; do not wait
      # for the preview upgrade, which needs the gated fetch.
      assert_selector "a[href='#{DOCS_URL}']", wait: 5
    ensure
      $drive_teardown_race_gate = nil
    end
  end
end

# Holds one Google::Client#drive_file call before its WebMock lookup so a
# test can pin an in-flight server request across its own teardown. Only
# the gated file id blocks, and only while the gate is set, so every
# other Drive request in the worker process passes through untouched.
module DriveTeardownRaceGate
  def drive_file(file_id)
    gate = $drive_teardown_race_gate
    if gate && file_id == gate[:file_id]
      gate[:started] << true
      gate[:proceed].pop
    end
    super
  end
end
Google::Client.prepend(DriveTeardownRaceGate)
