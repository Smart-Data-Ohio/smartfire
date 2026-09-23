require "application_system_test_case"

# The test browser must launch with chromedriver's managed profile: an
# explicit --user-data-dir launches Chrome without the data:, startup tab,
# leaving the driven tab inactive (document.hasFocus() false), which
# suppresses focus/focusin delivery and flakes every focus-dependent
# system test. TMPDIR redirection (see SystemTestChromeProfile) keeps the
# managed launch while holding profiles off the /tmp tmpfs; chrome://version
# reports the real command line and profile path, so this pins both.
class BrowserLaunchProfileTest < ApplicationSystemTestCase
  test "the browser launches with a managed profile under the worker TMPDIR" do
    visit root_path
    page.driver.browser.navigate.to("chrome://version/")

    info = page.evaluate_script("document.body.innerText")
    command_line = info_line(info, "Command Line")
    profile_path = info_line(info, "Profile Path")
    # The worker TMPDIR contract, restated (not read through the module)
    # so this also fails on code that predates the module API.
    worker_tmpdir = File.join(Dir.home, ".cache/campfire-chrome-tmp/#{Process.pid}") + "/"

    assert_includes command_line, "--user-data-dir=#{worker_tmpdir}",
      "expected chromedriver's managed profile under the worker TMPDIR, not an explicit --user-data-dir"
    assert_match %r{org\.chromium\.Chromium\.}, profile_path,
      "expected a chromedriver-managed temp profile, got: #{profile_path.inspect}"
  end

  private
    def info_line(info, label)
      line = info.each_line(chomp: true).find { |candidate| candidate.start_with?("#{label}\t") }
      assert_not_nil line, "expected a #{label.inspect} row on chrome://version"
      line.delete_prefix("#{label}\t")
    end
end
