# Stubs and drivers for Slack importer tests. Every stub matches the real
# request: GET with the documented query parameters and the user-token
# Authorization header (see Slack::Client).
module SlackImportTestHelper
  SLACK_FIXTURES = Rails.root.join("test/fixtures/files/slack")
  SLACK_TEST_TOKEN = "xoxp-test-token"
  SLACK_API = "https://slack.com/api"

  def slack_fixture(name)
    File.read(SLACK_FIXTURES.join(name))
  end

  def slack_auth_header(token = SLACK_TEST_TOKEN)
    "Bearer #{token}"
  end

  def create_slack_workspace!(**attributes)
    SlackWorkspace.create!({ client_id: "test-client-id",
      client_secret: "test-client-secret" }.merge(attributes))
  end

  def create_slack_connection!(workspace:, user:, slack_user_id: "UADMIN", token: SLACK_TEST_TOKEN)
    SlackConnection.create!(slack_workspace: workspace, user:,
      slack_user_id:, access_token: token,
      scopes: "channels:history channels:read groups:history groups:read im:history im:read mpim:history mpim:read users:read users:read.email team:read")
  end

  # Stubs the whole fake workspace. list selects the conversations.list
  # fixture (:workspace or :personal); *_overrides replace single responses
  # with a JSON string or parsed hash (nil removes the stub).
  def stub_slack_workspace!(token: SLACK_TEST_TOKEN, list: :workspace,
      users_body: nil, list_body: nil, history_overrides: {}, replies_overrides: {},
      members_overrides: {}, auth_error: nil, history_error_body: nil,
      history_first_responses: {})
    auth = { "Authorization" => slack_auth_header(token) }
    json = { "Content-Type" => "application/json" }

    stub_request(:get, "#{SLACK_API}/users.list")
      .with(query: hash_including({ "limit" => "200" }), headers: auth)
      .to_return(status: 200, body: users_body || slack_fixture("users.json"), headers: json)

    stub_request(:get, "#{SLACK_API}/conversations.list")
      .with(query: hash_including({ "exclude_archived" => "false" }), headers: auth)
      .to_return(status: 200, body: list_body || slack_fixture("conversations_#{list}.json"), headers: json)

    error_body = history_error_body || (auth_error && { ok: false, error: auth_error })
    if error_body
      # Every history call fails: limit is present on all of them.
      stub_request(:get, "#{SLACK_API}/conversations.history")
        .with(query: hash_including({ "limit" => "200" }), headers: auth)
        .to_return(status: 200, body: JSON.generate(error_body), headers: json)
    else
      {
        "CCHAN" => %w[ history_CCHAN_p1 history_CCHAN_p2 ], "CARCH" => %w[ history_CARCH ],
        "CPRIV" => %w[ history_CPRIV ], "DIM" => %w[ history_DIM ],
        "GMPIM" => %w[ history_GMPIM ]
      }.each do |channel, pages|
        bodies = history_overrides.key?(channel) ? Array(history_overrides[channel]) :
          pages.map { |page| slack_fixture("#{page}.json") }
        firsts = history_first_responses[channel]
        stub_history_pages(channel, bodies, auth, json, first_responses: firsts)
      end
    end

    { "CCHAN" => "1700000002.000002", "GMPIM" => "1700000050.000050" }.each do |channel, ts|
      body = replies_overrides[channel]
      body = slack_fixture("replies_#{channel}_parent.json") if body.nil? && !replies_overrides.key?(channel)
      next if body.nil?

      stub_request(:get, "#{SLACK_API}/conversations.replies")
        .with(query: hash_including({ "channel" => channel, "ts" => ts }), headers: auth)
        .to_return(status: 200, body: body.is_a?(String) ? body : JSON.generate(body), headers: json)
    end

    (%w[ CCHAN CARCH CPRIV DIM GMPIM ] + members_overrides.keys).uniq.each do |channel|
      body = members_overrides[channel]
      if body.nil? && !members_overrides.key?(channel)
        path = SLACK_FIXTURES.join("members_#{channel}.json")
        body = File.read(path) if File.exist?(path)
      end
      next if body.nil?

      stub_request(:get, "#{SLACK_API}/conversations.members")
        .with(query: hash_including({ "channel" => channel }), headers: auth)
        .to_return(status: 200, body: body.is_a?(String) ? body : JSON.generate(body), headers: json)
    end
  end

  # First page matches cursor-less requests; later pages chain in order
  # on cursor-bearing requests for the same channel. The query matcher is
  # required: a stub without one does not match requests carrying a query
  # string. first_responses replaces the cursor-less sequence (a 429 before
  # the first page, a cancelling body); each entry is a body or a
  # { status:, headers:, body: } response spec.
  def stub_history_pages(channel, bodies, auth, json, first_responses: nil)
    firsts = Array(first_responses.presence || [ bodies.first ])
    _first, *rest = bodies
    query = hash_including({ "channel" => channel })

    stub = stub_request(:get, "#{SLACK_API}/conversations.history")
      .with(query:, headers: auth) { |request| history_params(request, channel, cursor: false) }
    firsts.each { |spec| stub.to_return(**to_return_kwargs(spec, json)) }

    unless rest.empty?
      stub = stub_request(:get, "#{SLACK_API}/conversations.history")
        .with(query:, headers: auth) { |request| history_params(request, channel, cursor: true) }
      rest.each { |body| stub.to_return(status: 200, body: response_body(body), headers: json) }
    end
  end

  def to_return_kwargs(spec, json)
    if spec.is_a?(Hash) && spec.key?(:status)
      { status: spec[:status], body: response_body(spec.fetch(:body, "")),
        headers: json.merge(spec[:headers] || {}) }
    else
      { status: 200, body: response_body(spec), headers: json }
    end
  end

  def history_params(request, channel, cursor:)
    params = URI.decode_www_form(request.uri.query.to_s).to_h
    params["channel"] == channel && params["cursor"].present? == cursor
  end

  def response_body(body)
    return body if body.is_a?(String) || body.respond_to?(:call)

    JSON.generate(body)
  end

  # Runs step jobs inline until the run finishes, exercising the same
  # resumable path production takes across executions.
  def drive_import_to_completion(run, limit: 50)
    iterations = 0
    while run.reload.active? && iterations < limit
      SlackImport::StepJob.perform_now(run.id)
      iterations += 1
    end
    run.reload
    assert run.finished?, "expected run to finish in #{limit} steps, stuck #{run.status}"
    run
  end

  def drive_undo_to_completion(run, limit: 50)
    iterations = 0
    while run.reload.undoing? && iterations < limit
      SlackImport::UndoJob.perform_now(run.id)
      iterations += 1
    end
    run.reload
    assert_equal "undone", run.status, "expected run to undo in #{limit} steps, stuck #{run.status}"
    run
  end

  def with_google_domains(domains)
    previous = ENV["GOOGLE_SIGN_IN_DOMAINS"]
    ENV["GOOGLE_SIGN_IN_DOMAINS"] = domains
    yield
  ensure
    ENV["GOOGLE_SIGN_IN_DOMAINS"] = previous
  end
end
