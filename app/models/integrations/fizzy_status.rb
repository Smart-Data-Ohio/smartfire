module Integrations
  # Seam for Fizzy board health. No Fizzy integration lives in this repo
  # yet, so the health page reports it as not configured; when a Fizzy
  # client lands, wire it here: return configured true plus account
  # counts, recent errors, and sync state in the same shape as the other
  # sections, and the health page picks it up unchanged.
  module FizzyStatus
    class << self
      def snapshot
        { configured: false, note: "No Fizzy integration is configured in this workspace." }
      end
    end
  end
end
