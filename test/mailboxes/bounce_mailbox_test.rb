require "test_helper"

class BounceMailboxTest < ActionMailbox::TestCase
  test "mail to a non-room address is marked bounced without posting" do
    inbound = nil
    assert_no_difference -> { Message.count } do
      inbound = receive_inbound_email_from_mail(
        from: "david@37signals.com", to: "nobody@mail.test", body: "Hello"
      )
    end

    assert_predicate inbound.reload, :bounced?
  end

  test "mail without any room token never raises a routing error" do
    assert_nothing_raised do
      receive_inbound_email_from_mail(
        from: "spam@example.com", to: "hello@mail.test", body: "Buy this."
      )
    end
  end
end
