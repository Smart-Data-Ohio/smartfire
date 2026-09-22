module Google
  module SignIn
    # Resolves verified Google claims to a Campfire user. The immutable
    # Google subject wins: an existing GoogleIdentity signs in its user
    # even across email changes. First login links (or provisions) by
    # the verified email; later logins never consult the email for an
    # already-linked subject. Never reactivates, recreates, or
    # provisions privileged accounts. Raises Rejected on any mismatch.
    #
    # Linking by email trusts the Campfire account's address, so an
    # address the member typed in themselves (email_self_changed_at) is
    # never trusted: a member could otherwise claim a new hire's Workspace
    # address and receive that person's first Google sign-in. Only
    # accounts with google_email_link_allowed (those that existed before
    # this rule, with their original email, or that an administrator
    # vouched for) auto-link; every other match -- join-code signups,
    # self-changed emails -- is refused with :admin_link_required. Those
    # members link from their own profile while signed in (link_to_user!).
    class AccountLinker
      # Marker User#deactivate splices into the email local part, as in
      # "jane-deactivated-<uuid>@example.com".
      DEACTIVATED_MARKER = "-deactivated-"

      class << self
        def resolve!(claims)
          subject = claims["sub"].to_s
          email = claims["email"].to_s.strip
          domain = claims["hd"].to_s.strip.downcase
          raise Rejected, :bad_token if subject.blank? || email.blank?

          if (identity = GoogleIdentity.find_by(subject:))
            user = identity.user
            ensure_eligible!(user)
            identity.update!(email:, domain:) if identity.email != email || identity.domain != domain
            return user
          end

          link_or_provision!(subject:, email:, domain:, claims:)
        rescue ActiveRecord::RecordNotUnique
          # Lost a creation race: whoever won owns the subject now, so
          # re-resolve by subject once instead of duplicating the user.
          identity = GoogleIdentity.find_by(subject: claims["sub"].to_s)
          if identity
            ensure_eligible!(identity.user)
            return identity.user
          end
          raise Rejected, :retry
        end

        # Links verified claims to an already signed-in member, who proved
        # the account is theirs by being signed in. Refuses a subject that
        # belongs to someone else and a member already linked to another
        # subject (an administrator unlinks first). Returns the identity.
        def link_to_user!(claims, user)
          subject = claims["sub"].to_s
          email = claims["email"].to_s.strip
          domain = claims["hd"].to_s.strip.downcase
          raise Rejected, :bad_token if subject.blank? || email.blank?

          ensure_eligible!(user)

          if (identity = GoogleIdentity.find_by(subject:))
            raise Rejected, :subject_taken unless identity.user_id == user.id

            identity.update!(email:, domain:)
            return identity
          end

          raise Rejected, :already_linked if user.google_identity

          GoogleIdentity.create!(user:, subject:, email:, domain:)
        rescue ActiveRecord::RecordNotUnique
          raise Rejected, :subject_taken
        end

        private
          def email_link_allowed?(user)
            user.google_email_link_allowed? && user.email_self_changed_at.nil?
          end

          def link_or_provision!(subject:, email:, domain:, claims:)
            matches = User.where("LOWER(email_address) = ?", email.downcase).to_a
            raise Rejected, :ambiguous if matches.many?

            if (user = matches.first)
              ensure_eligible!(user)
              if user.google_identity && user.google_identity.subject != subject
                raise Rejected, :subject_mismatch
              end
              raise Rejected, :admin_link_required unless email_link_allowed?(user)

              GoogleIdentity.create!(user:, subject:, email:, domain:)
              return user
            end

            raise Rejected, :deactivated if deactivated_predecessor?(email)

            User.transaction do
              user = User.create!(
                name: display_name(claims, email),
                email_address: email,
                password: nil,
                role: :member
              )
              GoogleIdentity.create!(user:, subject:, email:, domain:)
              user
            end
          end

          # Only active humans sign in through Google. Bots, agent
          # users, deactivated, and banned users are rejected -- and a
          # deactivated account is never revived by signing in.
          def ensure_eligible!(user)
            if user.nil? || user.deactivated?
              raise Rejected, :deactivated
            elsif user.banned?
              raise Rejected, :banned
            elsif !user.active? || user.bot? || user.agent.present?
              raise Rejected, :ineligible
            end
          end

          # A deactivated user keeps a rewritten email, so the lookup
          # above cannot see them -- but signing in must not recreate
          # their account either. Match the rewrite pattern to refuse.
          def deactivated_predecessor?(email)
            local, _, domain = email.partition("@")
            return false if local.blank? || domain.blank?

            pattern = "#{sanitize_like(local.downcase)}#{DEACTIVATED_MARKER}%@#{sanitize_like(domain.downcase)}"
            User.deactivated.where("LOWER(email_address) LIKE ? ESCAPE '\\'", pattern).exists?
          end

          def sanitize_like(term)
            term.gsub(/[\\%_]/) { |char| "\\#{char}" }
          end

          def display_name(claims, email)
            claims["name"].to_s.strip.presence ||
              [ claims["given_name"], claims["family_name"] ].map(&:to_s).map(&:strip).compact_blank.join(" ").presence ||
              email.split("@").first
          end
      end
    end
  end
end
