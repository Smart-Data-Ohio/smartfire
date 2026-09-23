module Bots
  # Nulls the retired plaintext bot_token column wherever a digest exists,
  # so the key survives only as a SHA-256 digest. New and reset bots
  # already store the digest alone; this clears rows written before the
  # retirement. Rows without a digest are left untouched: their key is
  # unknown and must be reset, not silently dropped. Idempotent.
  class ClearPlaintextTokens
    def self.run!
      User.where.not(bot_token: nil).where.not(bot_token_digest: nil).update_all(bot_token: nil)
    end
  end
end
