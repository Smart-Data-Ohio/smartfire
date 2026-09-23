module Bots
  # Heals rows written before the digest retirement and nulls the retired
  # plaintext bot_token column: for every row that still has a plaintext
  # token, the digest is recomputed from that plaintext — the key its
  # holder was shown — and the plaintext is nulled, in one update per
  # row. Rows without a plaintext are untouched. Idempotent.
  class ClearPlaintextTokens
    def self.run!
      cleared = 0
      User.where.not(bot_token: nil).find_each do |user|
        user.update_columns(
          bot_token_digest: User.digest_bot_token(user.read_attribute(:bot_token)),
          bot_token: nil
        )
        cleared += 1
      end
      cleared
    end
  end
end
