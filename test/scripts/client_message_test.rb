require "test_helper"

class ClientMessageTest < ActiveSupport::TestCase
  test "pending message JavaScript respects delivery" do
    skip "node is required to drive the client message tests" unless system("node --version", out: File::NULL, err: File::NULL)

    output = IO.popen([ "node", "--test", Rails.root.join("test/scripts/client_message_test.mjs").to_s ], err: %i[ child out ], &:read)
    assert $?.success?, output
  end
end
