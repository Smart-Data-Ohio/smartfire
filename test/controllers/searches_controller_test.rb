require "test_helper"

class SearchesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @message = rooms(:designers).messages.create! body: "Hello world!", client_message_id: "search", creator: users(:david)
  end

  test "index initial view" do
    get searches_url

    assert_response :success
    assert_select ".message", count: 0
  end

  test "finding reachable messages" do
    get searches_url, params: { q: "hello" }

    assert_response :success
    assert_select ".message", text: /Hello world!/
  end

  test "unreachable messages are not found" do
    memberships(:david_designers).destroy!

    get searches_url, params: { q: "hello" }

    assert_response :success
    assert_select ".message", count: 0
  end

  test "operator words are searched literally instead of raising" do
    rooms(:designers).messages.create! body: "cats and dogs", client_message_id: "operators", creator: users(:david)

    get searches_url, params: { q: "NOT" }
    assert_response :success

    get searches_url, params: { q: "cats AND" }
    assert_response :success
    assert_select ".message", text: /cats and dogs/
  end

  test "a leading operator returns 200" do
    get searches_url, params: { q: "OR bar" }
    assert_response :success
  end

  test "a boolean-looking query does not exclude terms" do
    rooms(:designers).messages.create! body: "foo bar", client_message_id: "no-not", creator: users(:david)
    rooms(:designers).messages.create! body: "foo not bar", client_message_id: "with-not", creator: users(:david)

    get searches_url, params: { q: "foo NOT bar" }

    assert_response :success
    assert_select ".message", text: /foo not bar/
    assert_select ".message", text: /foo bar/, count: 0
  end

  test "a quote character returns 200 with sensible results" do
    get searches_url, params: { q: '"hello"' }

    assert_response :success
    assert_select ".message", text: /Hello world!/
  end

  test "create does not run the search" do
    SearchesController.any_instance.expects(:set_messages).never

    post searches_url, params: { q: "NOT" }

    assert_redirected_to searches_url(q: "NOT")
    assert users(:david).searches.exists?(query: "NOT")
  end

  test "clear does not run the search" do
    SearchesController.any_instance.expects(:set_messages).never

    delete clear_searches_url, params: { q: "NOT" }

    assert_redirected_to searches_url
  end

  # Turbo form submissions (the header search, clearing recents) send a
  # Turbo Stream Accept header that the redirected GET inherits.
  test "index renders the page for a Turbo Stream request without an older-results cursor" do
    get searches_url(q: "hello"), headers: { "Accept" => "text/vnd.turbo-stream.html, text/html, application/xhtml+xml" }

    assert_response :success
    assert_equal "text/html", response.media_type
    assert_select "#search-results .message", text: /Hello world!/
  end

  test "a query with no results shows an empty state" do
    get searches_url, params: { q: "zebra stripes tuxedo" }

    assert_response :success
    assert_select "p.searches__empty", text: /No messages match/
  end

  test "results page through Load older results" do
    base = Time.zone.local(2026, 1, 1, 12, 0, 0)
    created = 45.times.map do |index|
      rooms(:designers).messages.create!(
        body: "paging message number #{index}",
        client_message_id: "paging-#{index}",
        creator: users(:david),
        created_at: base + index.seconds
      )
    end

    get searches_url, params: { q: "paging message number" }

    assert_response :success
    assert_select "#search-results .message", count: 40
    assert_select "#search-results .message", text: /paging message number 44/
    assert_select "#search-results .message", text: /paging message number 0/, count: 0
    assert_select "a", text: "Load older results"

    get searches_url(q: "paging message number", before: created[5].id), as: :turbo_stream

    assert_response :success
    assert_includes response.body, "paging message number 0"
    assert_includes response.body, "paging message number 4"
    assert_not_includes response.body, "paging message number 44"
    assert_includes response.body, 'action="remove" target="load_older_results"'
  end

  test "an older window renders as a page without JavaScript" do
    base = Time.zone.local(2026, 1, 1, 12, 0, 0)
    created = 45.times.map do |index|
      rooms(:designers).messages.create!(
        body: "fallback paging message #{index}",
        client_message_id: "fallback-paging-#{index}",
        creator: users(:david),
        created_at: base + index.seconds
      )
    end

    get searches_url(q: "fallback paging message", before: created[5].id)

    assert_response :success
    assert_select "#search-results .message", count: 5
    assert_select "a", text: "Load older results", count: 0
  end

  test "create saves the search term" do
    assert_difference -> { users(:david).searches.count }, +1 do
      post searches_url, params: { q: "hello" }
    end

    assert_redirected_to searches_url(q: "hello")
    assert users(:david).searches.exists?(query: "hello")
  end

  test "create with no searchable words redirects back with a notice and records nothing" do
    [ "???", "🙂" ].each do |query|
      assert_no_difference -> { users(:david).searches.count } do
        post searches_url, params: { q: query }
      end

      assert_redirected_to searches_url
      assert_equal "Enter a word to search for.", flash[:notice]
    end
  end

  test "clear search history" do
    assert users(:david).searches.any?

    delete clear_searches_url

    assert users(:david).searches.none?
  end

  test "from: narrows results to that author" do
    rooms(:designers).messages.create! body: "authored alpha", client_message_id: "op-from-jz", creator: users(:jz)
    rooms(:designers).messages.create! body: "authored alpha", client_message_id: "op-from-david", creator: users(:david)

    get searches_url, params: { q: "from:@JZ authored alpha" }

    assert_response :success
    assert_select ".message", count: 1
    assert_select ".message", text: /authored alpha/
  end

  test "in: narrows results to that room" do
    rooms(:watercooler).messages.create! body: "roomscoped alpha", client_message_id: "op-in-water", creator: users(:david)

    get searches_url, params: { q: "in:#Designers roomscoped alpha" }

    assert_response :success
    assert_select ".message", count: 0

    get searches_url, params: { q: "in:#Talk roomscoped alpha" }

    assert_response :success
    assert_select ".message", count: 1
  end

  test "in: a room the user is not in returns nothing" do
    hidden = Rooms::Closed.create!(name: "NoEntryVault", creator: users(:kevin))
    hidden.memberships.grant_to(users(:kevin))
    hidden.messages.create!(body: "secluded alpha", client_message_id: "op-in-hidden", creator: users(:kevin))

    get searches_url, params: { q: "in:#NoEntryVault secluded alpha" }

    assert_response :success
    assert_select ".message", count: 0
    assert_select ".search-sections__section", count: 0
  end

  test "sections exclude soft-deleted rooms" do
    board = Rooms::Board.create!(name: "Vanishing Board", creator: users(:david))
    board.memberships.grant_to(users(:david))
    board.channel_threads.create!(name: "Vanishing launch plan", creator: users(:david), work_status: "planned")
    closed = Rooms::Closed.create!(name: "Vanishing Room", creator: users(:david))
    closed.memberships.grant_to(users(:david))
    closed.channel_threads.create!(name: "Vanishing launch work", creator: users(:david), work_status: "planned")
    closed.events.create!(organizer: users(:david), title: "Vanishing launch gathering",
      starts_at: 2.days.from_now, time_zone: "America/New_York")

    get searches_url, params: { q: "vanishing launch" }

    assert_response :success
    assert_select ".search-sections__section", text: /Vanishing launch plan/
    assert_select ".search-sections__section", text: /Vanishing launch work/
    assert_select ".search-sections__section", text: /Vanishing launch gathering/

    board.begin_destroy!
    closed.begin_destroy!

    get searches_url, params: { q: "vanishing launch" }

    assert_response :success
    assert_select ".search-sections__section", text: /Vanishing/, count: 0
  end

  test "has:pin narrows results to pinned messages" do
    pinned = rooms(:designers).messages.create! body: "pinned alpha", client_message_id: "op-pin-yes", creator: users(:david)
    MessagePin.pin!(message: pinned, pinner: users(:david))
    rooms(:designers).messages.create! body: "pinned alpha", client_message_id: "op-pin-no", creator: users(:david)

    get searches_url, params: { q: "has:pin pinned alpha" }

    assert_response :success
    assert_select ".message", count: 1
  end

  test "has:link narrows results to messages carrying a link" do
    rooms(:designers).messages.create!(
      markdown_source: "linked alpha at https://example.com/alpha",
      client_message_id: "op-link-yes", creator: users(:david)
    )
    rooms(:designers).messages.create! body: "linked alpha", client_message_id: "op-link-no", creator: users(:david)

    get searches_url, params: { q: "has:link linked alpha" }

    assert_response :success
    assert_select ".message", count: 1
  end

  test "on: narrows results to that day" do
    day = Date.new(2026, 4, 2)
    rooms(:designers).messages.create!(
      body: "dated alpha", client_message_id: "op-date-yes",
      creator: users(:david), created_at: day.in_time_zone("UTC").noon
    )
    rooms(:designers).messages.create!(
      body: "dated alpha", client_message_id: "op-date-no",
      creator: users(:david), created_at: (day - 5).in_time_zone("UTC").noon
    )

    get searches_url, params: { q: "on:2026-04-02 dated alpha" }

    assert_response :success
    assert_select ".message", count: 1
  end

  test "is:thread narrows results to thread messages" do
    thread = rooms(:designers).channel_threads.create!(name: "Operator thread", creator: users(:david))
    thread.post_message!(
      creator: users(:david),
      attributes: { body: "threaded alpha", client_message_id: "op-thread-yes" }
    )
    rooms(:designers).messages.create! body: "threaded alpha", client_message_id: "op-thread-no", creator: users(:david)

    get searches_url, params: { q: "is:thread threaded alpha" }

    assert_response :success
    assert_select ".message", count: 1
  end

  test "filter-only queries list without text and show chips" do
    get searches_url, params: { q: "from:@JZ" }

    assert_response :success
    assert_select ".search-filter-chip", text: /from: JZ/
    assert_select ".message", minimum: 1
  end

  test "chips link back without their operator" do
    get searches_url, params: { q: "from:@jz has:file launch" }

    assert_response :success
    assert_select ".search-filter-chip", count: 2
    assert_select '.search-filter-chip a[aria-label="Remove from: jz filter"]', count: 1
    link = css_select('.search-filter-chip a[aria-label="Remove from: jz filter"]').first["href"]
    assert_includes link, "q=has%3Afile+launch"
  end

  test "boards, work threads, and events render as sections scoped to access" do
    board = Rooms::Board.create!(name: "Section Board", creator: users(:david))
    board.memberships.grant_to(users(:david))
    board.channel_threads.create!(name: "Sectionable launch plan", creator: users(:david), work_status: "planned")
    rooms(:designers).channel_threads.create!(name: "Sectionable launch work", creator: users(:david), work_status: "planned")
    hidden = Rooms::Closed.create!(name: "Hidden Sections", creator: users(:kevin))
    hidden.memberships.grant_to(users(:kevin))
    hidden.channel_threads.create!(name: "Launch hidden work", creator: users(:kevin), work_status: "planned")

    get searches_url, params: { q: "launch" }

    assert_response :success
    assert_select ".search-sections__section", minimum: 2
    assert_select ".search-sections__section", text: /Sectionable launch plan/
    assert_select ".search-sections__section", text: /Sectionable launch work/
    assert_select ".search-sections__section", text: /Launch hidden work/, count: 0
    assert_select ".search-sections__section", text: /Launch party planning/
  end

  test "search sections cost the same queries regardless of section size" do
    one = [ create_dm_event(users(:jason), "count-one") ]
    small = count_sections_queries(one)

    many = one + [ create_dm_event(users(:kevin), "count-two"), create_dm_event(users(:jz), "count-three") ]
    large = count_sections_queries(many)

    assert_equal 0, small
    assert_equal small, large
  end

  test "search sections label direct rooms neutrally" do
    create_dm_event(users(:jason), "label-one")

    get searches_url, params: { q: "sectioncount" }

    assert_response :success
    assert_select ".search-sections__meta", text: /a direct message/
  end

  test "operator values cannot inject SQL or FTS syntax" do
    get searches_url, params: { q: %(" OR 1=1 --) }
    assert_response :success

    get searches_url, params: { q: %(from:@" OR "1"="1 hello) }
    assert_response :success
    assert_select ".message", count: 0

    get searches_url, params: { q: "in:#% hello" }
    assert_response :success
    assert_select ".message", count: 0

    get searches_url, params: { q: "on:2026-13-45 hello" }
    assert_response :success
  end

  private
    # An event in its own direct room, so every section row's room lookup
    # carries distinct binds and can never hide in the query cache.
    def create_dm_event(peer, seq)
      dm = with_current_user(users(:david)) do
        Rooms::Direct.find_or_create_for([ users(:david), peer ])
      end
      Event.create!(
        room: dm, organizer: users(:david), title: "Sectioncount gathering #{seq}",
        starts_at: 2.days.from_now, time_zone: "America/New_York"
      ).id
    end

    def count_sections_queries(event_ids)
      events = Event.where(id: event_ids).includes(:room).to_a
      count = 0
      subscriber = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 unless payload[:name] == "SCHEMA" || payload[:cached]
      end

      ActiveRecord::Base.connection_pool.clear_query_cache
      with_current_user(users(:david)) do
        ApplicationController.render(
          partial: "searches/sections",
          assigns: { board_posts: [], work_threads: [], events: events }
        )
      end
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscriber)
    end

    def with_current_user(user)
      previous = Current.user
      Current.user = user
      yield
    ensure
      Current.user = previous
    end
end
