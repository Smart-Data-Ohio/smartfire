require "test_helper"

class Messages::BoostsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @message = messages(:first)
  end

  test "create" do
    assert_turbo_stream_broadcasts [ @message.room, :messages ], count: 1 do
      assert_difference -> { @message.boosts.count }, 1 do
        post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: "Morning!" } }
        assert_redirected_to message_boosts_url(@message)
      end
    end
  end

  test "destroy" do
    assert_turbo_stream_broadcasts [ @message.room, :messages ], count: 1 do
      assert_difference -> { @message.boosts.count }, -1 do
        delete message_boost_url(@message, boosts(:first), format: :turbo_stream)
        assert_response :success
      end
    end
  end

  test "a human emoji toggle removes legacy duplicates under the message lock" do
    emoji = "👍"
    Boost.create!(message: @message, booster: users(:david), content: emoji)
    Boost.create!(message: @message, booster: users(:david), content: emoji)

    assert_difference -> { @message.boosts.where(booster: users(:david), content: emoji).count }, -2 do
      post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: emoji } }
      assert_redirected_to message_boosts_url(@message)
    end
  end

  test "a quick reaction sent as a shortcode toggles instead of duplicating" do
    assert_difference -> { @message.boosts.where(content: "👍").count }, 1 do
      post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":thumbsup:" } }
    end

    assert_difference -> { @message.boosts.where(content: "👍").count }, -1 do
      post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":thumbsup:" } }
    end
  end

  test "create accepts a brand shortcode and renders its icon" do
    assert_difference -> { @message.boosts.count }, 1 do
      post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":openai:" } }
      assert_redirected_to message_boosts_url(@message)
    end

    get message_boosts_url(@message)

    assert_response :success
    icon = Nokogiri::HTML5.fragment(response.body).at_css("img.icon--brand")
    assert icon, "expected an icon image in #{response.body}"
    assert_match %r{\A/assets/icons/brands/openai-[a-z0-9]+\.svg\z}, icon["src"]
    assert_equal ":openai:", icon["alt"]
  end

  test "create accepts a workspace icon shortcode and renders its image" do
    create_workspace_icon(name: "acme", title: "Acme Corp")

    assert_difference -> { @message.boosts.count }, 1 do
      post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":acme:" } }
      assert_redirected_to message_boosts_url(@message)
    end

    assert_equal ":acme:", @message.boosts.last.content

    get message_boosts_url(@message)

    assert_response :success
    icon = Nokogiri::HTML5.fragment(response.body).at_css("img.icon--custom")
    assert icon, "expected a custom icon image in #{response.body}"
    assert_equal "/icons/acme", icon["src"]
    assert_equal ":acme:", icon["alt"]
  end

  test "create stores an unknown shortcode as literal text" do
    assert_turbo_stream_broadcasts [ @message.room, :messages ], count: 1 do
      assert_difference -> { @message.boosts.count }, 1 do
        post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":lol:" } }
        assert_redirected_to message_boosts_url(@message)
      end
    end

    assert_equal ":lol:", @message.boosts.last.content

    get message_boosts_url(@message)

    assert_response :success
    assert_includes response.body, ":lol:"
    assert_empty Nokogiri::HTML5.fragment(response.body).css("img.icon--brand")
  end

  test "two users reacting with the same custom icon share one counted chip" do
    create_workspace_icon(name: "acme", title: "Acme Corp")

    post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":acme:" } }
    assert_redirected_to message_boosts_url(@message)

    sign_in :kevin
    post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":acme: " } }
    assert_redirected_to message_boosts_url(@message)

    get message_boosts_url(@message)
    assert_response :success

    chips = Nokogiri::HTML5.fragment(response.body).css(".reaction-chip[data-reaction=':acme:']")
    assert_equal 1, chips.length, "expected one shared chip in #{response.body}"
    assert_equal "2", chips.first.at_css(".reaction-chip__count").text
    assert chips.first.at_css("img.icon--custom"), "expected the icon in #{response.body}"
    legacy = Nokogiri::HTML5.fragment(response.body).css(".boosts__legacy .boost")
    assert legacy.none? { |node| node.at_css("img.icon--custom") }, "expected no legacy icon chip in #{response.body}"

    sign_in :david
    assert_difference -> { @message.boosts.where(content: ":acme:").count }, -1 do
      post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":acme:" } }
    end

    get message_boosts_url(@message)
    chip = Nokogiri::HTML5.fragment(response.body).at_css(".reaction-chip[data-reaction=':acme:']")
    assert_equal "1", chip.at_css(".reaction-chip__count").text
  end

  test "an autocompleted emoji shortcode with a trailing space renders its chip" do
    post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":fire: " } }
    assert_redirected_to message_boosts_url(@message)

    assert_equal "🔥", @message.boosts.last.content

    get message_boosts_url(@message)
    assert_response :success
    assert_not_nil Nokogiri::HTML5.fragment(response.body).at_css(".reaction-chip[data-reaction='🔥']")
  end

  test "an autocompleted brand shortcode with a trailing space renders its icon chip" do
    post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: ":anthropic: " } }
    assert_redirected_to message_boosts_url(@message)

    assert_equal ":anthropic:", @message.boosts.last.content

    get message_boosts_url(@message)
    assert_response :success
    icon = Nokogiri::HTML5.fragment(response.body).at_css(".reaction-chip[data-reaction=':anthropic:'] img.icon--brand")
    assert icon, "expected a brand icon chip in #{response.body}"
  end

  test "a non-quick emoji aggregates and toggles instead of duplicating" do
    post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: "💯" } }
    assert_redirected_to message_boosts_url(@message)

    assert_difference -> { @message.boosts.where(content: "💯").count }, -1 do
      post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: "💯" } }
    end
  end

  test "free text stays a per-person legacy boost without toggling" do
    assert_difference -> { @message.boosts.count }, 2 do
      2.times do
        post message_boosts_url(@message, format: :turbo_stream), params: { boost: { content: "Morning!" } }
        assert_redirected_to message_boosts_url(@message)
      end
    end

    get message_boosts_url(@message)
    assert_response :success
    assert_empty Nokogiri::HTML5.fragment(response.body).css(".reaction-chip[data-reaction='Morning!']")
    legacy = Nokogiri::HTML5.fragment(response.body).css(".boosts__legacy .boost").select { |node| node.text.include?("Morning!") }
    assert_equal 2, legacy.length
  end

  test "the boost action opens the soft keyboard" do
    get message_boosts_url(@message)

    assert_response :success
    link = Nokogiri::HTML5.fragment(response.body).at_css(".boost__action")
    assert_equal "soft-keyboard#open", link["data-action"]
  end

  test "action metadata groups reaction counts by distinct reactor" do
    emoji = "👍"
    Boost.create!(message: @message, booster: users(:david), content: emoji)
    Boost.create!(message: @message, booster: users(:david), content: emoji)
    Boost.create!(message: @message, booster: users(:jason), content: emoji)

    get actions_room_message_url(@message.room, @message, format: :json)

    assert_response :success
    reaction = response.parsed_body.dig("actions", "reactions", emoji)
    assert_equal 2, reaction.fetch("count")
    assert reaction.fetch("active")
  end
end
