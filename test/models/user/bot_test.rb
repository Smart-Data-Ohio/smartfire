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

  test "creation writes the digest alone and stored bots hide their key" do
    token = "5M0aLYwQyBXOXa5Wsz6NZb11EE4tW2"
    SecureRandom.stubs(:alphanumeric).returns(token)
    bot = User.create_bot!(name: "Bender")

    stored = User.find(bot.id)
    assert_equal Digest::SHA256.hexdigest(token), stored.bot_token_digest
    assert_nil stored.read_attribute(:bot_token)
    assert_nil stored.plain_bot_key
    assert_equal User::Bot::BOT_KEY_PLACEHOLDER, stored.bot_key
    assert_equal bot, User.authenticate_bot("#{bot.id}-#{token}")
  end

  test "leftover plaintext is never consulted" do
    bot = User.create_bot!(name: "Bender")
    key = bot.plain_bot_key
    bot.update_columns(bot_token: "DecoyToken12")

    assert_equal bot, User.authenticate_bot(key)
    assert_nil User.authenticate_bot("#{bot.id}-DecoyToken12")
    assert_equal "DecoyToken12", bot.reload.read_attribute(:bot_token), "no re-backfill runs"
  end

  test "bot auth keeps working after plaintext clearing" do
    bot = User.create_bot!(name: "Bender")
    key = bot.plain_bot_key
    bot.update_columns(bot_token: "PreRetire123")

    Bots::ClearPlaintextTokens.run!
    assert_nil bot.reload.read_attribute(:bot_token)

    assert_equal bot, User.authenticate_bot(key)
    assert_nil User.authenticate_bot("#{bot.id}-WrongToken12")
  end

  test "a bot without a digest cannot authenticate until its key is reset" do
    bot = User.create_bot!(name: "Legacy")
    bot.update_columns(bot_token: "OldRelease12", bot_token_digest: nil)

    assert_nil User.authenticate_bot("#{bot.id}-OldRelease12")
    assert_nil User.authenticate_bot("#{bot.id}-WrongToken12")

    new_key = User.find(bot.id).reset_bot_key
    assert_equal bot, User.authenticate_bot(new_key)
  end

  test "reset stores the digest alone, clears leftover plaintext, and retires the old key" do
    bot = User.create_bot!(name: "Bender")
    bot.update_columns(bot_token: "PreRetire123")
    old_key = bot.plain_bot_key
    new_key = User.find(bot.id).reset_bot_key

    stored = User.find(bot.id)
    assert_nil stored.read_attribute(:bot_token)
    assert_equal Digest::SHA256.hexdigest(new_key.split("-", 2).last), stored.bot_token_digest
    assert_nil User.authenticate_bot(old_key)
    assert_equal bot, User.authenticate_bot(new_key)
  end

  test "reply token authenticates its bot for its room only" do
    bot = User.create_bot!(name: "Bender")
    room = rooms(:pets)
    assert_includes bot.rooms, room

    token = bot.reply_token_for(room)
    assert_equal bot, User.authenticate_bot_reply_token(token, room_id: room.id)
    assert_nil User.authenticate_bot_reply_token(token, room_id: rooms(:hq).id)
    assert_nil User.authenticate_bot_reply_token("#{token}x", room_id: room.id)
    assert_nil User.authenticate_bot_reply_token("bogus", room_id: room.id)
    assert_nil User.authenticate_bot_reply_token("", room_id: room.id)
    assert_nil User.authenticate_bot(token), "a reply token is not a bot key"
  end

  test "reply token expires" do
    bot = User.create_bot!(name: "Bender")
    room = rooms(:pets)
    token = bot.reply_token_for(room, expires_in: 1.minute)

    assert_equal bot, User.authenticate_bot_reply_token(token, room_id: room.id)

    travel_to 2.minutes.from_now do
      assert_nil User.authenticate_bot_reply_token(token, room_id: room.id)
    end
  end

  test "reply token is refused once the bot leaves or deactivates" do
    bot = User.create_bot!(name: "Bender")
    room = rooms(:pets)
    token = bot.reply_token_for(room)

    room.memberships.find_by!(user: bot).destroy!
    assert_nil User.authenticate_bot_reply_token(token, room_id: room.id)

    room.memberships.grant_to(bot)
    assert_equal bot, User.authenticate_bot_reply_token(token, room_id: room.id)

    bot.deactivate
    assert_nil User.authenticate_bot_reply_token(token, room_id: room.id)
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
