module Bots
  # Heals rows written before the digest retirement and nulls the retired
  # plaintext bot_token column: for every row that still has a plaintext
  # token, the digest is recomputed from that plaintext — the key its
  # holder was shown — and the plaintext is nulled, in one update per
  # row. Rows without a plaintext are untouched. Idempotent.
  class ClearPlaintextTokens
    def self.run!
      User.where.not(bot_token: nil).pluck(:id, :bot_token).count do |id, plaintext|
        heal(id, plaintext)
      end
    end

    # Conditional on the plaintext still being the one read: a key reset
    # between the read and this write nulls the plaintext and sets a new
    # digest, which must win over the stale key.
    def self.heal(id, plaintext)
      User.where(id: id, bot_token: plaintext)
        .update_all(bot_token_digest: User.digest_bot_token(plaintext), bot_token: nil) == 1
    end
  end
end
