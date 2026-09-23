require "test_helper"

class Bots::ClearPlaintextTokensTest < ActiveSupport::TestCase
  test "nulls plaintext only where a digest exists" do
    Bots::ClearPlaintextTokens.run!

    clearable = User.create_bot!(name: "Clearable")
    clearable.update_columns(bot_token: "ClearMe1234")
    digestless = User.create_bot!(name: "Digestless")
    digestless.update_columns(bot_token: "KeepMe12345", bot_token_digest: nil)
    clean = User.create_bot!(name: "Clean")

    assert_equal 1, Bots::ClearPlaintextTokens.run!

    assert_nil clearable.reload.read_attribute(:bot_token)
    assert_equal User.digest_bot_token(clearable.plain_bot_key.split("-", 2).last),
      clearable.bot_token_digest
    assert_equal "KeepMe12345", digestless.reload.read_attribute(:bot_token),
      "without a digest the key is unknown: reset, don't drop"
    assert_nil clean.reload.read_attribute(:bot_token)
  end

  test "repeat runs are no-ops" do
    Bots::ClearPlaintextTokens.run!

    assert_equal 0, Bots::ClearPlaintextTokens.run!
  end
end
