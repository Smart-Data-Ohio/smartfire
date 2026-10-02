# Run the original two HuddlesTest declarations with read-only observations.
# bundle exec ruby -Itest /rails/rust/reference-tools/ws13_media_readiness_probe.rb
#   -n '/device.pickers.list|server.enforcement.removes.a.revoked.participant/'
require "/rails/test/system/huddles_test.rb"

module Ws13MediaReadinessProbe
  private
    def join_room(room)
      super
      observation = page.evaluate_script(<<~JS)
        (() => {
          const element = document.querySelector('[data-controller~="huddle-presence"]');
          const controller = element && window.Stimulus?.getControllerForElementAndIdentifier(element, "huddle-presence");
          return { ready_state: document.readyState, presence_controller: !!controller,
            presence_in_flight: !!controller?.inFlightRefresh };
        })()
      JS
      observation["page_load_strategy"] = page.driver.browser.capabilities.page_load_strategy
      puts "WS13 Rails navigation: #{JSON.generate(observation)}"
    end

    def assert_media_received(kind)
      super
      puts "WS13 Rails completed RTP sample: #{JSON.generate(kind: kind, bytes: inbound_rtp_bytes(kind))}"
    end
end
HuddlesTest.prepend(Ws13MediaReadinessProbe)
