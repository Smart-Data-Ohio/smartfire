# Chrome sometimes reports a node that Turbo just replaced as a generic
# "unhandled inspector error: Node with given id does not belong to the
# document" instead of a StaleElementReferenceError. Capybara only retries
# the errors a driver lists as invalid-element errors, so the generic form
# failed tests outright on busy CI runners. Treat that one message as a
# stale element so Capybara's normal synchronize retry applies.
module StaleNodeRetry
  STALE_NODE_MESSAGE = "does not belong to the document".freeze

  class StaleNodeError < Selenium::WebDriver::Error::StaleElementReferenceError; end

  module Bridge
    def execute(command, opts = {}, command_hash = nil)
      super
    rescue Selenium::WebDriver::Error::UnknownError => error
      raise error unless error.message.include?(STALE_NODE_MESSAGE)

      raise StaleNodeError, error.message
    end
  end
end

Selenium::WebDriver::Remote::Bridge.prepend(StaleNodeRetry::Bridge)
