# Bot keys are stored only as digests, so a fixture bot's key cannot be read
# back from its record. Fixture bots use these known tokens instead (see
# test/fixtures/users.yml); bots created in a test keep theirs in memory.
module BotKeyTestHelper
  FIXTURE_BOT_TOKENS = { "Bender Bot" => "BenderToken1" }.freeze

  def bot_key_for(bot)
    bot.plain_bot_key || "#{bot.id}-#{FIXTURE_BOT_TOKENS.fetch(bot.name)}"
  end
end
