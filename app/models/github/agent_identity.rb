module Github
  # Which linked GitHub account an agent acts with. Prefers the owner's
  # GitHub App token when the owner has a usable one; otherwise the
  # agent's own linked account (the machine-user PAT). Request time, the
  # approval UI, and execution time all resolve through here, so the
  # identity the decider approved is the identity that runs.
  module AgentIdentity
    class << self
      def resolve(agent)
        owner_account = agent&.owner&.github_connected_account
        if owner_account&.usable? && owner_account.app_token?
          owner_account
        else
          agent&.user&.github_connected_account
        end
      end
    end
  end
end
