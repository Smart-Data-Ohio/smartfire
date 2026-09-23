# Builders for Fizzy API payloads and stubs. Shapes follow the Fizzy API
# docs and JSON views (docs/api and app/views in github.com/basecamp/fizzy).
module FizzyTestHelper
  FIZZY_ACCOUNT_ID = "897362094"
  FIZZY_BOARD_ID = "03board1"
  FIZZY_COLUMN_ID = "03column1"

  def fizzy_user_payload(id: "03user1", name: "David", account_id: FIZZY_ACCOUNT_ID)
    {
      "id" => id,
      "name" => name,
      "role" => "member",
      "active" => true,
      "email_address" => "david@example.com",
      "created_at" => "2025-12-05T19:36:35.401Z",
      "url" => "https://app.fizzy.do/#{account_id}/users/#{id}",
      "avatar_url" => "https://app.fizzy.do/#{account_id}/users/#{id}/avatar"
    }
  end

  def fizzy_identity_payload(account_id: FIZZY_ACCOUNT_ID, account_name: "Smart Data", user_id: "03user1", user_name: "David")
    {
      "id" => "identity-1",
      "accounts" => [
        {
          "id" => "03account1",
          "name" => account_name,
          "slug" => "/#{account_id}",
          "created_at" => "2025-12-05T19:36:35.377Z",
          "user" => fizzy_user_payload(id: user_id, name: user_name, account_id: account_id)
        }
      ]
    }
  end

  def fizzy_board_payload(id: FIZZY_BOARD_ID, name: "Engineering")
    {
      "id" => id,
      "name" => name,
      "all_access" => true,
      "created_at" => "2025-12-05T19:36:35.534Z",
      "auto_postpone_period_in_days" => 30,
      "url" => "https://app.fizzy.do/#{FIZZY_ACCOUNT_ID}/boards/#{id}",
      "creator" => fizzy_user_payload
    }
  end

  def fizzy_column_payload(id: FIZZY_COLUMN_ID, name: "In Progress")
    {
      "id" => id,
      "name" => name,
      "color" => "var(--color-card-4)",
      "created_at" => "2025-12-05T19:36:35.534Z",
      "cards_url" => "https://app.fizzy.do/#{FIZZY_ACCOUNT_ID}/boards/#{FIZZY_BOARD_ID}/columns/#{id}/cards"
    }
  end

  def fizzy_card_payload(number: 579, title: "Fix the billing bug", board_name: "Engineering",
      column_name: "In Progress", closed: false, postponed: false, tags: %w[ billing urgent ],
      steps: [ { "id" => "03step1", "content" => "Reproduce", "completed" => true }, { "id" => "03step2", "content" => "Fix", "completed" => false } ],
      assignees: [ fizzy_user_payload ], account_id: FIZZY_ACCOUNT_ID)
    {
      "id" => "03card#{number}",
      "number" => number,
      "title" => title,
      "status" => "published",
      "description" => "Card description",
      "description_html" => "<div class=\"action-text-content\"><p>Card description</p></div>",
      "image_url" => nil,
      "has_attachments" => false,
      "tags" => tags,
      "closed" => closed,
      "postponed" => postponed,
      "golden" => false,
      "last_active_at" => "2026-09-20T10:00:00.000Z",
      "created_at" => "2026-09-19T10:00:00.000Z",
      "url" => "https://app.fizzy.do/#{account_id}/cards/#{number}",
      "board" => fizzy_board_payload(name: board_name),
      "creator" => fizzy_user_payload,
      "assignees" => assignees,
      "has_more_assignees" => false,
      "comments_url" => "https://app.fizzy.do/#{account_id}/cards/#{number}/comments",
      "reactions_url" => "https://app.fizzy.do/#{account_id}/cards/#{number}/reactions",
      "steps" => steps
    }.tap do |card|
      card["column"] = fizzy_column_payload(name: column_name) if column_name.present? && !closed
    end
  end

  def fizzy_comment_payload(number: 579, body: "Nice work", account_id: FIZZY_ACCOUNT_ID)
    {
      "id" => "03comment1",
      "created_at" => "2026-09-20T10:00:00.000Z",
      "updated_at" => "2026-09-20T10:00:00.000Z",
      "body" => { "plain_text" => body, "html" => "<div class=\"action-text-content\">#{body}</div>" },
      "creator" => fizzy_user_payload,
      "card" => { "id" => "03card#{number}", "url" => "https://app.fizzy.do/#{account_id}/cards/#{number}" },
      "reactions_url" => "https://app.fizzy.do/#{account_id}/cards/#{number}/comments/03comment1/reactions",
      "url" => "https://app.fizzy.do/#{account_id}/cards/#{number}/comments/03comment1"
    }
  end

  def link_fizzy!(user, token: "fizzy-token-for-#{user.respond_to?(:name) ? user.name.parameterize : user}",
      account_id: FIZZY_ACCOUNT_ID, account_name: "Smart Data", user_id: "03user1", user_name: nil)
    user_name ||= user.name
    FizzyConnectedAccount.create!(
      user: user.is_a?(User) ? user : users(user),
      access_token: token,
      fizzy_account_id: account_id,
      fizzy_account_name: account_name,
      fizzy_user_id: user_id,
      fizzy_user_name: user_name
    )
  end

  def stub_fizzy_identity(token, payload: fizzy_identity_payload, status: 200)
    stub_request(:get, "https://app.fizzy.do/my/identity.json")
      .with(headers: { "Authorization" => "Bearer #{token}" })
      .to_return(status: status, body: payload.to_json)
  end

  def stub_fizzy_card(number, payload: nil, status: 200, token: nil, account_id: FIZZY_ACCOUNT_ID)
    payload ||= fizzy_card_payload(number: number, account_id: account_id)
    request = stub_request(:get, "https://app.fizzy.do/#{account_id}/cards/#{number}.json")
    request = request.with(headers: { "Authorization" => "Bearer #{token}" }) if token
    request.to_return(status: status, body: status == 204 ? "" : payload.to_json)
  end
end
