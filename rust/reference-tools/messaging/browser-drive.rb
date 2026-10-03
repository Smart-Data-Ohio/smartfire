# Same WebMock and GoogleCalendarTestHelper boundary as the pinned originals.
# The helper and exact payloads are materialized from d7c7de92 per fresh fixture.
Rails.application.config.after_initialize do
  if ENV["WS8BM_DRIVE_STUBS"] == "1"
    require "webmock"
    WebMock.enable!
    WebMock.disable_net_connect!(allow_localhost: true)
    eval(File.read(Rails.root.join("storage/db/google-calendar-test-helper.rb")), TOPLEVEL_BINDING,
      "test/test_helpers/google_calendar_test_helper.rb")
    helper = Object.new.extend(WebMock::API).extend(GoogleCalendarTestHelper)
    helper.send(:stub_google_drive_list)
    helper.send(:stub_google_drive_file, "1AbcDefGhIjKlMnOpQrSt")
    if ENV["WS8BM_DRIVE_TWO"] == "1"
      helper.send(:stub_google_drive_file, "2BcdEfgHiJkLmNoPqRsTu",
        body: helper.send(:drive_file_payload, name: "Budget 2026", mime_type: "application/vnd.google-apps.spreadsheet"))
    end
    warn "WS8bm reference Drive: pinned GoogleCalendarTestHelper WebMock stubs"
  end
end
