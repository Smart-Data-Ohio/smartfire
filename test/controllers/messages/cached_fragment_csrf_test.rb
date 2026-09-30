require "test_helper"

# Message pages render messages/_message through a fragment cache shared by
# every viewer (message_with_pr_cards_cache_key has no viewer in it), so
# nothing inside that partial may depend on the viewer's session. Rails
# embeds a session-bound authenticity token in every form it renders unless
# told not to; a cached one hands the first viewer's tokens to everyone who
# loads the page after them.
class Messages::CachedFragmentCsrfTest < ActionDispatch::IntegrationTest
  TURBO_ACCEPT = "text/vnd.turbo-stream.html, text/html, application/xhtml+xml"

  setup do
    host! "smartfire.test"
    @room = rooms(:designers)

    pull_request = Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 3141)
    pull_request.update!(private: false, title: "Cached card", state: "open", fetched_at: Time.current)
    pull_request.update_column(:fetch_requested_at, nil)
    @room.messages.create!(creator: users(:david), markdown_source: "see https://github.com/rails/rails/pull/3141", client_message_id: "csrf-pr")

    poll_message = @room.root_messages.create!(creator: users(:david), markdown_source: "Lunch?", client_message_id: "csrf-poll")
    Poll.create_for_message!(message: poll_message, labels: [ "Tacos", "Pizza" ])

    boosted = @room.messages.create!(creator: users(:david), markdown_source: "Boost me", client_message_id: "csrf-boosts")
    boosted.boosts.create!(booster: users(:david), content: "👍")
    boosted.boosts.create!(booster: users(:jason), content: "Legacy text boost")
  end

  test "a cached message page serves no viewer's tokens to the next" do
    with_forgery_protection_and_fragment_caching do
      first = signed_in_session(:david)
      first.get room_messages_url(@room)
      assert_equal 200, first.response.status
      assert_renders_every_cached_form first.response.body

      second = signed_in_session(:jason)
      assert_served_from_cache { second.get room_messages_url(@room) }
      assert_equal 200, second.response.status

      assert_no_foreign_tokens second, first_viewer_page: first.response.body
    end
  end

  test "a cached refresh serves no viewer's tokens to the next" do
    since = 1.minute.ago.to_fs(:epoch)

    with_forgery_protection_and_fragment_caching do
      first = signed_in_session(:david)
      first.get room_refresh_url(@room, format: :turbo_stream), params: { since: since }
      assert_equal 200, first.response.status
      assert_renders_every_cached_form first.response.body

      second = signed_in_session(:jason)
      assert_served_from_cache { second.get room_refresh_url(@room, format: :turbo_stream), params: { since: since } }

      assert_no_foreign_tokens second, first_viewer_page: first.response.body
    end
  end

  test "a cached thread page serves no viewer's tokens to the next" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Cached thread")
    ThreadMembership.join!(thread, users(:david))
    reply = thread.post_message!(creator: users(:david), attributes: { markdown_source: "In the thread", client_message_id: "csrf-thread" })
    reply.boosts.create!(booster: users(:david), content: "🎉")

    with_forgery_protection_and_fragment_caching do
      first = signed_in_session(:david)
      first.get room_thread_messages_url(@room, thread)
      assert_equal 200, first.response.status
      assert forms(first.response.body).any? { |form| form[:action].end_with?("/messages/#{reply.id}/boosts") }

      second = signed_in_session(:jason)
      assert_served_from_cache { second.get room_thread_messages_url(@room, thread) }
      assert_equal 200, second.response.status

      assert_no_foreign_tokens second, first_viewer_page: first.response.body
    end
  end

  test "every form in a cached message submits with the page's header token" do
    # A legacy boost can only be deleted by its booster; keep the viewer's own.
    boosts(:first).destroy!

    with_forgery_protection_and_fragment_caching do
      signed_in_session(:david).get room_messages_url(@room)

      viewer = signed_in_session(:jason)
      viewer.get room_url(@room)
      header_token = Nokogiri::HTML5(viewer.response.body).at_css("meta[name=csrf-token]")["content"]
      assert header_token.present?

      assert_served_from_cache { viewer.get room_messages_url(@room) }
      assert_renders_every_cached_form viewer.response.body
      assert_empty form_tokens(viewer.response.body), "cached forms must rely on the header token alone"

      forms(viewer.response.body).each do |form|
        # What Turbo sends for a form submission: the form's own fields,
        # plus the page's csrf-token meta in X-CSRF-Token.
        submit viewer, form, headers: { "X-CSRF-Token" => header_token, "Accept" => TURBO_ACCEPT }
        assert_includes 200..303, viewer.response.status,
          "#{form[:method].upcase} #{form[:action]} failed with #{viewer.response.status}"
      end
    end
  end

  private
    def signed_in_session(user)
      open_session do |session|
        session.host! "smartfire.test"
        session.get sign_in_for_tests_path(email_address: users(user).email_address, password: "secret123456")
        assert session.cookies[:session_token].present?
      end
    end

    # Cached forms: reaction chips, legacy boost delete, poll vote and
    # retract, and the pull request card's Discuss button.
    def assert_renders_every_cached_form(body)
      actions = forms(body).map { |form| form[:action] }
      assert actions.any? { |action| action.match?(%r{/messages/\d+/boosts\z}) }, "no reaction form in #{actions}"
      assert actions.any? { |action| action.match?(%r{/messages/\d+/boosts/\d+\z}) }, "no boost delete form in #{actions}"
      assert_equal 2, actions.count { |action| action.match?(%r{/polls/\d+/vote\z}) }, "no poll forms in #{actions}"
      assert_includes actions, room_github_pull_request_threads_path(@room)
    end

    def assert_no_foreign_tokens(viewer, first_viewer_page:)
      tokens = form_tokens(viewer.response.body)

      assert_empty tokens & form_tokens(first_viewer_page),
        "the fragment cache served the first viewer's CSRF tokens to the second"

      rejected = forms(viewer.response.body).select { |form| form[:params].key?("authenticity_token") }.reject do |form|
        submit viewer, form
        true
      rescue ActionController::InvalidAuthenticityToken
        false
      rescue ActiveRecord::RecordNotFound
        true # Past the forgery check: the token was accepted, the record isn't the viewer's.
      end
      assert_empty rejected.map { |form| "#{form[:method].upcase} #{form[:action]}" },
        "the page embeds CSRF tokens that its own session rejects"
    end

    # Only what messages/_message rendered (the cached fragments), not the
    # layout around them, which renders per request.
    def message_fragments(body)
      Nokogiri::HTML4(body).css("[data-message-id]")
    end

    def form_tokens(body)
      message_fragments(body).css("input[name=authenticity_token]").map { |input| input["value"] }
    end

    # Each form as a browser without JavaScript would submit it: hidden
    # fields and the first choice of any option list.
    def forms(body)
      message_fragments(body).css("form").map do |form|
        params = {}
        form.css("input[type=hidden]").each { |input| params[input["name"]] = input["value"] }
        if (choice = form.at_css("input[type=radio], input[type=checkbox]"))
          params[choice["name"]] = [ choice["value"] ]
        end
        method = params.delete("_method") || form["method"] || "get"
        { action: form["action"], method: method.downcase, params: params }
      end
    end

    def submit(session, form, headers: {})
      session.process form[:method].to_sym, form[:action], params: form[:params], headers: headers
    end

    def assert_served_from_cache(&block)
      hits = []
      callback = ->(event) { hits << event.payload[:cache_hits] if event.payload.key?(:cache_hits) }
      ActiveSupport::Notifications.subscribed(callback, "render_collection.action_view", &block)
      assert hits.any?(&:positive?), "the second viewer's page did not come from the fragment cache"
    end

    def with_forgery_protection_and_fragment_caching
      original_forgery_protection = ActionController::Base.allow_forgery_protection
      original_cache = Rails.cache
      original_collection_cache = ActionView::PartialRenderer.collection_cache
      original_perform_caching = ActionController::Base.perform_caching

      ActionController::Base.allow_forgery_protection = true
      # The test environment caches to the null store. The collection
      # renderer snapshots its store at boot, so point it at the memory
      # store too or cached: keeps missing.
      Rails.cache = ActiveSupport::Cache::MemoryStore.new
      ActionView::PartialRenderer.collection_cache = Rails.cache
      ActionController::Base.perform_caching = true

      yield
    ensure
      ActionController::Base.perform_caching = original_perform_caching
      ActionView::PartialRenderer.collection_cache = original_collection_cache
      Rails.cache = original_cache
      ActionController::Base.allow_forgery_protection = original_forgery_protection
    end
end
