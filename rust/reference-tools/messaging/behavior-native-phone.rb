# Runs the pinned body/helpers, rather than emulating Selenium's key delivery.
require "json"
require "capybara"
require "capybara/dsl"
require "capybara/minitest"
require "selenium-webdriver"
require "minitest/autorun"
location = JSON.parse(File.read("/proof/native-location.json"))
if location.fetch("label") == "attachment"
  require "/rails/config/environment"
  ActiveRecord::Base.establish_connection(adapter: "sqlite3", database: ENV.fetch("WS8BM_NATIVE_DATABASE"), flags: SQLite3::Constants::Open::READONLY)
end
require "/proof/system_test_helper"
raise "wrong Capybara version" unless Capybara::VERSION == "3.40.0"
raise "wrong Selenium version" unless Selenium::WebDriver::VERSION == "4.35.0"
Capybara.enable_aria_label = true
Capybara.run_server = false
Capybara.default_max_wait_time = 2
Capybara.app_host = ENV.fetch("WS8BM_NATIVE_BASE")
Capybara.register_driver :ws8bm_native_phone do |app|
  options = Selenium::WebDriver::Chrome::Options.new
  options.binary = "/usr/lib/chromium/chromium"
  %w[--headless=new --ozone-platform=headless --no-sandbox --disable-dev-shm-usage --mute-audio --window-size=1400,1400].each { |arg| options.add_argument(arg) }
  Capybara::Selenium::Driver.new(app, browser: :remote, url: "http://127.0.0.1:52023", options: options)
end
Capybara.default_driver = :ws8bm_native_phone
class Ws8bmNativePhoneTest < Minitest::Test
  include Capybara::DSL
  include Capybara::Minitest::Assertions
  include SystemTestHelper
  include ActionView::RecordIdentifier if defined?(ActionView::RecordIdentifier)
  def setup
    visit Capybara.app_host + "/up"
    cookie = JSON.parse(File.read("/proof/sessions.json"))["sessions"].find { |entry| entry["user_name"] == "JZ" }["cookie_header"]
    name, value = cookie.split("=", 2)
    page.driver.browser.manage.add_cookie(name: name, value: value, path: "/")
    if JSON.parse(File.read("/proof/native-location.json"))["label"] == "attachment"
      # Remote ChromeDriver shares the mounted file path with Ruby. Match the
      # pinned local Selenium driver's native file-input action; no /se/file
      # Grid transfer endpoint or replacement browser write is involved.
      page.driver.browser.file_detector = ->(_keys) { nil }
      page.current_window.resize_to(1440, 1000)
      page.driver.browser.execute_cdp "Emulation.setEmulatedMedia", features: [ { name: "prefers-color-scheme", value: "light" } ]
    end
    visit Capybara.app_host + "/rooms/654632876"
    assert_selector "#composer", wait: 15
    page.document.synchronize(15) do
      raise Capybara::ExpectationNotMet, "Stimulus startup" unless page.evaluate_script('!!window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById("composer"),"composer")')
    end
    if %w[motion attachment].include?(JSON.parse(File.read("/proof/native-location.json"))["label"])
      wait_for_cable_connection
      dismiss_pwa_install_prompt
    else
      assert_selector 'turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]', visible: :all, wait: 15
    end
    # Match Rails.env.test?'s pinned layout input; this is not server parity credit.
    unless JSON.parse(File.read("/proof/native-location.json"))["label"] == "attachment"
      page.execute_script('document.documentElement.setAttribute("data-test-motion","off")')
    end
    page.execute_script(<<~'JS')
      window.__ws8bmPhoneTrace=[];
      const describe=n=>n?{tag:n.tagName,id:n.id,label:n.getAttribute?.('aria-label')}:null;
      for(const type of ['focusin','contextmenu','keydown']) document.addEventListener(type,e=>{
        if(type==='keydown'&&e.key!=='Escape')return;
        window.__ws8bmPhoneTrace.push({type,key:e.key,target:describe(e.target),active:describe(document.activeElement),menuContainsActive:document.querySelector('#message-actions-menu')?.contains(document.activeElement),panelHidden:document.querySelector('#thread-panel')?.getAttribute('aria-hidden')});
      },true);
    JS
  end
  def teardown
    puts "WS8bm native phone event trace #{Capybara.app_host}: #{JSON.generate(page.evaluate_script('window.__ws8bmPhoneTrace||[]'))}"
  ensure
    Capybara.reset_sessions!
  end
  # Generated from git show of the exact Rails pin, not local/untracked helpers.
  location = JSON.parse(File.read("/proof/native-location.json"))
  class_eval File.read("/proof/phone-body.rb"), location.fetch("sourcePath"), location.fetch("line")
  class_eval File.read("/proof/phone-helpers.rb"), location.fetch("sourcePath")
end
