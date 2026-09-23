require "test_helper"

class Bots::ClearPlaintextTokensTest < ActiveSupport::TestCase
  test "re-keys the digest from the plaintext, then clears it" do
    Bots::ClearPlaintextTokens.run!

    stale = User.create_bot!(name: "Stale Digest")
    stale_current_key = "#{stale.id}-CurrentKey12"
    stale.update_columns(bot_token: "CurrentKey12", bot_token_digest: User.digest_bot_token("OldKey123456"))
    stale_old_key = "#{stale.id}-OldKey123456"

    digestless = User.create_bot!(name: "Digestless")
    digestless_key = "#{digestless.id}-NoDigestKey1"
    digestless.update_columns(bot_token: "NoDigestKey1", bot_token_digest: nil)

    clean = User.create_bot!(name: "Clean")
    clean_key = clean.plain_bot_key

    assert_equal 2, Bots::ClearPlaintextTokens.run!

    assert_nil stale.reload.read_attribute(:bot_token)
    assert_equal User.digest_bot_token("CurrentKey12"), stale.bot_token_digest
    assert_equal stale, User.authenticate_bot(stale_current_key)
    assert_nil User.authenticate_bot(stale_old_key),
      "the key behind the stale digest no longer works"

    assert_nil digestless.reload.read_attribute(:bot_token)
    assert_equal User.digest_bot_token("NoDigestKey1"), digestless.bot_token_digest
    assert_equal digestless, User.authenticate_bot(digestless_key)

    assert_nil clean.reload.read_attribute(:bot_token)
    assert_equal clean, User.authenticate_bot(clean_key)
  end

  test "repeat runs are no-ops" do
    Bots::ClearPlaintextTokens.run!

    assert_equal 0, Bots::ClearPlaintextTokens.run!
  end
end
