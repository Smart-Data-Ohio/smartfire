require "test_helper"

# The relay ingress is the production front door for forward-to-room
# email. These tests pin its authentication: without the ingress
# password nothing is accepted, wrong credentials are rejected, and the
# right ones record the message for routing.
class RelayIngressTest < ActionDispatch::IntegrationTest
  RELAY_PATH = "/rails/action_mailbox/relay/inbound_emails"

  setup do
    @ingress_before = ActionMailbox.ingress
    ActionMailbox.ingress = :relay
    @password_before = ENV["RAILS_INBOUND_EMAIL_PASSWORD"]
    ENV.delete("RAILS_INBOUND_EMAIL_PASSWORD")
  end

  teardown do
    ActionMailbox.ingress = @ingress_before
    ENV["RAILS_INBOUND_EMAIL_PASSWORD"] = @password_before
  end

  test "a missing ingress password refuses instead of accepting" do
    assert_no_difference -> { ActionMailbox::InboundEmail.count } do
      assert_raises(ArgumentError) do
        post RELAY_PATH, params: raw_source, headers: { "Content-Type" => "message/rfc822" }
      end
    end
  end

  test "wrong credentials are rejected" do
    ENV["RAILS_INBOUND_EMAIL_PASSWORD"] = "correct-password"

    assert_no_difference -> { ActionMailbox::InboundEmail.count } do
      post RELAY_PATH, params: raw_source, headers: {
        "Content-Type" => "message/rfc822",
        "Authorization" => ActionController::HttpAuthentication::Basic.encode_credentials("actionmailbox", "wrong-password")
      }
    end

    assert_response :unauthorized
  end

  test "correct credentials record the message for routing" do
    ENV["RAILS_INBOUND_EMAIL_PASSWORD"] = "correct-password"

    assert_difference -> { ActionMailbox::InboundEmail.count }, 1 do
      post RELAY_PATH, params: raw_source, headers: {
        "Content-Type" => "message/rfc822",
        "Authorization" => ActionController::HttpAuthentication::Basic.encode_credentials("actionmailbox", "correct-password")
      }
    end

    assert_response :no_content
  end

  private
    def raw_source
      Mail.new(from: "david@37signals.com", to: "room-abc@mail.test", subject: "Hello", body: "Hi.").to_s
    end
end
