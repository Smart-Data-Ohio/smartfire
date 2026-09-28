module Github
  # Reconciles a message's PR references with the URLs its content currently
  # contains, and enqueues a fetch for every newly referenced or stale PR.
  # Idempotent: re-running with unchanged content enqueues nothing new.
  module PullRequestReferenceSync
    class << self
      # Pass `enqueue_fetches: false` from contexts that must stay quiet,
      # such as the Slack importer: references are still created, and each
      # card enqueues its own fetch the first time it renders.
      def call(message, enqueue_fetches: true)
        triples = PullRequestUrl.extract(reference_text(message))

        pull_requests = triples.map do |ref|
          PullRequest.for_reference(owner: ref.owner, repo: ref.repo, number: ref.number)
        end

        message.github_pull_request_references
          .where.not(github_pull_request_id: pull_requests.map(&:id))
          .delete_all

        pull_requests.each do |pull_request|
          reference = message.github_pull_request_references.find_or_create_by!(pull_request: pull_request)
          if enqueue_fetches && (reference.previously_new_record? || pull_request.stale?)
            Github::FetchPullRequestJob.perform_later(pull_request) if pull_request.claim_fetch_request!
          end
        end
      rescue ActiveRecord::RecordNotUnique
        retry
      end

      private
        def reference_text(message)
          [ PullRequestUrl.non_code_text(message.body.body&.to_html), message.forward_note ].compact_blank.join("\n")
        end
    end
  end
end
