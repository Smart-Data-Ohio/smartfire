# Runs the pinned body/helpers, rather than emulating Selenium's key delivery.
require "json"
require "capybara"
require "capybara/dsl"
require "capybara/minitest"
require "selenium-webdriver"
require "minitest/autorun"
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
  def setup
    visit Capybara.app_host + "/up"
    cookie = JSON.parse(File.read("/proof/sessions.json"))["sessions"].find { |entry| entry["user_name"] == "JZ" }["cookie_header"]
    name, value = cookie.split("=", 2)
    page.driver.browser.manage.add_cookie(name: name, value: value, path: "/")
    visit Capybara.app_host + "/rooms/654632876"
    assert_selector "#composer", wait: 15
    page.document.synchronize(15) do
      raise Capybara::ExpectationNotMet, "Stimulus startup" unless page.evaluate_script('!!window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById("composer"),"composer")')
    end
    assert_selector 'turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]', visible: :all, wait: 15
    # Match Rails.env.test?'s pinned layout input; this is not server parity credit.
    page.execute_script('document.documentElement.setAttribute("data-test-motion","off")')
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
  class_eval File.read("/proof/phone-body.rb"), "test/system/threads_test.rb", 293
  class_eval File.read("/proof/phone-helpers.rb"), "test/system/threads_test.rb"
end
