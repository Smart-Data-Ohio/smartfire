namespace :bots do
  desc "Null retired plaintext bot tokens where a digest exists"
  task clear_plaintext_tokens: :environment do
    cleared = Bots::ClearPlaintextTokens.run!
    puts "Cleared #{cleared} plaintext bot token(s)."
  end
end
