require "test_helper"

class FizzyConnectedAccountTest < ActiveSupport::TestCase
  include FizzyTestHelper

  test "a linked account is connected and usable" do
    account = link_fizzy!(users(:david))

    assert_predicate account, :connected?
    assert_predicate account, :usable?
  end

  test "one account per user" do
    link_fizzy!(users(:david))

    assert_raises(ActiveRecord::RecordInvalid) do
      link_fizzy!(users(:david), token: "another-token")
    end
  end

  test "marking disconnected reads as unusable" do
    account = link_fizzy!(users(:david))
    account.mark_disconnected!("Fizzy rejected the linked token (401)")

    assert_not account.reload.connected?
    assert_not account.usable?
  end

  test "deactivating the user disconnects the account" do
    account = link_fizzy!(users(:david))

    users(:david).deactivate

    assert_equal "Account deactivated", account.reload.disconnected_reason
  end
end
