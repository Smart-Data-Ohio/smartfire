# Runs the pinned body/helpers, rather than emulating Selenium's key delivery.
require "json"
require "capybara"
require "capybara/dsl"
require "capybara/minitest"
require "selenium-webdriver"
require "minitest/autorun"
location = JSON.parse(File.read("/proof/native-location.json"))
if %w[attachment release].include?(location.fetch("label"))
  require "/rails/config/environment"
  require "active_support/testing/assertions"
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
  options.add_option("goog:loggingPrefs", {browser: "ALL"})
  options.binary = "/usr/lib/chromium/chromium"
  %w[--headless=new --ozone-platform=headless --no-sandbox --disable-dev-shm-usage --mute-audio --window-size=1400,1400].each { |arg| options.add_argument(arg) }
  options.add_argument("--proxy-server=#{ENV.fetch("WS8BM_NATIVE_PROXY")}") if ENV["WS8BM_NATIVE_PROXY"]
  options.add_argument("--proxy-bypass-list=<-loopback>") if ENV["WS8BM_NATIVE_PROXY"]
  Capybara::Selenium::Driver.new(app, browser: :remote, url: "http://127.0.0.1:52023", options: options)
end
Capybara.default_driver = :ws8bm_native_phone
class Ws8bmNativePhoneTest < Minitest::Test
  include Capybara::DSL
  include Capybara::Minitest::Assertions
  include SystemTestHelper
  include ActiveSupport::Testing::Assertions if defined?(ActiveSupport::Testing::Assertions)
  include ActionView::RecordIdentifier if defined?(ActionView::RecordIdentifier)
  def messages(name)
    raise "unsupported native fixture" unless name == :third
    Message.find(607264868)
  end
  def setup
    if ENV["WS8BM_NATIVE_PROXY"]
      # Match translated served negatives: fetch the real mutated assets,
      # never a service-worker cache. Positive native controls retain it.
      driver=page.driver.browser
      driver.extend(Selenium::WebDriver::DriverExtensions::HasCDP) unless driver.respond_to?(:execute_cdp)
      driver.execute_cdp("Network.setBypassServiceWorker", bypass: true)
    end
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
    location = JSON.parse(File.read("/proof/native-location.json"))
    visit Capybara.app_host + (location["label"] == "motion" ? "/rooms/201306877" : "/rooms/654632876")
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
    if location["label"] == "release"
      page.execute_script(<<~'JS')
        window.__ws8bmReleaseClicks=[];
        document.addEventListener('click',event=>{
          const menu=event.target.closest?.('#message-actions-menu');
          if(menu) {
            const rect=document.querySelector('#message_0003').getBoundingClientRect();
            window.__ws8bmReleaseClicks.push({releaseClick:true,brokenGuard:window.__ws8bmBrokenReleaseGuard===true,menuVisible:!menu.hidden,
              trusted:event.isTrusted,atPressPoint:Math.hypot(event.clientX-(rect.left+rect.width/2),event.clientY-(rect.top+rect.height/2))<=20});
          }
        },true);
        const hit=document.elementFromPoint.bind(document);
        document.elementFromPoint=(x,y)=>{
          const node=hit(x,y),menu=document.querySelector('#message-actions-menu');
          window.__ws8bmReleaseGeometry={x,y,hit:node?.tagName,inMenu:!!node?.closest('#message-actions-menu'),top:menu?.getBoundingClientRect().top,inner:[innerWidth,innerHeight],outer:[outerWidth,outerHeight],motion:document.documentElement.dataset.testMotion};
          return node;
        };
      JS
    end
  end
  def teardown
    if JSON.parse(File.read("/proof/native-location.json"))["label"] == "attachment"
      puts "WS8bm native attachment DOM readback: #{JSON.generate(page.evaluate_script(<<~'JS'))}"
        [...document.querySelectorAll('.message[data-message-id]')].map(row=>({
          id:row.dataset.messageId,preview:row.querySelector('.message__reply-preview')?.textContent,
          body:row.querySelector('.message__body')?.textContent,
          content:row.querySelector('.message__body-content')?.innerHTML
        }))
      JS
      parent = Message.find_by(markdown_source: "**A useful point** with `inline code`.")
      attached = parent ? Message.joins(:attachment_blob).where(reply_to_message_id: parent.id).pluck("messages.id", "messages.reply_to_message_id", "messages.reply_notify_author", "active_storage_blobs.filename", "active_storage_blobs.byte_size") : []
      puts "WS8bm native attachment blob readback: #{JSON.generate(attached)}"
      puts "WS8bm native attachment saved readback: #{JSON.generate(Message.where("id > ?", 908005739).order(:id).pluck(:id, :reply_to_message_id, :reply_notify_author))}"
    elsif %w[motion release].include?(JSON.parse(File.read("/proof/native-location.json"))["label"])
      begin
        begin
          driver=page.driver.browser
          driver.extend(Selenium::WebDriver::DriverExtensions::HasLogs) unless driver.respond_to?(:logs)
          puts "WS8bm native browser logs: #{JSON.generate(driver.logs.get(:browser).map { |entry| {level: entry.level, message: entry.message} })}"
        rescue StandardError => error
          puts "WS8bm native log diagnostic failure: #{error.class}: #{error.message}"
        end
        state=page.evaluate_script('(() => { const surface=document.querySelector("#sidebar .sidebar__container"); return {room:location.pathname,open:document.querySelector("#sidebar")?.classList.contains("open"),duration:surface?getComputedStyle(surface).transitionDuration:null,transform:surface?getComputedStyle(surface).transform:null,releaseClicks:window.__ws8bmReleaseClicks,releaseGeometry:window.__ws8bmReleaseGeometry}; })()')
        puts "WS8bm native mutation state: #{JSON.generate(state)}"
      rescue StandardError => error
        puts "WS8bm native diagnostic failure: #{error.class}: #{error.message}"
      end
      puts "WS8bm native failures: #{JSON.generate(failures.map { |failure| {assertion: failure.is_a?(Minitest::Assertion),message: failure.message,backtrace: failure.backtrace} })}"
    end
    puts "WS8bm native phone event trace #{Capybara.app_host}: #{JSON.generate(page.evaluate_script('window.__ws8bmPhoneTrace||[]'))}"
  rescue => error
    warn "WS8bm native diagnostic failed: #{error.class}: #{error.message}"
  ensure
    Capybara.reset_sessions!
  end
  # Generated from git show of the exact Rails pin, not local/untracked helpers.
  location = JSON.parse(File.read("/proof/native-location.json"))
  class_eval File.read("/proof/phone-body.rb"), location.fetch("sourcePath"), location.fetch("line")
  class_eval File.read("/proof/phone-helpers.rb"), location.fetch("sourcePath")
end
