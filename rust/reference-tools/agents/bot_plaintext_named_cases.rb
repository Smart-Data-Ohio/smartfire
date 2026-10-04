ApplicationJob.queue_adapter=:test
Bots::ClearPlaintextTokens.run!
stale=User.create_bot!(name:"Stale Digest")
stale.update_columns(bot_token:"CurrentKey12",bot_token_digest:User.digest_bot_token("OldKey123456"))
digestless=User.create_bot!(name:"Digestless")
digestless.update_columns(bot_token:"NoDigestKey1",bot_token_digest:nil)
clean=User.create_bot!(name:"Clean");clean_key=clean.plain_bot_key
count=Bots::ClearPlaintextTokens.run!
record=->(bot,current,old=nil) do
  bot.reload
  {plaintext:bot.read_attribute(:bot_token),digest:bot.bot_token_digest,
   current_auth:User.authenticate_bot(current)==bot,old_auth:old ? User.authenticate_bot(old)==bot : nil}
end
results={heal:{count:count,stale:record.call(stale,"#{stale.id}-CurrentKey12","#{stale.id}-OldKey123456"),
  digestless:record.call(digestless,"#{digestless.id}-NoDigestKey1"),clean_plaintext:clean.reload.read_attribute(:bot_token),clean_auth:User.authenticate_bot(clean_key)==clean}}
bot=User.create_bot!(name:"Racing Reset")
bot.update_columns(bot_token:"LeakedKey123",bot_token_digest:nil)
bot.reload.reset_bot_key;new_key=bot.plain_bot_key
results[:race]={healed:Bots::ClearPlaintextTokens.heal(bot.id,"LeakedKey123"),new_auth:User.authenticate_bot(new_key)==bot,old_auth:User.authenticate_bot("#{bot.id}-LeakedKey123")==bot}
results[:repeat]=Bots::ClearPlaintextTokens.run!
puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],results:results)
