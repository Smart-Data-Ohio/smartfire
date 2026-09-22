require "test_helper"

class User::BotTest < ActiveSupport::TestCase
  test "create bot" do
    token = "5M0aLYwQyBXOXa5Wsz6NZb11EE4tW2"
    SecureRandom.stubs(:alphanumeric).returns(token)

    uuid = "3574925f-479d-44f8-82b7-fc039af5367c"
    Random.stubs(:uuid).returns(uuid)

    bot = User.create_bot!(name: "Bender")
    assert_equal "#{bot.id}-#{token}", bot.bot_key
  end

  test "reset bot key" do
    first_token = "5M0aLYwQyBXOXa5Wsz6NZb11EE4tW2"
    SecureRandom.stubs(:alphanumeric).returns(first_token)

    bot = User.create_bot!(name: "Bender")
    assert_equal "#{bot.id}-#{first_token}", bot.bot_key

    second_token = "R4kme9anwWRuz3sSoBXiB8Li8ioZPP"
    SecureRandom.stubs(:alphanumeric).returns(second_token)

    bot.reset_bot_key
    assert_equal "#{bot.id}-#{second_token}", bot.bot_key
  end

  test "authenticate" do
    bot = User.create_bot!(name: "Bender")
    assert User.authenticate_bot(bot.bot_key)
  end

  test "creation writes the digest and, for this release, the plaintext" do
    token = "5M0aLYwQyBXOXa5Wsz6NZb11EE4tW2"
    SecureRandom.stubs(:alphanumeric).returns(token)
    bot = User.create_bot!(name: "Bender")

    stored = User.find(bot.id)
    assert_equal Digest::SHA256.hexdigest(token), stored.bot_token_digest
    assert_equal token, stored.bot_token, "a rolled-back container still needs the plaintext"
    assert_nil stored.plain_bot_key
    assert_equal "#{bot.id}-#{token}", stored.bot_key
    assert_equal bot, User.authenticate_bot("#{bot.id}-#{token}")
  end

  test "authentication trusts the digest, not the plaintext column" do
    bot = User.create_bot!(name: "Bender")
    key = bot.plain_bot_key

    bot.update_columns(bot_token: "tampered1234")
    assert_equal bot, User.authenticate_bot(key)
    assert_nil User.authenticate_bot("#{bot.id}-tampered1234")

    bot.update_columns(bot_token_digest: Digest::SHA256.hexdigest("other"))
    assert_nil User.authenticate_bot(key)
  end

  test "a bot written without a digest by the previous release authenticates once and is backfilled" do
    bot = User.create_bot!(name: "Legacy")
    bot.update_columns(bot_token: "OldRelease12", bot_token_digest: nil)

    assert_nil User.authenticate_bot("#{bot.id}-WrongToken12")
    assert_nil bot.reload.bot_token_digest

    assert_equal bot, User.authenticate_bot("#{bot.id}-OldRelease12")
    assert_equal Digest::SHA256.hexdigest("OldRelease12"), bot.reload.bot_token_digest
  end

  test "reset writes both forms and retires the old key" do
    bot = User.create_bot!(name: "Bender")
    old_key = bot.plain_bot_key
    new_key = User.find(bot.id).reset_bot_key

    stored = User.find(bot.id)
    assert_equal new_key, "#{bot.id}-#{stored.bot_token}"
    assert_equal Digest::SHA256.hexdigest(stored.bot_token), stored.bot_token_digest
    assert_nil User.authenticate_bot(old_key)
    assert_equal bot, User.authenticate_bot(new_key)
  end

  test "authenticate refuses wrong, empty, and malformed keys" do
    bot = User.create_bot!(name: "Bender")
    token = bot.plain_bot_token

    assert_nil User.authenticate_bot("#{bot.id}-#{token}x")
    assert_nil User.authenticate_bot("#{bot.id}-")
    assert_nil User.authenticate_bot("#{bot.id}")
    assert_nil User.authenticate_bot("")
    assert_nil User.authenticate_bot(User::Bot::BOT_KEY_PLACEHOLDER)
    assert_nil User.authenticate_bot("#{users(:bender).id}-#{token}"), "another bot's token never matches"
    assert_nil User.authenticate_bot("#{users(:david).id}-#{token}"), "humans have no bot key"
  end

  test "a deactivated bot's key is refused" do
    bot = User.create_bot!(name: "Bender")
    key = bot.bot_key
    bot.deactivate

    assert_nil User.authenticate_bot(key)
  end

  test "existing keys keep working after the digest migration" do
    assert_equal users(:bender), User.authenticate_bot("#{users(:bender).id}-BenderToken1")
  end

  test "deliver message by webhook" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    perform_enqueued_jobs only: Bot::WebhookJob do
      users(:bender).deliver_webhook_later(messages(:first))
    end
  end
end
