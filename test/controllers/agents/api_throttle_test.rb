require "test_helper"

class Agents::ApiThrottleTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @second_bot = User.create_bot!(name: "Throttle Second")
    @second_agent = @second_bot.create_agent!(kind: :workspace, owner: users(:david))
    @room.memberships.grant_to(@second_bot)
    _, @secret = AgentCredential.create_with_secret!(agent: @agent, name: "throttle", created_by: users(:david))
    _, @second_secret = AgentCredential.create_with_secret!(agent: @second_agent, name: "throttle", created_by: users(:david))
  end

  test "polling throttles past 120 requests a minute per credential" do
    with_memory_cache do
      freeze_time do
        120.times do
          get agents_events_url, headers: bearer_headers(@secret)
          assert_response :success
        end

        get agents_events_url, headers: bearer_headers(@secret)

        assert_response :too_many_requests
        assert_equal "rate_limited", response.parsed_body["error"]
        assert_retry_after
      end
    end
  end

  test "throttle buckets are keyed per credential" do
    with_memory_cache do
      freeze_time do
        121.times { get agents_events_url, headers: bearer_headers(@secret) }
        assert_response :too_many_requests

        get agents_events_url, headers: bearer_headers(@second_secret)

        assert_response :success
      end
    end
  end

  test "acking throttles past 120 requests a minute" do
    @room.messages.create!(
      creator: users(:david), body: "Hey #{mention_attachment_for(:bender)}",
      client_message_id: "throttle-ack"
    )
    event_id = @agent.agent_events.deliverable.last.id

    with_memory_cache do
      freeze_time do
        120.times do
          post ack_agents_event_url(event_id), headers: bearer_headers(@secret)
          assert_response :success
        end

        post ack_agents_event_url(event_id), headers: bearer_headers(@secret)

        assert_response :too_many_requests
        assert_retry_after
      end
    end
  end

  test "posting messages throttles past 60 requests a minute" do
    with_memory_cache do
      freeze_time do
        60.times do |i|
          post room_agent_messages_url(@room),
            params: { message: { markdown_source: "Throttle #{i}", client_message_id: "throttle-msg-#{i}" } },
            headers: bearer_headers(@secret), as: :json
          assert_response :created
        end

        post room_agent_messages_url(@room),
          params: { message: { markdown_source: "Throttle over", client_message_id: "throttle-msg-over" } },
          headers: bearer_headers(@secret), as: :json

        assert_response :too_many_requests
        assert_retry_after
      end
    end
  end

  test "listing approvals throttles past 120 requests a minute" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "external_action")

    with_memory_cache do
      freeze_time do
        120.times do
          get agents_approvals_url, headers: bearer_headers(@secret)
          assert_response :success
        end

        get agents_approvals_url, headers: bearer_headers(@secret)

        assert_response :too_many_requests
        assert_retry_after
      end
    end
  end

  test "creating approvals throttles past 60 requests a minute" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "external_action")

    with_memory_cache do
      freeze_time do
        60.times do |i|
          post agents_approvals_url,
            params: { approval: { action: "deploy", summary: "Ship #{i}", room_id: @room.id } }.to_json,
            headers: bearer_headers(@secret)
          assert_response :created
        end

        post agents_approvals_url,
          params: { approval: { action: "deploy", summary: "Ship over", room_id: @room.id } }.to_json,
          headers: bearer_headers(@secret)

        assert_response :too_many_requests
        assert_retry_after
      end
    end
  end

  test "cancelling approvals throttles past 60 requests a minute" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "external_action")
    approvals = 61.times.map do |i|
      AgentApproval.create!(agent: @agent, room: @room, action: "deploy", summary: "Cancel #{i}")
    end

    with_memory_cache do
      freeze_time do
        approvals.first(60).each do |approval|
          delete "/agents/approvals/#{approval.id}", headers: bearer_headers(@secret)
          assert_response :success
        end

        delete "/agents/approvals/#{approvals.last.id}", headers: bearer_headers(@secret)

        assert_response :too_many_requests
        assert_retry_after
      end
    end
  end

  test "creating board posts throttles past 30 requests a minute" do
    board = Rooms::Board.create_for({ name: "Throttle Board", creator: users(:david) }, users: [ users(:david) ])
    board.memberships.grant_to(@bot)
    AgentGrant.create!(agent: @agent, room: board, granted_by: users(:david), capability: "post_messages")
    AgentGrant.create!(agent: @agent, room: board, granted_by: users(:david), capability: "manage_threads")

    with_memory_cache do
      freeze_time do
        30.times do |i|
          post room_agent_posts_url(board),
            params: { title: "Throttle post #{i}" }.to_json,
            headers: bearer_headers(@secret)
          assert_response :created
        end

        post room_agent_posts_url(board),
          params: { title: "Throttle post over" }.to_json,
          headers: bearer_headers(@secret)

        assert_response :too_many_requests
        assert_retry_after
      end
    end
  end

  test "creating pull request actions throttles past 60 requests a minute" do
    message = @room.messages.create!(
      creator: users(:david),
      markdown_source: "review https://github.com/rails/rails/pull/12",
      client_message_id: "throttle-pr-12"
    )
    pull_request = message.github_pull_requests.first
    thread = ChannelThread.create!(room: @room, creator: users(:david), parent_message: message)
    ThreadMembership.join!(thread, users(:david))
    Github::PullRequestThread.create!(pull_request: pull_request, room: @room, channel_thread: thread)
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "external_action")
    GithubConnectedAccount.create!(user: @bot, github_login: "bender-machine", access_token: "agent-token-abc")

    with_memory_cache do
      freeze_time do
        60.times do |i|
          post room_agent_github_pull_request_actions_url(@room),
            params: { pull_request_id: pull_request.id, kind: "comment", body: "Throttle #{i}" }.to_json,
            headers: bearer_headers(@secret)
          assert_response :accepted
        end

        post room_agent_github_pull_request_actions_url(@room),
          params: { pull_request_id: pull_request.id, kind: "comment", body: "Throttle over" }.to_json,
          headers: bearer_headers(@secret)

        assert_response :too_many_requests
        assert_retry_after
      end
    end
  end

  test "human session requests are never throttled" do
    sign_in users(:david)

    with_memory_cache do
      freeze_time do
        121.times do
          get agents_approvals_url
          assert_response :forbidden
        end
      end
    end
  end

  private
    def bearer_headers(secret)
      { "Authorization" => "Bearer #{secret}", "Content-Type" => "application/json" }
    end

    def assert_retry_after
      retry_after = response.headers["Retry-After"].to_i
      assert_operator retry_after, :>=, 1
      assert_operator retry_after, :<=, 60
    end

    def with_memory_cache
      store = ActiveSupport::Cache::MemoryStore.new
      previous = Rails.cache
      Rails.cache = store
      yield store
    ensure
      Rails.cache = previous
    end
end
