namespace :bots do
  desc "Digest leftover plaintext bot tokens and clear them"
  task clear_plaintext_tokens: :environment do
    cleared = Bots::ClearPlaintextTokens.run!
    puts "Cleared #{cleared} plaintext bot token(s)."
  end
end
