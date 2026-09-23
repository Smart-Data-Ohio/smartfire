# Wires the TOTP sudo verifier (:totp) to enforced two-step sign-in.
# Sudo offers it only to enrolled members (see verifier_available?),
# alongside the password -- password OR code -- plus Google re-auth
# for members with a linked identity. Verification reuses the
# sign-in challenge's replay-protected check and shared
# failure/lockout counters; the prompt's own rate limit already
# covers the create action every verifier posts to. Backup codes are
# NOT accepted here: they are single-use sign-in recovery, and
# spending one on a sudo confirmation would silently burn the
# member's way back in.
#
# Inside to_prepare like the other boot wiring: initializers run
# before autoloading, and the reopen must re-apply after every
# development reload. Registration is idempotent.
Rails.application.config.to_prepare do
  SudoMode.register_verifier(:totp)

  SudoMode.singleton_class.class_eval do
    # True when the code confirms the user, false when it rejects,
    # :unsupported when two-step sign-in is not enrolled. Locked
    # credentials reject even correct codes; a verified code clears
    # the failure run while a wrong one escalates the same lockout
    # the sign-in challenge enforces.
    def verify_totp(user, code)
      credential = user.two_factor_credential
      return :unsupported unless credential&.enabled?
      return false if credential.locked_out?

      if credential.verify_code(code)
        credential.register_challenge_success!
        true
      else
        credential.register_challenge_failure!
        false
      end
    end

    def verifier_available?(name, user)
      name.to_sym == :totp && user.two_factor_enabled?
    end
  end
end
